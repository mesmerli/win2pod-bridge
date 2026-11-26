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
    connections: Vec<Connection>,
    running: Arc<AtomicBool>,
    capture_thread: Option<JoinHandle<()>>,
    streamer: Option<AudioStreamer>,
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

            let (sender, decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 32);
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

            Ok(Self {
                connections,
                running,
                capture_thread: Some(capture_thread),
                streamer: Some(streamer),
            })
        } else {
            // Single device streaming
            let dev = core_devices.into_iter().next().unwrap();
            info!("Starting single device streaming to {} ({:?})", dev.name, dev.addresses);
            let mut conn = Connection::connect_auto(dev, config, "3939").await?;
            conn.setup().await?;
            let _ = conn.set_volume(volume).await;

            let (sender, decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 32);
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

            Ok(Self {
                connections: vec![conn],
                running,
                capture_thread: Some(capture_thread),
                streamer: None,
            })
        }
    }

    pub async fn set_volume(&mut self, volume: f32, device_id: Option<&str>) -> Result<()> {
        for conn in &mut self.connections {
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
        if let Some(handle) = self.capture_thread.take() {
            let _ = handle.join();
        }
        if let Some(mut streamer) = self.streamer.take() {
            let _ = streamer.stop().await;
        }
        for conn in &mut self.connections {
            let _ = conn.disconnect().await;
        }
        info!("Streaming session stopped successfully");
    }

    fn capture_loop(stop: Arc<AtomicBool>, sender: LiveFrameSender) -> Result<()> {
        let _com = initialize_mta();
        let enumerator = DeviceEnumerator::new()?;
        let device = enumerator.get_default_device(&Direction::Render)?;
        let mut audio_client = device.get_iaudioclient()?;

        let format = WaveFormat::new(32, 32, &SampleType::Float, RATE as usize, CHANNELS as usize, None);
        let (_default_period, min_period) = audio_client.get_periods()?;
        let mode = StreamMode::EventsShared {
            autoconvert: true,
            buffer_duration_hns: min_period,
        };
        audio_client.initialize_client(&format, &Direction::Capture, &mode)?;

        let h_event = audio_client.set_get_eventhandle()?;
        let capture_client = audio_client.get_audiocaptureclient()?;
        let mut queue: VecDeque<u8> = VecDeque::new();
        audio_client.start_stream()?;
        info!("Sidecar loopback capture started ({RATE} Hz, {CHANNELS}ch, f32)");

        let bytes_per_frame = CHANNELS as usize * 4; // f32 per sample
        let silence = vec![0i16; (RATE as usize / 20) * CHANNELS as usize];

        while !stop.load(Ordering::Relaxed) {
            capture_client.read_from_device_to_deque(&mut queue)?;

            let n_frames = queue.len() / bytes_per_frame;
            if n_frames > 0 {
                let n_samples = n_frames * CHANNELS as usize;
                let mut samples: Vec<i16> = Vec::with_capacity(n_samples);
                for _ in 0..n_samples {
                    let b = [
                        queue.pop_front().unwrap(),
                        queue.pop_front().unwrap(),
                        queue.pop_front().unwrap(),
                        queue.pop_front().unwrap(),
                    ];
                    let f = f32::from_le_bytes(b);
                    samples.push((f.clamp(-1.0, 1.0) * 32767.0) as i16);
                }
                sender.try_send(LivePcmFrame {
                    samples,
                    channels: CHANNELS,
                    sample_rate: RATE,
                });
            } else {
                sender.try_send(LivePcmFrame {
                    samples: silence.clone(),
                    channels: CHANNELS,
                    sample_rate: RATE,
                });
            }

            let _ = h_event.wait_for_event(50);
        }

        let _ = audio_client.stop_stream();
        Ok(())
    }
}
