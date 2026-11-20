//! Backend controller for HomePod-Cast GUI.
//! Handles discovery, connection, streaming session, WASAPI loopback capture, and diagnostics.

use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use airplay_audio::eq::{MAX_GAIN_DB, MIN_GAIN_DB};
use airplay_audio::{
    AlacEncoder, AudioStreamer, EqConfig, EqParams, LiveAudioDecoder, LiveFrameSender, LivePcmFrame,
    RetransmitRequest, SoftwareAudioMixer, SoundEnhancerMode, SoundEnhancerParams, WarmthEnhancerMode, WarmthEnhancerParams,
};
use airplay_receiver::{
    AirPlayReceiver, AirPlayReceiverConfig, AudioSink, CallbackSink, LocalSpeakerSink, SwitchableSink,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioSource {
    Device(String),
    Application {
        pid: u32,
        exe_name: String,
        display_name: String,
    },
}

impl Default for AudioSource {
    fn default() -> Self {
        Self::Device("default".to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AirPlayReceiverTarget {
    #[serde(rename = "default")]
    SystemDefault,
    #[serde(rename = "homepod")]
    HomePodRelay,
}

impl Default for AirPlayReceiverTarget {
    fn default() -> Self {
        Self::SystemDefault
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AirPlayReceiverState {
    #[serde(rename = "standby")]
    Standby,
    #[serde(rename = "connected")]
    Connected,
    #[serde(rename = "streaming")]
    Streaming,
}

impl Default for AirPlayReceiverState {
    fn default() -> Self {
        Self::Standby
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AirPlayReceiverSyncLevel {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "standard")]
    Standard,
    #[serde(rename = "strong")]
    Strong,
}

impl Default for AirPlayReceiverSyncLevel {
    fn default() -> Self {
        Self::Standard
    }
}

impl AirPlayReceiverSyncLevel {
    pub fn delay_stereo_samples(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Standard => 132300, // 1500ms at 44.1kHz stereo (剛好與 YouTube 完美同步)
            Self::Strong => 176400,   // 2000ms at 44.1kHz stereo (2.0秒標準 AirPlay 延遲，針對極大落後)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceStatus {
    Idle,
    Connecting,
    Streaming,
    AoMode(String),
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
    pub is_discovering: bool,
    pub status: DeviceStatus,
    pub connected_name: Option<String>,
    pub connected_device_ids: Vec<String>,
    pub volume: f32,
    pub is_muted: bool,
    pub audio_devices: Vec<AudioDeviceItem>,
    pub selected_audio_device: String,
    pub last_selected_device: String,
    pub last_selected_app: Option<(String, String)>,
    pub ao_output_device: String,
    pub audio_apps: Vec<crate::app_enumerator::AudioAppInfo>,
    pub audio_source: AudioSource,
    pub diagnostics: StreamDiagnostics,
    pub auto_mute_speakers: bool,
    pub auto_ao_mode: bool,
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
    pub eq_enabled: bool,
    pub eq_gains: [f32; 10],
    pub custom_eq_gains: [f32; 10],
    pub sound_enhancer_mode: SoundEnhancerMode,
    pub warmth_enhancer_mode: WarmthEnhancerMode,
    pub airpods_detected: bool,
    pub airpods_in_ear: bool,
    pub airpods_left_in_ear: bool,
    pub airpods_right_in_ear: bool,
    pub airpods_model: Option<String>,
    pub airpods_battery_left: Option<u8>,
    pub airpods_battery_right: Option<u8>,
    pub airpods_battery_case: Option<u8>,
    pub airpods_charging_left: bool,
    pub airpods_charging_right: bool,
    pub airpods_charging_case: bool,
    pub airpods_lid_opened: Option<bool>,
    pub airpods_both_in_case: Option<bool>,
    pub airpods_auto_routing: bool,
    pub airpods_auto_pause: bool,
    pub window_snapping: bool,
    pub snap_distance: i32,
    pub detach_equalizer_window: bool,
    pub paired_airpods: Vec<crate::airpods::PairedAirPodsDevice>,
    pub airpods_states: std::collections::HashMap<String, crate::airpods::protocol::AirPodsDeviceState>,
    pub airplay_receiver_enabled: bool,
    pub airplay_receiver_name: String,
    pub airplay_receiver_sync_level: AirPlayReceiverSyncLevel,
    pub airplay_receiver_streaming: bool,
    pub airplay_receiver_state: AirPlayReceiverState,
    pub airplay_receiver_client_name: Option<String>,
    pub airplay_receiver_device_type: Option<String>,
    pub airplay_receiver_client_ip: Option<String>,
    pub airplay_receiver_volume: Option<u8>,
    pub airplay_receiver_track_title: Option<String>,
    pub airplay_receiver_track_artist: Option<String>,
    pub airplay_receiver_track_album: Option<String>,
    pub airplay_receiver_artwork_bytes: Option<Vec<u8>>,
    pub airplay_receiver_auto_max_volume: bool,
    pub airplay_receiver_has_cached_session: bool,
    pub airplay_receiver_status_hint: Option<(String, std::time::Instant)>,
    pub airplay_receiver_output_device: String,
    pub airplay_receiver_apply_effects: bool,
    pub airplay_receiver_eq_enabled: bool,
    pub airplay_receiver_eq_gains: [f32; 10],
    pub airplay_receiver_custom_eq_gains: [f32; 10],
    pub airplay_receiver_sound_enhancer_mode: SoundEnhancerMode,
    pub airplay_receiver_warmth_enhancer_mode: WarmthEnhancerMode,
}

impl ControllerState {
    /// Determines whether AirPods are actively connected and in use (via Bluetooth audio endpoint, in-ear, or lid open).
    /// When true, battery readings reflect live hardware status.
    /// When false, battery readings represent stale/cached values from before disconnection.
    pub fn is_airpods_connected(&self) -> bool {
        if !self.airpods_detected {
            return false;
        }
        let is_stored_closed = self.airpods_lid_opened == Some(false) && self.airpods_both_in_case == Some(true);
        if is_stored_closed {
            return false;
        }
        let has_bluetooth_endpoint = self.audio_devices.iter().any(|d| is_airpods_playback_device(&d.name));
        has_bluetooth_endpoint || self.airpods_left_in_ear || self.airpods_right_in_ear
    }

    /// Returns true if an active Windows Bluetooth stereo audio endpoint for AirPods is currently connected and ready.
    pub fn is_airpods_audio_endpoint_active(&self) -> bool {
        self.audio_devices.iter().any(|d| is_airpods_playback_device(&d.name))
    }
}

impl Default for ControllerState {
    fn default() -> Self {
        let initial_source = load_audio_source();
        let initial_dev_str = match &initial_source {
            AudioSource::Device(_) => "default".to_string(),
            AudioSource::Application { exe_name, .. } => format!("app:{exe_name}"),
        };
        let last_app = load_last_selected_app().or_else(|| {
            match &initial_source {
                AudioSource::Application { exe_name, display_name, .. } => {
                    Some((exe_name.clone(), display_name.clone()))
                }
                _ => None,
            }
        });
        let last_dev = match &initial_source {
            AudioSource::Device(_) => "default".to_string(),
            AudioSource::Application { exe_name, .. } => format!("app:{exe_name}"),
        };

        Self {
            devices: Vec::new(),
            is_discovering: true,
            status: DeviceStatus::Idle,
            connected_name: None,
            connected_device_ids: Vec::new(),
            volume: load_volume(),
            is_muted: false,
            audio_devices: vec![AudioDeviceItem {
                id: "default".to_string(),
                name: "System Audio (Default Output Device)".to_string(),
            }],
            selected_audio_device: initial_dev_str,
            last_selected_device: last_dev,
            last_selected_app: last_app,
            ao_output_device: "default".to_string(),
            audio_apps: Vec::new(),
            audio_source: initial_source,
            diagnostics: StreamDiagnostics::default(),
            auto_mute_speakers: load_auto_mute(),
            auto_ao_mode: load_auto_ao_mode(),
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
            eq_enabled: load_eq_enabled(),
            eq_gains: load_eq_gains(),
            custom_eq_gains: load_custom_eq_gains(),
            sound_enhancer_mode: load_sound_enhancer_mode(),
            warmth_enhancer_mode: load_warmth_enhancer_mode(),
            airpods_detected: false,
            airpods_in_ear: false,
            airpods_left_in_ear: false,
            airpods_right_in_ear: false,
            airpods_model: None,
            airpods_battery_left: None,
            airpods_battery_right: None,
            airpods_battery_case: None,
            airpods_charging_left: false,
            airpods_charging_right: false,
            airpods_charging_case: false,
            airpods_lid_opened: None,
            airpods_both_in_case: None,
            airpods_auto_routing: load_airpods_auto_routing(),
            airpods_auto_pause: load_airpods_auto_pause(),
            window_snapping: load_window_snapping(),
            snap_distance: load_snap_distance(),
            detach_equalizer_window: load_detach_equalizer_window(),
            paired_airpods: Vec::new(),
            airpods_states: std::collections::HashMap::new(),
            airplay_receiver_enabled: load_airplay_receiver_enabled(),
            airplay_receiver_name: load_airplay_receiver_name(),
            airplay_receiver_sync_level: load_airplay_receiver_sync_level(),
            airplay_receiver_streaming: false,
            airplay_receiver_state: AirPlayReceiverState::Standby,
            airplay_receiver_client_name: None,
            airplay_receiver_device_type: None,
            airplay_receiver_client_ip: None,
            airplay_receiver_volume: None,
            airplay_receiver_track_title: None,
            airplay_receiver_track_artist: None,
            airplay_receiver_track_album: None,
            airplay_receiver_artwork_bytes: None,
            airplay_receiver_auto_max_volume: load_airplay_receiver_auto_max_volume(),
            airplay_receiver_has_cached_session: false,
            airplay_receiver_status_hint: None,
            airplay_receiver_output_device: load_airplay_receiver_output_device(),
            airplay_receiver_apply_effects: load_airplay_receiver_apply_effects(),
            airplay_receiver_eq_enabled: load_airplay_receiver_eq_enabled(),
            airplay_receiver_eq_gains: load_airplay_receiver_eq_gains(),
            airplay_receiver_custom_eq_gains: load_airplay_receiver_custom_eq_gains(),
            airplay_receiver_sound_enhancer_mode: load_airplay_receiver_sound_enhancer_mode(),
            airplay_receiver_warmth_enhancer_mode: load_airplay_receiver_warmth_enhancer_mode(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqPreset {
    BassBoost,
    TrebleBoost,
    Vocal,
    Pop,
    Rock,
    Electronic,
    Classical,
    Jazz,
    Custom,
}

impl EqPreset {
    pub const ALL: &'static [EqPreset] = &[
        Self::BassBoost,
        Self::TrebleBoost,
        Self::Vocal,
        Self::Pop,
        Self::Rock,
        Self::Electronic,
        Self::Classical,
        Self::Jazz,
        Self::Custom,
    ];

    pub fn gains(&self) -> [f32; 10] {
        match self {
            Self::BassBoost => [6.0, 5.0, 3.5, 1.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            Self::TrebleBoost => [0.0, 0.0, 0.0, 0.0, 0.5, 1.5, 3.0, 4.5, 5.5, 6.0],
            Self::Vocal => [-1.5, -1.0, 0.0, 1.5, 3.5, 4.0, 3.0, 1.5, 0.0, -1.0],
            Self::Pop => [-1.0, 1.5, 3.0, 4.0, 3.0, 0.5, -0.5, 1.0, 2.5, 3.0],
            Self::Rock => [4.5, 3.0, 1.5, 0.0, -1.0, -0.5, 1.5, 3.0, 4.0, 4.5],
            Self::Electronic => [5.5, 4.5, 2.0, 0.0, -1.5, 1.0, 0.5, 2.0, 4.0, 5.0],
            Self::Classical => [4.0, 3.0, 2.0, 1.5, -0.5, -0.5, 0.0, 1.5, 3.0, 3.5],
            Self::Jazz => [1.5, 3.5, 2.5, 1.5, 0.0, 0.0, 1.0, 2.0, 3.0, 2.0],
            Self::Custom => load_custom_eq_gains(),
        }
    }

    /// Check if a set of 10-band gains matches any defined preset within tolerance.
    pub fn matches_gains(gains: &[f32; 10]) -> Option<EqPreset> {
        Self::matches_gains_with_custom(gains, &load_custom_eq_gains())
    }

    /// Check if a set of 10-band gains matches any defined preset within tolerance, using a custom gains slice.
    pub fn matches_gains_with_custom(gains: &[f32; 10], custom: &[f32; 10]) -> Option<EqPreset> {
        for preset in Self::ALL {
            if *preset == Self::Custom {
                continue;
            }
            let p_gains = preset.gains();
            if gains.iter().zip(p_gains.iter()).all(|(a, b)| (a - b).abs() < 0.1) {
                return Some(*preset);
            }
        }
        if gains.iter().zip(custom.iter()).all(|(a, b)| (a - b).abs() < 0.1) {
            Some(Self::Custom)
        } else {
            None
        }
    }
}

pub enum GuiCmd {
    RefreshDevices,
    DevicesRefreshed(Vec<DeviceItem>),
    RefreshAudioDevices,
    RefreshAudioApps,
    SetAudioSource(AudioSource),
    Connect(Vec<DeviceItem>),
    Disconnect,
    SetAutoAoMode(bool),
    SetAoOutputDevice(String),
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
    SetEqGain(usize, f32),
    SetEqBypass(bool),
    ResetEq,
    ApplyEqPreset(EqPreset),
    SetSoundEnhancerMode(SoundEnhancerMode),
    SetWarmthEnhancerMode(WarmthEnhancerMode),
    LocalEndpointVolumeChanged { volume: f32, is_muted: bool },
    SetAirPodsAutoRouting(bool),
    SetAirPodsAutoPause(bool),
    SetWindowSnapping(bool),
    SetSnapDistance(i32),
    SetDetachEqualizerWindow(bool),
    AirPodsEventReceived(crate::airpods::AirPodsEvent),
    SetAirPlayReceiverEnabled(bool),
    SetAirPlayReceiverName(String),
    SetAirPlayReceiverSyncLevel(AirPlayReceiverSyncLevel),
    AirPlayReceiverStreamingStatus { streaming: bool, client_name: Option<String> },
    AirPlayReceiverClientConnected { name: String, device_type: String, ip: String },
    AirPlayReceiverClientDisconnected,
    AirPlayReceiverPaused,
    AirPlayReceiverVolume(f32),
    AirPlayReceiverClientName(String),
    AirPlayReceiverMetadata { title: Option<String>, artist: Option<String>, album: Option<String> },
    AirPlayReceiverArtwork(Option<Vec<u8>>),
    AirPlayReceiverDacpPlayPause,
    AirPlayReceiverDacpNext,
    AirPlayReceiverDacpPrev,
    SetAirPlayReceiverAutoMaxVolume(bool),
    AirPlayReceiverCheckWakeResult,
    SetAirPlayReceiverOutputDevice(String),
    SetAirPlayReceiverApplyEffects(bool),
    SetReceiverEqGain(usize, f32),
    SetReceiverEqBypass(bool),
    ResetReceiverEq,
    ApplyReceiverEqPreset(EqPreset),
    SetReceiverSoundEnhancerMode(SoundEnhancerMode),
    SetReceiverWarmthEnhancerMode(WarmthEnhancerMode),
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

/// Current instantaneous audio peak amplitude (0.0 ..= 1.0), stored as scaled u32 (0 ..= 10000)
pub static AUDIO_PEAK_LEVEL: AtomicU32 = AtomicU32::new(0);

/// Global flag for source-level muting (silencing audio capture without altering receiver hardware volume).
pub static IS_SOURCE_MUTED: AtomicBool = AtomicBool::new(false);

/// Real-time audio balance (-100 = full left, 0 = center, +100 = full right)
pub static AUDIO_BALANCE: AtomicI32 = AtomicI32::new(0);

/// Real-time mono mix toggle (true = mono, false = stereo)
pub static AUDIO_IS_MONO: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn set_source_mute(muted: bool) {
    IS_SOURCE_MUTED.store(muted, Ordering::Relaxed);
}

/// AudioSink wrapper that intercepts stream start/stop events to notify Controller for GUI status updates.
pub struct ReceiverStatusProxySink {
    pub target: Arc<dyn AudioSink>,
    pub cmd_tx: async_channel::Sender<GuiCmd>,
}

impl AudioSink for ReceiverStatusProxySink {
    fn on_audio_samples(&self, samples: &[i16], sample_rate: u32, channels: u8) {
        self.target.on_audio_samples(samples, sample_rate, channels);
    }
    fn on_volume_change(&self, volume_db: f32) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverVolume(volume_db));
        self.target.on_volume_change(volume_db);
    }
    fn on_stream_start(&self) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverStreamingStatus { streaming: true, client_name: None });
        self.target.on_stream_start();
    }
    fn on_stream_start_with_client(&self, client_name: Option<&str>) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverStreamingStatus {
            streaming: true,
            client_name: client_name.map(|s| s.to_string()),
        });
        self.target.on_stream_start_with_client(client_name);
    }
    fn on_client_name_change(&self, name: &str) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverClientName(name.to_string()));
        self.target.on_client_name_change(name);
    }
    fn on_client_connected(&self, client_name: &str, device_model: &str, client_ip: &str) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverClientConnected {
            name: client_name.to_string(),
            device_type: device_model.to_string(),
            ip: client_ip.to_string(),
        });
        self.target.on_client_connected(client_name, device_model, client_ip);
    }
    fn on_stream_paused(&self) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverPaused);
        self.target.on_stream_paused();
    }
    fn on_client_disconnected(&self) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverClientDisconnected);
        self.target.on_client_disconnected();
    }
    fn on_metadata_change(&self, title: Option<&str>, artist: Option<&str>, album: Option<&str>) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverMetadata {
            title: title.map(|s| s.to_string()),
            artist: artist.map(|s| s.to_string()),
            album: album.map(|s| s.to_string()),
        });
        self.target.on_metadata_change(title, artist, album);
    }
    fn on_artwork_change(&self, artwork_bytes: Option<&[u8]>) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverArtwork(artwork_bytes.map(|b| b.to_vec())));
        self.target.on_artwork_change(artwork_bytes);
    }
    fn on_stream_stop(&self) {
        let _ = self.cmd_tx.send_blocking(GuiCmd::AirPlayReceiverStreamingStatus { streaming: false, client_name: None });
        self.target.on_stream_stop();
    }
    fn on_flush(&self) {
        self.target.on_flush();
    }
    fn latency_frames(&self) -> u32 {
        self.target.latency_frames()
    }
}

#[inline]
pub fn set_audio_balance(val: f32) {
    let scaled = (val.clamp(-1.0, 1.0) * 100.0).round() as i32;
    AUDIO_BALANCE.store(scaled, Ordering::Relaxed);
}

#[inline]
pub fn get_audio_balance() -> f32 {
    AUDIO_BALANCE.load(Ordering::Relaxed) as f32 / 100.0
}

#[inline]
pub fn set_audio_mono(mono: bool) {
    AUDIO_IS_MONO.store(mono, Ordering::Relaxed);
}

#[inline]
pub fn is_audio_mono() -> bool {
    AUDIO_IS_MONO.load(Ordering::Relaxed)
}

#[inline]
#[allow(dead_code)]
pub fn is_source_muted() -> bool {
    IS_SOURCE_MUTED.load(Ordering::Relaxed)
}

#[inline]
pub fn set_audio_peak_level(peak: f32) {
    let scaled = (peak.clamp(0.0, 1.0) * 10000.0) as u32;
    AUDIO_PEAK_LEVEL.store(scaled, Ordering::Relaxed);
}

#[inline]
pub fn get_audio_peak_level() -> f32 {
    let scaled = AUDIO_PEAK_LEVEL.load(Ordering::Relaxed);
    scaled as f32 / 10000.0
}

pub const WIN2POD_EVENT_CONTEXT: windows::core::GUID = windows::core::GUID::from_u128(0x9a8b7c6d_5e4f_3a2b_1c0d_ef9876543210);

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioShieldState {
    /// Idle: No render endpoints are shielded; normal Windows playback.
    Idle,
    /// AirPlay Streaming: HomePod is playing over Wi-Fi.
    /// All local PC render endpoints are shielded to prevent echo.
    AirPlayStreaming,
    /// Local Target Routing (e.g. AirPods or dedicated DAC/headphones):
    /// PC background/laptop speakers are shielded so audio only plays through `target_endpoint_id`.
    /// The target endpoint itself is NEVER shielded or clamped to mute.
    LocalTarget {
        target_endpoint_id: String,
    },
}

static CURRENT_SHIELD_STATE: std::sync::Mutex<AudioShieldState> = std::sync::Mutex::new(AudioShieldState::Idle);

pub fn get_audio_shield_state() -> AudioShieldState {
    match CURRENT_SHIELD_STATE.lock() {
        Ok(g) => g.clone(),
        Err(p) => p.into_inner().clone(),
    }
}

pub fn is_airplay_streaming_active() -> bool {
    matches!(get_audio_shield_state(), AudioShieldState::AirPlayStreaming)
}

#[windows::core::implement(windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback)]
struct LocalEndpointVolumeCallback;

impl windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback_Impl for LocalEndpointVolumeCallback_Impl {
    fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        if !pnotify.is_null() && !is_airplay_streaming_active() {
            unsafe {
                let ctx = (*pnotify).guidEventContext;
                if ctx != WIN2POD_EVENT_CONTEXT {
                    let muted = (*pnotify).bMuted.as_bool();
                    let volume = (*pnotify).fMasterVolume;
                    if let Some(tx) = CONTROLLER_CMD_TX.get() {
                        let _ = tx.send_blocking(GuiCmd::LocalEndpointVolumeChanged { volume, is_muted: muted });
                    }
                }
            }
        }
        Ok(())
    }
}

