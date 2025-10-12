use mcp_rust_proxy::protocol::{ProtocolVersion, ServerConnectionState};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock MCP server for testing version negotiation
struct MockMcpServer {
    name: String,
    version: ProtocolVersion,
    initialized: Arc<RwLock<bool>>,
}

impl MockMcpServer {
    fn new(name: String, version: ProtocolVersion) -> Self {
        Self {
            name,
            version,
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    async fn handle_initialize(&self, request: serde_json::Value) -> serde_json::Value {
        // Validate request structure
        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["method"], "initialize");

        // Return initialize response with our version
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
                    "name": self.name,
                    "version": "1.0.0"
                }
            }
        })
    }

    async fn handle_initialized_notification(&self, notification: serde_json::Value) {
        assert_eq!(notification["jsonrpc"], "2.0");
        assert_eq!(notification["method"], "notifications/initialized");

        *self.initialized.write().await = true;
    }

    async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }
}

/// T022: Test multiple servers with different protocol versions
/// Test Case: 3 mock servers with different versions
/// Expected: All servers initialize successfully, versions stored correctly
#[tokio::test]
async fn test_three_servers_different_versions() {
    // Create 3 mock servers with different versions
    let server1 = MockMcpServer::new("server1".to_string(), ProtocolVersion::V20241105);
    let server2 = MockMcpServer::new("server2".to_string(), ProtocolVersion::V20250326);
    let server3 = MockMcpServer::new("server3".to_string(), ProtocolVersion::V20250618);

    // Create connection states for each
    let state1 = ServerConnectionState::new("server1".to_string());
    let state2 = ServerConnectionState::new("server2".to_string());
    let state3 = ServerConnectionState::new("server3".to_string());

    // Initialize server1
    state1
        .start_initialization("init-1".to_string())
        .await
        .unwrap();
    let request1 = json!({
        "jsonrpc": "2.0",
        "id": "init-1",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "proxy", "version": "1.0" }
        }
    });
    let response1 = server1.handle_initialize(request1).await;
    let (version1, _) =
        ProtocolVersion::from_string(response1["result"]["protocolVersion"].as_str().unwrap());
    state1.received_initialize_response(version1).await.unwrap();
    server1
        .handle_initialized_notification(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await;
    state1.complete_initialization().await.unwrap();

    // Initialize server2
    state2
        .start_initialization("init-2".to_string())
        .await
        .unwrap();
    let request2 = json!({
        "jsonrpc": "2.0",
        "id": "init-2",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "proxy", "version": "1.0" }
        }
    });
    let response2 = server2.handle_initialize(request2).await;
    let (version2, _) =
        ProtocolVersion::from_string(response2["result"]["protocolVersion"].as_str().unwrap());
    state2.received_initialize_response(version2).await.unwrap();
    server2
        .handle_initialized_notification(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await;
    state2.complete_initialization().await.unwrap();

    // Initialize server3
    state3
        .start_initialization("init-3".to_string())
        .await
        .unwrap();
    let request3 = json!({
        "jsonrpc": "2.0",
        "id": "init-3",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "proxy", "version": "1.0" }
        }
    });
    let response3 = server3.handle_initialize(request3).await;
    let (version3, _) =
        ProtocolVersion::from_string(response3["result"]["protocolVersion"].as_str().unwrap());
    state3.received_initialize_response(version3).await.unwrap();
    server3
        .handle_initialized_notification(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await;
    state3.complete_initialization().await.unwrap();

    // Verify all servers initialized successfully
    assert!(state1.is_ready().await);
    assert!(state2.is_ready().await);
    assert!(state3.is_ready().await);

    assert!(server1.is_initialized().await);
    assert!(server2.is_initialized().await);
    assert!(server3.is_initialized().await);

    // Verify each server's protocol version stored correctly
    assert_eq!(
        state1.protocol_version().await,
        Some(ProtocolVersion::V20241105)
    );
    assert_eq!(
        state2.protocol_version().await,
        Some(ProtocolVersion::V20250326)
    );
    assert_eq!(
        state3.protocol_version().await,
        Some(ProtocolVersion::V20250618)
    );
}

