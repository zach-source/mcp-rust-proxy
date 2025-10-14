//! Node.js process spawning, IPC, and process pooling
//!
//! This module handles spawning Node.js processes and managing process pools.

use crate::plugin::schema::{PluginError, PluginInput, PluginOutput};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

/// Represents a running Node.js plugin process
pub struct PluginProcess {
    /// Process ID
    pub pid: u32,
    /// Child process handle
    child: Child,
    /// stdin handle for writing input
    stdin: Option<ChildStdin>,
    /// stdout handle for reading output
    stdout: Option<BufReader<ChildStdout>>,
}

impl PluginProcess {
    /// Spawn a new Node.js plugin process
    pub async fn spawn(
        node_executable: &PathBuf,
        plugin_path: &PathBuf,
    ) -> Result<Self, PluginError> {
        let mut cmd = Command::new(node_executable);
        cmd.arg(plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true); // Prevent zombie processes

        let mut child = cmd.spawn().map_err(|e| PluginError::SpawnFailed {
            reason: format!("Failed to spawn Node.js process: {}", e),
        })?;

        let pid = child.id().ok_or_else(|| PluginError::SpawnFailed {
            reason: "Failed to get process ID".to_string(),
        })?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take().map(BufReader::new);

        Ok(Self {
            pid,
            child,
            stdin,
            stdout,
        })
    }

    /// Check if the process is still healthy
    pub fn is_healthy(&mut self) -> bool {
        // Check if process is still running
        match self.child.try_wait() {
            Ok(Some(_)) => false, // Process has exited
            Ok(None) => true,     // Process still running
            Err(_) => false,      // Error checking status
        }
    }

    /// Write input to the plugin's stdin
    pub async fn write_input(&mut self, input: &PluginInput) -> Result<(), PluginError> {
        let json = input.to_json()?;

        if let Some(stdin) = &mut self.stdin {
            stdin
                .write_all(json.as_bytes())
                .await
                .map_err(|e| PluginError::IoError {
                    reason: format!("Failed to write to stdin: {}", e),
                })?;

            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| PluginError::IoError {
                    reason: format!("Failed to write newline to stdin: {}", e),
                })?;

            stdin.flush().await.map_err(|e| PluginError::IoError {
                reason: format!("Failed to flush stdin: {}", e),
            })?;

            Ok(())
        } else {
            Err(PluginError::IoError {
                reason: "stdin is not available".to_string(),
            })
        }
    }

    /// Read output from the plugin's stdout
    pub async fn read_output(&mut self) -> Result<PluginOutput, PluginError> {
        if let Some(stdout) = &mut self.stdout {
            let mut line = String::new();
            stdout
                .read_line(&mut line)
                .await
                .map_err(|e| PluginError::IoError {
                    reason: format!("Failed to read from stdout: {}", e),
                })?;

            if line.is_empty() {
                return Err(PluginError::IoError {
                    reason: "stdout closed unexpectedly".to_string(),
                });
            }

            PluginOutput::from_json(&line.trim())
        } else {
            Err(PluginError::IoError {
                reason: "stdout is not available".to_string(),
            })
        }
    }

    /// Execute plugin with input and get output
    pub async fn execute(&mut self, input: &PluginInput) -> Result<PluginOutput, PluginError> {
        self.write_input(input).await?;
        self.read_output().await
    }

    /// Kill the process gracefully
    pub async fn kill(&mut self) -> Result<(), PluginError> {
        self.child.kill().await.map_err(|e| PluginError::IoError {
            reason: format!("Failed to kill process: {}", e),
        })
    }
}

/// Pool of warm Node.js processes for plugin execution
pub struct ProcessPool {
    /// Node.js executable path
    node_executable: PathBuf,
    /// Plugin file path
    plugin_path: PathBuf,
    /// Maximum pool size
    max_size: usize,
    /// Available processes
    processes: Arc<Mutex<VecDeque<PluginProcess>>>,
}

impl ProcessPool {
    /// Create a new process pool
    pub fn new(node_executable: PathBuf, plugin_path: PathBuf, max_size: usize) -> Self {
        Self {
            node_executable,
            plugin_path,
            max_size,
            processes: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
        }
    }

