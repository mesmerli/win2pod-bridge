//! AirPlay 2 Sender implementation for win2pod-engine.

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use win2pod_ipc::protocol::{ConnectTargetIpc, DeviceIpc, IpcEvent};

use airplay_audio::{AlacEncoder, AudioStreamer, LiveAudioDecoder, LiveFrameSender, LivePcmFrame};
use airplay_client::Connection;
use airplay_core::device::{Device, DeviceId};
use airplay_core::features::Features;
use airplay_core::stream::{PtpMode, StreamType, TimingProtocol};
use airplay_core::{AudioCodec, AudioFormat, StreamConfig};
use airplay_discovery::{Discovery, ServiceBrowser};
use wasapi::{initialize_mta, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

const RATE: u32 = 44_100;
const CHANNELS: u8 = 2;

fn to_core_device(ipc: &DeviceIpc) -> Result<Device> {
    let id = DeviceId::from_mac_string(&ipc.id)
        .map_err(|e| anyhow!("Invalid MAC address {}: {e}", ipc.id))?;
    let addresses: Vec<IpAddr> = ipc.addresses.iter()
        .filter_map(|s| s.parse::<IpAddr>().ok())
        .collect();
    if addresses.is_empty() {
        return Err(anyhow!("No valid IP addresses provided for device {}", ipc.name));
    }
    let port = if ipc.port == 0 { 7000 } else { ipc.port };
    let features = Features::from_txt_value("0x4A7FCA00,0x3C354BD0")
        .unwrap_or_default();

    let is_leader = ipc.stereo_role.as_deref() == Some("leader");
    let tight_sync_id = if ipc.is_stereo_pair {
        Some(uuid::Uuid::nil())
    } else {
        None
    };

    Ok(Device {
        id,
        name: ipc.name.clone(),
        model: if ipc.model.is_empty() { "AudioAccessory5,1".to_string() } else { ipc.model.clone() },
        manufacturer: None,
        serial_number: None,
        addresses,
        port,
        features,
        required_sender_features: None,
        public_key: None,
        source_version: Default::default(),
        firmware_version: None,
        os_version: None,
        protocol_version: None,
        requires_password: false,
        status_flags: 0,
        access_control: None,
        pairing_identity: None,
        system_pairing_identity: None,
        bluetooth_address: None,
        homekit_home_id: None,
        group_id: None,
        is_group_leader: is_leader,
        group_public_name: if ipc.is_stereo_pair { Some(ipc.name.clone()) } else { None },
        group_contains_discoverable_leader: false,
        home_group_id: None,
        household_id: None,
        parent_group_id: None,
        parent_group_contains_discoverable_leader: false,
        tight_sync_id,
        raop_port: None,
        raop_encryption_types: None,
        raop_codecs: None,
        raop_transport: None,
        raop_metadata_types: None,
        raop_digest_auth: false,
        vodka_version: None,
    })
}

pub struct ActiveStreamSession {
    connections: Arc<tokio::sync::Mutex<Vec<Connection>>>,
    running: Arc<AtomicBool>,
    capture_thread: Option<JoinHandle<()>>,
    streamer: Option<AudioStreamer>,
    feedback_task: Option<tokio::task::JoinHandle<()>>,
}

impl ActiveStreamSession {
    pub async fn start(
        target: ConnectTargetIpc,
        volume: f32,
        low_latency: bool,
        cache: &HashMap<String, Device>,
        _event_tx: mpsc::Sender<IpcEvent>,
    ) -> Result<Self> {
        let (latency_min, latency_max) = if low_latency {
            (11025, 44100) // ~250ms at 44.1kHz
        } else {
            (22050, 88200) // ~500ms at 44.1kHz
        };

        let audio_format = AudioFormat::default();
        let asc = if audio_format.codec == AudioCodec::Alac {
            Some(AlacEncoder::new(audio_format.clone())?.magic_cookie())
        } else {
            None
        };

        let target_ipc_devices: Vec<DeviceIpc> = match target {
            ConnectTargetIpc::Single(dev) => vec![dev],
            ConnectTargetIpc::StereoPair { left, right, .. } => vec![left, right],
            ConnectTargetIpc::Group { devices, .. } => devices,
        };

        if target_ipc_devices.is_empty() {
            return Err(anyhow!("No target devices provided"));
        }

        // Resolve core Device objects from cache or reconstruct directly from IPC DTO
        let mut core_devices = Vec::new();
        for ipc_d in &target_ipc_devices {
            if let Some(dev) = cache.get(&ipc_d.id) {
                core_devices.push(dev.clone());
            } else {
                match to_core_device(ipc_d) {
                    Ok(dev) => {
                        info!("Reconstructed device from IPC metadata: {} ({:?})", dev.name, dev.addresses);
                        core_devices.push(dev);
                    }
                    Err(e) => {
                        warn!("Could not reconstruct device from IPC metadata: {e:#}. Trying quick scan...");
                        let browser = ServiceBrowser::new()?;
                        let scanned = browser.scan(Duration::from_secs(2)).await?;
                        if let Some(found) = scanned.into_iter().find(|d| d.id.to_mac_string().eq_ignore_ascii_case(&ipc_d.id)) {
                            core_devices.push(found);
                        } else {
                            return Err(anyhow!("Target device {} ({}) could not be resolved on the network", ipc_d.name, ipc_d.id));
                        }
                    }
                }
            }
        }

        // Sort so group leader comes first
        core_devices.sort_by_key(|d| {
            if d.is_group_leader {
                0
            } else if d.tight_sync_id.is_some() || d.group_public_name.is_some() {
                1
            } else if d.group_id.is_some() {
                2
            } else {
                3
            }
        });

        let is_group = core_devices.len() > 1;

        let (timing_proto, ptp_m) = if is_group {
            (TimingProtocol::Ptp, PtpMode::Master)
        } else {
            (TimingProtocol::Ntp, PtpMode::Master)
        };

        let config = StreamConfig {
            stream_type: StreamType::Realtime,
            audio_format,
            timing_protocol: timing_proto,
            ptp_mode: ptp_m,
            latency_min,
            latency_max,
            supports_dynamic_stream_id: true,
            asc,
        };

        if is_group {
            info!("Starting group streaming to {} devices", core_devices.len());
            let mut connections = Vec::new();
            for dev in &core_devices {
                let conn = Connection::connect_auto(dev.clone(), config.clone(), "3939").await
                    .with_context(|| format!("Failed to connect to {}", dev.name))?;
                connections.push(conn);
            }

            let mut peer_addresses: Vec<String> = core_devices
                .iter()
                .flat_map(|d| d.addresses.iter().find(|a| a.is_ipv4()).map(|a| a.to_string()))
                .collect();
            if let Some(local_ip) = connections[0].local_addr().map(|sa| sa.ip()) {
                peer_addresses.push(local_ip.to_string());
            }

            connections[0].setup().await?;
            connections[0].send_setpeers(&peer_addresses).await?;

            let ptp_clock_id = connections[0]
                .ptp_master_clock_id()
                .ok_or_else(|| anyhow!("Primary device failed to acquire PTP master clock ID"))?;
            let timing_offset = connections[0]
                .timing_offset()
                .ok_or_else(|| anyhow!("Primary device failed to acquire clock offset"))?;
            let timing_rx = connections[0]
                .timing_rx()
                .ok_or_else(|| anyhow!("Primary device failed to acquire timing channel"))?;

            for i in 1..connections.len() {
                let rx_clone = timing_rx.clone();
                connections[i].setup_for_group(ptp_clock_id, timing_offset, rx_clone).await?;
                connections[i].send_setpeers(&peer_addresses).await?;
            }

            for conn in &mut connections {
                let _ = conn.set_volume(volume).await;
                let _ = conn.send_flush(0, 0).await;
            }

            let mut senders = Vec::new();
            for conn in &connections {
                senders.push(conn.build_rtp_sender()?);
            }

            let mut streamer = AudioStreamer::new(config.clone());
            streamer.set_rtp_senders(senders).await;
            streamer.set_timing_offset(timing_offset).await;
            streamer.set_timing_updates(timing_rx).await;
            streamer.set_ptp_sync_mode(ptp_clock_id).await;

            let (sender, decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 256);
            let running = Arc::new(AtomicBool::new(true));
            let running_clone = running.clone();

            let capture_thread = std::thread::Builder::new()
                .name("win2pod-engine-capture".into())
                .spawn(move || {
                    if let Err(e) = Self::capture_loop(running_clone, sender) {
                        error!("WASAPI loopback capture error: {e:#}");
                    }
                })
                .context("Failed to spawn WASAPI capture thread")?;

            if let Err(e) = streamer.start_live(decoder).await {
                running.store(false, Ordering::Relaxed);
                let _ = capture_thread.join();
                for mut c in connections {
                    let _ = c.disconnect().await;
                }
                return Err(e.into());
            }

            let connections = Arc::new(tokio::sync::Mutex::new(connections));
            let conns_clone = connections.clone();
            let running_feedback = running.clone();

            let feedback_task = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(2));
                interval.tick().await;
                while running_feedback.load(Ordering::Relaxed) {
                    interval.tick().await;
                    if !running_feedback.load(Ordering::Relaxed) {
                        break;
                    }
                    let mut conns = conns_clone.lock().await;
                    for c in conns.iter_mut() {
                        if let Err(e) = c.send_feedback().await {
                            warn!("AirPlay 2 feedback keepalive failed for {}: {e:#}", c.device().name);
                        }
                    }
                }
            });

            Ok(Self {
                connections,
                running,
                capture_thread: Some(capture_thread),
                streamer: Some(streamer),
                feedback_task: Some(feedback_task),
            })
        } else {
            // Single device streaming
            let dev = core_devices.into_iter().next().unwrap();
            info!("Starting single device streaming to {} ({:?})", dev.name, dev.addresses);
            let mut conn = Connection::connect_auto(dev, config, "3939").await?;
            conn.setup().await?;
            let _ = conn.set_volume(volume).await;

            let (sender, decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 256);
            let running = Arc::new(AtomicBool::new(true));
            let running_clone = running.clone();

            let capture_thread = std::thread::Builder::new()
                .name("win2pod-engine-capture".into())
                .spawn(move || {
                    if let Err(e) = Self::capture_loop(running_clone, sender) {
                        error!("WASAPI loopback capture error: {e:#}");
                    }
                })
                .context("Failed to spawn WASAPI capture thread")?;

            if let Err(e) = conn.start_streaming_live(decoder).await {
                running.store(false, Ordering::Relaxed);
                let _ = capture_thread.join();
                let _ = conn.disconnect().await;
                return Err(e.into());
            }

            let connections = Arc::new(tokio::sync::Mutex::new(vec![conn]));
            let conns_clone = connections.clone();
            let running_feedback = running.clone();

            let feedback_task = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(2));
                interval.tick().await;
                while running_feedback.load(Ordering::Relaxed) {
                    interval.tick().await;
                    if !running_feedback.load(Ordering::Relaxed) {
                        break;
                    }
                    let mut conns = conns_clone.lock().await;
                    for c in conns.iter_mut() {
                        if let Err(e) = c.send_feedback().await {
                            warn!("AirPlay 2 feedback keepalive failed for {}: {e:#}", c.device().name);
                        }
                    }
                }
            });

            Ok(Self {
                connections,
                running,
                capture_thread: Some(capture_thread),
                streamer: None,
                feedback_task: Some(feedback_task),
            })
        }
    }

    pub async fn set_volume(&mut self, volume: f32, device_id: Option<&str>) -> Result<()> {
        let mut conns = self.connections.lock().await;
        for conn in conns.iter_mut() {
            if let Some(id) = device_id {
                if !conn.device().id.to_mac_string().eq_ignore_ascii_case(id) {
                    continue;
                }
            }
            let _ = conn.set_volume(volume).await;
        }
        Ok(())
    }

    pub async fn set_eq(&self, _enabled: bool, _gains: [f32; 10], _enhancer_mode: u8, _warmth_mode: u8) {
        // Can be extended to forward EQ params to connections or streamer
    }

    pub async fn stop(&mut self) {
        info!("Stopping active streaming session...");
        self.running.store(false, Ordering::SeqCst);
        if let Some(task) = self.feedback_task.take() {
            task.abort();
        }
        if let Some(handle) = self.capture_thread.take() {
            let _ = handle.join();
        }
        if let Some(mut streamer) = self.streamer.take() {
            let _ = streamer.stop().await;
        }
        let mut conns = self.connections.lock().await;
        for mut conn in conns.drain(..) {
            let _ = conn.disconnect().await;
        }
        info!("Streaming session stopped successfully");
    }

    fn capture_loop(stop: Arc<AtomicBool>, sender: LiveFrameSender) -> Result<()> {
        let _com = initialize_mta();
        let enumerator = DeviceEnumerator::new()?;
        let bytes_per_frame = CHANNELS as usize * 4; // f32 per sample
        let silence = vec![0i16; (RATE as usize / 20) * CHANNELS as usize];

        while !stop.load(Ordering::Relaxed) {
            let device = match enumerator.get_default_device(&Direction::Render) {
                Ok(d) => d,
                Err(e) => {
                    warn!("Sidecar failed to get default render device: {e:#}, retrying in 300ms");
                    let _ = sender.try_send(LivePcmFrame {
                        samples: silence.clone(),
                        channels: CHANNELS,
                        sample_rate: RATE,
                    });
                    std::thread::sleep(Duration::from_millis(300));
                    continue;
                }
            };

            if let Ok(name) = device.get_friendlyname() {
                info!("Sidecar capturing from default render endpoint: {name}");
            }

            let mut audio_client = match device.get_iaudioclient() {
                Ok(c) => c,
                Err(e) => {
                    warn!("Sidecar failed to get IAudioClient: {e:#}, retrying in 300ms");
                    std::thread::sleep(Duration::from_millis(300));
                    continue;
                }
            };

            let format = WaveFormat::new(32, 32, &SampleType::Float, RATE as usize, CHANNELS as usize, None);
            let (_default_period, min_period) = match audio_client.get_device_period() {
                Ok(p) => p,
                Err(_) => (100_000, 100_000),
            };
            let mode = StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: min_period,
            };

            if let Err(e) = audio_client.initialize_client(&format, &Direction::Capture, &mode) {
                warn!("Sidecar failed to initialize audio client: {e:#}, retrying in 300ms");
                std::thread::sleep(Duration::from_millis(300));
                continue;
            }

            let h_event = match audio_client.set_get_eventhandle() {
                Ok(h) => h,
                Err(e) => {
                    warn!("Sidecar failed to set event handle: {e:#}, retrying in 300ms");
                    std::thread::sleep(Duration::from_millis(300));
                    continue;
                }
            };

            let capture_client = match audio_client.get_audiocaptureclient() {
                Ok(c) => c,
                Err(e) => {
                    warn!("Sidecar failed to get capture client: {e:#}, retrying in 300ms");
                    std::thread::sleep(Duration::from_millis(300));
                    continue;
                }
            };

            let mut queue: VecDeque<u8> = VecDeque::new();
            if let Err(e) = audio_client.start_stream() {
                warn!("Sidecar failed to start stream: {e:#}, retrying in 300ms");
                std::thread::sleep(Duration::from_millis(300));
                continue;
            }

            info!("Sidecar loopback capture stream started ({RATE} Hz, {CHANNELS}ch, f32)");

            while !stop.load(Ordering::Relaxed) {
                match capture_client.read_from_device_to_deque(&mut queue) {
                    Ok(_) => {
                        let n_frames = queue.len() / bytes_per_frame;
                        if n_frames > 0 {
                            let n_samples = n_frames * CHANNELS as usize;
                            let n_bytes = n_frames * bytes_per_frame;
                            let mut samples: Vec<i16> = Vec::with_capacity(n_samples);
                            let slice = &queue.make_contiguous()[..n_bytes];
                            for chunk in slice.chunks_exact(4) {
                                let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                                let s = (f.clamp(-1.0, 1.0) * 32767.0) as i16;
                                samples.push(s);
                            }
                            queue.drain(..n_bytes);
                            let _ = sender.try_send(LivePcmFrame {
                                samples,
                                channels: CHANNELS,
                                sample_rate: RATE,
                            });
                        } else {
                            let _ = sender.try_send(LivePcmFrame {
                                samples: silence.clone(),
                                channels: CHANNELS,
                                sample_rate: RATE,
                            });
                        }
                        let _ = h_event.wait_for_event(50);
                    }
                    Err(e) => {
                        warn!("Sidecar WASAPI capture read error/reset: {e:#}, recycling capture endpoint in 300ms...");
                        let _ = sender.try_send(LivePcmFrame {
                            samples: silence.clone(),
                            channels: CHANNELS,
                            sample_rate: RATE,
                        });
                        std::thread::sleep(Duration::from_millis(300));
                        break;
                    }
                }
            }

            let _ = audio_client.stop_stream();
        }

        info!("Sidecar WASAPI capture thread exiting cleanly");
        Ok(())
    }
}
