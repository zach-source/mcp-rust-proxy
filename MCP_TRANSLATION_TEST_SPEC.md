# MCP Protocol Translation Layer - Test Specification

## Overview
This document defines test cases for validating the MCP protocol translation layer that enables communication between clients and servers using different protocol versions.

---

## Test Environment Setup

### Test Fixtures
```rust
// Protocol versions
const V_2024_11_05: &str = "2024-11-05";
const V_2025_03_26: &str = "2025-03-26";
const V_2025_06_18: &str = "2025-06-18";

// Sample client info
fn sample_client_info() -> Implementation {
    Implementation {
        name: "test-client".to_string(),
        version: "1.0.0".to_string(),
    }
}

// Sample server info
fn sample_server_info() -> Implementation {
    Implementation {
        name: "test-server".to_string(),
        version: "1.0.0".to_string(),
    }
}
```

---

## Test Suite 1: Version Negotiation

### Test 1.1: Matching Versions (Pass-Through)
**Purpose**: Verify no translation occurs when versions match

| Test ID | Client Version | Server Version | Expected Result |
|---------|----------------|----------------|-----------------|
| 1.1.1 | 2024-11-05 | 2024-11-05 | Pass through, no modification |
| 1.1.2 | 2025-03-26 | 2025-03-26 | Pass through, no modification |
| 1.1.3 | 2025-06-18 | 2025-06-18 | Pass through, no modification |

**Test Code**:
```rust
#[test]
fn test_passthrough_same_version() {
    let versions = ["2024-11-05", "2025-03-26", "2025-06-18"];

    for version in versions {
        let init_request = create_init_request(version);
        let original = serde_json::to_value(&init_request).unwrap();

        let translated = translate_message(
            &init_request,
            version,
            version
        );

        let result = serde_json::to_value(&translated).unwrap();
        assert_eq!(original, result, "Pass-through failed for version {}", version);
    }
}
```

### Test 1.2: Version Detection
**Purpose**: Verify protocol version is correctly parsed

```rust
#[test]
fn test_version_detection() {
    assert_eq!(
        ProtocolVersion::from_string("2024-11-05"),
        Some(ProtocolVersion::V20241105)
    );
    assert_eq!(
        ProtocolVersion::from_string("2025-03-26"),
        Some(ProtocolVersion::V20250326)
    );
    assert_eq!(
        ProtocolVersion::from_string("2025-06-18"),
        Some(ProtocolVersion::V20250618)
    );
    assert_eq!(
        ProtocolVersion::from_string("invalid"),
        None
    );
}
```

---

## Test Suite 2: Initialize Request/Response Translation

### Test 2.1: Client Capabilities Translation (Forward)

**Test 2.1.1**: 2024-11-05 → 2025-06-18
```rust
#[test]
fn test_client_caps_forward_2024_to_2025() {
    let caps = ClientCapabilities {
        roots: Some(RootsCapability { list_changed: true }),
        sampling: Some(serde_json::json!({})),
        experimental: Some(HashMap::new()),
        elicitation: None, // Not supported in 2024-11-05
    };

    let result = translate_client_capabilities(
        &caps,
        ProtocolVersion::V20241105,
        ProtocolVersion::V20250618
    );

    // Should pass through as-is (no translation needed for forward compat)
    assert_eq!(result.roots, caps.roots);
    assert_eq!(result.sampling, caps.sampling);
}
```

### Test 2.2: Server Capabilities Translation (Backward)

**Test 2.2.1**: 2025-06-18 → 2024-11-05
```rust
#[test]
fn test_server_caps_backward_2025_to_2024() {
    let caps = ServerCapabilities {
        logging: Some(serde_json::json!({})),
        completions: Some(serde_json::json!({})), // Not in 2024-11-05
        prompts: Some(PromptsCapability { list_changed: true }),
        resources: Some(ResourcesCapability {
            subscribe: true,
            list_changed: true,
        }),
        tools: Some(ToolsCapability { list_changed: true }),
        experimental: Some(HashMap::new()),
    };

    let result = translate_server_capabilities(
        &caps,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );

    // Completions should be stripped
    assert!(result.completions.is_none());
    // Other capabilities should remain
    assert!(result.logging.is_some());
    assert!(result.prompts.is_some());
}
```

---

## Test Suite 3: Resources API Translation

### Test 3.1: resources/list Translation

