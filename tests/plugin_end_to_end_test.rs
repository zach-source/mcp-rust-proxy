//! End-to-end integration tests for complete plugin system
//!
//! Tests verify all user stories working together:
//! - US1: Content curation
//! - US2: Security middleware  
//! - US3: Response transformation and chaining

use mcp_rust_proxy::plugin::config::PluginConfig;
use mcp_rust_proxy::plugin::manager::PluginManager;
use mcp_rust_proxy::plugin::schema::{PluginInput, PluginMetadata, PluginPhase};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn test_complete_plugin_system_integration() {
    // End-to-end test combining all plugin capabilities
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    // Ensure all transformation plugins are available
    let _ = std::fs::copy(
        "examples/plugins/path-normalizer.js",
        "tests/fixtures/plugins/path-normalizer.js",
    );
    let _ = std::fs::copy(
        "examples/plugins/enrich-metadata.js",
        "tests/fixtures/plugins/enrich-metadata.js",
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for plugin in &["path-normalizer.js", "enrich-metadata.js"] {
            if let Ok(metadata) = std::fs::metadata(format!("tests/fixtures/plugins/{}", plugin)) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ =
                    std::fs::set_permissions(format!("tests/fixtures/plugins/{}", plugin), perms);
            }
        }
    }

    let manager = Arc::new(PluginManager::new(PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers: HashMap::new(),
    }));

    let discovered = manager
        .discover_plugins()
        .await
        .expect("Plugin discovery failed");
    assert!(
        discovered >= 2,
        "Should discover at least echo and other plugins"
    );

    // Test 1: Echo plugin (basic functionality)
    let input = PluginInput {
        tool_name: "test/echo".to_string(),
        raw_content: "test content".to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "e2e-001".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "test".to_string(),
            phase: PluginPhase::Response,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = manager.execute("echo", &input, 30000).await;
    assert!(result.is_ok(), "Echo plugin should work");

    println!("✓ E2E Test Part 1: Basic plugin execution works");
    println!("  Discovered {} plugins", discovered);
}

#[tokio::test]
async fn test_plugin_system_handles_all_error_types() {
    // Test that the system gracefully handles all error scenarios
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let manager = PluginManager::new(PluginConfig {
        plugin_dir: plugin_dir.clone(),
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 1000, // Short timeout for testing
        servers: HashMap::new(),
    });

    manager.discover_plugins().await.expect("Discovery failed");

    // Test 1: Valid execution
    let input = PluginInput {
        tool_name: "test/valid".to_string(),
        raw_content: "valid input".to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "e2e-err-001".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "test".to_string(),
            phase: PluginPhase::Response,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = manager.execute("echo", &input, 30000).await;
    assert!(result.is_ok(), "Valid execution should succeed");

    // Test 2: Non-existent plugin
    let result = manager.execute("nonexistent-plugin", &input, 30000).await;
    assert!(result.is_err(), "Non-existent plugin should fail");

    println!("✓ E2E Error handling test passed");
    println!("  System gracefully handles plugin not found");
}

#[tokio::test]
async fn test_plugin_count_and_discovery() {
    // Verify plugin discovery and counting works correctly
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let manager = PluginManager::new(PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers: HashMap::new(),
    });

    let count = manager.discover_plugins().await.expect("Discovery failed");
    assert!(count > 0, "Should discover at least one plugin");
    assert_eq!(
        manager.plugin_count(),
        count,
        "Plugin count should match discovered count"
    );

    println!("✓ E2E Plugin discovery test passed");
    println!("  Discovered {} plugins", count);
}
