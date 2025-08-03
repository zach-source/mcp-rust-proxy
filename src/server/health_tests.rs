#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::test_utils::*;
    use crate::protocol::{JsonRpcMessage, JsonRpcV2Message, JsonRpcResponse, JsonRpcId};
    use crate::config::{Config, ProxyConfig, WebUIConfig, HealthCheckConfig};
    use crate::state::AppState;
    use crate::transport::{Connection, TransportType};
    use bytes::Bytes;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::timeout;
    
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
            },
            servers: std::collections::HashMap::new(),
        }
    }
    
    #[tokio::test]
    async fn test_health_check_ping_success() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);
        
        // Create a mock connection that responds to ping
        let mock_conn = Arc::new(MockConnection::new());
        let conn_clone = mock_conn.clone();
        
        // Spawn a task to respond to ping
        tokio::spawn(async move {
            // Wait for the ping request
            let request = conn_clone.recv().await.unwrap();
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
                conn_clone.add_response(Bytes::from(format!("{}\n", response_json))).await;
            }
        });
        
        // Add mock connection to pool
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });
        
        state.connection_pool.add_server("test-server".to_string(), mock_transport).await.unwrap();
        
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
        let mock_conn = Arc::new(MockConnection::new());
        let conn_clone = mock_conn.clone();
        
        tokio::spawn(async move {
            let request = conn_clone.recv().await.unwrap();
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
                conn_clone.add_response(Bytes::from(format!("{}\n", response_json))).await;
            }
        });
        
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });
        
        state.connection_pool.add_server("test-server".to_string(), mock_transport).await.unwrap();
        
        let health_checker = HealthChecker::new("test-server".to_string(), state.clone());
        let result = health_checker.check_health().await;
        
        assert!(result.is_err(), "Health check should fail with error response");
    }
    
    #[tokio::test]
    async fn test_health_check_mismatched_id() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);
        
        // Create a mock connection that responds with wrong ID
        let mock_conn = Arc::new(MockConnection::new());
        let conn_clone = mock_conn.clone();
        
        tokio::spawn(async move {
            let request = conn_clone.recv().await.unwrap();
            let _request_str = std::str::from_utf8(&request).unwrap();
            
            // Send response with wrong ID
            let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(JsonRpcResponse {
                id: JsonRpcId::Number(999), // Wrong ID
                result: Some(json!({})),
                error: None,
            }));
            
            let response_json = serde_json::to_string(&response).unwrap();
            conn_clone.add_response(Bytes::from(format!("{}\n", response_json))).await;
        });
        
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });
        
        state.connection_pool.add_server("test-server".to_string(), mock_transport).await.unwrap();
        
        let health_checker = HealthChecker::new("test-server".to_string(), state.clone());
        let result = health_checker.check_health().await;
        
        assert!(result.is_err(), "Health check should fail with mismatched ID");
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
        
        state.connection_pool.add_server("test-server".to_string(), mock_transport).await.unwrap();
        
        let health_checker = HealthChecker::new("test-server".to_string(), state.clone());
        
        // This should timeout
        let result = timeout(Duration::from_millis(200), health_checker.check_health()).await;
        
        assert!(result.is_ok(), "Should complete within timeout");
        assert!(result.unwrap().is_err(), "Health check should fail due to no response");
    }
    
    #[tokio::test]
    async fn test_health_check_updates_server_state() {
        let mut config = create_test_config();
        config.health_check.interval_seconds = 1;
        let (state, mut shutdown_rx) = AppState::new(config);
        
        // Set initial state to Running
        state.register_server("test-server".to_string(), crate::state::ServerInfo {
            name: "test-server".to_string(),
            state: Arc::new(tokio::sync::RwLock::new(crate::state::ServerState::Running)),
            process_handle: None,
            restart_count: Arc::new(tokio::sync::RwLock::new(0)),
        }).await;
        
        // Create a mock connection that fails
        let mock_conn = Arc::new(MockConnection::new());
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });
        
        state.connection_pool.add_server("test-server".to_string(), mock_transport).await.unwrap();
        
        let health_checker = HealthChecker::new("test-server".to_string(), state.clone());
        
        // Run health checker in background
        let checker_handle = tokio::spawn(async move {
            health_checker.run().await;
        });
        
        // Wait a bit for health checks to run
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Check that server state changed to Failed
        let server_state = state.get_server_state("test-server").await;
        assert!(matches!(server_state, Some(crate::state::ServerState::Failed)));
        
        // Shutdown
        state.shutdown().await;
        let _ = timeout(Duration::from_secs(1), checker_handle).await;
    }
}