struct ActiveLocalEndpointListener {
    device_id: Option<String>,
    endpoint_vol: IAudioEndpointVolume,
    callback: windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback,
}

unsafe impl Send for ActiveLocalEndpointListener {}
unsafe impl Sync for ActiveLocalEndpointListener {}

static LOCAL_ENDPOINT_LISTENER: std::sync::Mutex<Option<ActiveLocalEndpointListener>> = std::sync::Mutex::new(None);
static CONTROLLER_CMD_TX: std::sync::OnceLock<async_channel::Sender<GuiCmd>> = std::sync::OnceLock::new();

/// Returns true if a device friendly name matches AirPods/Beats.
pub fn is_airpods_playback_device(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("airpods") || n.contains("beats") || n.contains("pods")
}

/// Returns true if a device name OR endpoint ID represents an AirPods device.
pub fn is_airpods_device_or_id(devices: &[AudioDeviceItem], name_or_id: &str) -> bool {
    if name_or_id == "default" || name_or_id.is_empty() {
        return false;
    }
    if is_airpods_playback_device(name_or_id) {
        return true;
    }
    if let Some(d) = devices.iter().find(|d| d.id.eq_ignore_ascii_case(name_or_id)) {
        return is_airpods_playback_device(&d.name);
    }
    if name_or_id.starts_with('{') {
        if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
            if let Ok(dev) = enumerator.get_device(name_or_id) {
                if let Ok(friendly) = dev.get_friendlyname() {
                    return is_airpods_playback_device(&friendly);
                }
            }
        }
    }
    false
}

/// Returns true if a device friendly name represents a dedicated stereo audio playback device
/// (i.e. not labeled as telephony/hands-free).
pub fn is_airpods_stereo_playback_device(name: &str) -> bool {
    let n = name.to_lowercase();
    let has_keyword = n.contains("airpods") || n.contains("beats") || n.contains("pods");
    let is_telephony = n.contains("hands-free") || n.contains("handsfree") || n.contains("免持") || n.contains("telephony") || n.contains("ag audio");
    has_keyword && !is_telephony
}

/// Finds the best matching AirPods audio device: prefers stereo profile,
/// falling back to telephony/hands-free profile if stereo is not available.
pub fn find_best_airpods_device<'a>(devices: &'a [AudioDeviceItem]) -> Option<&'a AudioDeviceItem> {
    if let Some(dev) = devices.iter().find(|d| is_airpods_stereo_playback_device(&d.name)) {
        return Some(dev);
    }
    devices.iter().find(|d| is_airpods_playback_device(&d.name))
}

static SAVED_STANDIN_DEVICE_ID: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static SHARED_STATE: std::sync::OnceLock<Arc<RwLock<ControllerState>>> = std::sync::OnceLock::new();

pub fn is_airpods_auto_routing_and_worn() -> bool {
    if let Some(shared) = SHARED_STATE.get() {
        if let Ok(state) = shared.read() {
            return state.airpods_auto_routing && (state.airpods_left_in_ear || state.airpods_right_in_ear || state.airpods_in_ear);
        }
    }
    false
}

pub fn is_monitor_playback_device(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("nvidia")
        || n.contains("intel(r) display")
        || n.contains("amd high definition")
        || n.contains("monitor")
        || n.contains("displayport")
        || n.contains("mp341")
}

pub fn update_saved_standin_device(device_id: &str) {
    if !device_id.is_empty() && device_id != "default" {
        if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
            if let Ok(dev) = enumerator.get_device(device_id) {
                if let Ok(name) = dev.get_friendlyname() {
                    if !is_airpods_playback_device(&name) && !is_monitor_playback_device(&name) {
                        if let Ok(mut saved) = SAVED_STANDIN_DEVICE_ID.lock() {
                            *saved = Some(device_id.to_string());
                        }
                        save_standin_device_id(device_id);
                    }
                }
            }
        }
    }
}

pub fn get_standin_device_id() -> Option<String> {
    let devices = get_audio_render_devices();
    let non_airpods: Vec<_> = devices
        .into_iter()
        .filter(|d| d.id != "default" && !d.id.is_empty() && !is_airpods_playback_device(&d.name))
        .collect();

    // 1st Priority: Always prefer C-Media Audio or dedicated USB Audio host device (non-monitor)
    if let Some(cmedia) = non_airpods.iter().find(|d| {
        let n = d.name.to_lowercase();
        (n.contains("c-media") || n.contains("cmedia") || n.contains("usb audio"))
            && !is_monitor_playback_device(&d.name)
    }) {
        let id = cmedia.id.clone();
        update_saved_standin_device(&id);
        return Some(id);
    }

    if let Ok(saved) = SAVED_STANDIN_DEVICE_ID.lock() {
        if let Some(ref id) = *saved {
            if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
                if let Ok(dev) = enumerator.get_device(id) {
                    if dev.get_state().ok() == Some(wasapi::DeviceState::Active) {
                        if let Ok(name) = dev.get_friendlyname() {
                            if !is_airpods_playback_device(&name) && !is_monitor_playback_device(&name) {
                                return Some(id.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    // Check config.json saved standin_device_id
    if let Some(cfg_id) = load_standin_device_id() {
        if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
            if let Ok(dev) = enumerator.get_device(&cfg_id) {
                if dev.get_state().ok() == Some(wasapi::DeviceState::Active) {
                    if let Ok(name) = dev.get_friendlyname() {
                        if !is_airpods_playback_device(&name) && !is_monitor_playback_device(&name) {
                            if let Ok(mut saved) = SAVED_STANDIN_DEVICE_ID.lock() {
                                *saved = Some(cfg_id.clone());
                            }
                            return Some(cfg_id);
                        }
                    }
                }
            }
        }
    }

    // 2nd Priority: Real physical speakers / sound cards (e.g. 喇叭, Speaker, Realtek)
    if let Some(speaker) = non_airpods.iter().find(|d| {
        let n = d.name.to_lowercase();
        (n.contains("喇叭")
            || n.contains("speaker")
            || n.contains("realtek")
            || n.contains("high definition audio"))
            && !is_monitor_playback_device(&d.name)
    }) {
        let id = speaker.id.clone();
        update_saved_standin_device(&id);
        return Some(id);
    }

    // 3rd Priority: Any non-monitor audio device
    if let Some(other) = non_airpods.iter().find(|d| !is_monitor_playback_device(&d.name)) {
        let id = other.id.clone();
        update_saved_standin_device(&id);
        return Some(id);
    }

    // Last resort fallback
    if let Some(first) = non_airpods.first() {
        let id = first.id.clone();
        return Some(id);
    }
    None
}

/// Finds an active, non-AirPods physical playback endpoint (e.g. Speakers, Realtek, USB Audio, HDMI/DP Monitor)
/// to serve as fallback when the system default is set to AirPods on startup or disconnects.
pub fn find_fallback_playback_device() -> Option<String> {
    let devices = get_audio_render_devices();
    let non_airpods: Vec<_> = devices
        .into_iter()
        .filter(|d| d.id != "default" && !d.id.is_empty() && !is_airpods_playback_device(&d.name))
        .collect();

    // 1st Priority: Physical speakers / sound cards / USB Audio / C-Media
    if let Some(speaker) = non_airpods.iter().find(|d| {
        let n = d.name.to_lowercase();
        n.contains("喇叭")
            || n.contains("speaker")
            || n.contains("realtek")
            || n.contains("high definition audio")
            || n.contains("c-media")
            || n.contains("cmedia")
            || n.contains("usb audio")
    }) {
        return Some(speaker.id.clone());
    }

    // 2nd Priority: Any non-AirPods active audio device (including monitor HDMI/DP)
    if let Some(first) = non_airpods.first() {
        return Some(first.id.clone());
    }

    None
}

/// Triggers a refresh of local audio devices across the UI and backend.
/// Debounced with a secondary delayed query so newly connected Bluetooth devices
/// that take a brief moment to finish driver/endpoint initialization are fully captured.
pub fn trigger_refresh_audio_devices() {
    static REFRESH_PENDING: AtomicBool = AtomicBool::new(false);
    if let Some(tx) = CONTROLLER_CMD_TX.get() {
        let _ = tx.try_send(GuiCmd::RefreshAudioDevices);
        if !REFRESH_PENDING.swap(true, Ordering::SeqCst) {
            let tx_delayed = tx.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(800));
                REFRESH_PENDING.store(false, Ordering::SeqCst);
                let _ = tx_delayed.try_send(GuiCmd::RefreshAudioDevices);
            });
        }
    }
}

pub fn register_local_endpoint_listener(device_id: Option<&str>) {
    unregister_local_endpoint_listener();
    let resolved = resolve_real_endpoint_id(device_id);
    let _com = initialize_mta();
    unsafe {
        let Ok(enumerator): Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return; };
        let device: IMMDevice = match resolved.as_deref() {
            Some(id) if id != "default" && !id.is_empty() => {
                let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
                match enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr())) {
                    Ok(d) => {
                        if d.GetState() != Ok(DEVICE_STATE_ACTIVE) {
                            return;
                        }
                        d
                    }
                    Err(_) => return,
                }
            }
            _ => match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                Ok(d) => d,
                Err(_) => return,
            },
        };

        let Ok(ep_vol): Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None) else { return; };
        let cb: windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback = LocalEndpointVolumeCallback.into();
        if ep_vol.RegisterControlChangeNotify(&cb).is_ok() {
            if let Ok(mut lock) = LOCAL_ENDPOINT_LISTENER.lock() {
                *lock = Some(ActiveLocalEndpointListener {
                    device_id: resolved,
                    endpoint_vol: ep_vol,
                    callback: cb,
                });
            }
        }
    }
}

pub fn unregister_local_endpoint_listener() {
    if let Ok(mut lock) = LOCAL_ENDPOINT_LISTENER.lock() {
        if let Some(active) = lock.take() {
            unsafe {
                let _ = active.endpoint_vol.UnregisterControlChangeNotify(&active.callback);
            }
        }
    }
}

#[windows::core::implement(windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback)]
struct AudioEndpointVolumeCallback {
    device_id: Option<String>,
    endpoint_vol: IAudioEndpointVolume,
    muting: AtomicBool,
}

