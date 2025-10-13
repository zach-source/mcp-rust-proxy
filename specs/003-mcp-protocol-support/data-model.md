# Data Model: MCP Protocol Version Support

**Feature**: MCP Protocol Version Negotiation and Translation Layer
**Branch**: `003-mcp-protocol-support`
**Date**: 2025-10-12

## Overview

This document defines the key entities, state machines, and data structures for implementing multi-version MCP protocol support in the Rust proxy.

---

## Core Entities

### 1. ProtocolVersion

**Purpose**: Represents a specific version of the MCP protocol specification.

**Definition**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolVersion {
    /// MCP Protocol version 2024-11-05 (initial stable release)
    V20241105,

    /// MCP Protocol version 2025-03-26 (adds completions, audio)
    V20250326,

    /// MCP Protocol version 2025-06-18 (adds structured output, titles)
    V20250618,
}

impl ProtocolVersion {
    /// Parse version string from initialize message
    pub fn from_string(s: &str) -> Result<Self, ProtocolError> {
        match s {
            "2024-11-05" => Ok(Self::V20241105),
            "2025-03-26" => Ok(Self::V20250326),
            "2025-06-18" => Ok(Self::V20250618),
            _ => Err(ProtocolError::UnsupportedVersion {
                reported_version: s.to_string(),
                supported_versions: vec![
                    "2024-11-05".to_string(),
                    "2025-03-26".to_string(),
                    "2025-06-18".to_string(),
                ],
            }),
        }
    }

    /// Get version string for initialize messages
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V20241105 => "2024-11-05",
            Self::V20250326 => "2025-03-26",
            Self::V20250618 => "2025-06-18",
        }
    }

    /// Check if this version supports audio content
    pub fn supports_audio_content(&self) -> bool {
        matches!(self, Self::V20250326 | Self::V20250618)
    }

    /// Check if this version supports completions capability
    pub fn supports_completions(&self) -> bool {
        matches!(self, Self::V20250326 | Self::V20250618)
    }

    /// Check if this version requires ResourceContents.name field
    pub fn requires_resource_name(&self) -> bool {
        matches!(self, Self::V20250618)
    }

    /// Check if this version supports structured content output
    pub fn supports_structured_content(&self) -> bool {
        matches!(self, Self::V20250618)
    }

    /// Check if this version supports elicitation capability
    pub fn supports_elicitation(&self) -> bool {
        matches!(self, Self::V20250618)
    }

    /// Check if this version supports title fields
    pub fn supports_title_fields(&self) -> bool {
        matches!(self, Self::V20250618)
    }

    /// Check if this version supports output schema in tools
    pub fn supports_output_schema(&self) -> bool {
        matches!(self, Self::V20250618)
    }
}
```

**Invariants**:
- Protocol version is immutable once detected
- Version string format is always `YYYY-MM-DD`
- Unsupported versions cause explicit errors (no silent fallback)

---

### 2. ProtocolAdapter

**Purpose**: Translates messages between different protocol versions.

**Definition**:
```rust
/// Trait for protocol version adapters
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// Get the source protocol version this adapter handles
    fn source_version(&self) -> ProtocolVersion;

    /// Get the target protocol version this adapter produces
    fn target_version(&self) -> ProtocolVersion;

    /// Translate a request from source to target version
    async fn translate_request(
        &self,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcRequest, ProtocolError>;

    /// Translate a response from target back to source version
    async fn translate_response(
        &self,
        response: JsonRpcResponse,
    ) -> Result<JsonRpcResponse, ProtocolError>;

    /// Translate a notification from target back to source version
    async fn translate_notification(
        &self,
        notification: JsonRpcNotification,
    ) -> Result<JsonRpcNotification, ProtocolError>;
}

/// Pass-through adapter (no translation needed)
pub struct PassThroughAdapter {
    version: ProtocolVersion,
}

