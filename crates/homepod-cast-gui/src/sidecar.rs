//! Sidecar Process Supervisor for win2pod-engine.
//!
//! Manages spawning, health monitoring, and Named Pipe IPC communication with
//! the GPL-3.0 backend engine daemon (`win2pod-engine.exe`) under a formal
//! finite state machine lifecycle.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};
use win2pod_ipc::protocol::*;
use win2pod_ipc::transport::{connect_pipe_client, PipeReader, PipeWriter, PipeTransport};

pub use crate::sidecar_fsm::{
    SidecarAction, SidecarLifecycleEvent, SidecarLifecycleFsm, SidecarLifecycleState,
};

pub struct SidecarClient {
    fsm: Arc<std::sync::Mutex<SidecarLifecycleFsm>>,
    writer: Arc<Mutex<Option<PipeWriter<tokio::io::WriteHalf<tokio::net::windows::named_pipe::NamedPipeClient>>>>>,
    next_req_id: Arc<AtomicU64>,
    pending_responses: Arc<std::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<IpcResponse>>>>,
    request_lock: Arc<Mutex<()>>,
    event_sender: Arc<std::sync::Mutex<Option<mpsc::Sender<IpcEvent>>>>,
    child_process: Arc<Mutex<Option<Child>>>,
    #[cfg(windows)]
    job_handle: Arc<std::sync::Mutex<Option<isize>>>,
}

#[cfg(windows)]
fn setup_job_object_for_child(child: &Child) -> Option<isize> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::*;
    use windows_sys::Win32::Foundation::HANDLE;

    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job == 0 as _ {
            warn!("[Sidecar] Failed to create Windows Job Object");
            return None;
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let res = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if res == 0 {
            warn!("[Sidecar] Failed to set JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE on Job Object");
            windows_sys::Win32::Foundation::CloseHandle(job);
            return None;
        }

        let child_handle = child.as_raw_handle() as HANDLE;
        let assign_res = AssignProcessToJobObject(job, child_handle);
        if assign_res == 0 {
            warn!("[Sidecar] Failed to assign child process to Windows Job Object");
            windows_sys::Win32::Foundation::CloseHandle(job);
            return None;
        }

        info!("[Sidecar] Successfully bound win2pod-engine to Windows Job Object (JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE)");
        Some(job as isize)
    }
}

