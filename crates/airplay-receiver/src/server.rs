//! AirPlay Receiver Server Coordinator.

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::config::AirPlayReceiverConfig;
use crate::mdns::MdnsAdvertiser;
use crate::rtsp::handle_rtsp_connection;
use crate::session::SessionState;
use crate::sink::AudioSink;

/// Pure Rust AirPlay 1 RAOP Audio Receiver Server.
pub struct AirPlayReceiver {
    config: AirPlayReceiverConfig,
    sink: Arc<dyn AudioSink>,
    session: Arc<Mutex<SessionState>>,
    stop_signal: Arc<AtomicBool>,
    server_handle: Option<JoinHandle<()>>,
    active_connections: Arc<Mutex<Vec<JoinHandle<()>>>>,
    mdns: Option<MdnsAdvertiser>,
}

impl AirPlayReceiver {
    pub fn new(config: AirPlayReceiverConfig, sink: Arc<dyn AudioSink>) -> Self {
        let ignore_vol = config.ignore_volume_control;
        let mut session_state = SessionState::default();
        session_state.ignore_volume_control = ignore_vol;
        Self {
            config,
            sink,
            session: Arc::new(Mutex::new(session_state)),
            stop_signal: Arc::new(AtomicBool::new(false)),
            server_handle: None,
            active_connections: Arc::new(Mutex::new(Vec::new())),
            mdns: None,
        }
    }

    /// Dynamically update whether client negative attenuation is ignored.
    pub async fn set_ignore_volume_control(&self, ignore: bool) {
        self.session.lock().await.ignore_volume_control = ignore;
    }

    /// Start the AirPlay receiver (mDNS announcement + RTSP server).
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.stop_signal.store(false, Ordering::Relaxed);

        // 1. Detect local LAN IP
        let local_ip = match self.config.local_ip {
            Some(ip) => ip,
            None => get_local_ip().unwrap_or_else(|| "127.0.0.1".parse().unwrap()),
        };

        let local_ip_bytes: Vec<u8> = match local_ip {
            IpAddr::V4(v4) => v4.octets().to_vec(),
            IpAddr::V6(v6) => v6.octets().to_vec(),
        };

        // 2. Start mDNS advertisement
        let mdns = MdnsAdvertiser::start(&self.config, local_ip)?;
        self.mdns = Some(mdns);

        // 3. Bind RTSP listener
        let addr = SocketAddr::new("0.0.0.0".parse().unwrap(), self.config.rtsp_port);
        let listener = TcpListener::bind(addr).await?;
        tracing::info!("AirPlay RTSP Server listening on {}", addr);

        let stop_signal = Arc::clone(&self.stop_signal);
        let session = Arc::clone(&self.session);
        let sink = Arc::clone(&self.sink);
        let config = self.config.clone();
        let active_conns = Arc::clone(&self.active_connections);
        let active_conn_count = Arc::new(AtomicUsize::new(0));

        let handle = tokio::spawn(async move {
            loop {
                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }

                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((socket, remote_addr)) => {
                                active_conn_count.fetch_add(1, Ordering::SeqCst);
                                tracing::info!("Accepted AirPlay connection from {}", remote_addr);
                                let session_clone = Arc::clone(&session);
                                let sink_clone = Arc::clone(&sink);
                                let config_clone = config.clone();
                                let ip_bytes_clone = local_ip_bytes.clone();
                                let stop_clone = Arc::clone(&stop_signal);
                                let active_conn_count_clone = Arc::clone(&active_conn_count);

                                let conn_h = tokio::spawn(async move {
                                    if let Err(e) = handle_rtsp_connection(
                                        socket,
                                        remote_addr,
                                        config_clone,
                                        ip_bytes_clone,
                                        session_clone,
                                        sink_clone,
                                        stop_clone,
                                        active_conn_count_clone,
                                    ).await {
                                        tracing::warn!("AirPlay RTSP connection closed with: {}", e);
                                    }
                                });

                                let mut conns = active_conns.lock().await;
                                conns.retain(|h| !h.is_finished());
                                conns.push(conn_h);
                            }
                            Err(e) => {
                                tracing::warn!("RTSP accept error: {}", e);
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                        if stop_signal.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
            }
        });

        self.server_handle = Some(handle);
        Ok(())
    }

    /// Stop the receiver and unregister mDNS.
    pub async fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);

        // 1. Explicitly unregister mDNS and broadcast Goodbye packets (TTL=0)
        if let Some(mut mdns) = self.mdns.take() {
            mdns.stop();
        }

        // 2. Abort main accept listener so no new incoming connections are accepted
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }

        // 3. Give existing connections a brief moment (200ms) to send disconnection headers and finish gracefully
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // 4. Clean up any remaining connection tasks
        {
            let mut conns = self.active_connections.lock().await;
            for handle in conns.drain(..) {
                handle.abort();
            }
        }

        // 4. Abort UDP streaming tasks and reset session state
        {
            let mut state = self.session.lock().await;
            state.abort_udp_tasks();
            state.reset();
        }

        // 5. Notify sink that stream has stopped
        self.sink.on_stream_stop();
        tracing::info!("AirPlayReceiver stopped cleanly");
    }

    pub fn is_running(&self) -> bool {
        self.server_handle.as_ref().map_or(false, |h| !h.is_finished())
    }

    pub fn config(&self) -> &AirPlayReceiverConfig {
        &self.config
    }

    /// Send a DACP playback command to the connected AirPlay client or last cached client.
    /// Returns true if command was dispatched to a known target IP & Active-Remote.
    pub async fn send_dacp(&self, cmd: crate::dacp::DacpCommand) -> bool {
        let (ip, port, active_remote, dacp_id) = {
            let st = self.session.lock().await;
            if let (Some(addr), Some(ref ar)) = (st.remote_addr, &st.active_remote) {
                (Some(addr.ip()), st.dacp_port, Some(ar.clone()), st.dacp_id.clone())
            } else if let Some((ip, port, ref ar, ref did)) = st.last_dacp_target {
                (Some(ip), port, Some(ar.clone()), did.clone())
            } else {
                (None, None, None, None)
            }
        };
        let session_clone = Arc::clone(&self.session);
        if let (Some(ip), Some(active_remote)) = (ip, active_remote) {
            tokio::spawn(async move {
                let target_port = if let Some(p) = port {
                    p
                } else {
                    tracing::info!("[DACP] Dynamic port not yet cached, querying mDNS for iPhone _dacp._tcp...");
                    let (_, resolved_port) = crate::dacp::resolve_dacp_device(ip, dacp_id).await;
                    if let Some(p) = resolved_port {
                        session_clone.lock().await.set_dacp_port(p);
                        p
                    } else {
                        tracing::warn!("[DACP] Could not discover iPhone DACP dynamic port via mDNS. Command dropped.");
                        return;
                    }
                };

                if let Err(e) = crate::dacp::send_dacp_command(ip, target_port, &active_remote, &cmd).await {
                    tracing::warn!("[DACP] Command failed on dynamic port {}: {}", target_port, e);
                }
            });
            true
        } else {
            tracing::warn!("[DACP] Cannot send command: client IP or Active-Remote missing (ip={:?})", ip);
            false
        }
    }

    pub async fn has_cached_client(&self) -> bool {
        self.session.lock().await.last_dacp_target.is_some()
    }
}

impl Drop for AirPlayReceiver {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(mut mdns) = self.mdns.take() {
            mdns.stop();
        }
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}

fn get_local_ip() -> Option<IpAddr> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip())
}