**Test 3.1.1**: Resource with title (2025-06-18 → 2024-11-05)
```rust
#[test]
fn test_resource_list_strip_title() {
    let resource = Resource {
        uri: "file:///test.txt".to_string(),
        name: "test.txt".to_string(),
        title: Some("Test File".to_string()), // Should be stripped
        description: Some("A test file".to_string()),
        mime_type: Some("text/plain".to_string()),
    };

    let result = translate_resource(
        &resource,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );

    assert_eq!(result.uri, resource.uri);
    assert_eq!(result.name, resource.name);
    assert!(result.title.is_none()); // Title should be removed
    assert_eq!(result.description, resource.description);
}
```

### Test 3.2: resources/read Translation (BREAKING CHANGE)

**Test 3.2.1**: Add name field (2024-11-05 → 2025-06-18)
```rust
#[test]
fn test_resource_contents_add_name() {
    let contents = ResourceContents {
        uri: "file:///project/src/main.rs".to_string(),
        name: None, // Not present in 2024-11-05
        title: None,
        mime_type: Some("text/x-rust".to_string()),
        text: Some("fn main() {}".to_string()),
        blob: None,
    };

    let result = translate_resource_contents(
        &contents,
        ProtocolVersion::V20241105,
        ProtocolVersion::V20250618
    );

    // Name should be generated from URI
    assert!(result.name.is_some());
    assert_eq!(result.name.unwrap(), "main.rs");
    assert_eq!(result.uri, contents.uri);
}
```

**Test 3.2.2**: Name generation strategies
```rust
#[test]
fn test_generate_name_from_uri() {
    let test_cases = vec![
        ("file:///path/to/file.txt", "file.txt"),
        ("file:///single", "single"),
        ("https://example.com/api/data.json", "data.json"),
        ("custom://resource", "resource"),
        ("no-scheme", "no-scheme"),
    ];

    for (uri, expected_name) in test_cases {
        let name = generate_name_from_uri(uri);
        assert_eq!(name, expected_name, "Failed for URI: {}", uri);
    }
}
```

**Test 3.2.3**: Strip name field (2025-06-18 → 2024-11-05)
```rust
#[test]
fn test_resource_contents_strip_name() {
    let contents = ResourceContents {
        uri: "file:///test.txt".to_string(),
        name: Some("test.txt".to_string()), // Should be stripped
        title: Some("Test".to_string()),    // Should be stripped
        mime_type: Some("text/plain".to_string()),
        text: Some("content".to_string()),
        blob: None,
    };

    let result = translate_resource_contents(
        &contents,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );

    assert!(result.name.is_none());
    assert!(result.title.is_none());
    assert_eq!(result.uri, contents.uri);
    assert_eq!(result.mime_type, contents.mime_type);
}
```

---

## Test Suite 4: Tools API Translation

### Test 4.1: tools/list Translation

**Test 4.1.1**: Tool with outputSchema (2025-06-18 → 2024-11-05)
```rust
#[test]
fn test_tool_strip_output_schema() {
    let tool = Tool {
        name: "calculate".to_string(),
        title: Some("Calculator".to_string()),
        description: "Perform calculations".to_string(),
        input_schema: json_schema!({
            "type": "object",
            "properties": {
                "expression": { "type": "string" }
            }
        }),
        output_schema: Some(json_schema!({ // Should be stripped
            "type": "object",
            "properties": {
                "result": { "type": "number" }
            }
        })),
    };

    let result = translate_tool(
        &tool,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );

    assert!(result.title.is_none());
    assert!(result.output_schema.is_none());
    assert_eq!(result.name, tool.name);
    assert_eq!(result.input_schema, tool.input_schema);
}
```

### Test 4.2: tools/call Response Translation

**Test 4.2.1**: Strip structuredContent (2025-06-18 → 2024-11-05)
```rust
#[test]
fn test_tool_result_strip_structured_content() {
    let result = CallToolResult {
        content: vec![
            Content::Text(TextContent {
                text: "Result: 42".to_string(),
            })
        ],
        structured_content: Some(json!({ // Should be stripped
            "result": 42,
            "unit": "answer"
        })),
        is_error: false,
    };

    let translated = translate_call_tool_result(
        &result,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );

    assert!(translated.structured_content.is_none());
    assert_eq!(translated.content.len(), 1);
    assert_eq!(translated.is_error, false);
}
```

---

## Test Suite 5: Content Type Translation

### Test 5.1: AudioContent Handling

