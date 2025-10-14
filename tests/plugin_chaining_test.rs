//! Integration tests for plugin chaining
//!
//! Tests verify that:
//! - Multiple plugins chain correctly
//! - Transformations applied in order
//! - Metadata from all plugins preserved
//! - Chain termination works as expected

use mcp_rust_proxy::plugin::chain::PluginChain;
use mcp_rust_proxy::plugin::config::{PluginAssignment, PluginConfig, ServerPluginConfig};
use mcp_rust_proxy::plugin::manager::PluginManager;
use mcp_rust_proxy::plugin::schema::{PluginInput, PluginMetadata, PluginPhase};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn test_three_plugin_chain_metadata_aggregation() {
    // Test that metadata from all 3 plugins is aggregated
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    // Copy transformation plugins to test fixtures
    std::fs::copy(
        "examples/plugins/path-normalizer.js",
        "tests/fixtures/plugins/path-normalizer.js",
    )
    .expect("Failed to copy path-normalizer");

    std::fs::copy(
        "examples/plugins/enrich-metadata.js",
        "tests/fixtures/plugins/enrich-metadata.js",
    )
    .expect("Failed to copy enrich-metadata");

    // Make them executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata("tests/fixtures/plugins/path-normalizer.js")
            .unwrap()
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions("tests/fixtures/plugins/path-normalizer.js", perms).unwrap();

        let mut perms = std::fs::metadata("tests/fixtures/plugins/enrich-metadata.js")
            .unwrap()
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions("tests/fixtures/plugins/enrich-metadata.js", perms).unwrap();
    }

    // Create a chain: echo → path-normalizer → enrich-metadata
    let mut servers = HashMap::new();
    servers.insert(
        "test-server".to_string(),
        ServerPluginConfig {
            request: vec![],
            response: vec![
                PluginAssignment {
                    name: "echo".to_string(),
                    order: 1,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "path-normalizer".to_string(),
                    order: 2,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "enrich-metadata".to_string(),
                    order: 3,
                    enabled: true,
                    timeout_ms: None,
                },
            ],
        },
    );

    let config = Arc::new(PluginConfig {
        plugin_dir: plugin_dir.clone(),
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers,
    });

    let manager = Arc::new(PluginManager::new((*config).clone()));
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    let chain = PluginChain::new(
        "test-server".to_string(),
        PluginPhase::Response,
        manager,
        config,
    );

    // Input with Windows-style path
    let content_with_path = "File located at C:\\Users\\Alice\\Documents\\report.txt";

    let input = PluginInput {
        tool_name: "filesystem/read".to_string(),
        raw_content: content_with_path.to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-chain-001".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "test-server".to_string(),
            phase: PluginPhase::Response,
            user_query: Some("Get file".to_string()),
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = chain.execute(input).await;
    assert!(result.is_ok(), "Chain execution failed: {:?}", result.err());

    let output = result.unwrap();

    // Assert content was transformed by path-normalizer
    assert!(
        output.text.contains("/Users/Alice/Documents/report.txt"),
        "Path should be normalized to Unix format"
    );

    // Assert metadata from all 3 plugins is present
    assert!(
        output.metadata.is_some(),
        "Aggregated metadata should be present"
    );

    let metadata = output.metadata.unwrap();
    let metadata_obj = metadata.as_object().expect("Metadata should be an object");

    // Check that we have metadata from all 3 plugins
    assert!(
        metadata_obj.contains_key("echo"),
        "Should have metadata from echo plugin"
    );
    assert!(
        metadata_obj.contains_key("path-normalizer"),
        "Should have metadata from path-normalizer plugin"
    );
    assert!(
        metadata_obj.contains_key("enrich-metadata"),
        "Should have metadata from enrich-metadata plugin"
    );

    // Verify path-normalizer metadata
    let path_meta = metadata_obj.get("path-normalizer").unwrap();
    assert!(
        path_meta.get("pathsNormalized").is_some(),
        "path-normalizer should report paths normalized"
    );

    println!("✓ Three-plugin chain metadata aggregation test passed");
    println!("  Plugins executed: echo → path-normalizer → enrich-metadata");
    let keys: Vec<&String> = metadata_obj.keys().collect();
    println!("  Metadata keys: {:?}", keys);
}

#[tokio::test]
async fn test_chain_execution_order() {
    // Test that plugins execute in the correct order
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let mut servers = HashMap::new();
    servers.insert(
        "order-test".to_string(),
        ServerPluginConfig {
            request: vec![],
            response: vec![
                PluginAssignment {
                    name: "enrich-metadata".to_string(),
                    order: 3,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "echo".to_string(),
                    order: 1,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "path-normalizer".to_string(),
                    order: 2,
                    enabled: true,
                    timeout_ms: None,
                },
            ],
        },
    );

    let config = Arc::new(PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers,
    });

    let manager = Arc::new(PluginManager::new((*config).clone()));
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    let chain = PluginChain::new(
        "order-test".to_string(),
        PluginPhase::Response,
        manager,
        config,
    );

    // Verify plugins are sorted by order
    let plugins = chain.build();
    assert_eq!(plugins.len(), 3);
    assert_eq!(plugins[0].name, "echo", "First should be echo (order 1)");
    assert_eq!(
        plugins[1].name, "path-normalizer",
        "Second should be path-normalizer (order 2)"
    );
    assert_eq!(
        plugins[2].name, "enrich-metadata",
        "Third should be enrich-metadata (order 3)"
    );

    println!("✓ Chain execution order test passed");
}

