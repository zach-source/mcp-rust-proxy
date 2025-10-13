# Contract: Message Translation Rules

**Feature**: MCP Protocol Version Support
**Contract Type**: Translation Specification
**Date**: 2025-10-12

## Overview

This document specifies the exact translation rules for converting MCP messages between protocol versions. These rules are implemented by the protocol adapters and ensure semantic equivalence across version boundaries.

---

## Translation Principles

### Principle 1: Semantic Preservation

**Rule**: Translation MUST preserve the semantic meaning of messages, even if exact structure changes.

**Example**: When converting `AudioContent` to `TextContent` (downgrade to 2024-11-05):
- Original: `{ "type": "audio", "data": "base64...", "mimeType": "audio/mp3" }`
- Translated: `{ "type": "text", "text": "[Audio content: audio/mp3]" }`
- Semantic meaning preserved: "There was audio content here with this MIME type"

---

### Principle 2: Data Loss Minimization

**Rule**: When data must be lost during translation, log a warning and preserve as much context as possible.

**Example**: When stripping `Tool.outputSchema` (downgrade to 2024-11-05):
```rust
if tool.output_schema.is_some() {
    tracing::warn!(
        tool_name = %tool.name,
        "Stripping outputSchema field during downgrade to 2024-11-05"
    );
    tool.output_schema = None;
}
```

---

### Principle 3: Forward Compatibility

**Rule**: Preserve unknown fields when possible to maintain forward compatibility.

**Implementation**: Use `#[serde(flatten)]` or manual JSON handling to preserve extra fields.

---

### Principle 4: Fail Explicitly

**Rule**: If translation cannot be performed correctly, return an error rather than silently corrupting data.

**Example**: If `ResourceContents.uri` is invalid and cannot be parsed for name generation:
```rust
Err(ProtocolError::TranslationError {
    from_version: ProtocolVersion::V20241105,
    to_version: ProtocolVersion::V20250618,
    message_type: "ResourceContents".to_string(),
    details: "Cannot generate name from invalid URI".to_string(),
})
```

---

## Version-Specific Translation Rules

### 1. Initialize Request Translation

**Direction**: Bidirectional (client ↔ proxy, proxy ↔ server)

#### Proxy (2025-06-18) → Server (2024-11-05)

**Changes**:
- Strip `elicitation` from `clientCapabilities`
- Preserve all other fields

**Implementation**:
```rust
fn translate_initialize_request_to_v20241105(
    request: InitializeRequest,
) -> InitializeRequest {
    let mut params = request.params;

    // Remove elicitation capability
    params.capabilities.elicitation = None;

    InitializeRequest {
        params,
        ..request
    }
}
```

#### Proxy (2025-06-18) → Server (2025-03-26)

**Changes**:
- Strip `elicitation` from `clientCapabilities`
- Preserve all other fields

**Implementation**: Same as above

#### Proxy (2025-06-18) → Server (2025-06-18)

**Changes**: None (pass-through)

---

### 2. Initialize Response Translation

**Direction**: Server → Proxy → Client

#### Server (2024-11-05) → Proxy (2025-06-18)

**Changes**:
- Preserve all fields (2024-11-05 is subset of 2025-06-18)
- No additions needed

**Implementation**: Pass-through (no translation needed)

#### Server (2025-03-26) → Proxy (2025-06-18)

**Changes**:
- Preserve all fields
- `completions` capability is already present in 2025-03-26

**Implementation**: Pass-through (no translation needed)

#### Server (2025-06-18) → Proxy (2024-11-05)

**Changes**:
- Strip `completions` from `serverCapabilities`
- Preserve all other fields

**Implementation**:
```rust
fn translate_initialize_response_to_v20241105(
    response: InitializeResponse,
) -> InitializeResponse {
    let mut result = response.result;

    // Remove completions capability
    result.capabilities.completions = None;

    InitializeResponse {
        result,
        ..response
    }
}
```

---

### 3. tools/list Request Translation