**Test 5.1.1**: Convert audio to text (2025-03-26 → 2024-11-05)
```rust
#[test]
fn test_audio_content_downgrade() {
    let audio = Content::Audio(AudioContent {
        data: "base64encodedaudio".to_string(),
        mime_type: "audio/mp3".to_string(),
    });

    let result = translate_content(
        &audio,
        ProtocolVersion::V20250326,
        ProtocolVersion::V20241105
    );

    match result {
        Content::Text(text) => {
            assert!(text.text.contains("Audio content"));
            assert!(text.text.contains("audio/mp3"));
        }
        _ => panic!("Expected TextContent"),
    }
}
```

**Test 5.1.2**: Preserve audio when supported (2025-03-26 → 2025-06-18)
```rust
#[test]
fn test_audio_content_preserve() {
    let audio = Content::Audio(AudioContent {
        data: "base64encodedaudio".to_string(),
        mime_type: "audio/mp3".to_string(),
    });

    let result = translate_content(
        &audio,
        ProtocolVersion::V20250326,
        ProtocolVersion::V20250618
    );

    assert!(matches!(result, Content::Audio(_)));
}
```

### Test 5.2: Mixed Content Arrays
```rust
#[test]
fn test_mixed_content_array_translation() {
    let contents = vec![
        Content::Text(TextContent { text: "Hello".to_string() }),
        Content::Image(ImageContent {
            data: "base64image".to_string(),
            mime_type: "image/png".to_string(),
        }),
        Content::Audio(AudioContent {
            data: "base64audio".to_string(),
            mime_type: "audio/wav".to_string(),
        }),
    ];

    let result = translate_content_array(
        &contents,
        ProtocolVersion::V20250326,
        ProtocolVersion::V20241105
    );

    assert_eq!(result.len(), 3);
    assert!(matches!(result[0], Content::Text(_)));
    assert!(matches!(result[1], Content::Image(_)));
    assert!(matches!(result[2], Content::Text(_))); // Audio converted
}
```

---

## Test Suite 6: Prompts API Translation

### Test 6.1: prompts/list Translation

**Test 6.1.1**: Prompt with title (2025-06-18 → 2024-11-05)
```rust
#[test]
fn test_prompt_strip_title() {
    let prompt = Prompt {
        name: "summarize".to_string(),
        title: Some("Summarize Document".to_string()), // Should be stripped
        description: Some("Summarize a document".to_string()),
        arguments: Some(vec![
            PromptArgument {
                name: "document".to_string(),
                description: Some("Document to summarize".to_string()),
                required: Some(true),
            }
        ]),
    };

    let result = translate_prompt(
        &prompt,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );

    assert!(result.title.is_none());
    assert_eq!(result.name, prompt.name);
    assert_eq!(result.description, prompt.description);
    assert_eq!(result.arguments, prompt.arguments);
}
```

---

## Test Suite 7: Notifications

### Test 7.1: Core Notifications (Unchanged)
```rust
#[test]
fn test_notifications_unchanged() {
    let notifications = vec![
        "notifications/initialized",
        "notifications/resources/list_changed",
        "notifications/tools/list_changed",
        "notifications/prompts/list_changed",
        "notifications/progress",
    ];

    for method in notifications {
        let notification = create_notification(method);
        let original = serde_json::to_value(&notification).unwrap();

        // Test all version combinations
        let versions = [
            ProtocolVersion::V20241105,
            ProtocolVersion::V20250326,
            ProtocolVersion::V20250618,
        ];

        for source_version in &versions {
            for target_version in &versions {
                let translated = translate_notification(
                    &notification,
                    *source_version,
                    *target_version
                );

                let result = serde_json::to_value(&translated).unwrap();
                assert_eq!(
                    original, result,
                    "Notification changed for {}: {:?} -> {:?}",
                    method, source_version, target_version
                );
            }
        }
    }
}
```

### Test 7.2: resources/updated Notification (2025-03-26+)
```rust
#[test]
fn test_resource_updated_notification() {
    let notification = Notification {
        method: "notifications/resources/updated".to_string(),
        params: Some(json!({
            "uri": "file:///test.txt"
        })),
    };

    // Should pass through for supported versions
    let result = translate_notification(
        &notification,
        ProtocolVersion::V20250326,
        ProtocolVersion::V20250618
    );
    assert_eq!(result.method, notification.method);

    // Should be handled (possibly dropped) for 2024-11-05
    // (Implementation decision: drop or pass through unknown notifications)
}
```

---

## Test Suite 8: Edge Cases and Error Handling

### Test 8.1: Malformed Messages
```rust
#[test]
fn test_malformed_json() {
    let invalid_json = r#"{"method": "initialize", "params": {"protocolVersion": 12345}}"#;
    let result = parse_and_translate(invalid_json, "2024-11-05", "2025-06-18");
    assert!(result.is_err());
}
```

