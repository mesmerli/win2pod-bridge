//! # airplay-receiver
//!
//! Pure Rust AirPlay 1 (RAOP) Audio Receiver for Windows and cross-platform streaming.
//!
//! Features:
//! - Full mDNS discovery as an AirTunes / AirPlay 1 audio receiver (`_raop._tcp`).
//! - RTSP handshake with RFC 3447 PKCS#1 v1.5 RSA-2048 `Apple-Challenge` digital signing.
//! - RSA-OAEP session AES-128 key decryption.
//! - AES-128-CBC real-time encrypted RTP payload decryption.
//! - Pure Rust ALAC frame decoding (via Symphonia) without GStreamer or external C/C++ libraries.
//! - Flexible `AudioSink` abstraction:
//!   - `LocalSpeakerSink`: Low-latency Windows WASAPI direct playback to PC speakers.
//!   - `LiveFrameSink`: Direct bridging into `LiveFrameSender` for relaying to HomePod!
//!   - `TeeSink`: Simultaneous multi-output (e.g. PC Speakers + HomePod multi-room).
//!   - `CallbackSink`: Custom callbacks for visualizers and UI level meters.

pub mod config;
pub mod control;
pub mod dacp;
pub mod mdns;
pub mod rtp;
pub mod rtsp;
pub mod server;
pub mod session;
pub mod sink;

pub use config::AirPlayReceiverConfig;
pub use dacp::{send_dacp_command, resolve_dacp_device, DacpCommand};
pub use server::AirPlayReceiver;
pub use session::SessionState;
pub use sink::{AudioSink, CallbackSink, LiveFrameSink, LocalSpeakerSink, MixerSink, ReceiverDspParams, SwitchableSink, TeeSink};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_config_builder() {
        let config = AirPlayReceiverConfig::new("Test Receiver")
            .with_ports(5000, 6000, 6001, 6002)
            .with_mac_address([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);

        assert_eq!(config.name, "Test Receiver");
        assert_eq!(config.rtsp_port, 5000);
        assert_eq!(config.audio_port, 6000);
        assert_eq!(config.control_port, 6001);
        assert_eq!(config.timing_port, 6002);
        assert_eq!(config.mac_address, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    }

    #[test]
    fn test_callback_sink() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&counter);

        let sink = CallbackSink::new(move |samples, _rate, _channels| {
            c.fetch_add(samples.len(), Ordering::Relaxed);
        });

        sink.on_audio_samples(&[1, 2, 3, 4], 44100, 2);
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }
}
