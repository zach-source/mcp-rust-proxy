# Research: MCP Protocol Version Negotiation

**Feature**: MCP Protocol Version Support and Translation Layer
**Branch**: `003-mcp-protocol-support`
**Date**: 2025-10-12

## Overview

This document consolidates research findings on implementing multi-version protocol support for the MCP Rust Proxy. The proxy must support three versions of the Model Context Protocol specification (2024-11-05, 2025-03-26, and 2025-06-18) and translate messages between versions as needed.

---

## Key Research Findings

### 1. Protocol Version Structure

**Finding**: All three MCP protocol versions use JSON-RPC 2.0 as their foundation with version-specific extensions.

**Evidence**:
- Protocol version is negotiated during the `initialize` handshake
- Version string format: `"YYYY-MM-DD"` (e.g., `"2024-11-05"`)
- All versions maintain the same initialization sequence: initialize request → response → initialized notification
- JSON-RPC 2.0 message structure is consistent across all versions

**Implication**: We can build a single JSON-RPC message handling layer with version-specific adapters on top.

---

### 2. Breaking Changes Analysis

**Finding**: Only ONE breaking change exists across the three versions.

**Breaking Change** (2025-03-26 → 2025-06-18):
- `ResourceContents.name` field is now **REQUIRED** in `resources/read` responses
- Previously this field did not exist at all
- Impact: Any translation from older versions to 2025-06-18 must synthesize this field

**All other changes are additive**:
- 2025-03-26 adds: `completions` capability, `AudioContent` type, `resources/updated` notification
- 2025-06-18 adds: `elicitation` capability, `title` fields, `outputSchema`, `structuredContent`

**Implication**: Forward compatibility is straightforward (add fields), backward compatibility requires field stripping and the special handling of `ResourceContents.name`.

---

### 3. Version Detection Strategy

**Decision**: Detect protocol version from the `initialize` response's `protocolVersion` field.

**Rationale**:
1. The server's `initialize` response explicitly declares the negotiated protocol version
2. This is the authoritative source of truth for which protocol version to use
3. Allows servers to downgrade gracefully if they don't support the client's requested version
4. Follows the MCP specification's version negotiation pattern

**Alternatives Considered**:
- **Alternative 1**: Probe servers with test requests
  - **Rejected**: Adds latency and complexity; unnecessary since version is explicitly declared
- **Alternative 2**: Configure version per-server in config file
  - **Rejected**: Fragile; servers can update their protocol version independently
- **Alternative 3**: Detect from message structure differences
  - **Rejected**: Unreliable; requires parsing every message and inferring from presence/absence of fields

**Implementation**:
```rust
// Extract version from initialize response
let protocol_version = ProtocolVersion::from_string(
    &init_response.result.protocol_version
)?;

// Store per-server state
server_state.set_protocol_version(protocol_version);
```

---

### 4. Translation Layer Architecture

**Decision**: Implement a bi-directional translation layer using the Adapter pattern.

**Rationale**:
1. **Separation of Concerns**: Translation logic is isolated from transport and routing logic
2. **Testability**: Each adapter can be tested independently with known inputs/outputs
3. **Extensibility**: New protocol versions can be added by implementing new adapters
4. **Performance**: Pass-through mode when versions match (zero overhead for common case)

**Architecture**:
```
Client Request (any version)
    ↓
Proxy (native version: 2025-06-18)
    ↓
Protocol Adapter (detects server version)
    ↓
[Translation if needed]
    ↓
Backend Server (version-specific format)
    ↓
[Translation if needed]
    ↓
Protocol Adapter
    ↓
Proxy
    ↓
Client Response (original version)
```

**Alternatives Considered**:
- **Alternative 1**: Inline translation in handlers
  - **Rejected**: Mixes concerns, difficult to test, hard to maintain
- **Alternative 2**: Separate proxy instances per version
  - **Rejected**: Requires multiple listening ports, doesn't support mixed-version backend servers
- **Alternative 3**: Runtime code generation
  - **Rejected**: Over-engineered; compile-time type safety is valuable

---

### 5. Message Translation Rules

**Decision**: Define explicit translation rules for each version pair and message type.

**Forward Translation Rules** (Older → Newer):

| From Version | To Version | Message Type | Rule |
|--------------|------------|--------------|------|
| 2024-11-05 | 2025-06-18 | resources/read | Generate `name` from `uri` (filename or full URI) |
| 2024-11-05 | 2025-06-18 | Any | Preserve all fields; add empty optional fields if needed |
| 2025-03-26 | 2025-06-18 | resources/read | Generate `name` from `uri` |
| 2025-03-26 | 2025-06-18 | Any | Preserve all fields |

**Backward Translation Rules** (Newer → Older):

