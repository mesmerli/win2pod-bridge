//! Win2Pod IPC Protocol definitions.
//!
//! Provides pure Rust, zero-GPL data transfer objects (DTOs) and message
//! definitions for communication between the GUI frontend and the backend Sidecar.

use serde::{Deserialize, Serialize};

pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\win2pod-bridge-ipc";

/// Envelope for all IPC communication over the named pipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcMessage {
    Request(IpcRequest),
    Response(IpcResponse),
    Event(IpcEvent),
}

/// Requests initiated by the GUI frontend to the Sidecar engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "params")]
pub enum IpcRequest {
    Ping {
        nonce: u64,
    },
    DiscoverDevices {
        timeout_secs: u64,
    },
    Connect {
        target: ConnectTargetIpc,
        volume: f32,
        low_latency: bool,
    },
    Disconnect,
    SetVolume {
        volume: f32,
        device_id: Option<String>,
    },
    SetEq {
        enabled: bool,
        gains: [f32; 10],
        enhancer_mode: u8,
        warmth_mode: u8,
    },
    SetAudioSource {
        source: AudioSourceIpc,
    },
    SetAirPlayReceiver {
        enabled: bool,
        name: String,
        sync_level: u8,
        auto_max_volume: bool,
        output_device: String,
        apply_effects: bool,
    },
    SetAirPodsMonitoring {
        enabled: bool,
    },
    EnumerateAudioEndpoints,
    SetEngineMode {
        mode: EngineModeIpc,
    },
    SetSpeakerMute {
        mute: bool,
        device_id: Option<String>,
    },
    SetStandinRedirection {
        standin_id: Option<String>,
    },
    GetStatus,
    Shutdown,
}

/// Responses sent back by the Sidecar engine in reply to requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum IpcResponse {
    Pong {
        nonce: u64,
    },
    Ok,
    Error(String),
    Status(EngineStatusIpc),
    AudioEndpoints(Vec<AudioEndpointIpc>),
}

/// Asynchronous events emitted by the Sidecar engine to the GUI frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum IpcEvent {
    DeviceDiscovered(DeviceIpc),
    StreamingStateChanged {
        state: StreamStateIpc,
        connected_name: Option<String>,
        connected_device_ids: Vec<String>,
        error: Option<String>,
    },
    DiagnosticsUpdate(DiagnosticsIpc),
    AirPodsStateUpdate(AirPodsStatusIpc),
    ReceiverStateUpdate(ReceiverStatusIpc),
    FftSpectrumUpdate(Vec<f32>),
    RawSpectrumUpdate(Vec<u8>),
    AudioActivityChanged {
        is_active: bool,
        peak_level: f32,
    },
    AudioEndpointsChanged(Vec<AudioEndpointIpc>),
    RemoteMediaControl(RemoteCmdIpc),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectTargetIpc {
    Single(DeviceIpc),
    StereoPair {
        name: String,
        left: DeviceIpc,
        right: DeviceIpc,
    },
    Group {
        name: String,
        devices: Vec<DeviceIpc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceIpc {
    pub id: String,
    pub name: String,
    pub model: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub is_stereo_pair: bool,
    pub is_group: bool,
    pub stereo_role: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamStateIpc {
    Idle,
    Connecting,
    Streaming,
    Paused,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiagnosticsIpc {
    pub rtt_ms: f64,
    pub clock_offset_us: i64,
    pub packets_sent: u64,
    pub packets_lost: u64,
    pub loss_rate: f32,
    pub buffer_occupancy: usize,
    pub underruns: u64,
    pub timing_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AirPodsStatusIpc {
    pub detected: bool,
    pub in_ear: bool,
    pub left_in_ear: bool,
    pub right_in_ear: bool,
    pub model: Option<String>,
    pub battery_left: Option<u8>,
    pub battery_right: Option<u8>,
    pub battery_case: Option<u8>,
    pub charging_left: bool,
    pub charging_right: bool,
    pub charging_case: bool,
    pub lid_opened: Option<bool>,
    pub both_in_case: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReceiverStatusIpc {
    pub state: String,
    pub client_name: Option<String>,
    pub client_ip: Option<String>,
    pub device_type: Option<String>,
    pub volume: Option<u8>,
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
    pub track_album: Option<String>,
    pub has_artwork: bool,
    pub artwork_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioSourceIpc {
    Device(String),
    Application {
        pid: u32,
        exe_name: String,
        display_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemoteCmdIpc {
    Play,
    Pause,
    TogglePlayPause,
    NextTrack,
    PreviousTrack,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioEndpointIpc {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EngineModeIpc {
    Idle,
    AirPlay,
    LocalAo {
        target_output_device: Option<String>,
    },
}

impl Default for EngineModeIpc {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngineStatusIpc {
    pub stream_state: StreamStateIpc,
    pub connected_name: Option<String>,
    pub active_devices: Vec<DeviceIpc>,
    pub receiver_enabled: bool,
    pub airpods_active: bool,
}

impl Default for StreamStateIpc {
    fn default() -> Self {
        Self::Idle
    }
}
