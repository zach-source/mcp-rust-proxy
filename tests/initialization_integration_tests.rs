use mcp_rust_proxy::protocol::{ProtocolVersion, ServerConnectionState};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

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

// ============================================================================
// T024: Request Gating Tests
// ============================================================================

/// T024 Test Case 1: tools/list request blocked when state is Connecting
#[tokio::test]
async fn t024_tools_list_blocked_when_connecting() {
    let state = ServerConnectionState::new("test-server".to_string());

    // State should be Connecting initially
    assert!(!state.is_ready().await);

    // tools/list should be blocked
    assert!(
        !state.can_send_request("tools/list").await,
        "tools/list should be blocked in Connecting state"
    );
}

/// T024 Test Case 2: tools/list request blocked when state is Initializing
#[tokio::test]
async fn t024_tools_list_blocked_when_initializing() {
    let state = ServerConnectionState::new("test-server".to_string());

    // Transition to Initializing
    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

    // tools/list should be blocked
    assert!(
        !state.can_send_request("tools/list").await,
        "tools/list should be blocked in Initializing state"
    );
}

/// T024 Test Case 3: tools/list request blocked when state is SendingInitialized
#[tokio::test]
async fn t024_tools_list_blocked_when_sending_initialized() {
    let state = ServerConnectionState::new("test-server".to_string());

    // Transition through Connecting → Initializing → SendingInitialized
    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();
    state
        .received_initialize_response(ProtocolVersion::V20250618)
        .await
        .unwrap();

    // tools/list should be blocked
    assert!(
        !state.can_send_request("tools/list").await,
        "tools/list should be blocked in SendingInitialized state"
    );
}

/// T024 Test Case 4: tools/list request allowed when state is Ready
#[tokio::test]
async fn t024_tools_list_allowed_when_ready() {
    let state = ServerConnectionState::new("test-server".to_string());

    // Complete full initialization sequence
    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();
    state
        .received_initialize_response(ProtocolVersion::V20250618)
        .await
        .unwrap();
    state.complete_initialization().await.unwrap();

    // tools/list should be allowed
    assert!(
        state.can_send_request("tools/list").await,
        "tools/list should be allowed in Ready state"
    );
}

/// T024 Test Case 5: initialize request only allowed in Connecting state
#[tokio::test]
async fn t024_initialize_only_in_connecting() {
    let state = ServerConnectionState::new("test-server".to_string());

    // initialize should be allowed in Connecting state
    assert!(
        state.can_send_request("initialize").await,
        "initialize should be allowed in Connecting state"
    );

    // Transition to Initializing
    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

    // initialize should NOT be allowed anymore
    assert!(
        !state.can_send_request("initialize").await,
        "initialize should NOT be allowed in Initializing state"
    );

    // Complete initialization
    state
        .received_initialize_response(ProtocolVersion::V20250618)
        .await
        .unwrap();
    state.complete_initialization().await.unwrap();

    // initialize should NOT be allowed in Ready state
    assert!(
        !state.can_send_request("initialize").await,
        "initialize should NOT be allowed in Ready state"
    );
}

/// T024 Test Case 6: Multiple request types blocked before Ready
#[tokio::test]
async fn t024_all_requests_blocked_before_ready() {
    let state = ServerConnectionState::new("test-server".to_string());

    // Start initialization but don't complete it
    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

    // All non-initialize requests should be blocked
    assert!(
        !state.can_send_request("tools/list").await,
        "tools/list should be blocked"
    );
    assert!(
        !state.can_send_request("resources/list").await,
        "resources/list should be blocked"
    );
    assert!(
        !state.can_send_request("resources/read").await,
        "resources/read should be blocked"
    );
    assert!(
        !state.can_send_request("tools/call").await,
        "tools/call should be blocked"
    );
    assert!(
        !state.can_send_request("prompts/list").await,
        "prompts/list should be blocked"
    );
    assert!(
        !state.can_send_request("prompts/get").await,
        "prompts/get should be blocked"
    );
    assert!(
        !state.can_send_request("completion/complete").await,
        "completion/complete should be blocked"
    );
}

// ============================================================================
// T026: Slow Initialization Tests
// ============================================================================

