use super::{Connection, Transport, TransportType};
use crate::error::{Result, TransportError};
use crate::state::{LogEntry, ServerInfo};
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

pub struct StdioTransport {
    command: String,
    args: Vec<String>,
    env: std::collections::HashMap<String, String>,
    working_dir: Option<std::path::PathBuf>,
    server_info: Option<Arc<ServerInfo>>,
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            working_dir: None,
            server_info: None,
        }
    }

    pub fn with_command(mut self, command: String, args: Vec<String>) -> Self {
        self.command = command;
        self.args = args;
        self
    }

    pub fn with_env(mut self, env: std::collections::HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn with_working_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    pub fn with_server_info(mut self, server_info: Arc<ServerInfo>) -> Self {
        self.server_info = Some(server_info);
        self
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn connect(&self) -> Result<Arc<dyn Connection>> {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Set environment variables
        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        // Set working directory if specified
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn().map_err(|e| {
            TransportError::ConnectionFailed(format!("Failed to spawn process: {e}"))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TransportError::ConnectionFailed("Failed to get stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TransportError::ConnectionFailed("Failed to get stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| TransportError::ConnectionFailed("Failed to get stderr".into()))?;

        // Start stderr reader if we have server info
        if let Some(ref server_info) = self.server_info {
            tracing::debug!("Starting stderr reader for server: {}", server_info.name);
            let server_info_clone = Arc::clone(server_info);
            tokio::spawn(async move {
                tracing::debug!("Stderr reader task started");
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!("Captured stderr from process: {}", line);

                    // Write to log file if logger is available
                    if let Some(ref logger) = server_info_clone.logger {
                        if let Err(e) = logger.write_stderr(&line).await {
                            tracing::error!("Failed to write stderr to log file: {}", e);
                        }
                    }

                    let log_entry = LogEntry {
                        timestamp: Utc::now(),
                        level: "error".to_string(),
                        message: line.clone(),
                    };
                    server_info_clone.broadcast_log(log_entry);
                }
                tracing::debug!("Stderr reader task ended");
            });
        } else {
            tracing::warn!("No server info provided, stderr will not be captured");
        }

        Ok(Arc::new(StdioConnection {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(stdout)),
            closed: Arc::new(AtomicBool::new(false)),
            server_info: self.server_info.clone(),
        }))
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Stdio
    }
}

pub struct StdioConnection {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<ChildStdout>>,
    closed: Arc<AtomicBool>,
    server_info: Option<Arc<ServerInfo>>,
}

#[async_trait]
impl Connection for StdioConnection {
    async fn send(&self, data: Bytes) -> Result<()> {
        if self.is_closed() {
            return Err(TransportError::Closed.into());
        }

        let mut stdin = self.stdin.lock().await;

        tracing::trace!(
            "Sending to stdio: {}",
            std::str::from_utf8(&data).unwrap_or("<binary>")
        );

        stdin
            .write_all(&data)
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;

        Ok(())
    }

    async fn recv(&self) -> Result<Bytes> {
        if self.is_closed() {
            return Err(TransportError::Closed.into());
        }

        let mut stdout = self.stdout.lock().await;
        let mut buffer = BytesMut::with_capacity(8192);

        // Read until we get a complete message (assuming newline-delimited JSON)
        loop {
            let n = stdout
                .read_buf(&mut buffer)
                .await
                .map_err(|e| TransportError::ReceiveFailed(e.to_string()))?;

            if n == 0 {
                self.closed.store(true, Ordering::SeqCst);
                return Err(TransportError::Closed.into());
            }

            // Check if we have a complete message
            if let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let message = buffer.split_to(pos + 1);
                let msg_bytes = message.freeze();
                let msg_str = std::str::from_utf8(&msg_bytes).unwrap_or("<binary>");
                tracing::trace!("Received from stdio: {}", msg_str);

                // Write to stdout log file if logger is available
                if let Some(ref server_info) = self.server_info {
                    if let Some(ref logger) = server_info.logger {
                        if let Err(e) = logger.write_stdout(msg_str.trim_end()).await {
                            tracing::error!("Failed to write stdout to log file: {}", e);
                        }
                    }

                    // Also broadcast non-JSON messages as logs
                    if !msg_str.trim_start().starts_with('{') {
                        let log_entry = LogEntry {
                            timestamp: Utc::now(),
                            level: "info".to_string(),
                            message: msg_str.trim().to_string(),
                        };
                        server_info.broadcast_log(log_entry);
                    }
                }

                return Ok(msg_bytes);
            }

            // Buffer is getting too large
            if buffer.len() > 1024 * 1024 {
                // 1MB limit
                return Err(TransportError::InvalidFormat.into());
            }
        }
    }

    async fn close(&self) -> Result<()> {
        self.closed.store(true, Ordering::SeqCst);

        let mut child = self.child.lock().await;

        // Try graceful shutdown first with SIGTERM on Unix
        #[cfg(unix)]
        {
            use nix::sys::signal::{self, Signal};
            use nix::unistd::Pid;

            if let Some(id) = child.id() {
                let pid = Pid::from_raw(id as i32);
                let _ = signal::kill(pid, Signal::SIGTERM);

                // Give process time to exit gracefully
                tokio::select! {
                    _ = child.wait() => {
                        tracing::debug!("Process exited gracefully");
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                        tracing::warn!("Process did not exit gracefully, forcing kill");
                        if let Err(e) = child.kill().await {
                            tracing::error!("Failed to kill child process: {}", e);
                        }
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            if let Err(e) = child.kill().await {
                tracing::warn!("Failed to kill child process: {}", e);
            }
        }

        // Wait for the process to actually exit
        let _ = child.wait().await;

        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}
