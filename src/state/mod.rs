use crate::config::Config;
use crate::context::tracker::ContextTracker;
use crate::error::Result;
use crate::logging::ServerLogger;
use crate::transport::pool::ConnectionPool;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod metrics;

pub use metrics::Metrics;

// #[cfg(test)]
// mod server_state_tests; // TODO: Add test module

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

/// Registry for storing server protocol versions and capabilities
#[derive(Clone)]
pub struct ServerVersion {
    pub protocol_version: String,
    pub capabilities: serde_json::Value,
    pub detected_at: DateTime<Utc>,
}

pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub servers: Arc<DashMap<String, ServerInfo>>,
    pub metrics: Arc<Metrics>,
    pub connection_pool: Arc<ConnectionPool>,
    pub shutdown_tx: tokio::sync::broadcast::Sender<()>,
    pub context_tracker: Arc<RwLock<Option<Arc<ContextTracker>>>>,
    pub plugin_manager: Option<Arc<crate::plugin::PluginManager>>,
    pub server_versions: Arc<DashMap<String, ServerVersion>>,
}

#[derive(Clone)]
pub struct ServerInfo {
    pub name: String,
    pub state: Arc<RwLock<ServerState>>,
    pub process_handle: Option<Arc<tokio::task::JoinHandle<()>>>,
    pub restart_count: Arc<RwLock<u32>>,
    pub last_health_check: Arc<RwLock<Option<HealthCheckStatus>>>,
    pub last_access_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    pub log_subscribers: Arc<DashMap<String, tokio::sync::mpsc::UnboundedSender<LogEntry>>>,
    pub logger: Option<Arc<ServerLogger>>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct HealthCheckStatus {
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
}

impl AppState {
    pub fn new(config: Config) -> (Arc<Self>, tokio::sync::broadcast::Receiver<()>) {
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(16);

        // Initialize plugin manager if plugins are configured
        let plugin_manager = config.plugins.as_ref().map(|plugin_config| {
            Arc::new(crate::plugin::PluginManager::new(plugin_config.clone()))
        });

        // Create server_versions registry to share with connection pool
        let server_versions = Arc::new(DashMap::new());

        let state = Arc::new(Self {
            config: Arc::new(RwLock::new(config)),
            servers: Arc::new(DashMap::new()),
            metrics: Arc::new(Metrics::new()),
            connection_pool: Arc::new(ConnectionPool::new(server_versions.clone())),
            shutdown_tx,
            context_tracker: Arc::new(RwLock::new(None)),
            plugin_manager,
            server_versions,
        });

        (state, shutdown_rx)
    }

    /// Initialize context tracker with storage backend
    ///
    /// This should be called during application startup if context tracing is enabled.
    ///
    /// # Arguments
    /// * `storage` - Storage backend for context tracing
    ///
    /// # Returns
    /// * `Ok(())` if initialized successfully
    /// * `Err(_)` if initialization fails
    pub async fn initialize_context_tracker(
        &self,
        storage: Arc<dyn crate::context::storage::StorageBackend>,
    ) -> Result<()> {
        let tracker = Arc::new(ContextTracker::new(storage));
        let mut context_tracker = self.context_tracker.write().await;
        *context_tracker = Some(tracker);
        Ok(())
    }

    pub async fn update_config(&self, new_config: Config) -> Result<()> {
        // Validate new config
        crate::config::validate(&new_config)?;

        // Update config
        let mut config = self.config.write().await;
        *config = new_config;

        // TODO: Notify all components of config change

        Ok(())
    }

    pub async fn register_server(&self, name: String, info: ServerInfo) {
        self.servers.insert(name.clone(), info);
        self.metrics.increment_server_count();
    }

    pub async fn unregister_server(&self, name: &str) {
        if self.servers.remove(name).is_some() {
            self.metrics.decrement_server_count();
        }
    }

    pub async fn get_server_state(&self, name: &str) -> Option<ServerState> {
        if let Some(info) = self.servers.get(name) {
            let state = info.state.read().await;
            Some(*state)
        } else {
            None
        }
    }

    pub async fn set_server_state(&self, name: &str, new_state: ServerState) -> Result<()> {
        if let Some(info) = self.servers.get(name) {
            let mut state = info.state.write().await;
            *state = new_state;

            // Update metrics
            match new_state {
                ServerState::Running => self.metrics.increment_running_servers(),
                ServerState::Failed => self.metrics.increment_failed_servers(),
                _ => {}
            }

            Ok(())
        } else {
            Err(crate::error::ProxyError::ServerNotFound(name.to_string()))
        }
    }

    pub async fn shutdown(&self) {
        tracing::info!("Initiating application shutdown");

        // Send shutdown signal to all components
        let _ = self.shutdown_tx.send(());

        // Close all connections
        let _ = self.connection_pool.close_all().await;

        // Stop all servers
        for entry in self.servers.iter() {
            let mut state = entry.value().state.write().await;
            *state = ServerState::Stopping;
        }
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_tx.receiver_count() == 0
    }

    pub async fn broadcast_update(&self) {
        // This is a placeholder for WebSocket broadcasting
        // In a real implementation, this would notify all connected WebSocket clients
        tracing::debug!("Broadcasting server state update");
    }
}

impl ServerInfo {
    pub fn new(name: String) -> Self {
        Self {
            name,
            state: Arc::new(RwLock::new(ServerState::Stopped)),
            process_handle: None,
            restart_count: Arc::new(RwLock::new(0)),
            last_health_check: Arc::new(RwLock::new(None)),
            last_access_time: Arc::new(RwLock::new(None)),
            log_subscribers: Arc::new(DashMap::new()),
            logger: None,
        }
    }

    pub async fn set_logger(&mut self, logger: Arc<ServerLogger>) {
        self.logger = Some(logger);
    }

    pub fn broadcast_log(&self, log_entry: LogEntry) {
        // Send to all subscribers
        let subscriber_count = self.log_subscribers.len();
        tracing::debug!(
            "Broadcasting log to {} subscribers: {}",
            subscriber_count,
            log_entry.message
        );

        self.log_subscribers
            .retain(|_id, sender| sender.send(log_entry.clone()).is_ok());
    }

    pub fn subscribe_logs(
        &self,
        subscriber_id: String,
    ) -> tokio::sync::mpsc::UnboundedReceiver<LogEntry> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.log_subscribers.insert(subscriber_id, tx);
        rx
    }

    pub fn unsubscribe_logs(&self, subscriber_id: &str) {
        self.log_subscribers.remove(subscriber_id);
    }
}
