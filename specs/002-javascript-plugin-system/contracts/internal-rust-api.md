# Internal Rust API Contract

**Feature**: JavaScript Plugin System
**Audience**: Rust developers implementing plugin system
**Module**: `src/plugin/`

## Module Structure

```rust
// src/plugin/mod.rs
pub mod config;     // Configuration loading and validation
pub mod runtime;    // Node.js runtime bridge
pub mod executor;   // Plugin execution engine
pub mod chain;      // Plugin chain orchestration
pub mod schema;     // Input/output schemas

pub use config::PluginConfig;
pub use executor::{PluginExecutor, PluginInput, PluginOutput, PluginError};
pub use chain::PluginChain;
```

## Public API

### 1. PluginExecutor

Main interface for executing individual plugins.

```rust
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Child;
use serde::{Serialize, Deserialize};

/// Executes JavaScript plugins as Node.js subprocesses
pub struct PluginExecutor {
    plugin_path: PathBuf,
    timeout: Duration,
    process_pool: ProcessPool,
    metrics: Arc<ExecutionMetrics>,
}

impl PluginExecutor {
    /// Create new executor for a plugin
    pub fn new(
        plugin_path: impl AsRef<Path>,
        timeout: Duration,
        pool_size: usize,
    ) -> Result<Self, PluginError>;

    /// Execute plugin with input, return output or error
    pub async fn execute(
        &self,
        input: PluginInput,
    ) -> Result<PluginOutput, PluginError>;

    /// Get execution metrics for this plugin
    pub fn metrics(&self) -> &ExecutionMetrics;

    /// Shutdown executor and terminate all processes
    pub async fn shutdown(self) -> Result<(), PluginError>;
}
```

### 2. PluginInput / PluginOutput

Data structures matching JSON schema.

```rust
/// Input to a plugin (matches plugin-api.md schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_query: Option<String>,

    pub tool_name: String,
    pub raw_content: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    pub metadata: PluginMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub request_id: String,
    pub timestamp: String,
    pub server_name: String,
    pub phase: PluginPhase,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginPhase {
    Request,
    Response,
}

/// Output from a plugin (matches plugin-api.md schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginOutput {
    pub text: String,

    #[serde(rename = "continue")]
    pub continue_processing: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PluginOutputMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginOutputMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifications_applied: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

/// Error output from plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginErrorOutput {
    pub error: String,
    pub original_content: String,

    #[serde(rename = "continue")]
    pub continue_processing: bool,
}
```

### 3. PluginChain

Orchestrates sequential execution of multiple plugins.

```rust
/// Executes an ordered sequence of plugins
pub struct PluginChain {
    plugins: Vec<Arc<PluginExecutor>>,
    phase: PluginPhase,
}

impl PluginChain {
    /// Create new plugin chain from executors
    pub fn new(
        plugins: Vec<Arc<PluginExecutor>>,
        phase: PluginPhase,
    ) -> Self;

    /// Execute entire chain, returning final output
    pub async fn execute(
        &self,
        initial_input: PluginInput,
    ) -> Result<PluginOutput, PluginError>;

    /// Get number of plugins in chain
    pub fn len(&self) -> usize;

    /// Check if chain is empty
    pub fn is_empty(&self) -> bool;
}
```

### 4. PluginConfig

Configuration loading and management.

```rust
use serde::{Deserialize, Serialize};

/// Plugin configuration from YAML/JSON/TOML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginConfig {
    pub name: String,
    pub path: PathBuf,

    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub timeout_ms: Option<u32>,

    #[serde(default)]
    pub order: u32,

    #[serde(default)]
    pub config: serde_json::Value,
}

/// Per-server plugin settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerPluginConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_pool_size")]
    pub pool_size: usize,

    #[serde(default)]
    pub global_timeout_ms: Option<u32>,

    #[serde(default)]
    pub request_phase: Vec<PluginConfig>,

    #[serde(default)]
    pub response_phase: Vec<PluginConfig>,
}

/// Global plugin settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalPluginSettings {
    #[serde(default = "default_plugin_dir")]
    pub plugin_directory: PathBuf,

    #[serde(default = "default_node_exe")]
    pub node_executable: String,

    #[serde(default = "default_timeout")]
    pub default_timeout_ms: u32,

    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_executions: usize,

    #[serde(default = "default_enable_metrics")]
    pub enable_metrics: bool,

    #[serde(default = "default_log_dir")]
    pub log_directory: PathBuf,
}

impl ServerPluginConfig {
    /// Load plugins for a server, creating executors
    pub async fn load_executors(
        &self,
        global: &GlobalPluginSettings,
    ) -> Result<(Vec<Arc<PluginExecutor>>, Vec<Arc<PluginExecutor>>), PluginError>;
}
```