impl PassThroughAdapter {
    pub fn new(version: ProtocolVersion) -> Self {
        Self { version }
    }
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
        // Zero-copy pass-through
        Ok(request)
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

**Concrete Adapters**:
```rust
/// Adapter for translating 2024-11-05 → 2025-06-18
pub struct V20241105ToV20250618Adapter;

/// Adapter for translating 2025-06-18 → 2024-11-05
pub struct V20250618ToV20241105Adapter;

/// Adapter for translating 2025-03-26 → 2025-06-18
pub struct V20250326ToV20250618Adapter;

/// Adapter for translating 2025-06-18 → 2025-03-26
pub struct V20250618ToV20250326Adapter;

// Additional adapters for other version pairs as needed
```

**Usage Pattern**:
```rust
// Select adapter based on version pair
let adapter: Box<dyn ProtocolAdapter> = match (client_version, server_version) {
    (v1, v2) if v1 == v2 => Box::new(PassThroughAdapter::new(v1)),
    (V20241105, V20250618) => Box::new(V20241105ToV20250618Adapter),
    (V20250618, V20241105) => Box::new(V20250618ToV20241105Adapter),
    // ... other combinations
};

// Use adapter
let translated_request = adapter.translate_request(original_request).await?;
```

---

### 3. ServerConnectionState

**Purpose**: Tracks the initialization state and protocol version for each backend server connection.

**Definition**:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection being established
    Connecting,

    /// Sent initialize request, waiting for response
    Initializing {
        request_id: String,
        started_at: Instant,
    },

    /// Received initialize response, sending initialized notification
    SendingInitialized {
        protocol_version: ProtocolVersion,
    },

    /// Fully initialized and ready for normal requests
    Ready {
        protocol_version: ProtocolVersion,
        initialized_at: Instant,
    },

    /// Connection failed during initialization
    Failed {
        error: String,
        failed_at: Instant,
    },

    /// Connection is being closed
    Closing,
}

pub struct ServerConnectionState {
    /// Current connection state
    state: Arc<Mutex<ConnectionState>>,

    /// Server name for logging
    server_name: String,

    /// Protocol adapter for this connection
    adapter: Arc<RwLock<Option<Box<dyn ProtocolAdapter>>>>,

    /// Last activity timestamp
    last_activity: Arc<Mutex<Instant>>,
}

impl ServerConnectionState {
    pub fn new(server_name: String) -> Self {
        Self {
            state: Arc::new(Mutex::new(ConnectionState::Connecting)),
            server_name,
            adapter: Arc::new(RwLock::new(None)),
            last_activity: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> ConnectionState {
        self.state.lock().await.clone()
    }

    /// Transition to Initializing state
    pub async fn start_initialization(&self, request_id: String)
        -> Result<(), ProtocolError>
    {
        let mut state = self.state.lock().await;
        match *state {
            ConnectionState::Connecting => {
                *state = ConnectionState::Initializing {
                    request_id,
                    started_at: Instant::now(),
                };
                Ok(())
            }
            _ => Err(ProtocolError::InvalidStateTransition {
                from: format!("{:?}", *state),
                to: "Initializing".to_string(),
            }),
        }
    }

    /// Transition to SendingInitialized state
    pub async fn received_initialize_response(
        &self,
        protocol_version: ProtocolVersion
    ) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        match *state {
            ConnectionState::Initializing { .. } => {
                *state = ConnectionState::SendingInitialized { protocol_version };
                Ok(())
            }
            _ => Err(ProtocolError::InvalidStateTransition {
                from: format!("{:?}", *state),
                to: "SendingInitialized".to_string(),
            }),
        }
    }

    /// Transition to Ready state
    pub async fn complete_initialization(&self) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        match *state {
            ConnectionState::SendingInitialized { protocol_version } => {
                *state = ConnectionState::Ready {
                    protocol_version,
                    initialized_at: Instant::now(),
                };
                Ok(())
            }
            _ => Err(ProtocolError::InvalidStateTransition {
                from: format!("{:?}", *state),
                to: "Ready".to_string(),
            }),
        }
    }

    /// Mark as failed
    pub async fn mark_failed(&self, error: String) {
        let mut state = self.state.lock().await;
        *state = ConnectionState::Failed {
            error,
            failed_at: Instant::now(),
        };
    }

    /// Check if server is ready to handle requests
    pub async fn is_ready(&self) -> bool {
        matches!(*self.state.lock().await, ConnectionState::Ready { .. })
    }

    /// Get protocol version (only available when Ready)
    pub async fn protocol_version(&self) -> Option<ProtocolVersion> {
        match *self.state.lock().await {
            ConnectionState::Ready { protocol_version, .. } |
            ConnectionState::SendingInitialized { protocol_version } => {
                Some(protocol_version)
            }
            _ => None,
        }
    }

