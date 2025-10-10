use super::ManagedServer;
use crate::error::Result;
use crate::logging::ServerLogger;
use crate::state::{AppState, ServerInfo, ServerState};
use std::sync::Arc;

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
            // Create and register server info BEFORE creating ManagedServer
            let mut info = ServerInfo::new(name.clone());

            // Create logger for this server
            match ServerLogger::new(name.clone(), None).await {
                Ok(logger) => {
                    tracing::info!("Created logger for server: {}", name);
                    info.set_logger(Arc::new(logger)).await;
                }
                Err(e) => {
                    tracing::error!("Failed to create logger for server {}: {}", name, e);
                }
            }

            self.state.register_server(name.clone(), info).await;

            let server =
                ManagedServer::new(name.clone(), server_config.clone(), self.state.clone()).await?;

            // Start server in background task
            let state = self.state.clone();
            let server = Arc::new(server);
            let name = name.clone();

            tokio::spawn(async move {
                if let Err(e) = server.start().await {
                    tracing::error!("Failed to start server {}: {}", name, e);
                    let _ = state.set_server_state(&name, ServerState::Failed).await;
                } else {
                    // Start health checker if enabled for this server
                    let config = state.config.read().await;
                    if config.get_server_health_check(&name).is_some() {
                        let health_checker = super::HealthChecker::new(name.clone(), state.clone());

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
        let servers: Vec<_> = self
            .state
            .servers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        // Stop all servers in parallel
        let mut stop_tasks = Vec::new();
        for name in servers {
            let state = self.state.clone();
            let name_clone = name.clone();

            stop_tasks.push(tokio::spawn(async move {
                // First remove from connection pool to prevent new connections
                state.connection_pool.remove(&name_clone);

                // Then close existing connections
                if let Err(e) = Self::force_stop_server(&state, &name_clone).await {
                    tracing::error!("Failed to stop server {}: {}", name_clone, e);
                }
            }));
        }

        // Wait for all servers to stop
        for task in stop_tasks {
            let _ = task.await;
        }

        Ok(())
    }

    async fn force_stop_server(state: &Arc<AppState>, name: &str) -> Result<()> {
        // Update state
        let _ = state.set_server_state(name, ServerState::Stopping).await;

        // Connection pool already handles closing connections
        tracing::info!("Server {} stopped", name);

        // Update final state
        let _ = state.set_server_state(name, ServerState::Stopped).await;

        Ok(())
    }

    pub async fn start_server(&self, name: &str) -> Result<()> {
        let config = self.state.config.read().await;

        if let Some(server_config) = config.servers.get(name) {
            let server =
                ManagedServer::new(name.to_string(), server_config.clone(), self.state.clone())
                    .await?;

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
                    )
                    .await?;

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
