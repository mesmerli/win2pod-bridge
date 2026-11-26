//! Win2Pod IPC Protocol definitions.
//!
//! Provides pure Rust, zero-GPL data transfer objects (DTOs) and message
//! definitions for communication between the GUI frontend and the backend Sidecar.

use serde::{Deserialize, Serialize};

pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\win2pod-bridge-ipc";

/// Envelope for all IPC communication over the named pipe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum IpcMessage {
    #[serde(rename = "Request")]
    Request {
        #[serde(default)]
        req_id: u64,
        payload: IpcRequest,
    },
    #[serde(rename = "Response")]
    Response {
        #[serde(default)]
        req_id: u64,
        payload: IpcResponse,
    },
    #[serde(rename = "Event")]
    Event {
        payload: IpcEvent,
    },
}

impl IpcMessage {
    /// Construct a new typed Request envelope.
    pub fn request(req_id: u64, payload: IpcRequest) -> Self {
        Self::Request { req_id, payload }
    }

    /// Construct a new typed Response envelope.
    pub fn response(req_id: u64, payload: IpcResponse) -> Self {
        Self::Response { req_id, payload }
    }

    /// Construct a new typed Event envelope.
    pub fn event(payload: IpcEvent) -> Self {
        Self::Event { payload }
    }

    /// Extract request ID if present in Request or Response.
    pub fn req_id(&self) -> Option<u64> {
        match self {
            Self::Request { req_id, .. } => Some(*req_id),
            Self::Response { req_id, .. } => Some(*req_id),
            Self::Event { .. } => None,
        }
    }
}

/// Requests initiated by the GUI frontend to the Sidecar engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
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

/// Errors occurring during IPC request execution or typed response parsing.
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum IpcError {
    #[error("Sidecar IPC remote error: {0}")]
    Remote(String),
    #[error("Unexpected IPC response: expected {expected}, got {actual}")]
    UnexpectedResponse {
        expected: &'static str,
        actual: String,
    },
    #[error("IPC channel closed")]
    ChannelClosed,
    #[error("IPC request timed out after {0:?}")]
    Timeout(std::time::Duration),
}

impl IpcResponse {
    /// Verify that the response is `Ok`, or return an `IpcError`.
    pub fn ok(self) -> Result<(), IpcError> {
        match self {
            IpcResponse::Ok => Ok(()),
            IpcResponse::Error(e) => Err(IpcError::Remote(e)),
            other => Err(IpcError::UnexpectedResponse {
                expected: "Ok",
                actual: format!("{other:?}"),
            }),
        }
    }

    /// Extract `Pong` nonce or return an `IpcError`.
    pub fn into_pong(self) -> Result<u64, IpcError> {
        match self {
            IpcResponse::Pong { nonce } => Ok(nonce),
            IpcResponse::Error(e) => Err(IpcError::Remote(e)),
            other => Err(IpcError::UnexpectedResponse {
                expected: "Pong",
                actual: format!("{other:?}"),
            }),
        }
    }

    /// Extract `Status` payload or return an `IpcError`.
    pub fn into_status(self) -> Result<EngineStatusIpc, IpcError> {
        match self {
            IpcResponse::Status(st) => Ok(st),
            IpcResponse::Error(e) => Err(IpcError::Remote(e)),
            other => Err(IpcError::UnexpectedResponse {
                expected: "Status",
                actual: format!("{other:?}"),
            }),
        }
    }

    /// Extract `AudioEndpoints` payload or return an `IpcError`.
    pub fn into_audio_endpoints(self) -> Result<Vec<AudioEndpointIpc>, IpcError> {
        match self {
            IpcResponse::AudioEndpoints(eps) => Ok(eps),
            IpcResponse::Error(e) => Err(IpcError::Remote(e)),
            other => Err(IpcError::UnexpectedResponse {
                expected: "AudioEndpoints",
                actual: format!("{other:?}"),
            }),
        }
    }

    /// Return true if status is `Ok`.
    pub fn is_ok(&self) -> bool {
        matches!(self, IpcResponse::Ok)
    }

    /// Return true if status is `Error`.
    pub fn is_error(&self) -> bool {
        matches!(self, IpcResponse::Error(_))
    }
}

/// Trait binding an IPC request to its strongly-typed expected response.
pub trait IpcRequestPayload: Sized + Into<IpcRequest> {
    type ExpectedResponse;
    fn extract_response(resp: IpcResponse) -> Result<Self::ExpectedResponse, IpcError>;
}

/// Typed request for engine ping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PingRequest {
    pub nonce: u64,
}

impl From<PingRequest> for IpcRequest {
    fn from(r: PingRequest) -> Self {
        IpcRequest::Ping { nonce: r.nonce }
    }
}

impl IpcRequestPayload for PingRequest {
    type ExpectedResponse = u64;
    fn extract_response(resp: IpcResponse) -> Result<Self::ExpectedResponse, IpcError> {
        resp.into_pong()
    }
}