    /// Check if a request can be sent in current state
    pub async fn can_send_request(&self, method: &str) -> bool {
        let state = self.state.lock().await;
        match (method, &*state) {
            // Initialize can only be sent when connecting
            ("initialize", ConnectionState::Connecting) => true,

            // All other requests require Ready state
            (_, ConnectionState::Ready { .. }) => true,

            // No other requests allowed in other states
            _ => false,
        }
    }

    /// Set the protocol adapter for this connection
    pub async fn set_adapter(&self, adapter: Box<dyn ProtocolAdapter>) {
        let mut guard = self.adapter.write().await;
        *guard = Some(adapter);
    }

    /// Get the protocol adapter (if initialized)
    pub async fn get_adapter(&self) -> Option<Arc<dyn ProtocolAdapter>> {
        let guard = self.adapter.read().await;
        guard.as_ref().map(|a| Arc::clone(a as &Arc<dyn ProtocolAdapter>))
    }
}
```

**State Transitions**:
```
Connecting
    ↓ start_initialization()
Initializing
    ↓ received_initialize_response(version)
SendingInitialized
    ↓ complete_initialization()
Ready
    ↓ (stable state)

Any state → Failed (on error)
Any state → Closing (on shutdown)
```

**Invariants**:
- State transitions are unidirectional (except for error/closing)
- Protocol version is set during `received_initialize_response()` and never changes
- Requests other than `initialize` are only allowed in `Ready` state
- Adapter is set when transitioning to `SendingInitialized`

---

### 4. InitializationHandshake

**Purpose**: Encapsulates the initialization handshake sequence and timing.

**Definition**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    pub jsonrpc: String, // Always "2.0"
    pub id: RequestId,
    pub method: String, // Always "initialize"
    pub params: InitializeParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: InitializeResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: Implementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializedNotification {
    pub jsonrpc: String, // Always "2.0"
    pub method: String, // Always "notifications/initialized"
    // No id field (notifications don't have IDs)
    // No params field (this notification has no parameters)
}

impl InitializedNotification {
    pub fn new() -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: "notifications/initialized".to_string(),
        }
    }
}

/// Tracks timing and status of initialization handshake
#[derive(Debug, Clone)]
pub struct InitializationHandshakeTracker {
    pub started_at: Instant,
    pub initialize_sent_at: Option<Instant>,
    pub initialize_received_at: Option<Instant>,
    pub initialized_sent_at: Option<Instant>,
    pub completed_at: Option<Instant>,
    pub timeout: Duration,
}

impl InitializationHandshakeTracker {
    pub fn new(timeout: Duration) -> Self {
        Self {
            started_at: Instant::now(),
            initialize_sent_at: None,
            initialize_received_at: None,
            initialized_sent_at: None,
            completed_at: None,
            timeout,
        }
    }

    pub fn mark_initialize_sent(&mut self) {
        self.initialize_sent_at = Some(Instant::now());
    }

    pub fn mark_initialize_received(&mut self) {
        self.initialize_received_at = Some(Instant::now());
    }

    pub fn mark_initialized_sent(&mut self) {
        self.initialized_sent_at = Some(Instant::now());
    }

    pub fn mark_completed(&mut self) {
        self.completed_at = Some(Instant::now());
    }

    pub fn is_timed_out(&self) -> bool {
        self.started_at.elapsed() > self.timeout
    }

    pub fn total_duration(&self) -> Option<Duration> {
        self.completed_at.map(|end| end.duration_since(self.started_at))
    }

    pub fn phase_durations(&self) -> InitializationPhaseTimings {
        InitializationPhaseTimings {
            send_initialize: self.initialize_sent_at
                .map(|t| t.duration_since(self.started_at)),
            wait_for_response: self.initialize_received_at
                .and_then(|recv| self.initialize_sent_at.map(|sent| recv.duration_since(sent))),
            send_initialized: self.initialized_sent_at
                .and_then(|sent| self.initialize_received_at.map(|recv| sent.duration_since(recv))),
            mark_ready: self.completed_at
                .and_then(|comp| self.initialized_sent_at.map(|sent| comp.duration_since(sent))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InitializationPhaseTimings {
    pub send_initialize: Option<Duration>,
    pub wait_for_response: Option<Duration>,
    pub send_initialized: Option<Duration>,
    pub mark_ready: Option<Duration>,
}
```