impl SidecarClient {
    pub fn new() -> Self {
        Self {
            fsm: Arc::new(std::sync::Mutex::new(SidecarLifecycleFsm::new())),
            writer: Arc::new(Mutex::new(None)),
            next_req_id: Arc::new(AtomicU64::new(1)),
            pending_responses: Arc::new(std::sync::Mutex::new(HashMap::new())),
            request_lock: Arc::new(Mutex::new(())),
            event_sender: Arc::new(std::sync::Mutex::new(None)),
            child_process: Arc::new(Mutex::new(None)),
            #[cfg(windows)]
            job_handle: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Read the current formal lifecycle state of the sidecar.
    pub fn state(&self) -> SidecarLifecycleState {
        self.fsm.lock().unwrap().state().clone()
    }

    /// Check if the sidecar is currently connected (Ready or Streaming).
    pub fn is_connected(&self) -> bool {
        self.fsm.lock().unwrap().is_connected()
    }

    /// Check if the sidecar is shutting down or terminated.
    pub fn is_shutting_down(&self) -> bool {
        self.fsm.lock().unwrap().is_shutting_down()
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
        fsm: Arc<std::sync::Mutex<SidecarLifecycleFsm>>,
        pending_responses: Arc<std::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<IpcResponse>>>>,
        event_sender: Arc<std::sync::Mutex<Option<mpsc::Sender<IpcEvent>>>>,
    ) {
        tokio::spawn(async move {
            info!("Sidecar dedicated reader loop started");
            loop {
                // If shutting down, stop reading
                {
                    let f = fsm.lock().unwrap();
                    if f.is_shutting_down() {
                        break;
                    }
                }

                match reader.read_message().await {
                    Ok(Some(IpcMessage::Event { payload: evt })) => {
                        let tx_opt = event_sender.lock().unwrap().clone();
                        if let Some(tx) = tx_opt {
                            let _ = tx.send(evt).await;
                        }
                    }
                    Ok(Some(IpcMessage::Response { req_id, payload: resp })) => {
                        let tx_opt = {
                            let mut map = pending_responses.lock().unwrap();
                            if req_id != 0 {
                                map.remove(&req_id)
                            } else {
                                // Fallback for legacy sidecar sending req_id = 0
                                let first_key = map.keys().next().copied();
                                first_key.and_then(|k| map.remove(&k))
                            }
                        };
                        if let Some(tx) = tx_opt {
                            let _ = tx.send(resp);
                        } else {
                            warn!("Received unexpected or timed-out IPC response for req_id={req_id}: {:?}", resp);
                        }
                    }
                    Ok(Some(msg)) => {
                        warn!("Received unexpected IPC message from sidecar: {:?}", msg);
                    }
                    Ok(None) => {
                        let is_shutting_down = {
                            let f = fsm.lock().unwrap();
                            f.is_shutting_down()
                        };
                        if is_shutting_down {
                            info!("Sidecar named pipe closed as expected during shutdown");
                        } else {
                            warn!("Sidecar named pipe unexpectedly closed by engine");
                            let mut f = fsm.lock().unwrap();
                            let _ = f.transition(SidecarLifecycleEvent::PipeBroken {
                                error: "Pipe closed by engine".into(),
                            });
                        }
                        break;
                    }
                    Err(e) => {
                        let is_shutting_down = {
                            let f = fsm.lock().unwrap();
                            f.is_shutting_down()
                        };
                        if is_shutting_down {
                            info!("Sidecar named pipe read error during shutdown: {e:#}");
                        } else {
                            warn!("Error reading from sidecar named pipe: {e:#}");
                            let mut f = fsm.lock().unwrap();
                            let _ = f.transition(SidecarLifecycleEvent::PipeBroken {
                                error: format!("Pipe read error: {e:#}"),
                            });
                        }
                        break;
                    }
                }
            }
            info!("Sidecar dedicated reader loop terminated");
        });
    }

    /// Start the sidecar engine process and bind it to a Windows Job Object.
    pub async fn ensure_started(&self, pipe_name: Option<&str>) -> Result<()> {
        let action = {
            let mut fsm = self.fsm.lock().unwrap();
            fsm.transition(SidecarLifecycleEvent::EnsureStarted)
                .map_err(|e| anyhow!("{e}"))?
        };

        if action == SidecarAction::NoAction {
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
            {
                let mut fsm = self.fsm.lock().unwrap();
                let _ = fsm.transition(SidecarLifecycleEvent::Connected);
            }

            Self::spawn_reader_task(
                reader,
                self.fsm.clone(),
                self.pending_responses.clone(),
                self.event_sender.clone(),
            );
            return Ok(());
        }

        let engine_path = Self::find_engine_executable()
            .ok_or_else(|| {
                let err = "Could not find win2pod-engine.exe binary".to_string();
                let mut fsm = self.fsm.lock().unwrap();
                let _ = fsm.transition(SidecarLifecycleEvent::FatalError { reason: err.clone() });
                anyhow!(err)
            })?;

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
        #[cfg(windows)]
        {
            if let Some(job) = setup_job_object_for_child(&child) {
                let mut jh = self.job_handle.lock().unwrap();
                *jh = Some(job);
            }
        }
        {
            let mut child_lock = self.child_process.lock().await;
            *child_lock = Some(child);
        }

        // Connect with retry (up to 5 seconds)
        info!("Connecting to newly spawned win2pod-engine...");
        let client = match connect_pipe_client(Some(name), 50, Duration::from_millis(100)).await {
            Ok(c) => c,
            Err(e) => {
                let mut fsm = self.fsm.lock().unwrap();
                let _ = fsm.transition(SidecarLifecycleEvent::PipeBroken {
                    error: format!("Failed to connect after spawning: {e:#}"),
                });
                return Err(e).context("Failed to connect to win2pod-engine after spawning");
            }
        };

        let transport = PipeTransport::new(client);
        let (reader, writer) = transport.split();

        {
            let mut w_lock = self.writer.lock().await;
            *w_lock = Some(writer);
        }
        {
            let mut fsm = self.fsm.lock().unwrap();
            let _ = fsm.transition(SidecarLifecycleEvent::Connected);
        }

        Self::spawn_reader_task(
            reader,
            self.fsm.clone(),
            self.pending_responses.clone(),
            self.event_sender.clone(),
        );

        info!("Successfully established IPC link with win2pod-engine");

        Ok(())
    }

    async fn send_request_internal(&self, req: IpcRequest, timeout: Duration) -> Result<IpcResponse> {
        // Sequentialize requests so only one request is in flight at a time
        let _req_guard = self.request_lock.lock().await;

        let req_id = self.next_req_id.fetch_add(1, Ordering::SeqCst);
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending_responses.lock().unwrap();
            pending.insert(req_id, resp_tx);
        }

        {
            let mut writer_lock = self.writer.lock().await;
            let writer = writer_lock.as_mut().ok_or_else(|| anyhow!("Sidecar writer not connected"))?;
            if let Err(e) = writer.write_message(&IpcMessage::Request { req_id, payload: req }).await {
                let mut pending = self.pending_responses.lock().unwrap();
                pending.remove(&req_id);
                return Err(e);
            }
        }

        match tokio::time::timeout(timeout, resp_rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_)) => {
                let mut pending = self.pending_responses.lock().unwrap();
                pending.remove(&req_id);
                Err(anyhow!("Sidecar channel closed before sending response (req_id={req_id})"))
            }
            Err(_) => {
                let mut pending = self.pending_responses.lock().unwrap();
                pending.remove(&req_id);
                Err(anyhow!("Sidecar request timed out after {:?} (req_id={req_id})", timeout))
            }
        }
    }

    /// Send a request to the sidecar and receive its response.
    pub async fn send_request(&self, req: IpcRequest) -> Result<IpcResponse> {
        if !self.is_connected() {
            return Err(anyhow!("Sidecar not connected (state: {:?})", self.state()));
        }

        self.send_request_internal(req, Duration::from_secs(5)).await
    }

    /// Send a strongly-typed request and directly extract its typed response.
    pub async fn send_typed_request<R: IpcRequestPayload>(&self, req: R) -> Result<R::ExpectedResponse> {
        if !self.is_connected() {
            return Err(anyhow!("Sidecar not connected (state: {:?})", self.state()));
        }
        let resp = self.send_request_internal(req.into(), Duration::from_secs(5)).await?;
        R::extract_response(resp).map_err(|e| anyhow!("{e}"))
    }

    /// Start listening for async events in the background.
    pub fn start_event_listener(
        &self,
        event_sender: mpsc::Sender<IpcEvent>,
    ) {
        let mut guard = self.event_sender.lock().unwrap();
        *guard = Some(event_sender);
    }

    /// Ping the backend Sidecar engine and receive verified Pong nonce.
    pub async fn ping(&self, nonce: u64) -> Result<u64> {
        self.ensure_started(None).await?;
        self.send_typed_request(PingRequest { nonce }).await
    }

    /// Query full Engine status.
    pub async fn get_status(&self) -> Result<EngineStatusIpc> {
        self.ensure_started(None).await?;
        self.send_typed_request(GetStatusRequest).await
    }

    /// Enumerate audio endpoints via the unified engine.
    pub async fn enumerate_audio_endpoints(&self) -> Result<Vec<AudioEndpointIpc>> {
        self.ensure_started(None).await?;
        self.send_typed_request(EnumerateAudioEndpointsRequest).await
    }

    /// Set hardware speaker mute on an audio endpoint via the unified engine.
    pub async fn set_speaker_mute(&self, mute: bool, device_id: Option<String>) -> Result<()> {
        self.ensure_started(None).await?;
        self.send_request(IpcRequest::SetSpeakerMute { mute, device_id }).await?
            .ok()
            .map_err(|e| anyhow!("{e}"))
    }

    /// Set Stand-in device redirection ID for AirPods protection.
    pub async fn set_standin_redirection(&self, standin_id: Option<String>) -> Result<()> {
        self.ensure_started(None).await?;
        self.send_request(IpcRequest::SetStandinRedirection { standin_id }).await?
            .ok()
            .map_err(|e| anyhow!("{e}"))
    }

    /// Switch the engine operating mode (Idle, AirPlay, LocalAo).
    pub async fn set_engine_mode(&self, mode: EngineModeIpc) -> Result<()> {
        self.ensure_started(None).await?;
        let res = self.send_request(IpcRequest::SetEngineMode { mode: mode.clone() }).await?
            .ok()
            .map_err(|e| anyhow!("{e}"));

        if res.is_ok() {
            let mut fsm = self.fsm.lock().unwrap();
            match mode {
                EngineModeIpc::AirPlay => {
                    let _ = fsm.transition(SidecarLifecycleEvent::StreamStarted { target: "AirPlay".into() });
                }
                EngineModeIpc::Idle => {
                    let _ = fsm.transition(SidecarLifecycleEvent::StreamStopped);
                }
                EngineModeIpc::LocalAo { .. } => {
                    let _ = fsm.transition(SidecarLifecycleEvent::StreamStarted { target: "LocalAo".into() });
                }
            }
        }

        res
    }

    /// Disconnect any active AirPlay streaming session.
    pub async fn disconnect(&self) -> Result<()> {
        self.ensure_started(None).await?;
        self.send_request(IpcRequest::Disconnect).await?
            .ok()
            .map_err(|e| anyhow!("{e}"))
    }

    /// Set stream volume for all or a specific target device.
    pub async fn set_volume(&self, volume: f32, device_id: Option<String>) -> Result<()> {
        self.ensure_started(None).await?;
        self.send_request(IpcRequest::SetVolume { volume, device_id }).await?
            .ok()
            .map_err(|e| anyhow!("{e}"))
    }

    /// Set audio equalizer and enhancer settings.
    pub async fn set_eq(&self, enabled: bool, gains: [f32; 10], enhancer_mode: u8, warmth_mode: u8) -> Result<()> {
        self.ensure_started(None).await?;
        self.send_request(IpcRequest::SetEq { enabled, gains, enhancer_mode, warmth_mode }).await?
            .ok()
            .map_err(|e| anyhow!("{e}"))
    }

    /// Trigger mDNS device discovery.
    pub async fn discover_devices(&self, timeout_secs: u64) -> Result<()> {
        self.ensure_started(None).await?;
        self.send_request(IpcRequest::DiscoverDevices { timeout_secs }).await?
            .ok()
            .map_err(|e| anyhow!("{e}"))
    }

    /// Gracefully shutdown sidecar under FSM management.
    pub async fn shutdown(&self) {
        let is_connected_prior = self.is_connected();
        {
            let mut fsm = self.fsm.lock().unwrap();
            let _ = fsm.transition(SidecarLifecycleEvent::ShutdownRequested);
        }

        if is_connected_prior {
            // Send shutdown IPC with a tight timeout to allow graceful clean exit
            let _ = self.send_request_internal(IpcRequest::Shutdown, Duration::from_millis(500)).await;
        }

        {
            let mut writer_lock = self.writer.lock().await;
            *writer_lock = None;
        }
        {
            let mut child_lock = self.child_process.lock().await;
            if let Some(mut child) = child_lock.take() {
                let _ = child.kill();
            }
        }
        #[cfg(windows)]
        {
            let mut jh = self.job_handle.lock().unwrap();
            if let Some(handle) = jh.take() {
                unsafe {
                    windows_sys::Win32::Foundation::CloseHandle(handle as windows_sys::Win32::Foundation::HANDLE);
                }
            }
        }
        {
            let mut fsm = self.fsm.lock().unwrap();
            let _ = fsm.transition(SidecarLifecycleEvent::CleanupComplete);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidecar_client_initialization_and_req_id() {
        let client = SidecarClient::new();
        assert_eq!(client.state(), SidecarLifecycleState::Unstarted);
        assert!(!client.is_connected());
        assert!(!client.is_shutting_down());

        let id1 = client.next_req_id.fetch_add(1, Ordering::SeqCst);
        let id2 = client.next_req_id.fetch_add(1, Ordering::SeqCst);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }
}