impl windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback_Impl for AudioEndpointVolumeCallback_Impl {
    fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        // Under state-based protection, only endpoints that are actively intended
        // to be shielded have this callback registered.
        if !pnotify.is_null() {
            if get_audio_shield_state() == AudioShieldState::Idle {
                return Ok(());
            }
            unsafe {
                let ctx = (*pnotify).guidEventContext;
                if ctx != WIN2POD_EVENT_CONTEXT {
                    if !(*pnotify).bMuted.as_bool() {
                        if !self.muting.swap(true, Ordering::SeqCst) {
                            tracing::info!(
                                "Audio endpoint [{:?}] unmuted by system/driver - event-driven clamping back to mute",
                                self.device_id
                            );
                            let _ = self.endpoint_vol.SetMute(true, &WIN2POD_EVENT_CONTEXT);
                            self.muting.store(false, Ordering::SeqCst);
                        }
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
        let id = unsafe { pwstrdeviceid.to_string().ok() };
        tracing::info!("Audio device state changed: {:?}, state={:?}", id, dwnewstate);
        DEFAULT_DEVICE_CHANGED.store(true, Ordering::SeqCst);
        if dwnewstate == DEVICE_STATE_ACTIVE {
            let id_clone = id.clone();
            std::thread::spawn(move || {
                let _com = initialize_mta();
                on_render_endpoint_active_state(id_clone.as_deref());
            });
        }
        trigger_refresh_audio_devices();
        Ok(())
    }

    fn OnDeviceAdded(&self, pwstrdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        let id = unsafe { pwstrdeviceid.to_string().ok() };
        tracing::info!("Audio device added: {:?}", id);
        DEFAULT_DEVICE_CHANGED.store(true, Ordering::SeqCst);
        let id_clone = id.clone();
        std::thread::spawn(move || {
            let _com = initialize_mta();
            on_render_endpoint_active_state(id_clone.as_deref());
        });
        trigger_refresh_audio_devices();
        Ok(())
    }

    fn OnDeviceRemoved(&self, pwstrdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        let id = unsafe { pwstrdeviceid.to_string().ok() };
        tracing::info!("Audio device removed: {:?}", id);
        DEFAULT_DEVICE_CHANGED.store(true, Ordering::SeqCst);
        trigger_refresh_audio_devices();
        Ok(())
    }

    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, _role: ERole, pwstrdefaultdeviceid: &windows::core::PCWSTR) -> windows::core::Result<()> {
        if flow == eRender {
            let id = unsafe { pwstrdefaultdeviceid.to_string().ok() };
            tracing::info!("Default render device changed to: {:?}", id);
            DEFAULT_DEVICE_CHANGED.store(true, Ordering::SeqCst);

            std::thread::spawn(move || {
                let _com = initialize_mta();
                let is_airpods = if let Some(ref dev_id) = id {
                    if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
                        enumerator.get_device(dev_id).ok()
                            .and_then(|d| d.get_friendlyname().ok())
                            .map(|name| is_airpods_playback_device(&name))
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !is_airpods {
                    if let Some(ref dev_id) = id {
                        update_saved_standin_device(dev_id);
                    }
                }

                match get_audio_shield_state() {
                    AudioShieldState::Idle => {
                        if is_airpods {
                            if is_airpods_auto_routing_and_worn() {
                                if let Some(standin_id) = get_standin_device_id() {
                                    tracing::info!("[Idle Shield] Detected default changed to AirPods while worn; reverting to stand-in [{standin_id}]");
                                    let _ = crate::policy_config::set_default_render_device(&standin_id);
                                    shield_endpoint(Some(&standin_id));
                                }
                            }
                        } else {
                            on_hardware_device_change();
                        }
                    }
                    AudioShieldState::AirPlayStreaming => {
                        if is_airpods {
                            if let Some(standin_id) = get_standin_device_id() {
                                tracing::info!("[AirPlayStreaming Shield] Detected default changed to AirPods; reverting to stand-in [{standin_id}]");
                                let _ = crate::policy_config::set_default_render_device(&standin_id);
                                shield_endpoint(Some(&standin_id));
                            }
                        } else {
                            shield_endpoint(id.as_deref());
                        }
                    }
                    AudioShieldState::LocalTarget { ref target_endpoint_id } => {
                        let is_target = id.as_deref().map(|d| d.eq_ignore_ascii_case(target_endpoint_id)).unwrap_or(false);
                        if is_target || is_airpods {
                            tracing::info!("[LocalTarget Shield] Detected default changed to target/AirPods [{id:?}]; enforcing stand-in proxy...");
                            if let Some(standin_id) = get_standin_device_id() {
                                let _ = crate::policy_config::set_default_render_device(&standin_id);
                                shield_endpoint(Some(&standin_id));
                            }
                        } else {
                            shield_endpoint(id.as_deref());
                        }
                    }
                }
                trigger_refresh_audio_devices();
            });
        }
        Ok(())
    }

    fn OnPropertyValueChanged(&self, _pwstrdeviceid: &windows::core::PCWSTR, _key: &PROPERTYKEY) -> windows::core::Result<()> {
        Ok(())
    }
}

fn on_render_endpoint_active_state(device_id: Option<&str>) {
    match get_audio_shield_state() {
        AudioShieldState::Idle => {}
        AudioShieldState::AirPlayStreaming => {
            shield_endpoint(device_id);
        }
        AudioShieldState::LocalTarget { ref target_endpoint_id } => {
            let is_target = device_id.map(|d| d.eq_ignore_ascii_case(target_endpoint_id)).unwrap_or(false);
            if !is_target {
                shield_endpoint(device_id);
            }
        }
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
                    Ok(d) => {
                        if d.GetState() != Ok(DEVICE_STATE_ACTIVE) {
                            return;
                        }
                        d
                    }
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

        // SAFETY BARRIER 1: Never shield or maximize volume on target playback endpoint (e.g. AirPods)
        if let AudioShieldState::LocalTarget { ref target_endpoint_id } = get_audio_shield_state() {
            if let Some(ref k) = key {
                if k.eq_ignore_ascii_case(target_endpoint_id) {
                    tracing::debug!("shield_endpoint: refusing to shield active target endpoint [{k}]");
                    return;
                }
            }
        }

        // SAFETY BARRIER 2: Never shield or maximize volume on any AirPods / Beats device
        let devices = get_audio_render_devices();
        if let Some(ref k) = key {
            if is_airpods_device_or_id(&devices, k) {
                tracing::warn!("shield_endpoint: refusing to shield AirPods device [{k}]");
                return;
            }
        }

        let Ok(ep_vol): Result<IAudioEndpointVolume, _> = device.Activate(CLSCTX_ALL, None) else { return; };

        // 1. Record original state if not yet saved
        if let Ok(mut saved) = SAVED_ENDPOINT_STATES.lock() {
            if !saved.iter().any(|s| s.device_id == key) {
                let mut was_muted = ep_vol.GetMute().map(|b| b.as_bool()).unwrap_or(false);
                let mut orig_volume = ep_vol.GetMasterVolumeLevelScalar().unwrap_or(1.0);

                // If this endpoint is the startup default device, prioritize the golden startup snapshot
                if let Ok(init_id_lock) = INITIAL_DEFAULT_DEVICE_ID.lock() {
                    if let Some(ref init_id) = *init_id_lock {
                        if key.as_deref().map(|k| k.eq_ignore_ascii_case(init_id)).unwrap_or(false) {
                            if let Ok(m_lock) = INITIAL_DEFAULT_DEVICE_MUTED.lock() {
                                if let Some(m) = *m_lock {
                                    was_muted = m;
                                }
                            }
                            if let Ok(v_lock) = INITIAL_DEFAULT_DEVICE_VOLUME.lock() {
                                if let Some(v) = *v_lock {
                                    orig_volume = v;
                                }
                            }
                        }
                    }
                }

                saved.push(SavedEndpointState {
                    device_id: key.clone(),
                    was_muted,
                    orig_volume,
                });
            }
        }

        // 2. Mute endpoint immediately and maximize capture volume to 100% for full dynamic range loopback
        let _ = ep_vol.SetMute(true, &WIN2POD_EVENT_CONTEXT);
        let _ = ep_vol.SetMasterVolumeLevelScalar(1.0, &WIN2POD_EVENT_CONTEXT);
        if let Ok(channel_count) = ep_vol.GetChannelCount() {
            for ch in 0..channel_count {
                let _ = ep_vol.SetChannelVolumeLevelScalar(ch, 1.0, &WIN2POD_EVENT_CONTEXT);
            }
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

/// Shields render endpoints based on the current shielding policy, optionally excluding a designated target endpoint.
fn shield_endpoints_for_state(target_to_exclude: Option<&str>) {
    let _com = initialize_mta();
    unsafe {
        let Ok(enumerator): Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return; };
        
        let default_dev = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).ok();
        let default_id_string = default_dev.as_ref().and_then(|d| d.GetId().ok().and_then(|p| p.to_string().ok()));

        let shield_default = match (&target_to_exclude, &default_id_string) {
            (Some(exc), Some(def_id)) => !exc.eq_ignore_ascii_case(def_id),
            _ => true,
        };

        let default_is_airpods = default_id_string.as_ref().map(|id| {
            let devices = get_audio_render_devices();
            devices.iter().any(|d| d.id.eq_ignore_ascii_case(id) && is_airpods_playback_device(&d.name))
        }).unwrap_or(false);

        if shield_default && !default_is_airpods {
            shield_endpoint(None); // Default device
        }

        if let Ok(collection) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
            if let Ok(count) = collection.GetCount() {
                for i in 0..count {
                    if let Ok(dev) = collection.Item(i) {
                        if let Ok(pwstr) = dev.GetId() {
                            if let Ok(id) = pwstr.to_string() {
                                if let Some(exc) = target_to_exclude {
                                    if id.eq_ignore_ascii_case(exc) {
                                        continue; // Do not shield or mute the target playback device!
                                    }
                                }
                                shield_endpoint(Some(&id));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Shields all currently active render endpoints in the system according to current state
pub fn shield_all_active_render_devices() {
    match get_audio_shield_state() {
        AudioShieldState::AirPlayStreaming => shield_endpoints_for_state(None),
        AudioShieldState::LocalTarget { ref target_endpoint_id } => shield_endpoints_for_state(Some(target_endpoint_id)),
        AudioShieldState::Idle => {}
    }
}

fn teardown_all_shields() {
    set_audio_peak_level(0.0);

    // 1. Unregister all IAudioEndpointVolumeCallback
    if let Ok(mut shields) = ACTIVE_VOLUME_SHIELDS.lock() {
        for s in shields.drain(..) {
            unsafe {
                let _ = s.endpoint_vol.UnregisterControlChangeNotify(&s.callback);
            }
        }
    }

    // 2. Restore all saved endpoint volume & mute states
    let mut saved = match SAVED_ENDPOINT_STATES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    for item in saved.drain(..) {
        let id_ref = item.device_id.as_deref();
        let _ = set_endpoint_volume(id_ref, item.orig_volume);
        let _ = set_endpoint_mute(id_ref, item.was_muted);
    }

    // 3. Guarantee that the startup default device is faithfully restored to its startup volume and mute state
    let orig_id = INITIAL_DEFAULT_DEVICE_ID.lock().ok().and_then(|g| g.clone());
    let orig_muted = INITIAL_DEFAULT_DEVICE_MUTED.lock().ok().and_then(|m| *m).unwrap_or(false);
    let orig_vol = INITIAL_DEFAULT_DEVICE_VOLUME.lock().ok().and_then(|v| *v);
    if let Some(ref id) = orig_id {
        if let Some(vol) = orig_vol {
            let _ = set_endpoint_volume(Some(id), vol);
        }
        let _ = set_endpoint_mute(Some(id), orig_muted);
    }
}

/// Precise state machine controller for audio protection / shielding.
/// Enforces clean exit of old state before applying new state rules.
pub fn set_audio_shield_state(new_state: AudioShieldState) {
    let old_state = {
        let mut current = match CURRENT_SHIELD_STATE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if *current == new_state {
            return;
        }

        tracing::info!("Audio shield state transition: {:?} -> {:?}", *current, new_state);
        let old = current.clone();
        *current = new_state.clone();
        old
    };

    // 1. Exit current state: cleanly teardown all active shields and callbacks
    match old_state {
        AudioShieldState::Idle => {}
        AudioShieldState::AirPlayStreaming | AudioShieldState::LocalTarget { .. } => {
            teardown_all_shields();
        }
    }

    // 2. Enter new state
    match &new_state {
        AudioShieldState::Idle => {
            STREAMING_ACTIVE.store(false, Ordering::SeqCst);
        }
        AudioShieldState::AirPlayStreaming => {
            STREAMING_ACTIVE.store(true, Ordering::SeqCst);
            DEFAULT_DEVICE_CHANGED.store(false, Ordering::SeqCst);
            ensure_notification_client_registered();

            shield_endpoints_for_state(None);
        }
        AudioShieldState::LocalTarget { target_endpoint_id } => {
            STREAMING_ACTIVE.store(false, Ordering::SeqCst);
            DEFAULT_DEVICE_CHANGED.store(false, Ordering::SeqCst);
            ensure_notification_client_registered();

            shield_endpoints_for_state(Some(target_endpoint_id));
            let _ = set_endpoint_mute(Some(target_endpoint_id), false);
        }
    }
}

/// Called on Win32 hardware PnP change notification (WM_DEVICECHANGE) or default device change
pub fn on_hardware_device_change() {
    trigger_refresh_audio_devices();
    match get_audio_shield_state() {
        AudioShieldState::AirPlayStreaming => shield_endpoints_for_state(None),
        AudioShieldState::LocalTarget { ref target_endpoint_id } => {
            shield_endpoints_for_state(Some(target_endpoint_id));
            let target = target_endpoint_id.clone();
            if let Ok(lock) = LOCAL_ENDPOINT_LISTENER.lock() {
                let needs_register = lock.as_ref().map(|l| l.device_id.as_deref() != Some(&target)).unwrap_or(true);
                drop(lock);
                if needs_register {
                    register_local_endpoint_listener(Some(&target));
                    if let Ok(cur_vol) = get_endpoint_volume(Some(&target)) {
                        let is_muted = get_endpoint_mute(Some(&target)).unwrap_or(false);
                        if let Some(tx) = CONTROLLER_CMD_TX.get() {
                            let _ = tx.send_blocking(GuiCmd::LocalEndpointVolumeChanged { volume: cur_vol, is_muted });
                        }
                    }
                }
            }
        }
        AudioShieldState::Idle => {
            if let Ok(lock) = LOCAL_ENDPOINT_LISTENER.lock() {
                let is_default = lock.as_ref().map(|l| l.device_id.is_none()).unwrap_or(true);
                drop(lock);
                if is_default {
                    let real_id = resolve_real_endpoint_id(None);
                    register_local_endpoint_listener(real_id.as_deref());
                    if let Ok(cur_vol) = get_endpoint_volume(real_id.as_deref()) {
                        let is_muted = get_endpoint_mute(real_id.as_deref()).unwrap_or(false);
                        if let Some(tx) = CONTROLLER_CMD_TX.get() {
                            let _ = tx.send_blocking(GuiCmd::LocalEndpointVolumeChanged { volume: cur_vol, is_muted });
                        }
                    }
                }
            }
        }
    }
}

pub fn ensure_notification_client_registered() {
    let mut notif = match NOTIFICATION_CLIENT.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if notif.is_none() {
        let _com = initialize_mta();
        unsafe {
            if let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                let client: IMMNotificationClient = AudioNotificationClient.into();
                if enumerator.RegisterEndpointNotificationCallback(&client).is_ok() {
                    *notif = Some(ActiveNotificationClient {
                        enumerator,
                        client,
                    });
                }
            }
        }
    }
}

pub fn unregister_notification_client() {
    if let Ok(mut notif) = NOTIFICATION_CLIENT.lock() {
        if let Some(active) = notif.take() {
            unsafe {
                let _ = active.enumerator.UnregisterEndpointNotificationCallback(&active.client);
            }
        }
    }
}

pub fn record_and_configure_streaming_source(source: &AudioSource) {
    let should_mute = match source {
        AudioSource::Device(dev_id) => dev_id == "default" || dev_id.is_empty(),
        AudioSource::Application { .. } => true,
    };

    if !should_mute {
        tracing::info!("Selected audio source ({:?}) is non-default device - audio devices will NOT be muted", source);
        set_audio_shield_state(AudioShieldState::Idle);
        return;
    }

    set_audio_shield_state(AudioShieldState::AirPlayStreaming);
}

pub fn record_and_configure_streaming_device(device_id: Option<&str>) {
    let is_system_mix = device_id.is_none() || device_id == Some("default") || device_id.unwrap().is_empty();

    if !is_system_mix {
        tracing::info!("Selected audio device is not system mix ({:?}) - audio devices will NOT be muted", device_id);
        set_audio_shield_state(AudioShieldState::Idle);
        return;
    }

    set_audio_shield_state(AudioShieldState::AirPlayStreaming);
}

#[allow(dead_code)]
pub fn record_and_configure_streaming_speakers() {
    record_and_configure_streaming_device(None);
}

#[allow(dead_code)]
pub fn record_and_mute_speakers() {
    record_and_configure_streaming_device(None);
}

static INITIAL_DEFAULT_DEVICE_ID: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
static INITIAL_DEFAULT_DEVICE_MUTED: std::sync::Mutex<Option<bool>> = std::sync::Mutex::new(None);
static INITIAL_DEFAULT_DEVICE_VOLUME: std::sync::Mutex<Option<f32>> = std::sync::Mutex::new(None);

pub fn resolve_real_endpoint_id(device_id: Option<&str>) -> Option<String> {
    if let Some(id) = device_id {
        if id != "default" && !id.is_empty() {
            return Some(id.to_string());
        }
    }
    // Caller wants default endpoint:
    // Check if Windows default device is currently the stand-in proxy (e.g. C-Media)
    if let Some(standin_id) = get_standin_device_id() {
        if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
            if let Ok(def_dev) = enumerator.get_default_device(&wasapi::Direction::Render) {
                if let Ok(cur_def_id) = def_dev.get_id() {
                    if cur_def_id.eq_ignore_ascii_case(&standin_id) {
                        if let Ok(lock) = INITIAL_DEFAULT_DEVICE_ID.lock() {
                            if let Some(ref real_id) = *lock {
                                if !real_id.eq_ignore_ascii_case(&standin_id) {
                                    return Some(real_id.clone());
                                }
                            }
                        }
                    } else {
                        return Some(cur_def_id);
                    }
                }
            }
        }
    }
    None
}

pub fn update_saved_real_default_device(id: &str) {
    if id == "default" || id.is_empty() {
        return;
    }
    if let Ok(mut lock) = INITIAL_DEFAULT_DEVICE_ID.lock() {
        tracing::info!("[Default Device] Saving real default device snapshot: [{id}]");
        *lock = Some(id.to_string());
    }
}

/// Captures the system default audio playback endpoint and its volume/mute state as early as possible on startup.
/// Note: Must run on an isolated worker thread so that `initialize_mta` does not contaminate
/// the main GUI thread's COM apartment mode (which requires STA / OleInitialize for winit/eframe).
pub fn capture_initial_default_device() {
    let handle = std::thread::spawn(|| {
        let _com = wasapi::initialize_mta();
        let already_captured = INITIAL_DEFAULT_DEVICE_ID.lock().map(|l| l.is_some()).unwrap_or(false);
        if already_captured {
            return;
        }

        let mut target_id = None;
        let mut target_muted = false;
        let mut target_vol = 1.0;

        if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
            if let Ok(def_dev) = enumerator.get_default_device(&wasapi::Direction::Render) {
                if let Ok(id) = def_dev.get_id() {
                    let name = def_dev.get_friendlyname().unwrap_or_default();
                    let is_airpods = is_airpods_playback_device(&name);
                    if !is_airpods {
                        let init_muted = get_endpoint_mute(Some(&id)).unwrap_or(false);
                        let init_vol = get_endpoint_volume(Some(&id)).unwrap_or(1.0);
                        tracing::info!(
                            "[Startup] Captured initial Windows default playback endpoint: [{}] ({}) (muted={}, volume={:.2})",
                            id, name, init_muted, init_vol
                        );
                        target_id = Some(id);
                        target_muted = init_muted;
                        target_vol = init_vol;
                    } else {
                        // Default was AirPods on startup:
                        // Find a non-AirPods physical speaker as initial fallback
                        if let Some(fb_id) = find_fallback_playback_device() {
                            let fb_muted = get_endpoint_mute(Some(&fb_id)).unwrap_or(false);
                            let fb_vol = get_endpoint_volume(Some(&fb_id)).unwrap_or(1.0);
                            tracing::info!(
                                "[Startup] Windows default was [{}] ({}); captured fallback device [{fb_id}] as initial fallback (muted={}, vol={:.2})",
                                id, name, fb_muted, fb_vol
                            );
                            target_id = Some(fb_id);
                            target_muted = fb_muted;
                            target_vol = fb_vol;
                        }
                    }
                }
            }
        }

        if let Some(final_id) = target_id {
            if let Ok(mut lock) = INITIAL_DEFAULT_DEVICE_ID.lock() {
                if lock.is_none() {
                    *lock = Some(final_id.clone());
                }
            }
            if let Ok(mut m_lock) = INITIAL_DEFAULT_DEVICE_MUTED.lock() {
                if m_lock.is_none() {
                    *m_lock = Some(target_muted);
                }
            }
            if let Ok(mut v_lock) = INITIAL_DEFAULT_DEVICE_VOLUME.lock() {
                if v_lock.is_none() {
                    *v_lock = Some(target_vol);
                }
            }
            update_saved_standin_device(&final_id);
        }
    });
    let _ = handle.join();
}

/// Restores the system default audio playback endpoint back to what was active at startup or before proxy handoff,
/// and faithfully restores its startup mute state and master volume.
pub fn restore_initial_default_device() {
    set_audio_shield_state(AudioShieldState::Idle);
    let _com = wasapi::initialize_mta();
    let orig_id = match INITIAL_DEFAULT_DEVICE_ID.lock() {
        Ok(g) => g.clone(),
        Err(p) => p.into_inner().clone(),
    };
    if let Some(id) = orig_id {
        let devices = get_audio_render_devices();
        // Under NO circumstances should exit restore Windows default to AirPods
        let target_device_id = if is_airpods_device_or_id(&devices, &id) {
            tracing::warn!("[Default Device] Captured startup default was AirPods [{}]; falling back to physical speaker on exit", id);
            find_fallback_playback_device()
        } else {
            Some(id)
        };

        let Some(final_target_id) = target_device_id else {
            return;
        };

        let orig_muted = INITIAL_DEFAULT_DEVICE_MUTED.lock().ok().and_then(|m| *m).unwrap_or(false);
        let orig_vol = INITIAL_DEFAULT_DEVICE_VOLUME.lock().ok().and_then(|v| *v);

        // Verify if target device is still present and active
        let (actual_id, is_fallback) = if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
            let orig_active = enumerator.get_device(&final_target_id).ok()
                .and_then(|d| d.get_state().ok())
                .map(|s| s == wasapi::DeviceState::Active)
                .unwrap_or(false);
            if orig_active {
                (final_target_id.clone(), false)
            } else {
                let fb = find_fallback_playback_device();
                (fb.unwrap_or_else(|| final_target_id.clone()), true)
            }
        } else {
            (final_target_id.clone(), false)
        };

        if is_fallback {
            tracing::warn!(
                "[Default Device] Target device was disconnected before exit. Falling back to active device: [{actual_id}]",
            );
        } else {
            tracing::info!(
                "[Default Device] Reverting Windows default playback endpoint to: [{actual_id}] (mute={}, volume={:?})",
                orig_muted, orig_vol
            );
        }

        let _ = crate::policy_config::set_default_render_device(&actual_id);
        // Restore mute and volume ONLY on the physical speaker, never on AirPods or blind default
        // Safety: ALWAYS lower volume before unmuting to avoid 100% volume blast!
        if !is_airpods_device_or_id(&devices, &actual_id) {
            if !is_fallback {
                if let Some(vol) = orig_vol {
                    let _ = set_endpoint_volume(Some(&actual_id), vol);
                }
                let _ = set_endpoint_mute(Some(&actual_id), orig_muted);
            } else {
                if let Some(vol) = orig_vol {
                    let _ = set_endpoint_volume(Some(&actual_id), vol);
                } else {
                    // Safe fallback volume if unknown, never leave unmuted at 100%
                    let _ = set_endpoint_volume(Some(&actual_id), 0.3);
                }
                let _ = set_endpoint_mute(Some(&actual_id), false);
            }
        }
    }
}

pub fn restore_original_speaker_state() {
    set_audio_shield_state(AudioShieldState::Idle);
    let orig_id = INITIAL_DEFAULT_DEVICE_ID.lock().ok().and_then(|g| g.clone());
    let orig_muted = INITIAL_DEFAULT_DEVICE_MUTED.lock().ok().and_then(|m| *m).unwrap_or(false);
    let orig_vol = INITIAL_DEFAULT_DEVICE_VOLUME.lock().ok().and_then(|v| *v);
    if let Some(ref id) = orig_id {
        let devices = get_audio_render_devices();
        if !is_airpods_device_or_id(&devices, id) {
            if let Some(vol) = orig_vol {
                let _ = set_endpoint_volume(Some(id), vol);
            }
            let _ = set_endpoint_mute(Some(id), orig_muted);
        }
    }
}

pub fn restore_original_speaker_mute() {
    restore_original_speaker_state();
}

extern "C" fn on_process_exit() {
    unregister_local_endpoint_listener();
    unregister_notification_client();
    restore_original_speaker_state();
    restore_initial_default_device();
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
    let resolved = resolve_real_endpoint_id(device_id);
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let clamped = level.clamp(0.0, 1.0);

        let set_vol = |ep_vol: &IAudioEndpointVolume| {
            if let Ok(channel_count) = ep_vol.GetChannelCount() {
                for ch in 0..channel_count {
                    let _ = ep_vol.SetChannelVolumeLevelScalar(ch, clamped, &WIN2POD_EVENT_CONTEXT);
                }
            }
            let _ = ep_vol.SetMasterVolumeLevelScalar(clamped, &WIN2POD_EVENT_CONTEXT);
        };

        if let Some(id) = resolved.as_deref() {
            let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
            let device = enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr()))?;
            let ep_vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
            set_vol(&ep_vol);
        } else {
            if let Ok(dev) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                if let Ok(ep_vol) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    set_vol(&ep_vol);
                }
            }
            if let Ok(dev) = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) {
                if let Ok(ep_vol) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    set_vol(&ep_vol);
                }
            }
        }
    }
    Ok(())
}

/// Query an audio endpoint's master volume (0.0 to 1.0). If device_id is None, queries default endpoint.
pub fn get_endpoint_volume(device_id: Option<&str>) -> anyhow::Result<f32> {
    let resolved = resolve_real_endpoint_id(device_id);
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device: IMMDevice = match resolved.as_deref() {
            Some(id) if id != "default" && !id.is_empty() => {
                let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
                enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr()))?
            }
            _ => enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?,
        };
        let ep_vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        let vol = ep_vol.GetMasterVolumeLevelScalar()?;
        Ok(vol)
    }
}

/// Control an audio endpoint's mute state. If device_id is None, controls default endpoint.
pub fn set_endpoint_mute(device_id: Option<&str>, mute: bool) -> anyhow::Result<()> {
    let resolved = resolve_real_endpoint_id(device_id);
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        if let Some(id) = resolved.as_deref() {
            let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
            let device = enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr()))?;
            let ep_vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
            ep_vol.SetMute(mute, &WIN2POD_EVENT_CONTEXT)?;
        } else {
            if let Ok(dev) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                if let Ok(ep_vol) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    let _ = ep_vol.SetMute(mute, &WIN2POD_EVENT_CONTEXT);
                }
            }
            if let Ok(dev) = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) {
                if let Ok(ep_vol) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    let _ = ep_vol.SetMute(mute, &WIN2POD_EVENT_CONTEXT);
                }
            }
        }
    }
    Ok(())
}

