use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ProxyEvent {
    // Server events
    ServerStarted {
        name: String,
    },
    ServerStopped {
        name: String,
    },
    ServerFailed {
        name: String,
        error: String,
    },
    ServerRestarting {
        name: String,
    },

    // Health check events
    HealthCheckSuccess {
        name: String,
        response_time_ms: u64,
    },
    HealthCheckFailed {
        name: String,
        error: String,
    },

    // Connection events
    ConnectionEstablished {
        server: String,
        client_id: String,
    },
    ConnectionClosed {
        server: String,
        client_id: String,
    },
    ConnectionError {
        server: String,
        error: String,
    },

    // Log events
    LogEntry {
        server: String,
        level: String,
        message: String,
    },
    LogStreamStarted {
        server: String,
    },
    LogStreamStopped {
        server: String,
    },

    // Proxy status events
    ProxyReady {
        port: u16,
    },
    ProxyShutdown,
    ProxyError {
        error: String,
    },

    // Configuration events
    ConfigLoaded,
    ConfigUpdated,
    ConfigError {
        error: String,
    },
}

pub struct EventEmitter {
    app_handle: AppHandle,
}

impl EventEmitter {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn emit(&self, event: ProxyEvent) -> Result<(), tauri::Error> {
        // Emit to all windows
        self.app_handle.emit("proxy-event", &event)?;

        // Also emit specific event types for targeted listening
        match &event {
            ProxyEvent::ServerStarted { name } => {
                self.app_handle
                    .emit(&format!("server:{}:started", name), &event)?;
            }
            ProxyEvent::ServerStopped { name } => {
                self.app_handle
                    .emit(&format!("server:{}:stopped", name), &event)?;
            }
            ProxyEvent::ServerFailed { name, .. } => {
                self.app_handle
                    .emit(&format!("server:{}:failed", name), &event)?;
            }
            ProxyEvent::HealthCheckSuccess { name, .. } => {
                self.app_handle
                    .emit(&format!("health:{}:success", name), &event)?;
            }
            ProxyEvent::HealthCheckFailed { name, .. } => {
                self.app_handle
                    .emit(&format!("health:{}:failed", name), &event)?;
            }
            ProxyEvent::LogEntry { server, .. } => {
                self.app_handle.emit(&format!("logs:{}", server), &event)?;
            }
            _ => {}
        }

        Ok(())
    }

    pub fn emit_notification(&self, title: &str, body: &str) -> Result<(), tauri::Error> {
        self.app_handle.emit(
            "notification",
            serde_json::json!({
                "title": title,
                "body": body,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        )?;
        Ok(())
    }
}

// WebSocket replacement using Tauri events
pub struct EventWebSocket {
    emitter: EventEmitter,
}

impl EventWebSocket {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            emitter: EventEmitter::new(app_handle),
        }
    }

    pub async fn broadcast_server_update(&self, server_name: &str, state: &str) {
        let event = match state {
            "running" => ProxyEvent::ServerStarted {
                name: server_name.to_string(),
            },
            "stopped" => ProxyEvent::ServerStopped {
                name: server_name.to_string(),
            },
            "failed" => ProxyEvent::ServerFailed {
                name: server_name.to_string(),
                error: "Unknown error".to_string(),
            },
            _ => return,
        };

        let _ = self.emitter.emit(event);
    }

    pub async fn stream_logs(&self, server_name: &str, log_line: &str) {
        let _ = self.emitter.emit(ProxyEvent::LogEntry {
            server: server_name.to_string(),
            level: "INFO".to_string(),
            message: log_line.to_string(),
        });
    }
}