---

## Version-Specific Message Formats

### Resource Messages

**resources/read Response (Version-Specific)**:

```rust
// 2024-11-05 and 2025-03-26 format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContentsV1 {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>, // Base64 encoded
}

// 2025-06-18 format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContentsV2 {
    pub uri: String,
    pub name: String, // REQUIRED in 2025-06-18
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

// Conversion functions
impl ResourceContentsV2 {
    /// Convert from V1 format (generate name from URI)
    pub fn from_v1(v1: ResourceContentsV1) -> Self {
        Self {
            name: generate_resource_name(&v1.uri),
            uri: v1.uri,
            title: None,
            mime_type: v1.mime_type,
            text: v1.text,
            blob: v1.blob,
        }
    }
}

impl From<ResourceContentsV2> for ResourceContentsV1 {
    /// Convert to V1 format (strip name and title)
    fn from(v2: ResourceContentsV2) -> Self {
        Self {
            uri: v2.uri,
            mime_type: v2.mime_type,
            text: v2.text,
            blob: v2.blob,
        }
    }
}
```

### Tool Messages

**tools/list Response (Version-Specific)**:

```rust
// 2024-11-05 and 2025-03-26 format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolV1 {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value, // JSON Schema
}

// 2025-06-18 format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolV2 {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

// Conversion
impl From<ToolV2> for ToolV1 {
    fn from(v2: ToolV2) -> Self {
        Self {
            name: v2.name,
            description: v2.description,
            input_schema: v2.input_schema,
        }
    }
}

impl From<ToolV1> for ToolV2 {
    fn from(v1: ToolV1) -> Self {
        Self {
            name: v1.name,
            title: None,
            description: v1.description,
            input_schema: v1.input_schema,
            output_schema: None,
        }
    }
}
```

**tools/call Response (Version-Specific)**:

```rust
// 2024-11-05 format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResultV1 {
    pub content: Vec<ContentV1>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

// 2025-03-26 format (adds AudioContent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResultV2 {
    pub content: Vec<ContentV2>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

// 2025-06-18 format (adds structuredContent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResultV3 {
    pub content: Vec<ContentV2>,
    #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<serde_json::Value>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}
```

