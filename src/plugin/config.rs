//! Plugin configuration types and parsing
//!
//! This module defines configuration structures for the plugin system.

use crate::plugin::schema::PluginPhase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Global plugin system configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    /// Directory containing plugin .js files
    pub plugin_dir: PathBuf,

    /// Path to Node.js executable (default: "node")
    #[serde(default = "default_node_executable")]
    pub node_executable: PathBuf,

    /// Maximum concurrent plugin executions (global semaphore limit)
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_executions: u32,

    /// Number of warm processes to maintain per plugin
    #[serde(default = "default_pool_size")]
    pub pool_size_per_plugin: u32,

    /// Default timeout for plugin execution (milliseconds)
    #[serde(default = "default_timeout_ms")]
    pub default_timeout_ms: u64,

    /// Server-specific plugin assignments
    #[serde(default)]
    pub servers: HashMap<String, ServerPluginConfig>,
}

/// Plugin configuration for a specific server
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPluginConfig {
    /// Request-phase plugins (run before forwarding to MCP server)
    #[serde(default)]
    pub request: Vec<PluginAssignment>,

    /// Response-phase plugins (run after receiving response from MCP server)
    #[serde(default)]
    pub response: Vec<PluginAssignment>,
}

/// Individual plugin assignment
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAssignment {
    /// Plugin name (matches .js filename without extension)
    pub name: String,

    /// Execution order (lower numbers execute first)
    pub order: u32,

    /// Whether this plugin is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Override default timeout for this plugin (milliseconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

// Default value functions
fn default_node_executable() -> PathBuf {
    PathBuf::from("node")
}

fn default_max_concurrent() -> u32 {
    10
}

fn default_pool_size() -> u32 {
    5
}

fn default_timeout_ms() -> u64 {
    30000 // 30 seconds
}

pub(crate) fn default_enabled() -> bool {
    true
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            plugin_dir: PathBuf::from("./plugins"),
            node_executable: default_node_executable(),
            max_concurrent_executions: default_max_concurrent(),
            pool_size_per_plugin: default_pool_size(),
            default_timeout_ms: default_timeout_ms(),
            servers: HashMap::new(),
        }
    }
}

impl PluginConfig {
    /// Validate plugin configuration
    pub fn validate(&self) -> Result<(), String> {
        // Check plugin directory exists
        if !self.plugin_dir.exists() {
            return Err(format!(
                "Plugin directory does not exist: {}",
                self.plugin_dir.display()
            ));
        }

        if !self.plugin_dir.is_dir() {
            return Err(format!(
                "Plugin directory path is not a directory: {}",
                self.plugin_dir.display()
            ));
        }

        // Validate concurrency limits
        if self.max_concurrent_executions == 0 {
            return Err("max_concurrent_executions must be greater than 0".to_string());
        }

        if self.max_concurrent_executions > 100 {
            return Err("max_concurrent_executions cannot exceed 100".to_string());
        }

        if self.pool_size_per_plugin > self.max_concurrent_executions {
            return Err(format!(
                "pool_size_per_plugin ({}) cannot exceed max_concurrent_executions ({})",
                self.pool_size_per_plugin, self.max_concurrent_executions
            ));
        }

        // Validate timeout
        if self.default_timeout_ms < 100 {
            return Err("default_timeout_ms must be at least 100ms".to_string());
        }

        if self.default_timeout_ms > 600_000 {
            return Err("default_timeout_ms cannot exceed 600000ms (10 minutes)".to_string());
        }

        // Validate server assignments
        for (server_name, server_config) in &self.servers {
            self.validate_server_config(server_name, server_config)?;
        }

        Ok(())
    }

    /// Validate server-specific plugin configuration
    fn validate_server_config(
        &self,
        server_name: &str,
        server_config: &ServerPluginConfig,
    ) -> Result<(), String> {
        // Validate request-phase plugins
        self.validate_plugin_list(server_name, "request", &server_config.request)?;

        // Validate response-phase plugins
        self.validate_plugin_list(server_name, "response", &server_config.response)?;

        Ok(())
    }

