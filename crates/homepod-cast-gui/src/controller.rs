//! Backend controller for HomePod-Cast GUI.
//! Handles discovery, connection, streaming session, WASAPI loopback capture, and diagnostics.

use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use airplay_audio::{
    AlacEncoder, AudioStreamer, LiveAudioDecoder, LiveFrameSender, LivePcmFrame, RetransmitRequest,
};
use airplay_client::Connection;
use airplay_core::device::Device;
use airplay_core::stream::{PtpMode, StreamType, TimingProtocol};
use airplay_core::{AudioCodec, AudioFormat, StreamConfig};
use airplay_discovery::{Discovery, ServiceBrowser};
use async_channel::Receiver as AsyncReceiver;
use crate::i18n::{Language, NoticeKind};
use wasapi::{initialize_mta, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

use windows::Win32::Foundation::PROPERTYKEY;

pub const RATE: u32 = 44_100;
pub const CHANNELS: u8 = 2;
pub const DEFAULT_VOLUME: f32 = 0.25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDeviceItem {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceStatus {
    Idle,
    Connecting,
    Streaming,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct DeviceItem {
    pub name: String,
    pub ip: IpAddr,
    pub model: String,
    pub raw_device: Device,
    pub member_devices: Vec<Device>,
    pub is_stereo_pair: bool,
    pub pair_count: usize,
}

impl DeviceItem {
    pub fn id_string(&self) -> String {
        self.raw_device.id.to_mac_string()
    }
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct StreamDiagnostics {
    pub sample_rate: u32,
    pub channels: u8,
    pub frames_captured: u64,
    pub packets_sent: u64,
    pub buffer_occupancy: usize,
    pub underruns: u64,
    pub timing_mode: String,
}

#[derive(Debug, Clone)]
pub struct ControllerState {
    pub devices: Vec<DeviceItem>,
    pub status: DeviceStatus,
    pub connected_name: Option<String>,
    pub connected_device_ids: Vec<String>,
    pub volume: f32,
    pub is_muted: bool,
    pub audio_devices: Vec<AudioDeviceItem>,
    pub selected_audio_device: String,
    pub diagnostics: StreamDiagnostics,
    pub auto_mute_speakers: bool,
    pub prevent_sleep: bool,
    pub low_latency: bool,
    pub auto_connect: bool,
    pub last_connected_name: Option<String>,
    pub device_volumes: std::collections::HashMap<String, f32>,
    pub is_audio_active: bool,
    pub notice_message: Option<(NoticeKind, std::time::Instant)>,
    pub hotkey: (u32, u32),
    pub language: Language,
    pub autostart: bool,
    pub license_tier: crate::license::LicenseTier,
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            status: DeviceStatus::Idle,
            connected_name: None,
            connected_device_ids: Vec::new(),
            volume: load_volume(),
            is_muted: false,
            audio_devices: vec![AudioDeviceItem {
                id: "default".to_string(),
                name: "System Audio (Default Output Device)".to_string(),
            }],
            selected_audio_device: load_audio_device(),
            diagnostics: StreamDiagnostics::default(),
            auto_mute_speakers: load_auto_mute(),
            prevent_sleep: load_prevent_sleep(),
            low_latency: load_low_latency(),
            auto_connect: load_auto_connect(),
            last_connected_name: load_last_connected_device().map(|t| t.name),
            device_volumes: std::collections::HashMap::new(),
            is_audio_active: false,
            notice_message: None,
            hotkey: load_hotkey(),
            language: load_language(),
            autostart: crate::autostart::is_autostart_enabled(),
            license_tier: crate::license::LicenseTier::default(),
        }
    }
}

pub enum GuiCmd {
    RefreshDevices,
    RefreshAudioDevices,
    SetAudioDevice(String),
    Connect(Vec<DeviceItem>),
    Disconnect,
    SetVolume(f32),
    SetDeviceVolume(String, f32),
    SetMute(bool),
    SetAutoMuteSpeakers(bool),
    UpdateLicenseTier(crate::license::LicenseTier),
    SetPreventSleep(bool),
    SetLowLatency(bool),
    SetAutoConnect(bool),
    SetAutostart(bool),
    SetHotkey(u32, u32),
    SetLanguage(Language),
    SystemSuspend,
    SystemResume,
    Quit,
    StartupDiscoveryDone(Vec<DeviceItem>, Option<Vec<DeviceItem>>),
}

#[derive(Debug, Clone)]
struct SavedEndpointState {
    device_id: Option<String>,
    was_muted: bool,
    orig_volume: f32,
}

static SAVED_ENDPOINT_STATES: std::sync::Mutex<Vec<SavedEndpointState>> = std::sync::Mutex::new(Vec::new());
static STREAMING_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static DEFAULT_DEVICE_CHANGED: AtomicBool = AtomicBool::new(false);

struct ActiveVolumeShield {
    device_id: Option<String>,
    endpoint_vol: IAudioEndpointVolume,
    callback: windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback,
}

unsafe impl Send for ActiveVolumeShield {}
unsafe impl Sync for ActiveVolumeShield {}

static ACTIVE_VOLUME_SHIELDS: std::sync::Mutex<Vec<ActiveVolumeShield>> = std::sync::Mutex::new(Vec::new());

struct ActiveNotificationClient {
    enumerator: IMMDeviceEnumerator,
    client: IMMNotificationClient,
}

unsafe impl Send for ActiveNotificationClient {}
unsafe impl Sync for ActiveNotificationClient {}

static NOTIFICATION_CLIENT: std::sync::Mutex<Option<ActiveNotificationClient>> = std::sync::Mutex::new(None);

#[windows::core::implement(windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback)]
struct AudioEndpointVolumeCallback {
    device_id: Option<String>,
    endpoint_vol: IAudioEndpointVolume,
    muting: AtomicBool,
}

impl windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback_Impl for AudioEndpointVolumeCallback_Impl {
    fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        if !pnotify.is_null() && STREAMING_ACTIVE.load(Ordering::Relaxed) {
            unsafe {
                if !(*pnotify).bMuted.as_bool() {
                    if !self.muting.swap(true, Ordering::SeqCst) {
                        tracing::info!(
                            "Audio endpoint [{:?}] unmuted by system/driver - event-driven clamping back to mute",
                            self.device_id
                        );
                        let _ = self.endpoint_vol.SetMute(true, std::ptr::null());
                        self.muting.store(false, Ordering::SeqCst);
                    }
                }
            }
        }
        Ok(())
    }
}

#[windows::core::implement(windows::Win32::Media::Audio::IMMNotificationClient)]
struct AudioNotificationClient;

impl windows::Win32::Media::Audio::IMMNotificationClient_Impl for AudioNotificationClient_Impl {
    fn OnDeviceStateChanged(&self, pwstrdeviceid: &windows::core::PCWSTR, dwnewstate: DEVICE_STATE) -> windows::core::Result<()> {
        if STREAMING_ACTIVE.load(Ordering::Relaxed) && dwnewstate == DEVICE_STATE_ACTIVE {
            let id = unsafe { pwstrdeviceid.to_string().ok() };
            tracing::info!("Audio device state changed to ACTIVE: {:?}, securing event shield", id);
            shield_endpoint(id.as_deref());
        }
        Ok(())
    }

    fn OnDeviceAdded(&self, pwstrdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        if STREAMING_ACTIVE.load(Ordering::Relaxed) {
            let id = unsafe { pwstrdeviceid.to_string().ok() };
            tracing::info!("Audio device added: {:?}, securing event shield", id);
            shield_endpoint(id.as_deref());
        }
        Ok(())
    }

    fn OnDeviceRemoved(&self, _pwstrdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, _role: ERole, pwstrdefaultdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        if flow == eRender && STREAMING_ACTIVE.load(Ordering::Relaxed) {
            let id = unsafe { pwstrdefaultdeviceid.to_string().ok() };
            tracing::info!("Default render device changed to: {:?}, signaling capture switch and securing mute", id);
            DEFAULT_DEVICE_CHANGED.store(true, Ordering::SeqCst);
            shield_endpoint(id.as_deref());
        }
        Ok(())
    }

    fn OnPropertyValueChanged(&self, _pwstrdeviceid: &windows::core::PCWSTR, _key: &PROPERTYKEY) -> windows::core::Result<()> {
        Ok(())
    }
}

/// Shields an endpoint by saving original volume/mute, muting it, and registering an `IAudioEndpointVolumeCallback`
/// so any attempt by the system or driver to unmute it will be immediately and accurately clamped back to mute.
pub fn shield_endpoint(device_id: Option<&str>) {
    let _com = initialize_mta();
    unsafe {
        let Ok(enumerator): Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return; };
        let device: IMMDevice = match device_id {
            Some(id) if id != "default" && !id.is_empty() => {
                let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
                match enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr())) {
                    Ok(d) => d,
                    Err(_) => return,
                }
            }
            _ => match enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) {
                Ok(d) => d,
                Err(_) => return,
            },
        };

        let dev_id_string = device.GetId().ok().and_then(|pwstr| pwstr.to_string().ok());
        let key = dev_id_string.clone().or_else(|| device_id.map(|s| s.to_string()));

        let Ok(ep_vol): Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None) else { return; };

        // 1. Record original state if not yet saved
        if let Ok(mut saved) = SAVED_ENDPOINT_STATES.lock() {
            if !saved.iter().any(|s| s.device_id == key) {
                let was_muted = ep_vol.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                let orig_volume = ep_vol.GetMasterVolumeLevelScalar().unwrap_or(1.0);
                saved.push(SavedEndpointState {
                    device_id: key.clone(),
                    was_muted,
                    orig_volume,
                });
            }
        }

        // 2. Mute endpoint immediately if streaming is active
        if STREAMING_ACTIVE.load(Ordering::Relaxed) {
            let _ = ep_vol.SetMute(true, std::ptr::null());
        }

        // 3. Register real-time volume callback if not already attached
        if let Ok(mut shields) = ACTIVE_VOLUME_SHIELDS.lock() {
            if !shields.iter().any(|s| s.device_id == key) {
                let cb: windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback = AudioEndpointVolumeCallback {
                    device_id: key.clone(),
                    endpoint_vol: ep_vol.clone(),
                    muting: AtomicBool::new(false),
                }.into();
                if ep_vol.RegisterControlChangeNotify(&cb).is_ok() {
                    shields.push(ActiveVolumeShield {
                        device_id: key,
                        endpoint_vol: ep_vol,
                        callback: cb,
                    });
                }
            }
        }
    }
}

