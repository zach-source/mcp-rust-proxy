//! Configuration for Claude API Proxy

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for the Claude API proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeProxyConfig {
    /// Whether the Claude API proxy is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Address to bind the proxy server (e.g., "127.0.0.1:8443")
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// Optional path to custom CA certificate (if not using default ~/.claude-proxy/ca.crt)
    #[serde(default)]
    pub ca_cert_path: Option<PathBuf>,

    /// Whether to capture requests and responses
    #[serde(default = "default_true")]
    pub capture_enabled: bool,

    /// Number of days to retain captured data
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

fn default_bind_address() -> String {
    "127.0.0.1:8443".to_string()
}

fn default_true() -> bool {
    true
}

fn default_retention_days() -> u32 {
    30
}

impl Default for ClaudeProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_address: default_bind_address(),
            ca_cert_path: None,
            capture_enabled: true,
            retention_days: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClaudeProxyConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.bind_address, "127.0.0.1:8443");
        assert!(config.capture_enabled);
        assert_eq!(config.retention_days, 30);
        assert!(config.ca_cert_path.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = ClaudeProxyConfig {
            enabled: true,
            bind_address: "localhost:9443".to_string(),
            ca_cert_path: Some(PathBuf::from("/tmp/ca.crt")),
            capture_enabled: false,
            retention_days: 7,
        };

        let yaml = serde_yaml::to_string(&config).expect("Failed to serialize");
        let deserialized: ClaudeProxyConfig =
            serde_yaml::from_str(&yaml).expect("Failed to deserialize");

        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.bind_address, deserialized.bind_address);
        assert_eq!(config.retention_days, deserialized.retention_days);
    }
}