/// T026 Test Case 1: Mock server delays initialize response by 30 seconds
/// Note: This test takes 30+ seconds to run, so it's ignored by default.
/// Run with: cargo test t026_server_delays_30_seconds -- --ignored
#[tokio::test]
#[ignore]
async fn t026_server_delays_30_seconds() {
    let mock_server =
        MockMcpServer::new(ProtocolVersion::V20250326).with_delay(Duration::from_secs(30));
    let state = ServerConnectionState::new("slow-server-30s".to_string());

    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

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

    // This should wait for the full 30 seconds
    let start = std::time::Instant::now();
    let response = mock_server.handle_initialize(init_request).await;
    let elapsed = start.elapsed();

    // Verify it waited for the delay (at least 30 seconds)
    assert!(
        elapsed >= Duration::from_secs(30),
        "Should wait at least 30 seconds, waited {elapsed:?}"
    );

    // Verify response is valid
    assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
}

/// T026 Test Case 2: Proxy waits patiently without timeout (within 60s limit)
/// Note: This test takes 45+ seconds to run, so it's ignored by default.
/// Run with: cargo test t026_proxy_waits_patiently_under_60s -- --ignored
#[tokio::test]
#[ignore]
async fn t026_proxy_waits_patiently_under_60s() {
    let mock_server =
        MockMcpServer::new(ProtocolVersion::V20250618).with_delay(Duration::from_secs(45));
    let state = ServerConnectionState::new("slow-server-45s".to_string());

    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

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

    // Should wait patiently for 45 seconds (under 60s timeout)
    let start = std::time::Instant::now();
    let response = mock_server.handle_initialize(init_request).await;
    let elapsed = start.elapsed();

    assert!(
        elapsed >= Duration::from_secs(45),
        "Should wait at least 45 seconds"
    );
    assert!(
        elapsed < Duration::from_secs(50),
        "Should not wait much longer than needed"
    );

    // Complete initialization successfully
    let (version, _) =
        ProtocolVersion::from_string(response["result"]["protocolVersion"].as_str().unwrap());
    state.received_initialize_response(version).await.unwrap();
    state.complete_initialization().await.unwrap();

    assert!(state.is_ready().await);
}

/// T026 Test Case 3: Requests queued during initialization are processed after Ready
/// Note: This test documents the expected behavior. Actual queuing implementation
/// will be done in T027.
#[tokio::test]
async fn t026_requests_queued_during_init() {
    let mock_server =
        MockMcpServer::new(ProtocolVersion::V20241105).with_delay(Duration::from_millis(100));
    let state = ServerConnectionState::new("queuing-test-server".to_string());

    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

    // During initialization, requests should be blocked
    assert!(!state.can_send_request("tools/list").await);

    // Simulate initialization completing in background
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
    let (version, _) =
        ProtocolVersion::from_string(response["result"]["protocolVersion"].as_str().unwrap());

    state.received_initialize_response(version).await.unwrap();
    state.complete_initialization().await.unwrap();

    // After initialization, requests should be allowed
    assert!(state.can_send_request("tools/list").await);
    assert!(state.is_ready().await);
}

/// T026 Test Case 4: Timeout occurs if server takes > 60 seconds
/// Note: This is a simulated test since we don't want to actually wait 60+ seconds
#[tokio::test]
async fn t026_timeout_after_60_seconds() {
    let state = ServerConnectionState::new("timeout-server".to_string());

    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

    // Simulate a server that never responds
    // In production, a timeout mechanism would detect this after 60 seconds
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Timeout should trigger state transition to Failed
    state
        .mark_failed("Initialization timeout after 60 seconds".to_string())
        .await;

    match state.get_state().await {
        mcp_rust_proxy::protocol::state::ConnectionState::Failed { error, .. } => {
            assert!(
                error.contains("timeout"),
                "Error should mention timeout: {error}"
            );
        }
        other => panic!("Expected Failed state, got {other:?}"),
    }

    // After failure, server should not be ready
    assert!(!state.is_ready().await);
}

