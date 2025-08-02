use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::Config;
use crate::transport::pool::ConnectionPool;
use crate::error::Result;

pub mod metrics;

pub use metrics::Metrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub servers: Arc<DashMap<String, ServerInfo>>,
    pub metrics: Arc<Metrics>,
    pub connection_pool: Arc<ConnectionPool>,
    pub shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

#[derive(Clone)]
pub struct ServerInfo {
    pub name: String,
    pub state: Arc<RwLock<ServerState>>,
    pub process_handle: Option<Arc<tokio::task::JoinHandle<()>>>,
    pub restart_count: Arc<RwLock<u32>>,
}

impl AppState {
    pub fn new(config: Config) -> (Arc<Self>, tokio::sync::broadcast::Receiver<()>) {
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(16);
        
        let state = Arc::new(Self {
            config: Arc::new(RwLock::new(config)),
            servers: Arc::new(DashMap::new()),
            metrics: Arc::new(Metrics::new()),
            connection_pool: Arc::new(ConnectionPool::new()),
            shutdown_tx,
        });
        
        (state, shutdown_rx)
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
}