use mcp_rust_proxy::*;
use mcp_rust_proxy::test_utils::*;
use mcp_rust_proxy::protocol::{JsonRpcMessage, JsonRpcV2Message, JsonRpcRequest, JsonRpcResponse, JsonRpcId, JsonRpcNotification};
use mcp_rust_proxy::config::{Config, ProxyConfig, WebUiConfig, HealthCheckConfig, ServerConfig, TransportConfig};
use mcp_rust_proxy::state::AppState;
use mcp_rust_proxy::proxy::ProxyServer;
use mcp_rust_proxy::server::ServerManager;
use bytes::Bytes;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn create_test_proxy_config() -> Config {
    Config {
        proxy: ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // Let OS assign port
            auth_token: None,
        },
        web_ui: WebUiConfig {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 0,
            auth_token: None,
        },
        health_check: HealthCheckConfig {
            enabled: false, // Disable for integration tests
            interval_ms: 5000,
            timeout_ms: 1000,
            unhealthy_threshold: 3,
        },
        servers: HashMap::new(),
        logging: Default::default(),
        metrics: Default::default(),
    }
}

#[tokio::test]
async fn test_client_request_passthrough() {
    let mut config = create_test_proxy_config().await;
    
    // Add a test server with mock transport
    let mock_transport = Arc::new(MockTransport::new());
    let mock_conn = mock_transport.connection.clone();
    
    // Set up mock server to echo requests back
    let server_conn = mock_conn.clone();
    tokio::spawn(async move {
        loop {
            match server_conn.recv().await {
                Ok(request) => {
                    // Parse request
                    let request_str = std::str::from_utf8(&request).unwrap();
                    if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim()) {
                        if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                            // Echo back with result
                            let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(JsonRpcResponse {
                                id: req.id,
                                result: Some(json!({
                                    "echo": req.method,
                                    "params": req.params
                                })),
                                error: None,
                            }));
                            
                            let response_json = format!("{}\n", serde_json::to_string(&response).unwrap());
                            server_conn.add_response(Bytes::from(response_json)).await;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Configure server
    config.servers.insert("test-server".to_string(), ServerConfig {
        command: "mock".to_string(),
        args: vec![],
        env: HashMap::new(),
        transport: TransportConfig::Stdio,
        restart_on_failure: false,
        working_directory: None,
        max_restarts: 0,
        restart_delay_ms: 1000,
    });
    
    let (state, mut shutdown_rx) = AppState::new(config.clone());
    
    // Manually add the mock transport to the connection pool
    state.connection_pool.add_server("test-server".to_string(), mock_transport).await.unwrap();
    
    // Start proxy server
    let proxy = ProxyServer::new(state.clone());
    let proxy_handle = tokio::spawn(async move {
        proxy.run().await
    });
    
    // Wait for proxy to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Get the actual port the proxy is listening on
    let proxy_port = state.config.read().await.proxy.port;
    
    // Connect as a client
    let mut client = TcpStream::connect(format!("127.0.0.1:{}", proxy_port)).await.unwrap();
    
    // Send a request
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    
    let request_str = format!("{}\n", serde_json::to_string(&request).unwrap());
    client.write_all(request_str.as_bytes()).await.unwrap();
    
    // Read response
    let mut buffer = vec![0u8; 1024];
    let n = client.read(&mut buffer).await.unwrap();
    let response_str = std::str::from_utf8(&buffer[..n]).unwrap();
    
    // Parse and verify response
    let response: JsonRpcMessage = serde_json::from_str(response_str.trim()).unwrap();
    if let JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) = response {
        assert_eq!(resp.id, JsonRpcId::Number(1));
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
        
        let result = resp.result.unwrap();
        assert_eq!(result["echo"], "tools/list");
    } else {
        panic!("Expected response message");
    }
    
    // Cleanup
    state.shutdown().await;
    let _ = tokio::time::timeout(Duration::from_secs(1), proxy_handle).await;
}

#[tokio::test]
async fn test_request_cancellation() {
    let mut config = create_test_proxy_config().await;
    
    // Add a test server that delays responses
    let mock_transport = Arc::new(MockTransport::new());
    let mock_conn = mock_transport.connection.clone();
    
    // Set up mock server to handle cancellation
    let server_conn = mock_conn.clone();
    let (cancel_tx, mut cancel_rx) = tokio::sync::mpsc::channel::<i64>(10);
    
    tokio::spawn(async move {
        let mut pending_requests = HashMap::new();
        
        loop {
            tokio::select! {
                // Handle incoming requests
                result = server_conn.recv() => {
                    match result {
                        Ok(request) => {
                            let request_str = std::str::from_utf8(&request).unwrap();
                            if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim()) {
                                match msg {
                                    JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) => {
                                        if req.method == "slow-operation" {
                                            // Store the request and wait
                                            if let JsonRpcId::Number(id) = req.id {
                                                pending_requests.insert(id, req);
                                                
                                                // Simulate slow operation
                                                let conn = server_conn.clone();
                                                let pending = pending_requests.clone();
                                                tokio::spawn(async move {
                                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                                    
                                                    // Check if still pending (not cancelled)
                                                    if pending.contains_key(&id) {
                                                        let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(JsonRpcResponse {
                                                            id: JsonRpcId::Number(id),
                                                            result: Some(json!({"status": "completed"})),
                                                            error: None,
                                                        }));
                                                        
                                                        let response_json = format!("{}\n", serde_json::to_string(&response).unwrap());
                                                        conn.add_response(Bytes::from(response_json)).await;
                                                    }
                                                });
                                            }
                                        }
                                    }
                                    JsonRpcMessage::V2(JsonRpcV2Message::Notification(notif)) => {
                                        if notif.method == "notifications/cancelled" {
                                            if let Some(params) = notif.params {
                                                if let Some(request_id) = params["requestId"].as_i64() {
                                                    // Remove cancelled request
                                                    pending_requests.remove(&request_id);
                                                    let _ = cancel_tx.send(request_id).await;
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });
    
    config.servers.insert("test-server".to_string(), ServerConfig {
        command: "mock".to_string(),
        args: vec![],
        env: HashMap::new(),
        transport: TransportConfig::Stdio,
        restart_on_failure: false,
        working_directory: None,
        max_restarts: 0,
        restart_delay_ms: 1000,
    });
    
    let (state, _) = AppState::new(config);
    state.connection_pool.add_server("test-server".to_string(), mock_transport).await.unwrap();
    
    // Start proxy
    let proxy = ProxyServer::new(state.clone());
    let proxy_handle = tokio::spawn(async move {
        proxy.run().await
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let proxy_port = state.config.read().await.proxy.port;
    let mut client = TcpStream::connect(format!("127.0.0.1:{}", proxy_port)).await.unwrap();
    
    // Send a slow request
    let request = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "slow-operation",
        "params": {}
    });
    
    let request_str = format!("{}\n", serde_json::to_string(&request).unwrap());
    client.write_all(request_str.as_bytes()).await.unwrap();
    
    // Send cancellation after a short delay
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let cancel_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancelled",
        "params": {
            "requestId": 42,
            "reason": "User cancelled"
        }
    });
    
    let cancel_str = format!("{}\n", serde_json::to_string(&cancel_notification).unwrap());
    client.write_all(cancel_str.as_bytes()).await.unwrap();
    
    // Verify cancellation was received
    let cancelled_id = tokio::time::timeout(Duration::from_secs(1), cancel_rx.recv()).await;
    assert!(cancelled_id.is_ok());
    assert_eq!(cancelled_id.unwrap().unwrap(), 42);
    
    // Cleanup
    state.shutdown().await;
    let _ = tokio::time::timeout(Duration::from_secs(1), proxy_handle).await;
}

#[tokio::test]
async fn test_multiple_concurrent_clients() {
    let mut config = create_test_proxy_config().await;
    
    // Add test server
    let mock_transport = Arc::new(MockTransport::new());
    let mock_conn = mock_transport.connection.clone();
    
    // Echo server
    tokio::spawn(async move {
        let mut request_count = 0;
        loop {
            match mock_conn.recv().await {
                Ok(request) => {
                    request_count += 1;
                    let request_str = std::str::from_utf8(&request).unwrap();
                    if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim()) {
                        if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                            let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(JsonRpcResponse {
                                id: req.id,
                                result: Some(json!({
                                    "client": req.params.as_ref().and_then(|p| p.get("client_id")),
                                    "request_count": request_count
                                })),
                                error: None,
                            }));
                            
                            let response_json = format!("{}\n", serde_json::to_string(&response).unwrap());
                            mock_conn.add_response(Bytes::from(response_json)).await;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    config.servers.insert("test-server".to_string(), ServerConfig {
        command: "mock".to_string(),
        args: vec![],
        env: HashMap::new(),
        transport: TransportConfig::Stdio,
        restart_on_failure: false,
        working_directory: None,
        max_restarts: 0,
        restart_delay_ms: 1000,
    });
    
    let (state, _) = AppState::new(config);
    state.connection_pool.add_server("test-server".to_string(), mock_transport).await.unwrap();
    
    let proxy = ProxyServer::new(state.clone());
    let proxy_handle = tokio::spawn(async move {
        proxy.run().await
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let proxy_port = state.config.read().await.proxy.port;
    
    // Connect multiple clients concurrently
    let mut handles = vec![];
    
    for client_id in 0..5 {
        let handle = tokio::spawn(async move {
            let mut client = TcpStream::connect(format!("127.0.0.1:{}", proxy_port)).await.unwrap();
            
            // Send request
            let request = json!({
                "jsonrpc": "2.0",
                "id": client_id,
                "method": "test",
                "params": {
                    "client_id": client_id
                }
            });
            
            let request_str = format!("{}\n", serde_json::to_string(&request).unwrap());
            client.write_all(request_str.as_bytes()).await.unwrap();
            
            // Read response
            let mut buffer = vec![0u8; 1024];
            let n = client.read(&mut buffer).await.unwrap();
            let response_str = std::str::from_utf8(&buffer[..n]).unwrap();
            
            let response: JsonRpcMessage = serde_json::from_str(response_str.trim()).unwrap();
            if let JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) = response {
                assert_eq!(resp.id, JsonRpcId::Number(client_id));
                assert!(resp.result.is_some());
                
                let result = resp.result.unwrap();
                assert_eq!(result["client"], client_id);
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all clients to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Cleanup
    state.shutdown().await;
    let _ = tokio::time::timeout(Duration::from_secs(1), proxy_handle).await;
}

#[tokio::test]
async fn test_server_routing() {
    let mut config = create_test_proxy_config().await;
    
    // Add two test servers
    let server1_transport = Arc::new(MockTransport::new());
    let server1_conn = server1_transport.connection.clone();
    
    let server2_transport = Arc::new(MockTransport::new());
    let server2_conn = server2_transport.connection.clone();
    
    // Server 1 handles "tools/*" methods
    tokio::spawn(async move {
        loop {
            match server1_conn.recv().await {
                Ok(request) => {
                    let request_str = std::str::from_utf8(&request).unwrap();
                    if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim()) {
                        if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                            if req.method.starts_with("tools/") {
                                let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(JsonRpcResponse {
                                    id: req.id,
                                    result: Some(json!({"server": "server1", "method": req.method})),
                                    error: None,
                                }));
                                
                                let response_json = format!("{}\n", serde_json::to_string(&response).unwrap());
                                server1_conn.add_response(Bytes::from(response_json)).await;
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Server 2 handles "resources/*" methods
    tokio::spawn(async move {
        loop {
            match server2_conn.recv().await {
                Ok(request) => {
                    let request_str = std::str::from_utf8(&request).unwrap();
                    if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim()) {
                        if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                            if req.method.starts_with("resources/") {
                                let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(JsonRpcResponse {
                                    id: req.id,
                                    result: Some(json!({"server": "server2", "method": req.method})),
                                    error: None,
                                }));
                                
                                let response_json = format!("{}\n", serde_json::to_string(&response).unwrap());
                                server2_conn.add_response(Bytes::from(response_json)).await;
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    config.servers.insert("server1".to_string(), ServerConfig {
        command: "mock1".to_string(),
        args: vec![],
        env: HashMap::new(),
        transport: TransportConfig::Stdio,
        restart_on_failure: false,
        working_directory: None,
        max_restarts: 0,
        restart_delay_ms: 1000,
    });
    
    config.servers.insert("server2".to_string(), ServerConfig {
        command: "mock2".to_string(),
        args: vec![],
        env: HashMap::new(),
        transport: TransportConfig::Stdio,
        restart_on_failure: false,
        working_directory: None,
        max_restarts: 0,
        restart_delay_ms: 1000,
    });
    
    let (state, _) = AppState::new(config);
    
    // Add servers to pool and register routing
    state.connection_pool.add_server("server1".to_string(), server1_transport).await.unwrap();
    state.connection_pool.add_server("server2".to_string(), server2_transport).await.unwrap();
    
    // Register routing rules
    state.request_router.register_tool("tools/list".to_string(), "server1".to_string());
    state.request_router.register_resource("resources/list".to_string(), "server2".to_string());
    
    let proxy = ProxyServer::new(state.clone());
    let proxy_handle = tokio::spawn(async move {
        proxy.run().await
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let proxy_port = state.config.read().await.proxy.port;
    let mut client = TcpStream::connect(format!("127.0.0.1:{}", proxy_port)).await.unwrap();
    
    // Test request to server1
    let request1 = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    
    let request_str = format!("{}\n", serde_json::to_string(&request1).unwrap());
    client.write_all(request_str.as_bytes()).await.unwrap();
    
    let mut buffer = vec![0u8; 1024];
    let n = client.read(&mut buffer).await.unwrap();
    let response_str = std::str::from_utf8(&buffer[..n]).unwrap();
    
    let response: JsonRpcMessage = serde_json::from_str(response_str.trim()).unwrap();
    if let JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) = response {
        let result = resp.result.unwrap();
        assert_eq!(result["server"], "server1");
    }
    
    // Test request to server2
    let request2 = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "resources/list",
        "params": {}
    });
    
    let request_str = format!("{}\n", serde_json::to_string(&request2).unwrap());
    client.write_all(request_str.as_bytes()).await.unwrap();
    
    let n = client.read(&mut buffer).await.unwrap();
    let response_str = std::str::from_utf8(&buffer[..n]).unwrap();
    
    let response: JsonRpcMessage = serde_json::from_str(response_str.trim()).unwrap();
    if let JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) = response {
        let result = resp.result.unwrap();
        assert_eq!(result["server"], "server2");
    }
    
    // Cleanup
    state.shutdown().await;
    let _ = tokio::time::timeout(Duration::from_secs(1), proxy_handle).await;
}