| From Version | To Version | Message Type | Rule |
|--------------|------------|--------------|------|
| 2025-06-18 | 2024-11-05 | tools/list | Strip `title`, `outputSchema` fields |
| 2025-06-18 | 2024-11-05 | tools/call | Strip `structuredContent` field |
| 2025-06-18 | 2024-11-05 | resources/list | Strip `title` field |
| 2025-06-18 | 2024-11-05 | resources/read | Strip `name`, `title` fields |
| 2025-06-18 | 2024-11-05 | prompts/list | Strip `title` field |
| 2025-06-18 | 2024-11-05 | Content array | Convert `AudioContent` to `TextContent` with description |
| 2025-06-18 | 2024-11-05 | initialize | Strip `elicitation` from client capabilities, `completions` from server capabilities |
| 2025-06-18 | 2025-03-26 | Same as above except keep AudioContent and completions capability |

**Rationale**:
- Explicit rules make behavior predictable and testable
- Field stripping prevents confusing older servers with unknown fields
- Field synthesis (name from URI) maintains required field constraints
- Content type conversion maintains semantic meaning where possible

**Implementation Note**: Use Rust's type system to enforce these rules at compile time where possible.

---

### 6. Initialization Sequence Handling

**Decision**: Enforce strict initialization sequencing with state machine tracking.

**Rationale**:
1. **Root Cause**: Current server crashes are caused by sending requests before initialization completes
2. **Specification Requirement**: MCP spec explicitly requires: initialize → response → initialized → normal operations
3. **State Safety**: State machine prevents race conditions and ensures correct ordering

**State Machine**:
```
[Connecting]
    ↓ send initialize request
[Initializing]
    ↓ receive initialize response
[SendingInitialized]
    ↓ send initialized notification
[Ready]
    ↓ can now handle normal requests
```

**Implementation**:
```rust
enum ServerConnectionState {
    Connecting,
    Initializing,
    SendingInitialized,
    Ready,
    Failed(String),
}

// Only allow tools/list, resources/list, etc. when state == Ready
fn can_send_request(&self, method: &str) -> bool {
    match (method, &self.state) {
        ("initialize", ServerConnectionState::Connecting) => true,
        (_, ServerConnectionState::Ready) => true,
        _ => false,
    }
}
```

**Alternatives Considered**:
- **Alternative 1**: Queue requests during initialization
  - **Rejected**: Adds complexity; initialization should be fast (< 60s per spec requirement)
- **Alternative 2**: Return errors for premature requests
  - **Partially Adopted**: Combined with queueing - return error if initialization fails, queue if in progress
- **Alternative 3**: No enforcement (rely on caller)
  - **Rejected**: Current crashes prove this is insufficient

---

### 7. Performance Considerations

**Decision**: Implement pass-through mode for matching versions with zero-copy where possible.

**Rationale**:
- Common case: Proxy and backend server use the same protocol version
- Zero translation overhead for the common case improves latency and throughput
- Translation overhead only applied when actually needed

**Implementation Strategy**:
```rust
fn translate_request(
    &self,
    request: &JsonRpcRequest,
    from_version: ProtocolVersion,
    to_version: ProtocolVersion,
) -> Result<JsonRpcRequest> {
    // Fast path: no translation needed
    if from_version == to_version {
        return Ok(request.clone()); // or use Cow for zero-copy
    }

    // Slow path: version-specific translation
    match (from_version, to_version) {
        (V20241105, V20250618) => self.translate_forward_full(request),
        (V20250618, V20241105) => self.translate_backward_full(request),
        // ... other combinations
    }
}
```

**Measurement Goal**: < 1ms translation overhead per message (P99 latency).

---

### 8. Field Generation Strategy for ResourceContents.name

**Decision**: Generate `name` field from URI using this priority:
1. Last path component if URI is path-like (e.g., `file:///path/to/doc.txt` → `"doc.txt"`)
2. Full URI if not path-like (e.g., `custom://resource-id` → `"custom://resource-id"`)

**Rationale**:
- Preserves human-readable names when available
- Ensures uniqueness (URI is already unique)
- Fails gracefully for non-standard URI schemes

**Implementation**:
```rust
fn generate_resource_name(uri: &str) -> String {
    if let Some(path) = uri.strip_prefix("file://") {
        // Extract filename from path
        path.split('/').last().unwrap_or(uri).to_string()
    } else if let Ok(parsed) = url::Url::parse(uri) {
        // Try to get last segment
        parsed.path_segments()
            .and_then(|s| s.last())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uri.to_string())
    } else {
        // Fallback: use full URI
        uri.to_string()
    }
}
```

**Test Cases**:
- `file:///home/user/document.txt` → `"document.txt"`
- `http://example.com/api/resource` → `"resource"`
- `custom://unique-id-12345` → `"custom://unique-id-12345"`
- `file:///` → `"file:///"`

---

### 9. AudioContent Conversion Strategy

**Decision**: Convert `AudioContent` to `TextContent` with descriptive placeholder when downgrading to 2024-11-05.

**Format**: `"[Audio content: {mimeType}]"`