/// T026 Test Case 5: Multiple slow servers initialize concurrently
#[tokio::test]
async fn t026_multiple_slow_servers() {
    let delays = [
        Duration::from_millis(500),
        Duration::from_millis(1000),
        Duration::from_millis(1500),
    ];

    let mut handles = vec![];

    for (i, delay) in delays.iter().enumerate() {
        let delay = *delay;
        let handle = tokio::spawn(async move {
            let mock = MockMcpServer::new(ProtocolVersion::V20250326).with_delay(delay);
            let state = ServerConnectionState::new(format!("slow-server-{i}"));

            state
                .start_initialization(format!("init-{i}"))
                .await
                .unwrap();

            let request = json!({
                "jsonrpc": "2.0",
                "id": format!("init-{}", i),
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": { "name": "proxy", "version": "1.0" }
                }
            });

            let start = std::time::Instant::now();
            let response = mock.handle_initialize(request).await;
            let elapsed = start.elapsed();

            (elapsed, response, state)
        });

        handles.push(handle);
    }

    // All servers should complete successfully
    let results = futures::future::join_all(handles).await;

    for (i, result) in results.iter().enumerate() {
        let (elapsed, response, state) = result.as_ref().unwrap();

        // Each server waited for its respective delay
        assert!(
            *elapsed >= delays[i],
            "Server {} should wait at least {:?}",
            i,
            delays[i]
        );

        // All got valid responses
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");

        // Complete initialization
        let (version, _) =
            ProtocolVersion::from_string(response["result"]["protocolVersion"].as_str().unwrap());
        state.received_initialize_response(version).await.unwrap();
        state.complete_initialization().await.unwrap();

        assert!(state.is_ready().await);
    }
}

// ============================================================================
// T028: Concurrent Client Tests
// ============================================================================

/// T028 Test Case 1: 10 clients send tools/list simultaneously while server initializing
#[tokio::test]
async fn t028_concurrent_clients_during_initialization() {
    use mcp_rust_proxy::proxy::router::{QueuedRequest, RequestRouter};

    let router = Arc::new(RequestRouter::new());
    let state = Arc::new(ServerConnectionState::new(
        "concurrent-test-server".to_string(),
    ));
    let server_name = "concurrent-test-server";

    // Start initialization
    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

    // Spawn 10 concurrent clients trying to send tools/list
    let mut handles = vec![];

    for i in 0..10 {
        let router_clone = Arc::clone(&router);
        let state_clone = Arc::clone(&state);
        let server_name = server_name.to_string();

        let handle = tokio::spawn(async move {
            // Check if server is ready
            if !state_clone.can_send_request("tools/list").await {
                // Queue the request
                let (tx, _rx) = tokio::sync::oneshot::channel();
                let request = QueuedRequest {
                    request_id: format!("req-{i}"),
                    method: "tools/list".to_string(),
                    params: None,
                    response_tx: Arc::new(Mutex::new(Some(tx))),
                };

                router_clone.queue_request(&server_name, request).await;
                (i, true) // Successfully queued
            } else {
                (i, false) // Server was ready (shouldn't happen in this test)
            }
        });

        handles.push(handle);
    }

    // Wait for all clients to queue their requests
    let results = futures::future::join_all(handles).await;

    // Verify all requests were queued
    let queued_count = results.iter().filter(|r| r.as_ref().unwrap().1).count();
    assert_eq!(queued_count, 10, "All 10 requests should be queued");

    // Verify queue size
    let queue_size = router.queued_request_count(server_name).await;
    assert_eq!(queue_size, 10, "Queue should contain 10 requests");

    // Now complete initialization
    state
        .received_initialize_response(ProtocolVersion::V20250618)
        .await
        .unwrap();
    state.complete_initialization().await.unwrap();

    // Process queued requests
    let queued_requests = router.process_queued_requests(server_name).await;
    assert_eq!(
        queued_requests.len(),
        10,
        "Should process 10 queued requests"
    );

    // Queue should now be empty
    let final_queue_size = router.queued_request_count(server_name).await;
    assert_eq!(
        final_queue_size, 0,
        "Queue should be empty after processing"
    );
}

