# Quickstart Guide: MCP Protocol Version Support

**Feature**: MCP Protocol Version Negotiation and Translation Layer
**Audience**: Developers implementing or testing protocol version support
**Date**: 2025-10-12

## Overview

This guide provides practical examples and workflows for implementing and testing multi-version MCP protocol support in the Rust proxy.

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Implementing a New Adapter](#implementing-a-new-adapter)
3. [Testing Your Implementation](#testing-your-implementation)
4. [Common Patterns](#common-patterns)
5. [Debugging Tips](#debugging-tips)
6. [Common Pitfalls](#common-pitfalls)

---

## Getting Started

### Prerequisites

- Rust 1.75 or later
- tokio runtime
- serde, serde_json for JSON handling
- Basic understanding of MCP protocol

### Project Structure

```
src/
├── protocol/
│   ├── mod.rs                    # Public API
│   ├── version.rs                # ProtocolVersion enum
│   ├── adapter.rs                # ProtocolAdapter trait
│   ├── adapters/
│   │   ├── mod.rs
│   │   ├── pass_through.rs       # PassThroughAdapter
│   │   ├── v20241105_to_v20250618.rs
│   │   ├── v20250618_to_v20241105.rs
│   │   └── ...
│   ├── state.rs                  # ServerConnectionState
│   └── error.rs                  # ProtocolError
├── types/
│   ├── jsonrpc.rs                # JSON-RPC message types
│   ├── mcp/
│   │   ├── v20241105.rs          # Version-specific types
│   │   ├── v20250326.rs
│   │   └── v20250618.rs
│   └── ...
```

### Quick Example: Using the Version Detection System

```rust
use mcp_rust_proxy::protocol::{ProtocolVersion, create_adapter};
use mcp_rust_proxy::types::JsonRpcResponse;

async fn handle_initialize_response(
    response: JsonRpcResponse,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Detect protocol version from response
    let server_version = ProtocolVersion::from_string(
        &response.result["protocolVersion"]
            .as_str()
            .unwrap()
    )?;

    println!("Detected server version: {:?}", server_version);

    // 2. Create appropriate adapter
    let proxy_version = ProtocolVersion::V20250618;
    let adapter = create_adapter(proxy_version, server_version);

    println!(
        "Created adapter: {} → {}",
        adapter.source_version().as_str(),
        adapter.target_version().as_str()
    );

    // 3. Use adapter for subsequent messages
    // (store in ServerConnectionState)

    Ok(())
}
```

---

## Implementing a New Adapter

### Step 1: Define the Adapter Struct

```rust
// src/protocol/adapters/v20250618_to_v20241105.rs

use async_trait::async_trait;
use crate::protocol::{ProtocolAdapter, ProtocolVersion, ProtocolError};
use crate::types::{JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};

pub struct V20250618ToV20241105Adapter;

impl V20250618ToV20241105Adapter {
    pub fn new() -> Self {
        Self
    }
}
```

### Step 2: Implement the ProtocolAdapter Trait

```rust
#[async_trait]
impl ProtocolAdapter for V20250618ToV20241105Adapter {
    fn source_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250618
    }

    fn target_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20241105
    }

    async fn translate_request(
        &self,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcRequest, ProtocolError> {
        // Most requests don't need translation
        // Only initialize request needs capability filtering
        if request.method == "initialize" {
            return self.translate_initialize_request(request);
        }

        Ok(request)
    }

    async fn translate_response(
        &self,
        response: JsonRpcResponse,
    ) -> Result<JsonRpcResponse, ProtocolError> {
        // Infer method from context or store it
        // For this example, we'll check the result structure
        if let Some(result) = &response.result {
            if result.get("tools").is_some() {
                return self.translate_tools_list_response(response);
            }
            if result.get("resources").is_some() {
                return self.translate_resources_list_response(response);
            }
            // ... other message types
        }

        Ok(response)
    }

    async fn translate_notification(
        &self,
        notification: JsonRpcNotification,
    ) -> Result<JsonRpcNotification, ProtocolError> {
        // Drop resources/updated notification (not supported in 2024-11-05)
        if notification.method == "notifications/resources/updated" {
            tracing::debug!(
                "Dropping resources/updated notification for 2024-11-05 client"
            );
            // Return a special marker or handle at call site
            return Err(ProtocolError::NotificationDropped {
                method: notification.method,
                reason: "Not supported in target version".to_string(),
            });
        }

        Ok(notification)
    }
}
```

### Step 3: Implement Helper Methods

```rust
impl V20250618ToV20241105Adapter {
    /// Translate initialize request (strip new capabilities)
    fn translate_initialize_request(
        &self,
        mut request: JsonRpcRequest,
    ) -> Result<JsonRpcRequest, ProtocolError> {
        if let Some(params) = request.params.as_mut() {
            if let Some(capabilities) = params.get_mut("capabilities") {
                // Remove elicitation capability
                if let Some(obj) = capabilities.as_object_mut() {
                    obj.remove("elicitation");
                }
            }
        }
        Ok(request)
    }

    /// Translate tools/list response (strip title and outputSchema)
    fn translate_tools_list_response(
        &self,
        mut response: JsonRpcResponse,
    ) -> Result<JsonRpcResponse, ProtocolError> {
        if let Some(result) = response.result.as_mut() {
            if let Some(tools) = result.get_mut("tools") {
                if let Some(tools_array) = tools.as_array_mut() {
                    for tool in tools_array.iter_mut() {
                        if let Some(tool_obj) = tool.as_object_mut() {
                            // Remove title field
                            if let Some(title) = tool_obj.remove("title") {
                                if !title.is_null() && title.as_str().unwrap_or("") != "" {
                                    tracing::warn!(
                                        tool_name = tool_obj.get("name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown"),
                                        title = title.as_str().unwrap_or(""),
                                        "Stripping non-empty title during downgrade"
                                    );
                                }
                            }

                            // Remove outputSchema field
                            if let Some(_) = tool_obj.remove("outputSchema") {
                                tracing::info!(
                                    tool_name = tool_obj.get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown"),
                                    "Stripping outputSchema during downgrade"
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(response)
    }

    /// Translate resources/list response (strip title)
    fn translate_resources_list_response(
        &self,
        mut response: JsonRpcResponse,
    ) -> Result<JsonRpcResponse, ProtocolError> {
        if let Some(result) = response.result.as_mut() {
            if let Some(resources) = result.get_mut("resources") {
                if let Some(resources_array) = resources.as_array_mut() {
                    for resource in resources_array.iter_mut() {
                        if let Some(resource_obj) = resource.as_object_mut() {
                            resource_obj.remove("title");
                        }
                    }
                }
            }
        }
        Ok(response)
    }
}
```

### Step 4: Register the Adapter

```rust
// src/protocol/adapter.rs

pub fn create_adapter(
    source_version: ProtocolVersion,
    target_version: ProtocolVersion,
) -> Box<dyn ProtocolAdapter> {
    use ProtocolVersion::*;

    if source_version == target_version {
        return Box::new(PassThroughAdapter::new(source_version));
    }

    match (source_version, target_version) {
        // Add your new adapter here
        (V20250618, V20241105) => Box::new(V20250618ToV20241105Adapter::new()),

        // Other adapters...
        _ => panic!("No adapter for {:?} → {:?}", source_version, target_version),
    }
}
```

---

## Testing Your Implementation

### Unit Tests

#### Test 1: Version Detection

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_protocol_version() {
        assert_eq!(
            ProtocolVersion::from_string("2024-11-05").unwrap(),
            ProtocolVersion::V20241105
        );
        assert_eq!(
            ProtocolVersion::from_string("2025-03-26").unwrap(),
            ProtocolVersion::V20250326
        );
        assert_eq!(
            ProtocolVersion::from_string("2025-06-18").unwrap(),
            ProtocolVersion::V20250618
        );
        assert!(ProtocolVersion::from_string("2026-01-01").is_err());
    }

    #[test]
    fn test_version_feature_support() {
        let v1 = ProtocolVersion::V20241105;
        let v2 = ProtocolVersion::V20250326;
        let v3 = ProtocolVersion::V20250618;

        assert!(!v1.supports_audio_content());
        assert!(v2.supports_audio_content());
        assert!(v3.supports_audio_content());

        assert!(!v1.requires_resource_name());
        assert!(!v2.requires_resource_name());
        assert!(v3.requires_resource_name());
    }
}
```

#### Test 2: Adapter Selection

```rust
#[test]
fn test_adapter_selection() {
    // Same version should use pass-through
    let adapter = create_adapter(
        ProtocolVersion::V20250618,
        ProtocolVersion::V20250618,
    );
    assert_eq!(adapter.source_version(), ProtocolVersion::V20250618);
    assert_eq!(adapter.target_version(), ProtocolVersion::V20250618);

    // Different versions should use translation adapter
    let adapter = create_adapter(
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105,
    );
    assert_eq!(adapter.source_version(), ProtocolVersion::V20250618);
    assert_eq!(adapter.target_version(), ProtocolVersion::V20241105);
}
```

#### Test 3: Translation Rules

```rust
#[tokio::test]
async fn test_translate_tools_list_response() {
    let adapter = V20250618ToV20241105Adapter::new();

    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: Some("1".into()),
        result: Some(json!({
            "tools": [
                {
                    "name": "test-tool",
                    "title": "Test Tool",
                    "description": "A test tool",
                    "inputSchema": {"type": "object"},
                    "outputSchema": {"type": "string"}
                }
            ]
        })),
        error: None,
    };

    let translated = adapter.translate_response(response).await.unwrap();

    let tools = translated.result.unwrap()["tools"].as_array().unwrap();
    let tool = &tools[0];

    assert_eq!(tool["name"], "test-tool");
    assert_eq!(tool["description"], "A test tool");
    assert!(tool.get("title").is_none());
    assert!(tool.get("outputSchema").is_none());
}
```

#### Test 4: Resource Name Generation

```rust
#[test]
fn test_generate_resource_name() {
    use crate::protocol::generate_resource_name;

    // File URI
    assert_eq!(
        generate_resource_name("file:///home/user/document.txt"),
        "document.txt"
    );

    // HTTP URL
    assert_eq!(
        generate_resource_name("https://example.com/api/resource.json"),
        "resource.json"
    );

    // Custom scheme
    assert_eq!(
        generate_resource_name("custom://unique-id-12345"),
        "custom://unique-id-12345"
    );

    // Edge case: no filename
    assert_eq!(
        generate_resource_name("file:///"),
        "file:///"
    );
}
```

### Integration Tests

#### Test 5: End-to-End Initialization

```rust
#[tokio::test]
async fn test_initialization_sequence() {
    // Setup mock backend server
    let mock_server = MockMcpServer::new(ProtocolVersion::V20250326);

    // Create proxy connection state
    let state = ServerConnectionState::new("test-server".to_string());

    // Send initialize request
    let init_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some("init-1".into()),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-proxy",
                "version": "0.1.0"
            }
        })),
    };

    state.start_initialization("init-1".to_string()).await.unwrap();

    // Simulate server response
    let init_response = mock_server.handle_initialize(init_request).await;

    // Detect version
    let server_version = ProtocolVersion::from_string(
        init_response.result.as_ref().unwrap()["protocolVersion"]
            .as_str()
            .unwrap()
    ).unwrap();

    assert_eq!(server_version, ProtocolVersion::V20250326);

    // Create adapter
    let adapter = create_adapter(ProtocolVersion::V20250618, server_version);
    state.set_adapter(adapter).await;

    state.received_initialize_response(server_version).await.unwrap();

    // Send initialized notification
    let initialized = JsonRpcNotification {
        jsonrpc: "2.0".to_string(),
        method: "notifications/initialized".to_string(),
        params: None,
    };

    mock_server.handle_notification(initialized).await;

    state.complete_initialization().await.unwrap();

    // Verify state
    assert!(state.is_ready().await);
    assert_eq!(state.protocol_version().await, Some(ProtocolVersion::V20250326));
}
```

### Property-Based Tests

#### Test 6: Round-Trip Translation

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_round_trip_translation(
        tool_name in "[a-z]{5,20}",
        tool_desc in "[a-z ]{10,100}",
    ) {
        // Create a tool with the generated data
        let original_tool = json!({
            "name": tool_name,
            "description": tool_desc,
            "inputSchema": {"type": "object"}
        });

        // Translate forward (2024-11-05 → 2025-06-18)
        let forward_adapter = V20241105ToV20250618Adapter::new();
        let forward_result = forward_adapter.translate_tool(original_tool.clone());

        // Translate backward (2025-06-18 → 2024-11-05)
        let backward_adapter = V20250618ToV20241105Adapter::new();
        let round_trip_result = backward_adapter.translate_tool(forward_result);

        // Should match original (semantically)
        prop_assert_eq!(round_trip_result["name"], original_tool["name"]);
        prop_assert_eq!(round_trip_result["description"], original_tool["description"]);
        prop_assert_eq!(round_trip_result["inputSchema"], original_tool["inputSchema"]);
    }
}
```

