use std::sync::Arc;
use tokio::time::{interval, timeout};
use crate::state::AppState;
use crate::error::HealthError;

pub struct HealthChecker {
    server_name: String,
    state: Arc<AppState>,
}

impl HealthChecker {
    pub fn new(server_name: String, state: Arc<AppState>) -> Self {
        Self {
            server_name,
            state,
        }
    }

    pub async fn run(self) {
        let config = self.state.config.read().await;
        let interval_duration = config.health_check_interval();
        let timeout_duration = config.health_check_timeout();
        drop(config);
        
        let mut interval = interval(interval_duration);
        
        loop {
            interval.tick().await;
            
            // Check if we're shutting down
            if self.state.is_shutting_down() {
                tracing::debug!("Health checker for {} stopping due to shutdown", self.server_name);
                break;
            }
            
            // Perform health check
            let result = timeout(
                timeout_duration,
                self.check_health()
            ).await;
            
            match result {
                Ok(Ok(_)) => {
                    tracing::trace!("Health check passed for server: {}", self.server_name);
                    self.state.metrics.record_health_check(true);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Health check failed for server {}: {}", self.server_name, e);
                    self.state.metrics.record_health_check(false);
                    self.handle_unhealthy_server().await;
                }
                Err(_) => {
                    tracing::warn!("Health check timed out for server: {}", self.server_name);
                    self.state.metrics.record_health_check(false);
                    self.handle_unhealthy_server().await;
                }
            }
        }
    }

    async fn check_health(&self) -> Result<(), HealthError> {
        // Get connection from pool
        let conn = self.state.connection_pool.get(&self.server_name)
            .await
            .map_err(|_| HealthError::Unhealthy)?;
        
        // Send health check request
        let health_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "health",
            "params": {},
            "id": 1
        });
        
        conn.send(bytes::Bytes::from(health_request.to_string()))
            .await
            .map_err(|_| HealthError::Unhealthy)?;
        
        // Wait for response
        let response = conn.recv()
            .await
            .map_err(|_| HealthError::Unhealthy)?;
        
        // Parse response
        let response: serde_json::Value = serde_json::from_slice(&response)
            .map_err(|_| HealthError::InvalidResponse)?;
        
        // Check if healthy
        if response["result"]["status"] == "healthy" {
            Ok(())
        } else {
            Err(HealthError::Unhealthy)
        }
    }

    async fn handle_unhealthy_server(&self) {
        // Get current server state
        if let Some(state) = self.state.get_server_state(&self.server_name).await {
            if state == crate::state::ServerState::Running {
                tracing::error!("Server {} is unhealthy, marking as failed", self.server_name);
                
                // Mark server as failed
                if let Err(e) = self.state.set_server_state(
                    &self.server_name,
                    crate::state::ServerState::Failed
                ).await {
                    tracing::error!("Failed to update server state: {}", e);
                }
                
                // TODO: Trigger restart if configured
            }
        }
    }
}