/// Shields all currently active render endpoints in the system
pub fn shield_all_active_render_devices() {
    let _com = initialize_mta();
    unsafe {
        let Ok(enumerator): Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return; };
        shield_endpoint(None); // Default device
        if let Ok(collection) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
            if let Ok(count) = collection.GetCount() {
                for i in 0..count {
                    if let Ok(dev) = collection.Item(i) {
                        if let Ok(pwstr) = dev.GetId() {
                            if let Ok(id) = pwstr.to_string() {
                                shield_endpoint(Some(&id));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Called on Win32 hardware PnP change notification (WM_DEVICECHANGE)
pub fn on_hardware_device_change() {
    if STREAMING_ACTIVE.load(Ordering::Relaxed) {
        shield_all_active_render_devices();
    }
}

/// Mutes all active render endpoints and registers COM callbacks for real-time,
/// event-driven mute protection ONLY when streaming the full system mix.
/// When not selecting the full system mix, audio devices are not muted.
pub fn record_and_configure_streaming_device(device_id: Option<&str>) {
    let is_system_mix = device_id.is_none() || device_id == Some("default") || device_id.unwrap().is_empty();

    if !is_system_mix {
        tracing::info!("Selected audio device is not system mix ({:?}) - audio devices will NOT be muted", device_id);
        restore_original_speaker_state();
        return;
    }

    let _com = initialize_mta();

    if STREAMING_ACTIVE.swap(true, Ordering::SeqCst) {
        shield_all_active_render_devices();
        return;
    }

    DEFAULT_DEVICE_CHANGED.store(false, Ordering::SeqCst);

    // Register system-wide IMMNotificationClient for device change events
    unsafe {
        if let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) {
            let client: IMMNotificationClient = AudioNotificationClient.into();
            if enumerator.RegisterEndpointNotificationCallback(&client).is_ok() {
                if let Ok(mut notif) = NOTIFICATION_CLIENT.lock() {
                    *notif = Some(ActiveNotificationClient {
                        enumerator,
                        client,
                    });
                }
            }
        }
    }

    shield_all_active_render_devices();
    let _ = set_endpoint_volume(None, 1.0);
}

/// Backwards-compatible alias for record_and_configure_streaming_speakers
#[allow(dead_code)]
pub fn record_and_configure_streaming_speakers() {
    record_and_configure_streaming_device(None);
}

/// Backwards-compatible alias for record_and_configure_streaming_speakers
#[allow(dead_code)]
pub fn record_and_mute_speakers() {
    record_and_configure_streaming_device(None);
}

/// Restores all saved endpoints to their original volume level and mute state.
pub fn restore_original_speaker_state() {
    STREAMING_ACTIVE.store(false, Ordering::SeqCst);

    // 1. Unregister all IAudioEndpointVolumeCallback
    if let Ok(mut shields) = ACTIVE_VOLUME_SHIELDS.lock() {
        for s in shields.drain(..) {
            unsafe {
                let _ = s.endpoint_vol.UnregisterControlChangeNotify(&s.callback);
            }
        }
    }

    // 2. Unregister IMMNotificationClient
    if let Ok(mut notif) = NOTIFICATION_CLIENT.lock() {
        if let Some(active) = notif.take() {
            unsafe {
                let _ = active.enumerator.UnregisterEndpointNotificationCallback(&active.client);
            }
        }
    }

    // 3. Restore all saved endpoint volume & mute states
    let mut saved = match SAVED_ENDPOINT_STATES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    for item in saved.drain(..) {
        let id_ref = item.device_id.as_deref();
        let _ = set_endpoint_volume(id_ref, item.orig_volume);
        let _ = set_endpoint_mute(id_ref, item.was_muted);
    }
}

/// Backwards-compatible alias for restore_original_speaker_state
pub fn restore_original_speaker_mute() {
    restore_original_speaker_state();
}

extern "C" fn on_process_exit() {
    restore_original_speaker_state();
}

/// Register a C-runtime atexit handler to ensure speakers are restored on any exit
pub fn register_exit_handler() {
    extern "C" {
        fn atexit(f: extern "C" fn()) -> i32;
    }
    unsafe {
        atexit(on_process_exit);
    }
}

/// Control an audio endpoint's master volume (0.0 to 1.0). If device_id is None, controls default endpoint.
pub fn set_endpoint_volume(device_id: Option<&str>, level: f32) -> anyhow::Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device: IMMDevice = match device_id {
            Some(id) if id != "default" && !id.is_empty() => {
                let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
                enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr()))?
            }
            _ => enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?,
        };
        let ep_vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        ep_vol.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null())?;
    }
    Ok(())
}

/// Query an audio endpoint's master volume (0.0 to 1.0). If device_id is None, queries default endpoint.
pub fn get_endpoint_volume(device_id: Option<&str>) -> anyhow::Result<f32> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device: IMMDevice = match device_id {
            Some(id) if id != "default" && !id.is_empty() => {
                let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
                enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr()))?
            }
            _ => enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?,
        };
        let ep_vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        let vol = ep_vol.GetMasterVolumeLevelScalar()?;
        Ok(vol)
    }
}

/// Control an audio endpoint's mute state. If device_id is None, controls default endpoint.
pub fn set_endpoint_mute(device_id: Option<&str>, mute: bool) -> anyhow::Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device: IMMDevice = match device_id {
            Some(id) if id != "default" && !id.is_empty() => {
                let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
                enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr()))?
            }
            _ => enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?,
        };
        let ep_vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        ep_vol.SetMute(mute, std::ptr::null())?;
    }
    Ok(())
}

/// Query an audio endpoint's mute state. If device_id is None, queries default endpoint.
pub fn get_endpoint_mute(device_id: Option<&str>) -> anyhow::Result<bool> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device: IMMDevice = match device_id {
            Some(id) if id != "default" && !id.is_empty() => {
                let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
                enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr()))?
            }
            _ => enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?,
        };
        let ep_vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        let mute = ep_vol.GetMute()?;
        Ok(mute.as_bool())
    }
}

/// Control the master Windows speaker output volume (0.0 to 1.0)
#[allow(dead_code)]
pub fn set_speaker_volume(level: f32) -> anyhow::Result<()> {
    set_endpoint_volume(None, level)
}

/// Query the master Windows speaker output volume (0.0 to 1.0)
#[allow(dead_code)]
pub fn get_speaker_volume() -> anyhow::Result<f32> {
    get_endpoint_volume(None)
}

/// Control the master Windows speaker output mute state via IAudioEndpointVolume
pub fn set_speaker_mute(mute: bool) -> anyhow::Result<()> {
    set_endpoint_mute(None, mute)
}

/// Query the master Windows speaker output mute state
#[allow(dead_code)]
pub fn get_speaker_mute() -> anyhow::Result<bool> {
    get_endpoint_mute(None)
}

static MONOTONIC_START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
pub fn monotonic_now_ms() -> u64 {
    let start = MONOTONIC_START.get_or_init(std::time::Instant::now);
    start.elapsed().as_millis() as u64
}

pub struct DiagnosticCounters {
    pub frames_captured: AtomicU64,
    pub underruns: AtomicU64,
    pub last_active_ts: AtomicU64,
}

