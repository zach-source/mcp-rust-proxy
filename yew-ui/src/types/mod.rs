use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerState {
    Running,
    Stopped,
    Failed,
    Starting,
    Stopping,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthCheck {
    pub success: bool,
    pub response_time_ms: Option<u64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub state: ServerState,
    pub restart_count: u32,
    pub health_check_enabled: bool,
    pub last_health_check: Option<HealthCheck>,
    pub last_access_time: Option<DateTime<Utc>>,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub total_servers: u32,
    pub running_servers: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metrics: Vec<MetricValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricValue {
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub metrics: Vec<Metric>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServersResponse {
    pub servers: Vec<Server>,
}

// WebSocket messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    Initial { data: WsData },
    Update { data: WsData },
    Log { data: LogData },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WsData {
    pub servers: Vec<Server>,
    pub stats: Option<Stats>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogData {
    pub server: String,
    pub timestamp: Option<String>,
    pub message: Option<String>,
    pub level: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsCommand {
    SubscribeLogs { server: String },
    UnsubscribeLogs { server: String },
}

// API responses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiResponse {
    pub message: Option<String>,
    pub error: Option<String>,
}
