# Contract: Version Detection and Negotiation

**Feature**: MCP Protocol Version Support
**Contract Type**: Protocol Behavior Specification
**Date**: 2025-10-12

## Overview

This document specifies how the MCP Rust Proxy detects protocol versions and negotiates version compatibility with backend MCP servers. It defines the initialization sequence, version negotiation rules, and fallback behaviors.

---

## Version Negotiation Flow

### Standard Initialization Sequence

```
┌────────┐                    ┌───────┐                    ┌────────────┐
│ Client │                    │ Proxy │                    │ Backend    │
│        │                    │       │                    │ Server     │
└────┬───┘                    └───┬───┘                    └─────┬──────┘
     │                            │                              │
     │  1. Client connects        │                              │
     │ ─────────────────────────> │                              │
     │                            │                              │
     │                            │  2. Initialize Request       │
     │                            │  (proxy's protocol version)  │
     │                            │ ──────────────────────────> │
     │                            │                              │
     │                            │  3. Initialize Response      │
     │                            │  (negotiated version)        │
     │                            │ <────────────────────────── │
     │                            │                              │
     │                            │  4. Parse protocol version   │
     │                            │     from response            │
     │                            │                              │
     │                            │  5. Create version adapter   │
     │                            │                              │
     │                            │  6. Initialized Notification │
     │                            │ ──────────────────────────> │
     │                            │                              │
     │                            │  7. Mark server as Ready     │
     │                            │                              │
     │  8. Tools/Resources List   │                              │
     │ ─────────────────────────> │                              │
     │                            │  9. Translated Request       │
     │                            │ ──────────────────────────> │
     │                            │                              │
     │                            │  10. Response                │
     │                            │ <────────────────────────── │
     │  11. Translated Response   │                              │
     │ <───────────────────────── │                              │
     │                            │                              │
```

### Detailed Steps

#### Step 1: Proxy Sends Initialize Request

**Timing**: Immediately after transport connection is established

**Request Format**:
```json
{
  "jsonrpc": "2.0",
  "id": "init-{server-name}-{timestamp}",
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "roots": {
        "listChanged": true
      },
      "sampling": {}
    },
    "clientInfo": {
      "name": "mcp-rust-proxy",
      "version": "0.1.0"
    }
  }
}
```

**Protocol Version Selection**:
- Proxy MUST send the **latest** protocol version it supports
- Currently: `"2025-06-18"`
- Rationale: Allows servers to negotiate down to older versions if needed

**Timeout**: 60 seconds (configurable)

---

#### Step 2: Server Returns Initialize Response

**Expected Response Format**:
```json
{
  "jsonrpc": "2.0",
  "id": "init-{server-name}-{timestamp}",
  "result": {
    "protocolVersion": "2025-03-26",
    "capabilities": {
      "logging": {},
      "prompts": {
        "listChanged": true
      },
      "resources": {
        "subscribe": true,
        "listChanged": true
      },
      "tools": {
        "listChanged": true
      },
      "completions": {}
    },
    "serverInfo": {
      "name": "example-mcp-server",
      "version": "1.2.0"
    }
  }
}
```

**Validation Rules**:
1. Response MUST have `result` object (not `error`)
2. `result.protocolVersion` MUST be present (string)
3. `result.capabilities` MUST be present (object, may be empty)
4. `result.serverInfo` MUST be present with `name` and `version`

**Error Cases**:
```json
{
  "jsonrpc": "2.0",
  "id": "init-{server-name}-{timestamp}",
  "error": {
    "code": -32602,
    "message": "Unsupported protocol version",
    "data": {
      "requested": "2025-06-18",
      "supported": ["2024-11-05", "2025-03-26"]
    }
  }
}
```

---

#### Step 3: Version Detection

**Parsing Logic**:
```rust
async fn detect_protocol_version(
    init_response: &JsonRpcResponse,
) -> Result<ProtocolVersion, ProtocolError> {
    // Extract result object
    let result = init_response.result
        .as_ref()
        .ok_or(ProtocolError::InitializationFailed {
            reason: "No result in initialize response".to_string(),
        })?;

    // Extract protocolVersion string
    let version_str = result
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .ok_or(ProtocolError::MissingRequiredField {
            field_name: "protocolVersion".to_string(),
            message_type: "InitializeResult".to_string(),
            version: ProtocolVersion::V20250618, // Expected version
        })?;

    // Parse to enum
    ProtocolVersion::from_string(version_str)
}
```