### 5. PluginError

Comprehensive error type for plugin system.

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin file not found: {0}")]
    PluginNotFound(PathBuf),

    #[error("Plugin execution timeout after {0:?}")]
    Timeout(Duration),

    #[error("Plugin process failed with exit code {code}: {stderr}")]
    ProcessFailed { code: i32, stderr: String },

    #[error("Invalid plugin output: {0}")]
    InvalidOutput(String),

    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Plugin returned error: {0}")]
    PluginReturnedError(String),

    #[error("Process pool exhausted")]
    PoolExhausted,

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl PluginError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Timeout(_) | Self::PoolExhausted)
    }

    /// Convert to fallback output with original content
    pub fn to_fallback_output(&self, original_content: String) -> PluginOutput {
        PluginOutput {
            text: original_content,
            continue_processing: false,
            metadata: None,
        }
    }
}
```

## Internal Components

### ProcessPool

Manages warm Node.js process instances.

```rust
use tokio::sync::Mutex;
use std::collections::VecDeque;

struct ProcessPool {
    available: Arc<Mutex<VecDeque<ChildProcess>>>,
    max_size: usize,
    plugin_path: PathBuf,
    node_executable: String,
}

impl ProcessPool {
    /// Create new pool and warm up processes
    async fn new(
        plugin_path: PathBuf,
        node_executable: String,
        size: usize,
    ) -> Result<Self, PluginError>;

    /// Acquire process from pool (or spawn if empty)
    async fn acquire(&self) -> Result<ChildProcess, PluginError>;

    /// Return process to pool (or discard if unhealthy)
    async fn release(&self, process: ChildProcess) -> Result<(), PluginError>;

    /// Shutdown all processes
    async fn shutdown(self) -> Result<(), PluginError>;
}

struct ChildProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
    spawned_at: Instant,
}

impl ChildProcess {
    /// Spawn new Node.js process
    async fn spawn(
        plugin_path: &Path,
        node_exe: &str,
    ) -> Result<Self, PluginError>;

    /// Execute plugin with input, return output
    async fn execute(
        &mut self,
        input: &PluginInput,
        timeout: Duration,
    ) -> Result<PluginOutput, PluginError>;

    /// Check if process is healthy
    fn is_healthy(&self) -> bool;

    /// Kill process
    async fn kill(mut self) -> Result<(), PluginError>;
}
```

### ExecutionMetrics

Tracks plugin performance.

```rust
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct ExecutionMetrics {
    total_executions: AtomicU64,
    successful_executions: AtomicU64,
    failed_executions: AtomicU64,
    timeout_executions: AtomicU64,
    total_execution_time_ms: AtomicU64,
}

