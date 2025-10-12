use mcp_rust_proxy::protocol::{ProtocolVersion, ServerConnectionState};
use serde_json::json;
use std::time::Duration;

/// Mock MCP server for testing initialization sequence
struct MockMcpServer {
    version: ProtocolVersion,
    response_delay: Option<Duration>,
}

impl MockMcpServer {
    fn new(version: ProtocolVersion) -> Self {
        Self {
            version,
            response_delay: None,
        }
    }

    fn with_delay(mut self, delay: Duration) -> Self {
        self.response_delay = Some(delay);
        self
    }

    async fn handle_initialize(&self, request: serde_json::Value) -> serde_json::Value {
        // Simulate delay if configured
        if let Some(delay) = self.response_delay {
            tokio::time::sleep(delay).await;
        }

        // Validate request structure
        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["method"], "initialize");

        // Return initialize response
        json!({
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {
                "protocolVersion": self.version.as_str(),
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "serverInfo": {
                    "name": "mock-server",
                    "version": "1.0.0"
                }
            }
        })
    }

    async fn expect_initialized_notification(&self, notification: serde_json::Value) {
        assert_eq!(notification["jsonrpc"], "2.0");
        assert_eq!(notification["method"], "notifications/initialized");
        assert!(notification["id"].is_null());
    }
}

#[tokio::test]
async fn test_send_initialize_request_to_mock_server() {
    let mock_server = MockMcpServer::new(ProtocolVersion::V20250326);

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-proxy",
                "version": "0.1.0"
            }
        }
    });

    let response = mock_server.handle_initialize(init_request).await;

    assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
    assert!(response["result"]["capabilities"].is_object());
}

#[tokio::test]
async fn test_receive_response_with_protocol_version() {
    let mock_server = MockMcpServer::new(ProtocolVersion::V20241105);

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": "init-1",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "proxy", "version": "1.0" }
        }
    });

    let response = mock_server.handle_initialize(init_request).await;

    let version_str = response["result"]["protocolVersion"].as_str().unwrap();
    let (version, is_supported) = ProtocolVersion::from_string(version_str);

    assert!(is_supported);
    assert_eq!(version, ProtocolVersion::V20241105);
}

#[tokio::test]
async fn test_send_initialized_notification() {
    let mock_server = MockMcpServer::new(ProtocolVersion::V20250618);

    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    mock_server
        .expect_initialized_notification(notification)
        .await;
}

#[tokio::test]
async fn test_state_transitions_to_ready() {
    let state = ServerConnectionState::new("test-server".to_string());

    // Initial state: Connecting
    assert!(!state.is_ready().await);

    // Transition to Initializing
    state
        .start_initialization("req-1".to_string())
        .await
        .unwrap();
    assert!(!state.is_ready().await);

    // Simulate receiving initialize response
    state
        .received_initialize_response(ProtocolVersion::V20250326)
        .await
        .unwrap();
    assert!(!state.is_ready().await);

    // Complete initialization
    state.complete_initialization().await.unwrap();
    assert!(state.is_ready().await);
}

#[tokio::test]
async fn test_subsequent_requests_allowed_after_ready() {
    let state = ServerConnectionState::new("test-server".to_string());

    // Complete initialization sequence
    state
        .start_initialization("req-1".to_string())
        .await
        .unwrap();
    state
        .received_initialize_response(ProtocolVersion::V20250326)
        .await
        .unwrap();
    state.complete_initialization().await.unwrap();

    // Now all requests should be allowed
    assert!(state.can_send_request("tools/list").await);
    assert!(state.can_send_request("resources/read").await);
    assert!(state.can_send_request("tools/call").await);
    assert!(state.can_send_request("prompts/list").await);
}

#[tokio::test]
async fn test_timeout_when_server_doesnt_respond() {
    let state = ServerConnectionState::new("test-server".to_string());

    state
        .start_initialization("req-1".to_string())
        .await
        .unwrap();

    // Simulate timeout scenario - server never responds
    tokio::time::sleep(Duration::from_millis(100)).await;

    // In a real implementation, the proxy would detect timeout and mark as failed
    // For now, verify state can transition to Failed
    state
        .mark_failed("Initialization timeout".to_string())
        .await;

    let current_state = state.get_state().await;
    match current_state {
        mcp_rust_proxy::protocol::state::ConnectionState::Failed { error, .. } => {
            assert_eq!(error, "Initialization timeout");
        }
        _ => panic!("Expected Failed state"),
    }
}

#[tokio::test]
async fn test_full_initialization_sequence_flow() {
    let mock_server = MockMcpServer::new(ProtocolVersion::V20250618);
    let state = ServerConnectionState::new("integration-test-server".to_string());

    // Step 1: Start initialization
    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

    // Step 2: Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": "init-1",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "proxy", "version": "1.0" }
        }
    });

    // Step 3: Receive response
    let response = mock_server.handle_initialize(init_request).await;
    let version_str = response["result"]["protocolVersion"].as_str().unwrap();
    let (version, _) = ProtocolVersion::from_string(version_str);

    // Step 4: Update state with protocol version
    state.received_initialize_response(version).await.unwrap();

    // Step 5: Send initialized notification
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    mock_server
        .expect_initialized_notification(notification)
        .await;

    // Step 6: Complete initialization
    state.complete_initialization().await.unwrap();

    // Verify final state
    assert!(state.is_ready().await);
    assert_eq!(
        state.protocol_version().await,
        Some(ProtocolVersion::V20250618)
    );
}

#[tokio::test]
async fn test_slow_server_initialization() {
    let mock_server =
        MockMcpServer::new(ProtocolVersion::V20250326).with_delay(Duration::from_millis(50));

    let state = ServerConnectionState::new("slow-server".to_string());
    state
        .start_initialization("req-1".to_string())
        .await
        .unwrap();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": "req-1",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "proxy", "version": "1.0" }
        }
    });

    // This should wait patiently for the response
    let start = std::time::Instant::now();
    let response = mock_server.handle_initialize(init_request).await;
    let elapsed = start.elapsed();

    // Verify it waited for the delay
    assert!(elapsed >= Duration::from_millis(50));

    // Verify response is valid
    assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
}
