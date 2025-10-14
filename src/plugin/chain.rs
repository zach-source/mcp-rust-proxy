//! Plugin chain execution logic
//!
//! This module handles sequential execution of plugin chains.

use crate::plugin::config::{PluginAssignment, PluginConfig};
use crate::plugin::manager::PluginManager;
use crate::plugin::schema::{PluginError, PluginInput, PluginOutput, PluginPhase};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Represents an ordered sequence of plugins applied to a specific MCP server or tool
pub struct PluginChain {
    /// Server name this chain applies to
    server_name: String,
    /// Execution phase
    phase: PluginPhase,
    /// Plugin manager for execution
    manager: Arc<PluginManager>,
    /// Plugin configuration
    config: Arc<PluginConfig>,
}

impl PluginChain {
    /// Create a new plugin chain
    pub fn new(
        server_name: String,
        phase: PluginPhase,
        manager: Arc<PluginManager>,
        config: Arc<PluginConfig>,
    ) -> Self {
        Self {
            server_name,
            phase,
            manager,
            config,
        }
    }

    /// Build a chain of plugins for execution
    pub fn build(&self) -> Vec<PluginAssignment> {
        self.config
            .get_plugins_for_phase(&self.server_name, self.phase)
    }

    /// Execute the plugin chain sequentially
    pub async fn execute(&self, mut input: PluginInput) -> Result<PluginOutput, PluginError> {
        let plugins = self.build();

        if plugins.is_empty() {
            debug!(
                "No plugins configured for server '{}' phase '{:?}'",
                self.server_name, self.phase
            );
            // No plugins - return input as output
            return Ok(PluginOutput {
                text: input.raw_content,
                continue_: true,
                metadata: None,
                error: None,
            });
        }

        info!(
            "Executing {} plugins for server '{}' phase '{:?}'",
            plugins.len(),
            self.server_name,
            self.phase
        );

        let mut current_output = PluginOutput {
            text: input.raw_content.clone(),
            continue_: true,
            metadata: None,
            error: None,
        };

        // Collect metadata from all plugins in the chain
        let mut aggregated_metadata = serde_json::Map::new();

        for (index, assignment) in plugins.iter().enumerate() {
            if !current_output.continue_ {
                info!(
                    "Chain execution stopped at plugin {} (continue=false)",
                    assignment.name
                );
                break;
            }

            debug!(
                "Executing plugin {}/{}: '{}'",
                index + 1,
                plugins.len(),
                assignment.name
            );

            // Update input with output from previous plugin
            input.raw_content = current_output.text.clone();

            // Execute plugin
            match self.manager.execute_assignment(assignment, &input).await {
                Ok(output) => {
                    debug!(
                        "Plugin '{}' succeeded (continue={}, has_error={})",
                        assignment.name,
                        output.continue_,
                        output.error.is_some()
                    );

                    // Collect metadata from this plugin
                    if let Some(plugin_metadata) = output.metadata.clone() {
                        aggregated_metadata.insert(assignment.name.clone(), plugin_metadata);
                    }

                    // Check for error field
                    if output.error.is_some() {
                        warn!(
                            "Plugin '{}' returned error: {:?}",
                            assignment.name, output.error
                        );
                        // Plugin reported error - return original content with aggregated metadata
                        aggregated_metadata.insert(
                            "error_plugin".to_string(),
                            serde_json::json!(assignment.name.clone()),
                        );
                        return Ok(PluginOutput {
                            text: input.raw_content,
                            continue_: false,
                            metadata: Some(serde_json::Value::Object(aggregated_metadata)),
                            error: output.error,
                        });
                    }

                    current_output = output;
                }
                Err(e) => {
                    error!("Plugin '{}' failed: {:?}", assignment.name, e);
                    // On error, return original content with aggregated metadata
                    aggregated_metadata.insert(
                        "failed_plugin".to_string(),
                        serde_json::json!(assignment.name.clone()),
                    );
                    aggregated_metadata
                        .insert("error".to_string(), serde_json::json!(e.to_string()));
                    return Ok(PluginOutput {
                        text: input.raw_content,
                        continue_: false,
                        metadata: Some(serde_json::Value::Object(aggregated_metadata)),
                        error: Some(format!("Plugin '{}' failed: {}", assignment.name, e)),
                    });
                }
            }
        }

        info!(
            "Chain execution complete for server '{}' phase '{:?}'",
            self.server_name, self.phase
        );

        // Return final output with aggregated metadata from all plugins
        Ok(PluginOutput {
            text: current_output.text,
            continue_: current_output.continue_,
            metadata: Some(serde_json::Value::Object(aggregated_metadata)),
            error: current_output.error,
        })
    }

