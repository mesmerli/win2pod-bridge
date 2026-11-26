//! # win2pod-engine
//!
//! Headless background daemon for Win2Pod Bridge.
//! Encapsulates AirPlay 2 sender, AirPlay 1 receiver, and Apple Continuity BLE
//! into an isolated process under GPL-3.0.

mod engine;
mod airplay_sender;

use std::sync::Arc;
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use win2pod_ipc::protocol::{IpcMessage, DEFAULT_PIPE_NAME};
use win2pod_ipc::transport::{create_pipe_server, PipeTransport};
use crate::engine::Engine;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "info,win2pod_engine=debug".into()),
            ),
        )
        .init();

    info!("Starting win2pod-engine Sidecar (GPL-3.0-or-later)...");

    let args: Vec<String> = std::env::args().collect();
    let mut pipe_name = DEFAULT_PIPE_NAME.to_string();

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--pipe-name" && i + 1 < args.len() {
            pipe_name = args[i + 1].clone();
            i += 2;
        } else {
            i += 1;
        }
    }

    info!("Listening on named pipe: {}", pipe_name);

    let (event_tx, mut event_rx) = mpsc::channel(128);
    let engine = Arc::new(Engine::new(event_tx));

    let mut is_first = true;
    loop {
        let server = match create_pipe_server(Some(&pipe_name), is_first) {
            Ok(s) => {
                is_first = false;
                s
            }
            Err(e) => {
                error!("Failed to create pipe server: {e:#}, retrying in 2s...");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
        };

        info!("Waiting for GUI client connection...");
        if let Err(e) = server.connect().await {
            warn!("Client connect failed: {e:#}");
            continue;
        }
        info!("GUI client connected!");

        let mut transport = PipeTransport::new(server);
        let engine_clone = engine.clone();

        // Handle client connection session
        loop {
            tokio::select! {
                read_res = transport.read_message() => {
                    match read_res {
                        Ok(Some(IpcMessage::Request(req))) => {
                            let resp = match engine_clone.handle_request(req).await {
                                Ok(r) => r,
                                Err(e) => win2pod_ipc::protocol::IpcResponse::Error(e.to_string()),
                            };
                            if let Err(e) = transport.write_message(&IpcMessage::Response(resp)).await {
                                warn!("Failed to write IPC response: {e:#}");
                                break;
                            }
                        }
                        Ok(Some(msg)) => {
                            warn!("Unexpected IPC message from client: {:?}", msg);
                        }
                        Ok(None) => {
                            info!("GUI client disconnected");
                            break;
                        }
                        Err(e) => {
                            warn!("Error reading IPC message: {e:#}");
                            break;
                        }
                    }
                }
                Some(event) = event_rx.recv() => {
                    if let Err(e) = transport.write_message(&IpcMessage::Event(event)).await {
                        warn!("Failed to write IPC event: {e:#}");
                        break;
                    }
                }
            }
        }

        info!("Session ended, cleaning up active stream...");
        let _ = engine.handle_request(win2pod_ipc::protocol::IpcRequest::Disconnect).await;
    }
}