/// Typed request for engine status query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetStatusRequest;

impl From<GetStatusRequest> for IpcRequest {
    fn from(_: GetStatusRequest) -> Self {
        IpcRequest::GetStatus
    }
}

impl IpcRequestPayload for GetStatusRequest {
    type ExpectedResponse = EngineStatusIpc;
    fn extract_response(resp: IpcResponse) -> Result<Self::ExpectedResponse, IpcError> {
        resp.into_status()
    }
}

/// Typed request for audio endpoints enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumerateAudioEndpointsRequest;

impl From<EnumerateAudioEndpointsRequest> for IpcRequest {
    fn from(_: EnumerateAudioEndpointsRequest) -> Self {
        IpcRequest::EnumerateAudioEndpoints
    }
}

impl IpcRequestPayload for EnumerateAudioEndpointsRequest {
    type ExpectedResponse = Vec<AudioEndpointIpc>;
    fn extract_response(resp: IpcResponse) -> Result<Self::ExpectedResponse, IpcError> {
        resp.into_audio_endpoints()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message_request_serde_with_req_id() {
        let msg = IpcMessage::Request {
            req_id: 42,
            payload: IpcRequest::Ping { nonce: 999 },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Request""#));
        assert!(json.contains(r#""req_id":42"#));
        assert!(json.contains(r#""action":"Ping""#));

        let deserialized: IpcMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg);
        assert_eq!(deserialized.req_id(), Some(42));
    }

    #[test]
    fn test_ipc_message_legacy_backward_compatibility() {
        // Legacy JSON without req_id field
        let legacy_json = r#"{"type":"Request","payload":{"action":"Ping","params":{"nonce":123}}}"#;
        let deserialized: IpcMessage = serde_json::from_str(legacy_json).unwrap();
        match deserialized {
            IpcMessage::Request { req_id, payload } => {
                assert_eq!(req_id, 0); // Defaults to 0 when omitted
                match payload {
                    IpcRequest::Ping { nonce } => assert_eq!(nonce, 123),
                    other => panic!("Unexpected request: {:?}", other),
                }
            }
            other => panic!("Unexpected message: {:?}", other),
        }
    }

    #[test]
    fn test_ipc_message_response_and_event() {
        let resp_msg = IpcMessage::response(88, IpcResponse::Pong { nonce: 12345 });
        let json = serde_json::to_string(&resp_msg).unwrap();
        let parsed: IpcMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, resp_msg);
        assert_eq!(parsed.req_id(), Some(88));

        let evt_msg = IpcMessage::event(IpcEvent::StreamingStateChanged {
            state: StreamStateIpc::Streaming,
            connected_name: Some("Living Room".into()),
            connected_device_ids: vec!["AA:BB:CC".into()],
            error: None,
        });
        let evt_json = serde_json::to_string(&evt_msg).unwrap();
        assert!(evt_json.contains(r#""type":"Event""#));
        let parsed_evt: IpcMessage = serde_json::from_str(&evt_json).unwrap();
        assert_eq!(parsed_evt, evt_msg);
        assert_eq!(parsed_evt.req_id(), None);
    }

    #[test]
    fn test_ipc_response_helpers() {
        assert!(IpcResponse::Ok.is_ok());
        assert!(!IpcResponse::Ok.is_error());
        assert!(IpcResponse::Ok.ok().is_ok());

        let pong = IpcResponse::Pong { nonce: 777 };
        assert_eq!(pong.into_pong().unwrap(), 777);

        let err_resp = IpcResponse::Error("Device unreachable".into());
        assert!(err_resp.is_error());
        assert_eq!(
            err_resp.ok().unwrap_err(),
            IpcError::Remote("Device unreachable".into())
        );

        let unexpected = IpcResponse::Ok;
        match unexpected.into_pong() {
            Err(IpcError::UnexpectedResponse { expected, .. }) => assert_eq!(expected, "Pong"),
            other => panic!("Expected UnexpectedResponse error, got {:?}", other),
        }
    }

    #[test]
    fn test_typed_ipc_payload() {
        let ping = PingRequest { nonce: 55 };
        let req: IpcRequest = ping.into();
        match req {
            IpcRequest::Ping { nonce } => assert_eq!(nonce, 55),
            other => panic!("Unexpected req: {:?}", other),
        }
        let pong = IpcResponse::Pong { nonce: 55 };
        let extracted = PingRequest::extract_response(pong).unwrap();
        assert_eq!(extracted, 55);

        let endpoints = vec![AudioEndpointIpc {
            id: "ep1".into(),
            name: "Default Speakers".into(),
            is_default: true,
        }];
        let ep_resp = IpcResponse::AudioEndpoints(endpoints.clone());
        let extracted_eps = EnumerateAudioEndpointsRequest::extract_response(ep_resp).unwrap();
        assert_eq!(extracted_eps, endpoints);
    }
}
