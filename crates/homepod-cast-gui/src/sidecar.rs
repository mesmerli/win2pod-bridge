//! Sidecar Process Supervisor for win2pod-engine.
//!
//! Manages spawning, health monitoring, and Named Pipe IPC communication with
//! the GPL-3.0 backend engine daemon (`win2pod-engine.exe`).

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};
use win2pod_ipc::protocol::*;
use win2pod_ipc::transport::{connect_pipe_client, PipeReader, PipeWriter, PipeTransport};

pub struct SidecarClient {
    writer: Arc<Mutex<Option<PipeWriter<tokio::io::WriteHalf<tokio::net::windows::named_pipe::NamedPipeClient>>>>>,
    pending_response: Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<IpcResponse>>>>,
    request_lock: Arc<Mutex<()>>,
    event_sender: Arc<std::sync::Mutex<Option<mpsc::Sender<IpcEvent>>>>,
    child_process: Arc<Mutex<Option<Child>>>,
    is_connected: Arc<AtomicBool>,
}

impl SidecarClient {
    pub fn new() -> Self {
        Self {
            writer: Arc::new(Mutex::new(None)),
            pending_response: Arc::new(std::sync::Mutex::new(None)),
            request_lock: Arc::new(Mutex::new(())),
            event_sender: Arc::new(std::sync::Mutex::new(None)),
            child_process: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Locate `win2pod-engine.exe` in the same directory as `win2pod.exe` or standard cargo output paths.
    pub fn find_engine_executable() -> Option<PathBuf> {
        if let Ok(mut exe_dir) = std::env::current_exe() {
            exe_dir.pop();
            let sibling = exe_dir.join("win2pod-engine.exe");
            if sibling.exists() {
                return Some(sibling);
            }
        }

        // Check workspace target dirs (debug / release)
        let candidates = [
            "target/debug/win2pod-engine.exe",
            "target/release/win2pod-engine.exe",
            "dist/win2pod-engine.exe",
            "../target/debug/win2pod-engine.exe",
            "../target/release/win2pod-engine.exe",
        ];

        for c in candidates {
            let path = Path::new(c);
            if path.exists() {
                return Some(path.to_path_buf());
            }
        }

        None
    }

    fn spawn_reader_task(
        mut reader: PipeReader<tokio::io::ReadHalf<tokio::net::windows::named_pipe::NamedPipeClient>>,
        is_connected: Arc<AtomicBool>,
        pending_response: Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<IpcResponse>>>>,
        event_sender: Arc<std::sync::Mutex<Option<mpsc::Sender<IpcEvent>>>>,
    ) {
        tokio::spawn(async move {
            info!("Sidecar dedicated reader loop started");
            while is_connected.load(Ordering::Relaxed) {
                match reader.read_message().await {
                    Ok(Some(IpcMessage::Event(evt))) => {
                        let tx_opt = event_sender.lock().unwrap().clone();
                        if let Some(tx) = tx_opt {
                            let _ = tx.send(evt).await;
                        }
                    }
                    Ok(Some(IpcMessage::Response(resp))) => {
                        let tx_opt = pending_response.lock().unwrap().take();
                        if let Some(tx) = tx_opt {
                            let _ = tx.send(resp);
                        } else {
                            warn!("Received unexpected or timed-out IPC response: {:?}", resp);
                        }
                    }
                    Ok(Some(msg)) => {
                        warn!("Received unexpected IPC message from sidecar: {:?}", msg);
                    }
                    Ok(None) => {
                        warn!("Sidecar named pipe closed by engine");
                        is_connected.store(false, Ordering::SeqCst);
                        break;
                    }
                    Err(e) => {
                        warn!("Error reading from sidecar named pipe: {e:#}");
                        is_connected.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }
            info!("Sidecar dedicated reader loop terminated");
        });
    }

    /// Start the sidecar engine process and bind it to a Windows Job Object.
    pub async fn ensure_started(&self, pipe_name: Option<&str>) -> Result<()> {
        if self.is_connected() {
            return Ok(());
        }

        let name = pipe_name.unwrap_or(DEFAULT_PIPE_NAME);

        // First attempt a fast connection in case it's already running
        if let Ok(client) = connect_pipe_client(Some(name), 1, Duration::from_millis(100)).await {
            info!("Connected to existing win2pod-engine daemon on {name}");
            let transport = PipeTransport::new(client);
            let (reader, writer) = transport.split();

            {
                let mut w_lock = self.writer.lock().await;
                *w_lock = Some(writer);
            }
            self.is_connected.store(true, Ordering::SeqCst);

            Self::spawn_reader_task(
                reader,
                self.is_connected.clone(),
                self.pending_response.clone(),
                self.event_sender.clone(),
            );
            return Ok(());
        }

        let engine_path = Self::find_engine_executable()
            .ok_or_else(|| anyhow!("Could not find win2pod-engine.exe binary"))?;

        info!("Launching sidecar engine: {:?}", engine_path);

        crate::firewall::ensure_firewall_rule_for_engine(&engine_path);

        let mut cmd = Command::new(&engine_path);
        cmd.arg("--pipe-name").arg(name);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = cmd.spawn().with_context(|| format!("Failed to spawn {:?}", engine_path))?;
        {
            let mut child_lock = self.child_process.lock().await;
            *child_lock = Some(child);
        }

        // Connect with retry (up to 5 seconds)
        info!("Connecting to newly spawned win2pod-engine...");
        let client = connect_pipe_client(Some(name), 50, Duration::from_millis(100)).await
            .context("Failed to connect to win2pod-engine after spawning")?;

        let transport = PipeTransport::new(client);
        let (reader, writer) = transport.split();

        {
            let mut w_lock = self.writer.lock().await;
            *w_lock = Some(writer);
        }
        self.is_connected.store(true, Ordering::SeqCst);

        Self::spawn_reader_task(
            reader,
            self.is_connected.clone(),
            self.pending_response.clone(),
            self.event_sender.clone(),
        );

        info!("Successfully established IPC link with win2pod-engine");

        Ok(())
    }

    /// Send a request to the sidecar and receive its response.
    pub async fn send_request(&self, req: IpcRequest) -> Result<IpcResponse> {
        if !self.is_connected() {
            return Err(anyhow!("Sidecar not connected"));
        }

        // Sequentialize requests so only one request is in flight at a time
        let _req_guard = self.request_lock.lock().await;

        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending_response.lock().unwrap();
            *pending = Some(resp_tx);
        }

        {
            let mut writer_lock = self.writer.lock().await;
            let writer = writer_lock.as_mut().ok_or_else(|| anyhow!("Sidecar writer not connected"))?;
            if let Err(e) = writer.write_message(&IpcMessage::Request(req)).await {
                let mut pending = self.pending_response.lock().unwrap();
                *pending = None;
                self.is_connected.store(false, Ordering::SeqCst);
                return Err(e);
            }
        }

        match tokio::time::timeout(Duration::from_secs(5), resp_rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_)) => {
                let mut pending = self.pending_response.lock().unwrap();
                *pending = None;
                Err(anyhow!("Sidecar channel closed before sending response"))
            }
            Err(_) => {
                let mut pending = self.pending_response.lock().unwrap();
                *pending = None;
                Err(anyhow!("Sidecar request timed out after 5 seconds"))
            }
        }
    }

    /// Start listening for async events in the background.
    pub fn start_event_listener(
        &self,
        event_sender: mpsc::Sender<IpcEvent>,
    ) {
        let mut guard = self.event_sender.lock().unwrap();
        *guard = Some(event_sender);
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    /// Enumerate audio endpoints via the unified engine.
    pub async fn enumerate_audio_endpoints(&self) -> Result<Vec<AudioEndpointIpc>> {
        self.ensure_started(None).await?;
        match self.send_request(IpcRequest::EnumerateAudioEndpoints).await? {
            IpcResponse::AudioEndpoints(endpoints) => Ok(endpoints),
            IpcResponse::Error(e) => Err(anyhow!("EnumerateAudioEndpoints error: {e}")),
            other => Err(anyhow!("Unexpected response to EnumerateAudioEndpoints: {:?}", other)),
        }
    }

    /// Set hardware speaker mute on an audio endpoint via the unified engine.
    pub async fn set_speaker_mute(&self, mute: bool, device_id: Option<String>) -> Result<()> {
        self.ensure_started(None).await?;
        match self.send_request(IpcRequest::SetSpeakerMute { mute, device_id }).await? {
            IpcResponse::Ok => Ok(()),
            IpcResponse::Error(e) => Err(anyhow!("SetSpeakerMute error: {e}")),
            other => Err(anyhow!("Unexpected response to SetSpeakerMute: {:?}", other)),
        }
    }

    /// Set Stand-in device redirection ID for AirPods protection.
    pub async fn set_standin_redirection(&self, standin_id: Option<String>) -> Result<()> {
        self.ensure_started(None).await?;
        match self.send_request(IpcRequest::SetStandinRedirection { standin_id }).await? {
            IpcResponse::Ok => Ok(()),
            IpcResponse::Error(e) => Err(anyhow!("SetStandinRedirection error: {e}")),
            other => Err(anyhow!("Unexpected response to SetStandinRedirection: {:?}", other)),
        }
    }

    /// Switch the engine operating mode (Idle, AirPlay, LocalAo).
    pub async fn set_engine_mode(&self, mode: EngineModeIpc) -> Result<()> {
        self.ensure_started(None).await?;
        match self.send_request(IpcRequest::SetEngineMode { mode }).await? {
            IpcResponse::Ok => Ok(()),
            IpcResponse::Error(e) => Err(anyhow!("SetEngineMode error: {e}")),
            other => Err(anyhow!("Unexpected response to SetEngineMode: {:?}", other)),
        }
    }

    /// Gracefully shutdown sidecar.
    pub async fn shutdown(&self) {
        if self.is_connected() {
            let _ = self.send_request(IpcRequest::Shutdown).await;
        }
        self.is_connected.store(false, Ordering::SeqCst);
        {
            let mut writer_lock = self.writer.lock().await;
            *writer_lock = None;
        }
        let mut child_lock = self.child_process.lock().await;
        if let Some(mut child) = child_lock.take() {
            let _ = child.kill();
        }
    }
}

pub fn device_to_ipc(d: &airplay_core::Device) -> DeviceIpc {
    let is_stereo = d.tight_sync_id.is_some() && d.group_public_name.is_some();
    let mut addrs: Vec<String> = d.addresses.iter()
        .filter(|a| a.is_ipv4())
        .map(|a| a.to_string())
        .collect();
    if addrs.is_empty() {
        addrs = d.addresses.iter().map(|a| a.to_string()).collect();
    }
    DeviceIpc {
        id: d.id.to_mac_string(),
        name: d.name.clone(),
        model: d.model.clone(),
        addresses: addrs,
        port: if d.port == 0 { 7000 } else { d.port },
        is_stereo_pair: is_stereo,
        is_group: false,
        stereo_role: if is_stereo {
            if d.is_group_leader {
                Some("leader".into())
            } else {
                Some("follower".into())
            }
        } else {
            None
        },
    }
}

pub fn device_items_to_target(items: &[crate::controller::DeviceItem]) -> ConnectTargetIpc {
    if items.is_empty() {
        return ConnectTargetIpc::Group {
            name: "Empty".into(),
            devices: vec![],
        };
    }

    if items.len() == 1 {
        let item = &items[0];
        if item.is_stereo_pair && item.member_devices.len() >= 2 {
            let left = device_to_ipc(&item.member_devices[0]);
            let right = device_to_ipc(&item.member_devices[1]);
            ConnectTargetIpc::StereoPair {
                name: item.name.clone(),
                left,
                right,
            }
        } else if !item.member_devices.is_empty() {
            let mut devs = Vec::new();
            for m in &item.member_devices {
                devs.push(device_to_ipc(m));
            }
            ConnectTargetIpc::Group {
                name: item.name.clone(),
                devices: devs,
            }
        } else {
            ConnectTargetIpc::Single(device_to_ipc(&item.raw_device))
        }
    } else {
        let mut all_devs = Vec::new();
        for item in items {
            if item.member_devices.is_empty() {
                all_devs.push(device_to_ipc(&item.raw_device));
            } else {
                for m in &item.member_devices {
                    all_devs.push(device_to_ipc(m));
                }
            }
        }
        ConnectTargetIpc::Group {
            name: items[0].name.clone(),
            devices: all_devs,
        }
    }
}