    /// Validate a list of plugin assignments
    fn validate_plugin_list(
        &self,
        server_name: &str,
        phase: &str,
        plugins: &[PluginAssignment],
    ) -> Result<(), String> {
        // Check for duplicate plugin names in same phase
        let mut seen_names = std::collections::HashSet::new();
        for plugin in plugins {
            if !seen_names.insert(&plugin.name) {
                return Err(format!(
                    "Duplicate plugin '{}' in {} phase for server '{}'",
                    plugin.name, phase, server_name
                ));
            }

            // Validate timeout if specified
            if let Some(timeout_ms) = plugin.timeout_ms {
                if timeout_ms < 100 {
                    return Err(format!(
                        "Plugin '{}' timeout_ms must be at least 100ms",
                        plugin.name
                    ));
                }
                if timeout_ms > 600_000 {
                    return Err(format!(
                        "Plugin '{}' timeout_ms cannot exceed 600000ms (10 minutes)",
                        plugin.name
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get plugins for a specific server and phase, sorted by order
    pub fn get_plugins_for_phase(
        &self,
        server_name: &str,
        phase: PluginPhase,
    ) -> Vec<PluginAssignment> {
        if let Some(server_config) = self.servers.get(server_name) {
            let mut plugins = match phase {
                PluginPhase::Request => server_config.request.clone(),
                PluginPhase::Response => server_config.response.clone(),
            };

            // Filter enabled plugins and sort by order
            plugins.retain(|p| p.enabled);
            plugins.sort_by_key(|p| p.order);
            plugins
        } else {
            Vec::new()
        }
    }

    /// Get the effective timeout for a plugin
    pub fn get_plugin_timeout(&self, assignment: &PluginAssignment) -> u64 {
        assignment.timeout_ms.unwrap_or(self.default_timeout_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PluginConfig::default();
        assert_eq!(config.max_concurrent_executions, 10);
        assert_eq!(config.pool_size_per_plugin, 5);
        assert_eq!(config.default_timeout_ms, 30000);
    }

    #[test]
    fn test_validation_concurrency_limits() {
        let mut config = PluginConfig::default();

        // Create temp plugin dir for validation
        let temp_dir = std::env::temp_dir().join("test-plugins");
        std::fs::create_dir_all(&temp_dir).unwrap();
        config.plugin_dir = temp_dir.clone();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Zero max_concurrent should fail
        config.max_concurrent_executions = 0;
        assert!(config.validate().is_err());

        // Pool size > max concurrent should fail
        config.max_concurrent_executions = 5;
        config.pool_size_per_plugin = 10;
        assert!(config.validate().is_err());

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_plugin_ordering() {
        let mut config = PluginConfig::default();

        let server_config = ServerPluginConfig {
            request: vec![],
            response: vec![
                PluginAssignment {
                    name: "plugin3".to_string(),
                    order: 3,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "plugin1".to_string(),
                    order: 1,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "plugin2".to_string(),
                    order: 2,
                    enabled: true,
                    timeout_ms: None,
                },
            ],
        };

        config.servers.insert("test".to_string(), server_config);

        let plugins = config.get_plugins_for_phase("test", PluginPhase::Response);
        assert_eq!(plugins.len(), 3);
        assert_eq!(plugins[0].name, "plugin1");
        assert_eq!(plugins[1].name, "plugin2");
        assert_eq!(plugins[2].name, "plugin3");
    }

    #[test]
    fn test_enabled_filtering() {
        let mut config = PluginConfig::default();

        let server_config = ServerPluginConfig {
            request: vec![],
            response: vec![
                PluginAssignment {
                    name: "enabled".to_string(),
                    order: 1,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "disabled".to_string(),
                    order: 2,
                    enabled: false,
                    timeout_ms: None,
                },
            ],
        };

        config.servers.insert("test".to_string(), server_config);

        let plugins = config.get_plugins_for_phase("test", PluginPhase::Response);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "enabled");
    }

    #[test]
    fn test_timeout_override() {
        let config = PluginConfig::default();

        let assignment = PluginAssignment {
            name: "test".to_string(),
            order: 1,
            enabled: true,
            timeout_ms: Some(45000),
        };

        assert_eq!(config.get_plugin_timeout(&assignment), 45000);

        let assignment_no_override = PluginAssignment {
            name: "test".to_string(),
            order: 1,
            enabled: true,
            timeout_ms: None,
        };

        assert_eq!(config.get_plugin_timeout(&assignment_no_override), 30000);
    }
}