**Direction**: Client → Proxy → Server

#### All Versions

**Changes**: None (request structure is identical across all versions)

**Implementation**: Pass-through

---

### 4. tools/list Response Translation

**Direction**: Server → Proxy → Client

#### Server (2025-06-18) → Client (2024-11-05)

**Changes**:
- Strip `title` field from each Tool
- Strip `outputSchema` field from each Tool
- Preserve `name`, `description`, `inputSchema`

**Implementation**:
```rust
fn translate_tools_list_response_to_v20241105(
    response: ListToolsResponse,
) -> ListToolsResponse {
    let tools: Vec<Tool> = response.tools
        .into_iter()
        .map(|tool| {
            // Log if stripping non-empty title
            if let Some(title) = &tool.title {
                if !title.is_empty() {
                    tracing::warn!(
                        tool_name = %tool.name,
                        title = %title,
                        "Stripping non-empty title field during downgrade"
                    );
                }
            }

            // Log if stripping output schema
            if tool.output_schema.is_some() {
                tracing::info!(
                    tool_name = %tool.name,
                    "Stripping outputSchema field during downgrade"
                );
            }

            Tool {
                name: tool.name,
                description: tool.description,
                input_schema: tool.input_schema,
                title: None,
                output_schema: None,
            }
        })
        .collect();

    ListToolsResponse {
        tools,
        next_cursor: response.next_cursor,
    }
}
```

#### Server (2024-11-05) → Client (2025-06-18)

**Changes**:
- Add empty `title` field (None)
- Add empty `outputSchema` field (None)

**Implementation**:
```rust
fn translate_tools_list_response_to_v20250618(
    response: ListToolsResponse,
) -> ListToolsResponse {
    let tools: Vec<Tool> = response.tools
        .into_iter()
        .map(|tool| Tool {
            name: tool.name,
            description: tool.description,
            input_schema: tool.input_schema,
            title: None, // Add empty optional field
            output_schema: None, // Add empty optional field
        })
        .collect();

    ListToolsResponse {
        tools,
        next_cursor: response.next_cursor,
    }
}
```

#### Server (2025-03-26) → Client (2025-06-18)