### Test 8.2: Missing Required Fields
```rust
#[test]
fn test_missing_required_field() {
    let init_without_version = json!({
        "method": "initialize",
        "params": {
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    let result = parse_initialize_request(&init_without_version);
    assert!(result.is_err());
}
```

### Test 8.3: Unknown Protocol Version
```rust
#[test]
fn test_unknown_protocol_version() {
    let init = create_init_request("2099-12-31");
    let result = translate_message(&init, "2099-12-31", "2024-11-05");
    // Should fail gracefully or use fallback behavior
    assert!(result.is_err() || result.unwrap().is_some());
}
```

### Test 8.4: Preserve Unknown Fields
```rust
#[test]
fn test_preserve_unknown_fields() {
    let mut resource = json!({
        "uri": "file:///test.txt",
        "name": "test.txt",
        "description": "A test",
        "customField": "should be preserved",
        "futureFeature": {"complex": "object"}
    });

    let result = translate_resource_json(
        &resource,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );

    // Unknown fields should be preserved
    assert_eq!(result["customField"], "should be preserved");
    assert_eq!(result["futureFeature"]["complex"], "object");
}
```

### Test 8.5: Empty and Null Values
```rust
#[test]
fn test_empty_and_null_handling() {
    let resource = ResourceContents {
        uri: "file:///test.txt".to_string(),
        name: Some("".to_string()), // Empty name
        title: None,
        mime_type: None,
        text: Some("".to_string()), // Empty text
        blob: None,
    };

    let result = translate_resource_contents(
        &resource,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );

    // Should handle empty strings gracefully
    assert!(result.name.is_none()); // Stripped
    assert_eq!(result.text, Some("".to_string())); // Preserved
}
```

---

## Test Suite 9: Performance and Stress Tests

### Test 9.1: Large Message Translation
```rust
#[test]
fn test_large_resource_list() {
    let resources: Vec<Resource> = (0..10000)
        .map(|i| Resource {
            uri: format!("file:///resource{}.txt", i),
            name: format!("resource{}.txt", i),
            title: Some(format!("Resource {}", i)),
            description: Some(format!("Description {}", i)),
            mime_type: Some("text/plain".to_string()),
        })
        .collect();

    let start = std::time::Instant::now();
    let result = translate_resource_list(
        &resources,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );
    let duration = start.elapsed();

    assert_eq!(result.len(), 10000);
    assert!(duration.as_millis() < 1000, "Translation too slow: {:?}", duration);
}
```

### Test 9.2: Deep Nesting
```rust
#[test]
fn test_deeply_nested_content() {
    let mut content = json!({"type": "text", "text": "base"});

    // Create deeply nested structure
    for i in 0..100 {
        content = json!({
            "type": "resource",
            "resource": {
                "uri": format!("file:///level{}.txt", i),
                "name": format!("level{}.txt", i),
                "contents": [content]
            }
        });
    }

    let result = translate_content_json(
        &content,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );

    // Should handle deep nesting without stack overflow
    assert!(result.is_ok());
}
```

---

## Test Suite 10: Integration Tests

### Test 10.1: Full Initialize Handshake
```rust
#[tokio::test]
async fn test_full_initialize_handshake() {
    // Simulate full handshake with version mismatch
    let client_version = ProtocolVersion::V20250618;
    let server_version = ProtocolVersion::V20241105;

    // Step 1: Client sends initialize
    let client_init = InitializeRequest {
        method: "initialize".to_string(),
        params: InitializeParams {
            protocol_version: client_version.as_str().to_string(),
            capabilities: ClientCapabilities {
                roots: Some(RootsCapability { list_changed: true }),
                sampling: Some(json!({})),
                elicitation: Some(json!({})), // Not supported by server
                experimental: None,
            },
            client_info: sample_client_info(),
        },
    };

    // Step 2: Translate to server version
    let translated_init = translate_init_request(
        &client_init,
        client_version,
        server_version
    );

    // Step 3: Verify elicitation was stripped
    assert!(translated_init.params.capabilities.elicitation.is_none());

    // Step 4: Server responds
    let server_response = InitializeResult {
        protocol_version: server_version.as_str().to_string(),
        capabilities: ServerCapabilities {
            logging: Some(json!({})),
            completions: None, // Not supported in 2024-11-05
            prompts: Some(PromptsCapability { list_changed: true }),
            resources: Some(ResourcesCapability {
                subscribe: true,
                list_changed: true,
            }),
            tools: Some(ToolsCapability { list_changed: true }),
            experimental: None,
        },
        server_info: sample_server_info(),
        instructions: None,
    };

    // Step 5: Translate response back to client version
    let translated_response = translate_init_result(
        &server_response,
        server_version,
        client_version
    );

    // Step 6: Verify capabilities match negotiated version
    assert!(translated_response.capabilities.completions.is_none());
    assert_eq!(
        translated_response.protocol_version,
        server_version.as_str()
    );
}
```

