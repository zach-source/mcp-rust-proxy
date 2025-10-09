#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::config::{
        Config, HealthCheckConfig, ProxyConfig, ServerConfig, TransportConfig, WebUIConfig,
    };
    use crate::state::{AppState, HealthCheckStatus, ServerInfo, ServerState};
    use chrono::Utc;
    use serde_json::json;
    use std::sync::Arc;
    use warp::test::request;

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
    async fn test_list_servers_empty() {
        let state = create_test_state();
        let routes = api::routes(state);

        let resp = request()
            .method("GET")
            .path("/api/servers")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["servers"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_list_servers_with_data() {
        let state = create_test_state();

        // Register some servers
        state
            .register_server(
                "server1".to_string(),
                ServerInfo {
                    name: "server1".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(2)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(Some(
                        HealthCheckStatus {
                            timestamp: Utc::now(),
                            success: true,
                            response_time_ms: Some(50),
                            error: None,
                        },
                    ))),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(Some(Utc::now()))),
                },
            )
            .await;

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

        let routes = api::routes(state);

        let resp = request()
            .method("GET")
            .path("/api/servers")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        let servers = body["servers"].as_array().unwrap();
        assert_eq!(servers.len(), 2);

        // Find server1
        let server1 = servers.iter().find(|s| s["name"] == "server1").unwrap();
        assert_eq!(server1["state"], "Running");
        assert_eq!(server1["restart_count"], 2);
        assert!(server1["last_health_check"].is_object());
        assert!(server1["last_health_check"]["success"].as_bool().unwrap());
        assert_eq!(server1["last_health_check"]["response_time_ms"], 50);
        assert!(server1["last_access_time"].is_string());

        // Find server2
        let server2 = servers.iter().find(|s| s["name"] == "server2").unwrap();
        assert_eq!(server2["state"], "Stopped");
        assert_eq!(server2["restart_count"], 0);
        assert!(server2["last_health_check"].is_null());
        assert!(server2["last_access_time"].is_null());
    }

    #[tokio::test]
    async fn test_server_status_endpoint() {
        let state = create_test_state();

        // Register a server
        state
            .register_server(
                "test-server".to_string(),
                ServerInfo {
                    name: "test-server".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(5)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(Some(
                        HealthCheckStatus {
                            timestamp: Utc::now(),
                            success: false,
                            response_time_ms: None,
                            error: Some("Connection timeout".to_string()),
                        },
                    ))),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(Some(Utc::now()))),
                },
            )
            .await;

        let routes = api::routes(state);

        let resp = request()
            .method("GET")
            .path("/api/servers/test-server")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["name"], "test-server");
        assert_eq!(body["state"], "Running");
        assert_eq!(body["restart_count"], 5);
        assert!(!body["last_health_check"]["success"].as_bool().unwrap());
        assert_eq!(body["last_health_check"]["error"], "Connection timeout");
    }

    #[tokio::test]
    async fn test_server_status_not_found() {
        let state = create_test_state();
        let routes = api::routes(state);

        let resp = request()
            .method("GET")
            .path("/api/servers/nonexistent")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 404);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body["error"].as_str().unwrap().contains("Server not found"));
    }

    #[tokio::test]
    async fn test_server_action_start() {
        let mut config = create_test_config();
        config.servers.insert(
            "test-server".to_string(),
            ServerConfig {
                command: "echo".to_string(),
                args: vec!["test".to_string()],
                env: std::collections::HashMap::new(),
                transport: TransportConfig::Stdio,
                restart_on_failure: false,
                working_directory: None,
                max_restarts: 0,
                restart_delay_ms: 1000,
                health_check: None,
            },
        );

        let (state, _) = AppState::new(config);

        // Register a stopped server
        state
            .register_server(
                "test-server".to_string(),
                ServerInfo {
                    name: "test-server".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Stopped)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let routes = api::routes(state.clone());

        let resp = request()
            .method("POST")
            .path("/api/servers/test-server/start")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["status"], "success");
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("Server test-server action start completed"));
    }

    #[tokio::test]
    async fn test_server_action_invalid() {
        let state = create_test_state();
        let routes = api::routes(state);

        let resp = request()
            .method("POST")
            .path("/api/servers/test-server/invalid-action")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 400);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body["error"].as_str().unwrap().contains("Unknown action"));
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let state = create_test_state();

        // Update some metrics
        state
            .metrics
            .record_request_duration(std::time::Duration::from_millis(100));
        state
            .metrics
            .record_request_duration(std::time::Duration::from_millis(200));
        state.metrics.record_failed_request();

        let routes = api::routes(state);

        let resp = request()
            .method("GET")
            .path("/api/metrics")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body["metrics"].is_array());

        let metrics = body["metrics"].as_array().unwrap();

        // Find requests total metric
        let requests_metric = metrics
            .iter()
            .find(|m| m["name"] == "mcp_proxy_requests_total")
            .unwrap();

        // Should have metrics for test-server
        let test_server_metric = requests_metric["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["labels"]["server"] == "test-server");

        assert!(test_server_metric.is_some());
    }

    #[tokio::test]
    async fn test_get_config() {
        let config = create_test_config();
        let expected_host = config.proxy.connection_pool_size;
        let (state, _) = AppState::new(config);

        let routes = api::routes(state);

        let resp = request()
            .method("GET")
            .path("/api/config")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["proxy"]["connectionPoolSize"], expected_host);
    }

    #[tokio::test]
    async fn test_update_config() {
        let state = create_test_state();
        let routes = api::routes(state.clone());

        let new_config = json!({
            "proxy": {
                "host": "127.0.0.1",
                "port": 3000,
                "connection_pool_size": 20,
                "request_timeout_ms": 10000,
                "max_concurrent_requests": 200
            },
            "web_ui": {
                "enabled": true,
                "host": "127.0.0.1",
                "port": 8082,
                "static_dir": null,
                "api_key": null
            },
            "health_check": {
                "enabled": false,
                "interval_seconds": 10,
                "timeout_seconds": 2,
                "max_attempts": 5,
                "retry_interval_seconds": 2
            },
            "servers": {}
        });

        let resp = request()
            .method("PUT")
            .path("/api/config")
            .json(&new_config)
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["status"], "success");

        // Verify config was updated
        let config = state.config.read().await;
        assert_eq!(config.proxy.connection_pool_size, 20);
        assert_eq!(config.health_check.enabled, false);
    }

    #[tokio::test]
    async fn test_health_check_enabled_in_response() {
        let mut config = create_test_config();

        // Add server with health check enabled
        config.servers.insert(
            "server1".to_string(),
            ServerConfig {
                command: "echo".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                transport: TransportConfig::Stdio,
                restart_on_failure: false,
                working_directory: None,
                max_restarts: 0,
                restart_delay_ms: 1000,
                health_check: Some(crate::config::ServerHealthCheckConfig {
                    enabled: Some(true),
                    interval_seconds: None,
                    timeout_seconds: None,
                    max_attempts: None,
                    retry_interval_seconds: None,
                }),
            },
        );

        // Add server with health check disabled
        config.servers.insert(
            "server2".to_string(),
            ServerConfig {
                command: "echo".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                transport: TransportConfig::Stdio,
                restart_on_failure: false,
                working_directory: None,
                max_restarts: 0,
                restart_delay_ms: 1000,
                health_check: Some(crate::config::ServerHealthCheckConfig {
                    enabled: Some(false),
                    interval_seconds: None,
                    timeout_seconds: None,
                    max_attempts: None,
                    retry_interval_seconds: None,
                }),
            },
        );

        let (state, _) = AppState::new(config);

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

        let routes = api::routes(state);

        let resp = request()
            .method("GET")
            .path("/api/servers")
            .reply(&routes)
            .await;

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        let servers = body["servers"].as_array().unwrap();

        let server1 = servers.iter().find(|s| s["name"] == "server1").unwrap();
        assert_eq!(server1["health_check_enabled"], true);

        let server2 = servers.iter().find(|s| s["name"] == "server2").unwrap();
        assert_eq!(server2["health_check_enabled"], false);
    }
}
