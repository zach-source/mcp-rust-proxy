use anyhow::Result;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use crate::commands::{ApiResponse, Metric, ServerInfo};

#[derive(Clone)]
pub struct AppState {
    servers: Arc<DashMap<String, ServerInfo>>,
    config: Arc<tokio::sync::RwLock<Value>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(DashMap::new()),
            config: Arc::new(tokio::sync::RwLock::new(Value::Null)),
        }
    }

    pub async fn get_servers(&self) -> Result<Vec<ServerInfo>> {
        Ok(self
            .servers
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    pub async fn server_action(&self, name: &str, action: &str) -> Result<ApiResponse> {
        // TODO: Implement server actions (start, stop, restart)
        Ok(ApiResponse {
            success: true,
            message: format!("Action '{}' performed on server '{}'", action, name),
        })
    }

    pub async fn get_metrics(&self) -> Result<Vec<Metric>> {
        // TODO: Implement metrics collection
        Ok(vec![
            Metric {
                name: "total_servers".to_string(),
                value: self.servers.len() as f64,
                unit: "count".to_string(),
            },
            Metric {
                name: "active_connections".to_string(),
                value: 0.0,
                unit: "count".to_string(),
            },
        ])
    }

    pub async fn get_logs(
        &self,
        server: &str,
        lines: usize,
        log_type: Option<&str>,
    ) -> Result<Vec<String>> {
        // TODO: Implement log fetching
        Ok(vec![format!(
            "Last {} lines of {} logs for server {}",
            lines,
            log_type.unwrap_or("all"),
            server
        )])
    }

    pub async fn get_config(&self) -> Result<Value> {
        Ok(self.config.read().await.clone())
    }

    pub async fn update_config(&self, config: Value) -> Result<()> {
        *self.config.write().await = config;
        Ok(())
    }

    pub async fn stream_logs(
        &self,
        server: &str,
        _log_type: Option<&str>,
        app: AppHandle,
    ) -> Result<()> {
        // TODO: Implement log streaming via Tauri events
        let event_name = format!("logs:{}", server);
        app.emit(&event_name, format!("Log streaming for {} started", server))?;
        Ok(())
    }
}
