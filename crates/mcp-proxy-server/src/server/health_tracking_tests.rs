#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::state::{AppState, HealthCheckStatus, ServerState};
    use chrono::Utc;
    use mcp_proxy_core::config::{
        Config, HealthCheckConfig, ProxyConfig, ServerHealthCheckConfig, WebUIConfig,
    };
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

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
                enabled: true,
                interval_seconds: 1,
                timeout_seconds: 1,
                max_attempts: 3,
                retry_interval_seconds: 1,
            },
            servers: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_health_check_status_tracking() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Register a test server
        state
            .register_server(
                "test-server".to_string(),
                crate::state::ServerInfo {
                    name: "test-server".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        // Verify initial state
        let server_info = state.servers.get("test-server").unwrap();
        let health_check = server_info.last_health_check.read().await;
        assert!(health_check.is_none());
        drop(health_check);

        // Update health check status - success
        let success_status = HealthCheckStatus {
            timestamp: Utc::now(),
            success: true,
            response_time_ms: Some(50),
            error: None,
        };

        {
            let mut health_check = server_info.last_health_check.write().await;
            *health_check = Some(success_status.clone());
        }

        // Verify success status
        let health_check = server_info.last_health_check.read().await;
        assert!(health_check.is_some());
        let status = health_check.as_ref().unwrap();
        assert!(status.success);
        assert_eq!(status.response_time_ms, Some(50));
        assert!(status.error.is_none());
        drop(health_check);

        // Update health check status - failure
        let failure_status = HealthCheckStatus {
            timestamp: Utc::now(),
            success: false,
            response_time_ms: None,
            error: Some("Connection timeout".to_string()),
        };

        {
            let mut health_check = server_info.last_health_check.write().await;
            *health_check = Some(failure_status.clone());
        }

        // Verify failure status
        let health_check = server_info.last_health_check.read().await;
        assert!(health_check.is_some());
        let status = health_check.as_ref().unwrap();
        assert!(!status.success);
        assert!(status.response_time_ms.is_none());
        assert_eq!(status.error, Some("Connection timeout".to_string()));
    }

    #[tokio::test]
    async fn test_health_check_response_time_tracking() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Register a test server
        state
            .register_server(
                "test-server".to_string(),
                crate::state::ServerInfo {
                    name: "test-server".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let server_info = state.servers.get("test-server").unwrap();

        // Simulate multiple health checks with different response times
        let response_times = vec![10, 25, 50, 100, 200];

        for response_time in response_times {
            let status = HealthCheckStatus {
                timestamp: Utc::now(),
                success: true,
                response_time_ms: Some(response_time),
                error: None,
            };

            {
                let mut health_check = server_info.last_health_check.write().await;
                *health_check = Some(status.clone());
            }

            // Verify the response time was recorded
            let health_check = server_info.last_health_check.read().await;
            let current_status = health_check.as_ref().unwrap();
            assert_eq!(current_status.response_time_ms, Some(response_time));
            drop(health_check);

            // Small delay between checks
            sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn test_health_check_timestamp_tracking() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Register a test server
        state
            .register_server(
                "test-server".to_string(),
                crate::state::ServerInfo {
                    name: "test-server".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let server_info = state.servers.get("test-server").unwrap();

        // First health check
        let timestamp1 = Utc::now();
        {
            let mut health_check = server_info.last_health_check.write().await;
            *health_check = Some(HealthCheckStatus {
                timestamp: timestamp1,
                success: true,
                response_time_ms: Some(50),
                error: None,
            });
        }

        // Wait a bit
        sleep(Duration::from_millis(100)).await;

        // Second health check
        let timestamp2 = Utc::now();
        {
            let mut health_check = server_info.last_health_check.write().await;
            *health_check = Some(HealthCheckStatus {
                timestamp: timestamp2,
                success: true,
                response_time_ms: Some(60),
                error: None,
            });
        }

        // Verify timestamps are different and second is later
        let health_check = server_info.last_health_check.read().await;
        let status = health_check.as_ref().unwrap();
        assert_eq!(status.timestamp, timestamp2);
        assert!(timestamp2 > timestamp1);
    }

    #[tokio::test]
    async fn test_concurrent_health_check_updates() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Register a test server
        state
            .register_server(
                "test-server".to_string(),
                crate::state::ServerInfo {
                    name: "test-server".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let server_info = state.servers.get("test-server").unwrap().clone();

        // Spawn multiple tasks updating health check status concurrently
        let mut handles = vec![];

        for i in 0..10 {
            let server_info_clone = server_info.clone();
            let handle = tokio::spawn(async move {
                let status = HealthCheckStatus {
                    timestamp: Utc::now(),
                    success: i % 2 == 0,
                    response_time_ms: Some(i as u64 * 10),
                    error: if i % 2 == 0 {
                        None
                    } else {
                        Some(format!("Error {}", i))
                    },
                };

                let mut health_check = server_info_clone.last_health_check.write().await;
                *health_check = Some(status);
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify final state is consistent
        let health_check = server_info.last_health_check.read().await;
        assert!(health_check.is_some());
        let status = health_check.as_ref().unwrap();
        // Should have a valid timestamp
        assert!(status.timestamp <= Utc::now());
        // Should have either success with response time or failure with error
        if status.success {
            assert!(status.response_time_ms.is_some());
            assert!(status.error.is_none());
        } else {
            assert!(status.error.is_some());
        }
    }

    #[tokio::test]
    async fn test_health_check_persistence_across_state_changes() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Register a test server
        state
            .register_server(
                "test-server".to_string(),
                crate::state::ServerInfo {
                    name: "test-server".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Running)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        let server_info = state.servers.get("test-server").unwrap();

        // Set initial health check
        let initial_status = HealthCheckStatus {
            timestamp: Utc::now(),
            success: true,
            response_time_ms: Some(50),
            error: None,
        };

        {
            let mut health_check = server_info.last_health_check.write().await;
            *health_check = Some(initial_status.clone());
        }

        // Change server state to Stopped
        {
            let mut server_state = server_info.state.write().await;
            *server_state = ServerState::Stopped;
        }

        // Verify health check persists
        let health_check = server_info.last_health_check.read().await;
        assert!(health_check.is_some());
        assert_eq!(
            health_check.as_ref().unwrap().success,
            initial_status.success
        );
        drop(health_check);

        // Change server state to Failed
        {
            let mut server_state = server_info.state.write().await;
            *server_state = ServerState::Failed;
        }

        // Update health check to reflect failure
        let failure_status = HealthCheckStatus {
            timestamp: Utc::now(),
            success: false,
            response_time_ms: None,
            error: Some("Server failed".to_string()),
        };

        {
            let mut health_check = server_info.last_health_check.write().await;
            *health_check = Some(failure_status.clone());
        }

        // Verify updated health check
        let health_check = server_info.last_health_check.read().await;
        assert!(health_check.is_some());
        assert!(!health_check.as_ref().unwrap().success);
        assert_eq!(
            health_check.as_ref().unwrap().error,
            Some("Server failed".to_string())
        );
    }
}
