//! RTSP Server and Connection Handling for AirPlay 1 RAOP.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use airplay_crypto::rsa::{decrypt_aes_key, generate_apple_response};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::config::AirPlayReceiverConfig;
use crate::control::{run_control_receiver, run_timing_receiver};
use crate::rtp::run_audio_receiver;
use crate::session::SessionState;
use crate::sink::AudioSink;

/// Parse DMAP (Digital Media Access Protocol) TLV structure to extract track metadata.
pub fn parse_dmap(
    data: &[u8],
    title: &mut Option<String>,
    artist: &mut Option<String>,
    album: &mut Option<String>,
) {
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let tag = &data[offset..offset + 4];
        let len = u32::from_be_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        offset += 8;
        if offset + len > data.len() {
            break;
        }
        let val = &data[offset..offset + len];
        match tag {
            b"minm" => {
                if let Ok(s) = std::str::from_utf8(val) {
                    *title = Some(s.to_string());
                }
            }
            b"asar" => {
                if let Ok(s) = std::str::from_utf8(val) {
                    *artist = Some(s.to_string());
                }
            }
            b"asal" => {
                if let Ok(s) = std::str::from_utf8(val) {
                    *album = Some(s.to_string());
                }
            }
            _ => {
                // If container or nested tags, recurse if length is at least 8 bytes
                if len >= 8 {
                    parse_dmap(val, title, artist, album);
                }
            }
        }
        offset += len;
    }
}