#[tokio::test]
async fn test_chain_stops_on_continue_false() {
    // Test that chain stops when a plugin returns continue=false
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    // Create a blocker plugin that returns continue=false
    std::fs::write(
        plugin_dir.join("blocker.js"),
        r#"#!/usr/bin/env node
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, terminal: false });
rl.on('line', (line) => {
  try {
    const input = JSON.parse(line);
    const output = { text: "BLOCKED", continue: false, metadata: { blocked: true } };
    process.stdout.write(JSON.stringify(output) + '\n');
  } catch (err) {
    const errorOutput = { text: "", continue: false, error: err.message };
    process.stdout.write(JSON.stringify(errorOutput) + '\n');
  }
});
rl.on('close', () => process.exit(0));
"#,
    )
    .expect("Failed to create blocker plugin");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(plugin_dir.join("blocker.js"))
            .unwrap()
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(plugin_dir.join("blocker.js"), perms).unwrap();
    }

    let mut servers = HashMap::new();
    servers.insert(
        "block-test".to_string(),
        ServerPluginConfig {
            request: vec![],
            response: vec![
                PluginAssignment {
                    name: "echo".to_string(),
                    order: 1,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "blocker".to_string(),
                    order: 2,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "enrich-metadata".to_string(),
                    order: 3,
                    enabled: true,
                    timeout_ms: None,
                },
            ],
        },
    );

    let config = Arc::new(PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers,
    });

    let manager = Arc::new(PluginManager::new((*config).clone()));
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    let chain = PluginChain::new(
        "block-test".to_string(),
        PluginPhase::Response,
        manager,
        config,
    );

    let input = PluginInput {
        tool_name: "test/block".to_string(),
        raw_content: "test content".to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-chain-002".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "block-test".to_string(),
            phase: PluginPhase::Response,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = chain.execute(input).await;
    assert!(
        result.is_ok(),
        "Chain should handle continue=false gracefully"
    );

    let output = result.unwrap();

    // Assert chain stopped (continue=false)
    assert_eq!(
        output.continue_, false,
        "Chain should stop when plugin returns continue=false"
    );

    // Assert output is from blocker plugin
    assert_eq!(output.text, "BLOCKED", "Should return blocker's output");

    // Assert metadata includes echo and blocker, but NOT enrich-metadata
    let metadata = output.metadata.expect("Should have metadata");
    let metadata_obj = metadata.as_object().unwrap();

    assert!(
        metadata_obj.contains_key("echo"),
        "Should have metadata from echo (executed)"
    );
    assert!(
        metadata_obj.contains_key("blocker"),
        "Should have metadata from blocker (executed)"
    );
    assert!(
        !metadata_obj.contains_key("enrich-metadata"),
        "Should NOT have metadata from enrich-metadata (not executed)"
    );

    println!("✓ Chain termination test passed");
    println!("  Chain stopped at blocker plugin (plugin 2/3)");
}

#[tokio::test]
async fn test_metadata_preserved_through_chain() {
    // Test that each plugin's metadata is preserved in the final output
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    let mut servers = HashMap::new();
    servers.insert(
        "metadata-test".to_string(),
        ServerPluginConfig {
            request: vec![],
            response: vec![
                PluginAssignment {
                    name: "echo".to_string(),
                    order: 1,
                    enabled: true,
                    timeout_ms: None,
                },
                PluginAssignment {
                    name: "enrich-metadata".to_string(),
                    order: 2,
                    enabled: true,
                    timeout_ms: None,
                },
            ],
        },
    );

    let config = Arc::new(PluginConfig {
        plugin_dir,
        node_executable: PathBuf::from("node"),
        max_concurrent_executions: 10,
        pool_size_per_plugin: 5,
        default_timeout_ms: 30000,
        servers,
    });

    let manager = Arc::new(PluginManager::new((*config).clone()));
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    let chain = PluginChain::new(
        "metadata-test".to_string(),
        PluginPhase::Response,
        manager,
        config,
    );

    let input = PluginInput {
        tool_name: "test/metadata".to_string(),
        raw_content: "test data".to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-chain-003".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "metadata-test".to_string(),
            phase: PluginPhase::Response,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = chain.execute(input).await;
    assert!(result.is_ok(), "Chain execution failed");

    let output = result.unwrap();

    // Assert metadata is aggregated
    let metadata = output.metadata.expect("Should have aggregated metadata");
    let metadata_obj = metadata.as_object().unwrap();

    assert_eq!(metadata_obj.len(), 2, "Should have metadata from 2 plugins");
    assert!(metadata_obj.contains_key("echo"));
    assert!(metadata_obj.contains_key("enrich-metadata"));

    // Verify enrich-metadata added its fields
    let enrich_meta = metadata_obj.get("enrich-metadata").unwrap();
    assert!(
        enrich_meta.get("processedAt").is_some(),
        "Should have processedAt timestamp"
    );
    assert!(
        enrich_meta.get("contentLength").is_some(),
        "Should have contentLength"
    );

    println!("✓ Metadata preservation test passed");
    let keys: Vec<&String> = metadata_obj.keys().collect();
    println!("  Metadata keys: {:?}", keys);
}