---

## Common Patterns

### Pattern 1: Method-Specific Translation

```rust
impl MyAdapter {
    async fn translate_response(
        &self,
        response: JsonRpcResponse,
    ) -> Result<JsonRpcResponse, ProtocolError> {
        // Use method context to dispatch to specific handler
        match self.infer_method(&response) {
            Some("tools/list") => self.translate_tools_list(response),
            Some("tools/call") => self.translate_tools_call(response),
            Some("resources/read") => self.translate_resources_read(response),
            _ => Ok(response), // Pass-through for unknown methods
        }
    }

    fn infer_method(&self, response: &JsonRpcResponse) -> Option<&str> {
        // Strategy 1: Store method in metadata (requires request tracking)
        // Strategy 2: Infer from response structure
        if let Some(result) = &response.result {
            if result.get("tools").is_some() {
                return Some("tools/list");
            }
            if result.get("contents").is_some() {
                return Some("resources/read");
            }
        }
        None
    }
}
```

### Pattern 2: Preserving Unknown Fields

```rust
fn strip_field_but_preserve_others(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    field_to_strip: &str,
) {
    // Only remove specific field, keep everything else
    obj.remove(field_to_strip);

    // Unknown fields are automatically preserved because
    // we're modifying the map directly
}
```

### Pattern 3: Conditional Logging

