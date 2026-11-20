//! AirPlay connection session state.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Session state of an active AirPlay connection.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub aes_key: Option<[u8; 16]>,
    pub aes_iv: Option<[u8; 16]>,
    pub volume_db: f32,
    pub client_name: Option<String>,
    pub dacp_id: Option<String>,
    pub active_remote: Option<String>,
    pub dacp_port: Option<u16>,
    pub is_recording: bool,
    pub remote_addr: Option<SocketAddr>,
    pub ignore_volume_control: bool,
    pub active_tasks: Arc<std::sync::Mutex<Vec<JoinHandle<()>>>>,
    pub last_dacp_target: Option<(std::net::IpAddr, Option<u16>, String, Option<String>)>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            aes_key: None,
            aes_iv: None,
            volume_db: 0.0, // Default to 0.0 dB (100%) so reconnect doesn't force iPhone volume to 50%
            client_name: None,
            dacp_id: None,
            active_remote: None,
            dacp_port: None,
            is_recording: false,
            remote_addr: None,
            ignore_volume_control: false,
            active_tasks: Arc::new(std::sync::Mutex::new(Vec::new())),
            last_dacp_target: None,
        }
    }
}

impl SessionState {
    pub fn cache_dacp_target(&mut self) {
        if let (Some(addr), Some(ref ar)) = (self.remote_addr, &self.active_remote) {
            self.last_dacp_target = Some((addr.ip(), self.dacp_port, ar.clone(), self.dacp_id.clone()));
        }
    }

    pub fn set_dacp_port(&mut self, port: u16) {
        self.dacp_port = Some(port);
        if let Some((_, ref mut p, _, _)) = self.last_dacp_target {
            *p = Some(port);
        }
    }

    pub fn reset(&mut self) {
        self.cache_dacp_target();
        self.aes_key = None;
        self.aes_iv = None;
        self.is_recording = false;
        self.client_name = None;
        self.remote_addr = None;
        self.dacp_id = None;
        self.active_remote = None;
        self.dacp_port = None;
        self.abort_udp_tasks();
    }

    pub fn abort_udp_tasks(&self) {
        if let Ok(mut tasks) = self.active_tasks.lock() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
    }
}
