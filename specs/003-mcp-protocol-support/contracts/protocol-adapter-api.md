# API Contract: Protocol Adapter

**Feature**: MCP Protocol Version Support
**Contract Type**: Internal Rust Trait API
**Date**: 2025-10-12

## Overview

The Protocol Adapter API defines the contract for translating MCP messages between different protocol versions. All adapters must implement the `ProtocolAdapter` trait to ensure consistent behavior across version pairs.

---

## Trait Definition

```rust
use async_trait::async_trait;
use crate::protocol::{ProtocolVersion, ProtocolError};
use crate::types::{JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};

/// Protocol adapter trait for translating messages between versions
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// Get the source protocol version this adapter handles
    ///
    /// This is the version of messages coming INTO the adapter.
    fn source_version(&self) -> ProtocolVersion;

    /// Get the target protocol version this adapter produces
    ///
    /// This is the version of messages coming OUT of the adapter.
    fn target_version(&self) -> ProtocolVersion;

    /// Translate a JSON-RPC request from source to target version
    ///
    /// # Arguments
    /// * `request` - The request in source version format
    ///
    /// # Returns
    /// * `Ok(JsonRpcRequest)` - The translated request in target version format
    /// * `Err(ProtocolError)` - If translation fails
    ///
    /// # Errors
    /// * `ProtocolError::TranslationError` - If message cannot be translated
    /// * `ProtocolError::MissingRequiredField` - If required field is missing
    /// * `ProtocolError::JsonError` - If JSON is malformed
    async fn translate_request(
        &self,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcRequest, ProtocolError>;

    /// Translate a JSON-RPC response from target back to source version
    ///
    /// This is the reverse translation - typically used for responses from
    /// the backend server back to the client.
    ///
    /// # Arguments
    /// * `response` - The response in target version format
    ///
    /// # Returns
    /// * `Ok(JsonRpcResponse)` - The translated response in source version format
    /// * `Err(ProtocolError)` - If translation fails
    ///
    /// # Errors
    /// * `ProtocolError::TranslationError` - If message cannot be translated
    /// * `ProtocolError::MissingRequiredField` - If required field is missing
    /// * `ProtocolError::JsonError` - If JSON is malformed
    async fn translate_response(
        &self,
        response: JsonRpcResponse,
    ) -> Result<JsonRpcResponse, ProtocolError>;

    /// Translate a JSON-RPC notification from target back to source version
    ///
    /// Notifications are one-way messages with no response expected.
    ///
    /// # Arguments
    /// * `notification` - The notification in target version format
    ///
    /// # Returns
    /// * `Ok(JsonRpcNotification)` - The translated notification in source version format
    /// * `Err(ProtocolError)` - If translation fails
    ///
    /// # Errors
    /// * `ProtocolError::TranslationError` - If message cannot be translated
    /// * `ProtocolError::JsonError` - If JSON is malformed
    async fn translate_notification(
        &self,
        notification: JsonRpcNotification,
    ) -> Result<JsonRpcNotification, ProtocolError>;
}
```

---

## Contract Guarantees

### Implementer Responsibilities

All implementations of `ProtocolAdapter` MUST:

1. **Be Thread-Safe**: Implement `Send + Sync` (enforced by trait bounds)

2. **Preserve Message Integrity**:
   - JSON-RPC structure must remain valid after translation
   - Required fields in target version must be present
   - Unknown/experimental fields should be preserved when possible

3. **Handle All Message Types**:
   - initialize request/response
   - tools/list, tools/call
   - resources/list, resources/read, resources/subscribe
   - prompts/list, prompts/get
   - All notification types

4. **Version Consistency**:
   - `source_version()` and `target_version()` must be constant for the lifetime of the adapter
   - These methods must be fast (O(1)) as they may be called frequently

5. **Error Handling**:
   - Return `ProtocolError` with descriptive details
   - Never panic on malformed input
   - Log warnings for data loss (e.g., stripping non-empty fields)

6. **Idempotency**:
   - Translating the same message multiple times must produce the same result
   - No internal state should affect translation behavior