**Changes**: Same as 2024-11-05 → 2025-06-18 (2025-03-26 doesn't have title/outputSchema)

---

### 5. tools/call Request Translation

**Direction**: Client → Proxy → Server

#### All Versions

**Changes**: None (request structure is identical across all versions)

**Implementation**: Pass-through

---

### 6. tools/call Response Translation

**Direction**: Server → Proxy → Client

#### Server (2025-06-18) → Client (2024-11-05)

**Changes**:
- Strip `structuredContent` field
- Convert `AudioContent` to `TextContent` in content array
- Preserve `content` and `isError` fields

**Implementation**:
```rust
fn translate_tool_call_response_to_v20241105(
    response: CallToolResponse,
) -> CallToolResponse {
    // Log if stripping structured content
    if response.structured_content.is_some() {
        tracing::info!("Stripping structuredContent during downgrade");
    }

    // Convert content items
    let content: Vec<Content> = response.content
        .into_iter()
        .map(|item| match item {
            Content::Audio { mime_type, .. } => {
                tracing::warn!(
                    mime_type = %mime_type,
                    "Converting AudioContent to TextContent during downgrade"
                );
                Content::Text {
                    text: format!("[Audio content: {}]", mime_type),
                }
            }
            Content::Resource { resource } => {
                // Also need to translate ResourceContents
                Content::Resource {
                    resource: translate_resource_contents_to_v20241105(resource),
                }
            }
            other => other, // Text and Image unchanged
        })
        .collect();

    CallToolResponse {
        content,
        structured_content: None, // Stripped
        is_error: response.is_error,
    }
}
```

#### Server (2025-06-18) → Client (2025-03-26)

**Changes**:
- Strip `structuredContent` field
- Preserve `AudioContent` (exists in 2025-03-26)
- Convert `ResourceContents` (if needed)

**Implementation**: Similar to above, but keep AudioContent

#### Server (2024-11-05) → Client (2025-06-18)

**Changes**:
- Add empty `structuredContent` field (None)
- Translate `ResourceContents` to V2 format (add name)

**Implementation**:
```rust
fn translate_tool_call_response_to_v20250618(
    response: CallToolResponse,
) -> CallToolResponse {
    let content: Vec<Content> = response.content
        .into_iter()
        .map(|item| match item {
            Content::Resource { resource } => {
                Content::Resource {
                    resource: translate_resource_contents_to_v20250618(resource),
                }
            }
            other => other,
        })
        .collect();

    CallToolResponse {
        content,
        structured_content: None, // Add empty optional field
        is_error: response.is_error,
    }
}
```

---

### 7. resources/list Request Translation

**Direction**: Client → Proxy → Server

#### All Versions

**Changes**: None (request structure is identical)

**Implementation**: Pass-through

---

### 8. resources/list Response Translation

**Direction**: Server → Proxy → Client

#### Server (2025-06-18) → Client (2024-11-05)

**Changes**:
- Strip `title` field from each Resource
- Preserve `uri`, `name`, `description`, `mimeType`

**Implementation**:
```rust
fn translate_resources_list_response_to_v20241105(
    response: ListResourcesResponse,
) -> ListResourcesResponse {
    let resources: Vec<Resource> = response.resources
        .into_iter()
        .map(|resource| {
            if let Some(title) = &resource.title {
                if !title.is_empty() {
                    tracing::warn!(
                        resource_name = %resource.name,
                        title = %title,
                        "Stripping non-empty title field during downgrade"
                    );
                }
            }

            Resource {
                uri: resource.uri,
                name: resource.name,
                description: resource.description,
                mime_type: resource.mime_type,
                title: None, // Stripped
            }
        })
        .collect();

    ListResourcesResponse {
        resources,
        next_cursor: response.next_cursor,
    }
}
```

#### Server (2024-11-05) → Client (2025-06-18)

**Changes**:
- Add empty `title` field (None)

**Implementation**:
```rust
fn translate_resources_list_response_to_v20250618(
    response: ListResourcesResponse,
) -> ListResourcesResponse {
    let resources: Vec<Resource> = response.resources
        .into_iter()
        .map(|resource| Resource {
            uri: resource.uri,
            name: resource.name,
            description: resource.description,
            mime_type: resource.mime_type,
            title: None, // Add empty optional field
        })
        .collect();

    ListResourcesResponse {
        resources,
        next_cursor: response.next_cursor,
    }
}
```

---

### 9. resources/read Request Translation

**Direction**: Client → Proxy → Server

#### All Versions

**Changes**: None (request structure is identical)

**Implementation**: Pass-through

---

### 10. resources/read Response Translation

**Direction**: Server → Proxy → Client

#### Server (2024-11-05) → Client (2025-06-18)

**Changes**:
- Add `name` field to each ResourceContents (REQUIRED in 2025-06-18)
- Add empty `title` field (None)
- Preserve `uri`, `mimeType`, `text`, `blob`

**Name Generation Algorithm**:
```rust
fn generate_resource_name(uri: &str) -> String {
    // Try to parse as URL
    if let Ok(url) = url::Url::parse(uri) {
        // For file:// URIs, extract filename
        if url.scheme() == "file" {
            if let Some(segments) = url.path_segments() {
                if let Some(last) = segments.last() {
                    if !last.is_empty() {
                        return last.to_string();
                    }
                }
            }
        }

        // For other URLs, get last path segment
        if let Some(segments) = url.path_segments() {
            if let Some(last) = segments.last() {
                if !last.is_empty() {
                    return last.to_string();
                }
            }
        }
    }

    // Fallback: use full URI
    uri.to_string()
}
```

**Examples**:
- `file:///home/user/document.txt` → `"document.txt"`
- `https://example.com/api/resource.json` → `"resource.json"`
- `custom://unique-id-12345` → `"custom://unique-id-12345"`
- `file:///` → `"file:///"`

**Implementation**:
```rust
fn translate_resource_contents_to_v20250618(
    contents: ResourceContentsV1,
) -> ResourceContentsV2 {
    ResourceContentsV2 {
        uri: contents.uri.clone(),
        name: generate_resource_name(&contents.uri),
        title: None,
        mime_type: contents.mime_type,
        text: contents.text,
        blob: contents.blob,
    }
}

fn translate_resources_read_response_to_v20250618(
    response: ReadResourceResponse,
) -> ReadResourceResponse {
    let contents: Vec<ResourceContentsV2> = response.contents
        .into_iter()
        .map(|c| translate_resource_contents_to_v20250618(c))
        .collect();

    ReadResourceResponse { contents }
}
```

#### Server (2025-06-18) → Client (2024-11-05)

**Changes**:
- Strip `name` field from each ResourceContents
- Strip `title` field from each ResourceContents
- Preserve `uri`, `mimeType`, `text`, `blob`

**Implementation**:
```rust
fn translate_resource_contents_to_v20241105(
    contents: ResourceContentsV2,
) -> ResourceContentsV1 {
    ResourceContentsV1 {
        uri: contents.uri,
        mime_type: contents.mime_type,
        text: contents.text,
        blob: contents.blob,
    }
}

fn translate_resources_read_response_to_v20241105(
    response: ReadResourceResponse,
) -> ReadResourceResponse {
    let contents: Vec<ResourceContentsV1> = response.contents
        .into_iter()
        .map(|c| translate_resource_contents_to_v20241105(c))
        .collect();

    ReadResourceResponse { contents }
}
```

---

### 11. prompts/list Request Translation

**Direction**: Client → Proxy → Server

#### All Versions

**Changes**: None (request structure is identical)

**Implementation**: Pass-through

---

### 12. prompts/list Response Translation

**Direction**: Server → Proxy → Client

#### Server (2025-06-18) → Client (2024-11-05)

**Changes**:
- Strip `title` field from each Prompt
- Preserve `name`, `description`, `arguments`

**Implementation**:
```rust
fn translate_prompts_list_response_to_v20241105(
    response: ListPromptsResponse,
) -> ListPromptsResponse {
    let prompts: Vec<Prompt> = response.prompts
        .into_iter()
        .map(|prompt| {
            if let Some(title) = &prompt.title {
                if !title.is_empty() {
                    tracing::warn!(
                        prompt_name = %prompt.name,
                        title = %title,
                        "Stripping non-empty title field during downgrade"
                    );
                }
            }

            Prompt {
                name: prompt.name,
                description: prompt.description,
                arguments: prompt.arguments,
                title: None, // Stripped
            }
        })
        .collect();

    ListPromptsResponse {
        prompts,
        next_cursor: response.next_cursor,
    }
}
```

#### Server (2024-11-05) → Client (2025-06-18)

**Changes**:
- Add empty `title` field (None)

**Implementation**:
```rust
fn translate_prompts_list_response_to_v20250618(
    response: ListPromptsResponse,
) -> ListPromptsResponse {
    let prompts: Vec<Prompt> = response.prompts
        .into_iter()
        .map(|prompt| Prompt {
            name: prompt.name,
            description: prompt.description,
            arguments: prompt.arguments,
            title: None, // Add empty optional field
        })
        .collect();

    ListPromptsResponse {
        prompts,
        next_cursor: response.next_cursor,
    }
}
```

---

### 13. prompts/get Request Translation

**Direction**: Client → Proxy → Server

#### All Versions

**Changes**: None (request structure is identical)

**Implementation**: Pass-through

---

### 14. prompts/get Response Translation

**Direction**: Server → Proxy → Client

#### Server (2025-06-18) → Client (2024-11-05)

**Changes**:
- Convert `AudioContent` to `TextContent` in message content arrays
- Convert `ResourceContents` to V1 format

**Implementation**:
```rust
fn translate_prompts_get_response_to_v20241105(
    response: GetPromptResponse,
) -> GetPromptResponse {
    let messages: Vec<PromptMessage> = response.messages
        .into_iter()
        .map(|message| {
            let content = message.content
                .into_iter()
                .map(|item| match item {
                    Content::Audio { mime_type, .. } => {
                        Content::Text {
                            text: format!("[Audio content: {}]", mime_type),
                        }
                    }
                    Content::Resource { resource } => {
                        Content::Resource {
                            resource: translate_resource_contents_to_v20241105(resource),
                        }
                    }
                    other => other,
                })
                .collect();

            PromptMessage {
                role: message.role,
                content,
            }
        })
        .collect();

    GetPromptResponse {
        description: response.description,
        messages,
    }
}
```

#### Server (2024-11-05) → Client (2025-06-18)

**Changes**:
- Convert `ResourceContents` to V2 format (add name)

**Implementation**: Similar to above, but only handle ResourceContents

---

### 15. Notifications Translation

**Direction**: Server → Proxy → Client

#### initialized

**Changes**: None (identical across all versions)

**Implementation**: Pass-through

#### resources/list_changed

**Changes**: None (identical across all versions)

**Implementation**: Pass-through

#### tools/list_changed

**Changes**: None (identical across all versions)

**Implementation**: Pass-through

#### prompts/list_changed

**Changes**: None (identical across all versions)

**Implementation**: Pass-through

#### resources/updated

**Availability**: 2025-03-26 and later only

**Translation Rules**:
- Server (2025-03-26+) → Client (2024-11-05): **Drop notification** (not supported)
- Server (2024-11-05) → Client (2025-03-26+): N/A (notification doesn't exist in source)

**Implementation**:
```rust
fn translate_notification_to_v20241105(
    notification: JsonRpcNotification,
) -> Option<JsonRpcNotification> {
    if notification.method == "notifications/resources/updated" {
        tracing::debug!(
            "Dropping resources/updated notification (not supported in 2024-11-05)"
        );
        return None; // Drop notification
    }
    Some(notification) // Pass through
}
```

#### progress

**Changes**: None (identical across all versions)

**Implementation**: Pass-through

---

## Content Type Translation

### Text Content

**All Versions**: Identical structure, no translation needed

```rust
{
    "type": "text",
    "text": "string"
}
```

---

### Image Content

**All Versions**: Identical structure, no translation needed

```rust
{
    "type": "image",
    "data": "base64-encoded-string",
    "mimeType": "image/png"
}
```

---

### Audio Content

**Availability**: 2025-03-26 and later

**Structure (2025-03-26+)**:
```rust
{
    "type": "audio",
    "data": "base64-encoded-string",
    "mimeType": "audio/mp3"
}
```

**Translation to 2024-11-05**:
```rust
{
    "type": "text",
    "text": "[Audio content: audio/mp3]"
}
```

**Implementation**:
```rust
fn translate_audio_content_to_text(audio: AudioContent) -> TextContent {
    TextContent {
        text: format!("[Audio content: {}]", audio.mime_type),
    }
}
```

---

### Embedded Resource

**Changes**: ResourceContents format varies by version

**Translation**: Apply ResourceContents translation rules

```rust
fn translate_embedded_resource(
    resource: EmbeddedResource,
    target_version: ProtocolVersion,
) -> EmbeddedResource {
    EmbeddedResource {
        resource: match target_version {
            ProtocolVersion::V20241105 | ProtocolVersion::V20250326 => {
                translate_resource_contents_to_v1(resource.resource)
            }
            ProtocolVersion::V20250618 => {
                translate_resource_contents_to_v2(resource.resource)
            }
        }
    }
}
```

---

## Validation After Translation

### Required Validation Checks

After translation, validate that:

1. **JSON-RPC Structure**:
   - `jsonrpc` is `"2.0"`
   - `id` is present (for requests/responses) or absent (for notifications)
   - `method` is present (for requests/notifications)
   - `result` XOR `error` is present (for responses)

2. **Version-Specific Required Fields**:
   - 2025-06-18: ResourceContents MUST have `name` field
   - All versions: Tool MUST have `name`, `description`, `inputSchema`
   - All versions: Resource MUST have `uri`, `name`

3. **Field Types**:
   - String fields are strings
   - Objects are objects
   - Arrays are arrays

**Implementation**:
```rust
fn validate_translated_message(
    message: &JsonRpcMessage,
    target_version: ProtocolVersion,
) -> Result<(), ProtocolError> {
    // Validate JSON-RPC structure
    validate_jsonrpc_structure(message)?;

    // Validate version-specific requirements
    match target_version {
        ProtocolVersion::V20250618 => {
            validate_v20250618_requirements(message)?;
        }
        ProtocolVersion::V20250326 => {
            validate_v20250326_requirements(message)?;
        }
        ProtocolVersion::V20241105 => {
            validate_v20241105_requirements(message)?;
        }
    }

    Ok(())
}
```

---

## Testing Translation Rules

### Test Matrix

For each translation rule, test:

1. **Valid Input**: Translation succeeds with expected output
2. **Missing Optional Fields**: Translation succeeds
3. **Extra Fields**: Translation preserves or strips as expected
4. **Invalid Input**: Translation returns appropriate error

### Example Test Cases

**Test: Translate Tool (2025-06-18 → 2024-11-05)**
```rust
#[test]
fn test_translate_tool_downgrade() {
    let tool_v2 = Tool {
        name: "test-tool".to_string(),
        title: Some("Test Tool".to_string()),
        description: "A test tool".to_string(),
        input_schema: json!({"type": "object"}),
        output_schema: Some(json!({"type": "string"})),
    };

    let tool_v1 = translate_tool_to_v20241105(tool_v2);

    assert_eq!(tool_v1.name, "test-tool");
    assert_eq!(tool_v1.description, "A test tool");
    assert_eq!(tool_v1.input_schema, json!({"type": "object"}));
    assert!(tool_v1.title.is_none());
    assert!(tool_v1.output_schema.is_none());
}
```

**Test: Generate Resource Name**
```rust
#[test]
fn test_generate_resource_name() {
    assert_eq!(
        generate_resource_name("file:///home/user/doc.txt"),
        "doc.txt"
    );
    assert_eq!(
        generate_resource_name("https://example.com/api/resource.json"),
        "resource.json"
    );
    assert_eq!(
        generate_resource_name("custom://unique-id"),
        "custom://unique-id"
    );
}
```

**Test: Audio Content Conversion**
```rust
#[test]
fn test_audio_to_text_conversion() {
    let audio = AudioContent {
        data: "base64data".to_string(),
        mime_type: "audio/wav".to_string(),
    };

    let text = translate_audio_content_to_text(audio);

    assert_eq!(text.text, "[Audio content: audio/wav]");
}
```

---

## Performance Considerations

### Translation Overhead

**Target**: < 1ms P99 latency for typical messages

**Optimization Strategies**:
1. **Pass-Through**: Zero-copy when versions match
2. **Lazy Translation**: Only translate fields that actually differ
3. **JSON Streaming**: For large payloads, use streaming parser
4. **Field Caching**: Cache generated fields (e.g., resource names) if same URI appears multiple times

### Memory Usage

**Target**: < 2x message size during translation

**Strategies**:
1. Use `Cow<str>` for unchanged strings
2. Reuse allocations where possible
3. Avoid unnecessary clones

---

## Future Enhancements

### Possible Optimizations

1. **Compilation of Translation Rules**: Generate code from declarative rules
2. **Schema-Driven Translation**: Use JSON Schema diffs to automate translation
3. **Streaming Translation**: Handle large messages without loading entirely into memory
4. **Translation Caching**: Cache translated messages for repeated requests

**Note**: These are out of scope for current implementation.