/// Test Case: All servers reach Ready state
#[tokio::test]
async fn test_all_servers_reach_ready_state() {
    let servers: Vec<(String, ProtocolVersion)> = vec![
        ("server-a".to_string(), ProtocolVersion::V20241105),
        ("server-b".to_string(), ProtocolVersion::V20250326),
        ("server-c".to_string(), ProtocolVersion::V20250618),
    ];

    let mut states = Vec::new();

    for (name, version) in servers {
        let mock = MockMcpServer::new(name.clone(), version);
        let state = ServerConnectionState::new(name.clone());

        // Initialize
        state
            .start_initialization(format!("init-{}", name))
            .await
            .unwrap();

        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("init-{}", name),
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "proxy", "version": "1.0" }
            }
        });

        let response = mock.handle_initialize(request).await;
        let (detected_version, _) =
            ProtocolVersion::from_string(response["result"]["protocolVersion"].as_str().unwrap());

        state
            .received_initialize_response(detected_version)
            .await
            .unwrap();

        mock.handle_initialized_notification(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await;

        state.complete_initialization().await.unwrap();

        states.push((name, state));
    }

    // All should be ready
    for (name, state) in &states {
        assert!(state.is_ready().await, "Server {} should be ready", name);
    }
}

/// Test Case: No cross-contamination of versions
#[tokio::test]
async fn test_no_cross_contamination_of_versions() {
    // Create servers with specific versions
    let server_configs: HashMap<String, ProtocolVersion> = [
        ("server-old".to_string(), ProtocolVersion::V20241105),
        ("server-mid".to_string(), ProtocolVersion::V20250326),
        ("server-new".to_string(), ProtocolVersion::V20250618),
    ]
    .iter()
    .cloned()
    .collect();

    let mut server_states: HashMap<String, ServerConnectionState> = HashMap::new();

    // Initialize all servers
    for (name, version) in &server_configs {
        let mock = MockMcpServer::new(name.clone(), *version);
        let state = ServerConnectionState::new(name.clone());

        state
            .start_initialization(format!("init-{}", name))
            .await
            .unwrap();

        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("init-{}", name),
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "proxy", "version": "1.0" }
            }
        });

        let response = mock.handle_initialize(request).await;
        let (detected_version, _) =
            ProtocolVersion::from_string(response["result"]["protocolVersion"].as_str().unwrap());

        state
            .received_initialize_response(detected_version)
            .await
            .unwrap();

        mock.handle_initialized_notification(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await;

        state.complete_initialization().await.unwrap();

        server_states.insert(name.clone(), state);
    }

    // Verify each server has its correct version (no contamination)
    for (name, expected_version) in server_configs {
        let state = server_states.get(&name).unwrap();
        let actual_version = state.protocol_version().await;

        assert_eq!(
            actual_version,
            Some(expected_version),
            "Server {} version mismatch",
            name
        );
    }

    // Verify all versions are different (sanity check)
    let mut versions_set: Vec<ProtocolVersion> = Vec::new();
    for state in server_states.values() {
        if let Some(version) = state.protocol_version().await {
            versions_set.push(version);
        }
    }

    versions_set.sort_by_key(|v| v.as_str());
    versions_set.dedup();

    assert_eq!(versions_set.len(), 3, "Should have 3 distinct versions");
}

/// Test Case: Concurrent initialization of multiple servers
#[tokio::test]
async fn test_concurrent_multi_server_initialization() {
    let server_specs = vec![
        ("concurrent-1", ProtocolVersion::V20241105),
        ("concurrent-2", ProtocolVersion::V20250326),
        ("concurrent-3", ProtocolVersion::V20250618),
    ];

    // Initialize all servers concurrently
    let futures = server_specs.into_iter().map(|(name, version)| async move {
        let mock = MockMcpServer::new(name.to_string(), version);
        let state = ServerConnectionState::new(name.to_string());

        state
            .start_initialization(format!("init-{}", name))
            .await
            .unwrap();

        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("init-{}", name),
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "proxy", "version": "1.0" }
            }
        });

        let response = mock.handle_initialize(request).await;
        let (detected_version, _) =
            ProtocolVersion::from_string(response["result"]["protocolVersion"].as_str().unwrap());

        state
            .received_initialize_response(detected_version)
            .await
            .unwrap();

        mock.handle_initialized_notification(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await;

        state.complete_initialization().await.unwrap();

        (name.to_string(), state, version)
    });

    let results = futures::future::join_all(futures).await;

    // All should succeed
    assert_eq!(results.len(), 3);

    for (name, state, expected_version) in results {
        assert!(state.is_ready().await, "Server {} should be ready", name);
        assert_eq!(
            state.protocol_version().await,
            Some(expected_version),
            "Server {} version mismatch",
            name
        );
    }
}