**Rationale**:
- Preserves awareness that audio was present
- Provides MIME type information for debugging
- Simple and predictable transformation
- Avoids data loss (audio data itself cannot be represented in text)

**Alternatives Considered**:
- **Alternative 1**: Drop audio content entirely
  - **Rejected**: Silent data loss is dangerous
- **Alternative 2**: Include base64-encoded audio data in text
  - **Rejected**: Defeats the purpose of downgrading; older clients won't decode it
- **Alternative 3**: Return error when audio content present
  - **Rejected**: Too strict; graceful degradation is better

---

### 10. Capability Filtering

**Decision**: Filter capabilities in `initialize` responses based on negotiated protocol version.

**Rules**:
- When downgrading to 2024-11-05: Remove `completions` from ServerCapabilities, `elicitation` from ClientCapabilities
- When downgrading to 2025-03-26: Remove `elicitation` from ClientCapabilities
- When using 2025-06-18: No filtering

**Rationale**:
- Prevents clients from attempting to use features that don't exist in negotiated protocol
- Maintains consistency between declared capabilities and actual behavior
- Follows principle of least surprise

**Implementation**: Apply filtering in `translate_initialize_response()` function.

---

### 11. Error Handling Strategy

**Decision**: Use structured error types with version context.

**Error Categories**:
1. **UnsupportedVersion**: Server reports a protocol version we don't support
2. **TranslationError**: Failed to translate a message between versions (e.g., malformed JSON)
3. **MissingRequiredField**: Required field missing after translation (indicates bug)
4. **InitializationTimeout**: Server didn't complete initialization within timeout (default 60s)

**Error Response Format**:
```rust
pub enum ProtocolError {
    UnsupportedVersion {
        reported_version: String,
        supported_versions: Vec<String>,
    },
    TranslationError {
        from_version: ProtocolVersion,
        to_version: ProtocolVersion,
        message_type: String,
        details: String,
    },
    MissingRequiredField {
        field_name: String,
        message_type: String,
        version: ProtocolVersion,
    },
    InitializationTimeout {
        server_name: String,
        duration: Duration,
    },
}
```

**Rationale**:
- Structured errors enable better debugging
- Version context helps diagnose version-specific issues
- Clients receive clear error messages

---

### 12. Testing Strategy

**Decision**: Multi-layered testing approach covering unit, integration, and protocol compliance.

**Test Layers**:

1. **Unit Tests** (per adapter):
   - Test each translation rule in isolation
   - Verify field stripping/addition
   - Test pass-through mode
   - Test error cases (malformed input)

2. **Integration Tests** (end-to-end):
   - Mock backend servers returning each protocol version
   - Send requests through proxy
   - Verify correct translation occurs
   - Verify initialization sequence

3. **Protocol Compliance Tests**:
   - Verify translated messages validate against JSON schemas
   - Test compatibility matrix (all version pairs)
   - Test edge cases from spec

**Coverage Goal**: 90%+ line coverage on translation logic.

---

## Implementation Decisions Summary

| Decision Area | Choice | Confidence |
|--------------|--------|------------|
| Version Detection | From initialize response | High |
| Architecture | Adapter pattern with bi-directional translation | High |
| Translation Strategy | Explicit rules per version pair | High |
| Pass-through Optimization | Zero-copy when versions match | High |
| ResourceContents.name | Generate from URI (filename or full URI) | Medium |
| AudioContent Conversion | Text placeholder with MIME type | High |
| Initialization Sequencing | State machine enforcement | High |
| Error Handling | Structured errors with version context | High |
| Testing | Multi-layered (unit/integration/compliance) | High |

---

## Open Questions

1. **Q**: Should we support automatic retry with version downgrade if initialization fails?
   - **Current stance**: No - fail fast and let operators diagnose version issues
   - **Revisit if**: We see frequent version negotiation failures in practice

2. **Q**: Should we cache translated messages to reduce overhead?
   - **Current stance**: No - premature optimization; measure first
   - **Revisit if**: Translation overhead exceeds 1ms P99

3. **Q**: Should we warn when stripping fields that contain data?
   - **Current stance**: Yes - log at WARN level when stripping non-empty fields
   - **Rationale**: Helps diagnose unexpected data loss

4. **Q**: Should we support unknown protocol versions with best-effort translation?
   - **Current stance**: No - fail explicitly for unsupported versions
   - **Rationale**: Correctness over flexibility; unknown versions may have incompatible changes

---

## References

- MCP Specification: https://modelcontextprotocol.io/specification/
- JSON-RPC 2.0: https://www.jsonrpc.org/specification
- Internal Research Documents:
  - `/Users/ztaylor/repos/workspaces/mcp-rust-proxy/main/MCP_VERSION_COMPARISON.md`
  - `/Users/ztaylor/repos/workspaces/mcp-rust-proxy/main/MCP_VERSION_DIFFERENCES_QUICK_REF.md`
