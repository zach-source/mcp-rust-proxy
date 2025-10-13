use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

use super::adapter::ProtocolAdapter;
use super::error::ProtocolError;
use super::version::ProtocolVersion;

/// Connection state enum representing the initialization sequence
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection being established
    Connecting,

    /// Sent initialize request, waiting for response
    Initializing {
        request_id: String,
        started_at: Instant,
    },

    /// Received initialize response, sending initialized notification
    SendingInitialized { protocol_version: ProtocolVersion },

    /// Fully initialized and ready for normal requests
    Ready {
        protocol_version: ProtocolVersion,
        initialized_at: Instant,
    },

    /// Connection failed during initialization
    Failed { error: String, failed_at: Instant },

    /// Connection is being closed
    Closing,
}

/// Manages the connection state and protocol adapter for a backend server
pub struct ServerConnectionState {
    /// Current connection state
    state: Arc<Mutex<ConnectionState>>,

    /// Server name for logging
    server_name: String,

    /// Protocol adapter for this connection
    adapter: Arc<RwLock<Option<Arc<dyn ProtocolAdapter>>>>,

    /// Last activity timestamp
    last_activity: Arc<Mutex<Instant>>,
}

impl ServerConnectionState {
    pub fn new(server_name: String) -> Self {
        Self {
            state: Arc::new(Mutex::new(ConnectionState::Connecting)),
            server_name,
            adapter: Arc::new(RwLock::new(None)),
            last_activity: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> ConnectionState {
        self.state.lock().await.clone()
    }

    /// Transition to Initializing state
    pub async fn start_initialization(&self, request_id: String) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        match *state {
            ConnectionState::Connecting => {
                *state = ConnectionState::Initializing {
                    request_id,
                    started_at: Instant::now(),
                };
                *self.last_activity.lock().await = Instant::now();
                Ok(())
            }
            _ => Err(ProtocolError::InvalidStateTransition {
                from: format!("{:?}", *state),
                to: "Initializing".to_string(),
            }),
        }
    }

    /// Transition to SendingInitialized state
    pub async fn received_initialize_response(
        &self,
        protocol_version: ProtocolVersion,
    ) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        match *state {
            ConnectionState::Initializing { .. } => {
                *state = ConnectionState::SendingInitialized { protocol_version };
                *self.last_activity.lock().await = Instant::now();
                Ok(())
            }
            _ => Err(ProtocolError::InvalidStateTransition {
                from: format!("{:?}", *state),
                to: "SendingInitialized".to_string(),
            }),
        }
    }

