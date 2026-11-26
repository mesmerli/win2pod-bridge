//! Transport helpers for Windows Named Pipe communication.

use std::time::Duration;
use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions};
use crate::protocol::{IpcMessage, DEFAULT_PIPE_NAME};

pub struct PipeTransport<S> {
    reader: BufReader<tokio::io::ReadHalf<S>>,
    writer: tokio::io::WriteHalf<S>,
}

impl<S: tokio::io::AsyncRead + tokio::io::AsyncWrite> PipeTransport<S> {
    pub fn new(stream: S) -> Self {
        let (read_half, write_half) = tokio::io::split(stream);
        Self {
            reader: BufReader::new(read_half),
            writer: write_half,
        }
    }

    /// Read next message delimited by newline.
    pub async fn read_message(&mut self) -> Result<Option<IpcMessage>> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await
            .context("Failed to read from named pipe")?;
        if bytes_read == 0 {
            return Ok(None);
        }
        let msg: IpcMessage = serde_json::from_str(line.trim_end())
            .context("Failed to parse IPC message JSON")?;
        Ok(Some(msg))
    }

    /// Write message formatted as JSON line.
    pub async fn write_message(&mut self, msg: &IpcMessage) -> Result<()> {
        let mut json = serde_json::to_string(msg)
            .context("Failed to serialize IPC message")?;
        json.push('\n');
        self.writer.write_all(json.as_bytes()).await
            .context("Failed to write to named pipe")?;
        self.writer.flush().await
            .context("Failed to flush named pipe")?;
        Ok(())
    }
}

/// Create a new Named Pipe server instance.
pub fn create_pipe_server(pipe_name: Option<&str>, is_first: bool) -> Result<NamedPipeServer> {
    let name = pipe_name.unwrap_or(DEFAULT_PIPE_NAME);
    let mut opts = ServerOptions::new();
    opts.first_pipe_instance(is_first)
        .max_instances(16);
    let server = opts.create(name)
        .with_context(|| format!("Failed to create named pipe server on {name} (is_first={is_first})"))?;
    Ok(server)
}

/// Connect to a Named Pipe server as a client with retry.
pub async fn connect_pipe_client(pipe_name: Option<&str>, max_retries: usize, retry_interval: Duration) -> Result<NamedPipeClient> {
    let name = pipe_name.unwrap_or(DEFAULT_PIPE_NAME);
    let mut retries = 0;
    loop {
        match ClientOptions::new().open(name) {
            Ok(client) => return Ok(client),
            Err(e) => {
                retries += 1;
                if retries >= max_retries {
                    return Err(e).with_context(|| format!("Failed to connect to named pipe {name} after {max_retries} attempts"));
                }
                tokio::time::sleep(retry_interval).await;
            }
        }
    }
}