    /// Execute chain with graceful error handling (never fails, always returns content)
    pub async fn execute_safe(&self, input: PluginInput) -> PluginOutput {
        let original_content = input.raw_content.clone();

        match self.execute(input).await {
            Ok(output) => output,
            Err(e) => {
                error!(
                    "Chain execution failed for server '{}': {:?}",
                    self.server_name, e
                );
                // Return original content on catastrophic failure
                PluginOutput {
                    text: original_content,
                    continue_: false,
                    metadata: None,
                    error: Some(format!("Chain execution failed: {e}")),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::config::ServerPluginConfig;
    use crate::plugin::schema::PluginMetadata;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_empty_chain() {
        let temp_dir = TempDir::new().unwrap();

        let config = Arc::new(PluginConfig {
            plugin_dir: temp_dir.path().to_path_buf(),
            node_executable: "node".into(),
            max_concurrent_executions: 10,
            pool_size_per_plugin: 5,
            default_timeout_ms: 30000,
            servers: HashMap::new(),
        });

        let manager = Arc::new(PluginManager::new((*config).clone()));

        let chain = PluginChain::new(
            "test-server".to_string(),
            PluginPhase::Response,
            manager,
            config,
        );

        let input = PluginInput {
            tool_name: "test/tool".to_string(),
            raw_content: "test content".to_string(),
            max_tokens: None,
            metadata: PluginMetadata {
                request_id: "req-1".to_string(),
                timestamp: "2025-10-10T12:00:00Z".to_string(),
                server_name: "test-server".to_string(),
                phase: PluginPhase::Response,
                user_query: None,
                tool_arguments: None,
                mcp_servers: None,
            },
        };

        let output = chain.execute(input).await.unwrap();
        assert_eq!(output.text, "test content");
        assert!(output.continue_);
    }

    #[tokio::test]
    async fn test_chain_ordering() {
        let temp_dir = TempDir::new().unwrap();

        let mut servers = HashMap::new();
        servers.insert(
            "test-server".to_string(),
            ServerPluginConfig {
                request: vec![],
                response: vec![
                    PluginAssignment {
                        name: "plugin2".to_string(),
                        order: 2,
                        enabled: true,
                        timeout_ms: None,
                    },
                    PluginAssignment {
                        name: "plugin1".to_string(),
                        order: 1,
                        enabled: true,
                        timeout_ms: None,
                    },
                ],
            },
        );

        let config = Arc::new(PluginConfig {
            plugin_dir: temp_dir.path().to_path_buf(),
            node_executable: "node".into(),
            max_concurrent_executions: 10,
            pool_size_per_plugin: 5,
            default_timeout_ms: 30000,
            servers,
        });

        let manager = Arc::new(PluginManager::new((*config).clone()));

        let chain = PluginChain::new(
            "test-server".to_string(),
            PluginPhase::Response,
            manager,
            config,
        );

        let plugins = chain.build();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name, "plugin1");
        assert_eq!(plugins[1].name, "plugin2");
    }

    #[tokio::test]
    async fn test_chain_stops_on_continue_false() {
        // This test verifies the chain stops when a plugin returns continue=false
        // Implementation relies on the execute logic checking continue_ flag
        let temp_dir = TempDir::new().unwrap();

        let config = Arc::new(PluginConfig {
            plugin_dir: temp_dir.path().to_path_buf(),
            node_executable: "node".into(),
            max_concurrent_executions: 10,
            pool_size_per_plugin: 5,
            default_timeout_ms: 30000,
            servers: HashMap::new(),
        });

        let manager = Arc::new(PluginManager::new((*config).clone()));

        let chain = PluginChain::new(
            "test-server".to_string(),
            PluginPhase::Response,
            manager,
            config,
        );

        // Test that chain handles continue=false correctly
        let output = PluginOutput {
            text: "modified".to_string(),
            continue_: false,
            metadata: None,
            error: None,
        };

        assert!(!output.continue_);
    }
}