**Supported Versions**:
- `"2024-11-05"` → `ProtocolVersion::V20241105`
- `"2025-03-26"` → `ProtocolVersion::V20250326`
- `"2025-06-18"` → `ProtocolVersion::V20250618`

**Unsupported Versions**:
- Any other string → `ProtocolError::UnsupportedVersion`
- Missing field → `ProtocolError::MissingRequiredField`
- Wrong type (not string) → `ProtocolError::InvalidFieldType`

---

#### Step 4: Adapter Selection

**Selection Logic**:
```rust
fn select_adapter(
    proxy_version: ProtocolVersion,
    server_version: ProtocolVersion,
) -> Box<dyn ProtocolAdapter> {
    // Fast path: same version (no translation)
    if proxy_version == server_version {
        return Box::new(PassThroughAdapter::new(proxy_version));
    }

    // Create bidirectional adapter
    match (proxy_version, server_version) {
        (V20250618, V20241105) => Box::new(V20250618ToV20241105Adapter),
        (V20250618, V20250326) => Box::new(V20250618ToV20250326Adapter),
        (V20241105, V20250618) => Box::new(V20241105ToV20250618Adapter),
        (V20250326, V20250618) => Box::new(V20250326ToV20250618Adapter),
        // ... other combinations
    }
}
```

**Adapter Lifetime**:
- Created once during initialization
- Stored in `ServerConnectionState`
- Reused for all subsequent messages
- Recreated if connection is re-established

---

#### Step 5: Send Initialized Notification

**Timing**: Immediately after version detection and adapter creation

**Notification Format**:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

**Requirements**:
- MUST NOT have `id` field (notifications never have IDs)
- MUST NOT have `params` field (this notification has no parameters)
- MUST be sent before any other requests (tools/list, resources/list, etc.)

**Translation**:
- This notification is version-agnostic (same format in all versions)
- No translation needed

---

#### Step 6: Mark Server as Ready

**State Transition**:
```
SendingInitialized → Ready
```

**Recorded Data**:
- Protocol version (from initialize response)
- Server capabilities (from initialize response)
- Initialized timestamp
- Protocol adapter reference

**Ready State**:
- Server can now receive normal requests (tools/list, resources/list, etc.)
- All subsequent requests/responses will use the selected adapter

---

## Version Negotiation Rules

### Rule 1: Client Requests Latest

**Behavior**: Proxy always requests its latest supported version

**Rationale**:
- Maximizes feature availability
- Allows servers to negotiate down if needed
- Future-proof (when proxy is updated to support newer versions)

**Example**:
```
Proxy supports: [2024-11-05, 2025-03-26, 2025-06-18]
Proxy requests: "2025-06-18"
```

---

### Rule 2: Server Chooses Version

**Behavior**: Backend server returns the version it will use

**Possibilities**:
1. **Server supports requested version**: Returns same version
   ```
   Proxy requests: "2025-06-18"
   Server returns: "2025-06-18"
   Result: Perfect match, pass-through adapter
   ```

2. **Server supports older version**: Returns older version
   ```
   Proxy requests: "2025-06-18"
   Server returns: "2025-03-26"
   Result: Proxy adapts to server version
   ```

3. **Server supports even older version**: Returns much older version
   ```
   Proxy requests: "2025-06-18"
   Server returns: "2024-11-05"
   Result: Proxy adapts to oldest supported version
   ```

4. **Server supports unsupported version**: Returns unknown version
   ```
   Proxy requests: "2025-06-18"
   Server returns: "2026-01-01"
   Result: ERROR - proxy cannot handle this version
   ```

---

### Rule 3: Proxy Validates Returned Version

**Validation**:
```rust
let server_version = ProtocolVersion::from_string(response_version)?;

// This will return Err if version is unsupported
if let Err(e) = server_version {
    // Log error
    tracing::error!(
        server = %server_name,
        reported_version = %response_version,
        "Backend server reported unsupported protocol version"
    );

    // Mark connection as failed
    state.mark_failed(format!("Unsupported version: {}", response_version)).await;

    // Return error
    return Err(e);
}
```

**Acceptance Criteria**:
- Version string MUST be one of: `"2024-11-05"`, `"2025-03-26"`, `"2025-06-18"`
- Any other value causes connection failure

---

### Rule 4: No Retry or Re-negotiation

**Behavior**: Version negotiation happens exactly once per connection

**Rationale**:
- Simplifies state management
- Matches MCP specification (no re-negotiation defined)
- Failures should be debugged, not automatically retried

