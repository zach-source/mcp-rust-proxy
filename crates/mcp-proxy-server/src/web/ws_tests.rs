#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::state::{AppState, HealthCheckStatus, ServerInfo, ServerState};
    use chrono::Utc;
    use mcp_proxy_core::config::{Config, HealthCheckConfig, ProxyConfig, WebUIConfig};
    use serde_json::json;
    use std::sync::Arc;
    use warp::test::ws;
    use warp::ws::Message;

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
                enabled: true,
                host: "127.0.0.1".to_string(),
                port: 8081,
                static_dir: None,
                api_key: None,
            },
            health_check: HealthCheckConfig {
                enabled: true,
                interval_seconds: 5,
                timeout_seconds: 1,
                max_attempts: 3,
                retry_interval_seconds: 1,
            },
            servers: std::collections::HashMap::new(),
        }
    }

    fn create_test_state() -> Arc<AppState> {
        let config = create_test_config();
        let (state, _) = AppState::new(config);
        state
    }

    #[tokio::test]
    async fn test_websocket_connection() {
        let state = create_test_state();
        let route = super::super::ws::route(state);

        let mut client = warp::test::ws()
            .path("/api/ws")
            .handshake(route)
            .await
            .expect("handshake");

        // Should receive initial message
        let msg = client.recv().await.unwrap();
        let text = msg.to_str().unwrap();
        let data: serde_json::Value = serde_json::from_str(text).unwrap();

        assert_eq!(data["type"], "initial");
        assert!(data["data"]["servers"].is_array());
    }

    #[tokio::test]
    async fn test_websocket_server_updates() {
        let state = create_test_state();

        // Register initial server
        state
            .register_server(
                "server1".to_string(),
                ServerInfo {
                    name: "server1".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let route = super::super::ws::route(state.clone());

        let mut client = warp::test::ws()
            .path("/api/ws")
            .handshake(route)
            .await
            .expect("handshake");

        // Should receive initial message with server1
        let msg = client.recv().await.unwrap();
        let text = msg.to_str().unwrap();
        let data: serde_json::Value = serde_json::from_str(text).unwrap();

        assert_eq!(data["type"], "initial");
        let servers = data["data"]["servers"].as_array().unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0]["name"], "server1");

        // Register another server
        state
            .register_server(
                "server2".to_string(),
                ServerInfo {
                    name: "server2".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Stopped)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        // Trigger update
        state.broadcast_update().await;

        // Should receive update message
        let msg = client.recv().await.unwrap();
        let text = msg.to_str().unwrap();
        let data: serde_json::Value = serde_json::from_str(text).unwrap();

        assert_eq!(data["type"], "update");
        let servers = data["data"]["servers"].as_array().unwrap();
        assert_eq!(servers.len(), 2);
    }

    #[tokio::test]
    async fn test_websocket_log_subscription() {
        let state = create_test_state();
        let route = super::super::ws::route(state);

        let mut client = warp::test::ws()
            .path("/api/ws")
            .handshake(route)
            .await
            .expect("handshake");

        // Skip initial message
        let _ = client.recv().await.unwrap();

        // Subscribe to logs for a server
        let subscribe_msg = json!({
            "type": "subscribe_logs",
            "server": "test-server"
        });

        client.send(Message::text(subscribe_msg.to_string())).await;

        // Should handle message without error
        // In real scenario, would receive log messages when server emits them
    }

    #[tokio::test]
    async fn test_websocket_log_unsubscription() {
        let state = create_test_state();
        let route = super::super::ws::route(state);

        let mut client = warp::test::ws()
            .path("/api/ws")
            .handshake(route)
            .await
            .expect("handshake");

        // Skip initial message
        let _ = client.recv().await.unwrap();

        // Subscribe first
        let subscribe_msg = json!({
            "type": "subscribe_logs",
            "server": "test-server"
        });
        client.send(Message::text(subscribe_msg.to_string())).await;

        // Then unsubscribe
        let unsubscribe_msg = json!({
            "type": "unsubscribe_logs",
            "server": "test-server"
        });
        client
            .send(Message::text(unsubscribe_msg.to_string()))
            .await;

        // Should handle both messages without error
    }

    #[tokio::test]
    async fn test_websocket_invalid_message() {
        let state = create_test_state();
        let route = super::super::ws::route(state);

        let mut client = warp::test::ws()
            .path("/api/ws")
            .handshake(route)
            .await
            .expect("handshake");

        // Skip initial message
        let _ = client.recv().await.unwrap();

        // Send invalid JSON
        client.send(Message::text("invalid json")).await;

        // Connection should remain open (error is logged but not fatal)
        // Send valid message to verify connection is still alive
        let valid_msg = json!({
            "type": "ping"
        });
        client.send(Message::text(valid_msg.to_string())).await;
    }

    #[tokio::test]
    async fn test_websocket_server_stats() {
        let state = create_test_state();

        // Register servers
        state
            .register_server(
                "server1".to_string(),
                ServerInfo {
                    name: "server1".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
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
                ServerInfo {
                    name: "server2".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        state
            .register_server(
                "server3".to_string(),
                ServerInfo {
                    name: "server3".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Stopped)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let route = super::super::ws::route(state);

        let mut client = warp::test::ws()
            .path("/api/ws")
            .handshake(route)
            .await
            .expect("handshake");

        // Should receive initial message with stats
        let msg = client.recv().await.unwrap();
        let text = msg.to_str().unwrap();
        let data: serde_json::Value = serde_json::from_str(text).unwrap();

        assert_eq!(data["type"], "initial");
        assert_eq!(data["data"]["stats"]["total_servers"], 3);
        assert_eq!(data["data"]["stats"]["running_servers"], 2);
    }

    #[tokio::test]
    async fn test_websocket_concurrent_clients() {
        let state = create_test_state();
        let route = super::super::ws::route(state.clone());

        // Connect multiple clients
        let mut client1 = warp::test::ws()
            .path("/api/ws")
            .handshake(route.clone())
            .await
            .expect("handshake");

        let mut client2 = warp::test::ws()
            .path("/api/ws")
            .handshake(route)
            .await
            .expect("handshake");

        // Both should receive initial message
        let msg1 = client1.recv().await.unwrap();
        let msg2 = client2.recv().await.unwrap();

        let data1: serde_json::Value = serde_json::from_str(msg1.to_str().unwrap()).unwrap();
        let data2: serde_json::Value = serde_json::from_str(msg2.to_str().unwrap()).unwrap();

        assert_eq!(data1["type"], "initial");
        assert_eq!(data2["type"], "initial");

        // Register a server
        state
            .register_server(
                "new-server".to_string(),
                ServerInfo {
                    name: "new-server".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        // Trigger update
        state.broadcast_update().await;

        // Both clients should receive update
        let update1 = client1.recv().await.unwrap();
        let update2 = client2.recv().await.unwrap();

        let data1: serde_json::Value = serde_json::from_str(update1.to_str().unwrap()).unwrap();
        let data2: serde_json::Value = serde_json::from_str(update2.to_str().unwrap()).unwrap();

        assert_eq!(data1["type"], "update");
        assert_eq!(data2["type"], "update");
    }

    #[tokio::test]
    async fn test_websocket_health_check_updates() {
        let state = create_test_state();

        // Register server with health check
        state
            .register_server(
                "server1".to_string(),
                ServerInfo {
                    name: "server1".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(Some(
                        HealthCheckStatus {
                            timestamp: Utc::now(),
                            success: true,
                            response_time_ms: Some(50),
                            error: None,
                        },
                    ))),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let route = super::super::ws::route(state.clone());

        let mut client = warp::test::ws()
            .path("/api/ws")
            .handshake(route)
            .await
            .expect("handshake");

        // Should receive initial message with health check data
        let msg = client.recv().await.unwrap();
        let text = msg.to_str().unwrap();
        let data: serde_json::Value = serde_json::from_str(text).unwrap();

        let server = &data["data"]["servers"][0];
        assert!(server["last_health_check"].is_object());
        assert_eq!(server["last_health_check"]["success"], true);
        assert_eq!(server["last_health_check"]["response_time_ms"], 50);

        // Update health check status
        if let Some(server_info) = state.servers.get("server1") {
            let mut health_check = server_info.last_health_check.write().await;
            *health_check = Some(HealthCheckStatus {
                timestamp: Utc::now(),
                success: false,
                response_time_ms: None,
                error: Some("Connection failed".to_string()),
            });
        }

        // Trigger update
        state.broadcast_update().await;

        // Should receive update with new health check data
        let msg = client.recv().await.unwrap();
        let text = msg.to_str().unwrap();
        let data: serde_json::Value = serde_json::from_str(text).unwrap();

        let server = &data["data"]["servers"][0];
        assert_eq!(server["last_health_check"]["success"], false);
        assert_eq!(server["last_health_check"]["error"], "Connection failed");
    }
}