### Caller Responsibilities

Code using `ProtocolAdapter` MUST:

1. **Select Correct Adapter**: Use the adapter matching the actual version pair (source, target)

2. **Handle Errors**: Treat `ProtocolError` as fatal for that message (no automatic retry with different adapter)

3. **Respect Direction**:
   - Use `translate_request()` for client → server messages
   - Use `translate_response()` for server → client responses
   - Use `translate_notification()` for server → client notifications

4. **Avoid Double Translation**: Only translate once per message per direction

---

## Behavioral Contracts

### Pass-Through Adapter

When `source_version() == target_version()`:
- The adapter SHOULD be a `PassThroughAdapter` (zero-copy, no translation)
- `translate_request()` MUST return the input unchanged
- `translate_response()` MUST return the input unchanged
- `translate_notification()` MUST return the input unchanged

```rust
pub struct PassThroughAdapter {
    version: ProtocolVersion,
}

#[async_trait]
impl ProtocolAdapter for PassThroughAdapter {
    fn source_version(&self) -> ProtocolVersion {
        self.version
    }

    fn target_version(&self) -> ProtocolVersion {
        self.version
    }

    async fn translate_request(&self, request: JsonRpcRequest)
        -> Result<JsonRpcRequest, ProtocolError>
    {
        Ok(request) // Zero-copy pass-through
    }

    async fn translate_response(&self, response: JsonRpcResponse)
        -> Result<JsonRpcResponse, ProtocolError>
    {
        Ok(response)
    }

    async fn translate_notification(&self, notification: JsonRpcNotification)
        -> Result<JsonRpcNotification, ProtocolError>
    {
        Ok(notification)
    }
}
```

---

### Forward Translation Adapter (Old → New)

When translating FROM an older version TO a newer version:

**Behavior**:
1. **Add Missing Fields**: Generate values for newly required fields
   - Example: Generate `ResourceContents.name` from URI when upgrading to 2025-06-18
2. **Preserve Existing Fields**: Keep all fields from source version
3. **Add Empty Optional Fields**: Set new optional fields to None/null
4. **No Field Removal**: Never remove fields (newer versions are supersets)

**Example**:
```rust
// 2024-11-05 → 2025-06-18
async fn translate_request(&self, request: JsonRpcRequest)
    -> Result<JsonRpcRequest, ProtocolError>
{
    if request.method == "resources/read" {
        // Response will need translation
        // Request itself is unchanged
        Ok(request)
    } else {
        // Most requests don't need translation
        Ok(request)
    }
}

async fn translate_response(&self, response: JsonRpcResponse)
    -> Result<JsonRpcResponse, ProtocolError>
{
    if let Some(method) = infer_method_from_response(&response) {
        if method == "resources/read" {
            // Add 'name' field to ResourceContents
            return self.add_resource_name_field(response);
        }
    }
    Ok(response)
}
```

---

### Backward Translation Adapter (New → Old)

When translating FROM a newer version TO an older version:

**Behavior**:
1. **Remove New Fields**: Strip fields that don't exist in target version
   - Example: Remove `Tool.title` when downgrading to 2024-11-05
2. **Convert Incompatible Types**: Transform content types that don't exist in target version
   - Example: Convert `AudioContent` to `TextContent` when downgrading to 2024-11-05
3. **Filter Capabilities**: Remove capability flags not supported by target version
4. **Preserve Core Fields**: Keep all fields that exist in target version

**Data Loss Warnings**:
- MUST log at WARN level when stripping non-empty optional fields
- MUST log at INFO level when stripping empty optional fields
- MUST log at WARN level when converting content types (AudioContent → TextContent)

**Example**:
```rust
// 2025-06-18 → 2024-11-05
async fn translate_response(&self, response: JsonRpcResponse)
    -> Result<JsonRpcResponse, ProtocolError>
{
    let mut translated = response.clone();

    // Strip version-specific fields
    if let Some(result) = translated.result.as_mut() {
        self.strip_title_fields(result)?;
        self.strip_output_schema(result)?;
        self.strip_structured_content(result)?;
        self.convert_audio_to_text(result)?;
    }

    Ok(translated)
}
```

