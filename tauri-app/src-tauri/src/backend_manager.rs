use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatus {
    pub running: bool,
    pub port: u16,
    pub api_port: u16,
    pub pid: Option<u32>,
    pub uptime: Option<u64>,
}

pub struct BackendManager {
    process: Arc<RwLock<Option<CommandChild>>>,
    config_path: PathBuf,
    port: u16,
    start_time: Arc<RwLock<Option<std::time::Instant>>>,
}

impl BackendManager {
    pub fn new(config_path: PathBuf, port: u16) -> Self {
        Self {
            process: Arc::new(RwLock::new(None)),
            config_path,
            port,
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start(&self, app_handle: AppHandle) -> Result<()> {
        // Check if already running
        if self.is_running().await {
            return Err(anyhow!("Backend is already running"));
        }

        let shell = app_handle.shell();

        // Get the sidecar binary
        let sidecar_command = shell.sidecar("mcp-proxy-server")?;

        // Prepare arguments
        let args = vec![
            "--config".to_string(),
            self.config_path.to_string_lossy().to_string(),
            "--port".to_string(),
            self.port.to_string(),
            "--api-port".to_string(),
            (self.port + 1).to_string(),
        ];

        // Spawn the backend process
        let (mut rx, child) = sidecar_command.args(args).spawn()?;

        // Store the process handle
        let mut process = self.process.write().await;
        *process = Some(child);

        // Record start time
        let mut start_time = self.start_time.write().await;
        *start_time = Some(std::time::Instant::now());

        // Spawn a task to handle process events
        let app_handle_clone = app_handle.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        app_handle_clone.emit("backend-stdout", line).ok();
                    }
                    CommandEvent::Stderr(line) => {
                        app_handle_clone.emit("backend-stderr", line).ok();
                    }
                    CommandEvent::Error(error) => {
                        app_handle_clone.emit("backend-error", error).ok();
                    }
                    CommandEvent::Terminated(payload) => {
                        app_handle_clone.emit("backend-terminated", payload).ok();
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Wait for backend to be ready
        self.wait_for_ready().await?;

        // Emit success event
        app_handle
            .emit("backend-started", self.get_status().await?)
            .ok();

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut process = self.process.write().await;
        if let Some(mut child) = process.take() {
            child.kill()?;

            // Clear start time
            let mut start_time = self.start_time.write().await;
            *start_time = None;
        } else {
            return Err(anyhow!("Backend is not running"));
        }
        Ok(())
    }

    pub async fn restart(&self, app_handle: AppHandle) -> Result<()> {
        // Stop if running
        if self.is_running().await {
            self.stop().await.ok();
            // Wait a bit for clean shutdown
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Start again
        self.start(app_handle).await
    }

    pub async fn get_status(&self) -> Result<BackendStatus> {
        let process = self.process.read().await;
        let running = process.is_some();

        let pid = if let Some(ref child) = *process {
            child.pid()
        } else {
            None
        };

        let uptime = if running {
            let start_time = self.start_time.read().await;
            start_time.as_ref().map(|t| t.elapsed().as_secs())
        } else {
            None
        };

        Ok(BackendStatus {
            running,
            port: self.port,
            api_port: self.port + 1,
            pid,
            uptime,
        })
    }

    pub async fn is_running(&self) -> bool {
        let process = self.process.read().await;
        process.is_some()
    }

    async fn wait_for_ready(&self) -> Result<()> {
        let max_retries = 30;
        let retry_delay = Duration::from_millis(100);

        for i in 0..max_retries {
            if self.health_check().await.is_ok() {
                return Ok(());
            }

            // Check if process is still alive
            if !self.is_running().await {
                return Err(anyhow!("Backend process terminated unexpectedly"));
            }

            tokio::time::sleep(retry_delay).await;
        }

        Err(anyhow!(
            "Backend failed to become ready after {} attempts",
            max_retries
        ))
    }

    async fn health_check(&self) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("http://localhost:{}/health", self.port + 1);

        let response = client
            .get(&url)
            .timeout(Duration::from_secs(1))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Health check returned status {}",
                response.status()
            ))
        }
    }
}

// Backend monitor for automatic recovery
pub struct BackendMonitor {
    manager: Arc<BackendManager>,
    app_handle: AppHandle,
    running: Arc<RwLock<bool>>,
}

impl BackendMonitor {
    pub fn new(manager: Arc<BackendManager>, app_handle: AppHandle) -> Self {
        Self {
            manager,
            app_handle,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start_monitoring(self: Arc<Self>) {
        let mut running = self.running.write().await;
        if *running {
            return; // Already monitoring
        }
        *running = true;
        drop(running);

        let self_clone = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                // Check if we should continue monitoring
                let running = self_clone.running.read().await;
                if !*running {
                    break;
                }
                drop(running);

                tokio::time::sleep(Duration::from_secs(5)).await;

                // Check backend status
                if let Ok(status) = self_clone.manager.get_status().await {
                    if !status.running {
                        // Attempt to restart
                        self_clone.app_handle.emit("backend-restarting", ()).ok();

                        if let Err(e) = self_clone
                            .manager
                            .restart(self_clone.app_handle.clone())
                            .await
                        {
                            self_clone
                                .app_handle
                                .emit("backend-restart-failed", format!("{}", e))
                                .ok();
                        } else {
                            self_clone.app_handle.emit("backend-restarted", ()).ok();
                        }
                    }
                }
            }
        });
    }

    pub async fn stop_monitoring(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }
}