impl DiagnosticCounters {
    pub fn new() -> Self {
        Self {
            frames_captured: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            last_active_ts: AtomicU64::new(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeadDeviceInfo {
    pub mac: String,
    pub addresses: Vec<std::net::IpAddr>,
}

pub struct Session {
    conn: Connection,
    secondary_conns: Vec<Connection>,
    group_streamer: Option<AudioStreamer>,
    ctrl_stop: Option<Arc<AtomicBool>>,
    ctrl_handle: Option<JoinHandle<()>>,
    sender: LiveFrameSender,
    cap_stop: Arc<AtomicBool>,
    cap_handle: Option<JoinHandle<()>>,
    pub counters: Arc<DiagnosticCounters>,
    pub timing_desc: String,
    volume_rx: Option<tokio::sync::mpsc::UnboundedReceiver<(String, f32)>>,
}

impl Session {
    pub async fn start(
        device_items: Vec<DeviceItem>,
        volume: f32,
        device_volumes: &std::collections::HashMap<String, f32>,
        audio_device_id: String,
        low_latency: bool,
    ) -> anyhow::Result<Self> {
        // 1. Sort device_items so stereo pairs / multi-speaker units ALWAYS come first
        let mut sorted_items = device_items.clone();
        sorted_items.sort_by_key(|item| {
            (
                u8::from(!item.is_stereo_pair),
                std::cmp::Reverse(item.member_devices.len()),
            )
        });

        let mut clean_devices: Vec<Device> = Vec::new();
        let mut seen_ips = std::collections::HashSet::new();

        for item in &sorted_items {
            for mut d in item.member_devices.clone() {
                if let Some(ipv4) = d.addresses.iter().find(|a| a.is_ipv4()).copied() {
                    if seen_ips.insert(ipv4) {
                        d.addresses = vec![ipv4];
                        clean_devices.push(d);
                    }
                }
            }
            if !seen_ips.contains(&item.ip) {
                let mut d = item.raw_device.clone();
                if let Some(ipv4) = d.addresses.iter().find(|a| a.is_ipv4()).copied() {
                    if seen_ips.insert(ipv4) {
                        d.addresses = vec![ipv4];
                        clean_devices.push(d);
                    }
                }
            }
        }

        if clean_devices.is_empty() {
            anyhow::bail!("No available IPv4 address found for selected device");
        }

        // Critical: Sort clean_devices so that the established PTP Group Leader / Stereo Pair member comes FIRST!
        // In Apple AirPlay 2, a HomePod stereo pair has a designated Group Leader (is_group_leader == true)
        // that serves as the hardware PTP Master Clock for its pair.
        // If a standalone HomePod is placed at index 0 as the primary, the stereo pair's internal PTP engine
        // conflicts and stalls, causing all audio to fail.
        // Placing the Group Leader at index 0 ensures the entire multi-room group synchronizes cleanly.
        clean_devices.sort_by_key(|d| {
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

        let is_group = clean_devices.len() > 1;

        let (timing_proto, ptp_m, desc) = if is_group {
            let label = if device_items.len() == 1 && device_items[0].is_stereo_pair {
                format!("PTP (Stereo • {})", clean_devices.len())
            } else {
                format!("PTP (Group • {})", clean_devices.len())
            };
            (TimingProtocol::Ptp, PtpMode::Master, label)
        } else {
            (TimingProtocol::Ntp, PtpMode::Master, "NTP".to_string())
        };

        let audio_format = AudioFormat::default();
        let asc = if audio_format.codec == AudioCodec::Alac {
            Some(AlacEncoder::new(audio_format.clone())?.magic_cookie())
        } else {
            None
        };

        let (latency_min, latency_max) = if low_latency {
            (11025, 44100) // ~250ms at 44.1kHz (Low-Latency Video Sync)
        } else {
            (22050, 88200) // ~500ms at 44.1kHz (Standard Jitter-Tolerant)
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
            tracing::info!(
                "Starting group streaming to {} devices: {:?}",
                clean_devices.len(),
                clean_devices.iter().map(|d| (&d.name, &d.addresses)).collect::<Vec<_>>()
            );

            // 1. Connect and pair all devices
            let mut connections: Vec<Connection> = Vec::new();
            for dev in &clean_devices {
                tracing::info!("Connecting to group member {} ({:?})", dev.name, dev.addresses);
                let conn = Connection::connect_auto(dev.clone(), config.clone(), "3939").await?;
                connections.push(conn);
            }

            // 2. Collect peer addresses (device IPs + our local IP)
            let mut peer_addresses: Vec<String> = clean_devices
                .iter()
                .flat_map(|d| d.addresses.iter().find(|a| a.is_ipv4()).map(|a| a.to_string()))
                .collect();
            if let Some(local_addr) = connections[0].local_addr() {
                peer_addresses.push(local_addr.ip().to_string());
            }
            tracing::info!("Peer addresses for SETPEERS: {:?}", peer_addresses);

            // 3. Setup primary (Leader) with BMCA PTP sync
            tracing::info!("Setting up primary device: {}", clean_devices[0].name);
            connections[0].setup().await?;
            connections[0].send_setpeers(&peer_addresses).await?;

            let ptp_clock_id = connections[0]
                .ptp_master_clock_id()
                .ok_or_else(|| anyhow::anyhow!("Primary HomePod failed to acquire PTP master clock ID"))?;
            let timing_offset = connections[0]
                .timing_offset()
                .ok_or_else(|| anyhow::anyhow!("Primary HomePod failed to acquire clock offset"))?;
            let timing_rx = connections[0]
                .timing_rx()
                .ok_or_else(|| anyhow::anyhow!("Primary HomePod failed to acquire timing channel"))?;

            // 4. Setup secondary devices sharing primary PTP clock
            for i in 1..connections.len() {
                tracing::info!("Setting up secondary member {}: {}", i, clean_devices[i].name);
                let rx_clone = timing_rx.clone();
                connections[i].setup_for_group(ptp_clock_id, timing_offset, rx_clone).await?;
                connections[i].send_setpeers(&peer_addresses).await?;
            }

            // 5. Set initial volume on all devices based on device_volumes map
            for conn in &mut connections {
                let dev_mac = conn.device().id.to_mac_string();
                let target_vol = sorted_items
                    .iter()
                    .find(|item| {
                        item.raw_device.id.to_mac_string().eq_ignore_ascii_case(&dev_mac)
                            || item.member_devices.iter().any(|m| m.id.to_mac_string().eq_ignore_ascii_case(&dev_mac))
                            || conn.device().addresses.iter().any(|a| item.raw_device.addresses.contains(a) || item.member_devices.iter().any(|m| m.addresses.contains(a)))
                    })
                    .and_then(|item| device_volumes.get(&item.id_string()).copied())
                    .unwrap_or(volume);
                let _ = conn.set_volume(target_vol).await;
            }

            // 6. Send FLUSH on all connections
            for conn in &mut connections {
                let _ = conn.send_flush(0, 0).await;
            }

            // 7. Build RTP senders for all connections
            let mut senders = Vec::new();
            for conn in &connections {
                senders.push(conn.build_rtp_sender()?);
            }
            tracing::info!("Built {} RTP senders for group streaming", senders.len());

            // 8. Create shared AudioStreamer configured for multi-speaker group
            let mut streamer = AudioStreamer::new(config.clone());
            streamer.set_rtp_senders(senders).await;
            streamer.set_timing_offset(timing_offset).await;
            streamer.set_timing_updates(timing_rx).await;
            streamer.set_ptp_sync_mode(ptp_clock_id).await;

            // 9. Start audio capture
            let (sender, decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 32);
            let session_sender = sender.clone();
            let cap_stop = Arc::new(AtomicBool::new(false));
            let stop_clone = cap_stop.clone();
            let counters = Arc::new(DiagnosticCounters::new());
            let counters_clone = counters.clone();

            let cap_handle = std::thread::spawn(move || {
                if let Err(e) = run_capture(sender, stop_clone, counters_clone, audio_device_id) {
                    tracing::error!("Audio capture error: {e:#}");
                }
            });

            // 10. Start live streaming to all devices
            if let Err(e) = streamer.start_live(decoder).await {
                cap_stop.store(true, Ordering::Relaxed);
                let _ = cap_handle.join();
                for mut c in connections {
                    let _ = c.disconnect().await;
                }
                return Err(e.into());
            }

            // 11. Spawn group control channel listener for retransmit requests (PT=85)
            let mut control_sockets: Vec<(usize, std::net::UdpSocket)> = Vec::new();
            for (i, conn) in connections.iter().enumerate() {
                if let Some(sock) = conn.clone_control_socket_for_recv() {
                    control_sockets.push((i, sock));
                }
            }

            let ctrl_stop = Arc::new(AtomicBool::new(false));
            let ctrl_stop_clone = ctrl_stop.clone();
            let streamer_clone = streamer.clone();
            let rt_handle = tokio::runtime::Handle::current();

            let ctrl_handle = std::thread::Builder::new()
                .name("group-ctrl".into())
                .spawn(move || {
                    for (_, sock) in &control_sockets {
                        let _ = sock.set_read_timeout(Some(Duration::from_millis(200)));
                    }
                    let mut buf = [0u8; 2048];
                    while !ctrl_stop_clone.load(Ordering::Relaxed) {
                        for &(device_index, ref sock) in &control_sockets {
                            match sock.recv_from(&mut buf) {
                                Ok((len, _)) => {
                                    if len < 4 {
                                        continue;
                                    }
                                    let payload_type = buf[1] & 0x7F;
                                    if payload_type == 85 {
                                        let request = if len == 8 {
                                            let first_seq = u16::from_be_bytes([buf[4], buf[5]]);
                                            let count = u16::from_be_bytes([buf[6], buf[7]]);
                                            Some(RetransmitRequest {
                                                first_sequence: first_seq,
                                                count,
                                            })
                                        } else if len >= 12 {
                                            RetransmitRequest::parse(&buf[..len]).ok()
                                        } else {
                                            None
                                        };
                                        if let Some(req) = request {
                                            let _ = rt_handle.block_on(
                                                streamer_clone.handle_retransmit_for_target(device_index, &req),
                                            );
                                        }
                                    }
                                }
                                Err(ref e)
                                    if e.kind() == std::io::ErrorKind::TimedOut
                                        || e.kind() == std::io::ErrorKind::WouldBlock => {}
                                Err(_) => {}
                            }
                        }
                    }
                })
                .ok();

            let mut iter = connections.into_iter();
            let mut primary_conn = iter.next().unwrap();
            let mut secondary_conns: Vec<Connection> = iter.collect();

            let (combined_tx, combined_rx) = tokio::sync::mpsc::unbounded_channel::<(String, f32)>();

            let p_mac = primary_conn.device().id.to_mac_string();
            if let Some(mut rx) = primary_conn.take_volume_events_rx() {
                let tx = combined_tx.clone();
                let mac = p_mac.clone();
                tokio::spawn(async move {
                    while let Some(vol) = rx.recv().await {
                        let _ = tx.send((mac.clone(), vol));
                    }
                });
            }

            for sec in &mut secondary_conns {
                let s_mac = sec.device().id.to_mac_string();
                if let Some(mut rx) = sec.take_volume_events_rx() {
                    let tx = combined_tx.clone();
                    let mac = s_mac.clone();
                    tokio::spawn(async move {
                        while let Some(vol) = rx.recv().await {
                            let _ = tx.send((mac.clone(), vol));
                        }
                    });
                }
            }

            Ok(Self {
                conn: primary_conn,
                secondary_conns,
                group_streamer: Some(streamer),
                ctrl_stop: Some(ctrl_stop),
                ctrl_handle,
                sender: session_sender,
                cap_stop,
                cap_handle: Some(cap_handle),
                counters,
                timing_desc: desc,
                volume_rx: Some(combined_rx),
            })
        } else {
            // --- Single device streaming path ---
            let single_vol = sorted_items
                .first()
                .and_then(|item| device_volumes.get(&item.id_string()).copied())
                .unwrap_or(volume);

            let mut conn = Connection::connect_auto(clean_devices.into_iter().next().unwrap(), config, "3939").await?;
            conn.setup().await?;
            let _ = conn.set_volume(single_vol).await;

            let (sender, decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 32);
            let session_sender = sender.clone();
            let cap_stop = Arc::new(AtomicBool::new(false));
            let stop_clone = cap_stop.clone();
            let counters = Arc::new(DiagnosticCounters::new());
            let counters_clone = counters.clone();

            let cap_handle = std::thread::spawn(move || {
                if let Err(e) = run_capture(sender, stop_clone, counters_clone, audio_device_id) {
                    tracing::error!("Audio capture error: {e:#}");
                }
            });

            if let Err(e) = conn.start_streaming_live(decoder).await {
                cap_stop.store(true, Ordering::Relaxed);
                let _ = cap_handle.join();
                return Err(e.into());
            }

            let s_mac = conn.device().id.to_mac_string();
            let (combined_tx, combined_rx) = tokio::sync::mpsc::unbounded_channel::<(String, f32)>();
            if let Some(mut rx) = conn.take_volume_events_rx() {
                let tx = combined_tx.clone();
                let mac = s_mac.clone();
                tokio::spawn(async move {
                    while let Some(vol) = rx.recv().await {
                        let _ = tx.send((mac.clone(), vol));
                    }
                });
            }

            Ok(Self {
                conn,
                secondary_conns: Vec::new(),
                group_streamer: None,
                ctrl_stop: None,
                ctrl_handle: None,
                sender: session_sender,
                cap_stop,
                cap_handle: Some(cap_handle),
                counters,
                timing_desc: desc,
                volume_rx: Some(combined_rx),
            })
        }
    }

    pub fn switch_audio_device(&mut self, audio_device_id: String) {
        tracing::info!("Hot-swapping WASAPI capture device to [{audio_device_id}] (AirPlay stream uninterrupted)");
        self.cap_stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.cap_handle.take() {
            let _ = h.join();
        }

        let new_stop = Arc::new(AtomicBool::new(false));
        let stop_clone = new_stop.clone();
        let sender_clone = self.sender.clone();
        let counters_clone = self.counters.clone();

        let new_handle = std::thread::spawn(move || {
            if let Err(e) = run_capture(sender_clone, stop_clone, counters_clone, audio_device_id) {
                tracing::error!("Audio capture error: {e:#}");
            }
        });

        self.cap_stop = new_stop;
        self.cap_handle = Some(new_handle);
    }

    pub async fn recv_volume_change(&mut self) -> Option<(String, f32)> {
        if let Some(ref mut rx) = self.volume_rx {
            rx.recv().await
        } else {
            None
        }
    }

    pub async fn set_device_volume(
        &mut self,
        target_macs: &[String],
        target_ips: &[IpAddr],
        volume: f32,
    ) -> anyhow::Result<()> {
        let is_match = |c: &Connection| -> bool {
            let mac = c.device().id.to_mac_string();
            if target_macs.iter().any(|m| m.eq_ignore_ascii_case(&mac)) {
                return true;
            }
            if c.device().addresses.iter().any(|a| target_ips.contains(a)) {
                return true;
            }
            false
        };

        if is_match(&self.conn) {
            let _ = self.conn.set_volume(volume).await;
        }
        for sec in &mut self.secondary_conns {
            if is_match(sec) {
                let _ = sec.set_volume(volume).await;
            }
        }
        Ok(())
    }

    pub async fn set_volume(&mut self, volume: f32) -> anyhow::Result<()> {
        let _ = self.conn.set_volume(volume).await;
        for c in &mut self.secondary_conns {
            let _ = c.set_volume(volume).await;
        }
        Ok(())
    }

    pub async fn check_dead_devices(&mut self) -> Vec<DeadDeviceInfo> {
        let mut dead = Vec::new();
        // Only primary connection coordinates RTSP feedback for the session.
        if let Err(e) = self.conn.send_feedback().await {
            tracing::warn!(
                "Primary connection [{}] ({:?}) feedback failed: {e:#}",
                self.conn.device().id.to_mac_string(),
                self.conn.device().addresses
            );
            dead.push(DeadDeviceInfo {
                mac: self.conn.device().id.to_mac_string(),
                addresses: self.conn.device().addresses.clone(),
            });
        }
        dead
    }

    pub async fn stop(mut self) {
        self.cap_stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.cap_handle.take() {
            let _ = h.join();
        }
        if let Some(ref cs) = self.ctrl_stop {
            cs.store(true, Ordering::Relaxed);
        }
        if let Some(h) = self.ctrl_handle.take() {
            let _ = h.join();
        }
        if let Some(mut streamer) = self.group_streamer.take() {
            let _ = streamer.stop().await;
        }
        let _ = tokio::time::timeout(Duration::from_secs(3), self.conn.disconnect()).await;
        for mut c in self.secondary_conns {
            let _ = tokio::time::timeout(Duration::from_secs(3), c.disconnect()).await;
        }
    }
}

fn update_thread_execution_state(prevent_sleep: bool, is_streaming: bool) {
    unsafe {
        let flags = if is_streaming && prevent_sleep {
            windows_sys::Win32::System::Power::ES_CONTINUOUS
                | windows_sys::Win32::System::Power::ES_SYSTEM_REQUIRED
        } else {
            windows_sys::Win32::System::Power::ES_CONTINUOUS
        };
        windows_sys::Win32::System::Power::SetThreadExecutionState(flags);
    }
}

/// Helper to deduplicate repeated nested error messages in anyhow error chains
fn format_clean_error(err: &anyhow::Error) -> String {
    let mut messages: Vec<String> = Vec::new();
    for cause in err.chain() {
        let s = cause.to_string();
        if !messages.iter().any(|m| m.contains(&s)) {
            messages.retain(|m| !s.contains(m));
            messages.push(s);
        }
    }
    let full = messages.join(" - ");
    let cleaned = full
        .replace("RTSP error: Setup failed: ", "")
        .replace("Setup failed: ", "");
    if cleaned.trim().is_empty() {
        err.to_string()
    } else {
        cleaned
    }
}

pub fn spawn_controller(
    cmd_tx: async_channel::Sender<GuiCmd>,
    cmd_rx: AsyncReceiver<GuiCmd>,
    shared_state: Arc<RwLock<ControllerState>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime");

        rt.block_on(async move {
            let broadcast = |s: &ControllerState| {
                if let Ok(mut lock) = shared_state.write() {
                    *lock = s.clone();
                }
                crate::tray::notify_tray_state();
                crate::gui::request_ui_repaint();
            };

            let mut state = ControllerState::default();
            state.audio_devices = get_audio_render_devices();
            let last_target = load_last_connected_device();
            if let Some(ref target) = last_target {
                state.last_connected_name = Some(target.name.clone());
            }
            broadcast(&state);

            let mut active_session: Option<Session> = None;
            let mut current_connected_devices: Vec<DeviceItem> = Vec::new();
            let mut tick_interval = tokio::time::interval(Duration::from_millis(500));
            tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut keepalive_interval = tokio::time::interval(Duration::from_secs(3));
            keepalive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut last_reconnect_time = tokio::time::Instant::now() - Duration::from_secs(10);

            // Spawn background discovery and auto-connection immediately so event loop is never blocked
            let bg_tx = cmd_tx.clone();
            tokio::spawn(async move {
                let last_tgt = load_last_connected_device();
                let mut devs = discover(Duration::from_millis(3000)).await.unwrap_or_default();
                let mut matched = None;
                if let Some(ref target) = last_tgt {
                    matched = find_matching_devices(&devs, target);
                    if matched.is_none() {
                        if let Ok(devs2) = discover(Duration::from_millis(2000)).await {
                            matched = find_matching_devices(&devs2, target);
                            devs = devs2;
                        }
                    }
                }
                let _ = bg_tx.send(GuiCmd::StartupDiscoveryDone(devs, matched)).await;
            });

            // Spawn background Microsoft Store license verification and periodic refresh
            let license_tx = cmd_tx.clone();
            tokio::spawn(async move {
                let tier = crate::license::check_store_license().await;
                let _ = license_tx.send(GuiCmd::UpdateLicenseTier(tier)).await;

                // Re-check periodically every 1 hour (e.g. if user purchased from store in background)
                let mut interval = tokio::time::interval(Duration::from_secs(3600));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    interval.tick().await;
                    let refreshed = crate::license::check_store_license().await;
                    let _ = license_tx.send(GuiCmd::UpdateLicenseTier(refreshed)).await;
                }
            });

            loop {
                tokio::select! {
                    _ = keepalive_interval.tick() => {
                        if let Some(ref mut session) = active_session {
                            let dead_devs = session.check_dead_devices().await;
                            if !dead_devs.is_empty() {
                                tracing::warn!(
                                    "AirPlay connection(s) lost or preempted: {:?}",
                                    dead_devs.iter().map(|d| &d.mac).collect::<Vec<_>>()
                                );
                                let dead_names: Vec<String> = current_connected_devices
                                    .iter()
                                    .filter(|item| {
                                        dead_devs.iter().any(|info| {
                                            item.raw_device.id.to_mac_string().eq_ignore_ascii_case(&info.mac)
                                                || item.member_devices.iter().any(|m| m.id.to_mac_string().eq_ignore_ascii_case(&info.mac))
                                                || item.raw_device.addresses.iter().any(|a| info.addresses.contains(a))
                                                || item.member_devices.iter().any(|m| m.addresses.iter().any(|a| info.addresses.contains(a)))
                                        })
                                    })
                                    .map(|d| d.name.clone())
                                    .collect();
                                let dead_title = if dead_names.is_empty() {
                                    state.connected_name.clone().unwrap_or_else(|| "HomePod".to_string())
                                } else {
                                    dead_names.join("、")
                                };

                                if let Some(s) = active_session.take() {
                                    s.stop().await;
                                }

                                // Attempt automatic reconnection first before concluding the device was taken over!
                                tracing::info!("Attempting automatic reconnection to recover lost stream for [{dead_title}]...");
                                state.status = DeviceStatus::Connecting;
                                state.notice_message = Some((NoticeKind::ConnectionLostReconnecting(dead_title.clone()), std::time::Instant::now()));
                                broadcast(&state);

                                let devs = current_connected_devices.clone();
                                let eff_vol = if state.is_muted { 0.0 } else { state.volume };
                                let mut recovered = false;
                                let mut last_error_str = String::new();

                                for attempt in 1..=2 {
                                    tokio::time::sleep(Duration::from_millis(1500)).await;
                                    match Session::start(devs.clone(), eff_vol, &state.device_volumes, state.selected_audio_device.clone(), state.low_latency).await {
                                        Ok(sess) => {
                                            tracing::info!("Successfully recovered stream for [{dead_title}] on attempt {attempt}!");
                                            state.status = DeviceStatus::Streaming;
                                            state.diagnostics.timing_mode = sess.timing_desc.clone();
                                            state.notice_message = Some((NoticeKind::ConnectionRestored(dead_title.clone()), std::time::Instant::now()));
                                            active_session = Some(sess);
                                            update_thread_execution_state(state.prevent_sleep, true);
                                            broadcast(&state);
                                            recovered = true;
                                            break;
                                        }
                                        Err(e) => {
                                            last_error_str = format!("{e:#}");
                                            tracing::warn!("Auto-recovery attempt {attempt} failed: {last_error_str}");
                                        }
                                    }
                                }

                                if !recovered {
                                    let is_preempted = last_error_str.contains("453") || last_error_str.to_lowercase().contains("busy");
                                    let notice_kind = if is_preempted {
                                        NoticeKind::Preempted(dead_title.clone())
                                    } else {
                                        NoticeKind::Disconnected(dead_title.clone())
                                    };
                                    tracing::info!("Reconnection failed for [{dead_title}]");
                                    state.notice_message = Some((notice_kind, std::time::Instant::now()));
                                    restore_original_speaker_state();
                                    current_connected_devices.clear();
                                    state.status = DeviceStatus::Idle;
                                    state.connected_name = None;
                                    state.connected_device_ids.clear();
                                    update_thread_execution_state(state.prevent_sleep, false);
                                    broadcast(&state);
                                }
                            }
                        }
                    }
                    _ = tick_interval.tick() => {
                        if let Some(ref session) = active_session {
                            state.diagnostics.sample_rate = RATE;
                            state.diagnostics.channels = CHANNELS;
                            state.diagnostics.frames_captured = session.counters.frames_captured.load(Ordering::Relaxed);
                            state.diagnostics.underruns = session.counters.underruns.load(Ordering::Relaxed);
                            if state.diagnostics.timing_mode != session.timing_desc {
                                state.diagnostics.timing_mode = session.timing_desc.clone();
                            }

                            let last_active = session.counters.last_active_ts.load(Ordering::Relaxed);
                            let now = monotonic_now_ms();
                            // Active if non-silent audio was captured within the last 30 seconds
                            let is_active = last_active > 0 && now.saturating_sub(last_active) <= 30_000;
                            if state.is_audio_active != is_active {
                                state.is_audio_active = is_active;
                                update_thread_execution_state(state.prevent_sleep, state.is_audio_active);
                                tracing::info!(
                                    "Audio activity state changed: active={}, sleep_suppressed={}",
                                    state.is_audio_active,
                                    state.prevent_sleep && state.is_audio_active
                                );
                            }

                            // Update shared state in-place without cloning large device/volume structures
                            if let Ok(mut lock) = shared_state.write() {
                                lock.diagnostics.sample_rate = RATE;
                                lock.diagnostics.channels = CHANNELS;
                                lock.diagnostics.frames_captured = state.diagnostics.frames_captured;
                                lock.diagnostics.underruns = state.diagnostics.underruns;
                                if lock.diagnostics.timing_mode != state.diagnostics.timing_mode {
                                    lock.diagnostics.timing_mode = state.diagnostics.timing_mode.clone();
                                }
                                lock.is_audio_active = state.is_audio_active;
                            }
                        }
                    }
                    vol_opt = async {
                        if let Some(ref mut session) = active_session {
                            session.recv_volume_change().await
                        } else {
                            std::future::pending::<Option<(String, f32)>>().await
                        }
                    } => {
                        if let Some((mac, new_vol)) = vol_opt {
                            let clamped = new_vol.clamp(0.0, 1.0);
                            let matched_id = state.devices.iter().find(|d| {
                                d.raw_device.id.to_mac_string().eq_ignore_ascii_case(&mac)
                                    || d.member_devices.iter().any(|m| m.id.to_mac_string().eq_ignore_ascii_case(&mac))
                            }).map(|d| d.id_string());

                            if let Some(dev_id) = matched_id {
                                let old_vol = state.device_volumes.get(&dev_id).copied().unwrap_or(state.volume);
                                if (old_vol - clamped).abs() > 0.005 {
                                    tracing::info!("Hardware volume change from HomePod [{}]: {:.3} (was {:.3})", dev_id, clamped, old_vol);
                                    state.device_volumes.insert(dev_id, clamped);

                                    let sum: f32 = state.connected_device_ids.iter().filter_map(|id| state.device_volumes.get(id)).sum();
                                    let count = state.connected_device_ids.iter().filter(|id| state.device_volumes.contains_key(*id)).count();
                                    if count > 0 {
                                        state.volume = sum / (count as f32);
                                        state.is_muted = state.volume <= 0.001;
                                        save_volume(state.volume);
                                    }
                                    broadcast(&state);
                                }
                            }
                        }
                    }
                    res = cmd_rx.recv() => {
                        let cmd = match res {
                            Ok(c) => c,
                            Err(_) => break,
                        };

                        match cmd {
                            GuiCmd::StartupDiscoveryDone(devs, matched) => {
                                tracing::info!(
                                    "[CONTROLLER] StartupDiscoveryDone: found {} device(s), matched={}",
                                    devs.len(),
                                    matched.as_ref().map(|v| v.len()).unwrap_or(0)
                                );
                                state.devices = devs;
                                if state.auto_connect && active_session.is_none() {
                                    if let Some(connect_devs) = matched {
                                        tracing::info!("Startup auto-connecting to target ({} device(s))...", connect_devs.len());
                                        let _ = cmd_tx.send(GuiCmd::Connect(connect_devs)).await;
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::UpdateLicenseTier(tier) => {
                                tracing::info!("[CONTROLLER] Store license tier updated: {:?}", tier);
                                state.license_tier = tier;
                                if !tier.is_auto_mute_allowed() {
                                    if state.auto_mute_speakers {
                                        tracing::warn!("Store trial expired! Auto-muting speakers feature is locked and disabled.");
                                        state.auto_mute_speakers = false;
                                        save_auto_mute(false);
                                        if active_session.is_some() {
                                            restore_original_speaker_mute();
                                        }
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::RefreshDevices => {
                                state.status = DeviceStatus::Idle;
                                broadcast(&state);
                                if let Ok(devs) = discover(Duration::from_secs(4)).await {
                                    state.devices = devs;
                                    broadcast(&state);
                                }
                            }
                            GuiCmd::RefreshAudioDevices => {
                                state.audio_devices = get_audio_render_devices();
                                broadcast(&state);
                            }
                            GuiCmd::SetAudioDevice(dev_id) => {
                                state.selected_audio_device = dev_id.clone();
                                save_audio_device(&dev_id);

                                // When switching audio device, restore previous endpoint states first
                                restore_original_speaker_state();
                                if state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed() {
                                    record_and_configure_streaming_device(Some(&state.selected_audio_device));
                                }

                                if let Some(ref mut session) = active_session {
                                    // Hot-swap only the WASAPI audio capture device!
                                    // The AirPlay 2 connection, PTP clock sync, and HomePod buffers remain 100% uninterrupted!
                                    session.switch_audio_device(state.selected_audio_device.clone());
                                }
                                broadcast(&state);
                            }
                            GuiCmd::Connect(mut dev_items) => {
                                if dev_items.is_empty() {
                                    continue;
                                }

                                // Always ensure stereo pairs / group leaders are prioritized first
                                dev_items.sort_by_key(|d| {
                                    (
                                        u8::from(!d.is_stereo_pair),
                                        std::cmp::Reverse(d.member_devices.len()),
                                    )
                                });

                                if let Some(s) = active_session.take() {
                                    s.stop().await;
                                    tokio::time::sleep(Duration::from_millis(300)).await;
                                }

                                for dev in &dev_items {
                                    state.device_volumes.entry(dev.id_string()).or_insert(state.volume);
                                }

                                let names: Vec<String> = dev_items.iter().map(|d| d.name.clone()).collect();
                                let display_title = if dev_items.len() == 1 {
                                    names[0].clone()
                                } else {
                                    format!("{} (+{})", names[0], dev_items.len() - 1)
                                };

                                state.status = DeviceStatus::Connecting;
                                state.notice_message = None;
                                state.connected_name = Some(display_title.clone());
                                state.connected_device_ids = dev_items.iter().map(|d| d.id_string()).collect();
                                current_connected_devices = dev_items.clone();
                                broadcast(&state);

                                let eff_vol = if state.is_muted { 0.0 } else { state.volume };
                                match Session::start(dev_items, eff_vol, &state.device_volumes, state.selected_audio_device.clone(), state.low_latency).await {
                                    Ok(sess) => {
                                        state.status = DeviceStatus::Streaming;
                                        state.diagnostics.timing_mode = sess.timing_desc.clone();
                                        active_session = Some(sess);

                                        // Configure target device speakers for streaming if auto-mute is active
                                        if state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed() {
                                            record_and_configure_streaming_device(Some(&state.selected_audio_device));
                                        } else {
                                            restore_original_speaker_state();
                                        }

                                        // Update power keepalive (initially false until audio activity is detected)
                                        state.is_audio_active = false;
                                        update_thread_execution_state(state.prevent_sleep, false);

                                        save_last_connected_device(&current_connected_devices, &display_title);
                                        state.last_connected_name = Some(display_title);

                                        broadcast(&state);
                                    }
                                    Err(err) => {
                                        let msg = format_clean_error(&err);
                                        tracing::error!("Connection error: {msg}");
                                        state.status = DeviceStatus::Error(msg);
                                        state.connected_name = None;
                                        state.connected_device_ids.clear();
                                        current_connected_devices.clear();
                                        restore_original_speaker_state();
                                        update_thread_execution_state(state.prevent_sleep, false);

                                        broadcast(&state);
                                    }
                                }
                            }
                            GuiCmd::Disconnect => {
                                state.notice_message = None;
                                current_connected_devices.clear();
                                if let Some(s) = active_session.take() {
                                    s.stop().await;
                                    tokio::time::sleep(Duration::from_millis(150)).await;
                                }
                                update_thread_execution_state(state.prevent_sleep, false);
                                restore_original_speaker_state();
                                state.status = DeviceStatus::Idle;
                                state.connected_name = None;
                                state.connected_device_ids.clear();
                                state.diagnostics = StreamDiagnostics::default();
                                broadcast(&state);
                            }
                            GuiCmd::SetVolume(vol) => {
                                let clamped = vol.clamp(0.0, 1.0);
                                let delta = clamped - state.volume;
                                state.volume = clamped;
                                save_volume(state.volume);

                                if state.is_muted && clamped > 0.001 {
                                    state.is_muted = false;
                                }

                                for id in &state.connected_device_ids {
                                    let cur = state.device_volumes.get(id).copied().unwrap_or(clamped);
                                    let new_dev_vol = (cur + delta).clamp(0.0, 1.0);
                                    state.device_volumes.insert(id.clone(), new_dev_vol);
                                }

                                if let Some(ref mut sess) = active_session {
                                    if !state.is_muted {
                                        if state.connected_device_ids.len() <= 1 {
                                            let _ = sess.set_volume(clamped).await;
                                        } else {
                                            let mut any_set = false;
                                            for id in &state.connected_device_ids {
                                                if let Some(target_item) = current_connected_devices.iter().chain(state.devices.iter()).find(|d| d.id_string() == *id) {
                                                    let dev_vol = state.device_volumes.get(id).copied().unwrap_or(clamped);
                                                    let mut target_macs = vec![target_item.raw_device.id.to_mac_string()];
                                                    let mut target_ips = vec![target_item.ip];
                                                    for m in &target_item.member_devices {
                                                        target_macs.push(m.id.to_mac_string());
                                                        target_ips.extend(m.addresses.iter().filter(|a| a.is_ipv4()));
                                                    }
                                                    let _ = sess.set_device_volume(&target_macs, &target_ips, dev_vol).await;
                                                    any_set = true;
                                                }
                                            }
                                            if !any_set {
                                                let _ = sess.set_volume(clamped).await;
                                            }
                                        }
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetDeviceVolume(dev_id, vol) => {
                                let clamped = vol.clamp(0.0, 1.0);
                                state.device_volumes.insert(dev_id.clone(), clamped);

                                if state.is_muted && clamped > 0.001 {
                                    state.is_muted = false;
                                }

                                if !state.connected_device_ids.is_empty() {
                                    let sum: f32 = state.connected_device_ids.iter().filter_map(|id| state.device_volumes.get(id)).sum();
                                    let count = state.connected_device_ids.iter().filter(|id| state.device_volumes.contains_key(*id)).count();
                                    if count > 0 {
                                        state.volume = sum / (count as f32);
                                        save_volume(state.volume);
                                    }
                                }

                                if let Some(ref mut sess) = active_session {
                                    if !state.is_muted {
                                        if let Some(target_item) = current_connected_devices.iter().chain(state.devices.iter()).find(|d| d.id_string() == dev_id) {
                                            let mut target_macs = vec![target_item.raw_device.id.to_mac_string()];
                                            let mut target_ips = vec![target_item.ip];
                                            for m in &target_item.member_devices {
                                                target_macs.push(m.id.to_mac_string());
                                                target_ips.extend(m.addresses.iter().filter(|a| a.is_ipv4()));
                                            }
                                            let _ = sess.set_device_volume(&target_macs, &target_ips, clamped).await;
                                        } else {
                                            let _ = sess.set_volume(clamped).await;
                                        }
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetMute(muted) => {
                                state.is_muted = muted;
                                if let Some(ref mut sess) = active_session {
                                    if muted {
                                        let _ = sess.set_volume(0.0).await;
                                    } else {
                                        if state.connected_device_ids.len() <= 1 {
                                            let _ = sess.set_volume(state.volume).await;
                                        } else {
                                            let mut any_set = false;
                                            for id in &state.connected_device_ids {
                                                if let Some(target_item) = current_connected_devices.iter().chain(state.devices.iter()).find(|d| d.id_string() == *id) {
                                                    let dev_vol = state.device_volumes.get(id).copied().unwrap_or(state.volume);
                                                    let mut target_macs = vec![target_item.raw_device.id.to_mac_string()];
                                                    let mut target_ips = vec![target_item.ip];
                                                    for m in &target_item.member_devices {
                                                        target_macs.push(m.id.to_mac_string());
                                                        target_ips.extend(m.addresses.iter().filter(|a| a.is_ipv4()));
                                                    }
                                                    let _ = sess.set_device_volume(&target_macs, &target_ips, dev_vol).await;
                                                    any_set = true;
                                                }
                                            }
                                            if !any_set {
                                                let _ = sess.set_volume(state.volume).await;
                                            }
                                        }
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetAutoMuteSpeakers(enabled) => {
                                let allowed = state.license_tier.is_auto_mute_allowed();
                                let actual = if allowed { enabled } else { false };
                                state.auto_mute_speakers = actual;
                                save_auto_mute(actual);
                                if active_session.is_some() {
                                    if actual {
                                        record_and_configure_streaming_device(Some(&state.selected_audio_device));
                                    } else {
                                        restore_original_speaker_state();
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetPreventSleep(enabled) => {
                                state.prevent_sleep = enabled;
                                save_prevent_sleep(enabled);
                                let is_active = active_session.is_some() && state.is_audio_active;
                                update_thread_execution_state(state.prevent_sleep, is_active);
                                broadcast(&state);
                            }
                            GuiCmd::SetLowLatency(enabled) => {
                                state.low_latency = enabled;
                                save_low_latency(enabled);
                                if !current_connected_devices.is_empty() {
                                    if let Some(s) = active_session.take() {
                                        s.stop().await;
                                    }
                                    let devs = current_connected_devices.clone();
                                    let eff_vol = if state.is_muted { 0.0 } else { state.volume };
                                    match Session::start(devs, eff_vol, &state.device_volumes, state.selected_audio_device.clone(), state.low_latency).await {
                                        Ok(sess) => {
                                            state.status = DeviceStatus::Streaming;
                                            state.diagnostics.timing_mode = sess.timing_desc.clone();
                                            active_session = Some(sess);
                                            update_thread_execution_state(state.prevent_sleep, true);
                                            if state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed() {
                                                record_and_configure_streaming_device(Some(&state.selected_audio_device));
                                            }
                                        }
                                        Err(e) => {
                                            state.status = DeviceStatus::Error(format_clean_error(&e));
                                        }
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetAutoConnect(enabled) => {
                                state.auto_connect = enabled;
                                save_auto_connect(enabled);
                                broadcast(&state);
                            }
                            GuiCmd::SetAutostart(enabled) => {
                                state.autostart = enabled;
                                if let Err(e) = crate::autostart::set_autostart(enabled) {
                                    tracing::error!("Failed to update autostart setting: {e:#}");
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetHotkey(mods, vk) => {
                                state.hotkey = (mods, vk);
                                save_hotkey(mods, vk);
                                crate::tray::update_registered_hotkey(mods, vk);
                                broadcast(&state);
                            }
                            GuiCmd::SetLanguage(lang) => {
                                state.language = lang;
                                save_language(lang);
                                broadcast(&state);
                            }
                            GuiCmd::SystemSuspend => {
                                tracing::info!("Power event: System suspend (Sleep). Securing speakers and stopping stream for clean sleep...");
                                if state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed() && (state.selected_audio_device == "default" || state.selected_audio_device.is_empty()) {
                                    let guard = match SAVED_ENDPOINT_STATES.lock() {
                                        Ok(g) => g,
                                        Err(p) => p.into_inner(),
                                    };
                                    for item in guard.iter() {
                                        let _ = set_endpoint_mute(item.device_id.as_deref(), true);
                                    }
                                    let _ = set_speaker_mute(true);
                                }

                                if let Some(s) = active_session.take() {
                                    tracing::info!("Stopping active AirPlay session before system sleep to prevent audio leakage...");
                                    s.stop().await;
                                    // Retain connected_name and current_connected_devices so SystemResume can reconnect
                                    state.status = DeviceStatus::Connecting;
                                    broadcast(&state);
                                }
                                update_thread_execution_state(state.prevent_sleep, false);
                            }
                            GuiCmd::SystemResume => {
                                tracing::info!("Power event: System resume from sleep. Reconnecting HomePod stream...");
                                if state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed() && (state.selected_audio_device == "default" || state.selected_audio_device.is_empty()) {
                                    let guard = match SAVED_ENDPOINT_STATES.lock() {
                                        Ok(g) => g,
                                        Err(p) => p.into_inner(),
                                    };
                                    for item in guard.iter() {
                                        let _ = set_endpoint_mute(item.device_id.as_deref(), true);
                                    }
                                    let _ = set_speaker_mute(true);
                                }

                                if !current_connected_devices.is_empty() && last_reconnect_time.elapsed() >= Duration::from_millis(1500) {
                                    last_reconnect_time = tokio::time::Instant::now();
                                    if let Some(s) = active_session.take() {
                                        s.stop().await;
                                    }

                                    let devs = current_connected_devices.clone();
                                    let display_title = state.connected_name.clone().unwrap_or_else(|| "HomePod".to_string());
                                    state.status = DeviceStatus::Connecting;
                                    state.notice_message = Some((NoticeKind::ResumeReconnecting(display_title.clone()), std::time::Instant::now()));
                                    broadcast(&state);

                                    let eff_vol = if state.is_muted { 0.0 } else { state.volume };
                                    let sel_dev = state.selected_audio_device.clone();
                                    let dev_vols = state.device_volumes.clone();
                                    let low_lat = state.low_latency;
                                    let prevent_slp = state.prevent_sleep;
                                    let auto_mute = state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed();

                                    let mut reconnected = false;
                                    let mut last_err = String::new();

                                    // Retry up to 5 times (total ~10s window for Wi-Fi and HomePod mDNS to stabilize)
                                    for attempt in 1..=5 {
                                        let wait_ms = if attempt == 1 { 1200 } else { 1800 };
                                        tokio::time::sleep(Duration::from_millis(wait_ms)).await;

                                        if current_connected_devices.is_empty() {
                                            tracing::info!("Wake auto-reconnect cancelled because connection was cleared");
                                            break;
                                        }

                                        // In later attempts, refresh device addresses via mDNS in case DHCP changed IP
                                        let target_devs = if attempt >= 2 {
                                            if let Ok(fresh) = discover(Duration::from_millis(1000)).await {
                                                let mut updated = Vec::new();
                                                for d in &devs {
                                                    let id_str = d.id_string();
                                                    if let Some(found) = fresh.iter().find(|f| f.id_string().eq_ignore_ascii_case(&id_str)) {
                                                        updated.push(found.clone());
                                                    } else {
                                                        updated.push(d.clone());
                                                    }
                                                }
                                                updated
                                            } else {
                                                devs.clone()
                                            }
                                        } else {
                                            devs.clone()
                                        };

                                        tracing::info!("Wake auto-reconnect attempt {}/5 to [{display_title}]...", attempt);
                                        match Session::start(target_devs, eff_vol, &dev_vols, sel_dev.clone(), low_lat).await {
                                            Ok(sess) => {
                                                tracing::info!("HomePod stream successfully restored after wake!");
                                                state.status = DeviceStatus::Streaming;
                                                state.diagnostics.timing_mode = sess.timing_desc.clone();
                                                state.notice_message = Some((NoticeKind::ResumeRestored(display_title.clone()), std::time::Instant::now()));
                                                active_session = Some(sess);
                                                update_thread_execution_state(prevent_slp, true);
                                                if auto_mute {
                                                    record_and_configure_streaming_device(Some(&sel_dev));
                                                }
                                                broadcast(&state);
                                                reconnected = true;
                                                break;
                                            }
                                            Err(e) => {
                                                last_err = format!("{e:#}");
                                                tracing::warn!("Wake auto-reconnect attempt {} failed: {last_err}", attempt);
                                            }
                                        }
                                    }

                                    if !reconnected && !current_connected_devices.is_empty() {
                                        tracing::error!("All 5 wake auto-reconnect attempts failed. Falling back to local speakers.");
                                        let is_preempted = last_err.contains("453") || last_err.to_lowercase().contains("busy");
                                        let notice_kind = if is_preempted {
                                            NoticeKind::ResumePreempted(display_title.clone())
                                        } else {
                                            NoticeKind::ResumeFailed(display_title.clone())
                                        };
                                        state.notice_message = Some((notice_kind, std::time::Instant::now()));
                                        restore_original_speaker_state();
                                        current_connected_devices.clear();
                                        state.status = DeviceStatus::Idle;
                                        state.connected_name = None;
                                        state.connected_device_ids.clear();
                                        update_thread_execution_state(state.prevent_sleep, false);
                                        broadcast(&state);
                                    }
                                }
                            }
                            GuiCmd::Quit => {
                                restore_original_speaker_mute();
                                if let Some(s) = active_session.take() {
                                    s.stop().await;
                                    tokio::time::sleep(Duration::from_millis(150)).await;
                                }
                                update_thread_execution_state(state.prevent_sleep, false);
                                break;
                            }
                        }
                    }
                }
            }
        });
    })
}

pub async fn discover(timeout: Duration) -> anyhow::Result<Vec<DeviceItem>> {
    let browser = ServiceBrowser::new()?;
    let mut devices = browser.scan(timeout).await?;
    devices.retain(|d| d.supports_airplay2() && d.addresses.iter().any(|a| a.is_ipv4()));
    
    let items = group_discovered_devices(devices);
    Ok(items)
}

pub fn group_discovered_devices(devices: Vec<Device>) -> Vec<DeviceItem> {
    use std::collections::HashMap;

    // Group devices by tight_sync_id, group_public_name, or group_id
    let mut groups: HashMap<String, Vec<Device>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for d in devices {
        let key = if let Some(tsid) = d.tight_sync_id {
            format!("tsid:{}", tsid)
        } else if let Some(ref gpn) = d.group_public_name {
            format!("gpn:{}", gpn)
        } else if let Some(gid) = d.group_id {
            if d.is_group_leader || d.group_contains_discoverable_leader {
                format!("gid:{}", gid)
            } else {
                format!("id:{}", d.id.to_mac_string())
            }
        } else {
            format!("id:{}", d.id.to_mac_string())
        };

        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push(d);
    }

    let mut items = Vec::new();
    for key in order {
        if let Some(mut member_devices) = groups.remove(&key) {
            // Sort members so group leader (is_group_leader == true) is the primary target
            member_devices.sort_by_key(|d| u8::from(!d.is_group_leader));

            let leader = member_devices.first().cloned().unwrap();
            let Some(ip) = leader.addresses.iter().find(|a| a.is_ipv4()).copied() else {
                continue;
            };

            let count = member_devices.len();
            let is_stereo = count >= 2 || (leader.tight_sync_id.is_some() && leader.group_public_name.is_some());

            let display_name = if let Some(ref gpn) = leader.group_public_name {
                gpn.clone()
            } else {
                leader.name.clone()
            };

            let friendly_model = match leader.model.as_str() {
                "AudioAccessory1,1" | "AudioAccessory1,2" => "HomePod (1st gen)",
                "AudioAccessory5,1" => "HomePod mini",
                "AudioAccessory6,1" => "HomePod (2nd gen)",
                m if m.starts_with("AppleTV") => "Apple TV",
                m if m.starts_with("Mac") => "Mac",
                _ => &leader.model,
            };

            let model_desc = if is_stereo {
                format!("Stereo Pair ({count} • {friendly_model})")
            } else {
                friendly_model.to_string()
            };

            items.push(DeviceItem {
                name: display_name,
                ip,
                model: model_desc,
                raw_device: leader,
                member_devices,
                is_stereo_pair: is_stereo,
                pair_count: count,
            });
        }
    }

    // Sort: HomePods first, and within HomePods, stereo pairs first
    items.sort_by_key(|item| {
        let is_homepod = item.raw_device.model.starts_with("AudioAccessory");
        (u8::from(!is_homepod), u8::from(!item.is_stereo_pair))
    });

    items
}

pub fn get_audio_render_devices() -> Vec<AudioDeviceItem> {
    let mut list = vec![AudioDeviceItem {
        id: "default".to_string(),
        name: "System Audio (Default Output Device)".to_string(),
    }];

    let _com = initialize_mta();
    if let Ok(enumerator) = DeviceEnumerator::new() {
        if let Ok(collection) = enumerator.get_device_collection(&Direction::Render) {
            if let Ok(count) = collection.get_nbr_devices() {
                for i in 0..count {
                    if let Ok(dev) = collection.get_device_at_index(i) {
                        if let (Ok(id), Ok(name)) = (dev.get_id(), dev.get_friendlyname()) {
                            list.push(AudioDeviceItem { id, name });
                        }
                    }
                }
            }
        }
    }
    list
}

fn run_capture(
    sender: LiveFrameSender,
    stop: Arc<AtomicBool>,
    counters: Arc<DiagnosticCounters>,
    audio_device_id: String,
) -> anyhow::Result<()> {
    let _com = initialize_mta();
    let bytes_per_frame = CHANNELS as usize * 4;
    let silence = vec![0i16; (RATE as usize / 20) * CHANNELS as usize];

    let is_system_mix = audio_device_id == "default" || audio_device_id.is_empty();

    while !stop.load(Ordering::Relaxed) {
        if is_system_mix {
            shield_endpoint(None);
        }
        let enumerator = match DeviceEnumerator::new() {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to create DeviceEnumerator: {e}, retrying in 500ms");
                let _ = sender.try_send(LivePcmFrame {
                    samples: silence.clone(),
                    channels: CHANNELS,
                    sample_rate: RATE,
                });
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let device = if is_system_mix {
            match enumerator.get_default_device(&Direction::Render) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Failed to get default render device: {e}, retrying in 500ms");
                    let _ = sender.try_send(LivePcmFrame {
                        samples: silence.clone(),
                        channels: CHANNELS,
                        sample_rate: RATE,
                    });
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        } else {
            match enumerator.get_device(&audio_device_id) {
                Ok(d) => d,
                Err(e) => {
                    tracing::info!("Target device [{audio_device_id}] not ready or asleep ({e}). Sending silence & retrying in 500ms...");
                    let _ = sender.try_send(LivePcmFrame {
                        samples: silence.clone(),
                        channels: CHANNELS,
                        sample_rate: RATE,
                    });
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        };

        let mut audio_client = match device.get_iaudioclient() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to get IAudioClient: {e}, retrying in 500ms");
                let _ = sender.try_send(LivePcmFrame {
                    samples: silence.clone(),
                    channels: CHANNELS,
                    sample_rate: RATE,
                });
                std::thread::sleep(Duration::from_millis(500));
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
            tracing::warn!("Failed to initialize audio client: {e}, retrying in 500ms");
            let _ = sender.try_send(LivePcmFrame {
                samples: silence.clone(),
                channels: CHANNELS,
                sample_rate: RATE,
            });
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }

        let h_event = match audio_client.set_get_eventhandle() {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("Failed to set event handle: {e}, retrying in 500ms");
                let _ = sender.try_send(LivePcmFrame {
                    samples: silence.clone(),
                    channels: CHANNELS,
                    sample_rate: RATE,
                });
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let capture_client = match audio_client.get_audiocaptureclient() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to get IAudioCaptureClient: {e}, retrying in 500ms");
                let _ = sender.try_send(LivePcmFrame {
                    samples: silence.clone(),
                    channels: CHANNELS,
                    sample_rate: RATE,
                });
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let current_device_id = device.get_id().ok();

        let mut queue: VecDeque<u8> = VecDeque::new();
        if let Err(e) = audio_client.start_stream() {
            tracing::warn!("Failed to start audio stream: {e}, retrying in 500ms");
            let _ = sender.try_send(LivePcmFrame {
                samples: silence.clone(),
                channels: CHANNELS,
                sample_rate: RATE,
            });
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }

        if is_system_mix {
            let _ = set_speaker_mute(true);
        }

        tracing::info!("WASAPI loopback audio capture active on device: {:?}", current_device_id);

        while !stop.load(Ordering::Relaxed) {
            // Purely event-driven default device change notification from IMMNotificationClient::OnDefaultDeviceChanged
            if DEFAULT_DEVICE_CHANGED.swap(false, Ordering::SeqCst) && is_system_mix {
                tracing::info!("IMMNotificationClient signaled default render device changed, switching capture...");
                break;
            }

            match capture_client.read_from_device_to_deque(&mut queue) {
                Ok(_) => {
                    let n_frames = queue.len() / bytes_per_frame;
                    if n_frames > 0 {
                        counters.frames_captured.fetch_add(n_frames as u64, Ordering::Relaxed);
                        let n_samples = n_frames * CHANNELS as usize;
                        let n_bytes = n_frames * bytes_per_frame;
                        let mut samples: Vec<i16> = Vec::with_capacity(n_samples);
                        let mut has_audio = false;

                        let slice = &queue.make_contiguous()[..n_bytes];
                        for b in slice.chunks_exact(4) {
                            let f = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                            let s = (f.clamp(-1.0, 1.0) * 32767.0) as i16;
                            if s.abs() > 32 {
                                has_audio = true;
                            }
                            samples.push(s);
                        }
                        queue.drain(..n_bytes);
                        if has_audio {
                            counters.last_active_ts.store(monotonic_now_ms(), Ordering::Relaxed);
                        }
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
                    tracing::warn!("WASAPI capture error (screen off / device switch): {e:#}, cooling down...");
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

    Ok(())
}

pub fn app_data_dir() -> std::path::PathBuf {
    let base = std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join("win2pod")
}

pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;
pub const MOD_NOREPEAT: u32 = 0x4000;

pub const DEFAULT_HOTKEY_MODS: u32 = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT;
pub const DEFAULT_HOTKEY_VK: u32 = 0x48; // 'H' (Ctrl + Alt + H)

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastConnectedTarget {
    pub name: String,
    pub is_stereo_pair: bool,
    pub device_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_audio_device")]
    pub audio_device: String,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default = "default_true")]
    pub auto_mute_speakers: bool,
    #[serde(default = "default_true")]
    pub prevent_sleep: bool,
    #[serde(default = "default_true")]
    pub low_latency: bool,
    #[serde(default = "default_true")]
    pub auto_connect: bool,
    #[serde(default)]
    pub last_device: Option<LastConnectedTarget>,
    #[serde(default = "default_hotkey")]
    pub hotkey: (u32, u32),
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_audio_device() -> String { "default".to_string() }
fn default_volume() -> f32 { DEFAULT_VOLUME }
fn default_true() -> bool { true }
fn default_hotkey() -> (u32, u32) { (DEFAULT_HOTKEY_MODS, DEFAULT_HOTKEY_VK) }
fn default_language() -> String { Language::detect_system_language().code().to_string() }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            audio_device: default_audio_device(),
            volume: default_volume(),
            auto_mute_speakers: default_true(),
            prevent_sleep: default_true(),
            low_latency: default_true(),
            auto_connect: default_true(),
            last_device: None,
            hotkey: default_hotkey(),
            language: default_language(),
        }
    }
}

static CONFIG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl AppConfig {
    fn load_internal() -> Self {
        let dir = app_data_dir();
        let config_path = dir.join("config.json");
        if config_path.exists() {
            if let Ok(data) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&data) {
                    return config;
                }
            }
        }

        let config = AppConfig::default();
        config.save_internal();
        config
    }

    fn save_internal(&self) {
        let dir = app_data_dir();
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join("config.json");
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(config_path, json);
        }
    }
}

pub fn load_config() -> AppConfig {
    let _lock = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    AppConfig::load_internal()
}

pub fn update_config<F: FnOnce(&mut AppConfig)>(f: F) {
    let _lock = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut config = AppConfig::load_internal();
    f(&mut config);
    config.save_internal();
}

pub fn load_audio_device() -> String {
    load_config().audio_device
}

pub fn save_audio_device(id: &str) {
    let trimmed = id.trim().to_string();
    update_config(|c| c.audio_device = trimmed);
}

pub fn load_volume() -> f32 {
    load_config().volume
}

pub fn save_volume(v: f32) {
    let clamped = v.clamp(0.0, 1.0);
    update_config(|c| c.volume = clamped);
}

pub fn load_auto_mute() -> bool {
    load_config().auto_mute_speakers
}

pub fn save_auto_mute(enabled: bool) {
    update_config(|c| c.auto_mute_speakers = enabled);
}

pub fn load_prevent_sleep() -> bool {
    load_config().prevent_sleep
}

pub fn save_prevent_sleep(enabled: bool) {
    update_config(|c| c.prevent_sleep = enabled);
}

pub fn load_low_latency() -> bool {
    load_config().low_latency
}

pub fn save_low_latency(enabled: bool) {
    update_config(|c| c.low_latency = enabled);
}

pub fn load_auto_connect() -> bool {
    load_config().auto_connect
}

pub fn save_auto_connect(enabled: bool) {
    update_config(|c| c.auto_connect = enabled);
}

pub fn load_last_connected_device() -> Option<LastConnectedTarget> {
    load_config().last_device
}

pub fn save_last_connected_device(devices: &[DeviceItem], name: &str) {
    if devices.is_empty() {
        return;
    }
    let is_stereo = devices.iter().any(|d| d.is_stereo_pair);
    let mut ids: Vec<String> = Vec::new();
    for dev in devices {
        let d_id = dev.id_string();
        if !ids.contains(&d_id) {
            ids.push(d_id);
        }
        for m in &dev.member_devices {
            let mac = m.id.to_mac_string();
            if !ids.contains(&mac) {
                ids.push(mac);
            }
        }
    }
    let target = LastConnectedTarget {
        name: name.to_string(),
        is_stereo_pair: is_stereo,
        device_ids: ids,
    };
    update_config(|c| c.last_device = Some(target));
}

pub fn load_hotkey() -> (u32, u32) {
    load_config().hotkey
}

pub fn save_hotkey(mods: u32, vk: u32) {
    update_config(|c| c.hotkey = (mods, vk));
}

pub fn load_language() -> Language {
    let code = load_config().language;
    Language::from_code(&code).unwrap_or_else(Language::detect_system_language)
}

pub fn save_language(lang: Language) {
    update_config(|c| c.language = lang.code().to_string());
}

pub fn find_matching_devices(devices: &[DeviceItem], target: &LastConnectedTarget) -> Option<Vec<DeviceItem>> {
    // 1. If target is a stereo pair, check if there's a stereo pair matching any of the member MACs
    if target.is_stereo_pair {
        for dev in devices {
            if dev.is_stereo_pair {
                let matched = dev.member_devices.iter().any(|m| {
                    target.device_ids.iter().any(|tid| tid.eq_ignore_ascii_case(&m.id.to_mac_string()))
                }) || target.device_ids.iter().any(|tid| tid.eq_ignore_ascii_case(&dev.id_string()));
                if matched {
                    return Some(vec![dev.clone()]);
                }
            }
        }
    }

    // 2. Exact match on single device ID / MAC address
    for dev in devices {
        if target.device_ids.iter().any(|tid| tid.eq_ignore_ascii_case(&dev.id_string())) {
            return Some(vec![dev.clone()]);
        }
    }

    // 3. Fallback match on name if MAC address formatting differs
    for dev in devices {
        if dev.name.eq_ignore_ascii_case(&target.name) {
            return Some(vec![dev.clone()]);
        }
    }

    None
}