```rust
fn strip_title_with_logging(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    entity_name: &str,
) {
    if let Some(title) = obj.remove("title") {
        // Only log if the field had actual content
        if !title.is_null() {
            if let Some(title_str) = title.as_str() {
                if !title_str.is_empty() {
                    tracing::warn!(
                        entity = entity_name,
                        title = title_str,
                        "Stripping non-empty title field during downgrade"
                    );
                }
            }
        }
    }
}
```

### Pattern 4: Error Context

```rust
fn translate_with_context<T, F>(
    message_type: &str,
    translator: F,
) -> Result<T, ProtocolError>
where
    F: FnOnce() -> Result<T, serde_json::Error>,
{
    translator().map_err(|e| ProtocolError::TranslationError {
        from_version: self.source_version(),
        to_version: self.target_version(),
        message_type: message_type.to_string(),
        details: e.to_string(),
    })
}

// Usage
let translated = translate_with_context("tools/list", || {
    serde_json::from_value(response.result.unwrap())
})?;
```

---

## Debugging Tips

### Tip 1: Enable Detailed Logging

```rust
// In your test or main
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .with_target(true)
    .with_thread_ids(true)
    .with_line_number(true)
    .init();
```

### Tip 2: Print JSON Diffs

```rust
fn debug_translation(original: &JsonRpcResponse, translated: &JsonRpcResponse) {
    let original_json = serde_json::to_string_pretty(original).unwrap();
    let translated_json = serde_json::to_string_pretty(translated).unwrap();

    println!("=== ORIGINAL ===");
    println!("{}", original_json);
    println!("=== TRANSLATED ===");
    println!("{}", translated_json);
}
```

