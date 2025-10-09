#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::config::{Config, HealthCheckConfig, ProxyConfig, WebUIConfig};
    use chrono::Utc;
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
    async fn test_server_state_transitions() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Register a server
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

        // Test initial state
        let current_state = state.get_server_state("test-server").await;
        assert_eq!(current_state, Some(ServerState::Stopped));

        // Test transition to Starting
        state
            .set_server_state("test-server", ServerState::Starting)
            .await
            .unwrap();
        let current_state = state.get_server_state("test-server").await;
        assert_eq!(current_state, Some(ServerState::Starting));

        // Test transition to Running
        state
            .set_server_state("test-server", ServerState::Running)
            .await
            .unwrap();
        let current_state = state.get_server_state("test-server").await;
        assert_eq!(current_state, Some(ServerState::Running));

        // Test transition to Stopping
        state
            .set_server_state("test-server", ServerState::Stopping)
            .await
            .unwrap();
        let current_state = state.get_server_state("test-server").await;
        assert_eq!(current_state, Some(ServerState::Stopping));

        // Test transition to Stopped
        state
            .set_server_state("test-server", ServerState::Stopped)
            .await
            .unwrap();
        let current_state = state.get_server_state("test-server").await;
        assert_eq!(current_state, Some(ServerState::Stopped));

        // Test transition to Failed
        state
            .set_server_state("test-server", ServerState::Failed)
            .await
            .unwrap();
        let current_state = state.get_server_state("test-server").await;
        assert_eq!(current_state, Some(ServerState::Failed));
    }

    #[tokio::test]
    async fn test_server_registration() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Initially no servers
        assert_eq!(state.servers.len(), 0);

        // Register first server
        state
            .register_server(
                "server1".to_string(),
                ServerInfo {
                    name: "server1".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Stopped)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        assert_eq!(state.servers.len(), 1);
        assert!(state.servers.contains_key("server1"));

        // Register second server
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

        assert_eq!(state.servers.len(), 2);
        assert!(state.servers.contains_key("server2"));

        // Verify states
        assert_eq!(
            state.get_server_state("server1").await,
            Some(ServerState::Stopped)
        );
        assert_eq!(
            state.get_server_state("server2").await,
            Some(ServerState::Running)
        );
    }

    #[tokio::test]
    async fn test_restart_count_tracking() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Register a server
        state
            .register_server(
                "test-server".to_string(),
                ServerInfo {
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

        // Initial restart count should be 0
        let count = *server_info.restart_count.read().await;
        assert_eq!(count, 0);

        // Increment restart count
        {
            let mut count = server_info.restart_count.write().await;
            *count += 1;
        }

        let count = *server_info.restart_count.read().await;
        assert_eq!(count, 1);

        // Increment multiple times
        for _ in 0..5 {
            let mut count = server_info.restart_count.write().await;
            *count += 1;
        }

        let count = *server_info.restart_count.read().await;
        assert_eq!(count, 6);
    }

    #[tokio::test]
    async fn test_last_access_time_tracking() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Register a server
        state
            .register_server(
                "test-server".to_string(),
                ServerInfo {
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

        // Initially no access time
        let access_time = server_info.last_access_time.read().await;
        assert!(access_time.is_none());
        drop(access_time);

        // Update access time
        let time1 = Utc::now();
        {
            let mut access_time = server_info.last_access_time.write().await;
            *access_time = Some(time1);
        }

        // Verify access time
        let access_time = server_info.last_access_time.read().await;
        assert!(access_time.is_some());
        assert_eq!(access_time.unwrap(), time1);
        drop(access_time);

        // Wait and update again
        sleep(Duration::from_millis(100)).await;
        let time2 = Utc::now();
        {
            let mut access_time = server_info.last_access_time.write().await;
            *access_time = Some(time2);
        }

        // Verify newer access time
        let access_time = server_info.last_access_time.read().await;
        assert!(access_time.is_some());
        assert_eq!(access_time.unwrap(), time2);
        assert!(time2 > time1);
    }

    #[tokio::test]
    async fn test_concurrent_state_updates() {
        let config = create_test_config();
        let (state_arc, _) = AppState::new(config);

        // Register a server
        state_arc
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

        // Spawn multiple tasks updating state concurrently
        let mut handles = vec![];
        let states = vec![
            ServerState::Starting,
            ServerState::Running,
            ServerState::Stopping,
            ServerState::Stopped,
            ServerState::Failed,
        ];

        for (i, new_state) in states.into_iter().enumerate() {
            let state_clone = state_arc.clone();
            let handle = tokio::spawn(async move {
                // Small delay to ensure concurrent execution
                sleep(Duration::from_millis((i as u64) * 10)).await;
                state_clone
                    .set_server_state("test-server", new_state)
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Final state should be one of the valid states
        let final_state = state_arc.get_server_state("test-server").await;
        assert!(final_state.is_some());
        assert!(matches!(
            final_state.unwrap(),
            ServerState::Starting
                | ServerState::Running
                | ServerState::Stopping
                | ServerState::Stopped
                | ServerState::Failed
        ));
    }

    #[tokio::test]
    async fn test_nonexistent_server_operations() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Try to get state of non-existent server
        let server_state = state.get_server_state("nonexistent").await;
        assert!(server_state.is_none());

        // Try to set state of non-existent server
        let result = state
            .set_server_state("nonexistent", ServerState::Running)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_metrics_updates_with_state_changes() {
        let config = create_test_config();
        let (state, _) = AppState::new(config);

        // Initial metrics
        let initial_total = state.metrics.total_servers.get();
        let initial_running = state.metrics.running_servers.get();

        // Register a stopped server
        state
            .register_server(
                "server1".to_string(),
                ServerInfo {
                    name: "server1".to_string(),
                    state: Arc::new(tokio::sync::RwLock::new(ServerState::Stopped)),
                    process_handle: None,
                    restart_count: Arc::new(tokio::sync::RwLock::new(0)),
                    last_health_check: Arc::new(tokio::sync::RwLock::new(None)),
                    last_access_time: Arc::new(tokio::sync::RwLock::new(None)),
                },
            )
            .await;

        // Server count should increase
        assert_eq!(state.metrics.total_servers.get(), initial_total + 1);
        assert_eq!(state.metrics.running_servers.get(), initial_running);

        // Start the server
        state
            .set_server_state("server1", ServerState::Running)
            .await
            .unwrap();
        assert_eq!(state.metrics.running_servers.get(), initial_running + 1);

        // Register a running server
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

        assert_eq!(state.metrics.total_servers.get(), initial_total + 2);
        assert_eq!(state.metrics.running_servers.get(), initial_running + 2);
    }
}
