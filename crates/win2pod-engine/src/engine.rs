//! Core coordinator for win2pod-engine.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{error, info, warn};
use win2pod_ipc::protocol::*;

use airplay_core::device::Device;
use airplay_discovery::{Discovery, ServiceBrowser};

pub struct Engine {
    event_tx: mpsc::Sender<IpcEvent>,
    status: Arc<RwLock<EngineStatusIpc>>,
    discovered_cache: Arc<RwLock<HashMap<String, Device>>>,
    active_streamer: Arc<Mutex<Option<crate::airplay_sender::ActiveStreamSession>>>,
}

impl Engine {
    pub fn new(event_tx: mpsc::Sender<IpcEvent>) -> Self {
        Self {
            event_tx,
            status: Arc::new(RwLock::new(EngineStatusIpc::default())),
            discovered_cache: Arc::new(RwLock::new(HashMap::new())),
            active_streamer: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn handle_request(&self, req: IpcRequest) -> Result<IpcResponse> {
        match req {
            IpcRequest::Ping { nonce } => Ok(IpcResponse::Pong { nonce }),
            IpcRequest::GetStatus => {
                let status = self.status.read().await.clone();
                Ok(IpcResponse::Status(status))
            }
            IpcRequest::DiscoverDevices { timeout_secs } => {
                let tx = self.event_tx.clone();
                let cache_arc = self.discovered_cache.clone();
                tokio::spawn(async move {
                    if let Err(e) = Self::run_discovery(timeout_secs, tx, cache_arc).await {
                        error!("Discovery failed: {e:#}");
                    }
                });
                Ok(IpcResponse::Ok)
            }
            IpcRequest::Connect { target, volume, low_latency } => {
                info!("Connect requested for target: {:?}", target);
                let mut session_lock = self.active_streamer.lock().await;
                if let Some(mut existing) = session_lock.take() {
                    existing.stop().await;
                }

                let event_tx = self.event_tx.clone();
                let status_arc = self.status.clone();
                let cache = self.discovered_cache.read().await.clone();

                match crate::airplay_sender::ActiveStreamSession::start(target.clone(), volume, low_latency, &cache, event_tx.clone()).await {
                    Ok(session) => {
                        let target_name = match &target {
                            ConnectTargetIpc::Single(d) => d.name.clone(),
                            ConnectTargetIpc::StereoPair { name, .. } => name.clone(),
                            ConnectTargetIpc::Group { name, .. } => name.clone(),
                        };
                        let dev_ids = match &target {
                            ConnectTargetIpc::Single(d) => vec![d.id.clone()],
                            ConnectTargetIpc::StereoPair { left, right, .. } => vec![left.id.clone(), right.id.clone()],
                            ConnectTargetIpc::Group { devices, .. } => devices.iter().map(|d| d.id.clone()).collect(),
                        };

                        *session_lock = Some(session);

                        {
                            let mut st = status_arc.write().await;
                            st.stream_state = StreamStateIpc::Streaming;
                            st.connected_name = Some(target_name.clone());
                        }

                        let _ = event_tx.send(IpcEvent::StreamingStateChanged {
                            state: StreamStateIpc::Streaming,
                            connected_name: Some(target_name),
                            connected_device_ids: dev_ids,
                            error: None,
                        }).await;

                        Ok(IpcResponse::Ok)
                    }
                    Err(e) => {
                        error!("Failed to start streaming session: {e:#}");
                        let _ = event_tx.send(IpcEvent::StreamingStateChanged {
                            state: StreamStateIpc::Error,
                            connected_name: None,
                            connected_device_ids: vec![],
                            error: Some(e.to_string()),
                        }).await;
                        Ok(IpcResponse::Error(e.to_string()))
                    }
                }
            }
            IpcRequest::Disconnect => {
                info!("Disconnect requested");
                let mut session_lock = self.active_streamer.lock().await;
                if let Some(mut session) = session_lock.take() {
                    session.stop().await;
                }
                {
                    let mut st = self.status.write().await;
                    st.stream_state = StreamStateIpc::Idle;
                    st.connected_name = None;
                }
                let _ = self.event_tx.send(IpcEvent::StreamingStateChanged {
                    state: StreamStateIpc::Idle,
                    connected_name: None,
                    connected_device_ids: vec![],
                    error: None,
                }).await;
                Ok(IpcResponse::Ok)
            }
            IpcRequest::SetVolume { volume, device_id } => {
                let mut session_lock = self.active_streamer.lock().await;
                if let Some(session) = session_lock.as_mut() {
                    session.set_volume(volume, device_id.as_deref()).await?;
                }
                Ok(IpcResponse::Ok)
            }
            IpcRequest::SetEq { enabled, gains, enhancer_mode, warmth_mode } => {
                let session_lock = self.active_streamer.lock().await;
                if let Some(session) = session_lock.as_ref() {
                    session.set_eq(enabled, gains, enhancer_mode, warmth_mode).await;
                }
                Ok(IpcResponse::Ok)
            }
            IpcRequest::SetAudioSource { source } => {
                info!("SetAudioSource: {:?}", source);
                Ok(IpcResponse::Ok)
            }
            IpcRequest::SetAirPlayReceiver { enabled, name, sync_level: _, auto_max_volume: _, output_device: _, apply_effects: _ } => {
                info!("SetAirPlayReceiver: enabled={enabled}, name={name}");
                Ok(IpcResponse::Ok)
            }
            IpcRequest::SetAirPodsMonitoring { enabled } => {
                info!("SetAirPodsMonitoring: {enabled}");
                Ok(IpcResponse::Ok)
            }
            IpcRequest::EnumerateAudioEndpoints => {
                match crate::audio_manager::enumerate_render_endpoints() {
                    Ok(endpoints) => Ok(IpcResponse::AudioEndpoints(endpoints)),
                    Err(e) => Ok(IpcResponse::Error(e.to_string())),
                }
            }
            IpcRequest::SetSpeakerMute { mute, device_id } => {
                info!("SetSpeakerMute: mute={mute}, device_id={:?}", device_id);
                if let Err(e) = crate::audio_manager::set_speaker_mute(mute, device_id.as_deref()) {
                    warn!("Failed to set speaker mute: {e:#}");
                    return Ok(IpcResponse::Error(e.to_string()));
                }
                Ok(IpcResponse::Ok)
            }
            IpcRequest::SetStandinRedirection { standin_id } => {
                info!("SetStandinRedirection: standin_id={:?}", standin_id);
                Ok(IpcResponse::Ok)
            }
            IpcRequest::SetEngineMode { mode } => {
                info!("SetEngineMode: mode={:?}", mode);
                Ok(IpcResponse::Ok)
            }
            IpcRequest::Shutdown => {
                info!("Engine shutdown requested");
                Ok(IpcResponse::Ok)
            }
        }
    }

    async fn run_discovery(
        timeout_secs: u64,
        event_tx: mpsc::Sender<IpcEvent>,
        cache_arc: Arc<RwLock<HashMap<String, Device>>>,
    ) -> Result<()> {
        let browser = ServiceBrowser::new()?;
        let devices = browser.scan(Duration::from_secs(timeout_secs)).await?;
        let mut cache = cache_arc.write().await;

        for d in devices {
            if !d.supports_airplay2() || !d.addresses.iter().any(|a| a.is_ipv4()) {
                continue;
            }
            let mac = d.id.to_mac_string();
            cache.insert(mac.clone(), d.clone());

            let is_stereo = d.tight_sync_id.is_some() && d.group_public_name.is_some();
            let stereo_role = if is_stereo {
                if d.is_group_leader {
                    Some("leader".into())
                } else {
                    Some("follower".into())
                }
            } else {
                None
            };

            let ipc_dev = DeviceIpc {
                id: mac,
                name: d.name.clone(),
                model: d.model.clone(),
                addresses: d.addresses.iter().map(|a| a.to_string()).collect(),
                port: d.port,
                is_stereo_pair: is_stereo,
                is_group: false,
                stereo_role,
            };
            let _ = event_tx.send(IpcEvent::DeviceDiscovered(ipc_dev)).await;
        }
        Ok(())
    }
}