**If Negotiation Fails**:
1. Mark server as `Failed` state
2. Log error with version details
3. Do not attempt to reconnect automatically
4. Operator must diagnose and fix (update server or proxy)

---

## Compatibility Matrix

### Supported Version Pairs

| Proxy Version | Server Version | Compatible? | Adapter Type | Notes |
|---------------|----------------|-------------|--------------|-------|
| 2024-11-05 | 2024-11-05 | ✅ Yes | PassThrough | Perfect match |
| 2024-11-05 | 2025-03-26 | ✅ Yes | Forward | Add audio support |
| 2024-11-05 | 2025-06-18 | ✅ Yes | Forward | Add audio + structured content |
| 2025-03-26 | 2024-11-05 | ✅ Yes | Backward | Strip audio, completions |
| 2025-03-26 | 2025-03-26 | ✅ Yes | PassThrough | Perfect match |
| 2025-03-26 | 2025-06-18 | ✅ Yes | Forward | Add structured content |
| 2025-06-18 | 2024-11-05 | ✅ Yes | Backward | Strip title, audio, structured |
| 2025-06-18 | 2025-03-26 | ✅ Yes | Backward | Strip title, structured |
| 2025-06-18 | 2025-06-18 | ✅ Yes | PassThrough | Perfect match |

### Unsupported Version Pairs

| Proxy Version | Server Version | Error |
|---------------|----------------|-------|
| Any | 2026-01-01 | UnsupportedVersion |
| Any | 2024-06-01 | UnsupportedVersion |
| Any | "unknown" | UnsupportedVersion |
| Any | (missing) | MissingRequiredField |

---

## Error Handling

### Initialization Timeout

**Condition**: Server does not respond to initialize request within timeout period (default 60s)

**Behavior**:
```rust
if handshake_tracker.is_timed_out() {
    state.mark_failed(format!(
        "Initialization timeout after {:?}",
        handshake_tracker.timeout
    )).await;

    return Err(ProtocolError::InitializationTimeout {
        server_name: server_name.clone(),
        duration: handshake_tracker.timeout,
    });
}
```

**Recovery**: Operator must investigate server logs and restart if needed

---

### Unsupported Version

**Condition**: Server reports a protocol version the proxy doesn't support

**Behavior**:
```rust
Err(ProtocolError::UnsupportedVersion {
    reported_version: "2026-01-01".to_string(),
    supported_versions: vec![
        "2024-11-05".to_string(),
        "2025-03-26".to_string(),
        "2025-06-18".to_string(),
    ],
})
```

**Recovery**:
- Update proxy to support new version, OR
- Update server to use supported version

---

### Malformed Initialize Response

**Condition**: Server returns response that doesn't match expected structure

**Behavior**:
```rust
// Missing protocolVersion field
Err(ProtocolError::MissingRequiredField {
    field_name: "protocolVersion".to_string(),
    message_type: "InitializeResult".to_string(),
    version: ProtocolVersion::V20250618,
})

// Wrong type (not string)
Err(ProtocolError::InvalidFieldType {
    field_name: "protocolVersion".to_string(),
    expected_type: "string".to_string(),
    actual_type: "number".to_string(),
})
```

**Recovery**: Fix backend server to return valid response

---

### Server Returns Error

**Condition**: Server returns JSON-RPC error response to initialize request

**Behavior**:
```rust
if let Some(error) = init_response.error {
    state.mark_failed(format!(
        "Initialize request failed: {} (code: {})",
        error.message,
        error.code
    )).await;

    return Err(ProtocolError::InitializationFailed {
        reason: error.message,
    });
}
```

**Recovery**: Check server logs for details, fix configuration or server issue

---

## State Machine

### State Definitions

```rust
pub enum ConnectionState {
    /// Initial state after transport connection established
    Connecting,

    /// Initialize request sent, waiting for response
    Initializing {
        request_id: String,
        started_at: Instant,
    },

    /// Initialize response received, sending initialized notification
    SendingInitialized {
        protocol_version: ProtocolVersion,
    },

    /// Fully initialized, ready for normal requests
    Ready {
        protocol_version: ProtocolVersion,
        initialized_at: Instant,
    },

    /// Initialization failed
    Failed {
        error: String,
        failed_at: Instant,
    },

    /// Connection closing
    Closing,
}
```

### Transitions