### Tip 3: Validate After Translation

```rust
async fn translate_and_validate(
    &self,
    response: JsonRpcResponse,
) -> Result<JsonRpcResponse, ProtocolError> {
    let translated = self.translate_response(response).await?;

    // Validate the translated message
    validate_jsonrpc_structure(&translated)?;
    validate_version_requirements(&translated, self.target_version())?;

    Ok(translated)
}
```

### Tip 4: Use Snapshot Testing

```rust
#[test]
fn test_translation_snapshot() {
    let adapter = V20250618ToV20241105Adapter::new();
    let response = load_test_fixture("tools_list_v20250618.json");

    let translated = adapter.translate_response(response).await.unwrap();

    // Compare with saved snapshot
    insta::assert_json_snapshot!(translated);
}
```

---

## Common Pitfalls

### Pitfall 1: Forgetting to Handle Nested Resources

**Problem**: ResourceContents can appear in multiple places (resources/read, tool call responses, prompt messages).

**Solution**: Create a central `translate_resource_contents()` function and call it everywhere.

```rust
fn translate_resource_contents(
    contents: serde_json::Value,
    target_version: ProtocolVersion,
) -> Result<serde_json::Value, ProtocolError> {
    // Centralized translation logic
    match target_version {
        ProtocolVersion::V20250618 => add_name_field(contents),
        ProtocolVersion::V20241105 | ProtocolVersion::V20250326 => {
            strip_name_and_title_fields(contents)
        }
    }
}
```