### Test 10.2: tools/call Round Trip
```rust
#[tokio::test]
async fn test_tool_call_round_trip() {
    // Client: 2025-06-18, Server: 2024-11-05

    // Client sends tool call
    let client_request = CallToolRequest {
        method: "tools/call".to_string(),
        params: CallToolParams {
            name: "calculate".to_string(),
            arguments: Some(json!({
                "expression": "2 + 2"
            })),
        },
    };

    // Translate request (no changes needed)
    let server_request = translate_call_tool_request(
        &client_request,
        ProtocolVersion::V20250618,
        ProtocolVersion::V20241105
    );
    assert_eq!(server_request, client_request);

    // Server responds (old format)
    let server_response = CallToolResult {
        content: vec![
            Content::Text(TextContent {
                text: "Result: 4".to_string(),
            })
        ],
        structured_content: None, // Not supported
        is_error: false,
    };

    // Translate response back to client
    let client_response = translate_call_tool_result(
        &server_response,
        ProtocolVersion::V20241105,
        ProtocolVersion::V20250618
    );

    // Response should be identical (no structured content to add)
    assert_eq!(client_response.content, server_response.content);
    assert!(client_response.structured_content.is_none());
}
```

---

## Test Coverage Requirements

### Minimum Coverage Targets
- **Line Coverage**: 90%
- **Branch Coverage**: 85%
- **Function Coverage**: 95%

### Critical Paths (Must have 100% coverage)
1. Version detection and negotiation
2. ResourceContents.name generation/stripping
3. Capability filtering
4. Content type conversion

---

## Continuous Testing

### Regression Tests
- Run full test suite on every commit
- Test against reference implementations
- Validate against official MCP test vectors (if available)

### Compatibility Testing Matrix
Test all 9 version combinations:
```
[ ] 2024-11-05 ↔ 2024-11-05
[ ] 2024-11-05 ↔ 2025-03-26
[ ] 2024-11-05 ↔ 2025-06-18
[ ] 2025-03-26 ↔ 2024-11-05
[ ] 2025-03-26 ↔ 2025-03-26
[ ] 2025-03-26 ↔ 2025-06-18
[ ] 2025-06-18 ↔ 2024-11-05
[ ] 2025-06-18 ↔ 2025-03-26
[ ] 2025-06-18 ↔ 2025-06-18
```

---

## Test Execution Commands

```bash
# Run all tests
cargo test --package mcp-proxy --lib translation

# Run specific test suite
cargo test --package mcp-proxy --lib translation::tests::resources

# Run with coverage
cargo tarpaulin --out Html --output-dir coverage

# Run integration tests only
cargo test --test integration_tests

# Run with verbose output
cargo test -- --nocapture

# Run stress tests (long-running)
cargo test --release -- --ignored stress_test
```

---

## Manual Testing Checklist

### Real-World Scenarios
- [ ] Connect Claude Desktop (old client) to new server
- [ ] Connect new client to legacy filesystem server
- [ ] Mix of old and new servers in multi-server configuration
- [ ] Upgrade proxy while clients/servers are connected
- [ ] Server restart during active session
- [ ] Large file transfers (resources/read with multi-MB content)
- [ ] Rapid tool calls (stress test)
- [ ] Long-running tools with progress notifications
- [ ] Resource subscription updates
- [ ] Tool list changes during operation

---

## Debugging Tools

### Translation Logger
```rust
#[cfg(test)]
fn enable_translation_logging() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();
}
```

### Message Comparator
```rust
#[cfg(test)]
fn compare_messages(original: &Value, translated: &Value) -> Vec<Difference> {
    // Deep comparison utility for debugging translation issues
    find_differences(original, translated, "$")
}
```

### Test Message Generator
```rust
#[cfg(test)]
fn generate_test_messages(version: ProtocolVersion) -> Vec<TestMessage> {
    // Generate comprehensive set of test messages for a version
    vec![
        generate_init_request(version),
        generate_tool_list(version),
        generate_resource_read(version),
        // ... etc
    ]
}
```