```
Connecting
    |
    | send initialize request
    v
Initializing
    |
    | receive initialize response
    | + detect version
    | + create adapter
    v
SendingInitialized
    |
    | send initialized notification
    v
Ready
    |
    | (stable state)
    v
    (continue handling requests)

Any State
    |
    | error or timeout
    v
Failed

Any State
    |
    | shutdown signal
    v
Closing
```

### State Predicates

```rust
impl ServerConnectionState {
    /// Can this server receive a specific request?
    pub async fn can_send_request(&self, method: &str) -> bool {
        let state = self.get_state().await;
        match (method, state) {
            ("initialize", ConnectionState::Connecting) => true,
            (_, ConnectionState::Ready { .. }) => true,
            _ => false,
        }
    }

    /// Is this server ready for normal operations?
    pub async fn is_ready(&self) -> bool {
        matches!(
            self.get_state().await,
            ConnectionState::Ready { .. }
        )
    }

    /// Get protocol version (if known)
    pub async fn protocol_version(&self) -> Option<ProtocolVersion> {
        match self.get_state().await {
            ConnectionState::SendingInitialized { protocol_version } |
            ConnectionState::Ready { protocol_version, .. } => {
                Some(protocol_version)
            }
            _ => None,
        }
    }
}
```

---

## Logging and Observability

### Required Log Events

**Initialization Started**:
```rust
tracing::info!(
    server = %server_name,
    protocol_version = %proxy_version.as_str(),
    "Sending initialize request to backend server"
);
```

**Version Detected**:
```rust
tracing::info!(
    server = %server_name,
    protocol_version = %detected_version.as_str(),
    duration_ms = %elapsed.as_millis(),
    "Detected backend server protocol version"
);
```

**Adapter Created**:
```rust
tracing::debug!(
    server = %server_name,
    from_version = %proxy_version.as_str(),
    to_version = %server_version.as_str(),
    adapter_type = %adapter_type_name,
    "Created protocol adapter for version translation"
);
```

**Initialization Complete**:
```rust
tracing::info!(
    server = %server_name,
    protocol_version = %version.as_str(),
    total_duration_ms = %total_elapsed.as_millis(),
    "Backend server initialization complete"
);
```

**Unsupported Version**:
```rust
tracing::error!(
    server = %server_name,
    reported_version = %reported_version,
    supported_versions = ?SUPPORTED_VERSIONS,
    "Backend server reported unsupported protocol version"
);
```

### Metrics

**Recommended Metrics**:
- `mcp_proxy_initialization_duration_seconds` (histogram)
- `mcp_proxy_initialization_failures_total` (counter)
- `mcp_proxy_protocol_version` (gauge with labels: server, version)
- `mcp_proxy_adapter_type` (gauge with labels: server, adapter_type)

---

## Testing Strategy

### Unit Tests

1. **Version Parsing**:
   - Valid version strings → correct enum
   - Invalid version strings → error
   - Missing field → error
   - Wrong type → error

2. **Adapter Selection**:
   - Same version → PassThroughAdapter
   - Different versions → correct bidirectional adapter
   - All version pair combinations

3. **State Transitions**:
   - Valid transitions succeed
   - Invalid transitions return error
   - Concurrent access is safe

### Integration Tests

1. **Mock Backend Servers**:
   - Server returns 2024-11-05 → proxy adapts correctly
   - Server returns 2025-03-26 → proxy adapts correctly
   - Server returns 2025-06-18 → pass-through mode
   - Server returns unsupported version → connection fails

2. **Timeout Handling**:
   - Server never responds → timeout error after 60s
   - Slow server (30s) → succeeds before timeout

3. **Error Cases**:
   - Server returns error response → initialization fails
   - Malformed response → initialization fails
   - Network error → initialization fails

### End-to-End Tests

1. Connect to real MCP servers with different versions
2. Verify tools/resources are accessible
3. Verify translation is correct
4. Verify no server crashes

---

## Future Enhancements

### Possible Future Features

1. **Version Preference Configuration**:
   - Allow operators to specify preferred version per server
   - Proxy could request that version instead of latest

2. **Automatic Retry with Downgrade**:
   - If server rejects requested version, automatically retry with older version
   - Requires specification of retry logic

3. **Version Capability Discovery**:
   - Probe server for supported versions before initialize
   - Not part of current MCP spec

4. **Dynamic Adapter Hot-Reload**:
   - Load new adapters without restarting proxy
   - Support for new protocol versions without recompilation

**Note**: These are out of scope for current implementation.