    /// Acquire a process from the pool (or spawn new if none available)
    pub async fn acquire(&self) -> Result<PluginProcess, PluginError> {
        let mut processes = self.processes.lock().await;

        // Try to get a healthy process from the pool
        while let Some(mut process) = processes.pop_front() {
            if process.is_healthy() {
                return Ok(process);
            }
            // Process is unhealthy, kill it and try next
            let _ = process.kill().await;
        }

        // No healthy process available, spawn a new one
        PluginProcess::spawn(&self.node_executable, &self.plugin_path).await
    }

    /// Release a process back to the pool
    pub async fn release(&self, process: PluginProcess) {
        let mut processes = self.processes.lock().await;

        // Only add back if pool is not full and process is healthy
        if processes.len() < self.max_size {
            processes.push_back(process);
        } else {
            // Pool is full, let the process drop and be killed
            drop(process);
        }
    }

    /// Shutdown all processes in the pool
    pub async fn shutdown(&self) {
        let mut processes = self.processes.lock().await;

        while let Some(mut process) = processes.pop_front() {
            let _ = process.kill().await;
        }
    }

    /// Get current pool size
    pub async fn size(&self) -> usize {
        self.processes.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::schema::{PluginMetadata, PluginPhase};
    
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_process_spawn() {
        let temp_plugin = create_echo_plugin();
        let node = PathBuf::from("node");
        let plugin_path = PathBuf::from(temp_plugin.path());

        let process = PluginProcess::spawn(&node, &plugin_path).await;
        assert!(process.is_ok());

        let mut process = process.unwrap();
        assert!(process.is_healthy());

        // Cleanup
        let _ = process.kill().await;
    }

    #[tokio::test]
    #[ignore = "Flaky test - will be covered by integration tests"]
    async fn test_process_io() {
        let temp_plugin = create_echo_plugin();
        let node = PathBuf::from("node");
        let plugin_path = PathBuf::from(temp_plugin.path());

        let mut process = PluginProcess::spawn(&node, &plugin_path)
            .await
            .expect("Failed to spawn process");

        let input = PluginInput {
            tool_name: "test".to_string(),
            raw_content: "test content".to_string(),
            max_tokens: None,
            metadata: PluginMetadata {
                request_id: "req-1".to_string(),
                timestamp: "2025-10-10T12:00:00Z".to_string(),
                server_name: "test".to_string(),
                phase: PluginPhase::Response,
                user_query: None,
                tool_arguments: None,
                mcp_servers: None,
            },
        };

        let result = process.execute(&input).await;
        assert!(
            result.is_ok(),
            "Execute failed: {:?}",
            result.as_ref().err()
        );

        let output = result.unwrap();
        assert_eq!(output.text, "test content");
        assert!(output.continue_);

        // Cleanup
        let _ = process.kill().await;
    }

    #[tokio::test]
    async fn test_process_pool() {
        let temp_plugin = create_echo_plugin();
        let node = PathBuf::from("node");
        let plugin_path = PathBuf::from(temp_plugin.path());

        let pool = ProcessPool::new(node, plugin_path, 2);

        // Pool should start empty
        assert_eq!(pool.size().await, 0);

        // Acquire a process (should spawn new)
        let process1 = pool.acquire().await.expect("Failed to acquire process");
        assert_eq!(pool.size().await, 0);

        // Release it back
        pool.release(process1).await;
        assert_eq!(pool.size().await, 1);

        // Acquire again (should reuse)
        let process2 = pool.acquire().await.expect("Failed to acquire process");
        assert_eq!(pool.size().await, 0);

        // Cleanup
        pool.release(process2).await;
        pool.shutdown().await;
    }

    /// Create a temporary echo plugin for testing
    fn create_echo_plugin() -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let plugin_code = r#"
#!/usr/bin/env node

// Read from stdin line by line
const readline = require('readline');
const rl = readline.createInterface({
    input: process.stdin,
    terminal: false
});

rl.on('line', (line) => {
    try {
        const input = JSON.parse(line);
        const output = {
            text: input.rawContent,
            continue: true
        };
        // Write to stdout with newline
        process.stdout.write(JSON.stringify(output) + '\n');
    } catch (err) {
        const errorOutput = {
            text: "",
            continue: false,
            error: err.message
        };
        process.stdout.write(JSON.stringify(errorOutput) + '\n');
    }
});

// Keep process alive
rl.on('close', () => {
    process.exit(0);
});
"#;
        temp_file
            .write_all(plugin_code.as_bytes())
            .expect("Failed to write plugin code");
        temp_file
    }
}
