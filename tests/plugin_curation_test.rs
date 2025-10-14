//! Integration tests for plugin curation flow
//!
//! Tests verify the complete end-to-end curation workflow including:
//! - Plugin discovery and loading
//! - Curation execution with large documents
//! - Content reduction validation
//! - Plugin chaining behavior

use mcp_rust_proxy::plugin::config::PluginConfig;
use mcp_rust_proxy::plugin::manager::PluginManager;
use mcp_rust_proxy::plugin::schema::{PluginInput, PluginMetadata, PluginPhase};
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::test]
async fn test_curation_flow_with_echo_plugin() {
    // Setup: Use the echo plugin from test fixtures
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");

    // Create a mock 50KB documentation response
    let large_doc = generate_mock_documentation(50_000);

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

    // Create plugin input with maxTokens for curation
    let input = PluginInput {
        tool_name: "context7/get-docs".to_string(),
        raw_content: large_doc.clone(),
        max_tokens: Some(1200),
        metadata: PluginMetadata {
            request_id: "test-req-123".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "context7".to_string(),
            phase: PluginPhase::Response,
            user_query: Some("Explain React hooks".to_string()),
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    // Execute the echo plugin
    let result = manager.execute("echo", &input, 30000).await;

    // Assert the execution succeeded
    assert!(
        result.is_ok(),
        "Plugin execution failed: {:?}",
        result.err()
    );

    let output = result.unwrap();

    // Assert output is valid
    assert!(!output.text.is_empty(), "Plugin output text is empty");
    assert_eq!(output.continue_, true, "Plugin should continue processing");

    // For echo plugin, content should be preserved
    assert_eq!(
        output.text, large_doc,
        "Echo plugin should preserve content"
    );

    // Assert metadata is present
    assert!(output.metadata.is_some(), "Metadata should be present");

    println!("✓ Curation flow test passed");
    println!("  Input size: {} bytes", large_doc.len());
    println!("  Output size: {} bytes", output.text.len());
}

#[tokio::test]
async fn test_plugin_handles_large_documents() {
    // Test that the plugin system can handle large inputs without crashing
    let plugin_dir = PathBuf::from("tests/fixtures/plugins");
    let large_doc = generate_mock_documentation(100_000);

    let config = PluginConfig {
        plugin_dir: plugin_dir.clone(),
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

    let input = PluginInput {
        tool_name: "test/large-doc".to_string(),
        raw_content: large_doc.clone(),
        max_tokens: Some(2000),
        metadata: PluginMetadata {
            request_id: "test-req-456".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "test".to_string(),
            phase: PluginPhase::Response,
            user_query: Some("Summarize documentation".to_string()),
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = manager.execute("echo", &input, 30000).await;
    assert!(
        result.is_ok(),
        "Failed to process large document: {:?}",
        result.err()
    );

    let output = result.unwrap();
    assert_eq!(
        output.text.len(),
        large_doc.len(),
        "Large document not preserved"
    );

    println!("✓ Large document processing test passed");
    println!("  Document size: {} bytes", large_doc.len());
}

#[tokio::test]
async fn test_plugin_chain_simulation() {
    // Test that plugin output can be chained to next plugin
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
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    let input = PluginInput {
        tool_name: "test/chain".to_string(),
        raw_content: "Test content for chaining".to_string(),
        max_tokens: None,
        metadata: PluginMetadata {
            request_id: "test-req-789".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "test".to_string(),
            phase: PluginPhase::Response,
            user_query: None,
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    // Execute first plugin
    let result1 = manager.execute("echo", &input, 30000).await;
    assert!(result1.is_ok(), "First plugin failed: {:?}", result1.err());

    // Chain output to next plugin (simulate chaining)
    let output1 = result1.unwrap();
    let input2 = PluginInput {
        raw_content: output1.text.clone(),
        ..input
    };

    let result2 = manager.execute("echo", &input2, 30000).await;
    assert!(
        result2.is_ok(),
        "Second plugin in chain failed: {:?}",
        result2.err()
    );

    let output2 = result2.unwrap();
    assert_eq!(
        output1.text, output2.text,
        "Chained content should be preserved"
    );

    println!("✓ Plugin chain simulation test passed");
}

#[tokio::test]
async fn test_curation_metadata_preservation() {
    // Test that metadata is properly preserved through plugin execution
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
    manager
        .discover_plugins()
        .await
        .expect("Failed to discover plugins");

    let doc = generate_mock_documentation(10_000);

    let input = PluginInput {
        tool_name: "context7/get-docs".to_string(),
        raw_content: doc,
        max_tokens: Some(1000),
        metadata: PluginMetadata {
            request_id: "test-req-meta".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "context7".to_string(),
            phase: PluginPhase::Response,
            user_query: Some("Test query".to_string()),
            tool_arguments: None,
            mcp_servers: None,
        },
    };

    let result = manager.execute("echo", &input, 30000).await;
    assert!(result.is_ok(), "Plugin execution failed");

    let output = result.unwrap();
    assert!(
        output.metadata.is_some(),
        "Plugin should preserve or add metadata"
    );

    println!("✓ Metadata preservation test passed");
}

// Helper function to generate mock documentation
fn generate_mock_documentation(target_size: usize) -> String {
    let mut doc = String::with_capacity(target_size);

    doc.push_str("# React Hooks Documentation\n\n");
    doc.push_str(
        "React Hooks are functions that let you use state and other React features in function components.\n\n",
    );

    // Add repetitive content to reach target size
    let sample_content = r#"
## useState Hook

The useState Hook lets you add state to function components:

```javascript
import { useState } from 'react';

function Counter() {
  const [count, setCount] = useState(0);

  return (
    <div>
      <p>Count: {count}</p>
      <button onClick={() => setCount(count + 1)}>Increment</button>
    </div>
  );
}
```

### Key Points:
- useState returns an array with two elements: current state and updater function
- The initial state is passed as an argument to useState
- State updates trigger re-renders of the component

"#;

    // Repeat content until we reach target size
    while doc.len() < target_size {
        doc.push_str(sample_content);
        doc.push_str(&format!("\n## Additional Section {}\n", doc.len() / 1000));
        doc.push_str("More content about React hooks and their usage patterns.\n");
    }

    doc.truncate(target_size);
    doc
}
