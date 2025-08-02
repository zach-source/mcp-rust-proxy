use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub servers: HashMap<String, ServerConfig>,
    pub proxy: ProxyConfig,
    pub web_ui: WebUIConfig,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub transport: TransportConfig,
    #[serde(default = "default_restart_on_failure")]
    pub restart_on_failure: bool,
    #[serde(default)]
    pub working_directory: Option<PathBuf>,
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    #[serde(default = "default_restart_delay")]
    pub restart_delay_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TransportConfig {
    Stdio,
    #[serde(rename_all = "camelCase")]
    HttpSse {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default = "default_http_timeout")]
        timeout_ms: u64,
    },
    #[serde(rename_all = "camelCase")]
    WebSocket {
        url: String,
        #[serde(default)]
        protocols: Vec<String>,
        #[serde(default = "default_ws_reconnect")]
        auto_reconnect: bool,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    #[serde(default = "default_proxy_port")]
    pub port: u16,
    #[serde(default = "default_proxy_host")]
    pub host: String,
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: usize,
    #[serde(default = "default_request_timeout")]
    pub request_timeout_ms: u64,
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebUIConfig {
    #[serde(default = "default_web_ui_enabled")]
    pub enabled: bool,
    #[serde(default = "default_web_ui_port")]
    pub port: u16,
    #[serde(default = "default_web_ui_host")]
    pub host: String,
    #[serde(default)]
    pub static_dir: Option<PathBuf>,
    #[serde(default = "default_api_key")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckConfig {
    #[serde(default = "default_health_check_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_health_check_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_health_check_enabled")]
    pub enabled: bool,
}

// Default value functions
fn default_restart_on_failure() -> bool {
    true
}

fn default_max_restarts() -> u32 {
    3
}

fn default_restart_delay() -> u64 {
    5000
}

fn default_proxy_port() -> u16 {
    3000
}

fn default_proxy_host() -> String {
    "0.0.0.0".to_string()
}

fn default_connection_pool_size() -> usize {
    10
}

fn default_request_timeout() -> u64 {
    30000
}

fn default_max_concurrent_requests() -> usize {
    100
}

fn default_web_ui_enabled() -> bool {
    true
}

fn default_web_ui_port() -> u16 {
    3001
}

fn default_web_ui_host() -> String {
    "0.0.0.0".to_string()
}

fn default_api_key() -> Option<String> {
    None
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_health_check_timeout() -> u64 {
    5
}

fn default_health_check_enabled() -> bool {
    true
}

fn default_http_timeout() -> u64 {
    30000
}

fn default_ws_reconnect() -> bool {
    true
}

impl Config {
    pub fn health_check_interval(&self) -> Duration {
        Duration::from_secs(self.health_check.interval_seconds)
    }

    pub fn health_check_timeout(&self) -> Duration {
        Duration::from_secs(self.health_check.timeout_seconds)
    }

    pub fn request_timeout(&self) -> Duration {
        Duration::from_millis(self.proxy.request_timeout_ms)
    }
}