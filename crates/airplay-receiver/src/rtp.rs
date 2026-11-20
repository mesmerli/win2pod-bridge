//! RTP UDP Audio Receiver and ALAC Decryption/Decoding.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use airplay_audio::AlacPacketDecoder;
use airplay_crypto::aes::AesCbcCipher;

use crate::session::SessionState;
use crate::sink::AudioSink;

pub async fn bind_reusable_udp(port: u16) -> Result<UdpSocket, Box<dyn std::error::Error + Send + Sync>> {
    use socket2::{Domain, Protocol, Socket, Type};
    use std::net::SocketAddr;

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let mut last_err = None;

    for attempt in 0..5 {
        let sock = (|| -> std::io::Result<UdpSocket> {
            let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
            socket.set_reuse_address(true)?;
            #[cfg(windows)]
            socket.set_nonblocking(true)?;
            socket.bind(&addr.into())?;
            let std_sock: std::net::UdpSocket = socket.into();
            UdpSocket::from_std(std_sock)
        })();

        match sock {
            Ok(s) => return Ok(s),
            Err(e) => {
                last_err = Some(e);
                if attempt < 4 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
                }
            }
        }
    }

    Err(Box::new(last_err.unwrap()))
}

pub async fn run_audio_receiver(
    port: u16,
    aes_key: [u8; 16],
    aes_iv: [u8; 16],
    session: Arc<Mutex<SessionState>>,
    sink: Arc<dyn AudioSink>,
    stop_signal: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let socket = bind_reusable_udp(port).await?;
    tracing::info!("UDP Audio Socket bound to 0.0.0.0:{}", port);

    let mut buf = vec![0u8; 4096];
    let cipher = AesCbcCipher::new(aes_key, aes_iv);
    let mut alac_decoder = AlacPacketDecoder::new()
        .map_err(|e| format!("Failed to create ALAC decoder: {}", e))?;

    while !stop_signal.load(Ordering::Relaxed) {
        let (len, _src) = match socket.recv_from(&mut buf).await {
            Ok(res) => res,
            Err(e) => {
                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }
                tracing::warn!("UDP audio receive error: {}", e);
                continue;
            }
        };

        if len < 12 {
            continue;
        }

        // RTP Header (12 bytes)
        let seq = u16::from_be_bytes([buf[2], buf[3]]);
        let timestamp = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);

        let payload = &buf[12..len];
        if payload.is_empty() {
            continue;
        }

        // AirPlay 1 RAOP audio encryption: only full 16-byte blocks are encrypted via AES-CBC
        let encrypted_len = (payload.len() / 16) * 16;
        let mut decrypted_payload = Vec::with_capacity(payload.len());

        if encrypted_len > 0 {
            match cipher.decrypt(&payload[..encrypted_len]) {
                Ok(decrypted_blocks) => {
                    decrypted_payload.extend_from_slice(&decrypted_blocks);
                    decrypted_payload.extend_from_slice(&payload[encrypted_len..]);
                }
                Err(e) => {
                    tracing::warn!("Failed to decrypt audio packet seq {}: {}", seq, e);
                    continue;
                }
            }
        } else {
            decrypted_payload.extend_from_slice(payload);
        }

        // Decode ALAC payload into 16-bit PCM
        match alac_decoder.decode_packet(&decrypted_payload, timestamp as u64) {
            Ok(samples) if !samples.is_empty() => {
                let (vol_db, ignore_vol) = {
                    let st = session.lock().await;
                    (st.volume_db, st.ignore_volume_control)
                };
                let vol_multiplier = if ignore_vol {
                    1.0_f32
                } else {
                    db_to_linear(vol_db)
                };

                let scaled_samples: Vec<i16> = if (vol_multiplier - 1.0).abs() > 0.001 {
                    samples
                        .into_iter()
                        .map(|s| (s as f32 * vol_multiplier).clamp(-32768.0, 32767.0) as i16)
                        .collect()
                } else {
                    samples
                };

                sink.on_audio_samples(&scaled_samples, 44100, 2);
            }
            Ok(_) => {} // RAOP initialization packet (<= 32 bytes)
            Err(e) => {
                tracing::debug!("ALAC decode error on seq {}: {}", seq, e);
            }
        }
    }

    Ok(())
}

pub fn db_to_linear(db: f32) -> f32 {
    if db <= -140.0 {
        0.0
    } else {
        10.0f32.powf(db / 20.0).clamp(0.0, 1.0)
    }
}