    /// Transition to Ready state
    pub async fn complete_initialization(&self) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        match *state {
            ConnectionState::SendingInitialized { protocol_version } => {
                *state = ConnectionState::Ready {
                    protocol_version,
                    initialized_at: Instant::now(),
                };
                *self.last_activity.lock().await = Instant::now();

                // T053: Log successful version negotiation
                tracing::info!(
                    server_name = %self.server_name,
                    protocol_version = %protocol_version.as_str(),
                    "Protocol version negotiated successfully"
                );

                // T055: Warn about deprecated versions
                if protocol_version.is_deprecated() {
                    tracing::warn!(
                        server_name = %self.server_name,
                        protocol_version = %protocol_version.as_str(),
                        "Server is using deprecated protocol version 2024-11-05. Consider upgrading to 2025-06-18."
                    );
                }

                Ok(())
            }
            _ => Err(ProtocolError::InvalidStateTransition {
                from: format!("{:?}", *state),
                to: "Ready".to_string(),
            }),
        }
    }

    /// Mark as failed
    pub async fn mark_failed(&self, error: String) {
        let mut state = self.state.lock().await;
        *state = ConnectionState::Failed {
            error,
            failed_at: Instant::now(),
        };
        *self.last_activity.lock().await = Instant::now();
    }

    /// Check if server is ready to handle requests
    pub async fn is_ready(&self) -> bool {
        matches!(*self.state.lock().await, ConnectionState::Ready { .. })
    }

    /// Get protocol version (only available when Ready or SendingInitialized)
    pub async fn protocol_version(&self) -> Option<ProtocolVersion> {
        match *self.state.lock().await {
            ConnectionState::Ready {
                protocol_version, ..
            }
            | ConnectionState::SendingInitialized { protocol_version } => Some(protocol_version),
            _ => None,
        }
    }

    /// Check if a request can be sent in current state
    pub async fn can_send_request(&self, method: &str) -> bool {
        let state = self.state.lock().await;
        match (method, &*state) {
            // Initialize can only be sent when connecting
            ("initialize", ConnectionState::Connecting) => true,
            ("initialize", _) => false, // initialize not allowed in any other state

            // All other requests require Ready state
            (_, ConnectionState::Ready { .. }) => true,

            // No other requests allowed in other states
            _ => false,
        }
    }

    /// Set the protocol adapter for this connection
    pub async fn set_adapter(&self, adapter: Arc<dyn ProtocolAdapter>) {
        let mut guard = self.adapter.write().await;
        *guard = Some(adapter);
    }

    /// Get the protocol adapter (if initialized)
    pub async fn get_adapter(&self) -> Option<Arc<dyn ProtocolAdapter>> {
        let guard = self.adapter.read().await;
        guard.as_ref().cloned()
    }

    /// Get server name
    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initial_state_is_connecting() {
        let state = ServerConnectionState::new("test-server".to_string());
        assert!(matches!(
            state.get_state().await,
            ConnectionState::Connecting
        ));
    }

    #[tokio::test]
    async fn test_valid_transition_connecting_to_initializing() {
        let state = ServerConnectionState::new("test-server".to_string());
        let result = state.start_initialization("req-1".to_string()).await;

        assert!(result.is_ok());
        match state.get_state().await {
            ConnectionState::Initializing { request_id, .. } => {
                assert_eq!(request_id, "req-1");
            }
            _ => panic!("Expected Initializing state"),
        }
    }

    #[tokio::test]
    async fn test_valid_transition_initializing_to_sending_initialized() {
        let state = ServerConnectionState::new("test-server".to_string());
        state
            .start_initialization("req-1".to_string())
            .await
            .unwrap();

        let result = state
            .received_initialize_response(ProtocolVersion::V20250326)
            .await;

        assert!(result.is_ok());
        match state.get_state().await {
            ConnectionState::SendingInitialized { protocol_version } => {
                assert_eq!(protocol_version, ProtocolVersion::V20250326);
            }
            _ => panic!("Expected SendingInitialized state"),
        }
    }

    #[tokio::test]
    async fn test_valid_transition_sending_initialized_to_ready() {
        let state = ServerConnectionState::new("test-server".to_string());
        state
            .start_initialization("req-1".to_string())
            .await
            .unwrap();
        state
            .received_initialize_response(ProtocolVersion::V20250326)
            .await
            .unwrap();

        let result = state.complete_initialization().await;

        assert!(result.is_ok());
        match state.get_state().await {
            ConnectionState::Ready {
                protocol_version, ..
            } => {
                assert_eq!(protocol_version, ProtocolVersion::V20250326);
            }
            _ => panic!("Expected Ready state"),
        }
    }

    #[tokio::test]
    async fn test_invalid_transition_returns_error() {
        let state = ServerConnectionState::new("test-server".to_string());
        // Try to go directly to Ready without going through the proper sequence
        let result = state.complete_initialization().await;

        assert!(result.is_err());
        match result {
            Err(ProtocolError::InvalidStateTransition { .. }) => {}
            _ => panic!("Expected InvalidStateTransition error"),
        }
    }

    #[tokio::test]
    async fn test_protocol_version_set_during_received_initialize_response() {
        let state = ServerConnectionState::new("test-server".to_string());
        state
            .start_initialization("req-1".to_string())
            .await
            .unwrap();
        state
            .received_initialize_response(ProtocolVersion::V20250618)
            .await
            .unwrap();

        let version = state.protocol_version().await;
        assert_eq!(version, Some(ProtocolVersion::V20250618));
    }

    #[tokio::test]
    async fn test_can_send_request_returns_correct_values() {
        let state = ServerConnectionState::new("test-server".to_string());

        // In Connecting state, only initialize is allowed
        assert!(state.can_send_request("initialize").await);
        assert!(!state.can_send_request("tools/list").await);

        // Transition to Ready
        state
            .start_initialization("req-1".to_string())
            .await
            .unwrap();
        state
            .received_initialize_response(ProtocolVersion::V20250326)
            .await
            .unwrap();
        state.complete_initialization().await.unwrap();

        // In Ready state, all methods are allowed
        assert!(state.can_send_request("tools/list").await);
        assert!(state.can_send_request("resources/read").await);
        assert!(state.can_send_request("tools/call").await);
    }

    #[tokio::test]
    async fn test_is_ready_only_true_in_ready_state() {
        let state = ServerConnectionState::new("test-server".to_string());

        assert!(!state.is_ready().await);

        state
            .start_initialization("req-1".to_string())
            .await
            .unwrap();
        assert!(!state.is_ready().await);

        state
            .received_initialize_response(ProtocolVersion::V20250326)
            .await
            .unwrap();
        assert!(!state.is_ready().await);

        state.complete_initialization().await.unwrap();
        assert!(state.is_ready().await);
    }

    #[tokio::test]
    async fn test_failed_state_transition() {
        let state = ServerConnectionState::new("test-server".to_string());
        state.mark_failed("Connection timeout".to_string()).await;

        match state.get_state().await {
            ConnectionState::Failed { error, .. } => {
                assert_eq!(error, "Connection timeout");
            }
            _ => panic!("Expected Failed state"),
        }
    }

    #[tokio::test]
    async fn test_state_machine_concurrent_access() {
        let state = Arc::new(ServerConnectionState::new("test-server".to_string()));

        let state1 = Arc::clone(&state);
        let state2 = Arc::clone(&state);

        let handle1 = tokio::spawn(async move {
            for _ in 0..100 {
                let _ = state1.is_ready().await;
            }
        });

        let handle2 = tokio::spawn(async move {
            for _ in 0..100 {
                let _ = state2.protocol_version().await;
            }
        });

        let _ = tokio::join!(handle1, handle2);
        // If we get here without panicking, concurrent access works
    }
}
