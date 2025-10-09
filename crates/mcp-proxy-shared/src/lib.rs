//! MCP Proxy Shared Types
//!
//! This crate contains shared types and utilities used across
//! the MCP Proxy ecosystem.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Server state representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ServerState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
    Unknown,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub state: ServerState,
    pub restart_count: u32,
    pub health_check_enabled: bool,
    pub last_health_check: Option<HealthCheck>,
    pub last_access_time: Option<DateTime<Utc>>,
    pub disabled: bool,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub healthy: bool,
    pub timestamp: DateTime<Utc>,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
}

/// API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub total_requests: u64,
    pub active_connections: u32,
    pub error_count: u64,
    pub average_response_time_ms: f64,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub server: String,
}

/// Log level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for ServerState {
    fn default() -> Self {
        ServerState::Stopped
    }
}

impl ServerInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            state: ServerState::default(),
            restart_count: 0,
            health_check_enabled: false,
            last_health_check: None,
            last_access_time: None,
            disabled: false,
        }
    }
}