/// Query an audio endpoint's mute state. If device_id is None, queries default endpoint.
pub fn get_endpoint_mute(device_id: Option<&str>) -> anyhow::Result<bool> {
    let resolved = resolve_real_endpoint_id(device_id);
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device: IMMDevice = match resolved.as_deref() {
            Some(id) if id != "default" && !id.is_empty() => {
                let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
                enumerator.GetDevice(windows::core::PCWSTR(w_id.as_ptr()))?
            }
            _ => enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?,
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
    pub eq_params: Arc<EqParams>,
    pub sound_enhancer_params: Arc<SoundEnhancerParams>,
    pub warmth_enhancer_params: Arc<WarmthEnhancerParams>,
}

impl Session {
    pub async fn start(
        device_items: Vec<DeviceItem>,
        volume: f32,
        device_volumes: &std::collections::HashMap<String, f32>,
        audio_source: AudioSource,
        low_latency: bool,
        eq_enabled: bool,
        eq_gains: &[f32; 10],
        sound_enhancer_mode: SoundEnhancerMode,
        warmth_enhancer_mode: WarmthEnhancerMode,
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

        let eq_config = EqConfig::ten_band();
        let eq_params = Arc::new(EqParams::new(eq_config.num_bands()));
        eq_params.set_bypass(!eq_enabled);
        for (i, &g) in eq_gains.iter().enumerate() {
            eq_params.set_gain_db(i, g);
        }
        let sound_enhancer_params = Arc::new(SoundEnhancerParams::new(sound_enhancer_mode));
        let warmth_enhancer_params = Arc::new(WarmthEnhancerParams::new(warmth_enhancer_mode));

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

            // 2. Collect peer addresses (device IPs + our local IP on matching physical interface)
            let mut peer_addresses: Vec<String> = clean_devices
                .iter()
                .flat_map(|d| d.addresses.iter().find(|a| a.is_ipv4()).map(|a| a.to_string()))
                .collect();
            let local_ip = connections[0].local_addr().map(|sa| sa.ip())
                .or_else(|| {
                    clean_devices[0].addresses.iter().find(|a| a.is_ipv4())
                        .and_then(|target_ip| airplay_core::net::select_physical_local_ip_for_target(target_ip))
                });
            if let Some(ip) = local_ip {
                peer_addresses.push(ip.to_string());
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
            streamer.set_eq_params(eq_config.clone(), eq_params.clone()).await;
            streamer.set_sound_enhancer_params(sound_enhancer_params.clone()).await;
            streamer.set_warmth_enhancer_params(warmth_enhancer_params.clone()).await;

            // 9. Start audio capture with SoftwareAudioMixer support
            let (hp_sender, decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 256);
            let mixer = Arc::new(SoftwareAudioMixer::new(hp_sender, RATE, CHANNELS));

            let (sender, mut cap_decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 256);
            let session_sender = sender.clone();
            let cap_stop = Arc::new(AtomicBool::new(false));
            let stop_clone = cap_stop.clone();
            let counters = Arc::new(DiagnosticCounters::new());
            let counters_clone = counters.clone();

            let cap_source = audio_source.clone();
            let cap_handle = std::thread::spawn(move || {
                match cap_source {
                    AudioSource::Device(dev_id) => {
                        if let Err(e) = run_capture(sender, stop_clone, counters_clone, dev_id) {
                            tracing::error!("Audio capture error: {e:#}");
                        }
                    }
                    AudioSource::Application { pid, exe_name, .. } => {
                        let _com = wasapi::initialize_mta();
                        if let Err(e) = crate::process_loopback::run_process_loopback_capture(pid, Some(exe_name), sender, stop_clone, counters_clone) {
                            tracing::error!("Application audio capture error: {e:#}");
                        }
                    }
                }
            });

            let bridge_mixer = mixer.clone();
            let bridge_stop = cap_stop.clone();
            std::thread::Builder::new()
                .name("homepod-multi-mixer-bridge".into())
                .spawn(move || {
                    while !bridge_stop.load(Ordering::Relaxed) {
                        match cap_decoder.decode_frame() {
                            Ok(Some(frame)) => {
                                bridge_mixer.process_primary(LivePcmFrame {
                                    samples: frame.samples,
                                    channels: frame.channels,
                                    sample_rate: frame.sample_rate,
                                });
                            }
                            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                            Err(_) => break,
                        }
                    }
                })
                .ok();

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
            if let Some(mut rx) = primary_conn.take_remote_control_rx() {
                tokio::spawn(async move {
                    while let Some(cmd) = rx.recv().await {
                        forward_homepod_remote_command(cmd);
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
                if let Some(mut rx) = sec.take_remote_control_rx() {
                    tokio::spawn(async move {
                        while let Some(cmd) = rx.recv().await {
                            forward_homepod_remote_command(cmd);
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
                eq_params: eq_params.clone(),
                sound_enhancer_params: sound_enhancer_params.clone(),
                warmth_enhancer_params: warmth_enhancer_params.clone(),
            })
        } else {
            // --- Single device streaming path ---
            let single_vol = sorted_items
                .first()
                .and_then(|item| device_volumes.get(&item.id_string()).copied())
                .unwrap_or(volume);

            let target_dev = clean_devices.into_iter().next().unwrap();
            tracing::info!("Starting single device streaming to {} ({:?})", target_dev.name, target_dev.addresses);
            let mut conn = Connection::connect_auto(target_dev, config, "3939").await?;
            conn.setup().await?;
            conn.set_eq_params(eq_config, eq_params.clone());
            conn.set_sound_enhancer_params(sound_enhancer_params.clone());
            conn.set_warmth_enhancer_params(warmth_enhancer_params.clone());
            let _ = conn.set_volume(single_vol).await;

            let (hp_sender, decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 256);
            let mixer = Arc::new(SoftwareAudioMixer::new(hp_sender, RATE, CHANNELS));

            let (sender, mut cap_decoder) = LiveAudioDecoder::create_pair(RATE, CHANNELS, 256);
            let session_sender = sender.clone();
            let cap_stop = Arc::new(AtomicBool::new(false));
            let stop_clone = cap_stop.clone();
            let counters = Arc::new(DiagnosticCounters::new());
            let counters_clone = counters.clone();

            let cap_source = audio_source.clone();
            let cap_handle = std::thread::spawn(move || {
                match cap_source {
                    AudioSource::Device(dev_id) => {
                        if let Err(e) = run_capture(sender, stop_clone, counters_clone, dev_id) {
                            tracing::error!("Audio capture error: {e:#}");
                        }
                    }
                    AudioSource::Application { pid, exe_name, .. } => {
                        let _com = wasapi::initialize_mta();
                        if let Err(e) = crate::process_loopback::run_process_loopback_capture(pid, Some(exe_name), sender, stop_clone, counters_clone) {
                            tracing::error!("Application audio capture error: {e:#}");
                        }
                    }
                }
            });

            let bridge_mixer = mixer.clone();
            let bridge_stop = cap_stop.clone();
            std::thread::Builder::new()
                .name("homepod-single-mixer-bridge".into())
                .spawn(move || {
                    while !bridge_stop.load(Ordering::Relaxed) {
                        match cap_decoder.decode_frame() {
                            Ok(Some(frame)) => {
                                bridge_mixer.process_primary(LivePcmFrame {
                                    samples: frame.samples,
                                    channels: frame.channels,
                                    sample_rate: frame.sample_rate,
                                });
                            }
                            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                            Err(_) => break,
                        }
                    }
                })
                .ok();

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
            if let Some(mut rx) = conn.take_remote_control_rx() {
                tokio::spawn(async move {
                    while let Some(cmd) = rx.recv().await {
                        forward_homepod_remote_command(cmd);
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
                eq_params,
                sound_enhancer_params,
                warmth_enhancer_params,
            })
        }
    }

    pub fn switch_audio_source(&mut self, audio_source: AudioSource) {
        tracing::info!("Hot-swapping audio source to [{:?}] (AirPlay stream uninterrupted)", audio_source);
        self.cap_stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.cap_handle.take() {
            let _ = h.join();
        }

        let new_stop = Arc::new(AtomicBool::new(false));
        let stop_clone = new_stop.clone();
        let sender_clone = self.sender.clone();
        let counters_clone = self.counters.clone();

        let new_handle = std::thread::spawn(move || {
            match audio_source {
                AudioSource::Device(dev_id) => {
                    if let Err(e) = run_capture(sender_clone, stop_clone, counters_clone, dev_id) {
                        tracing::error!("Audio capture error: {e:#}");
                    }
                }
                AudioSource::Application { pid, exe_name, .. } => {
                    let _com = wasapi::initialize_mta();
                    if let Err(e) = crate::process_loopback::run_process_loopback_capture(
                        pid,
                        Some(exe_name),
                        sender_clone,
                        stop_clone,
                        counters_clone,
                    ) {
                        tracing::error!("Application audio capture error: {e:#}");
                    }
                }
            }
        });

        self.cap_stop = new_stop;
        self.cap_handle = Some(new_handle);
    }

    pub fn switch_audio_device(&mut self, audio_device_id: String) {
        self.switch_audio_source(AudioSource::Device(audio_device_id));
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

    /// Sends track metadata to all connected speakers so HomePod touch gestures (Next/Prev) are enabled.
    pub async fn send_track_info(&mut self, title: &str, artist: &str, album: &str) {
        let _ = self.conn.send_track_info(title, artist, album).await;
        for sec in &mut self.secondary_conns {
            let _ = sec.send_track_info(title, artist, album).await;
        }
    }

    /// Sends track progress timeline to all connected speakers.
    pub async fn send_progress(&mut self, pos: f64, dur: f64) {
        let _ = self.conn.send_progress(pos, dur).await;
        for sec in &mut self.secondary_conns {
            let _ = sec.send_progress(pos, dur).await;
        }
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

    /// Query latest hardware volume from active speakers via RTSP GET_PARAMETER
    pub async fn poll_hardware_volume(&mut self) {
        let _ = self.conn.get_volume().await;
        for sec in &mut self.secondary_conns {
            let _ = sec.get_volume().await;
        }
    }

    pub async fn stop(mut self) {
        set_audio_peak_level(0.0);
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
        for mut c in std::mem::take(&mut self.secondary_conns) {
            let _ = tokio::time::timeout(Duration::from_secs(3), c.disconnect()).await;
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {}
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

pub fn broadcast_state(shared_state: &Arc<RwLock<ControllerState>>, s: &ControllerState) {
    if let Ok(mut lock) = shared_state.write() {
        *lock = s.clone();
    }
    crate::tray::notify_tray_state();
    crate::gui::request_ui_repaint();
}

struct AoRunner {
    stop_flag: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl AoRunner {
    async fn stop(self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        let handle = self.handle;
        // Offload blocking thread join to spawn_blocking with a timeout to guarantee
        // that Tokio's async worker thread is NEVER blocked or starved!
        let _ = tokio::time::timeout(
            Duration::from_millis(800),
            tokio::task::spawn_blocking(move || {
                let _ = handle.join();
            }),
        )
        .await;
    }
}

enum AudioRouteRuntime {
    Idle,
    AoMode(AoRunner),
    Streaming(Session),
}

impl AudioRouteRuntime {
    fn is_streaming(&self) -> bool {
        matches!(self, Self::Streaming(_))
    }

    fn session_mut(&mut self) -> Option<&mut Session> {
        match self {
            Self::Streaming(s) => Some(s),
            _ => None,
        }
    }

    fn session(&self) -> Option<&Session> {
        match self {
            Self::Streaming(s) => Some(s),
            _ => None,
        }
    }

    async fn stop(&mut self) {
        match std::mem::replace(self, Self::Idle) {
            Self::Streaming(session) => {
                session.stop().await;
            }
            Self::AoMode(runner) => {
                runner.stop().await;
            }
            Self::Idle => {}
        }
        crate::spectrum::reset_spectrum_levels();
        set_audio_peak_level(0.0);
    }
}

fn spawn_ao_runner(
    target_source: &AudioSource,
    ao_out: &str,
    global_eq_params: &Arc<EqParams>,
    global_sound_enhancer_params: &Arc<SoundEnhancerParams>,
    global_warmth_enhancer_params: &Arc<WarmthEnhancerParams>,
    audio_devices: &[AudioDeviceItem],
) -> (AoRunner, DeviceStatus) {
    let out_device_name = if ao_out != "default" && !ao_out.is_empty() {
        audio_devices
            .iter()
            .find(|d| d.id == ao_out)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| ao_out.to_string())
    } else {
        "default".to_string()
    };

    match target_source {
        AudioSource::Application { pid, exe_name, .. } => {
            let stop_flag = Arc::new(AtomicBool::new(false));
            let stop_clone = stop_flag.clone();
            let eq_clone = global_eq_params.clone();
            let enh_clone = global_sound_enhancer_params.clone();
            let warmth_clone = global_warmth_enhancer_params.clone();
            let target_pid = *pid;
            let target_exe = exe_name.clone();
            let out_device = if ao_out != "default" && !ao_out.is_empty() {
                Some(ao_out.to_string())
            } else {
                None
            };

            let handle = std::thread::spawn(move || {
                let _ = crate::local_renderer::run_ao_process_engine(
                    target_pid,
                    Some(target_exe),
                    out_device,
                    true,
                    stop_clone,
                    eq_clone,
                    enh_clone,
                    warmth_clone,
                );
            });
            (AoRunner { stop_flag, handle }, DeviceStatus::AoMode(out_device_name))
        }
        AudioSource::Device(dev_id) => {
            let stop_flag = Arc::new(AtomicBool::new(false));
            let stop_clone = stop_flag.clone();
            let eq_clone = global_eq_params.clone();
            let enh_clone = global_sound_enhancer_params.clone();
            let warmth_clone = global_warmth_enhancer_params.clone();
            let dev_id_clone = dev_id.clone();
            let out_device = if ao_out != "default" && !ao_out.is_empty() {
                Some(ao_out.to_string())
            } else {
                None
            };

            let handle = std::thread::spawn(move || {
                let _ = crate::local_renderer::run_ao_system_engine(
                    dev_id_clone,
                    out_device,
                    stop_clone,
                    eq_clone,
                    enh_clone,
                    warmth_clone,
                );
            });
            (AoRunner { stop_flag, handle }, DeviceStatus::AoMode(out_device_name))
        }
    }
}

async fn transition_to_local_route(
    runtime: &mut AudioRouteRuntime,
    state: &mut ControllerState,
    saved_homepod_volume: &mut f32,
    global_eq_params: &Arc<EqParams>,
    global_sound_enhancer_params: &Arc<SoundEnhancerParams>,
    global_warmth_enhancer_params: &Arc<WarmthEnhancerParams>,
    preserve_homepod_vol: bool,
    shared_state: &Arc<RwLock<ControllerState>>,
) {
    // 1. Immediately reset connection metadata and optimistically broadcast non-streaming status
    state.connected_name = None;
    state.connected_device_ids.clear();
    state.device_volumes.clear();
    state.diagnostics = StreamDiagnostics::default();
    update_thread_execution_state(state.prevent_sleep, false);

    let has_target_device = !state.ao_output_device.is_empty() && state.ao_output_device != "default";
    let preliminary_status = if state.auto_ao_mode || has_target_device {
        let out_device_name = if has_target_device {
            state.audio_devices
                .iter()
                .find(|d| d.id == state.ao_output_device)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| state.ao_output_device.clone())
        } else {
            "default".to_string()
        };
        DeviceStatus::AoMode(out_device_name)
    } else {
        DeviceStatus::Idle
    };
    state.status = preliminary_status;
    broadcast_state(shared_state, state);

    // 2. Stop audio route runtime (RTSP TEARDOWN and local thread join)
    runtime.stop().await;

    let has_target_device = !state.ao_output_device.is_empty() && state.ao_output_device != "default";

    if has_target_device {
        if let Some(standin_id) = get_standin_device_id() {
            let is_cur_standin = if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
                if let Ok(def_dev) = enumerator.get_default_device(&wasapi::Direction::Render) {
                    let def_id = def_dev.get_id().unwrap_or_default();
                    if !def_id.eq_ignore_ascii_case(&standin_id) && !is_airpods_device_or_id(&state.audio_devices, &def_id) {
                        tracing::info!("[Pre-Proxy Snapshot] Capturing real default device before switching to stand-in: [{def_id}]");
                        update_saved_real_default_device(&def_id);
                    }
                    def_id.eq_ignore_ascii_case(&standin_id)
                } else {
                    false
                }
            } else {
                false
            };
            if !is_cur_standin {
                tracing::info!("[AO Mode] Setting Windows default endpoint to stand-in device [{standin_id}]");
                let _ = crate::policy_config::set_default_render_device(&standin_id);
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            }
        }
    } else {
        // Not routing to a target device (returning to default / idle)
        // Never forcibly change the user's Windows default device.
        // Only restore if Windows is currently diverted to stand-in device!
        let is_cur_standin = if let Some(standin_id) = get_standin_device_id() {
            if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
                if let Ok(def_dev) = enumerator.get_default_device(&wasapi::Direction::Render) {
                    def_dev.get_id().unwrap_or_default().eq_ignore_ascii_case(&standin_id)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        if is_cur_standin {
            restore_initial_default_device();
        }
    }

    // Transition audio shield state machine
    if has_target_device {
        set_audio_shield_state(AudioShieldState::LocalTarget {
            target_endpoint_id: state.ao_output_device.clone(),
        });
        if state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed() {
            if let Some(standin_id) = get_standin_device_id() {
                shield_endpoint(Some(&standin_id));
            }
        }
    } else {
        set_audio_shield_state(AudioShieldState::Idle);
    }

    if !preserve_homepod_vol {
        *saved_homepod_volume = state.volume;
        save_volume(*saved_homepod_volume);
    }

    let local_dev = if has_target_device {
        Some(state.ao_output_device.as_str())
    } else {
        None
    };

    if let Some(target_dev) = local_dev {
        // When routing to a designated target (e.g. AirPods), ensure the target itself is not muted
        let _ = set_endpoint_mute(Some(target_dev), false);
        state.is_muted = false;
        set_source_mute(false);
    } else {
        // Returning to default endpoint: sync with endpoint's real mute state without forcing unmute
        if let Ok(is_muted) = get_endpoint_mute(None) {
            state.is_muted = is_muted;
            set_source_mute(is_muted);
        }
    }

    if let Ok(cur_vol) = get_endpoint_volume(local_dev) {
        state.volume = cur_vol;
    }
    // Always register listener on the active audio device (AirPods if local_dev, or real default speakers)
    register_local_endpoint_listener(local_dev);

    let should_enable_ao = state.auto_ao_mode || has_target_device;
    if should_enable_ao {
        tracing::info!("[Transition] Spawning AO runner for target [{:?}]...", state.ao_output_device);
        let (runner, status) = spawn_ao_runner(
            &state.audio_source,
            &state.ao_output_device,
            global_eq_params,
            global_sound_enhancer_params,
            global_warmth_enhancer_params,
            &state.audio_devices,
        );
        *runtime = AudioRouteRuntime::AoMode(runner);
        state.status = status;
    } else {
        *runtime = AudioRouteRuntime::Idle;
        state.status = DeviceStatus::Idle;
    }

    broadcast_state(shared_state, state);
}

#[derive(Debug, Clone, PartialEq)]
enum InterruptedRoute {
    /// HomePod AirPlay streaming was interrupted by AirPods
    HomePod,
}

async fn restore_interrupted_route(
    interrupted_route: Option<InterruptedRoute>,
    cmd_tx: &async_channel::Sender<GuiCmd>,
    state: &mut ControllerState,
) {
    tracing::info!("[Interrupted Route] Restoring route snapshot: {:?}", interrupted_route);
    match interrupted_route {
        Some(InterruptedRoute::HomePod) => {
            let homepod_targets = load_last_connected_device()
                .and_then(|ref last_target| find_matching_devices(&state.devices, last_target));

            if let Some(targets) = homepod_targets {
                tracing::info!(
                    "[Interrupted Route] Restoring interrupted HomePod playback to ({})",
                    targets.iter().map(|d| d.name.as_str()).collect::<Vec<_>>().join(", ")
                );
                state.ao_output_device = "default".to_string();
                save_ao_output_device("default");
                let _ = cmd_tx.send(GuiCmd::Connect(targets)).await;
            }
        }
        None => {
            tracing::info!("[Interrupted Route] Previous route was local audio - letting Windows natively handle endpoint switching without intervention");
        }
    }
}

async fn try_route_to_airpods(
    runtime: &mut AudioRouteRuntime,
    state: &mut ControllerState,
    saved_homepod_volume: &mut f32,
    global_eq_params: &Arc<EqParams>,
    global_sound_enhancer_params: &Arc<SoundEnhancerParams>,
    global_warmth_enhancer_params: &Arc<WarmthEnhancerParams>,
    shared_state: &Arc<RwLock<ControllerState>>,
    interrupted_route: &mut Option<InterruptedRoute>,
    airpods_paused_at: &mut Option<tokio::time::Instant>,
    cmd_tx: &async_channel::Sender<GuiCmd>,
    is_initial_wear: bool,
) -> bool {
    let airpods_dev = find_best_airpods_device(&state.audio_devices);

    if let Some(dev) = airpods_dev {
        if matches!(
            state.notice_message,
            Some((NoticeKind::AirPodsOccupied, _)) | Some((NoticeKind::AirPodsOccupiedFallbackHomePod, _))
        ) {
            state.notice_message = None;
        }

        let was_streaming = runtime.is_streaming();
        // AirPods Priority: snapshot HomePod stream if currently streaming
        if was_streaming {
            tracing::info!(
                "[AirPods Priority] Active AirPods device found ({}) while HomePod streaming - snapshot interrupted route as HomePod",
                dev.name
            );
            *interrupted_route = Some(InterruptedRoute::HomePod);
        }

        if state.ao_output_device != dev.id || was_streaming {
            state.ao_output_device = dev.id.clone();

            transition_to_local_route(
                runtime,
                state,
                saved_homepod_volume,
                global_eq_params,
                global_sound_enhancer_params,
                global_warmth_enhancer_params,
                true,
                shared_state,
            ).await;
        }

        // Check if within the auto-resume memory window (Apple standard: 60 seconds)
        if state.airpods_auto_pause {
            const AUTO_RESUME_TIMEOUT: Duration = Duration::from_secs(60);
            if let Some(paused_time) = airpods_paused_at.take() {
                if paused_time.elapsed() <= AUTO_RESUME_TIMEOUT {
                    tracing::info!(
                        "[AirPods] Auto-resuming media playback (paused {:?} ago, within 60s window)",
                        paused_time.elapsed()
                    );
                    crate::handoff::arm(crate::handoff::HandoffTarget::AirPods, Duration::from_secs(6));
                    crate::handoff::on_route_ready();
                } else {
                    tracing::info!(
                        "[AirPods] Earbud put back after {:?} (> 60s timeout) - auto-resume expired, remains paused",
                        paused_time.elapsed()
                    );
                }
            } else {
                tracing::info!("[AirPods] Earbud put back, but media was not paused by ear removal - remains paused");
            }
        }
        true
    } else {
        if is_initial_wear {
            tracing::warn!(
                "[AirPods] Earbuds worn but active Windows audio endpoint not found (device occupied by iPhone or disconnected). Preserving previous route."
            );
            state.notice_message = Some((
                NoticeKind::AirPodsOccupied,
                std::time::Instant::now(),
            ));

            if !runtime.is_streaming() && interrupted_route.is_some() {
                restore_interrupted_route(
                    interrupted_route.take(),
                    cmd_tx,
                    state,
                ).await;
            }
        }
        false
    }
}

pub fn spawn_controller(
    cmd_tx: async_channel::Sender<GuiCmd>,
    cmd_rx: AsyncReceiver<GuiCmd>,
    shared_state: Arc<RwLock<ControllerState>>,
) -> JoinHandle<()> {
    SHARED_STATE.set(shared_state.clone()).ok();
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
            crate::snapping::SNAPPING_ENABLED.store(state.window_snapping, std::sync::atomic::Ordering::SeqCst);
            crate::snapping::SNAP_DISTANCE.store(state.snap_distance, std::sync::atomic::Ordering::SeqCst);
            state.audio_devices = get_audio_render_devices();
            state.audio_apps = crate::app_enumerator::get_audio_applications();

            state.ao_output_device = "default".to_string();
            broadcast(&state);

            let mut runtime = AudioRouteRuntime::Idle;
            let mut current_connected_devices: Vec<DeviceItem> = Vec::new();
            let mut tick_interval = tokio::time::interval(Duration::from_millis(500));
            tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut keepalive_interval = tokio::time::interval(Duration::from_millis(1500));
            keepalive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let mut last_reconnect_time = tokio::time::Instant::now() - Duration::from_secs(10);
            let mut is_refreshing_devices = false;
            let mut airpods_paused_at: Option<tokio::time::Instant> = None;
            let mut airpods_prev_worn_count: u8 = 0;
            let mut airpods_prev_in_case_closed: bool = false;
            let mut airpods_both_removed_at: Option<tokio::time::Instant> = None;
            let mut interrupted_route: Option<InterruptedRoute> = None;

            // Auto-trim initial startup working set after background components settle (2.5s)
            tokio::spawn(async {
                tokio::time::sleep(Duration::from_millis(2500)).await;
                crate::tray::trim_working_set();
            });

            // Start Windows GSMTC media session tracker for WinAMP player view
            crate::media_control::spawn_media_tracker();

            // Spawn background discovery and HomePod matching immediately so event loop is never blocked
            let bg_tx = cmd_tx.clone();
            tokio::spawn(async move {
                let last_tgt = load_last_connected_device();
                let mut devs = discover(Duration::from_millis(3500)).await.unwrap_or_default();
                let mut matched = if let Some(ref target) = last_tgt {
                    find_matching_devices(&devs, target)
                } else {
                    None
                };
                let need_retry = if let Some(ref target) = last_tgt {
                    let found_count = matched.as_ref().map(|m| m.len()).unwrap_or(0);
                    let expected = if target.expected_device_count > 0 {
                        target.expected_device_count
                    } else if target.device_ids.len() > 1 && !target.is_stereo_pair {
                        target.device_ids.len()
                    } else {
                        1
                    };
                    found_count < expected
                } else {
                    false
                };
                if need_retry {
                    if let Ok(devs2) = discover(Duration::from_millis(2000)).await {
                        for d in devs2 {
                            if !devs.iter().any(|existing| existing.id_string() == d.id_string()) {
                                devs.push(d);
                            }
                        }
                        matched = last_tgt.as_ref().and_then(|t| find_matching_devices(&devs, t));
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

            let mut last_sent_track: Option<(String, String, String)> = None;
            let mut saved_homepod_volume: f32 = load_volume();

            let _ = CONTROLLER_CMD_TX.set(cmd_tx.clone());
            ensure_notification_client_registered();

            if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
                if let Ok(def_dev) = enumerator.get_default_device(&wasapi::Direction::Render) {
                    if let (Ok(id), Ok(name)) = (def_dev.get_id(), def_dev.get_friendlyname()) {
                        if !is_airpods_playback_device(&name) {
                            update_saved_standin_device(&id);
                        }
                    }
                }
            }

            // Spawn background WinRT AirPods BLE Watcher
            let airpods_tx = cmd_tx.clone();
            let _airpods_watcher = crate::airpods::AirPodsWatcher::start(move |event| {
                let _ = airpods_tx.send_blocking(GuiCmd::AirPodsEventReceived(event));
            });
            if let Err(ref e) = _airpods_watcher {
                tracing::warn!("[AirPods] Could not start BLE watcher: {e}");
            }

            // Initial enumeration of paired AirPods
            state.paired_airpods = crate::airpods::enumerate_paired_airpods();

            // When starting disconnected, synchronize volume slider with the actual Windows endpoint volume & mute
            let init_dev = if state.ao_output_device != "default" && !state.ao_output_device.is_empty() {
                Some(state.ao_output_device.as_str())
            } else {
                None
            };
            if let Ok(cur_vol) = get_endpoint_volume(init_dev) {
                state.volume = cur_vol;
            }
            if let Ok(is_muted) = get_endpoint_mute(init_dev) {
                state.is_muted = is_muted;
                set_source_mute(is_muted);
            }
            register_local_endpoint_listener(init_dev);
            broadcast(&state);

            let eq_config = EqConfig::ten_band();
            let global_eq_params = Arc::new(EqParams::new(eq_config.num_bands()));
            global_eq_params.set_bypass(!state.eq_enabled);
            for (i, &g) in state.eq_gains.iter().enumerate() {
                global_eq_params.set_gain_db(i, g);
            }
            let global_sound_enhancer_params = Arc::new(SoundEnhancerParams::new(state.sound_enhancer_mode));
            let global_warmth_enhancer_params = Arc::new(WarmthEnhancerParams::new(state.warmth_enhancer_mode));

            let rx_apply_effects = Arc::new(AtomicBool::new(state.airplay_receiver_apply_effects));
            let rx_eq_params = Arc::new(EqParams::new(eq_config.num_bands()));
            rx_eq_params.set_bypass(!state.airplay_receiver_eq_enabled);
            for (i, &g) in state.airplay_receiver_eq_gains.iter().enumerate() {
                rx_eq_params.set_gain_db(i, g);
            }
            let rx_sound_enhancer_params = Arc::new(SoundEnhancerParams::new(state.airplay_receiver_sound_enhancer_mode));
            let rx_warmth_enhancer_params = Arc::new(WarmthEnhancerParams::new(state.airplay_receiver_warmth_enhancer_mode));

            let rx_dsp_params = airplay_receiver::ReceiverDspParams {
                apply_effects: rx_apply_effects.clone(),
                eq_params: rx_eq_params.clone(),
                sound_enhancer_params: rx_sound_enhancer_params.clone(),
                warmth_enhancer_params: rx_warmth_enhancer_params.clone(),
            };

            let rx_output_dev = if state.airplay_receiver_output_device.is_empty() || state.airplay_receiver_output_device == "default" {
                None
            } else {
                Some(state.airplay_receiver_output_device.clone())
            };

            // Initialize AirPlay Receiver switchable sink and server
            let initial_sink: Arc<dyn AudioSink> = match LocalSpeakerSink::new_with_device_and_dsp(rx_output_dev, Some(rx_dsp_params.clone())) {
                Ok(s) => Arc::new(s),
                Err(_) => Arc::new(CallbackSink::new(|_, _, _| {})),
            };
            let switchable_sink = SwitchableSink::new(initial_sink);
            switchable_sink.set_delay_samples(state.airplay_receiver_sync_level.delay_stereo_samples());

            let proxy_sink = Arc::new(ReceiverStatusProxySink {
                target: Arc::new(switchable_sink.clone()),
                cmd_tx: cmd_tx.clone(),
            });

            let mut receiver_instance: Option<AirPlayReceiver> = None;
            let mut last_rx_phone_volume: Option<u8> = None;
            if state.airplay_receiver_enabled {
                let mut config = AirPlayReceiverConfig::new(&state.airplay_receiver_name);
                config.ignore_volume_control = state.airplay_receiver_auto_max_volume;
                let mut rx_server = AirPlayReceiver::new(config, proxy_sink.clone());
                match rx_server.start().await {
                    Ok(_) => {
                        tracing::info!("[AirPlayReceiver] Started at launch: '{}'", state.airplay_receiver_name);
                        receiver_instance = Some(rx_server);
                    }
                    Err(e) => {
                        tracing::error!("[AirPlayReceiver] Failed to start at launch: {}", e);
                    }
                }
            }

            loop {
                tokio::select! {
                    _ = keepalive_interval.tick() => {
                        if let Some(ref mut session) = runtime.session_mut() {
                            let dead_devs = session.check_dead_devices().await;
                            session.poll_hardware_volume().await;

                            let (cur_track, cur_pos, cur_dur) = {
                                let t = crate::media_control::get_current_media_track().read().unwrap().clone();
                                let meta = if !t.title.is_empty() || !t.artist.is_empty() {
                                    (t.title, t.artist, t.album)
                                } else {
                                    ("Win2Pod Stream".to_string(), "Windows PC".to_string(), "Win2Pod".to_string())
                                };
                                (meta, t.position_secs, t.duration_secs)
                            };

                            let track_changed = last_sent_track.as_ref() != Some(&cur_track);
                            if track_changed {
                                last_sent_track = Some(cur_track.clone());
                                session.send_track_info(&cur_track.0, &cur_track.1, &cur_track.2).await;
                                session.send_progress(cur_pos, cur_dur).await;
                            } else if cur_dur > 0.0 {
                                session.send_progress(cur_pos, cur_dur).await;
                            }
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

                                last_sent_track = None;
                                unregister_local_endpoint_listener();
                                runtime.stop().await;

                                // Attempt automatic reconnection first before concluding the device was taken over!
                                tracing::info!("Attempting automatic reconnection to recover lost stream for [{dead_title}]...");
                                state.status = DeviceStatus::Connecting;
                                state.notice_message = Some((NoticeKind::ConnectionLostReconnecting(dead_title.clone()), std::time::Instant::now()));
                                broadcast(&state);

                                let devs = current_connected_devices.clone();
                                let eff_vol = state.volume;
                                let mut recovered = false;
                                let mut last_error_str = String::new();

                                for attempt in 1..=2 {
                                    tokio::time::sleep(Duration::from_millis(1500)).await;
                                    match Session::start(devs.clone(), eff_vol, &state.device_volumes, state.audio_source.clone(), state.low_latency, state.eq_enabled, &state.eq_gains, state.sound_enhancer_mode, state.warmth_enhancer_mode).await {
                                        Ok(sess) => {
                                            tracing::info!("Successfully recovered stream for [{dead_title}] on attempt {attempt}!");
                                            state.status = DeviceStatus::Streaming;
                                            state.diagnostics.timing_mode = sess.timing_desc.clone();
                                            state.notice_message = Some((NoticeKind::ConnectionRestored(dead_title.clone()), std::time::Instant::now()));
                                            runtime = AudioRouteRuntime::Streaming(sess);
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
                                    current_connected_devices.clear();

                                    transition_to_local_route(
                                        &mut runtime,
                                        &mut state,
                                        &mut saved_homepod_volume,
                                        &global_eq_params,
                                        &global_sound_enhancer_params,
                                        &global_warmth_enhancer_params,
                                        true,
                                        &shared_state,
                                    ).await;
                                }
                            }
                        }
                    }
                    _ = tick_interval.tick() => {
                        if let Some(ref session) = runtime.session() {
                            state.diagnostics.sample_rate = RATE;
                            state.diagnostics.channels = CHANNELS;
                            state.diagnostics.frames_captured = session.counters.frames_captured.load(Ordering::Relaxed);
                            state.diagnostics.underruns = session.counters.underruns.load(Ordering::Relaxed);
                            if state.diagnostics.timing_mode != session.timing_desc {
                                state.diagnostics.timing_mode = session.timing_desc.clone();
                            }

                            let last_active = session.counters.last_active_ts.load(Ordering::Relaxed);
                            let now = monotonic_now_ms();
                            // Active if non-silent audio was captured within the last 30 seconds and not muted (Scenario B)
                            let is_active = !state.is_muted && last_active > 0 && now.saturating_sub(last_active) <= 30_000;
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

                        if let Some(removed_at) = airpods_both_removed_at {
                            const AIRPODS_IDLE_DESK_TIMEOUT: Duration = Duration::from_secs(10 * 60);
                            if removed_at.elapsed() >= AIRPODS_IDLE_DESK_TIMEOUT {
                                airpods_both_removed_at = None;
                                if state.airpods_auto_routing && interrupted_route.is_some() {
                                    tracing::info!("[AirPods] Earbuds removed and idle on desk for 10 minutes - safely releasing route back to interrupted snapshot");
                                    restore_interrupted_route(
                                        interrupted_route.take(),
                                        &cmd_tx,
                                        &mut state,
                                    ).await;
                                }
                            }
                        }
                    }
                    vol_opt = async {
                        if let Some(ref mut session) = runtime.session_mut() {
                            session.recv_volume_change().await
                        } else {
                            std::future::pending::<Option<(String, f32)>>().await
                        }
                    } => {
                        if let Some((mac, new_vol)) = vol_opt {
                            let clamped = new_vol.clamp(0.0, 1.0);
                            let matched_id = state.devices.iter()
                                .chain(current_connected_devices.iter())
                                .find(|d| {
                                    d.raw_device.id.to_mac_string().eq_ignore_ascii_case(&mac)
                                        || d.member_devices.iter().any(|m| m.id.to_mac_string().eq_ignore_ascii_case(&mac))
                                })
                                .map(|d| d.id_string())
                                .or_else(|| {
                                    state.connected_device_ids.iter().find(|id| id.eq_ignore_ascii_case(&mac)).cloned()
                                })
                                .or_else(|| state.connected_device_ids.first().cloned())
                                .unwrap_or_else(|| mac.clone());

                            let old_vol = state.device_volumes.get(&matched_id).copied().unwrap_or(state.volume);
                            if (old_vol - clamped).abs() > 0.005 || (state.volume - clamped).abs() > 0.005 {
                                tracing::info!(
                                    "Hardware volume change from HomePod [{}]: {:.3} (was {:.3}, current master was {:.3})",
                                    matched_id, clamped, old_vol, state.volume
                                );
                                state.device_volumes.insert(matched_id.clone(), clamped);

                                let sum: f32 = state.connected_device_ids.iter().filter_map(|id| state.device_volumes.get(id)).sum();
                                let count = state.connected_device_ids.iter().filter(|id| state.device_volumes.contains_key(*id)).count();
                                if count > 0 {
                                    state.volume = sum / (count as f32);
                                } else {
                                    state.volume = clamped;
                                }
                                if state.is_muted && clamped > 0.001 {
                                    state.is_muted = false;
                                    set_source_mute(false);
                                } else if state.volume <= 0.001 {
                                    state.is_muted = true;
                                    set_source_mute(true);
                                }
                                save_volume(state.volume);
                                broadcast(&state);
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
                                state.is_discovering = false;
                                state.audio_devices = get_audio_render_devices();

                                // Auto-connect to previously used HomePod if enabled
                                if state.auto_connect
                                    && !runtime.is_streaming()
                                    && !matches!(runtime, AudioRouteRuntime::AoMode(_))
                                    && !state.airpods_left_in_ear
                                    && !state.airpods_right_in_ear
                                {
                                    let target = matched.or_else(|| {
                                        load_last_connected_device()
                                            .and_then(|ref last_target| find_matching_devices(&state.devices, last_target))
                                    });
                                    if let Some(connect_devs) = target {
                                        tracing::info!("Startup: auto-connecting to HomePod ({} device(s))...", connect_devs.len());
                                        let _ = cmd_tx.send(GuiCmd::Connect(connect_devs)).await;
                                    }
                                }

                                if state.ao_output_device != "default" && !state.audio_devices.iter().any(|d| d.id == state.ao_output_device) {
                                    tracing::info!(
                                        "[Endpoint Disconnected] Target audio device [{}] is no longer present; resetting to default audio endpoint",
                                        state.ao_output_device
                                    );
                                    state.ao_output_device = "default".to_string();
                                    save_ao_output_device("default");
                                    transition_to_local_route(
                                        &mut runtime,
                                        &mut state,
                                        &mut saved_homepod_volume,
                                        &global_eq_params,
                                        &global_sound_enhancer_params,
                                        &global_warmth_enhancer_params,
                                        true,
                                        &shared_state,
                                    ).await;
                                    tokio::time::sleep(Duration::from_millis(300)).await;
                                    crate::handoff::on_route_ready();
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
                                        if runtime.is_streaming() {
                                            restore_original_speaker_mute();
                                        }
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::RefreshDevices => {
                                if !is_refreshing_devices {
                                    is_refreshing_devices = true;
                                    state.is_discovering = true;
                                    broadcast(&state);
                                    tracing::info!("[CONTROLLER] Initiating background device refresh scan...");
                                    let refresh_tx = cmd_tx.clone();
                                    tokio::spawn(async move {
                                        let devs = discover(Duration::from_secs(3)).await.unwrap_or_default();
                                        let _ = refresh_tx.send(GuiCmd::DevicesRefreshed(devs)).await;
                                    });
                                }
                            }
                            GuiCmd::DevicesRefreshed(fresh_devs) => {
                                is_refreshing_devices = false;
                                state.is_discovering = false;
                                tracing::info!("[CONTROLLER] Background device refresh completed: found {} device(s)", fresh_devs.len());
                                let mut merged = fresh_devs;
                                // Crucial: Keep currently connected active streaming devices in the list
                                // even if mDNS dropped packets during this quick scan!
                                for active in &current_connected_devices {
                                    let active_id = active.id_string();
                                    if !merged.iter().any(|d| d.id_string().eq_ignore_ascii_case(&active_id)) {
                                        merged.push(active.clone());
                                    }
                                }
                                state.devices = merged;
                                // If the controller was previously in an error state (and not streaming), reset to idle
                                if matches!(state.status, DeviceStatus::Error(_)) {
                                    state.status = DeviceStatus::Idle;
                                }
                                broadcast(&state);
                            }
                            GuiCmd::RefreshAudioDevices => {
                                state.audio_devices = get_audio_render_devices();
                                state.paired_airpods = crate::airpods::enumerate_paired_airpods();

                                if state.airpods_auto_routing && (state.airpods_left_in_ear || state.airpods_right_in_ear) {
                                    let is_already_on_airpods = !runtime.is_streaming()
                                        && find_best_airpods_device(&state.audio_devices)
                                            .map_or(false, |d| d.id == state.ao_output_device);

                                    if !is_already_on_airpods {
                                        try_route_to_airpods(
                                            &mut runtime,
                                            &mut state,
                                            &mut saved_homepod_volume,
                                            &global_eq_params,
                                            &global_sound_enhancer_params,
                                            &global_warmth_enhancer_params,
                                            &shared_state,
                                            &mut interrupted_route,
                                            &mut airpods_paused_at,
                                            &cmd_tx,
                                            false,
                                        ).await;
                                    }
                                }

                                if state.ao_output_device != "default" && !state.audio_devices.iter().any(|d| d.id == state.ao_output_device) {
                                    tracing::info!(
                                        "[Endpoint Disconnected] Target audio device [{}] is no longer present; resetting to default audio endpoint",
                                        state.ao_output_device
                                    );
                                    state.ao_output_device = "default".to_string();
                                    save_ao_output_device("default");
                                    transition_to_local_route(
                                        &mut runtime,
                                        &mut state,
                                        &mut saved_homepod_volume,
                                        &global_eq_params,
                                        &global_sound_enhancer_params,
                                        &global_warmth_enhancer_params,
                                        true,
                                        &shared_state,
                                    ).await;
                                    tokio::time::sleep(Duration::from_millis(300)).await;
                                    crate::handoff::on_route_ready();
                                }

                                broadcast(&state);
                            }
                            GuiCmd::RefreshAudioApps => {
                                state.audio_apps = crate::app_enumerator::get_audio_applications();
                                broadcast(&state);
                            }
                            GuiCmd::SetAudioSource(source) => {
                                state.audio_source = source.clone();
                                match &source {
                                    AudioSource::Device(dev_id) => {
                                        state.selected_audio_device = dev_id.clone();
                                        state.last_selected_device = dev_id.clone();
                                        save_last_selected_device(dev_id);
                                    }
                                    AudioSource::Application { exe_name, display_name, .. } => {
                                        state.selected_audio_device = format!("app:{exe_name}");
                                        state.last_selected_app = Some((exe_name.clone(), display_name.clone()));
                                        save_last_selected_app(exe_name, display_name);
                                    }
                                }
                                save_audio_source(&source);

                                if let Some(ref mut session) = runtime.session_mut() {
                                    if state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed() {
                                        record_and_configure_streaming_source(&state.audio_source);
                                    } else {
                                        restore_original_speaker_state();
                                    }
                                    session.switch_audio_source(state.audio_source.clone());
                                    broadcast(&state);
                                } else {
                                    transition_to_local_route(
                                        &mut runtime,
                                        &mut state,
                                        &mut saved_homepod_volume,
                                        &global_eq_params,
                                        &global_sound_enhancer_params,
                                        &global_warmth_enhancer_params,
                                        true,
                                        &shared_state,
                                    ).await;
                                }
                            }
                            GuiCmd::Connect(mut dev_items) => {
                                if dev_items.is_empty() {
                                    continue;
                                }

                                tracing::info!(
                                    "GuiCmd::Connect received for {} device(s): [{}]",
                                    dev_items.len(),
                                    dev_items.iter().map(|d| d.name.as_str()).collect::<Vec<_>>().join(", ")
                                );

                                // Always ensure stereo pairs / group leaders are prioritized first
                                dev_items.sort_by_key(|d| {
                                    (
                                        u8::from(!d.is_stereo_pair),
                                        std::cmp::Reverse(d.member_devices.len()),
                                    )
                                });

                                // If already streaming to these exact devices, do not stop and reconnect!
                                if runtime.is_streaming() && current_connected_devices.len() == dev_items.len() {
                                    let is_same = dev_items.iter().all(|d| current_connected_devices.iter().any(|c| c.id_string() == d.id_string()));
                                    if is_same {
                                        tracing::info!("Already streaming to requested device(s), ignoring redundant connect");
                                        continue;
                                    }
                                }

                                last_sent_track = None;
                                unregister_local_endpoint_listener();
                                tracing::info!("Stopping active audio runtime before connecting...");
                                runtime.stop().await;
                                tracing::info!("Active audio runtime stopped cleanly, preparing connection parameters...");
                                tokio::time::sleep(Duration::from_millis(150)).await;

                                // Crucial: Switch volume and mute to saved HomePod state BEFORE setting device volumes
                                state.volume = saved_homepod_volume;
                                state.is_muted = saved_homepod_volume <= 0.001;
                                set_source_mute(state.is_muted);
                                state.device_volumes.clear();
                                for dev in &dev_items {
                                    state.device_volumes.insert(dev.id_string(), saved_homepod_volume);
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
                                if interrupted_route.is_some() {
                                    tracing::info!("[AirPods] User connected to HomePod - clearing interrupted route snapshot");
                                    interrupted_route = None;
                                }
                                broadcast(&state);

                                let eff_vol = state.volume;
                                match Session::start(dev_items, eff_vol, &state.device_volumes, state.audio_source.clone(), state.low_latency, state.eq_enabled, &state.eq_gains, state.sound_enhancer_mode, state.warmth_enhancer_mode).await {
                                    Ok(sess) => {
                                        state.status = DeviceStatus::Streaming;
                                        state.diagnostics.timing_mode = sess.timing_desc.clone();
                                        runtime = AudioRouteRuntime::Streaming(sess);

                                        // Configure target device speakers for streaming if auto-mute is active
                                        if state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed() {
                                            record_and_configure_streaming_source(&state.audio_source);
                                        } else {
                                            restore_original_speaker_state();
                                        }

                                        // Update power keepalive (initially false until audio activity is detected)
                                        state.is_audio_active = false;
                                        update_thread_execution_state(state.prevent_sleep, false);

                                        save_last_connected_device(&current_connected_devices, &display_title);
                                        save_last_route_type(LastRouteType::HomePod);
                                        state.last_connected_name = Some(display_title);

                                        broadcast(&state);
                                        crate::handoff::on_route_ready();
                                    }
                                    Err(err) => {
                                        let msg = format_clean_error(&err);
                                        tracing::error!("Connection error: {msg}");
                                        state.notice_message = None;
                                        current_connected_devices.clear();
                                        
                                        transition_to_local_route(
                                            &mut runtime,
                                            &mut state,
                                            &mut saved_homepod_volume,
                                            &global_eq_params,
                                            &global_sound_enhancer_params,
                                            &global_warmth_enhancer_params,
                                            true,
                                            &shared_state,
                                        ).await;

                                        state.status = DeviceStatus::Error(msg);
                                        broadcast(&state);
                                    }
                                }
                            }
                            GuiCmd::Disconnect => {
                                                                state.notice_message = None;
                                current_connected_devices.clear();
                                if interrupted_route.is_some() {
                                    tracing::info!("[AirPods] User manually disconnected - clearing interrupted route snapshot");
                                    interrupted_route = None;
                                }
                                last_sent_track = None;
                                save_last_route_type(LastRouteType::Local);
                                let dev_name = if state.ao_output_device == "default" || state.ao_output_device.is_empty() {
                                    "__default__".to_string()
                                } else {
                                    state.audio_devices.iter()
                                        .find(|d| d.id == state.ao_output_device)
                                        .map(|d| d.name.clone())
                                        .unwrap_or_else(|| state.ao_output_device.clone())
                                };
                                state.last_connected_name = Some(dev_name);
                                state.connected_name = None;
                                state.connected_device_ids.clear();
                                state.device_volumes.clear();
                                if state.auto_ao_mode || (!state.ao_output_device.is_empty() && state.ao_output_device != "default") {
                                    state.status = DeviceStatus::AoMode("default".to_string());
                                } else {
                                    state.status = DeviceStatus::Idle;
                                }
                                broadcast_state(&shared_state, &state);
                                transition_to_local_route(
                                    &mut runtime,
                                    &mut state,
                                    &mut saved_homepod_volume,
                                    &global_eq_params,
                                    &global_sound_enhancer_params,
                                    &global_warmth_enhancer_params,
                                    false,
                                    &shared_state,
                                ).await;
                            }
                            GuiCmd::SetAutoAoMode(enabled) => {
                                if interrupted_route.is_some() {
                                    tracing::info!("[AirPods] User manually toggled Auto AO mode - clearing interrupted route snapshot");
                                    interrupted_route = None;
                                }
                                state.auto_ao_mode = enabled;
                                save_auto_ao_mode(enabled);
                                if !runtime.is_streaming() {
                                    transition_to_local_route(
                                        &mut runtime,
                                        &mut state,
                                        &mut saved_homepod_volume,
                                        &global_eq_params,
                                        &global_sound_enhancer_params,
                                        &global_warmth_enhancer_params,
                                        true,
                                        &shared_state,
                                    ).await;
                                } else {
                                    broadcast(&state);
                                }
                            }
                            GuiCmd::SetAoOutputDevice(dev_id) => {
                                if interrupted_route.is_some() {
                                    tracing::info!("[AirPods] User manually changed AO output device - clearing interrupted route snapshot");
                                    interrupted_route = None;
                                }
                                state.ao_output_device = dev_id.clone();
                                save_ao_output_device(&dev_id);
                                if !runtime.is_streaming() {
                                    if !is_airpods_playback_device(&dev_id) {
                                        save_last_route_type(LastRouteType::Local);
                                        let dev_name = if dev_id == "default" || dev_id.is_empty() {
                                            "__default__".to_string()
                                        } else {
                                            state.audio_devices.iter()
                                                .find(|d| d.id == dev_id)
                                                .map(|d| d.name.clone())
                                                .unwrap_or_else(|| dev_id.clone())
                                        };
                                        state.last_connected_name = Some(dev_name);
                                    }
                                    transition_to_local_route(
                                        &mut runtime,
                                        &mut state,
                                        &mut saved_homepod_volume,
                                        &global_eq_params,
                                        &global_sound_enhancer_params,
                                        &global_warmth_enhancer_params,
                                        true,
                                        &shared_state,
                                    ).await;
                                } else {
                                    broadcast(&state);
                                }
                            }
                            GuiCmd::SetVolume(vol) => {
                                let clamped = vol.clamp(0.0, 1.0);
                                let delta = clamped - state.volume;
                                state.volume = clamped;

                                if let Some(ref mut sess) = runtime.session_mut() {
                                    saved_homepod_volume = clamped;
                                    save_volume(clamped);

                                    if state.is_muted && clamped > 0.001 {
                                        state.is_muted = false;
                                        set_source_mute(false);
                                    }

                                    for id in &state.connected_device_ids {
                                    let cur = state.device_volumes.get(id).copied().unwrap_or(clamped);
                                    let new_dev_vol = (cur + delta).clamp(0.0, 1.0);
                                        state.device_volumes.insert(id.clone(), new_dev_vol);
                                    }

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
                                } else {
                                    // When not connected to HomePod, directly set the Windows audio endpoint volume!
                                    let target_dev = if state.ao_output_device != "default" && !state.ao_output_device.is_empty() {
                                        Some(state.ao_output_device.as_str())
                                    } else {
                                        None
                                    };
                                    let _ = set_endpoint_volume(target_dev, clamped);
                                    if state.is_muted && clamped > 0.001 {
                                        state.is_muted = false;
                                        set_source_mute(false);
                                        let _ = set_endpoint_mute(target_dev, false);
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetDeviceVolume(dev_id, vol) => {
                                let clamped = vol.clamp(0.0, 1.0);
                                state.device_volumes.insert(dev_id.clone(), clamped);

                                if state.is_muted && clamped > 0.001 {
                                    state.is_muted = false;
                                    set_source_mute(false);
                                }

                                if !state.connected_device_ids.is_empty() {
                                    let sum: f32 = state.connected_device_ids.iter().filter_map(|id| state.device_volumes.get(id)).sum();
                                    let count = state.connected_device_ids.iter().filter(|id| state.device_volumes.contains_key(*id)).count();
                                    if count > 0 {
                                        state.volume = sum / (count as f32);
                                        save_volume(state.volume);
                                    }
                                }

                                if let Some(ref mut sess) = runtime.session_mut() {
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
                                set_source_mute(muted);
                                if let Some(ref mut sess) = runtime.session_mut() {
                                    let last_active = sess.counters.last_active_ts.load(Ordering::Relaxed);
                                    let now = monotonic_now_ms();
                                    let is_active = !muted && last_active > 0 && now.saturating_sub(last_active) <= 30_000;
                                    if state.is_audio_active != is_active {
                                        state.is_audio_active = is_active;
                                        update_thread_execution_state(state.prevent_sleep, state.is_audio_active);
                                        tracing::info!(
                                            "Mute changed to {}: audio active={}, sleep_suppressed={}",
                                            muted,
                                            state.is_audio_active,
                                            state.prevent_sleep && state.is_audio_active
                                        );
                                    }
                                } else {
                                    let target_dev = if state.ao_output_device != "default" && !state.ao_output_device.is_empty() {
                                        Some(state.ao_output_device.as_str())
                                    } else {
                                        None
                                    };
                                    let _ = set_endpoint_mute(target_dev, muted);
                                }
                                broadcast(&state);
                            }
                            GuiCmd::LocalEndpointVolumeChanged { volume, is_muted } => {
                                if !runtime.is_streaming() {
                                    let vol_diff = (state.volume - volume).abs();
                                    let mute_diff = state.is_muted != is_muted;
                                    if vol_diff > 0.005 || mute_diff {
                                        state.volume = volume;
                                        state.is_muted = is_muted;
                                        set_source_mute(is_muted);
                                        broadcast(&state);
                                    }
                                }
                            }
                            GuiCmd::SetAutoMuteSpeakers(enabled) => {
                                let allowed = state.license_tier.is_auto_mute_allowed();
                                let actual = if allowed { enabled } else { false };
                                state.auto_mute_speakers = actual;
                                save_auto_mute(actual);
                                if actual {
                                    if runtime.is_streaming() {
                                        record_and_configure_streaming_source(&state.audio_source);
                                    } else {
                                        let has_target_device = !state.ao_output_device.is_empty() && state.ao_output_device != "default";
                                        if has_target_device {
                                            set_audio_shield_state(AudioShieldState::LocalTarget {
                                                target_endpoint_id: state.ao_output_device.clone(),
                                            });
                                        } else {
                                            set_audio_shield_state(AudioShieldState::Idle);
                                        }
                                    }
                                } else {
                                    set_audio_shield_state(AudioShieldState::Idle);
                                    restore_original_speaker_state();
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetPreventSleep(enabled) => {
                                state.prevent_sleep = enabled;
                                save_prevent_sleep(enabled);
                                let is_active = runtime.is_streaming() && state.is_audio_active;
                                update_thread_execution_state(state.prevent_sleep, is_active);
                                broadcast(&state);
                            }
                            GuiCmd::SetLowLatency(enabled) => {
                                state.low_latency = enabled;
                                save_low_latency(enabled);
                                if !current_connected_devices.is_empty() {
                                    runtime.stop().await;
                                    let devs = current_connected_devices.clone();
                                    let eff_vol = state.volume;
                                    match Session::start(devs, eff_vol, &state.device_volumes, state.audio_source.clone(), state.low_latency, state.eq_enabled, &state.eq_gains, state.sound_enhancer_mode, state.warmth_enhancer_mode).await {
                                        Ok(sess) => {
                                            state.status = DeviceStatus::Streaming;
                                            state.diagnostics.timing_mode = sess.timing_desc.clone();
                                            runtime = AudioRouteRuntime::Streaming(sess);
                                            update_thread_execution_state(state.prevent_sleep, true);
                                            if state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed() {
                                                record_and_configure_streaming_source(&state.audio_source);
                                            }
                                        }
                                        Err(e) => {
                                            let msg = format_clean_error(&e);
                                            tracing::error!("LowLatency restart error: {msg}");
                                            transition_to_local_route(
                                                &mut runtime,
                                                &mut state,
                                                &mut saved_homepod_volume,
                                                &global_eq_params,
                                                &global_sound_enhancer_params,
                                                &global_warmth_enhancer_params,
                                                true,
                                                &shared_state,
                                            ).await;
                                            state.status = DeviceStatus::Error(msg);
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
                                crate::visualizer::set_language(lang.code());
                                broadcast(&state);
                            }
                            GuiCmd::SetEqGain(band, gain_db) => {
                                if band < state.eq_gains.len() {
                                    let clamped = gain_db.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
                                    state.eq_gains[band] = clamped;
                                    state.custom_eq_gains[band] = clamped;
                                    save_eq_gains(&state.eq_gains);
                                    save_custom_eq_gains(&state.custom_eq_gains);
                                }
                                if !state.eq_enabled {
                                    state.eq_enabled = true;
                                    save_eq_enabled(true);
                                }
                                global_eq_params.set_bypass(false);
                                global_eq_params.set_gain_db(band, gain_db);
                                if let Some(ref sess) = runtime.session() {
                                    sess.eq_params.set_bypass(false);
                                    sess.eq_params.set_gain_db(band, gain_db);
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetEqBypass(bypass) => {
                                state.eq_enabled = !bypass;
                                save_eq_enabled(state.eq_enabled);
                                global_eq_params.set_bypass(bypass);
                                if let Some(ref sess) = runtime.session() {
                                    sess.eq_params.set_bypass(bypass);
                                }
                                broadcast(&state);
                            }
                            GuiCmd::ResetEq => {
                                state.eq_gains = [0.0; 10];
                                save_eq_gains(&state.eq_gains);
                                global_eq_params.reset();
                                if let Some(ref sess) = runtime.session() {
                                    sess.eq_params.reset();
                                }
                                broadcast(&state);
                            }
                            GuiCmd::ApplyEqPreset(preset) => {
                                let gains = if preset == EqPreset::Custom {
                                    state.custom_eq_gains
                                } else {
                                    preset.gains()
                                };
                                state.eq_gains = gains;
                                save_eq_gains(&state.eq_gains);
                                if !state.eq_enabled {
                                    state.eq_enabled = true;
                                    save_eq_enabled(true);
                                }
                                global_eq_params.set_bypass(false);
                                for (i, &g) in state.eq_gains.iter().enumerate() {
                                    global_eq_params.set_gain_db(i, g);
                                }
                                if let Some(ref sess) = runtime.session() {
                                    sess.eq_params.set_bypass(false);
                                    for (i, &g) in state.eq_gains.iter().enumerate() {
                                        sess.eq_params.set_gain_db(i, g);
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetSoundEnhancerMode(mode) => {
                                state.sound_enhancer_mode = mode;
                                save_sound_enhancer_mode(mode);
                                global_sound_enhancer_params.set_mode(mode);
                                if let Some(ref sess) = runtime.session() {
                                    sess.sound_enhancer_params.set_mode(mode);
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetWarmthEnhancerMode(mode) => {
                                state.warmth_enhancer_mode = mode;
                                save_warmth_enhancer_mode(mode);
                                global_warmth_enhancer_params.set_mode(mode);
                                if let Some(ref sess) = runtime.session() {
                                    sess.warmth_enhancer_params.set_mode(mode);
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetAirPodsAutoRouting(enabled) => {
                                state.airpods_auto_routing = enabled;
                                save_airpods_auto_routing(enabled);
                                if enabled && (state.airpods_left_in_ear || state.airpods_right_in_ear) {
                                    trigger_refresh_audio_devices();
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetAirPodsAutoPause(enabled) => {
                                state.airpods_auto_pause = enabled;
                                save_airpods_auto_pause(enabled);
                                broadcast(&state);
                            }
                            GuiCmd::SetWindowSnapping(enabled) => {
                                state.window_snapping = enabled;
                                save_window_snapping(enabled);
                                crate::snapping::SNAPPING_ENABLED.store(enabled, std::sync::atomic::Ordering::SeqCst);
                                if !enabled {
                                    crate::snapping::break_dock_bond();
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetSnapDistance(dist) => {
                                let clamped = dist.clamp(0, 50);
                                state.snap_distance = clamped;
                                save_snap_distance(clamped);
                                crate::snapping::SNAP_DISTANCE.store(clamped, std::sync::atomic::Ordering::SeqCst);
                                if clamped == 0 {
                                    crate::snapping::break_dock_bond();
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetDetachEqualizerWindow(enabled) => {
                                state.detach_equalizer_window = enabled;
                                save_detach_equalizer_window(enabled);
                                broadcast(&state);
                            }
                            GuiCmd::SetAirPlayReceiverEnabled(enabled) => {
                                state.airplay_receiver_enabled = enabled;
                                save_airplay_receiver_enabled(enabled);
                                if enabled {
                                    if receiver_instance.is_none() {
                                        let mut config = AirPlayReceiverConfig::new(&state.airplay_receiver_name);
                                        config.ignore_volume_control = state.airplay_receiver_auto_max_volume;
                                        let mut rx_server = AirPlayReceiver::new(config, proxy_sink.clone());
                                        match rx_server.start().await {
                                            Ok(_) => {
                                                tracing::info!("[AirPlayReceiver] Started: '{}'", state.airplay_receiver_name);
                                                receiver_instance = Some(rx_server);
                                            }
                                            Err(e) => {
                                                tracing::error!("[AirPlayReceiver] Failed to start: {}", e);
                                            }
                                        }
                                    }
                                } else {
                                    if let Some(mut rx_server) = receiver_instance.take() {
                                        rx_server.stop().await;
                                        tracing::info!("[AirPlayReceiver] Stopped and disabled");
                                    }
                                    state.airplay_receiver_streaming = false;
                                    state.airplay_receiver_client_name = None;
                                    state.airplay_receiver_state = AirPlayReceiverState::Standby;
                                    state.airplay_receiver_device_type = None;
                                    state.airplay_receiver_client_ip = None;
                                    state.airplay_receiver_track_title = None;
                                    state.airplay_receiver_track_artist = None;
                                    state.airplay_receiver_track_album = None;
                                    state.airplay_receiver_artwork_bytes = None;
                                    state.airplay_receiver_volume = None;
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetAirPlayReceiverName(name) => {
                                state.airplay_receiver_name = name.clone();
                                save_airplay_receiver_name(&name);
                                if state.airplay_receiver_enabled {
                                    if let Some(mut rx_server) = receiver_instance.take() {
                                        rx_server.stop().await;
                                    }
                                    let mut config = AirPlayReceiverConfig::new(&state.airplay_receiver_name);
                                    config.ignore_volume_control = state.airplay_receiver_auto_max_volume;
                                    let mut rx_server = AirPlayReceiver::new(config, proxy_sink.clone());
                                    match rx_server.start().await {
                                        Ok(_) => {
                                            tracing::info!("[AirPlayReceiver] Restarted with new name: '{}'", state.airplay_receiver_name);
                                            receiver_instance = Some(rx_server);
                                        }
                                        Err(e) => {
                                            tracing::error!("[AirPlayReceiver] Failed to restart: {}", e);
                                        }
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetAirPlayReceiverSyncLevel(level) => {
                                state.airplay_receiver_sync_level = level;
                                save_airplay_receiver_sync_level(level);
                                switchable_sink.set_delay_samples(level.delay_stereo_samples());
                                broadcast(&state);
                            }
                            GuiCmd::AirPlayReceiverStreamingStatus { streaming, client_name } => {
                                tracing::info!(
                                    "[AirPlayReceiver] StreamingStatus: streaming={}, client={:?}, prev_state={:?}",
                                    streaming, client_name, state.airplay_receiver_state
                                );
                                state.airplay_receiver_streaming = streaming;
                                if streaming {
                                    state.airplay_receiver_state = AirPlayReceiverState::Streaming;
                                    state.airplay_receiver_has_cached_session = true;
                                    state.airplay_receiver_status_hint = None;
                                    if client_name.is_some() {
                                        state.airplay_receiver_client_name = client_name;
                                    }
                                    if state.airplay_receiver_volume.is_none() {
                                        state.airplay_receiver_volume = last_rx_phone_volume;
                                    }
                                } else {
                                    // When streaming ends, only transition to Connected if we were actively Streaming
                                    // and a client session is still present. Never overwrite Standby back to Connected!
                                    if state.airplay_receiver_state == AirPlayReceiverState::Streaming {
                                        if state.airplay_receiver_client_name.is_some() {
                                            state.airplay_receiver_state = AirPlayReceiverState::Connected;
                                        } else {
                                            state.airplay_receiver_state = AirPlayReceiverState::Standby;
                                        }
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::AirPlayReceiverClientConnected { name, device_type, ip } => {
                                tracing::info!(
                                    "[AirPlayReceiver] ClientConnected: name='{}', type='{}', ip='{}', prev_state={:?}",
                                    name, device_type, ip, state.airplay_receiver_state
                                );
                                state.airplay_receiver_has_cached_session = true;
                                state.airplay_receiver_status_hint = None;
                                state.airplay_receiver_client_name = Some(name);
                                state.airplay_receiver_device_type = Some(device_type);
                                state.airplay_receiver_client_ip = Some(ip);
                                if state.airplay_receiver_volume.is_none() {
                                    state.airplay_receiver_volume = last_rx_phone_volume;
                                }
                                if state.airplay_receiver_state == AirPlayReceiverState::Standby {
                                    state.airplay_receiver_state = AirPlayReceiverState::Connected;
                                }
                                broadcast(&state);
                            }
                            GuiCmd::AirPlayReceiverPaused => {
                                tracing::info!("[AirPlayReceiver] Paused (setting state to Connected)");
                                state.airplay_receiver_streaming = false;
                                state.airplay_receiver_state = AirPlayReceiverState::Connected;
                                broadcast(&state);
                            }
                            GuiCmd::AirPlayReceiverClientDisconnected => {
                                tracing::info!("[AirPlayReceiver] ClientDisconnected -> Standby");
                                state.airplay_receiver_streaming = false;
                                state.airplay_receiver_state = AirPlayReceiverState::Standby;
                                state.airplay_receiver_client_name = None;
                                state.airplay_receiver_device_type = None;
                                state.airplay_receiver_client_ip = None;
                                state.airplay_receiver_track_title = None;
                                state.airplay_receiver_track_artist = None;
                                state.airplay_receiver_track_album = None;
                                state.airplay_receiver_artwork_bytes = None;
                                state.airplay_receiver_volume = None;
                                broadcast(&state);
                            }
                            GuiCmd::AirPlayReceiverClientName(name) => {
                                state.airplay_receiver_client_name = Some(name);
                                broadcast(&state);
                            }
                            GuiCmd::AirPlayReceiverMetadata { title, artist, album } => {
                                state.airplay_receiver_track_title = title;
                                state.airplay_receiver_track_artist = artist;
                                state.airplay_receiver_track_album = album;
                                broadcast(&state);
                            }
                            GuiCmd::AirPlayReceiverArtwork(bytes) => {
                                state.airplay_receiver_artwork_bytes = bytes;
                                broadcast(&state);
                            }
                            GuiCmd::AirPlayReceiverDacpPlayPause => {
                                if let Some(ref rx) = receiver_instance {
                                    let sent = rx.send_dacp(airplay_receiver::DacpCommand::PlayPause).await;
                                    if !sent {
                                        state.airplay_receiver_status_hint = Some((
                                            state.language.airplay_receiver_require_connect().to_string(),
                                            std::time::Instant::now(),
                                        ));
                                        broadcast(&state);
                                    } else if state.airplay_receiver_state == AirPlayReceiverState::Standby {
                                        let cmd_tx_clone = cmd_tx.clone();
                                        tokio::spawn(async move {
                                            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                                            let _ = cmd_tx_clone.send(GuiCmd::AirPlayReceiverCheckWakeResult).await;
                                        });
                                    }
                                } else {
                                    state.airplay_receiver_status_hint = Some((
                                        state.language.airplay_receiver_require_connect().to_string(),
                                        std::time::Instant::now(),
                                    ));
                                    broadcast(&state);
                                }
                            }
                            GuiCmd::AirPlayReceiverCheckWakeResult => {
                                if state.airplay_receiver_state == AirPlayReceiverState::Standby {
                                    state.airplay_receiver_status_hint = Some((
                                        state.language.airplay_receiver_wake_failed().to_string(),
                                        std::time::Instant::now(),
                                    ));
                                    broadcast(&state);
                                }
                            }
                            GuiCmd::AirPlayReceiverDacpNext => {
                                if let Some(ref rx) = receiver_instance {
                                    rx.send_dacp(airplay_receiver::DacpCommand::Next).await;
                                }
                            }
                            GuiCmd::AirPlayReceiverDacpPrev => {
                                if let Some(ref rx) = receiver_instance {
                                    rx.send_dacp(airplay_receiver::DacpCommand::Prev).await;
                                }
                            }
                            GuiCmd::SetAirPlayReceiverAutoMaxVolume(enabled) => {
                                state.airplay_receiver_auto_max_volume = enabled;
                                save_airplay_receiver_auto_max_volume(enabled);
                                if let Some(ref rx) = receiver_instance {
                                    rx.set_ignore_volume_control(enabled).await;
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetAirPlayReceiverOutputDevice(dev_id) => {
                                state.airplay_receiver_output_device = dev_id.clone();
                                save_airplay_receiver_output_device(&dev_id);
                                let opt_dev = if dev_id.is_empty() || dev_id == "default" {
                                    None
                                } else {
                                    Some(dev_id)
                                };
                                match LocalSpeakerSink::new_with_device_and_dsp(opt_dev, Some(rx_dsp_params.clone())) {
                                    Ok(s) => {
                                        switchable_sink.set_target(Arc::new(s));
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to switch LocalSpeakerSink device: {e}");
                                    }
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetAirPlayReceiverApplyEffects(enabled) => {
                                state.airplay_receiver_apply_effects = enabled;
                                save_airplay_receiver_apply_effects(enabled);
                                rx_apply_effects.store(enabled, Ordering::Relaxed);
                                broadcast(&state);
                            }
                            GuiCmd::SetReceiverEqGain(band, gain_db) => {
                                if band < state.airplay_receiver_eq_gains.len() {
                                    let clamped = gain_db.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
                                    state.airplay_receiver_eq_gains[band] = clamped;
                                    state.airplay_receiver_custom_eq_gains[band] = clamped;
                                    save_airplay_receiver_eq_gains(&state.airplay_receiver_eq_gains);
                                    save_airplay_receiver_custom_eq_gains(&state.airplay_receiver_custom_eq_gains);
                                }
                                if !state.airplay_receiver_eq_enabled {
                                    state.airplay_receiver_eq_enabled = true;
                                    save_airplay_receiver_eq_enabled(true);
                                }
                                rx_eq_params.set_bypass(false);
                                rx_eq_params.set_gain_db(band, gain_db);
                                broadcast(&state);
                            }
                            GuiCmd::SetReceiverEqBypass(bypass) => {
                                state.airplay_receiver_eq_enabled = !bypass;
                                save_airplay_receiver_eq_enabled(state.airplay_receiver_eq_enabled);
                                rx_eq_params.set_bypass(bypass);
                                broadcast(&state);
                            }
                            GuiCmd::ResetReceiverEq => {
                                state.airplay_receiver_eq_gains = [0.0; 10];
                                save_airplay_receiver_eq_gains(&state.airplay_receiver_eq_gains);
                                rx_eq_params.reset();
                                broadcast(&state);
                            }
                            GuiCmd::ApplyReceiverEqPreset(preset) => {
                                let gains = if preset == EqPreset::Custom {
                                    state.airplay_receiver_custom_eq_gains
                                } else {
                                    preset.gains()
                                };
                                state.airplay_receiver_eq_gains = gains;
                                save_airplay_receiver_eq_gains(&state.airplay_receiver_eq_gains);
                                if !state.airplay_receiver_eq_enabled {
                                    state.airplay_receiver_eq_enabled = true;
                                    save_airplay_receiver_eq_enabled(true);
                                }
                                rx_eq_params.set_bypass(false);
                                for (i, &g) in state.airplay_receiver_eq_gains.iter().enumerate() {
                                    rx_eq_params.set_gain_db(i, g);
                                }
                                broadcast(&state);
                            }
                            GuiCmd::SetReceiverSoundEnhancerMode(mode) => {
                                state.airplay_receiver_sound_enhancer_mode = mode;
                                save_airplay_receiver_sound_enhancer_mode(mode);
                                rx_sound_enhancer_params.set_mode(mode);
                                broadcast(&state);
                            }
                            GuiCmd::SetReceiverWarmthEnhancerMode(mode) => {
                                state.airplay_receiver_warmth_enhancer_mode = mode;
                                save_airplay_receiver_warmth_enhancer_mode(mode);
                                rx_warmth_enhancer_params.set_mode(mode);
                                broadcast(&state);
                            }
                            GuiCmd::AirPlayReceiverVolume(volume_db) => {
                                let pct = if volume_db <= -140.0 {
                                    0
                                } else {
                                    let linear = ((volume_db + 30.0) / 30.0).clamp(0.0, 1.0) * 100.0;
                                    linear.round() as u8
                                };
                                last_rx_phone_volume = Some(pct);
                                state.airplay_receiver_volume = Some(pct);
                                broadcast(&state);
                            }
                            GuiCmd::AirPodsEventReceived(event) => {
                                match event {
                                    crate::airpods::AirPodsEvent::StateChanged {
                                        model,
                                        rssi,
                                        is_any_in_ear,
                                        is_both_in_ear: _,
                                        left_in_ear,
                                        right_in_ear,
                                        left_battery,
                                        right_battery,
                                        case_battery,
                                        left_charging,
                                        right_charging,
                                        case_charging,
                                        is_lid_opened,
                                        is_both_pods_in_case,
                                    } => {
                                        // 1. Identify which paired AirPods is currently in active audio use:
                                        let in_use_paired_name = state.paired_airpods.iter().find(|ap| {
                                            let is_conn = ap.is_connected || state.audio_devices.iter().any(|ad| {
                                                is_airpods_playback_device(&ad.name)
                                                    && (ad.name.to_lowercase().contains(&ap.name.to_lowercase())
                                                        || ap.name.to_lowercase().contains(&ad.name.to_lowercase()))
                                            });
                                            if !is_conn {
                                                return false;
                                            }
                                            match &state.status {
                                                DeviceStatus::AoMode(ref dev_name) => {
                                                    let dn = dev_name.to_lowercase();
                                                    let an = ap.name.to_lowercase();
                                                    dn.contains(&an) || an.contains(&dn)
                                                }
                                                _ => {
                                                    state.ao_output_device != "default" && state.audio_devices.iter().any(|ad| {
                                                        ad.id == state.ao_output_device
                                                            && (ad.name.to_lowercase().contains(&ap.name.to_lowercase())
                                                                || ap.name.to_lowercase().contains(&ad.name.to_lowercase()))
                                                    })
                                                }
                                            }
                                        }).map(|ap| ap.name.clone());

                                        // 2. Match incoming BLE advertisement to a specific paired device:
                                        let matched_paired_dev = state.paired_airpods.iter().find(|ap| {
                                            if let Some(ref in_use) = in_use_paired_name {
                                                if ap.name == *in_use && model.matches_device_name(&ap.name) {
                                                    return true;
                                                }
                                            }
                                            false
                                        }).or_else(|| {
                                            state.paired_airpods.iter().find(|ap| {
                                                let is_conn = ap.is_connected || state.audio_devices.iter().any(|ad| {
                                                    is_airpods_playback_device(&ad.name)
                                                        && (ad.name.to_lowercase().contains(&ap.name.to_lowercase())
                                                            || ap.name.to_lowercase().contains(&ad.name.to_lowercase()))
                                                });
                                                is_conn && model.matches_device_name(&ap.name)
                                            })
                                        }).or_else(|| {
                                            state.paired_airpods.iter().find(|ap| model.matches_device_name(&ap.name))
                                        }).or_else(|| {
                                            if state.paired_airpods.len() == 1 {
                                                state.paired_airpods.first()
                                            } else {
                                                None
                                            }
                                        });

                                        let matched_name = matched_paired_dev
                                            .map(|d| d.name.clone())
                                            .unwrap_or_else(|| model.short_name().to_string());

                                        let is_matched_dev_connected = matched_paired_dev.map_or(false, |ap| {
                                            ap.is_connected || state.audio_devices.iter().any(|ad| {
                                                is_airpods_playback_device(&ad.name)
                                                    && (ad.name.to_lowercase().contains(&ap.name.to_lowercase())
                                                        || ap.name.to_lowercase().contains(&ad.name.to_lowercase()))
                                            })
                                        });

                                        let is_matched_dev_in_use = in_use_paired_name.as_ref() == Some(&matched_name);

                                        // 3. Update the per-device state slot:
                                        let dev_entry = state.airpods_states.entry(matched_name.clone()).or_default();
                                        dev_entry.model = Some(model.short_name().to_string());
                                        dev_entry.in_ear = is_any_in_ear;
                                        dev_entry.left_in_ear = left_in_ear;
                                        dev_entry.right_in_ear = right_in_ear;
                                        dev_entry.left_battery = left_battery;
                                        dev_entry.right_battery = right_battery;
                                        dev_entry.case_battery = case_battery;
                                        dev_entry.left_charging = left_charging;
                                        dev_entry.right_charging = right_charging;
                                        dev_entry.case_charging = case_charging;
                                        dev_entry.is_lid_opened = Some(is_lid_opened);
                                        dev_entry.is_both_pods_in_case = Some(is_both_pods_in_case);
                                        dev_entry.rssi = rssi;
                                        dev_entry.is_connected = is_matched_dev_connected;
                                        dev_entry.is_in_use = is_matched_dev_in_use;

                                        // 4. "In-Use First" Principle:
                                        // If an AirPods is currently in active audio use, ONLY events for that device update global state.
                                        let should_update_active = if let Some(ref in_use) = in_use_paired_name {
                                            matched_name == *in_use
                                        } else {
                                            is_matched_dev_connected || (left_in_ear || right_in_ear)
                                        };

                                        if !should_update_active {
                                            tracing::debug!(
                                                "[AirPods] Background event for '{}' saved to per-device slot; skipping active route overwrite",
                                                matched_name
                                            );
                                            broadcast(&state);
                                            continue;
                                        }

                                        state.airpods_detected = true;
                                        state.airpods_in_ear = is_any_in_ear;
                                        state.airpods_left_in_ear = left_in_ear;
                                        state.airpods_right_in_ear = right_in_ear;
                                        state.airpods_charging_left = left_charging;
                                        state.airpods_charging_right = right_charging;
                                        state.airpods_charging_case = case_charging;
                                        state.airpods_lid_opened = Some(is_lid_opened);
                                        state.airpods_both_in_case = Some(is_both_pods_in_case);
                                        state.airpods_model = Some(model.short_name().to_string());
                                        state.airpods_battery_left = left_battery;
                                        state.airpods_battery_right = right_battery;
                                        state.airpods_battery_case = case_battery;

                                        let current_worn_count = (left_in_ear as u8) + (right_in_ear as u8);
                                        // Both pods in case + lid closed, OR single-pod user with pod charging in case + lid closed
                                        let is_stored_in_case_and_closed = !is_lid_opened && (is_both_pods_in_case || (!is_any_in_ear && (left_charging || right_charging)));
                                        let is_both_in_case_unworn = is_both_pods_in_case && current_worn_count == 0;
                                        let is_stored_in_case = is_stored_in_case_and_closed || is_both_in_case_unworn;

                                        tracing::info!(
                                            "[AirPods] Event processed: worn_count={} (prev={}), left_in_ear={}, right_in_ear={}, lid_opened={}, both_in_case={}, auto_routing={}",
                                            current_worn_count, airpods_prev_worn_count, left_in_ear, right_in_ear, is_lid_opened, is_both_pods_in_case, state.airpods_auto_routing
                                        );

                                        if state.airpods_auto_routing {
                                            if is_stored_in_case {
                                                // 1. Stored in case (lid closed OR both earbuds in case):
                                                airpods_both_removed_at = None;

                                                // Only trigger route transition ON THE EDGE (exact moment earbuds placed in case)
                                                if !airpods_prev_in_case_closed {
                                                    tracing::info!(
                                                        "[AirPods] Edge event: Earbuds stored in case (closed={}, both_in_case={})",
                                                        is_stored_in_case_and_closed, is_both_in_case_unworn
                                                    );

                                                    const CASE_CLOSE_RESUME_TIMEOUT: Duration = Duration::from_secs(30);
                                                    let (is_playing, was_recently_playing) = crate::media_control::is_playing_or_recently_playing(Duration::from_secs(6));
                                                    let should_resume = (state.airpods_auto_pause || state.airpods_auto_routing)
                                                        && (
                                                            airpods_paused_at.as_ref().map_or(false, |t| t.elapsed() <= CASE_CLOSE_RESUME_TIMEOUT)
                                                            || is_playing
                                                            || was_recently_playing
                                                        );

                                                    if should_resume {
                                                        tracing::info!(
                                                            "[AirPods Handoff] Earbuds stored in case (paused_at={:?}) - arming event-driven auto-resume on restored route",
                                                            airpods_paused_at.as_ref().map(|t| t.elapsed())
                                                        );
                                                        let target = if interrupted_route.is_some() {
                                                            crate::handoff::HandoffTarget::HomePod
                                                        } else {
                                                            crate::handoff::HandoffTarget::DefaultSpeakers
                                                        };
                                                        crate::handoff::arm(target, Duration::from_secs(8));
                                                    }
                                                    airpods_paused_at = None;

                                                    if !runtime.is_streaming() && !matches!(state.status, DeviceStatus::Connecting) {
                                                        if interrupted_route.is_some() {
                                                            restore_interrupted_route(
                                                                interrupted_route.take(),
                                                                &cmd_tx,
                                                                &mut state,
                                                            ).await;
                                                        } else if state.ao_output_device != "default" {
                                                            tracing::info!("[AirPods] Earbuds stored in case: resetting local route to default speakers");
                                                            state.ao_output_device = "default".to_string();
                                                            save_ao_output_device("default");
                                                            transition_to_local_route(
                                                                &mut runtime,
                                                                &mut state,
                                                                &mut saved_homepod_volume,
                                                                &global_eq_params,
                                                                &global_sound_enhancer_params,
                                                                &global_warmth_enhancer_params,
                                                                true,
                                                                &shared_state,
                                                            ).await;
                                                        }
                                                    }
                                                } else if airpods_paused_at.is_some() {
                                                    airpods_paused_at = None;
                                                }
                                            } else if current_worn_count > airpods_prev_worn_count {
                                                // 2. Earbuds in ear: check audio route status
                                                airpods_both_removed_at = None;
                                                tracing::info!(
                                                    "[AirPods] Earbuds in ear (count={}, prev={}), checking audio route status",
                                                    current_worn_count, airpods_prev_worn_count
                                                );

                                                state.audio_devices = get_audio_render_devices();
                                                try_route_to_airpods(
                                                    &mut runtime,
                                                    &mut state,
                                                    &mut saved_homepod_volume,
                                                    &global_eq_params,
                                                    &global_sound_enhancer_params,
                                                    &global_warmth_enhancer_params,
                                                    &shared_state,
                                                    &mut interrupted_route,
                                                    &mut airpods_paused_at,
                                                    &cmd_tx,
                                                    true,
                                                ).await;
                                            } else if current_worn_count < airpods_prev_worn_count && airpods_prev_worn_count > 0 {
                                                // 3. Earbud removed to desk (not stored in case):
                                                if state.airpods_auto_pause {
                                                    let (is_playing, was_recently_playing) = crate::media_control::is_playing_or_recently_playing(Duration::from_secs(6));

                                                    if is_playing || was_recently_playing {
                                                        tracing::info!(
                                                            "[AirPods] Earbud removed (worn {} -> {}) while playing (active={}, recent={}) - marking auto-resume window (60s)",
                                                            airpods_prev_worn_count, current_worn_count, is_playing, was_recently_playing
                                                        );
                                                        airpods_paused_at = Some(tokio::time::Instant::now());
                                                        if is_playing {
                                                            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Pause);
                                                        }
                                                    } else if airpods_paused_at.is_some() {
                                                        tracing::info!(
                                                            "[AirPods] Additional earbud removed (worn {} -> {}) while already auto-paused - preserving auto-resume window",
                                                            airpods_prev_worn_count, current_worn_count
                                                        );
                                                    } else {
                                                        tracing::info!(
                                                            "[AirPods] Earbud removed (worn {} -> {}) but media was already paused - no auto-resume mark",
                                                            airpods_prev_worn_count, current_worn_count
                                                        );
                                                        airpods_paused_at = None;
                                                    }
                                                } else {
                                                    tracing::info!(
                                                        "[AirPods] Earbud removed (worn {} -> {}) but auto-pause is disabled - ignoring pause",
                                                        airpods_prev_worn_count, current_worn_count
                                                    );
                                                }
                                                if current_worn_count == 0 && airpods_both_removed_at.is_none() {
                                                    airpods_both_removed_at = Some(tokio::time::Instant::now());
                                                }
                                            }
                                        }

                                        airpods_prev_worn_count = current_worn_count;
                                        airpods_prev_in_case_closed = is_stored_in_case;
                                        broadcast(&state);
                                    }
                                }
                            }
                            GuiCmd::SystemSuspend => {
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

                                let was_streaming = runtime.is_streaming();
                                runtime.stop().await;

                                if was_streaming {
                                    tracing::info!("AirPlay stream stopped before system sleep; marked for reconnect on resume");
                                    state.status = DeviceStatus::Connecting;
                                    broadcast(&state);
                                }

                                crate::spectrum::reset_spectrum_levels();
                                set_audio_peak_level(0.0);
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
                                    runtime.stop().await;

                                    let devs = current_connected_devices.clone();
                                    let display_title = state.connected_name.clone().unwrap_or_else(|| "HomePod".to_string());
                                    state.status = DeviceStatus::Connecting;
                                    state.notice_message = Some((NoticeKind::ResumeReconnecting(display_title.clone()), std::time::Instant::now()));
                                    broadcast(&state);

                                    let eff_vol = state.volume;
                                    let sel_source = state.audio_source.clone();
                                    let dev_vols = state.device_volumes.clone();
                                    let low_lat = state.low_latency;
                                    let prevent_slp = state.prevent_sleep;
                                    let auto_mute = state.auto_mute_speakers && state.license_tier.is_auto_mute_allowed();
                                    let eq_enabled = state.eq_enabled;
                                    let eq_gains = state.eq_gains;
                                    let sound_enhancer_mode = state.sound_enhancer_mode;
                                    let warmth_enhancer_mode = state.warmth_enhancer_mode;

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
                                        match Session::start(target_devs, eff_vol, &dev_vols, sel_source.clone(), low_lat, eq_enabled, &eq_gains, sound_enhancer_mode, warmth_enhancer_mode).await {
                                            Ok(sess) => {
                                                tracing::info!("HomePod stream successfully restored after wake!");
                                                state.status = DeviceStatus::Streaming;
                                                state.diagnostics.timing_mode = sess.timing_desc.clone();
                                                state.notice_message = Some((NoticeKind::ResumeRestored(display_title.clone()), std::time::Instant::now()));
                                                runtime = AudioRouteRuntime::Streaming(sess);
                                                update_thread_execution_state(prevent_slp, true);
                                                if auto_mute {
                                                    record_and_configure_streaming_source(&sel_source);
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
                                        tracing::error!("All 5 wake auto-reconnect attempts failed. Falling back to local route.");
                                        let is_preempted = last_err.contains("453") || last_err.to_lowercase().contains("busy");
                                        let notice_kind = if is_preempted {
                                            NoticeKind::ResumePreempted(display_title.clone())
                                        } else {
                                            NoticeKind::ResumeFailed(display_title.clone())
                                        };
                                        state.notice_message = Some((notice_kind, std::time::Instant::now()));
                                        current_connected_devices.clear();

                                        transition_to_local_route(
                                            &mut runtime,
                                            &mut state,
                                            &mut saved_homepod_volume,
                                            &global_eq_params,
                                            &global_sound_enhancer_params,
                                            &global_warmth_enhancer_params,
                                            true,
                                            &shared_state,
                                        ).await;
                                    }
                                } else if !runtime.is_streaming() {
                                    tracing::info!("Power event: System resume from sleep without HomePod streaming. Restarting local route...");
                                    runtime.stop().await;
                                    crate::spectrum::reset_spectrum_levels();
                                    set_audio_peak_level(0.0);

                                    // Give audio endpoints and Windows Audio service a moment to power up
                                    tokio::time::sleep(Duration::from_millis(800)).await;
                                    if !runtime.is_streaming() {
                                        transition_to_local_route(
                                            &mut runtime,
                                            &mut state,
                                            &mut saved_homepod_volume,
                                            &global_eq_params,
                                            &global_sound_enhancer_params,
                                            &global_warmth_enhancer_params,
                                            true,
                                            &shared_state,
                                        ).await;
                                    }
                                }
                            }
                            GuiCmd::Quit => {
                                static IS_SHUTTING_DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                                if IS_SHUTTING_DOWN.compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst).is_err() {
                                    tracing::warn!("[Controller] Duplicate GuiCmd::Quit received, ignoring.");
                                    continue;
                                }
                                tracing::info!("[Controller] Processing GuiCmd::Quit: restoring speaker and devices...");
                                if let Some(mut rx_server) = receiver_instance.take() {
                                    rx_server.stop().await;
                                    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                                }
                                unregister_notification_client();
                                unregister_local_endpoint_listener();
                                restore_original_speaker_mute();
                                restore_initial_default_device();
                                runtime.stop().await;
                                restore_original_speaker_mute();
                                restore_initial_default_device();
                                update_thread_execution_state(state.prevent_sleep, false);
                                tracing::info!("[Controller] Shutdown complete, exiting cleanly.");
                                std::process::exit(0);
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
                        if let Ok(state) = dev.get_state() {
                            if state != wasapi::DeviceState::Active {
                                continue;
                            }
                        }
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
        if is_system_mix && is_airplay_streaming_active() {
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
                Ok(d) => {
                    let is_airpods = d.get_friendlyname().map(|n| is_airpods_playback_device(&n)).unwrap_or(false);
                    if is_airpods {
                        if let Some(standin_id) = get_standin_device_id() {
                            tracing::warn!("[AirPlay Capture] Default device is AirPods; diverting loopback capture to stand-in [{standin_id}]");
                            let _ = crate::policy_config::set_default_render_device(&standin_id);
                            shield_endpoint(Some(&standin_id));
                            enumerator.get_device(&standin_id).unwrap_or(d)
                        } else {
                            d
                        }
                    } else {
                        d
                    }
                }
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

        tracing::info!("WASAPI loopback audio capture active on device: {:?}", current_device_id);

        while !stop.load(Ordering::Relaxed) {
            // Purely event-driven default device change notification from IMMNotificationClient::OnDefaultDeviceChanged
            if DEFAULT_DEVICE_CHANGED.swap(false, Ordering::SeqCst) && is_system_mix {
                if stop.load(Ordering::Relaxed) || !STREAMING_ACTIVE.load(Ordering::SeqCst) {
                    break;
                }
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
                        if IS_SOURCE_MUTED.load(Ordering::Relaxed) {
                            samples.fill(0);
                            set_audio_peak_level(0.0);
                            crate::spectrum::reset_spectrum_levels();
                        } else {
                            let is_mono = is_audio_mono();
                            let bal = get_audio_balance();
                            if is_mono || bal != 0.0 {
                                for chunk in samples.chunks_exact_mut(2) {
                                    let mut l = chunk[0] as f32;
                                    let mut r = chunk[1] as f32;
                                    if is_mono {
                                        let m = (l + r) * 0.5;
                                        l = m;
                                        r = m;
                                    }
                                    if bal < 0.0 {
                                        r *= 1.0 + bal;
                                    } else if bal > 0.0 {
                                        l *= 1.0 - bal;
                                    }
                                    chunk[0] = l.clamp(-32768.0, 32767.0) as i16;
                                    chunk[1] = r.clamp(-32768.0, 32767.0) as i16;
                                }
                            }
                            if has_audio {
                                counters.last_active_ts.store(monotonic_now_ms(), Ordering::Relaxed);
                            }
                        }
                        let _ = sender.try_send(LivePcmFrame {
                            samples,
                            channels: CHANNELS,
                            sample_rate: RATE,
                        });
                    } else {
                        set_audio_peak_level(0.0);
                        crate::spectrum::reset_spectrum_levels();
                        let _ = sender.try_send(LivePcmFrame {
                            samples: silence.clone(),
                            channels: CHANNELS,
                            sample_rate: RATE,
                        });
                    }
                    let _ = h_event.wait_for_event(50);
                }
                Err(e) => {
                    set_audio_peak_level(0.0);
                    crate::spectrum::reset_spectrum_levels();
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

        crate::spectrum::reset_spectrum_levels();
        set_audio_peak_level(0.0);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LastRouteType {
    HomePod,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastConnectedTarget {
    pub name: String,
    pub is_stereo_pair: bool,
    pub device_ids: Vec<String>,
    #[serde(default)]
    pub expected_device_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_audio_device")]
    pub audio_device: String,
    #[serde(default)]
    pub audio_source: Option<AudioSource>,
    #[serde(default)]
    pub last_selected_app: Option<(String, String)>,
    #[serde(default = "default_audio_device")]
    pub last_selected_device: String,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default = "default_true")]
    pub auto_mute_speakers: bool,
    #[serde(default = "default_false")]
    pub auto_ao_mode: bool,
    #[serde(default = "default_audio_device")]
    pub ao_output_device: String,
    #[serde(default = "default_true")]
    pub prevent_sleep: bool,
    #[serde(default = "default_true")]
    pub low_latency: bool,
    #[serde(default = "default_true")]
    pub auto_connect: bool,
    #[serde(default)]
    pub last_device: Option<LastConnectedTarget>,
    #[serde(default)]
    pub last_route_type: Option<LastRouteType>,
    #[serde(default = "default_hotkey")]
    pub hotkey: (u32, u32),
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_false")]
    pub eq_enabled: bool,
    #[serde(default = "default_eq_gains")]
    pub eq_gains: Vec<f32>,
    #[serde(default = "default_custom_eq_gains")]
    pub custom_eq_gains: Vec<f32>,
    #[serde(default)]
    pub sound_enhancer_mode: SoundEnhancerMode,
    #[serde(default)]
    pub warmth_enhancer_mode: WarmthEnhancerMode,
    #[serde(default = "default_false")]
    pub always_on_top: bool,
    #[serde(default = "default_false")]
    pub visualizer_pinned: bool,
    #[serde(default = "default_true")]
    pub airpods_auto_routing: bool,
    #[serde(default = "default_true")]
    pub airpods_auto_pause: bool,
    #[serde(default = "default_true")]
    pub window_snapping: bool,
    #[serde(default = "default_snap_distance")]
    pub snap_distance: i32,
    #[serde(default = "default_false")]
    pub detach_equalizer_window: bool,
    #[serde(default)]
    pub standin_device_id: Option<String>,
    #[serde(default = "default_false")]
    pub airplay_receiver_enabled: bool,
    #[serde(default = "default_receiver_name")]
    pub airplay_receiver_name: String,
    #[serde(default)]
    pub airplay_receiver_target: AirPlayReceiverTarget,
    #[serde(default)]
    pub airplay_receiver_sync_level: AirPlayReceiverSyncLevel,
    #[serde(default = "default_true")]
    pub airplay_receiver_auto_max_volume: bool,
    #[serde(default = "default_audio_device")]
    pub airplay_receiver_output_device: String,
    #[serde(default = "default_false")]
    pub airplay_receiver_apply_effects: bool,
    #[serde(default = "default_false")]
    pub airplay_receiver_eq_enabled: bool,
    #[serde(default = "default_eq_gains")]
    pub airplay_receiver_eq_gains: Vec<f32>,
    #[serde(default = "default_custom_eq_gains")]
    pub airplay_receiver_custom_eq_gains: Vec<f32>,
    #[serde(default)]
    pub airplay_receiver_sound_enhancer_mode: SoundEnhancerMode,
    #[serde(default)]
    pub airplay_receiver_warmth_enhancer_mode: WarmthEnhancerMode,
}

fn default_receiver_name() -> String { "Win2Pod Audio Receiver".to_string() }
fn default_snap_distance() -> i32 { 12 }
fn default_audio_device() -> String { "default".to_string() }
fn default_volume() -> f32 { DEFAULT_VOLUME }
fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_eq_gains() -> Vec<f32> { vec![0.0; 10] }
fn default_custom_eq_gains() -> Vec<f32> { vec![0.0; 10] }
fn default_hotkey() -> (u32, u32) { (DEFAULT_HOTKEY_MODS, DEFAULT_HOTKEY_VK) }
fn default_language() -> String { Language::detect_system_language().code().to_string() }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            audio_device: default_audio_device(),
            audio_source: None,
            last_selected_app: None,
            last_selected_device: default_audio_device(),
            volume: default_volume(),
            auto_mute_speakers: default_true(),
            auto_ao_mode: false,
            ao_output_device: default_audio_device(),
            prevent_sleep: default_true(),
            low_latency: default_true(),
            auto_connect: default_true(),
            last_device: None,
            last_route_type: None,
            hotkey: default_hotkey(),
            language: default_language(),
            eq_enabled: false,
            eq_gains: default_eq_gains(),
            custom_eq_gains: default_custom_eq_gains(),
            sound_enhancer_mode: SoundEnhancerMode::Off,
            warmth_enhancer_mode: WarmthEnhancerMode::Off,
            always_on_top: default_false(),
            visualizer_pinned: default_false(),
            airpods_auto_routing: default_true(),
            airpods_auto_pause: default_true(),
            window_snapping: default_true(),
            snap_distance: default_snap_distance(),
            detach_equalizer_window: default_false(),
            standin_device_id: None,
            airplay_receiver_enabled: false,
            airplay_receiver_name: default_receiver_name(),
            airplay_receiver_target: AirPlayReceiverTarget::default(),
            airplay_receiver_sync_level: AirPlayReceiverSyncLevel::default(),
            airplay_receiver_auto_max_volume: default_true(),
            airplay_receiver_output_device: default_audio_device(),
            airplay_receiver_apply_effects: default_false(),
            airplay_receiver_eq_enabled: false,
            airplay_receiver_eq_gains: default_eq_gains(),
            airplay_receiver_custom_eq_gains: default_custom_eq_gains(),
            airplay_receiver_sound_enhancer_mode: SoundEnhancerMode::Off,
            airplay_receiver_warmth_enhancer_mode: WarmthEnhancerMode::Off,
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

pub fn load_audio_source() -> AudioSource {
    let cfg = load_config();
    if let Some(src) = cfg.audio_source {
        match src {
            AudioSource::Device(_) => AudioSource::Device("default".to_string()),
            AudioSource::Application { exe_name, display_name, .. } => {
                let pid = crate::app_enumerator::find_pid_by_exe(&exe_name).unwrap_or(0);
                AudioSource::Application { pid, exe_name, display_name }
            }
        }
    } else {
        AudioSource::Device("default".to_string())
    }
}

pub fn save_audio_source(source: &AudioSource) {
    update_config(|c| {
        c.audio_source = Some(source.clone());
        match source {
            AudioSource::Device(id) => {
                c.audio_device = id.clone();
                c.last_selected_device = id.clone();
            }
            AudioSource::Application { exe_name, display_name, .. } => {
                c.audio_device = format!("app:{exe_name}");
                c.last_selected_app = Some((exe_name.clone(), display_name.clone()));
            }
        }
    });
}

pub fn load_last_selected_app() -> Option<(String, String)> {
    load_config().last_selected_app
}

pub fn save_last_selected_app(exe: &str, display_name: &str) {
    update_config(|c| {
        c.last_selected_app = Some((exe.to_string(), display_name.to_string()));
    });
}

pub fn load_last_selected_device() -> String {
    let dev = load_config().last_selected_device;
    if dev.starts_with("app:") {
        dev
    } else {
        "default".to_string()
    }
}

pub fn save_last_selected_device(dev: &str) {
    let trimmed = dev.trim().to_string();
    update_config(|c| {
        c.last_selected_device = trimmed;
    });
}

pub fn load_audio_device() -> String {
    let dev = load_config().audio_device;
    if dev.starts_with("app:") {
        dev
    } else {
        "default".to_string()
    }
}

pub fn save_audio_device(id: &str) {
    let trimmed = id.trim().to_string();
    update_config(|c| {
        c.audio_device = trimmed.clone();
        c.audio_source = Some(AudioSource::Device(trimmed));
    });
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

pub fn load_auto_ao_mode() -> bool {
    load_config().auto_ao_mode
}

pub fn save_auto_ao_mode(enabled: bool) {
    update_config(|c| c.auto_ao_mode = enabled);
}

pub fn load_ao_output_device() -> String {
    load_config().ao_output_device
}

pub fn save_ao_output_device(dev: &str) {
    if dev.is_empty() || dev == "default" {
        update_config(|c| c.ao_output_device = "default".to_string());
        return;
    }
    if is_airpods_playback_device(dev) {
        tracing::warn!("save_ao_output_device: refusing to save AirPods name [{dev}] to config");
        return;
    }
    if dev.starts_with('{') {
        if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
            if let Ok(d) = enumerator.get_device(dev) {
                if let Ok(name) = d.get_friendlyname() {
                    if is_airpods_playback_device(&name) {
                        tracing::warn!("save_ao_output_device: refusing to save AirPods endpoint [{dev}] ({name}) to config");
                        return;
                    }
                }
            }
        }
    }
    let dev = dev.to_string();
    update_config(|c| c.ao_output_device = dev);
}

pub fn load_standin_device_id() -> Option<String> {
    load_config().standin_device_id
}

pub fn save_standin_device_id(id: &str) {
    let id_str = id.to_string();
    update_config(|c| c.standin_device_id = Some(id_str));
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
        expected_device_count: devices.len(),
    };
    update_config(|c| c.last_device = Some(target));
}

pub fn load_last_route_type() -> LastRouteType {
    LastRouteType::HomePod
}

pub fn save_last_route_type(_rt: LastRouteType) {
    // Disabled as requested: do not persist audio route across program runs
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

pub fn load_eq_enabled() -> bool {
    load_config().eq_enabled
}

pub fn save_eq_enabled(enabled: bool) {
    update_config(|c| c.eq_enabled = enabled);
}

pub fn load_eq_gains() -> [f32; 10] {
    let cfg = load_config();
    let mut gains = [0.0; 10];
    for (i, &g) in cfg.eq_gains.iter().take(10).enumerate() {
        gains[i] = g;
    }
    gains
}

pub fn save_eq_gains(gains: &[f32; 10]) {
    update_config(|c| c.eq_gains = gains.to_vec());
}

pub fn load_custom_eq_gains() -> [f32; 10] {
    let cfg = load_config();
    let mut gains = [0.0; 10];
    for (i, &g) in cfg.custom_eq_gains.iter().take(10).enumerate() {
        gains[i] = g;
    }
    gains
}

pub fn save_custom_eq_gains(gains: &[f32; 10]) {
    update_config(|c| c.custom_eq_gains = gains.to_vec());
}

pub fn load_sound_enhancer_mode() -> SoundEnhancerMode {
    load_config().sound_enhancer_mode
}

pub fn save_sound_enhancer_mode(mode: SoundEnhancerMode) {
    update_config(|c| c.sound_enhancer_mode = mode);
}

pub fn load_warmth_enhancer_mode() -> WarmthEnhancerMode {
    load_config().warmth_enhancer_mode
}

pub fn save_warmth_enhancer_mode(mode: WarmthEnhancerMode) {
    update_config(|c| c.warmth_enhancer_mode = mode);
}

pub fn load_always_on_top() -> bool {
    load_config().always_on_top
}

pub fn save_always_on_top(enabled: bool) {
    update_config(|c| c.always_on_top = enabled);
}

pub fn load_visualizer_pinned() -> bool {
    load_config().visualizer_pinned
}

pub fn save_visualizer_pinned(pinned: bool) {
    update_config(|c| c.visualizer_pinned = pinned);
}

pub fn load_airpods_auto_routing() -> bool {
    load_config().airpods_auto_routing
}

pub fn save_airpods_auto_routing(enabled: bool) {
    update_config(|c| c.airpods_auto_routing = enabled);
}

pub fn load_airpods_auto_pause() -> bool {
    load_config().airpods_auto_pause
}

pub fn save_airpods_auto_pause(enabled: bool) {
    update_config(|c| c.airpods_auto_pause = enabled);
}

pub fn load_window_snapping() -> bool {
    load_config().window_snapping
}

pub fn save_window_snapping(enabled: bool) {
    update_config(|c| c.window_snapping = enabled);
}

pub fn load_snap_distance() -> i32 {
    load_config().snap_distance
}

pub fn save_snap_distance(dist: i32) {
    update_config(|c| c.snap_distance = dist);
}

pub fn load_detach_equalizer_window() -> bool {
    load_config().detach_equalizer_window
}

pub fn save_detach_equalizer_window(enabled: bool) {
    update_config(|c| c.detach_equalizer_window = enabled);
}

pub fn load_airplay_receiver_enabled() -> bool {
    load_config().airplay_receiver_enabled
}

pub fn save_airplay_receiver_enabled(enabled: bool) {
    update_config(|c| c.airplay_receiver_enabled = enabled);
}

pub fn load_airplay_receiver_name() -> String {
    load_config().airplay_receiver_name
}

pub fn save_airplay_receiver_name(name: &str) {
    let name_owned = name.to_string();
    update_config(|c| c.airplay_receiver_name = name_owned);
}

pub fn load_airplay_receiver_sync_level() -> AirPlayReceiverSyncLevel {
    load_config().airplay_receiver_sync_level
}

pub fn save_airplay_receiver_sync_level(level: AirPlayReceiverSyncLevel) {
    update_config(|c| c.airplay_receiver_sync_level = level);
}

pub fn load_airplay_receiver_auto_max_volume() -> bool {
    load_config().airplay_receiver_auto_max_volume
}

pub fn save_airplay_receiver_auto_max_volume(auto_max: bool) {
    update_config(|c| c.airplay_receiver_auto_max_volume = auto_max);
}

pub fn load_airplay_receiver_output_device() -> String {
    load_config().airplay_receiver_output_device
}

pub fn save_airplay_receiver_output_device(dev: &str) {
    let dev_owned = dev.to_string();
    update_config(|c| c.airplay_receiver_output_device = dev_owned);
}

pub fn load_airplay_receiver_apply_effects() -> bool {
    load_config().airplay_receiver_apply_effects
}

pub fn save_airplay_receiver_apply_effects(enabled: bool) {
    update_config(|c| c.airplay_receiver_apply_effects = enabled);
}

pub fn load_airplay_receiver_eq_enabled() -> bool {
    load_config().airplay_receiver_eq_enabled
}

pub fn save_airplay_receiver_eq_enabled(enabled: bool) {
    update_config(|c| c.airplay_receiver_eq_enabled = enabled);
}

pub fn load_airplay_receiver_eq_gains() -> [f32; 10] {
    let cfg = load_config();
    let mut gains = [0.0; 10];
    for (i, &g) in cfg.airplay_receiver_eq_gains.iter().take(10).enumerate() {
        gains[i] = g;
    }
    gains
}

pub fn save_airplay_receiver_eq_gains(gains: &[f32; 10]) {
    update_config(|c| c.airplay_receiver_eq_gains = gains.to_vec());
}

pub fn load_airplay_receiver_custom_eq_gains() -> [f32; 10] {
    let cfg = load_config();
    let mut gains = [0.0; 10];
    for (i, &g) in cfg.airplay_receiver_custom_eq_gains.iter().take(10).enumerate() {
        gains[i] = g;
    }
    gains
}

pub fn save_airplay_receiver_custom_eq_gains(gains: &[f32; 10]) {
    update_config(|c| c.airplay_receiver_custom_eq_gains = gains.to_vec());
}

pub fn load_airplay_receiver_sound_enhancer_mode() -> SoundEnhancerMode {
    load_config().airplay_receiver_sound_enhancer_mode
}

pub fn save_airplay_receiver_sound_enhancer_mode(mode: SoundEnhancerMode) {
    update_config(|c| c.airplay_receiver_sound_enhancer_mode = mode);
}

pub fn load_airplay_receiver_warmth_enhancer_mode() -> WarmthEnhancerMode {
    load_config().airplay_receiver_warmth_enhancer_mode
}

pub fn save_airplay_receiver_warmth_enhancer_mode(mode: WarmthEnhancerMode) {
    update_config(|c| c.airplay_receiver_warmth_enhancer_mode = mode);
}

pub fn find_matching_devices(devices: &[DeviceItem], target: &LastConnectedTarget) -> Option<Vec<DeviceItem>> {
    let mut matched_devices: Vec<DeviceItem> = Vec::new();

    for dev in devices {
        let dev_id = dev.id_string();
        let is_matched = target.device_ids.iter().any(|tid| tid.eq_ignore_ascii_case(&dev_id))
            || dev.member_devices.iter().any(|m| {
                target.device_ids.iter().any(|tid| tid.eq_ignore_ascii_case(&m.id.to_mac_string()))
            });

        if is_matched && !matched_devices.iter().any(|d| d.id_string().eq_ignore_ascii_case(&dev_id)) {
            matched_devices.push(dev.clone());
        }
    }

    if !matched_devices.is_empty() {
        return Some(matched_devices);
    }

    // Fallback: match by name if MAC addresses were unavailable or mismatched
    for dev in devices {
        if dev.name.eq_ignore_ascii_case(&target.name) {
            return Some(vec![dev.clone()]);
        }
    }

    None
}

/// Dispatches playback control commands received from a HomePod receiver to the Windows SMTC media controller.
fn forward_homepod_remote_command(cmd: airplay_client::RemoteControlCmd) {
    tracing::info!("Dispatching HomePod remote control command: {:?}", cmd);
    match cmd {
        airplay_client::RemoteControlCmd::Play => {
            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Play);
        }
        airplay_client::RemoteControlCmd::Pause | airplay_client::RemoteControlCmd::TogglePlayPause => {
            // Note: In realtime streaming mode, the HomePod does not have an anchor timeline
            // and often remains in the pause state on the device side. Translating Pause to
            // TogglePlayPause ensures that tapping HomePod will alternate between pause and resume.
            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::TogglePlayPause);
        }
        airplay_client::RemoteControlCmd::NextTrack => {
            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Next);
        }
        airplay_client::RemoteControlCmd::PreviousTrack => {
            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Previous);
        }
        airplay_client::RemoteControlCmd::Stop => {
            crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Stop);
        }
    }
}
