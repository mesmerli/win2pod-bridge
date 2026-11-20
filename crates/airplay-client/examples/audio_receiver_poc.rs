//! # AirPlay 1 RAOP Audio Receiver (Standalone Binary)
//!
//! Powered by `airplay-receiver`:
//! - Pure Rust AirPlay 1 RAOP server
//! - RSA-OAEP session key decryption
//! - Low-latency ALAC decode
//! - Windows WASAPI Local Speaker output

use std::sync::Arc;
use airplay_receiver::{AirPlayReceiver, AirPlayReceiverConfig, LocalSpeakerSink};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    println!("======================================================================");
    println!("     Win2Pod Bridge: Pure Rust AirPlay Audio Receiver                 ");
    println!("======================================================================");

    let config = AirPlayReceiverConfig::new("Win2Pod Audio Receiver");
    println!(" [1] Service Name : {}", config.name);
    println!(" [2] RTSP Server  : 0.0.0.0:{}", config.rtsp_port);
    println!(" [3] Audio Port   : UDP {}", config.audio_port);
    println!(" [4] Control Port : UDP {}", config.control_port);
    println!(" [5] Timing Port  : UDP {}", config.timing_port);
    println!("======================================================================");

    let sink = Arc::new(LocalSpeakerSink::new()?);
    let mut receiver = AirPlayReceiver::new(config, sink);

    receiver.start().await?;
    println!(" [✓] Ready! AirPlay receiver is running. Look for it on iPhone/Mac!");
    println!("     Press Ctrl+C to terminate.\n");

    tokio::signal::ctrl_c().await?;
    println!("\nShutting down receiver...");
    receiver.stop().await;
    println!("Done.");

    Ok(())
}