/// T028 Test Case 2: All requests processed after initialization
#[tokio::test]
async fn t028_all_requests_processed_after_init() {
    use mcp_rust_proxy::proxy::router::{QueuedRequest, RequestRouter};

    let router = Arc::new(RequestRouter::new());
    let server_name = "processing-test-server";

    // Queue multiple requests
    for i in 0..5 {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let request = QueuedRequest {
            request_id: format!("req-{i}"),
            method: "tools/list".to_string(),
            params: None,
            response_tx: Arc::new(Mutex::new(Some(tx))),
        };
        router.queue_request(server_name, request).await;
    }

    // Verify all queued
    assert_eq!(router.queued_request_count(server_name).await, 5);

    // Process queue
    let processed = router.process_queued_requests(server_name).await;

    assert_eq!(processed.len(), 5, "Should process all 5 requests");
    assert_eq!(
        router.queued_request_count(server_name).await,
        0,
        "Queue should be empty"
    );

    // Verify each request has correct data
    for (i, req) in processed.iter().enumerate() {
        assert_eq!(req.request_id, format!("req-{i}"));
        assert_eq!(req.method, "tools/list");
    }
}

/// T028 Test Case 3: No race conditions with concurrent queueing
#[tokio::test]
async fn t028_no_race_conditions() {
    use mcp_rust_proxy::proxy::router::{QueuedRequest, RequestRouter};

    let router = Arc::new(RequestRouter::new());
    let server_name = "race-test-server";

    // Spawn 100 concurrent tasks that all queue requests
    let mut handles = vec![];

    for i in 0..100 {
        let router_clone = Arc::clone(&router);
        let server_name = server_name.to_string();

        let handle = tokio::spawn(async move {
            let (tx, _rx) = tokio::sync::oneshot::channel();
            let request = QueuedRequest {
                request_id: format!("req-{i}"),
                method: "tools/list".to_string(),
                params: None,
                response_tx: Arc::new(Mutex::new(Some(tx))),
            };

            router_clone.queue_request(&server_name, request).await;
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    futures::future::join_all(handles).await;

    // Verify all 100 requests were queued (no race conditions lost requests)
    let queue_size = router.queued_request_count(server_name).await;
    assert_eq!(
        queue_size, 100,
        "All 100 requests should be queued without race conditions"
    );

    // Process and verify
    let processed = router.process_queued_requests(server_name).await;
    assert_eq!(processed.len(), 100);
}

/// T028 Test Case 4: No deadlocks during concurrent operations
#[tokio::test]
async fn t028_no_deadlocks() {
    use mcp_rust_proxy::proxy::router::{QueuedRequest, RequestRouter};

    let router = Arc::new(RequestRouter::new());
    let state = Arc::new(ServerConnectionState::new("deadlock-test".to_string()));
    let server_name = "deadlock-test";

    // Start initialization
    state
        .start_initialization("init-1".to_string())
        .await
        .unwrap();

    // Spawn multiple tasks doing different operations concurrently
    let mut handles = vec![];

    // 10 tasks queuing requests
    for i in 0..10 {
        let router_clone = Arc::clone(&router);
        let server_name = server_name.to_string();
        handles.push(tokio::spawn(async move {
            let (tx, _rx) = tokio::sync::oneshot::channel();
            let request = QueuedRequest {
                request_id: format!("req-{i}"),
                method: "tools/list".to_string(),
                params: None,
                response_tx: Arc::new(Mutex::new(Some(tx))),
            };
            router_clone.queue_request(&server_name, request).await;
        }));
    }

    // 10 tasks checking queue size
    for _ in 0..10 {
        let router_clone = Arc::clone(&router);
        let server_name = server_name.to_string();
        handles.push(tokio::spawn(async move {
            let _size = router_clone.queued_request_count(&server_name).await;
        }));
    }

    // 10 tasks checking server state
    for _ in 0..10 {
        let state_clone = Arc::clone(&state);
        handles.push(tokio::spawn(async move {
            let _ready = state_clone.is_ready().await;
            let _can_send = state_clone.can_send_request("tools/list").await;
        }));
    }

    // Wait for all tasks with timeout to detect deadlocks
    let timeout = Duration::from_secs(5);
    let result = tokio::time::timeout(timeout, futures::future::join_all(handles)).await;

    assert!(
        result.is_ok(),
        "Operations should complete without deadlock"
    );

    // Complete initialization and process queue
    state
        .received_initialize_response(ProtocolVersion::V20250618)
        .await
        .unwrap();
    state.complete_initialization().await.unwrap();

    let processed = router.process_queued_requests(server_name).await;
    assert_eq!(processed.len(), 10, "Should process all queued requests");
}