---

## Method-Specific Contracts

### translate_request()

**Input**: JSON-RPC request in source version format
**Output**: JSON-RPC request in target version format

**Invariants**:
- `request.jsonrpc` must remain `"2.0"`
- `request.id` must be preserved unchanged
- `request.method` must be preserved unchanged
- `request.params` may be modified based on version differences

**Special Cases**:
- `initialize` request: May need to add/remove capability flags
- Other requests: Usually pass-through (changes are in responses)

---

### translate_response()

**Input**: JSON-RPC response in target version format
**Output**: JSON-RPC response in source version format

**Invariants**:
- `response.jsonrpc` must remain `"2.0"`
- `response.id` must be preserved unchanged
- `response.result` XOR `response.error` must remain present (exactly one)
- `response.result` may be modified based on version differences

**Special Cases**:
- `initialize` response: May need to filter capabilities
- `tools/list` response: May need to strip title/outputSchema fields
- `tools/call` response: May need to strip structuredContent, convert AudioContent
- `resources/read` response: May need to add/strip name field
- `resources/list` response: May need to strip title fields
- `prompts/list` response: May need to strip title fields

---

### translate_notification()

**Input**: JSON-RPC notification in target version format
**Output**: JSON-RPC notification in source version format

**Invariants**:
- `notification.jsonrpc` must remain `"2.0"`
- `notification.method` must be preserved unchanged
- `notification.params` may be modified based on version differences
- Notifications MUST NOT have an `id` field

**Special Cases**:
- Most notifications are version-agnostic and pass through unchanged
- `resources/updated` notification: Only exists in 2025-03-26+ (drop if translating to 2024-11-05)

---

## Error Handling Contract

### When to Return Errors

Return `ProtocolError` in these cases:

1. **TranslationError**:
   - Malformed JSON that cannot be parsed
   - Message structure doesn't match expected schema
   - Unrecognized method name (if method-specific translation fails)

2. **MissingRequiredField**:
   - Required field is missing in source message
   - Cannot generate required field for target version
   - Example: Cannot generate `name` from an invalid URI

3. **JsonError**:
   - JSON parsing fails
   - JSON serialization fails
   - Invalid JSON structure

### When NOT to Return Errors

Do NOT return errors for:

1. **Missing Optional Fields**: Optional fields can be absent
2. **Unknown Fields**: Preserve unknown fields for forward compatibility
3. **Extra Fields**: Ignore extra fields that aren't in the spec
4. **Empty Values**: Empty strings, null values in optional fields are valid

### Error Message Quality

All errors MUST include:
- Source and target versions
- Message method/type being translated
- Specific field or issue causing the error
- Enough context for debugging

Example:
```rust
Err(ProtocolError::MissingRequiredField {
    field_name: "name".to_string(),
    message_type: "ResourceContents".to_string(),
    version: ProtocolVersion::V20250618,
})
```

---

## Performance Contract

### Latency Requirements

- `source_version()`: O(1), < 10ns
- `target_version()`: O(1), < 10ns
- `translate_request()`: O(n) where n = message size, target < 1ms P99 for typical messages
- `translate_response()`: O(n) where n = message size, target < 1ms P99 for typical messages
- `translate_notification()`: O(n) where n = message size, target < 1ms P99 for typical messages

### Memory Requirements

- Adapter instance: < 200 bytes
- Translation overhead: Should not allocate more than 2x message size
- Pass-through adapter: Should use zero-copy (Cow or reference) where possible

---

## Concurrency Contract

### Thread Safety

- All methods can be called concurrently from multiple threads
- No internal mutable state (or use proper synchronization)
- No blocking operations in async methods

### Async Behavior

- Methods are `async` to allow for future enhancements (e.g., external lookups)
- Current implementations may be synchronous (return immediately)
- Must not block the tokio runtime

---

## Testing Requirements

All `ProtocolAdapter` implementations MUST have:

1. **Unit Tests**:
   - Test each method in isolation
   - Test with valid inputs (all message types)
   - Test with invalid inputs (malformed JSON, missing fields)
   - Test error cases

