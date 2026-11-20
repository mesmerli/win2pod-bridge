//! UDP Control (Sync) and Timing (NTP) Sockets.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::rtp::bind_reusable_udp;

pub async fn run_control_receiver(
    port: u16,
    stop_signal: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let socket = bind_reusable_udp(port).await?;
    let mut buf = vec![0u8; 2048];

    while !stop_signal.load(Ordering::Relaxed) {
        let (len, _src) = match socket.recv_from(&mut buf).await {
            Ok(res) => res,
            Err(_) => {
                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }
                continue;
            }
        };

        if len >= 12 {
            let pt = buf[1] & 0x7F;
            if pt == 84 {
                // Sync packet
                tracing::trace!("Received RTP Sync packet (PT=84)");
            }
        }
    }

    Ok(())
}

pub async fn run_timing_receiver(
    port: u16,
    stop_signal: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let socket = bind_reusable_udp(port).await?;
    let mut buf = vec![0u8; 2048];

    while !stop_signal.load(Ordering::Relaxed) {
        let (len, src) = match socket.recv_from(&mut buf).await {
            Ok(res) => res,
            Err(_) => {
                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }
                continue;
            }
        };

        if len >= 32 {
            let pt = buf[1] & 0x7F;
            if pt == 82 {
                // TIMING_REQUEST -> respond with TIMING_RESPONSE (PT=83)
                let mut resp = buf[..len].to_vec();
                resp[1] = (buf[1] & 0x80) | 83;
                let _ = socket.send_to(&resp, src).await;
            }
        }
    }

    Ok(())
}
