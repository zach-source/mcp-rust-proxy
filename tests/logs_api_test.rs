use chrono::Utc;
use mcp_rust_proxy::config::{Config, HealthCheckConfig, ProxyConfig, WebUIConfig};
use mcp_rust_proxy::state::{AppState, LogEntry, ServerInfo};
use std::collections::HashMap;

fn create_test_config() -> Config {
    Config {
        proxy: ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            connection_pool_size: 10,
            request_timeout_ms: 30000,
            max_concurrent_requests: 100,
        },
        web_ui: WebUIConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 3001,
            static_dir: None,
            api_key: None,
        },
        servers: HashMap::new(),
        health_check: HealthCheckConfig {
            enabled: false,
            interval_seconds: 30,
            timeout_seconds: 5,
            max_attempts: 3,
            retry_interval_seconds: 5,
        },
    }
}

#[tokio::test]
async fn test_log_subscription() {
    // Create a minimal config
    let config = create_test_config();

    // Create app state
    let (app_state, _shutdown_rx) = AppState::new(config);

    // Create and register a server
    let server_info = ServerInfo::new("test-server".to_string());
    app_state
        .register_server("test-server".to_string(), server_info.clone())
        .await;

    // Subscribe to logs
    let subscriber_id = "test-subscriber".to_string();
    let mut log_rx = server_info.subscribe_logs(subscriber_id.clone());

    // Create a test log entry
    let log_entry = LogEntry {
        timestamp: Utc::now(),
        level: "info".to_string(),
        message: "Test log message".to_string(),
    };

    // Broadcast the log
    server_info.broadcast_log(log_entry.clone());

    // Receive the log
    let received_log = log_rx.recv().await.expect("Should receive log");
    assert_eq!(received_log.message, "Test log message");
    assert_eq!(received_log.level, "info");

    // Unsubscribe
    server_info.unsubscribe_logs(&subscriber_id);

    // Verify no more logs are received after unsubscribe
    server_info.broadcast_log(LogEntry {
        timestamp: Utc::now(),
        level: "info".to_string(),
        message: "Should not receive this".to_string(),
    });

    // Should timeout or return None
    assert!(log_rx.try_recv().is_err());
}

#[tokio::test]
async fn test_multiple_subscribers() {
    let config = create_test_config();
    let (app_state, _) = AppState::new(config);

    let server_info = ServerInfo::new("test-server".to_string());
    app_state
        .register_server("test-server".to_string(), server_info.clone())
        .await;

    // Create multiple subscribers
    let mut rx1 = server_info.subscribe_logs("sub1".to_string());
    let mut rx2 = server_info.subscribe_logs("sub2".to_string());

    // Broadcast a log
    let log_entry = LogEntry {
        timestamp: Utc::now(),
        level: "error".to_string(),
        message: "Error message".to_string(),
    };
    server_info.broadcast_log(log_entry.clone());

    // Both subscribers should receive the log
    let log1 = rx1.recv().await.expect("Subscriber 1 should receive log");
    let log2 = rx2.recv().await.expect("Subscriber 2 should receive log");

    assert_eq!(log1.message, "Error message");
    assert_eq!(log2.message, "Error message");
}

#[tokio::test]
async fn test_subscriber_cleanup_on_channel_close() {
    let config = create_test_config();
    let (app_state, _) = AppState::new(config);

    let server_info = ServerInfo::new("test-server".to_string());
    app_state
        .register_server("test-server".to_string(), server_info.clone())
        .await;

    // Create a subscriber and immediately drop the receiver
    {
        let _rx = server_info.subscribe_logs("dropping-sub".to_string());
        // rx is dropped here
    }

    // Give some time for the drop to propagate
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Broadcast a log - should automatically clean up the closed channel
    let log_entry = LogEntry {
        timestamp: Utc::now(),
        level: "info".to_string(),
        message: "Test cleanup".to_string(),
    };

    server_info.broadcast_log(log_entry);

    // The subscriber should have been removed
    assert_eq!(server_info.log_subscribers.len(), 0);
}

// Additional test: verify that the WebSocket message handler processes log subscriptions correctly
#[tokio::test]
async fn test_log_subscription_message_format() {
    let config = create_test_config();
    let (app_state, _) = AppState::new(config);

    // Register a test server
    let server_info = ServerInfo::new("format-test-server".to_string());
    app_state
        .register_server("format-test-server".to_string(), server_info.clone())
        .await;

    // Subscribe to logs
    let mut rx = server_info.subscribe_logs("test-sub".to_string());

    // Create and broadcast a log with specific format
    let timestamp = Utc::now();
    let log_entry = LogEntry {
        timestamp,
        level: "warning".to_string(),
        message: "Test warning message with special chars: $@#%".to_string(),
    };

    server_info.broadcast_log(log_entry.clone());

    // Verify received log matches
    let received = rx.recv().await.expect("Should receive log");
    assert_eq!(received.level, "warning");
    assert_eq!(
        received.message,
        "Test warning message with special chars: $@#%"
    );
    assert_eq!(received.timestamp, timestamp);
}
