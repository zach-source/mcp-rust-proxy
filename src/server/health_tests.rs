#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::config::{Config, HealthCheckConfig, ProxyConfig, WebUIConfig};
    use crate::protocol::{JsonRpcMessage, JsonRpcResponse, JsonRpcV2Message};
    use crate::state::AppState;
    use crate::transport::{Connection, Transport, TransportType};
    use bytes::Bytes;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{mpsc, RwLock};
    use tokio::time::timeout;

    // Mock connection for testing
    struct MockConnection {
        request_tx: mpsc::UnboundedSender<Bytes>,
        request_rx: Arc<RwLock<mpsc::UnboundedReceiver<Bytes>>>,
        response_tx: mpsc::UnboundedSender<Bytes>,
        response_rx: Arc<RwLock<mpsc::UnboundedReceiver<Bytes>>>,
    }

    impl MockConnection {
        fn new() -> Self {
            let (request_tx, request_rx) = mpsc::unbounded_channel();
            let (response_tx, response_rx) = mpsc::unbounded_channel();
            Self {
                request_tx,
                request_rx: Arc::new(RwLock::new(request_rx)),
                response_tx,
                response_rx: Arc::new(RwLock::new(response_rx)),
            }
        }

        async fn add_response(&self, data: Bytes) {
            let _ = self.response_tx.send(data);
        }

        fn with_auto_initialize(self: Arc<Self>) -> Arc<Self> {
            let conn = self.clone();
            tokio::spawn(async move {
                loop {
                    let mut rx = conn.request_rx.write().await;
                    match rx.recv().await {
                        Some(request) => {
                            drop(rx); // Drop the lock before processing
                            let request_str = std::str::from_utf8(&request).unwrap();
                            if let Ok(msg) =
                                serde_json::from_str::<JsonRpcMessage>(request_str.trim())
                            {
                                if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                                    if req.method == "initialize" {
                                        // Send initialize response
                                        let response = JsonRpcMessage::V2(
                                            JsonRpcV2Message::Response(JsonRpcResponse {
                                                id: req.id,
                                                result: Some(json!({
                                                    "protocolVersion": "0.1.0",
                                                    "capabilities": {},
                                                    "serverInfo": {
                                                        "name": "mock-server",
                                                        "version": "0.1.0"
                                                    }
                                                })),
                                                error: None,
                                            }),
                                        );

                                        let response_json = format!(
                                            "{}\n",
                                            serde_json::to_string(&response).unwrap()
                                        );
                                        conn.add_response(Bytes::from(response_json)).await;
                                    }
                                }
                            }
                        }
                        None => break,
                    }
                }
            });
            self
        }
    }

    #[async_trait::async_trait]
    impl Connection for MockConnection {
        async fn send(&self, data: Bytes) -> crate::error::Result<()> {
            let _ = self.request_tx.send(data);
            Ok(())
        }

        async fn recv(&self) -> crate::error::Result<Bytes> {
            let mut rx = self.response_rx.write().await;
            rx.recv().await.ok_or_else(|| {
                crate::error::TransportError::ConnectionFailed("Connection closed".to_string())
                    .into()
            })
        }

        async fn close(&self) -> crate::error::Result<()> {
            Ok(())
        }

        fn is_closed(&self) -> bool {
            false
        }
    }

    struct MockTransport {
        transport_type: TransportType,
        connection: Arc<MockConnection>,
    }

    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn connect(&self) -> crate::error::Result<Arc<dyn Connection>> {
            Ok(self.connection.clone() as Arc<dyn Connection>)
        }

        fn transport_type(&self) -> TransportType {
            self.transport_type.clone()
        }
    }

    fn create_test_config() -> Config {
        Config {
            proxy: ProxyConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                connection_pool_size: 10,
                request_timeout_ms: 5000,
                max_concurrent_requests: 100,
            },
            web_ui: WebUIConfig {
                enabled: false,
                host: "127.0.0.1".to_string(),
                port: 8081,
                static_dir: None,
                api_key: None,
            },
            health_check: HealthCheckConfig {
                enabled: true,
                interval_seconds: 1, // Fast for testing
                timeout_seconds: 1,
                max_attempts: 3,
                retry_interval_seconds: 1,
            },
            servers: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_health_check_ping_success() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Create a mock connection that responds to ping
        let mock_conn = Arc::new(MockConnection::new()).with_auto_initialize();
        let conn_clone = mock_conn.clone();

        // Spawn a task to respond to ping
        tokio::spawn(async move {
            // Wait for the ping request
            let mut rx = conn_clone.request_rx.write().await;
            let request = rx.recv().await.unwrap();
            drop(rx); // Drop the lock before processing
            let request_str = std::str::from_utf8(&request).unwrap();
            let request_msg: JsonRpcMessage = serde_json::from_str(request_str.trim()).unwrap();

            // Verify it's a ping request
            if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = request_msg {
                assert_eq!(req.method, "ping");

                // Send ping response
                let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(JsonRpcResponse {
                    id: req.id,
                    result: Some(json!({})),
                    error: None,
                }));

                let response_json = serde_json::to_string(&response).unwrap();
                conn_clone
                    .add_response(Bytes::from(format!("{}\n", response_json)))
                    .await;
            }
        });

        // Add mock connection to pool
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });

        state
            .connection_pool
            .add_server("test-server".to_string(), mock_transport)
            .await
            .unwrap();

        // Create and run health check
        let health_checker = HealthChecker::new("test-server".to_string(), state.clone());

        // Perform a single health check
        let result = health_checker.check_health().await;
        assert!(result.is_ok(), "Health check should succeed");
    }

    #[tokio::test]
    async fn test_health_check_ping_error_response() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Create a mock connection that responds with error
        let mock_conn = Arc::new(MockConnection::new()).with_auto_initialize();
        let conn_clone = mock_conn.clone();

        tokio::spawn(async move {
            let mut rx = conn_clone.request_rx.write().await;
            let request = rx.recv().await.unwrap();
            drop(rx); // Drop the lock before processing
            let request_str = std::str::from_utf8(&request).unwrap();
            let request_msg: JsonRpcMessage = serde_json::from_str(request_str.trim()).unwrap();

            if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = request_msg {
                // Send error response
                let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(JsonRpcResponse {
                    id: req.id,
                    result: None,
                    error: Some(crate::protocol::JsonRpcError {
                        code: -32601,
                        message: "Method not found".to_string(),
                        data: None,
                    }),
                }));

                let response_json = serde_json::to_string(&response).unwrap();
                conn_clone
                    .add_response(Bytes::from(format!("{}\n", response_json)))
                    .await;
            }
        });

        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });

        state
            .connection_pool
            .add_server("test-server".to_string(), mock_transport)
            .await
            .unwrap();

        let health_checker = HealthChecker::new("test-server".to_string(), state.clone());
        let result = health_checker.check_health().await;

        assert!(
            result.is_err(),
            "Health check should fail with error response"
        );
    }

    #[tokio::test]
    async fn test_health_check_mismatched_id() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Create a mock connection that responds with wrong ID
        let mock_conn = Arc::new(MockConnection::new());
        let conn_clone = mock_conn.clone();

        tokio::spawn(async move {
            let mut rx = conn_clone.request_rx.write().await;
            let request = rx.recv().await.unwrap();
            drop(rx); // Drop the lock before processing
            let _request_str = std::str::from_utf8(&request).unwrap();

            // Send response with wrong ID
            let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(JsonRpcResponse {
                id: crate::protocol::JsonRpcId::Number(999), // Wrong ID
                result: Some(json!({})),
                error: None,
            }));

            let response_json = serde_json::to_string(&response).unwrap();
            conn_clone
                .add_response(Bytes::from(format!("{}\n", response_json)))
                .await;
        });

        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });

        state
            .connection_pool
            .add_server("test-server".to_string(), mock_transport)
            .await
            .unwrap();

        let health_checker = HealthChecker::new("test-server".to_string(), state.clone());
        let result = health_checker.check_health().await;

        assert!(
            result.is_err(),
            "Health check should fail with mismatched ID"
        );
    }

    #[tokio::test]
    async fn test_health_check_timeout() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Create a mock connection that never responds
        let mock_conn = Arc::new(MockConnection::new());

        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });

        state
            .connection_pool
            .add_server("test-server".to_string(), mock_transport)
            .await
            .unwrap();

        let health_checker = HealthChecker::new("test-server".to_string(), state.clone());

        // This should timeout
        let result = timeout(Duration::from_millis(200), health_checker.check_health()).await;

        assert!(result.is_ok(), "Should complete within timeout");
        assert!(
            result.unwrap().is_err(),
            "Health check should fail due to no response"
        );
    }

    #[tokio::test]
    async fn test_health_check_updates_server_state() {
        let mut config = create_test_config();
        config.health_check.interval_seconds = 1;
        config.health_check.max_attempts = 1; // Fail fast for testing
        let (state, _shutdown_rx) = AppState::new(config);

        // Set initial state to Running
        state
            .register_server(
                "test-server".to_string(),
                crate::state::ServerInfo {
                    name: "test-server".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(crate::state::ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        // Create a mock connection that fails
        let mock_conn = Arc::new(MockConnection::new());
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });

        state
            .connection_pool
            .add_server("test-server".to_string(), mock_transport)
            .await
            .unwrap();

        let health_checker = HealthChecker::new("test-server".to_string(), state.clone());

        // Run health checker in background
        let checker_handle = tokio::spawn(async move {
            health_checker.run().await;
        });

        // Wait a bit for health checks to run
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Check that server state changed to Failed
        let server_state = state.get_server_state("test-server").await;
        assert!(matches!(
            server_state,
            Some(crate::state::ServerState::Failed)
        ));

        // Shutdown
        state.shutdown().await;
        let _ = timeout(Duration::from_secs(1), checker_handle).await;
    }
}
