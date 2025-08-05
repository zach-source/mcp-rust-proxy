use crate::error::HealthError;
use crate::protocol::{mcp, JsonRpcId, JsonRpcMessage, JsonRpcV2Message};
use crate::state::{AppState, HealthCheckStatus};
use chrono::Utc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::time::{interval, timeout, Duration, Instant};

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

        // Get effective health check configuration for this server
        let config = self.state.config.read().await;
        let effective_config = match config.get_server_health_check(&self.server_name) {
            Some(hc) => hc,
            None => {
                tracing::debug!("Health checks disabled for server: {}", self.server_name);
                return;
            }
        };

        let interval_duration = Duration::from_secs(effective_config.interval_seconds);
        let timeout_duration = Duration::from_secs(effective_config.timeout_seconds);
        let max_attempts = effective_config.max_attempts;
        let retry_interval = Duration::from_secs(effective_config.retry_interval_seconds);
        drop(config);

        let mut interval = interval(interval_duration);
        let mut consecutive_failures = 0;

        loop {
            interval.tick().await;

            // Check if we're shutting down
            if self.state.is_shutting_down() {
                tracing::debug!(
                    "Health checker for {} stopping due to shutdown",
                    self.server_name
                );
                break;
            }

            // Perform health check with retries
            let mut attempt = 0;
            let mut check_passed = false;

            let mut last_error = None;
            let mut response_time_ms = None;

            while attempt < max_attempts {
                if attempt > 0 {
                    tokio::time::sleep(retry_interval).await;
                }

                let start_time = Instant::now();
                let result = timeout(timeout_duration, self.check_health()).await;
                let elapsed = start_time.elapsed();

                match result {
                    Ok(Ok(_)) => {
                        check_passed = true;
                        response_time_ms = Some(elapsed.as_millis() as u64);
                        break;
                    }
                    Ok(Err(e)) => {
                        last_error = Some(format!("Health check failed: {}", e));
                        tracing::debug!(
                            "Health check attempt {} failed for server {}: {}",
                            attempt + 1,
                            self.server_name,
                            e
                        );
                    }
                    Err(_) => {
                        last_error = Some("Health check timed out".to_string());
                        tracing::debug!(
                            "Health check attempt {} timed out for server: {}",
                            attempt + 1,
                            self.server_name
                        );
                    }
                }

                attempt += 1;
            }

            // Update health check status
            if let Some(server_info) = self.state.servers.get(&self.server_name) {
                let mut health_status = server_info.last_health_check.write().await;
                *health_status = Some(HealthCheckStatus {
                    timestamp: Utc::now(),
                    success: check_passed,
                    response_time_ms,
                    error: if check_passed { None } else { last_error },
                });
            }

            if check_passed {
                tracing::trace!("Health check passed for server: {}", self.server_name);
                self.state.metrics.record_health_check(true);
                consecutive_failures = 0;
            } else {
                tracing::warn!(
                    "Health check failed for server {} after {} attempts",
                    self.server_name,
                    max_attempts
                );
                self.state.metrics.record_health_check(false);
                consecutive_failures += 1;

                // Only mark as unhealthy after multiple consecutive failures
                if consecutive_failures >= 2 {
                    self.handle_unhealthy_server().await;
                }
            }
        }
    }

    async fn check_health(&self) -> Result<(), HealthError> {
        // Get connection from pool
        let conn = self
            .state
            .connection_pool
            .get(&self.server_name)
            .await
            .map_err(|_| HealthError::Unhealthy)?;

        // Create ping request with unique ID
        let request_id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        let ping_request = mcp::create_ping_request(JsonRpcId::Number(request_id));

        // Serialize and send ping request
        let request_json =
            serde_json::to_string(&ping_request).map_err(|_| HealthError::InvalidResponse)?;
        let request_bytes = bytes::Bytes::from(format!("{}\n", request_json));

        tracing::debug!(
            "Sending ping request to {}: {}",
            self.server_name,
            request_json
        );
        conn.send(request_bytes)
            .await
            .map_err(|_| HealthError::Unhealthy)?;

        // Wait for response
        let response_bytes = conn.recv().await.map_err(|_| HealthError::Unhealthy)?;

        // Parse response
        let response_str =
            std::str::from_utf8(&response_bytes).map_err(|_| HealthError::InvalidResponse)?;
        tracing::debug!(
            "Received response from {}: {}",
            self.server_name,
            response_str.trim()
        );
        let response: JsonRpcMessage = serde_json::from_str(response_str.trim()).map_err(|e| {
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
                tracing::error!(
                    "Server {} is unhealthy, marking as failed",
                    self.server_name
                );

                // Mark server as failed
                if let Err(e) = self
                    .state
                    .set_server_state(&self.server_name, crate::state::ServerState::Failed)
                    .await
                {
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

// #[cfg(test)]
// #[path = "health_tracking_tests.rs"]
// mod health_tracking_tests; // TODO: Add test module
