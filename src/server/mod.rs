use std::sync::Arc;
use tokio::time::{sleep, Duration};
use crate::config::ServerConfig;
use crate::error::{Result, ServerError};
use crate::state::{AppState, ServerState};
use crate::transport::{create_transport, Transport};

pub mod lifecycle;
pub mod health;

pub use lifecycle::ServerManager;
pub use health::HealthChecker;

pub struct ManagedServer {
    pub name: String,
    pub config: ServerConfig,
    pub transport: Arc<dyn Transport>,
    pub state: Arc<AppState>,
}

impl ManagedServer {
    pub async fn new(
        name: String,
        config: ServerConfig,
        state: Arc<AppState>,
    ) -> Result<Self> {
        let transport = create_transport(&config.transport)?;
        
        Ok(Self {
            name,
            config,
            transport,
            state,
        })
    }

    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting server: {}", self.name);
        
        // Update state
        self.state.set_server_state(&self.name, ServerState::Starting).await?;
        
        // Create transport and add to connection pool
        match self.state.connection_pool.add_server(
            self.name.clone(),
            self.transport.clone(),
        ).await {
            Ok(_) => {
                tracing::info!("Server {} started successfully", self.name);
                self.state.set_server_state(&self.name, ServerState::Running).await?;
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to start server {}: {}", self.name, e);
                self.state.set_server_state(&self.name, ServerState::Failed).await?;
                Err(e)
            }
        }
    }

    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping server: {}", self.name);
        
        // Update state
        self.state.set_server_state(&self.name, ServerState::Stopping).await?;
        
        // Remove from connection pool
        self.state.connection_pool.remove(&self.name);
        
        // Update state
        self.state.set_server_state(&self.name, ServerState::Stopped).await?;
        
        tracing::info!("Server {} stopped", self.name);
        Ok(())
    }

    pub async fn restart(&self) -> Result<()> {
        tracing::info!("Restarting server: {}", self.name);
        
        // Stop the server
        self.stop().await?;
        
        // Wait before restarting
        sleep(Duration::from_millis(self.config.restart_delay_ms)).await;
        
        // Start the server
        self.start().await?;
        
        Ok(())
    }

    pub async fn handle_failure(&self, restart_count: u32) -> Result<()> {
        tracing::error!("Server {} failed (restart count: {})", self.name, restart_count);
        
        if !self.config.restart_on_failure {
            tracing::info!("Restart on failure disabled for server {}", self.name);
            return Ok(());
        }

        if restart_count >= self.config.max_restarts {
            tracing::error!(
                "Server {} exceeded maximum restart attempts ({})",
                self.name, self.config.max_restarts
            );
            self.state.set_server_state(&self.name, ServerState::Failed).await?;
            return Err(ServerError::Crashed(format!(
                "Exceeded maximum restart attempts: {}",
                self.config.max_restarts
            )).into());
        }

        // Wait before attempting restart
        let delay = Duration::from_millis(
            self.config.restart_delay_ms * (restart_count + 1) as u64
        );
        tracing::info!(
            "Waiting {:?} before restarting server {}",
            delay, self.name
        );
        sleep(delay).await;

        // Attempt restart
        match self.restart().await {
            Ok(_) => {
                tracing::info!("Server {} restarted successfully", self.name);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to restart server {}: {}", self.name, e);
                Err(e)
            }
        }
    }
}