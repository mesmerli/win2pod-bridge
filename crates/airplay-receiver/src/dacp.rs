//! DACP (Digital Audio Control Protocol) Remote Control Client and mDNS Discovery.

use std::net::{IpAddr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Commands supported by the DACP remote control protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DacpCommand {
    PlayPause,
    Next,
    Prev,
    Play,
    Pause,
}

impl DacpCommand {
    pub fn to_path(&self) -> String {
        match self {
            Self::PlayPause => "playpause".to_string(),
            Self::Next => "nextitem".to_string(),
            Self::Prev => "previtem".to_string(),
            Self::Play => "play".to_string(),
            Self::Pause => "pause".to_string(),
        }
    }
}

/// Send a fire-and-forget DACP command to an Apple client with a strict 500ms timeout.
pub async fn send_dacp_command(
    ip: IpAddr,
    port: u16,
    active_remote: &str,
    cmd: &DacpCommand,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::new(ip, port);
    let mut stream = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        tokio::net::TcpStream::connect(addr),
    )
    .await??;

    let path = cmd.to_path();
    let request = format!(
        "GET /ctrl-int/1/{} HTTP/1.1\r\n\
         Host: {}:{}\r\n\
         Active-Remote: {}\r\n\
         Connection: close\r\n\
         \r\n",
        path, ip, port, active_remote
    );

    tokio::time::timeout(
        std::time::Duration::from_millis(500),
        stream.write_all(request.as_bytes()),
    )
    .await??;

    let mut resp_buf = [0u8; 256];
    if let Ok(Ok(n)) = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        stream.read(&mut resp_buf),
    )
    .await {
        if n > 0 {
            let resp_str = String::from_utf8_lossy(&resp_buf[..n]);
            let status_line = resp_str.lines().next().unwrap_or("").trim();
            tracing::info!("[DACP] Sent '{}' to {}:{} -> Phone reply: {}", path, ip, port, status_line);
            return Ok(());
        }
    }

    tracing::info!("[DACP] Sent '{}' to {}:{}", path, ip, port);
    Ok(())
}

/// Query mDNS for reverse PTR (`<ip>.in-addr.arpa`) to get the device hostname (e.g. `LeoBaiPhone`).
/// This is ultra-fast (~5ms) and works across all apps (YouTube, Safari, etc.) via iOS kernel mDNS.
pub async fn resolve_mdns_hostname(ip: IpAddr) -> Option<String> {
    let v4 = match ip {
        IpAddr::V4(v4) => v4,
        IpAddr::V6(_) => return None,
    };
    let octets = v4.octets();
    let parts = [
        octets[3].to_string(),
        octets[2].to_string(),
        octets[1].to_string(),
        octets[0].to_string(),
        "in-addr".to_string(),
        "arpa".to_string(),
    ];
    let mut qname = Vec::new();
    for p in &parts {
        qname.push(p.len() as u8);
        qname.extend_from_slice(p.as_bytes());
    }
    qname.push(0);

    let mut packet = vec![0u8; 12];
    packet[5] = 1; // QDCOUNT = 1
    packet.extend_from_slice(&qname);
    packet.extend_from_slice(&[0x00, 0x0C]); // QTYPE = PTR (12)
    packet.extend_from_slice(&[0x00, 0x01]); // QCLASS = IN (1)

    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await.ok()?;
    let target: SocketAddr = "224.0.0.251:5353".parse().ok()?;
    socket.send_to(&packet, target).await.ok()?;

    let mut buf = [0u8; 1024];
    let timeout = std::time::Duration::from_millis(500);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        let remaining = timeout.saturating_sub(start.elapsed());
        let (len, from) = tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await.ok()?.ok()?;
        if from.ip() == ip {
            if let Some(name) = extract_ptr_target(&buf[..len]) {
                let clean = name.trim_end_matches(".local").trim_end_matches('.');
                if !clean.is_empty() {
                    return Some(clean.to_string());
                }
            }
        }
    }
    None
}

/// Helper to extract hostname from DNS PTR response packet.
fn extract_ptr_target(buf: &[u8]) -> Option<String> {
    if buf.len() < 12 {
        return None;
    }
    if let Some(pos) = buf.windows(7).position(|w| w == b"\x05local\0") {
        for len in 1..=64 {
            if pos >= len + 1 {
                let label_len = buf[pos - 1 - len];
                if label_len as usize == len {
                    if let Ok(name) = std::str::from_utf8(&buf[pos - len..pos]) {
                        return Some(name.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Query mDNS for `_dacp._tcp.local.` to discover friendly device name (`CtlN`) and DACP port.
pub async fn resolve_dacp_service(
    remote_ip: IpAddr,
    dacp_id: Option<String>,
) -> (Option<String>, Option<u16>) {
    let timeout = std::time::Duration::from_millis(1000);
    let start = std::time::Instant::now();

    let daemon = match mdns_sd::ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("[DACP] Failed to create mDNS daemon: {}", e);
            return (None, None);
        }
    };

    let receiver = match daemon.browse("_dacp._tcp.local.") {
        Ok(rx) => rx,
        Err(e) => {
            tracing::warn!("[DACP] Failed to browse _dacp._tcp.local.: {}", e);
            return (None, None);
        }
    };

    let mut found_name: Option<String> = None;
    let mut found_port: Option<u16> = None;

    while start.elapsed() < timeout {
        let remaining = timeout.saturating_sub(start.elapsed());
        let res = tokio::task::spawn_blocking({
            let receiver = receiver.clone();
            move || receiver.recv_timeout(remaining)
        })
        .await;

        match res {
            Ok(Ok(event)) => {
                if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
                    let fullname = info.get_fullname();
                    let addresses = info.get_addresses();
                    let matches_ip = addresses.iter().any(|a| *a == remote_ip);
                    let db_id = info.get_property_val_str("DbId");
                    let matches_dbid = match (&dacp_id, db_id) {
                        (Some(req_id), Some(adv_id)) => req_id.eq_ignore_ascii_case(adv_id),
                        _ => false,
                    };
                    let matches_fullname = if let Some(ref req_id) = dacp_id {
                        fullname.to_ascii_lowercase().contains(&req_id.to_ascii_lowercase())
                    } else {
                        false
                    };

                    if matches_ip || matches_dbid || matches_fullname {
                        if let Some(ctln) = info.get_property_val_str("CtlN") {
                            let trimmed = ctln.trim();
                            if !trimmed.is_empty() {
                                found_name = Some(trimmed.to_string());
                            }
                        }
                        let port = info.get_port();
                        found_port = Some(port);
                        tracing::info!("[DACP] Discovered iPhone DACP dynamic service '{}' on {}:{}", fullname, remote_ip, port);
                        if found_name.is_some() {
                            break;
                        }
                    }
                }
            }
            _ => break,
        }
    }

    let _ = daemon.shutdown();
    (found_name, found_port)
}

/// Combined resolver: runs fast mDNS reverse PTR resolution and `_dacp._tcp` discovery.
pub async fn resolve_dacp_device(
    remote_ip: IpAddr,
    dacp_id: Option<String>,
) -> (Option<String>, Option<u16>) {
    // 1. First attempt ultra-fast mDNS reverse PTR query (~5ms)
    let ptr_name = resolve_mdns_hostname(remote_ip).await;

    // 2. Also browse _dacp._tcp for port and CtlN (if DACP-enabled app like Apple Music is running)
    let (dacp_name, dacp_port) = resolve_dacp_service(remote_ip, dacp_id).await;

    let final_name = dacp_name.or(ptr_name);
    (final_name, dacp_port)
}
