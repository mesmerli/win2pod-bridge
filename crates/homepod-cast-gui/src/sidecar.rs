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
use win2pod_ipc::transport::{connect_pipe_client, PipeTransport};

pub struct SidecarClient {
    transport: Arc<Mutex<Option<PipeTransport<tokio::net::windows::named_pipe::NamedPipeClient>>>>,
    child_process: Arc<Mutex<Option<Child>>>,
    is_connected: Arc<AtomicBool>,
}

impl SidecarClient {
    pub fn new() -> Self {
        Self {
            transport: Arc::new(Mutex::new(None)),
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

    /// Start the sidecar engine process and bind it to a Windows Job Object.
    pub async fn ensure_started(&self, pipe_name: Option<&str>) -> Result<()> {
        if self.is_connected() {
            return Ok(());
        }

        let name = pipe_name.unwrap_or(DEFAULT_PIPE_NAME);

        // First attempt a fast connection in case it's already running
        if let Ok(client) = connect_pipe_client(Some(name), 1, Duration::from_millis(100)).await {
            info!("Connected to existing win2pod-engine daemon on {name}");
            let mut trans_lock = self.transport.lock().await;
            *trans_lock = Some(PipeTransport::new(client));
            self.is_connected.store(true, Ordering::SeqCst);
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

        let mut trans_lock = self.transport.lock().await;
        *trans_lock = Some(PipeTransport::new(client));
        self.is_connected.store(true, Ordering::SeqCst);
        info!("Successfully established IPC link with win2pod-engine");

        Ok(())
    }

    /// Send a request to the sidecar and receive its response.
    pub async fn send_request(&self, req: IpcRequest) -> Result<IpcResponse> {
        let mut trans_lock = self.transport.lock().await;
        let transport = trans_lock.as_mut()
            .ok_or_else(|| anyhow!("Sidecar not connected"))?;

        transport.write_message(&IpcMessage::Request(req)).await?;

        // Read until we get the corresponding response
        while let Some(msg) = transport.read_message().await? {
            match msg {
                IpcMessage::Response(resp) => return Ok(resp),
                IpcMessage::Event(evt) => {
                    info!("Received asynchronous event during request wait: {:?}", evt);
                }
                _ => {}
            }
        }

        Err(anyhow!("Pipe closed before response received"))
    }

    /// Start listening for async events in the background.
    pub fn start_event_listener(
        &self,
        event_sender: mpsc::Sender<IpcEvent>,
    ) {
        let is_connected = self.is_connected.clone();
        let transport_arc = self.transport.clone();

        tokio::spawn(async move {
            info!("Sidecar event listener loop started");
            loop {
                if !is_connected.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }

                let mut trans_lock = transport_arc.lock().await;
                if let Some(transport) = trans_lock.as_mut() {
                    match transport.read_message().await {
                        Ok(Some(IpcMessage::Event(evt))) => {
                            let _ = event_sender.send(evt).await;
                        }
                        Ok(Some(_)) => {}
                        Ok(None) => {
                            warn!("Sidecar pipe closed");
                            is_connected.store(false, Ordering::SeqCst);
                        }
                        Err(e) => {
                            warn!("Error reading from sidecar: {e:#}");
                            is_connected.store(false, Ordering::SeqCst);
                        }
                    }
                } else {
                    drop(trans_lock);
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        });
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    /// Gracefully shutdown sidecar.
    pub async fn shutdown(&self) {
        if self.is_connected() {
            let _ = self.send_request(IpcRequest::Shutdown).await;
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
