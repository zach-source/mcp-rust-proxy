//! Integration tests for plugin security blocking flow
//!
//! Tests verify the request-phase security validation including:
//! - Security plugin blocks requests with sensitive data
//! - Blocked requests don't reach MCP server
//! - Clear error messages are returned
//! - Safe requests pass through unchanged

use mcp_rust_proxy::plugin::config::PluginConfig;
use mcp_rust_proxy::plugin::manager::PluginManager;
use mcp_rust_proxy::plugin::schema::{PluginInput, PluginMetadata, PluginPhase};
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::test]
async fn test_security_blocks_sensitive_password() {
    // Setup: Use the security plugin from test fixtures
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let config = PluginConfig {
        plugin_dir: plugin_dir.clone(),
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers: HashMap::new(),
    };

    let manager = PluginManager::new(config);

    // Discover plugins
    let count = manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");
    assert!(count > 0, "No plugins discovered");

    // Create request with sensitive password pattern
    let sensitive_request = "password: secret123";

    let input = PluginInput {
        tool_name: "filesystem/write".to_string(),
        raw_content: sensitive_request.to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-sec-001".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "filesystem".to_string(),
            phase: PluginPhase::Request,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    // Execute the security plugin
    let result = manager.execute("security-plugin", &input, 30000).await;

    // Assert the execution succeeded (plugin runs successfully)
    assert!(
        result.is_ok(),
        "Plugin execution failed: {:?}",
        result.err()
    );

    let output = result.unwrap();

    // Assert request was blocked
    assert!(
        !output.continue_,
        "Security plugin should block sensitive requests"
    );

    // Assert error message is present
    assert!(
        output.error.is_some(),
        "Blocked request should include error message"
    );

    let error = output.error.unwrap();
    assert!(
        error.contains("Security violation") || error.contains("sensitive data"),
        "Error should indicate security violation"
    );

    // Assert text is blocked marker
    assert_eq!(
        output.text, "[BLOCKED]",
        "Blocked request should return [BLOCKED] text"
    );

    println!("✓ Security blocking test passed");
    println!("  Blocked pattern: password");
    println!("  Error: {error}");
}

#[tokio::test]
async fn test_security_blocks_api_key() {
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let config = PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers: HashMap::new(),
    };

    let manager = PluginManager::new(config);
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    // Request with API key pattern
    let sensitive_request = "api_key: sk-abc123xyz";

    let input = PluginInput {
        tool_name: "external-api/call".to_string(),
        raw_content: sensitive_request.to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-sec-002".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "external-api".to_string(),
            phase: PluginPhase::Request,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = manager.execute("security-plugin", &input, 30000).await;
    assert!(result.is_ok(), "Plugin execution failed");

    let output = result.unwrap();
    assert!(!output.continue_, "API key should be blocked");
    assert!(output.error.is_some(), "Should have error message");

    println!("✓ API key blocking test passed");
}

#[tokio::test]
async fn test_security_allows_safe_requests() {
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let config = PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers: HashMap::new(),
    };

    let manager = PluginManager::new(config);
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    // Safe request with no sensitive patterns
    let safe_request = r#"{"action": "read", "path": "/docs/readme.md"}"#;

    let input = PluginInput {
        tool_name: "filesystem/read".to_string(),
        raw_content: safe_request.to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-sec-003".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "filesystem".to_string(),
            phase: PluginPhase::Request,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = manager.execute("security-plugin", &input, 30000).await;
    assert!(result.is_ok(), "Plugin execution failed");

    let output = result.unwrap();

    // Assert request was allowed to continue
    assert!(
        output.continue_,
        "Security plugin should allow safe requests"
    );

    // Assert no error
    assert!(output.error.is_none(), "Safe request should not have error");

    // Assert content preserved
    assert_eq!(
        output.text, safe_request,
        "Safe request content should be preserved"
    );

    // Assert security check metadata
    assert!(
        output.metadata.is_some(),
        "Should include security check metadata"
    );

    println!("✓ Safe request allowance test passed");
    println!("  Request allowed through security check");
}

#[tokio::test]
async fn test_security_plugin_only_processes_request_phase() {
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let config = PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers: HashMap::new(),
    };

    let manager = PluginManager::new(config);
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    // Response with sensitive pattern (should pass through)
    let response_with_sensitive = r#"{"result": "password: ******, token: ******"}"#;

    let input = PluginInput {
        tool_name: "test/response".to_string(),
        raw_content: response_with_sensitive.to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-sec-004".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "test".to_string(),
            phase: PluginPhase::Response, // Response phase
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = manager.execute("security-plugin", &input, 30000).await;
    assert!(result.is_ok(), "Plugin execution failed");

    let output = result.unwrap();

    // Assert response phase is not blocked (security only applies to requests)
    assert!(
        output.continue_,
        "Security plugin should pass through responses"
    );

    assert_eq!(
        output.text, response_with_sensitive,
        "Response content should be preserved"
    );

    println!("✓ Response phase passthrough test passed");
}

#[tokio::test]
async fn test_security_blocks_bearer_token() {
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let config = PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers: HashMap::new(),
    };

    let manager = PluginManager::new(config);
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    // Request with Bearer token
    let token_request = r#"{"headers": {"Authorization": "Bearer eyJhbGciOiJIUzI1NiIs..."}}"#;

    let input = PluginInput {
        tool_name: "api/request".to_string(),
        raw_content: token_request.to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-sec-005".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "api".to_string(),
            phase: PluginPhase::Request,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = manager.execute("security-plugin", &input, 30000).await;
    assert!(result.is_ok(), "Plugin execution failed");

    let output = result.unwrap();
    assert!(!output.continue_, "Bearer token should be blocked");
    assert!(output.error.is_some(), "Should have error message");

    println!("✓ Bearer token blocking test passed");
}

#[tokio::test]
async fn test_security_blocks_private_key() {
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let config = PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers: HashMap::new(),
    };

    let manager = PluginManager::new(config);
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    // Request with private key
    let key_request = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQE...";

    let input = PluginInput {
        tool_name: "crypto/sign".to_string(),
        raw_content: key_request.to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-sec-006".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "crypto".to_string(),
            phase: PluginPhase::Request,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = manager.execute("security-plugin", &input, 30000).await;
    assert!(result.is_ok(), "Plugin execution failed");

    let output = result.unwrap();
    assert!(!output.continue_, "Private key should be blocked");
    assert!(output.error.is_some(), "Should have error message");

    println!("✓ Private key blocking test passed");
}