2. **Round-Trip Tests**:
   - For bidirectional version pairs (e.g., A→B and B→A), test that translating forward then backward preserves semantics
   - Note: Exact equality may not hold (due to field stripping), but semantic meaning must be preserved

3. **Property Tests** (recommended):
   - Valid input always produces valid output
   - Required fields are never lost
   - JSON structure remains valid

4. **Benchmark Tests** (required for performance-critical adapters):
   - Measure latency for typical message sizes
   - Verify < 1ms P99 latency requirement

---

## Factory Function

```rust
/// Create the appropriate adapter for the given version pair
///
/// # Arguments
/// * `source_version` - The version of incoming messages
/// * `target_version` - The version of outgoing messages
///
/// # Returns
/// A boxed adapter implementing the ProtocolAdapter trait
///
/// # Examples
/// ```
/// let adapter = create_adapter(
///     ProtocolVersion::V20241105,
///     ProtocolVersion::V20250618
/// );
/// ```
pub fn create_adapter(
    source_version: ProtocolVersion,
    target_version: ProtocolVersion,
) -> Box<dyn ProtocolAdapter> {
    use ProtocolVersion::*;

    if source_version == target_version {
        return Box::new(PassThroughAdapter::new(source_version));
    }

    match (source_version, target_version) {
        (V20241105, V20250326) => Box::new(V20241105ToV20250326Adapter),
        (V20241105, V20250618) => Box::new(V20241105ToV20250618Adapter),
        (V20250326, V20241105) => Box::new(V20250326ToV20241105Adapter),
        (V20250326, V20250618) => Box::new(V20250326ToV20250618Adapter),
        (V20250618, V20241105) => Box::new(V20250618ToV20241105Adapter),
        (V20250618, V20250326) => Box::new(V20250618ToV20250326Adapter),
        // All cases covered above
    }
}
```

---

## Versioning and Evolution

### Adding New Protocol Versions

When a new protocol version is released:

1. Add new variant to `ProtocolVersion` enum
2. Implement adapters for new version ↔ all existing versions
3. Update `create_adapter()` factory function
4. Add tests for new version pairs
5. Update documentation

### Deprecating Old Versions

When deprecating a protocol version:

1. Mark adapters as deprecated
2. Add runtime warnings when deprecated version is detected
3. Update documentation with migration guide
4. Remove adapters after deprecation period (maintain backward compatibility)

---

## Examples

### Using an Adapter

```rust
use crate::protocol::{create_adapter, ProtocolVersion};
use crate::types::JsonRpcRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create adapter for client (v2024-11-05) to server (v2025-06-18)
    let adapter = create_adapter(
        ProtocolVersion::V20241105,
        ProtocolVersion::V20250618,
    );

    // Translate a request
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: "1".into(),
        method: "tools/list".to_string(),
        params: None,
    };

    let translated = adapter.translate_request(request).await?;

    // Send translated request to server...
    // let response = send_to_server(translated).await?;

    // Translate response back
    // let translated_response = adapter.translate_response(response).await?;

    Ok(())
}
```

### Implementing a Custom Adapter

```rust
use async_trait::async_trait;
use crate::protocol::{ProtocolAdapter, ProtocolVersion, ProtocolError};
use crate::types::{JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};

pub struct MyCustomAdapter;

#[async_trait]
impl ProtocolAdapter for MyCustomAdapter {
    fn source_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20241105
    }

    fn target_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250618
    }

    async fn translate_request(&self, request: JsonRpcRequest)
        -> Result<JsonRpcRequest, ProtocolError>
    {
        // Implementation here...
        Ok(request)
    }

    async fn translate_response(&self, response: JsonRpcResponse)
        -> Result<JsonRpcResponse, ProtocolError>
    {
        // Implementation here...
        Ok(response)
    }

    async fn translate_notification(&self, notification: JsonRpcNotification)
        -> Result<JsonRpcNotification, ProtocolError>
    {
        // Implementation here...
        Ok(notification)
    }
}
```
