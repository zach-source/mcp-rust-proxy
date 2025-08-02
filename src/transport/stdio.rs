use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::error::{TransportError, Result};
use super::{Connection, Transport, TransportType};

pub struct StdioTransport {
    command: String,
    args: Vec<String>,
    env: std::collections::HashMap<String, String>,
    working_dir: Option<std::path::PathBuf>,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            working_dir: None,
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

        let mut child = cmd.spawn()
            .map_err(|e| TransportError::ConnectionFailed(format!("Failed to spawn process: {}", e)))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| TransportError::ConnectionFailed("Failed to get stdin".into()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| TransportError::ConnectionFailed("Failed to get stdout".into()))?;

        Ok(Arc::new(StdioConnection {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(stdout)),
            closed: Arc::new(AtomicBool::new(false)),
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
}

#[async_trait]
impl Connection for StdioConnection {
    async fn send(&self, data: Bytes) -> Result<()> {
        if self.is_closed() {
            return Err(TransportError::Closed.into());
        }

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(&data)
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        stdin.flush()
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
            let n = stdout.read_buf(&mut buffer)
                .await
                .map_err(|e| TransportError::ReceiveFailed(e.to_string()))?;
            
            if n == 0 {
                self.closed.store(true, Ordering::SeqCst);
                return Err(TransportError::Closed.into());
            }

            // Check if we have a complete message
            if let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let message = buffer.split_to(pos + 1);
                return Ok(message.freeze());
            }

            // Buffer is getting too large
            if buffer.len() > 1024 * 1024 { // 1MB limit
                return Err(TransportError::InvalidFormat.into());
            }
        }
    }

    async fn close(&self) -> Result<()> {
        self.closed.store(true, Ordering::SeqCst);
        
        let mut child = self.child.lock().await;
        
        // Try graceful shutdown first
        if let Err(e) = child.kill().await {
            tracing::warn!("Failed to kill child process: {}", e);
        }

        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}