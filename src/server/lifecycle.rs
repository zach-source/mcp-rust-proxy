use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::Result;
use crate::state::{AppState, ServerInfo, ServerState};
use super::ManagedServer;

pub struct ServerManager {
    state: Arc<AppState>,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
}

impl ServerManager {
    pub fn new(state: Arc<AppState>, shutdown_rx: tokio::sync::broadcast::Receiver<()>) -> Self {
        Self { state, shutdown_rx }
    }

    pub async fn run(mut self) -> Result<()> {
        tracing::info!("Server manager started");
        
        // Start all configured servers
        self.start_all_servers().await?;
        
        // Wait for shutdown signal
        let _ = self.shutdown_rx.recv().await;
        
        // Stop all servers
        self.stop_all_servers().await?;
        
        tracing::info!("Server manager stopped");
        Ok(())
    }

    async fn start_all_servers(&self) -> Result<()> {
        let config = self.state.config.read().await;
        
        for (name, server_config) in &config.servers {
            let server = ManagedServer::new(
                name.clone(),
                server_config.clone(),
                self.state.clone(),
            ).await?;
            
            // Create server info
            let info = ServerInfo {
                name: name.clone(),
                state: Arc::new(RwLock::new(ServerState::Stopped)),
                process_handle: None,
                restart_count: Arc::new(RwLock::new(0)),
            };
            
            // Register server
            self.state.register_server(name.clone(), info).await;
            
            // Start server in background task
            let state = self.state.clone();
            let server = Arc::new(server);
            let name = name.clone();
            
            tokio::spawn(async move {
                if let Err(e) = server.start().await {
                    tracing::error!("Failed to start server {}: {}", name, e);
                    let _ = state.set_server_state(&name, ServerState::Failed).await;
                } else {
                    // Start health checker if enabled
                    if state.config.read().await.health_check.enabled {
                        let health_checker = super::HealthChecker::new(
                            name.clone(),
                            state.clone(),
                        );
                        
                        tokio::spawn(async move {
                            health_checker.run().await;
                        });
                    }
                }
            });
        }
        
        Ok(())
    }

    async fn stop_all_servers(&self) -> Result<()> {
        let servers: Vec<_> = self.state.servers.iter()
            .map(|entry| entry.key().clone())
            .collect();
        
        for name in servers {
            if let Err(e) = self.stop_server(&name).await {
                tracing::error!("Failed to stop server {}: {}", name, e);
            }
        }
        
        Ok(())
    }

    pub async fn start_server(&self, name: &str) -> Result<()> {
        let config = self.state.config.read().await;
        
        if let Some(server_config) = config.servers.get(name) {
            let server = ManagedServer::new(
                name.to_string(),
                server_config.clone(),
                self.state.clone(),
            ).await?;
            
            server.start().await
        } else {
            Err(crate::error::ProxyError::ServerNotFound(name.to_string()))
        }
    }

    pub async fn stop_server(&self, name: &str) -> Result<()> {
        if let Some(info) = self.state.servers.get(name) {
            let state = info.state.read().await;
            if *state == ServerState::Running {
                drop(state);
                
                let config = self.state.config.read().await;
                if let Some(server_config) = config.servers.get(name) {
                    let server = ManagedServer::new(
                        name.to_string(),
                        server_config.clone(),
                        self.state.clone(),
                    ).await?;
                    
                    server.stop().await
                } else {
                    Err(crate::error::ProxyError::ServerNotFound(name.to_string()))
                }
            } else {
                Ok(())
            }
        } else {
            Err(crate::error::ProxyError::ServerNotFound(name.to_string()))
        }
    }

    pub async fn restart_server(&self, name: &str) -> Result<()> {
        self.stop_server(name).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        self.start_server(name).await
    }
}