use crate::backend_manager::{BackendManager, BackendStatus};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub state: String,
    pub transport_type: String,
    pub health_check_enabled: bool,
    pub last_health_check: Option<HealthCheck>,
    pub last_access_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub success: bool,
    pub response_time_ms: Option<u64>,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub async fn get_servers(state: State<'_, AppState>) -> Result<Vec<ServerInfo>, String> {
    state
        .get_servers()
        .await
        .map_err(|e| format!("Failed to get servers: {}", e))
}

#[tauri::command]
pub async fn server_action(
    name: String,
    action: String,
    state: State<'_, AppState>,
) -> Result<ApiResponse, String> {
    state
        .server_action(&name, &action)
        .await
        .map_err(|e| format!("Failed to perform action: {}", e))
}

#[tauri::command]
pub async fn get_metrics(state: State<'_, AppState>) -> Result<Vec<Metric>, String> {
    state
        .get_metrics()
        .await
        .map_err(|e| format!("Failed to get metrics: {}", e))
}

#[tauri::command]
pub async fn get_logs(
    server: String,
    lines: Option<usize>,
    log_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state
        .get_logs(&server, lines.unwrap_or(100), log_type.as_deref())
        .await
        .map_err(|e| format!("Failed to get logs: {}", e))
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Value, String> {
    state
        .get_config()
        .await
        .map_err(|e| format!("Failed to get config: {}", e))
}

#[tauri::command]
pub async fn update_config(config: Value, state: State<'_, AppState>) -> Result<(), String> {
    state
        .update_config(config)
        .await
        .map_err(|e| format!("Failed to update config: {}", e))
}

#[tauri::command]
pub async fn stream_logs(
    server: String,
    log_type: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .stream_logs(&server, log_type.as_deref(), app)
        .await
        .map_err(|e| format!("Failed to stream logs: {}", e))
}

// Backend management commands
#[tauri::command]
pub async fn start_backend(
    backend: State<'_, Arc<RwLock<BackendManager>>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    backend
        .read()
        .await
        .start(app_handle)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_backend(backend: State<'_, Arc<RwLock<BackendManager>>>) -> Result<(), String> {
    backend.read().await.stop().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restart_backend(
    backend: State<'_, Arc<RwLock<BackendManager>>>,
    app_handle: AppHandle,
) -> Result<(), String> {
    backend
        .read()
        .await
        .restart(app_handle)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_backend_status(
    backend: State<'_, Arc<RwLock<BackendManager>>>,
) -> Result<BackendStatus, String> {
    backend
        .read()
        .await
        .get_status()
        .await
        .map_err(|e| e.to_string())
}