### Content Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentV1 {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image {
        data: String, // Base64
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    #[serde(rename = "resource")]
    Resource { resource: ResourceContentsV1 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentV2 {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    #[serde(rename = "audio")]
    Audio {
        data: String, // Base64
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    #[serde(rename = "resource")]
    Resource { resource: ResourceContentsV2 },
}

// Conversion: V2 → V1 (AudioContent becomes TextContent)
impl From<ContentV2> for ContentV1 {
    fn from(v2: ContentV2) -> Self {
        match v2 {
            ContentV2::Text { text } => ContentV1::Text { text },
            ContentV2::Image { data, mime_type } => ContentV1::Image { data, mime_type },
            ContentV2::Audio { mime_type, .. } => {
                // Convert audio to text description
                ContentV1::Text {
                    text: format!("[Audio content: {}]", mime_type),
                }
            }
            ContentV2::Resource { resource } => ContentV1::Resource {
                resource: resource.into(),
            },
        }
    }
}
```

---

## Relationships Between Entities

```
ServerConnectionState
    ├── contains: ConnectionState (enum)
    ├── has: ProtocolVersion (when Ready)
    ├── has: ProtocolAdapter (when initialized)
    └── uses: InitializationHandshakeTracker

ProtocolAdapter
    ├── translates: JsonRpcRequest
    ├── translates: JsonRpcResponse
    └── translates: JsonRpcNotification

ProtocolVersion
    ├── determines: which ProtocolAdapter to use
    └── determines: message format (via version-specific structs)

InitializationHandshake
    ├── produces: ProtocolVersion (from response)
    └── tracked by: InitializationHandshakeTracker
```

---

## Data Flow

### Initialization Flow

```
1. Proxy creates ServerConnectionState (state = Connecting)
2. Proxy sends InitializeRequest to backend server
3. ServerConnectionState transitions to Initializing
4. Backend server returns InitializeResponse with protocolVersion
5. Proxy extracts ProtocolVersion from response
6. Proxy creates appropriate ProtocolAdapter based on version
7. ServerConnectionState transitions to SendingInitialized
8. Proxy sends InitializedNotification
9. ServerConnectionState transitions to Ready
10. Normal operations begin
```

### Request Translation Flow

```
1. Client sends request to proxy (version unknown/mixed)
2. Proxy looks up ServerConnectionState for target backend
3. Proxy checks if server is Ready (blocks if not)
4. Proxy retrieves ProtocolAdapter from ServerConnectionState
5. ProtocolAdapter translates request to server's protocol version
6. Proxy sends translated request to backend server
7. Backend server responds
8. ProtocolAdapter translates response back to client's version
9. Proxy returns translated response to client
```

---

## Storage and Persistence

**In-Memory State** (no persistence needed):
- `ServerConnectionState`: One per active backend server connection
- `ProtocolAdapter`: Cached per connection, recreated on reconnect
- `InitializationHandshakeTracker`: Ephemeral, discarded after initialization

**No Persistent Storage**:
- Protocol version is re-negotiated on each connection
- No need to persist version history
- Configuration may specify preferred/expected versions, but actual version is always negotiated

---

## Validation Rules

### Message Validation

1. **InitializeResponse Validation**:
   - MUST have `protocolVersion` field
   - `protocolVersion` MUST be one of supported versions
   - MUST have `capabilities` object (can be empty)
   - MUST have `serverInfo` with `name` and `version`

2. **ResourceContents Validation** (version-dependent):
   - 2024-11-05/2025-03-26: MUST have `uri`, one of `text` or `blob`
   - 2025-06-18: MUST have `uri` and `name`, one of `text` or `blob`

3. **Tool Validation** (version-dependent):
   - All versions: MUST have `name`, `description`, `inputSchema`
   - 2025-06-18: `inputSchema` and `outputSchema` MUST be valid JSON Schema if present

### State Transition Validation

- State transitions must follow defined state machine
- Invalid transitions return `ProtocolError::InvalidStateTransition`
- Protocol version can only be set once (during initialization)

---

## Performance Characteristics

**Memory Footprint**:
- `ServerConnectionState`: ~200 bytes per connection
- `ProtocolAdapter`: ~100 bytes (mostly vtable pointer)
- `InitializationHandshakeTracker`: ~80 bytes

**Time Complexity**:
- Version detection: O(1) (string match)
- Adapter selection: O(1) (match expression)
- Message translation: O(n) where n = message size (JSON parsing)
- Pass-through (same version): O(1) (no translation)

**Optimization Opportunities**:
- Pass-through adapter for same-version cases (zero overhead)
- Lazy adapter creation (only when needed)
- Message streaming for large payloads (future enhancement)

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Unsupported protocol version: {reported_version}. Supported: {supported_versions:?}")]
    UnsupportedVersion {
        reported_version: String,
        supported_versions: Vec<String>,
    },

    #[error("Translation failed from {from_version:?} to {to_version:?} for {message_type}: {details}")]
    TranslationError {
        from_version: ProtocolVersion,
        to_version: ProtocolVersion,
        message_type: String,
        details: String,
    },

    #[error("Missing required field '{field_name}' in {message_type} for {version:?}")]
    MissingRequiredField {
        field_name: String,
        message_type: String,
        version: ProtocolVersion,
    },

    #[error("Initialization timeout for server '{server_name}' after {duration:?}")]
    InitializationTimeout {
        server_name: String,
        duration: Duration,
    },

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition {
        from: String,
        to: String,
    },

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

---

## Testing Considerations

### Unit Test Targets

1. **ProtocolVersion**:
   - Test `from_string()` with valid/invalid versions
   - Test feature detection methods (`supports_*`)

2. **ProtocolAdapter**:
   - Test each adapter in isolation
   - Test pass-through adapter (no-op)
   - Test field stripping/addition
   - Test error cases (malformed input)

3. **ServerConnectionState**:
   - Test state transitions (valid and invalid)
   - Test concurrent state queries
   - Test timeout detection

4. **Resource Name Generation**:
   - Test various URI formats
   - Test edge cases (empty URI, special characters)

### Integration Test Targets

1. Full initialization sequence with mock backend
2. Request translation end-to-end
3. Version negotiation with multiple backends
4. Error propagation through layers

### Property Test Targets

1. Round-trip translation (translate forward then backward = identity)
2. Required fields preserved through translation
3. Valid JSON maintained through translation