pub async fn handle_rtsp_connection(
    mut stream: TcpStream,
    remote_addr: SocketAddr,
    config: AirPlayReceiverConfig,
    local_ip_bytes: Vec<u8>,
    session: Arc<Mutex<SessionState>>,
    sink: Arc<dyn AudioSink>,
    stop_signal: Arc<AtomicBool>,
    active_conn_count: Arc<AtomicUsize>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut read_buf = vec![0u8; 8192];
    let mut acc_buf = Vec::new();
    let mut udp_stop_signals: Vec<Arc<AtomicBool>> = Vec::new();
    let conn_alive = Arc::new(AtomicBool::new(true));

    {
        let mut st = session.lock().await;
        st.remote_addr = Some(remote_addr);
    }

    let mut device_model = "iPhone".to_string();

    loop {
        if stop_signal.load(Ordering::Relaxed) {
            tracing::info!("AirPlay receiver is stopping, closing RTSP stream to {}", remote_addr);
            break;
        }

        // 1. Process any fully buffered requests in acc_buf
        while let Some(header_end) = acc_buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let header_str = String::from_utf8_lossy(&acc_buf[..header_end]);
            let lines: Vec<&str> = header_str.lines().collect();
            if lines.is_empty() {
                acc_buf.drain(..header_end + 4);
                continue;
            }

            let req_line: Vec<&str> = lines[0].split_whitespace().collect();
            if req_line.is_empty() {
                acc_buf.drain(..header_end + 4);
                continue;
            }

            let method = req_line[0];
            let uri = req_line.get(1).copied().unwrap_or("");
            let mut headers = HashMap::new();

            for line in &lines[1..] {
                if line.trim().is_empty() {
                    break;
                }
                if let Some((k, v)) = line.split_once(':') {
                    headers.insert(k.trim().to_lowercase(), v.trim().to_string());
                }
            }

            let content_length: usize = headers
                .get("content-length")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);

            let total_req_len = header_end + 4 + content_length;
            if acc_buf.len() < total_req_len {
                // Incomplete body, break out to read more bytes from stream
                break;
            }

            let body_bytes = &acc_buf[header_end + 4..total_req_len];

            // Capture DACP identifiers from headers
            if let Some(dacp) = headers.get("dacp-id") {
                let mut st = session.lock().await;
                st.dacp_id = Some(dacp.clone());
            }
            if let Some(remote) = headers.get("active-remote") {
                let mut st = session.lock().await;
                st.active_remote = Some(remote.clone());
            }

            // Capture client device name from headers if available
            if let Some(name) = headers.get("x-apple-client-name") {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    let mut st = session.lock().await;
                    if st.client_name.as_deref() != Some(trimmed) {
                        st.client_name = Some(trimmed.to_string());
                    }
                }
            } else if let Some(ua) = headers.get("user-agent") {
                if ua.contains("iPad") {
                    device_model = "iPad".to_string();
                } else if ua.contains("Macintosh") || ua.contains("Mac") {
                    device_model = "Mac".to_string();
                } else if ua.contains("AppleTV") {
                    device_model = "Apple TV".to_string();
                } else if ua.contains("iPhone") {
                    device_model = "iPhone".to_string();
                }
                let mut st = session.lock().await;
                if st.client_name.is_none() {
                    st.client_name = Some(device_model.clone());
                }
            }

            // Apple-Challenge authentication response
            let mut apple_response_header = String::new();
            if let Some(challenge_b64) = headers.get("apple-challenge") {
                let effective_ip_bytes = match stream.local_addr() {
                    Ok(addr) => match addr.ip() {
                        IpAddr::V4(v4) => v4.octets().to_vec(),
                        IpAddr::V6(v6) => v6.octets().to_vec(),
                    },
                    Err(_) => local_ip_bytes.clone(),
                };
                match generate_apple_response(challenge_b64, &effective_ip_bytes, &config.mac_address) {
                    Ok(resp) => {
                        apple_response_header = format!("Apple-Response: {}\r\n", resp);
                    }
                    Err(e) => {
                        tracing::error!("Failed to generate Apple-Response: {}", e);
                    }
                }
            }

            let cseq = headers.get("cseq").cloned().unwrap_or_else(|| "1".to_string());
            tracing::debug!("[RTSP] {} {} (CSeq: {})", method, uri, cseq);

            match method {
                "OPTIONS" => {
                    let remote_ip = remote_addr.ip();
                    let current_name = session.lock().await.client_name.clone().unwrap_or_else(|| device_model.clone());
                    sink.on_client_connected(&current_name, &device_model, &remote_ip.to_string());

                    // Start background mDNS PTR resolution immediately on OPTIONS so device name appears in ~5ms
                    let session_clone = Arc::clone(&session);
                    let sink_clone = Arc::clone(&sink);
                    let dm_clone = device_model.clone();
                    let dacp_id = session.lock().await.dacp_id.clone();
                    let conn_alive_clone = Arc::clone(&conn_alive);
                    tokio::spawn(async move {
                        let (resolved_name, resolved_port) = crate::dacp::resolve_dacp_device(remote_ip, dacp_id).await;
                        if !conn_alive_clone.load(Ordering::Relaxed) {
                            return;
                        }
                        if let Some(port) = resolved_port {
                            session_clone.lock().await.dacp_port = Some(port);
                        }
                        if let Some(name) = resolved_name {
                            if !conn_alive_clone.load(Ordering::Relaxed) {
                                return;
                            }
                            session_clone.lock().await.client_name = Some(name.clone());
                            sink_clone.on_client_name_change(&name);
                            sink_clone.on_client_connected(&name, &dm_clone, &remote_ip.to_string());
                        }
                    });

                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         Public: ANNOUNCE, SETUP, RECORD, PAUSE, FLUSH, TEARDOWN, OPTIONS, GET_PARAMETER, SET_PARAMETER\r\n\
                         Audio-Jack-Status: connected; type=analog\r\n\
                         {}\
                         \r\n",
                        cseq, apple_response_header
                    );
                    stream.write_all(response.as_bytes()).await?;
                }

                "ANNOUNCE" => {
                    // Abort any lingering UDP tasks and reset prior session state
                    {
                        let st = session.lock().await;
                        st.abort_udp_tasks();
                    }
                    for stop in udp_stop_signals.drain(..) {
                        stop.store(true, Ordering::Relaxed);
                    }

                    let body = String::from_utf8_lossy(body_bytes);
                    let mut sdp_rsaaeskey = None;
                    let mut sdp_aesiv = None;

                    for line in body.lines() {
                        let trimmed = line.trim();
                        if let Some(rest) = trimmed.strip_prefix("a=rsaaeskey:") {
                            sdp_rsaaeskey = Some(rest.trim().to_string());
                        } else if let Some(rest) = trimmed.strip_prefix("a=aesiv:") {
                            sdp_aesiv = Some(rest.trim().to_string());
                        }
                    }

                    if let (Some(b64_key), Some(b64_iv)) = (sdp_rsaaeskey, sdp_aesiv) {
                        if let (Ok(enc_key), Ok(raw_iv)) = (BASE64.decode(&b64_key), BASE64.decode(&b64_iv)) {
                            match decrypt_aes_key(&enc_key) {
                                Ok(aes_key) => {
                                    tracing::info!("AirPlay AES session key successfully decrypted");
                                    let mut state = session.lock().await;
                                    state.aes_key = Some(aes_key);
                                    let mut iv = [0u8; 16];
                                    iv.copy_from_slice(&raw_iv[..16]);
                                    state.aes_iv = Some(iv);
                                    state.remote_addr = Some(remote_addr);
                                }
                                Err(e) => tracing::error!("Failed to decrypt AES key: {}", e),
                            }
                        }
                    }

                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         Audio-Jack-Status: connected; type=analog\r\n\
                         {}\
                         \r\n",
                        cseq, apple_response_header
                    );
                    stream.write_all(response.as_bytes()).await?;
                }

                "SETUP" => {
                    // Terminate any previous UDP tasks so sockets are immediately released
                    {
                        let state_guard = session.lock().await;
                        state_guard.abort_udp_tasks();
                    }
                    tokio::task::yield_now().await;

                    let state = session.lock().await.clone();
                    let (aes_key, aes_iv) = match (state.aes_key, state.aes_iv) {
                        (Some(k), Some(iv)) => (k, iv),
                        _ => {
                            tracing::warn!("RTSP SETUP received from {} without prior ANNOUNCE (no AES key). Responding with 454 Session Not Found.", remote_addr);
                            let response = format!(
                                "RTSP/1.0 454 Session Not Found\r\n\
                                 CSeq: {}\r\n\
                                 Server: AirTunes/220.68\r\n\
                                 {}\
                                 \r\n",
                                cseq, apple_response_header
                            );
                            stream.write_all(response.as_bytes()).await?;
                            acc_buf.drain(..total_req_len);
                            continue;
                        }
                    };

                    // 1. Audio UDP socket
                    let stop_audio = Arc::new(AtomicBool::new(false));
                    let stop_audio_clone = Arc::clone(&stop_audio);
                    let session_clone = Arc::clone(&session);
                    let sink_clone = Arc::clone(&sink);
                    let audio_port = config.audio_port;
                    let h_audio = tokio::spawn(async move {
                        if let Err(e) = run_audio_receiver(audio_port, aes_key, aes_iv, session_clone, sink_clone, stop_audio_clone).await {
                            tracing::error!("Audio receiver error: {:?}", e);
                        }
                    });
                    udp_stop_signals.push(stop_audio);

                    // 2. Control UDP socket (PT=84)
                    let stop_control = Arc::new(AtomicBool::new(false));
                    let stop_control_clone = Arc::clone(&stop_control);
                    let control_port = config.control_port;
                    let h_control = tokio::spawn(async move {
                        if let Err(e) = run_control_receiver(control_port, stop_control_clone).await {
                            tracing::error!("Control receiver error: {:?}", e);
                        }
                    });
                    udp_stop_signals.push(stop_control);

                    // 3. Timing UDP socket (PT=82/83)
                    let stop_timing = Arc::new(AtomicBool::new(false));
                    let stop_timing_clone = Arc::clone(&stop_timing);
                    let timing_port = config.timing_port;
                    let h_timing = tokio::spawn(async move {
                        if let Err(e) = run_timing_receiver(timing_port, stop_timing_clone).await {
                            tracing::error!("Timing receiver error: {:?}", e);
                        }
                    });
                    udp_stop_signals.push(stop_timing);

                    if let Ok(mut tasks) = session.lock().await.active_tasks.lock() {
                        tasks.push(h_audio);
                        tasks.push(h_control);
                        tasks.push(h_timing);
                    }

                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         Transport: RTP/AVP/UDP;unicast;mode=record;server_port={};control_port={};timing_port={}\r\n\
                         Session: 1\r\n\
                         Audio-Jack-Status: connected; type=analog\r\n\
                         {}\
                         \r\n",
                        cseq, config.audio_port, config.control_port, config.timing_port, apple_response_header
                    );
                    stream.write_all(response.as_bytes()).await?;

                    let current_name = session.lock().await.client_name.clone().unwrap_or_else(|| device_model.clone());
                    sink.on_client_connected(&current_name, &device_model, &remote_addr.ip().to_string());
                }

                "RECORD" => {
                    let is_ready = {
                        let st = session.lock().await;
                        st.aes_key.is_some()
                    };

                    if !is_ready || udp_stop_signals.is_empty() {
                        tracing::warn!("RTSP RECORD received from {} without prior ANNOUNCE/SETUP. Responding with 454 Session Not Found.", remote_addr);
                        let response = format!(
                            "RTSP/1.0 454 Session Not Found\r\n\
                             CSeq: {}\r\n\
                             Server: AirTunes/220.68\r\n\
                             {}\
                             \r\n",
                            cseq, apple_response_header
                        );
                        stream.write_all(response.as_bytes()).await?;
                        acc_buf.drain(..total_req_len);
                        continue;
                    }

                    let (initial_client_name, cur_volume) = {
                        let mut state = session.lock().await;
                        state.is_recording = true;
                        let initial = state.client_name.clone().unwrap_or_else(|| "iPhone".to_string());
                        state.client_name = Some(initial.clone());
                        let vol = state.volume_db;
                        (initial, vol)
                    };
                    sink.on_stream_start_with_client(Some(&initial_client_name));
                    sink.on_volume_change(cur_volume);
                    tracing::info!("AirPlay audio stream started from {} (client: {:?}, vol: {:.1} dB)", remote_addr, initial_client_name, cur_volume);

                    // Launch background mDNS discovery task for friendly client name and DACP port
                    {
                        let remote_ip = remote_addr.ip();
                        let dacp_id = session.lock().await.dacp_id.clone();
                        let session_clone = Arc::clone(&session);
                        let sink_clone = Arc::clone(&sink);
                        let dm_clone = device_model.clone();
                        let conn_alive_clone = Arc::clone(&conn_alive);
                        tokio::spawn(async move {
                            let (resolved_name, resolved_port) = crate::dacp::resolve_dacp_device(remote_ip, dacp_id).await;
                            if !conn_alive_clone.load(Ordering::Relaxed) {
                                return;
                            }
                            if let Some(port) = resolved_port {
                                session_clone.lock().await.set_dacp_port(port);
                                tracing::info!("[DACP] Resolved DACP port for {}: {}", remote_ip, port);
                            }
                            if !conn_alive_clone.load(Ordering::Relaxed) {
                                return;
                            }
                            let display_name = match resolved_name {
                                Some(name) => name,
                                None => "iPhone".to_string(),
                            };
                            session_clone.lock().await.client_name = Some(display_name.clone());
                            sink_clone.on_client_name_change(&display_name);
                            sink_clone.on_client_connected(&display_name, &dm_clone, &remote_ip.to_string());
                            tracing::info!("[DACP] Resolved friendly device name: '{}'", display_name);
                        });
                    }

                    let latency_frames = sink.latency_frames();
                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         Session: 1\r\n\
                         Audio-Latency: {}\r\n\
                         Audio-Jack-Status: connected; type=analog\r\n\
                         {}\
                         \r\n",
                        cseq, latency_frames, apple_response_header
                    );
                    stream.write_all(response.as_bytes()).await?;
                }

                "SET_PARAMETER" => {
                    let content_type = headers.get("content-type").map(|s| s.to_lowercase()).unwrap_or_default();
                    if content_type.contains("application/x-dmap-tagged") {
                        let mut title = None;
                        let mut artist = None;
                        let mut album = None;
                        parse_dmap(body_bytes, &mut title, &mut artist, &mut album);
                        tracing::info!("AirPlay track metadata: Title={:?}, Artist={:?}, Album={:?}", title, artist, album);
                        sink.on_metadata_change(title.as_deref(), artist.as_deref(), album.as_deref());
                    } else if content_type.contains("image/jpeg") || content_type.contains("image/png") {
                        if !body_bytes.is_empty() {
                            tracing::info!("AirPlay album artwork received: {} bytes ({})", body_bytes.len(), content_type);
                            sink.on_artwork_change(Some(body_bytes));
                        } else {
                            sink.on_artwork_change(None);
                        }
                    } else if content_type.contains("image/none") {
                        sink.on_artwork_change(None);
                    } else {
                        let body = String::from_utf8_lossy(body_bytes);
                        tracing::info!("[RTSP] SET_PARAMETER text payload (len={}):\n{}", body_bytes.len(), body.trim());
                        for line in body.lines() {
                            let trimmed = line.trim();
                            let lower = trimmed.to_ascii_lowercase();
                            if let Some(rest) = lower.strip_prefix("volume:") {
                                let val_part = &trimmed[trimmed.len() - rest.len()..];
                                if let Ok(db) = val_part.trim().parse::<f32>() {
                                    {
                                        let mut state = session.lock().await;
                                        state.volume_db = db;
                                    }
                                    sink.on_volume_change(db);
                                    tracing::info!("[RTSP] AirPlay volume updated: {:.1} dB", db);
                                    break;
                                }
                            }
                        }
                    }

                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         {}\
                         \r\n",
                        cseq, apple_response_header
                    );
                    stream.write_all(response.as_bytes()).await?;
                }

                "GET_PARAMETER" => {
                    let state = session.lock().await.clone();
                    let body = format!("volume: {:.6}\r\n", state.volume_db);
                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         Content-Type: text/parameters\r\n\
                         Content-Length: {}\r\n\
                         {}\
                         \r\n\
                         {}",
                        cseq, body.len(), apple_response_header, body
                    );
                    stream.write_all(response.as_bytes()).await?;
                }

                "PAUSE" => {
                    sink.on_stream_paused();
                    tracing::info!("AirPlay stream paused from {}", remote_addr);
                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         {}\
                         \r\n",
                        cseq, apple_response_header
                    );
                    stream.write_all(response.as_bytes()).await?;
                }

                "FLUSH" => {
                    sink.on_flush();
                    tracing::info!("AirPlay stream flush requested");
                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         {}\
                         \r\n",
                        cseq, apple_response_header
                    );
                    stream.write_all(response.as_bytes()).await?;
                }

                "TEARDOWN" => {
                    tracing::info!("AirPlay stream teardown from {}", remote_addr);
                    conn_alive.store(false, Ordering::Relaxed);
                    for stop in udp_stop_signals.drain(..) {
                        stop.store(true, Ordering::Relaxed);
                    }
                    {
                        let mut state = session.lock().await;
                        state.is_recording = false;
                        state.abort_udp_tasks();
                    }

                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         Audio-Jack-Status: connected; type=analog\r\n\
                         {}\
                         \r\n",
                        cseq, apple_response_header
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    acc_buf.drain(..total_req_len);
                    break;
                }

                _ => {
                    tracing::debug!("RTSP unhandled method: {}", method);
                    let response = format!(
                        "RTSP/1.0 200 OK\r\n\
                         CSeq: {}\r\n\
                         Server: AirTunes/220.68\r\n\
                         Audio-Jack-Status: connected; type=analog\r\n\
                         {}\
                         \r\n",
                        cseq, apple_response_header
                    );
                    stream.write_all(response.as_bytes()).await?;
                }
            }

            acc_buf.drain(..total_req_len);
        }

        let n = tokio::select! {
            read_res = stream.read(&mut read_buf) => {
                match read_res {
                    Ok(n) => n,
                    Err(e) => {
                        tracing::debug!("RTSP read error from {}: {}", remote_addr, e);
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                if stop_signal.load(Ordering::Relaxed) {
                    tracing::info!("AirPlay receiver stopped, closing RTSP stream to {}", remote_addr);
                    break;
                }
                continue;
            }
        };

        if n == 0 {
            tracing::info!("AirPlay sender {} disconnected.", remote_addr);
            break;
        }

        acc_buf.extend_from_slice(&read_buf[..n]);
    }

    conn_alive.store(false, Ordering::Relaxed);
    for stop in udp_stop_signals {
        stop.store(true, Ordering::Relaxed);
    }
    {
        let st = session.lock().await;
        st.abort_udp_tasks();
    }
    // Always signal audio stream stopped when a connection ends
    sink.on_stream_stop();

    let remaining = active_conn_count.fetch_sub(1, Ordering::SeqCst).saturating_sub(1);
    tracing::info!("RTSP connection from {} closed (remaining active connections: {})", remote_addr, remaining);

    if remaining == 0 {
        tracing::info!("All AirPlay RTSP connections closed, resetting session to Standby");
        {
            let mut st = session.lock().await;
            st.reset();
        }
        sink.on_client_disconnected();
    }

    // Clean TCP FIN shutdown without sending bogus unsolicited responses
    let _ = stream.shutdown().await;

    Ok(())
}