### Pitfall 2: Not Handling Audio Content in All Contexts

**Problem**: AudioContent can appear in tool responses AND prompt messages.

**Solution**: Use a visitor pattern or recursive content translator.

```rust
fn translate_content_array(
    contents: Vec<Content>,
    target_version: ProtocolVersion,
) -> Vec<Content> {
    contents.into_iter().map(|content| {
        match content {
            Content::Audio { mime_type, .. } if !target_version.supports_audio_content() => {
                Content::Text {
                    text: format!("[Audio content: {}]", mime_type),
                }
            }
            Content::Resource { resource } => {
                Content::Resource {
                    resource: translate_resource_contents(resource, target_version),
                }
            }
            other => other,
        }
    }).collect()
}
```

### Pitfall 3: Ignoring Pass-Through Optimization

**Problem**: Translating messages even when versions match.

**Solution**: Always check for version match first.

```rust
async fn translate_request(
    &self,
    request: JsonRpcRequest,
) -> Result<JsonRpcRequest, ProtocolError> {
    // Fast path
    if self.source_version() == self.target_version() {
        return Ok(request);
    }

    // Slow path (actual translation)
    self.translate_request_inner(request)
}
```

### Pitfall 4: Mutating Input Instead of Cloning

**Problem**: Accidentally modifying the original message.

**Solution**: Clone before modification or use owned types.

```rust
// BAD: Mutates input
async fn translate_response(&self, mut response: JsonRpcResponse) -> ... {
    response.result.as_mut().unwrap().remove("title");
    Ok(response)
}

// GOOD: Clones first
async fn translate_response(&self, response: JsonRpcResponse) -> ... {
    let mut translated = response.clone();
    translated.result.as_mut().unwrap().remove("title");
    Ok(translated)
}
```

### Pitfall 5: Not Testing All Version Pairs

**Problem**: Only testing one direction of translation.

**Solution**: Test matrix covering all combinations.

```rust
#[tokio::test]
async fn test_all_version_pairs() {
    let versions = vec![
        ProtocolVersion::V20241105,
        ProtocolVersion::V20250326,
        ProtocolVersion::V20250618,
    ];

    for source in &versions {
        for target in &versions {
            let adapter = create_adapter(*source, *target);

            // Test with sample message
            let sample = create_sample_message();
            let result = adapter.translate_request(sample.clone()).await;

            assert!(result.is_ok(), "Failed to translate {:?} → {:?}", source, target);
        }
    }
}
```

---

## Quick Reference Commands

### Run Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_translate_tools_list_response

# Run with logging
RUST_LOG=debug cargo test -- --nocapture

# Run integration tests only
cargo test --test integration_tests

# Run with coverage
cargo tarpaulin --out Html
```

### Format and Lint

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy -- -D warnings

# Check for unused dependencies
cargo +nightly udeps
```

### Benchmarks

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench translation_benchmark

# Generate flamegraph
cargo flamegraph --bench translation_benchmark
```

---

## Next Steps

1. **Read the Contracts**: Understand the [protocol-adapter-api.md](contracts/protocol-adapter-api.md) and [translation-rules.md](contracts/translation-rules.md)

2. **Implement Your Adapter**: Follow the patterns in this guide

3. **Write Tests**: Aim for 90%+ coverage on translation logic

4. **Test with Real Servers**: Connect to actual MCP servers with different versions

5. **Performance Profiling**: Ensure translation overhead is < 1ms P99

6. **Documentation**: Document any edge cases or special handling

---

## Resources

- **MCP Specification**: https://modelcontextprotocol.io/specification/
- **JSON-RPC 2.0**: https://www.jsonrpc.org/specification
- **Project Documentation**:
  - [research.md](research.md) - Research findings and decisions
  - [data-model.md](data-model.md) - Core entities and state machines
  - [contracts/](contracts/) - API and behavior contracts

---

## Getting Help

- Check the project's CLAUDE.md for development guidelines
- Review test files for examples
- Look at existing adapters for patterns
- Check tracing logs for runtime behavior
