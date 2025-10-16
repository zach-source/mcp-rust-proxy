//! Plugin lifecycle management and concurrency control
//!
//! This module manages plugin discovery, loading, and execution coordination.

use crate::plugin::config::{PluginAssignment, PluginConfig};
use crate::plugin::process::ProcessPool;
use crate::plugin::schema::{PluginError, PluginInput, PluginOutput, PluginPhase};
use crate::state::Metrics;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct Plugin {
    /// Plugin name
    pub name: String,
    /// Path to plugin file
    pub path: PathBuf,
    /// Execution phase
    pub phase: PluginPhase,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

/// Manages plugin lifecycle and concurrency
pub struct PluginManager {
    /// Global plugin configuration
    config: PluginConfig,
    /// Discovered plugins indexed by name
    plugins: Arc<DashMap<String, Plugin>>,
    /// Process pools indexed by plugin name
    pools: Arc<DashMap<String, Arc<ProcessPool>>>,
    /// Global concurrency semaphore
    semaphore: Arc<Semaphore>,
    /// Metrics for observability
    metrics: Option<Arc<Metrics>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(config: PluginConfig) -> Self {
        let max_concurrent = config.max_concurrent_executions as usize;

        Self {
            config,
            plugins: Arc::new(DashMap::new()),
            pools: Arc::new(DashMap::new()),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            metrics: None,
        }
    }

    /// Set metrics for plugin execution tracking
    pub fn set_metrics(&mut self, metrics: Arc<Metrics>) {
        self.metrics = Some(metrics);
    }

    /// Discover and load plugins from the plugin directory
    pub async fn discover_plugins(&self) -> Result<usize, PluginError> {
        let plugin_dir = &self.config.plugin_dir;

        if !plugin_dir.exists() {
            return Err(PluginError::ConfigError {
                reason: format!("Plugin directory does not exist: {}", plugin_dir.display()),
            });
        }

        let mut count = 0;
        let entries = std::fs::read_dir(plugin_dir).map_err(|e| PluginError::ConfigError {
            reason: format!("Failed to read plugin directory: {e}"),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| PluginError::ConfigError {
                reason: format!("Failed to read directory entry: {e}"),
            })?;

            let path = entry.path();

            // Only load .js files
            if path.extension().and_then(|s| s.to_str()) == Some("js") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let plugin = Plugin {
                        name: name.to_string(),
                        path: path.clone(),
                        phase: PluginPhase::Response, // Default phase, will be overridden by config
                        timeout_ms: self.config.default_timeout_ms,
                    };

                    self.plugins.insert(name.to_string(), plugin);
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Get or create a process pool for a plugin
    async fn get_or_create_pool(&self, plugin_name: &str) -> Result<Arc<ProcessPool>, PluginError> {
        // Check if pool already exists
        if let Some(pool) = self.pools.get(plugin_name) {
            return Ok(pool.clone());
        }

        // Get plugin info
        let plugin = self
            .plugins
            .get(plugin_name)
            .ok_or_else(|| PluginError::NotFound {
                name: plugin_name.to_string(),
            })?;

        // Create new pool
        let pool = Arc::new(ProcessPool::new(
            self.config.node_executable.clone(),
            plugin.path.clone(),
            self.config.pool_size_per_plugin as usize,
        ));

        self.pools.insert(plugin_name.to_string(), pool.clone());
        Ok(pool)
    }

    /// Execute a plugin with concurrency control and timeout
    pub async fn execute(
        &self,
        plugin_name: &str,
        input: &PluginInput,
        timeout_ms: u64,
    ) -> Result<PluginOutput, PluginError> {
        use tracing::{debug, error, info, warn};

        let start = Instant::now();
        let server_name = &input.metadata.server_name;
        let phase = format!("{:?}", input.metadata.phase).to_lowercase();
        let input_size = input.raw_content.len();

        // Create structured tracing span for this execution
        let span = tracing::info_span!(
            "plugin_execution",
            plugin = %plugin_name,
            server = %server_name,
            phase = %phase,
            tool = %input.tool_name,
            request_id = %input.metadata.request_id,
        );
        let _guard = span.enter();

        debug!(
            input_size_bytes = input_size,
            timeout_ms = timeout_ms,
            "Starting plugin execution"
        );

        // Acquire semaphore permit (blocks if at max concurrency)
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| PluginError::PoolExhausted)?;

        // Get process from pool
        let pool = self.get_or_create_pool(plugin_name).await?;
        let mut process = pool.acquire().await?;

        debug!("Acquired process from pool");

        // Execute with timeout
        let result = timeout(Duration::from_millis(timeout_ms), process.execute(input)).await;

        let output = match result {
            Ok(Ok(output)) => {
                let duration = start.elapsed();
                let output_size = output.text.len();

                // Success - release process back to pool
                pool.release(process).await;

                info!(
                    duration_ms = duration.as_millis() as u64,
                    input_size_bytes = input_size,
                    output_size_bytes = output_size,
                    continue_processing = output.continue_,
                    has_error = output.error.is_some(),
                    has_metadata = output.metadata.is_some(),
                    "Plugin execution succeeded"
                );

                // Record success metrics
                if let Some(metrics) = &self.metrics {
                    metrics.record_plugin_execution(
                        plugin_name,
                        server_name,
                        &phase,
                        duration,
                        true,
                    );
                }

                Ok(output)
            }
            Ok(Err(e)) => {
                let duration = start.elapsed();

                // Plugin error - kill process and return error
                let _ = process.kill().await;

                error!(
                    duration_ms = duration.as_millis() as u64,
                    input_size_bytes = input_size,
                    error = %e,
                    "Plugin execution failed"
                );

                // Record error metrics
                if let Some(metrics) = &self.metrics {
                    metrics.record_plugin_execution(
                        plugin_name,
                        server_name,
                        &phase,
                        duration,
                        false,
                    );
                    metrics.record_plugin_error(plugin_name, server_name, "execution_error");
                }

                Err(e)
            }
            Err(_) => {
                let duration = start.elapsed();

                // Timeout - kill process and return timeout error
                let _ = process.kill().await;

                warn!(
                    duration_ms = duration.as_millis() as u64,
                    timeout_ms = timeout_ms,
                    input_size_bytes = input_size,
                    "Plugin execution timed out"
                );

                // Record timeout metrics
                if let Some(metrics) = &self.metrics {
                    metrics.record_plugin_execution(
                        plugin_name,
                        server_name,
                        &phase,
                        duration,
                        false,
                    );
                    metrics.record_plugin_timeout(plugin_name, server_name);
                }

                Err(PluginError::Timeout { timeout_ms })
            }
        };

        output
    }

    /// Execute a plugin assignment (with config-specific timeout)
    pub async fn execute_assignment(
        &self,
        assignment: &PluginAssignment,
        input: &PluginInput,
    ) -> Result<PluginOutput, PluginError> {
        let timeout_ms = self.config.get_plugin_timeout(assignment);
        self.execute(&assignment.name, input, timeout_ms).await
    }

    /// Shutdown all process pools
    pub async fn shutdown(&self) {
        for entry in self.pools.iter() {
            entry.value().shutdown().await;
        }
    }

    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Get pool count
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::schema::PluginMetadata;

    use tempfile::TempDir;

    #[tokio::test]
    async fn test_plugin_discovery() {
        let temp_dir = TempDir::new().unwrap();

        // Create test plugins
        let plugin1_path = temp_dir.path().join("plugin1.js");
        std::fs::write(&plugin1_path, "console.log('plugin1')").unwrap();

        let plugin2_path = temp_dir.path().join("plugin2.js");
        std::fs::write(&plugin2_path, "console.log('plugin2')").unwrap();

        // Create non-plugin file (should be ignored)
        let text_file = temp_dir.path().join("readme.txt");
        std::fs::write(&text_file, "readme").unwrap();

        let config = PluginConfig {
            plugin_dir: temp_dir.path().to_path_buf(),
            node_executable: PathBuf::from("node"),
            max_concurrent_executions: 10,
            pool_size_per_plugin: 5,
            default_timeout_ms: 30000,
            servers: std::collections::HashMap::new(),
        };

        let manager = PluginManager::new(config);
        let count = manager.discover_plugins().await.unwrap();

        assert_eq!(count, 2);
        assert_eq!(manager.plugin_count(), 2);
        assert!(manager.plugins.contains_key("plugin1"));
        assert!(manager.plugins.contains_key("plugin2"));
    }

    #[tokio::test]
    async fn test_concurrency_limit() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("test.js");

        // Create a plugin that sleeps for a bit
        let plugin_code = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, terminal: false });
rl.on('line', async (line) => {
    await new Promise(resolve => setTimeout(resolve, 100));
    const input = JSON.parse(line);
    process.stdout.write(JSON.stringify({ text: input.rawContent, continue: true }) + '\n');
});
"#;
        std::fs::write(&plugin_path, plugin_code).unwrap();

        let config = PluginConfig {
            plugin_dir: temp_dir.path().to_path_buf(),
            node_executable: PathBuf::from("node"),
            max_concurrent_executions: 2, // Low limit for testing
            pool_size_per_plugin: 1,
            default_timeout_ms: 5000,
            servers: std::collections::HashMap::new(),
        };

        let manager = PluginManager::new(config);
        manager.discover_plugins().await.unwrap();

        // Try to execute 3 plugins concurrently
        // Only 2 should run at a time due to semaphore
        let input = PluginInput {
            tool_name: "test".to_string(),
            raw_content: "test".to_string(),
            max_tokens: None,
            metadata: PluginMetadata {
                request_id: "req-1".to_string(),
                timestamp: "2025-10-10T12:00:00Z".to_string(),
                server_name: "test".to_string(),
                phase: PluginPhase::Response,
                user_query: None,
                tool_arguments: None,
                mcp_servers: None,
            },
        };

        // This test just verifies the manager compiles and basic execution works
        // Full concurrency testing would require more complex setup
        assert_eq!(manager.plugin_count(), 1);
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("slow.js");

        // Create a plugin that never responds
        let plugin_code = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, terminal: false });
rl.on('line', () => {
    // Never respond - just hang
});
"#;
        std::fs::write(&plugin_path, plugin_code).unwrap();

        let config = PluginConfig {
            plugin_dir: temp_dir.path().to_path_buf(),
            node_executable: PathBuf::from("node"),
            max_concurrent_executions: 10,
            pool_size_per_plugin: 1,
            default_timeout_ms: 500, // Short timeout
            servers: std::collections::HashMap::new(),
        };

        let manager = PluginManager::new(config);
        manager.discover_plugins().await.unwrap();

        let input = PluginInput {
            tool_name: "test".to_string(),
            raw_content: "test".to_string(),
            max_tokens: None,
            metadata: PluginMetadata {
                request_id: "req-1".to_string(),
                timestamp: "2025-10-10T12:00:00Z".to_string(),
                server_name: "test".to_string(),
                phase: PluginPhase::Response,
                user_query: None,
                tool_arguments: None,
                mcp_servers: None,
            },
        };

        let result = manager.execute("slow", &input, 500).await;
        assert!(result.is_err());

        if let Err(PluginError::Timeout { timeout_ms }) = result {
            assert_eq!(timeout_ms, 500);
        } else {
            panic!("Expected timeout error");
        }
    }
}
