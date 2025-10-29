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
    #[serde(default)]
    pub context_tracing: ContextTracingConfig,
    #[serde(default)]
    pub plugins: Option<crate::plugin::PluginConfig>,
    #[serde(default)]
    pub claude_proxy: Option<crate::claude_proxy::ClaudeProxyConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub transport: TransportConfig,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_restart_on_failure")]
    pub restart_on_failure: bool,
    #[serde(default)]
    pub working_directory: Option<PathBuf>,
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    #[serde(default = "default_restart_delay")]
    pub restart_delay_ms: u64,
    #[serde(default)]
    pub initialization_delay_ms: Option<u64>,
    #[serde(default)]
    pub health_check: Option<ServerHealthCheckConfig>,
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
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_retry_interval")]
    pub retry_interval_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerHealthCheckConfig {
    #[serde(default = "default_server_health_check_enabled")]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub interval_seconds: Option<u64>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub max_attempts: Option<u32>,
    #[serde(default)]
    pub retry_interval_seconds: Option<u64>,
}

// Default value functions
fn default_enabled() -> bool {
    true
}

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

fn default_max_attempts() -> u32 {
    3
}

fn default_retry_interval() -> u64 {
    10
}

fn default_server_health_check_enabled() -> Option<bool> {
    None
}

fn default_http_timeout() -> u64 {
    30000
}

fn default_ws_reconnect() -> bool {
    true
}

// Context tracing defaults
fn default_context_tracing_enabled() -> bool {
    true
}

fn default_sqlite_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".mcp-proxy")
        .join("context-tracing.db")
}

fn default_cache_size() -> usize {
    10_000
}

fn default_cache_ttl_seconds() -> i64 {
    7 * 24 * 60 * 60 // 7 days
}

fn default_retention_days() -> u32 {
    90
}

/// Context tracing configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextTracingConfig {
    /// Enable context tracing (default: true)
    #[serde(default = "default_context_tracing_enabled")]
    pub enabled: bool,

    /// Storage type for context tracing
    #[serde(default)]
    pub storage_type: StorageType,

    /// Path to SQLite database file
    #[serde(default = "default_sqlite_path")]
    pub sqlite_path: PathBuf,

    /// Maximum cache size (number of items, default: 10000)
    #[serde(default = "default_cache_size")]
    pub cache_size: usize,

    /// Cache TTL in seconds (default: 7 days = 604800 seconds)
    #[serde(default = "default_cache_ttl_seconds")]
    pub cache_ttl_seconds: i64,

    /// Retention period in days (default: 90)
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

impl Default for ContextTracingConfig {
    fn default() -> Self {
        Self {
            enabled: default_context_tracing_enabled(),
            storage_type: StorageType::default(),
            sqlite_path: default_sqlite_path(),
            cache_size: default_cache_size(),
            cache_ttl_seconds: default_cache_ttl_seconds(),
            retention_days: default_retention_days(),
        }
    }
}

/// Storage backend type
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StorageType {
    /// Hybrid DashMap + SQLite storage (recommended)
    Hybrid,
    /// SQLite only (no caching)
    SqliteOnly,
}

impl Default for StorageType {
    fn default() -> Self {
        Self::Hybrid
    }
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

    /// Get the effective health check configuration for a specific server
    pub fn get_server_health_check(&self, server_name: &str) -> Option<EffectiveHealthCheckConfig> {
        let server = self.servers.get(server_name)?;

        // If global health checks are disabled, return None
        if !self.health_check.enabled {
            return None;
        }

        // Check if server has health check configured
        if let Some(server_hc) = &server.health_check {
            // If server explicitly disables health checks, return None
            if server_hc.enabled == Some(false) {
                return None;
            }
        }

        // Build effective configuration
        let server_hc = server.health_check.as_ref();
        Some(EffectiveHealthCheckConfig {
            interval_seconds: server_hc
                .and_then(|hc| hc.interval_seconds)
                .unwrap_or(self.health_check.interval_seconds),
            timeout_seconds: server_hc
                .and_then(|hc| hc.timeout_seconds)
                .unwrap_or(self.health_check.timeout_seconds),
            max_attempts: server_hc
                .and_then(|hc| hc.max_attempts)
                .unwrap_or(self.health_check.max_attempts),
            retry_interval_seconds: server_hc
                .and_then(|hc| hc.retry_interval_seconds)
                .unwrap_or(self.health_check.retry_interval_seconds),
        })
    }
}

#[derive(Debug, Clone)]
pub struct EffectiveHealthCheckConfig {
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
    pub max_attempts: u32,
    pub retry_interval_seconds: u64,
}
