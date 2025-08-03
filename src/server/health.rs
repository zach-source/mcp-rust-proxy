use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::time::{interval, timeout, Duration};
use crate::state::AppState;
use crate::error::HealthError;
use crate::protocol::{JsonRpcMessage, JsonRpcV2Message, JsonRpcId, mcp};

pub struct HealthChecker {
    server_name: String,
    state: Arc<AppState>,
    request_id_counter: AtomicI64,
}

impl HealthChecker {
    pub fn new(server_name: String, state: Arc<AppState>) -> Self {
        Self {
            server_name,
            state,
            request_id_counter: AtomicI64::new(0),
        }
    }

    pub async fn run(self) {
        // Give servers time to fully initialize after the handshake
        tokio::time::sleep(Duration::from_secs(10)).await;
        
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
        
        // Create ping request with unique ID
        let request_id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        let ping_request = mcp::create_ping_request(JsonRpcId::Number(request_id));
        
        // Serialize and send ping request
        let request_json = serde_json::to_string(&ping_request)
            .map_err(|_| HealthError::InvalidResponse)?;
        let request_bytes = bytes::Bytes::from(format!("{}\n", request_json));
        
        tracing::debug!("Sending ping request to {}: {}", self.server_name, request_json);
        conn.send(request_bytes)
            .await
            .map_err(|_| HealthError::Unhealthy)?;
        
        // Wait for response
        let response_bytes = conn.recv()
            .await
            .map_err(|_| HealthError::Unhealthy)?;
        
        // Parse response
        let response_str = std::str::from_utf8(&response_bytes)
            .map_err(|_| HealthError::InvalidResponse)?;
        tracing::debug!("Received response from {}: {}", self.server_name, response_str.trim());
        let response: JsonRpcMessage = serde_json::from_str(response_str.trim())
            .map_err(|e| {
                tracing::error!("Failed to parse response from {}: {}", self.server_name, e);
                HealthError::InvalidResponse
            })?;
        
        // Check if it's a valid ping response
        match response {
            JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) => {
                match resp.id {
                    JsonRpcId::Number(id) if id == request_id => {
                        // Check if response has error
                        if resp.error.is_some() {
                            return Err(HealthError::Unhealthy);
                        }
                        // Valid ping response
                        Ok(())
                    }
                    _ => {
                        tracing::warn!("Received response with mismatched ID for ping");
                        Err(HealthError::InvalidResponse)
                    }
                }
            }
            _ => {
                tracing::warn!("Received non-response message for ping");
                Err(HealthError::InvalidResponse)
            }
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

#[cfg(test)]
#[path = "health_tests.rs"]
mod health_tests;