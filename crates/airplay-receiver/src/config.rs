//! Configuration for AirPlay 1 RAOP Audio Receiver.

use std::net::IpAddr;

/// Configuration options for the AirPlay receiver.
#[derive(Debug, Clone)]
pub struct AirPlayReceiverConfig {
    /// Human-readable service name advertised via mDNS (e.g. "Win2Pod Audio Receiver").
    pub name: String,
    /// RTSP server TCP port (default 5000).
    pub rtsp_port: u16,
    /// UDP RTP audio data port (default 6000).
    pub audio_port: u16,
    /// UDP control socket port for sync packets (default 6001).
    pub control_port: u16,
    /// UDP timing socket port for NTP packets (default 6002).
    pub timing_port: u16,
    /// Virtual MAC address advertised in the RAOP TXT record (6 bytes).
    pub mac_address: [u8; 6],
    /// Explicit local IP address to bind and advertise. If None, auto-detected.
    pub local_ip: Option<IpAddr>,
    /// When true, received audio is always decoded and output at full 100% digital scale (0 dB),
    /// ignoring negative attenuation from the iOS client volume slider.
    pub ignore_volume_control: bool,
}

impl Default for AirPlayReceiverConfig {
    fn default() -> Self {
        Self {
            name: "Win2Pod Audio Receiver".to_string(),
            rtsp_port: 5000,
            audio_port: 6000,
            control_port: 6001,
            timing_port: 6002,
            mac_address: [0x02, 0x00, 0x00, 0x00, 0x00, 0x01],
            local_ip: None,
            ignore_volume_control: false,
        }
    }
}

impl AirPlayReceiverConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn with_ports(mut self, rtsp: u16, audio: u16, control: u16, timing: u16) -> Self {
        self.rtsp_port = rtsp;
        self.audio_port = audio;
        self.control_port = control;
        self.timing_port = timing;
        self
    }

    pub fn with_mac_address(mut self, mac: [u8; 6]) -> Self {
        self.mac_address = mac;
        self
    }

    pub fn with_local_ip(mut self, ip: IpAddr) -> Self {
        self.local_ip = Some(ip);
        self
    }
}
