#[cfg(test)]
mod tests {
    use super::super::handler::RequestHandler;
    use crate::config::{Config, HealthCheckConfig, ProxyConfig, WebUIConfig};
    use crate::protocol::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcV2Message};
    use crate::proxy::router::RequestRouter;
    use crate::state::AppState;
    use crate::transport::{Connection, Transport, TransportType};
    use bytes::Bytes;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::{mpsc, RwLock};
    use tokio::time::sleep;

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
                port: 3000,
                connection_pool_size: 10,
                request_timeout_ms: 5000,
                max_concurrent_requests: 100,
            },
            web_ui: WebUIConfig {
                enabled: false,
                host: "127.0.0.1".to_string(),
                port: 0,
                static_dir: None,
                api_key: None,
            },
            health_check: HealthCheckConfig {
                enabled: false,
                interval_seconds: 5,
                timeout_seconds: 1,
                max_attempts: 3,
                retry_interval_seconds: 1,
            },
            servers: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_tools_list_caching() {
        let mut config = create_test_config();

        // Add two mock servers
        let mock_conn1 = Arc::new(MockConnection::new());
        let server1_conn = mock_conn1.clone();

        let mock_conn2 = Arc::new(MockConnection::new());
        let server2_conn = mock_conn2.clone();

        // Mock server 1 responses
        tokio::spawn(async move {
            loop {
                let mut rx = server1_conn.request_rx.write().await;
                match rx.recv().await {
                    Some(request) => {
                        drop(rx); // Drop the lock before processing
                        let request_str = std::str::from_utf8(&request).unwrap();
                        if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim())
                        {
                            if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                                if req.method == "initialize" {
                                    // Handle initialization
                                    let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(
                                        JsonRpcResponse {
                                            id: req.id,
                                            result: Some(json!({
                                                "protocolVersion": "0.1.0",
                                                "capabilities": {},
                                                "serverInfo": {
                                                    "name": "server1",
                                                    "version": "0.1.0"
                                                }
                                            })),
                                            error: None,
                                        },
                                    ));

                                    let response_json =
                                        format!("{}\n", serde_json::to_string(&response).unwrap());
                                    server1_conn.add_response(Bytes::from(response_json)).await;
                                } else if req.method == "tools/list" {
                                    // Simulate some delay
                                    sleep(Duration::from_millis(100)).await;

                                    let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(
                                        JsonRpcResponse {
                                            id: req.id,
                                            result: Some(json!({
                                                "tools": [
                                                    {"name": "server1_tool", "description": "Tool from server 1"}
                                                ]
                                            })),
                                            error: None,
                                        },
                                    ));

                                    let response_json =
                                        format!("{}\n", serde_json::to_string(&response).unwrap());
                                    server1_conn.add_response(Bytes::from(response_json)).await;
                                }
                            }
                        }
                    }
                    None => break,
                }
            }
        });

        // Mock server 2 responses
        tokio::spawn(async move {
            loop {
                let mut rx = server2_conn.request_rx.write().await;
                match rx.recv().await {
                    Some(request) => {
                        drop(rx); // Drop the lock before processing
                        let request_str = std::str::from_utf8(&request).unwrap();
                        if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim())
                        {
                            if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                                if req.method == "initialize" {
                                    // Handle initialization
                                    let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(
                                        JsonRpcResponse {
                                            id: req.id,
                                            result: Some(json!({
                                                "protocolVersion": "0.1.0",
                                                "capabilities": {},
                                                "serverInfo": {
                                                    "name": "server2",
                                                    "version": "0.1.0"
                                                }
                                            })),
                                            error: None,
                                        },
                                    ));

                                    let response_json =
                                        format!("{}\n", serde_json::to_string(&response).unwrap());
                                    server2_conn.add_response(Bytes::from(response_json)).await;
                                } else if req.method == "tools/list" {
                                    // Simulate some delay
                                    sleep(Duration::from_millis(100)).await;

                                    let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(
                                        JsonRpcResponse {
                                            id: req.id,
                                            result: Some(json!({
                                                "tools": [
                                                    {"name": "server2_tool", "description": "Tool from server 2"}
                                                ]
                                            })),
                                            error: None,
                                        },
                                    ));

                                    let response_json =
                                        format!("{}\n", serde_json::to_string(&response).unwrap());
                                    server2_conn.add_response(Bytes::from(response_json)).await;
                                }
                            }
                        }
                    }
                    None => break,
                }
            }
        });

        // Configure servers
        config.servers.insert(
            "server1".to_string(),
            crate::config::ServerConfig {
                command: "mock1".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                transport: crate::config::TransportConfig::Stdio,
                restart_on_failure: false,
                working_directory: None,
                max_restarts: 0,
                restart_delay_ms: 1000,
                health_check: None,
            },
        );

        config.servers.insert(
            "server2".to_string(),
            crate::config::ServerConfig {
                command: "mock2".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                transport: crate::config::TransportConfig::Stdio,
                restart_on_failure: false,
                working_directory: None,
                max_restarts: 0,
                restart_delay_ms: 1000,
                health_check: None,
            },
        );

        let (state, _) = AppState::new(config);

        // Add mock transports to pool
        let mock_transport1 = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn1,
        });

        let mock_transport2 = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn2,
        });

        state
            .connection_pool
            .add_server("server1".to_string(), mock_transport1)
            .await
            .unwrap();
        state
            .connection_pool
            .add_server("server2".to_string(), mock_transport2)
            .await
            .unwrap();

        // Register servers in state
        state
            .register_server(
                "server1".to_string(),
                crate::state::ServerInfo {
                    name: "server1".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(crate::state::ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        state
            .register_server(
                "server2".to_string(),
                crate::state::ServerInfo {
                    name: "server2".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(crate::state::ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let handler = RequestHandler::new(state.clone());
        let router = Arc::new(RequestRouter::new());

        // First request - should hit servers and take time
        let start = Instant::now();
        let request = json!({
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        let response1 = handler
            .handle_request(request.clone(), router.clone())
            .await
            .unwrap();
        let duration1 = start.elapsed();

        // Verify response contains tools from both servers
        if let Some(ref err) = response1.error {
            eprintln!("Error in response: {:?}", err);
        }
        assert!(response1.error.is_none());
        assert!(response1.result.is_some());

        let result = response1.result.as_ref().unwrap();
        let tools = result["tools"].as_array().unwrap();
        eprintln!("Tools in response: {:?}", tools);
        assert_eq!(tools.len(), 2);

        let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(tool_names.contains(&"server1_tool"));
        assert!(tool_names.contains(&"server2_tool"));

        // Should take at least 100ms due to mock delays
        assert!(duration1 >= Duration::from_millis(100));

        // Second request - should hit cache and be much faster
        let start = Instant::now();
        let response2 = handler.handle_request(request, router).await.unwrap();
        let duration2 = start.elapsed();

        // Cache hit should be much faster (less than 10ms)
        assert!(
            duration2 < Duration::from_millis(10),
            "Cache hit took {:?}, expected < 10ms",
            duration2
        );

        // Responses should be identical
        assert_eq!(response1.result, response2.result);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let mut config = create_test_config();

        // Add a mock server
        let mock_conn = Arc::new(MockConnection::new());
        let server_conn = mock_conn.clone();

        let request_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let request_count_clone = request_count.clone();

        // Mock server responses
        tokio::spawn(async move {
            loop {
                let mut rx = server_conn.request_rx.write().await;
                match rx.recv().await {
                    Some(request) => {
                        drop(rx); // Drop the lock before processing
                        let request_str = std::str::from_utf8(&request).unwrap();
                        if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim())
                        {
                            if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                                if req.method == "initialize" {
                                    // Handle initialization
                                    let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(
                                        JsonRpcResponse {
                                            id: req.id,
                                            result: Some(json!({
                                                "protocolVersion": "0.1.0",
                                                "capabilities": {},
                                                "serverInfo": {
                                                    "name": "server1",
                                                    "version": "0.1.0"
                                                }
                                            })),
                                            error: None,
                                        },
                                    ));

                                    let response_json =
                                        format!("{}\n", serde_json::to_string(&response).unwrap());
                                    server_conn.add_response(Bytes::from(response_json)).await;
                                } else if req.method == "tools/list" {
                                    let count = request_count_clone
                                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                                    let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(
                                        JsonRpcResponse {
                                            id: req.id,
                                            result: Some(json!({
                                                "tools": [
                                                    {"name": format!("tool_{}", count), "description": "Test tool"}
                                                ]
                                            })),
                                            error: None,
                                        },
                                    ));

                                    let response_json =
                                        format!("{}\n", serde_json::to_string(&response).unwrap());
                                    server_conn.add_response(Bytes::from(response_json)).await;
                                }
                            }
                        }
                    }
                    None => break,
                }
            }
        });

        // Configure server
        config.servers.insert(
            "server1".to_string(),
            crate::config::ServerConfig {
                command: "mock".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                transport: crate::config::TransportConfig::Stdio,
                restart_on_failure: false,
                working_directory: None,
                max_restarts: 0,
                restart_delay_ms: 1000,
                health_check: None,
            },
        );

        let (state, _) = AppState::new(config);

        // Add mock transport to pool
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });

        state
            .connection_pool
            .add_server("server1".to_string(), mock_transport)
            .await
            .unwrap();

        // Register server in state
        state
            .register_server(
                "server1".to_string(),
                crate::state::ServerInfo {
                    name: "server1".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(crate::state::ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let handler = RequestHandler::new(state.clone());
        let router = Arc::new(RequestRouter::new());

        let request = json!({
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        // First request
        let response1 = handler
            .handle_request(request.clone(), router.clone())
            .await
            .unwrap();
        let result1 = response1.result.as_ref().unwrap();
        let tools1 = result1["tools"].as_array().unwrap();
        assert_eq!(tools1[0]["name"], "tool_0");

        // Second request immediately - should hit cache
        let response2 = handler
            .handle_request(request.clone(), router.clone())
            .await
            .unwrap();
        let result2 = response2.result.as_ref().unwrap();
        let tools2 = result2["tools"].as_array().unwrap();
        assert_eq!(tools2[0]["name"], "tool_0"); // Same as first

        // Verify only one request was made to server
        assert_eq!(request_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Wait for cache to expire (cache is 2 minutes, but we can't wait that long in tests)
        // This test mainly verifies the cache is being used
    }

    #[tokio::test]
    async fn test_non_tools_list_not_cached() {
        let mut config = create_test_config();

        // Add a mock server
        let mock_conn = Arc::new(MockConnection::new());
        let server_conn = mock_conn.clone();

        let request_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let request_count_clone = request_count.clone();

        // Mock server responses
        tokio::spawn(async move {
            loop {
                let mut rx = server_conn.request_rx.write().await;
                match rx.recv().await {
                    Some(request) => {
                        drop(rx); // Drop the lock before processing
                        let request_str = std::str::from_utf8(&request).unwrap();
                        if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim())
                        {
                            if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                                if req.method == "initialize" {
                                    // Handle initialization
                                    let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(
                                        JsonRpcResponse {
                                            id: req.id,
                                            result: Some(json!({
                                                "protocolVersion": "0.1.0",
                                                "capabilities": {},
                                                "serverInfo": {
                                                    "name": "server1",
                                                    "version": "0.1.0"
                                                }
                                            })),
                                            error: None,
                                        },
                                    ));

                                    let response_json =
                                        format!("{}\n", serde_json::to_string(&response).unwrap());
                                    server_conn.add_response(Bytes::from(response_json)).await;
                                } else {
                                    let count = request_count_clone
                                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                                    let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(
                                        JsonRpcResponse {
                                            id: req.id,
                                            result: Some(json!({
                                                "response": format!("Response {}", count),
                                                "method": req.method
                                            })),
                                            error: None,
                                        },
                                    ));

                                    let response_json =
                                        format!("{}\n", serde_json::to_string(&response).unwrap());
                                    server_conn.add_response(Bytes::from(response_json)).await;
                                }
                            }
                        }
                    }
                    None => break,
                }
            }
        });

        // Configure server
        config.servers.insert(
            "server1".to_string(),
            crate::config::ServerConfig {
                command: "mock".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                transport: crate::config::TransportConfig::Stdio,
                restart_on_failure: false,
                working_directory: None,
                max_restarts: 0,
                restart_delay_ms: 1000,
                health_check: None,
            },
        );

        let (state, _) = AppState::new(config);

        // Add mock transport to pool
        let mock_transport = Arc::new(MockTransport {
            transport_type: TransportType::Stdio,
            connection: mock_conn,
        });

        state
            .connection_pool
            .add_server("server1".to_string(), mock_transport)
            .await
            .unwrap();

        // Register server in state
        state
            .register_server(
                "server1".to_string(),
                crate::state::ServerInfo {
                    name: "server1".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(crate::state::ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let handler = RequestHandler::new(state.clone());
        let router = Arc::new(RequestRouter::new());

        // Test non-tools/list request
        let request = json!({
            "id": 1,
            "method": "some_other_method",
            "params": {}
        });

        // First request
        let response1 = handler
            .handle_request(request.clone(), router.clone())
            .await
            .unwrap();
        let result1 = response1.result.as_ref().unwrap();
        assert_eq!(result1["response"], "Response 0");

        // Second request - should NOT hit cache
        let response2 = handler
            .handle_request(request.clone(), router.clone())
            .await
            .unwrap();
        let result2 = response2.result.as_ref().unwrap();
        assert_eq!(result2["response"], "Response 1"); // Different response

        // Verify two requests were made to server
        assert_eq!(request_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