impl ExecutionMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_success(&self, duration_ms: u64) {
        self.total_executions.fetch_add(1, Ordering::Relaxed);
        self.successful_executions.fetch_add(1, Ordering::Relaxed);
        self.total_execution_time_ms.fetch_add(duration_ms, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.total_executions.fetch_add(1, Ordering::Relaxed);
        self.failed_executions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_timeout(&self) {
        self.total_executions.fetch_add(1, Ordering::Relaxed);
        self.timeout_executions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn avg_execution_time_ms(&self) -> u64 {
        let total = self.total_executions.load(Ordering::Relaxed);
        if total == 0 { return 0; }
        self.total_execution_time_ms.load(Ordering::Relaxed) / total
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_executions.load(Ordering::Relaxed);
        if total == 0 { return 0.0; }
        let success = self.successful_executions.load(Ordering::Relaxed);
        success as f64 / total as f64
    }
}
```

## Integration with Proxy

### Hook Points in Request Flow

```rust
// src/proxy/handler.rs

use crate::plugin::{PluginChain, PluginInput, PluginPhase};

async fn handle_request(
    request: McpRequest,
    server_config: &ServerConfig,
    plugin_chains: &PluginChains,
) -> Result<McpResponse, ProxyError> {
    // 1. Execute request-phase plugins
    let modified_request = if let Some(chain) = &plugin_chains.request {
        let input = PluginInput {
            user_query: extract_query(&request),
            tool_name: request.tool_name.clone(),
            raw_content: serde_json::to_string(&request.params)?,
            max_tokens: None,
            metadata: create_metadata(&request, PluginPhase::Request),
        };

        match chain.execute(input).await {
            Ok(output) if output.continue_processing => {
                modify_request(request, &output.text)?
            }
            Ok(output) => {
                // Plugin halted: return early
                return Ok(create_response_from_plugin(output));
            }
            Err(e) => {
                warn!("Request plugin failed: {}", e);
                request // Use original on error
            }
        }
    } else {
        request
    };

    // 2. Forward to MCP server
    let server_response = forward_to_server(&modified_request, server_config).await?;

    // 3. Execute response-phase plugins
    let final_response = if let Some(chain) = &plugin_chains.response {
        let input = PluginInput {
            user_query: extract_query(&modified_request),
            tool_name: modified_request.tool_name.clone(),
            raw_content: serde_json::to_string(&server_response)?,
            max_tokens: Some(10000),
            metadata: create_metadata(&modified_request, PluginPhase::Response),
        };

        match chain.execute(input).await {
            Ok(output) if output.continue_processing => {
                modify_response(server_response, &output.text)?
            }
            Ok(output) => {
                create_response_from_plugin(output)
            }
            Err(e) => {
                warn!("Response plugin failed: {}", e);
                server_response // Use original on error
            }
        }
    } else {
        server_response
    };

    Ok(final_response)
}
```

### State Management

```rust
// src/state/plugin.rs

use dashmap::DashMap;
use crate::plugin::{PluginExecutor, PluginChain};

/// Global plugin state
pub struct PluginState {
    /// Executors by plugin name
    executors: Arc<DashMap<String, Arc<PluginExecutor>>>,

    /// Plugin chains by server name
    request_chains: Arc<DashMap<String, Arc<PluginChain>>>,
    response_chains: Arc<DashMap<String, Arc<PluginChain>>>,

    /// Global settings
    settings: Arc<GlobalPluginSettings>,
}

impl PluginState {
    pub fn new(settings: GlobalPluginSettings) -> Self {
        Self {
            executors: Arc::new(DashMap::new()),
            request_chains: Arc::new(DashMap::new()),
            response_chains: Arc::new(DashMap::new()),
            settings: Arc::new(settings),
        }
    }

    /// Load plugins for a server
    pub async fn load_server_plugins(
        &self,
        server_name: &str,
        config: &ServerPluginConfig,
    ) -> Result<(), PluginError>;

    /// Get plugin chains for a server
    pub fn get_chains(&self, server_name: &str) -> Option<PluginChains>;

    /// Reload all plugins
    pub async fn reload(&self) -> Result<(), PluginError>;

    /// Shutdown all plugins
    pub async fn shutdown(self) -> Result<(), PluginError>;
}

pub struct PluginChains {
    pub request: Option<Arc<PluginChain>>,
    pub response: Option<Arc<PluginChain>>,
}
```

## Testing Interfaces

### Mock PluginExecutor for Tests

```rust
#[cfg(test)]
pub struct MockPluginExecutor {
    output: PluginOutput,
    delay: Duration,
    should_fail: bool,
}

#[cfg(test)]
impl MockPluginExecutor {
    pub fn success(text: String) -> Self {
        Self {
            output: PluginOutput {
                text,
                continue_processing: true,
                metadata: None,
            },
            delay: Duration::from_millis(10),
            should_fail: false,
        }
    }

    pub fn error() -> Self {
        Self {
            output: PluginOutput::default(),
            delay: Duration::from_millis(10),
            should_fail: true,
        }
    }

    pub async fn execute(&self, _input: PluginInput) -> Result<PluginOutput, PluginError> {
        tokio::time::sleep(self.delay).await;
        if self.should_fail {
            Err(PluginError::PluginReturnedError("mock error".into()))
        } else {
            Ok(self.output.clone())
        }
    }
}
```

## Logging and Observability

```rust
use tracing::{info, warn, error, debug};

// Log plugin execution
info!(
    plugin = %plugin_name,
    duration_ms = %duration.as_millis(),
    success = %success,
    "Plugin execution completed"
);

// Log plugin errors
warn!(
    plugin = %plugin_name,
    error = %e,
    "Plugin execution failed, using original content"
);

// Metrics via tracing spans
let _span = tracing::info_span!(
    "plugin_execution",
    plugin = %plugin_name,
    phase = ?phase,
).entered();
```

## Performance Considerations

1. **Process Pooling**: Amortize spawn overhead across requests
2. **Timeout Enforcement**: Hard timeout via `tokio::time::timeout`
3. **Graceful Degradation**: Always fall back to original content on error
4. **Concurrent Limit**: Global semaphore on `max_concurrent_executions`
5. **Metrics**: Low-overhead atomic counters for performance tracking

## Security Considerations

1. **No Sandboxing (MVP)**: Assume trusted plugins
2. **Process Isolation**: Each plugin runs in separate process
3. **Timeout Protection**: Prevent runaway plugins
4. **Input Validation**: Validate JSON schema before execution
5. **Error Information Leakage**: Sanitize error messages in production
