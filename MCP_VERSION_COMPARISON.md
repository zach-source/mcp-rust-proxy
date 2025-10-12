# Model Context Protocol (MCP) Version Comparison

## Executive Summary

This document compares three versions of the Model Context Protocol (MCP) specification:
- **2024-11-05**: Initial stable release
- **2025-03-26**: First major update
- **2025-06-18**: Latest version with new features

## Protocol Versions

| Aspect | 2024-11-05 | 2025-03-26 | 2025-06-18 |
|--------|------------|------------|------------|
| Protocol Version String | `"2024-11-05"` | `"2025-03-26"` | `"2025-06-18"` |
| JSON-RPC Version | `"2.0"` | `"2.0"` | `"2.0"` |
| Base Transport | JSON-RPC 2.0 | JSON-RPC 2.0 | JSON-RPC 2.0 |

---

## Initialization Sequence

### Common Flow (All Versions)
1. **Client → Server**: `initialize` request with protocol version and capabilities
2. **Server → Client**: Response with negotiated protocol version and server capabilities
3. **Client → Server**: `initialized` notification (signals completion)
4. Normal operations begin

### Initialization Request Format

#### 2024-11-05
```typescript
interface InitializeRequest {
  method: "initialize"
  params: {
    protocolVersion: string          // "2024-11-05"
    capabilities: ClientCapabilities
    clientInfo: Implementation       // name, version
  }
}
```

#### 2025-03-26
```typescript
interface InitializeRequest {
  method: "initialize"
  params: {
    protocolVersion: string          // "2025-03-26"
    capabilities: ClientCapabilities
    clientInfo: Implementation       // name, version
  }
}
```
**Changes**: None (structure identical)

#### 2025-06-18
```typescript
interface InitializeRequest {
  method: "initialize"
  params: {
    protocolVersion: string          // "2025-06-18"
    capabilities: ClientCapabilities
    clientInfo: Implementation       // name, version
  }
}
```
**Changes**: None (structure identical)

### Initialization Response Format

#### All Versions (Identical Structure)
```typescript
interface InitializeResult {
  protocolVersion: string              // Negotiated version
  capabilities: ServerCapabilities
  serverInfo: Implementation           // name, version
  instructions?: string                // Optional server instructions
}
```

---

## Capability Negotiation

### Client Capabilities

#### 2024-11-05
```typescript
interface ClientCapabilities {
  experimental?: { [key: string]: object }
  roots?: {
    listChanged?: boolean              // Support for root change notifications
  }
  sampling?: object                    // LLM sampling support
}
```

#### 2025-03-26
```typescript
interface ClientCapabilities {
  experimental?: { [key: string]: object }
  roots?: {
    listChanged?: boolean
  }
  sampling?: object
}
```
**Changes**: None

#### 2025-06-18
```typescript
interface ClientCapabilities {
  experimental?: { [key: string]: object }
  roots?: {
    listChanged?: boolean
  }
  sampling?: object
  elicitation?: object                 // NEW: Support for elicitation
}
```
**Changes**:
- ✨ **NEW**: Added `elicitation` capability for client-side elicitation support

### Server Capabilities

#### 2024-11-05
```typescript
interface ServerCapabilities {
  experimental?: { [key: string]: object }
  logging?: object                     // Server logging support
  prompts?: {
    listChanged?: boolean              // Prompt list change notifications
  }
  resources?: {
    subscribe?: boolean                // Resource subscription support
    listChanged?: boolean              // Resource list change notifications
  }
  tools?: {
    listChanged?: boolean              // Tool list change notifications
  }
}
```

#### 2025-03-26
```typescript
interface ServerCapabilities {
  experimental?: { [key: string]: object }
  logging?: object
  completions?: object                 // NEW: Code completion support
  prompts?: {
    listChanged?: boolean
  }
  resources?: {
    subscribe?: boolean
    listChanged?: boolean
  }
  tools?: {
    listChanged?: boolean
  }
}
```
**Changes**:
- ✨ **NEW**: Added `completions` capability for autocomplete/suggestion support

#### 2025-06-18
```typescript
interface ServerCapabilities {
  experimental?: { [key: string]: object }
  logging?: object
  completions?: object
  prompts?: {
    listChanged?: boolean
  }
  resources?: {
    subscribe?: boolean
    listChanged?: boolean
  }
  tools?: {
    listChanged?: boolean
  }
}
```
**Changes**: None (same as 2025-03-26)

---

## Message Formats

### Common JSON-RPC Structure (All Versions)

#### Request
```typescript
{
  jsonrpc: "2.0"
  id: string | number                  // Must be unique, cannot be null
  method: string
  params?: { [key: string]: unknown }
}
```

#### Response
```typescript
{
  jsonrpc: "2.0"
  id: string | number                  // Matches request ID
  result?: { [key: string]: unknown }  // Present on success
  error?: {                            // Present on error (mutually exclusive with result)
    code: number
    message: string
    data?: unknown
  }
}
```

#### Notification
```typescript
{
  jsonrpc: "2.0"
  method: string
  params?: { [key: string]: unknown }
  // Note: No 'id' field - notifications don't expect responses
}
```

---

## Tools API

### tools/list Request (All Versions - Identical)
```typescript
interface ListToolsRequest {
  method: "tools/list"
  params?: {
    cursor?: string                    // Optional pagination cursor
  }
}
```

### tools/list Response

#### 2024-11-05
```typescript
interface ListToolsResult {
  tools: Tool[]
  nextCursor?: string                  // Pagination cursor
}

interface Tool {
  name: string                         // Unique identifier
  description: string                  // Human-readable description
  inputSchema: JSONSchema              // JSON Schema for parameters
}
```

#### 2025-03-26
```typescript
interface ListToolsResult {
  tools: Tool[]
  nextCursor?: string
}

interface Tool {
  name: string
  description: string
  inputSchema: JSONSchema
}
```
**Changes**: None

#### 2025-06-18
```typescript
interface ListToolsResult {
  tools: Tool[]
  nextCursor?: string
}

interface Tool {
  name: string
  title?: string                       // NEW: Optional human-readable title
  description: string
  inputSchema: JSONSchema
  outputSchema?: JSONSchema            // NEW: Optional output schema
}
```
**Changes**:
- ✨ **NEW**: Added optional `title` field for display purposes
- ✨ **NEW**: Added optional `outputSchema` for structured output validation

### tools/call Request (All Versions - Identical)
```typescript
interface CallToolRequest {
  method: "tools/call"
  params: {
    name: string                       // Tool identifier (required)
    arguments?: { [key: string]: unknown }  // Tool-specific parameters
  }
}
```

### tools/call Response

#### 2024-11-05
```typescript
interface CallToolResult {
  content: Content[]                   // Array of text/image/resource items
  isError?: boolean                    // Indicates failure (default: false)
}

type Content = TextContent | ImageContent | EmbeddedResource
```

#### 2025-03-26
```typescript
interface CallToolResult {
  content: Content[]
  isError?: boolean
}

type Content = TextContent | ImageContent | AudioContent | EmbeddedResource
```
**Changes**:
- ✨ **NEW**: Added `AudioContent` type support

#### 2025-06-18
```typescript
interface CallToolResult {
  content: Content[]
  structuredContent?: JSONValue        // NEW: Optional structured output
  isError?: boolean
}

type Content = TextContent | ImageContent | AudioContent | EmbeddedResource
```
**Changes**:
- ✨ **NEW**: Added `structuredContent` field for machine-readable structured responses
- Complements the `outputSchema` field in Tool definition

---

## Resources API

### resources/list Request (All Versions - Identical)
```typescript
interface ListResourcesRequest {
  method: "resources/list"
  params?: {
    cursor?: string                    // Optional pagination cursor
  }
}
```

### resources/list Response

#### 2024-11-05
```typescript
interface ListResourcesResult {
  resources: Resource[]
  nextCursor?: string
}

interface Resource {
  uri: string                          // Unique identifier
  name: string                         // Human-readable name
  description?: string                 // Optional description
  mimeType?: string                    // Optional MIME type
}
```

#### 2025-03-26
```typescript
// Identical to 2024-11-05
```
**Changes**: None

#### 2025-06-18
```typescript
interface ListResourcesResult {
  resources: Resource[]
  nextCursor?: string
}

interface Resource {
  uri: string
  name: string
  title?: string                       // NEW: Optional human-readable title
  description?: string
  mimeType?: string
}
```
**Changes**:
- ✨ **NEW**: Added optional `title` field (consistent with Tool changes)

### resources/read Request (All Versions - Identical)
```typescript
interface ReadResourceRequest {
  method: "resources/read"
  params: {
    uri: string                        // Required resource URI
  }
}
```

### resources/read Response

#### 2024-11-05
```typescript
interface ReadResourceResult {
  contents: ResourceContents[]
}

interface ResourceContents {
  uri: string                          // Resource identifier
  mimeType?: string                    // Optional MIME type
  text?: string                        // For text resources
  blob?: string                        // For binary resources (base64)
}
```

#### 2025-03-26
```typescript
// Identical to 2024-11-05
```
**Changes**: None

#### 2025-06-18
```typescript
interface ReadResourceResult {
  contents: ResourceContents[]
}

interface ResourceContents {
  uri: string
  name: string                         // NEW: Resource name (required)
  title?: string                       // NEW: Optional title
  mimeType?: string
  text?: string
  blob?: string
}
```
**Changes**:
- ⚠️ **BREAKING**: Added required `name` field to ResourceContents
- ✨ **NEW**: Added optional `title` field

---

## Prompts API

### prompts/list Request (All Versions - Identical)
```typescript
interface ListPromptsRequest {
  method: "prompts/list"
  params?: {
    cursor?: string                    // Optional pagination cursor
  }
}
```

### prompts/list Response

#### 2024-11-05
```typescript
interface ListPromptsResult {
  prompts: Prompt[]
  nextCursor?: string
}

interface Prompt {
  name: string                         // Unique identifier
  description?: string                 // Optional description
  arguments?: PromptArgument[]         // Optional argument definitions
}

interface PromptArgument {
  name: string
  description?: string
  required?: boolean
}
```

#### 2025-03-26
```typescript
// Identical to 2024-11-05
```
**Changes**: None

#### 2025-06-18
```typescript
interface ListPromptsResult {
  prompts: Prompt[]
  nextCursor?: string
}

interface Prompt {
  name: string
  title?: string                       // NEW: Optional human-readable title
  description?: string
  arguments?: PromptArgument[]
}
```
**Changes**:
- ✨ **NEW**: Added optional `title` field (consistent pattern across all list endpoints)

### prompts/get Request (All Versions - Identical)
```typescript
interface GetPromptRequest {
  method: "prompts/get"
  params: {
    name: string                       // Prompt identifier (required)
    arguments?: { [key: string]: string }  // Prompt-specific parameters
  }
}
```

### prompts/get Response (All Versions - Identical)
```typescript
interface GetPromptResult {
  description?: string
  messages: PromptMessage[]
}

interface PromptMessage {
  role: "user" | "assistant"
  content: Content[]                   // Text, image, audio, or resource content
}
```

---

## Notifications

### Core Notifications (All Versions)

#### initialized
```typescript
{
  method: "notifications/initialized"
  // No params - signals initialization complete
}
```
**Direction**: Client → Server
**When**: After receiving InitializeResult, before any other requests
**Changes across versions**: None

#### resources/list_changed
```typescript
{
  method: "notifications/resources/list_changed"
  // No params - signals resource list has changed
}
```
**Direction**: Server → Client
**When**: When the list of available resources changes
**Requires**: Server declares `resources.listChanged: true` capability
**Changes across versions**: None

#### tools/list_changed
```typescript
{
  method: "notifications/tools/list_changed"
  // No params - signals tool list has changed
}
```
**Direction**: Server → Client
**When**: When the list of available tools changes
**Requires**: Server declares `tools.listChanged: true` capability
**Changes across versions**: None

#### prompts/list_changed
```typescript
{
  method: "notifications/prompts/list_changed"
  // No params - signals prompt list has changed
}
```
**Direction**: Server → Client
**When**: When the list of available prompts changes
**Requires**: Server declares `prompts.listChanged: true` capability
**Changes across versions**: None

### Additional Notifications

#### resources/updated (2025-03-26+)
```typescript
{
  method: "notifications/resources/updated"
  params: {
    uri: string                        // URI of the updated resource
  }
}
```
**Direction**: Server → Client
**When**: When a subscribed resource's content changes
**Requires**: Server declares `resources.subscribe: true` capability
**Added in**: 2025-03-26
**Changes**: None between 2025-03-26 and 2025-06-18

#### progress (All versions)
```typescript
{
  method: "notifications/progress"
  params: {
    progressToken: string | number     // Token from original request
    progress: number                   // Current progress value
    total?: number                     // Optional total value
  }
}
```
**Direction**: Bidirectional (Server → Client or Client → Server)
**When**: During long-running operations
**Changes across versions**: None

---

## Breaking Changes Summary

### 2024-11-05 → 2025-03-26
**No breaking changes** - All changes are additive:
- ✨ Added `completions` server capability
- ✨ Added `AudioContent` content type
- ✨ Added `resources/updated` notification

### 2025-03-26 → 2025-06-18
**One breaking change**:
- ⚠️ **BREAKING**: `ResourceContents.name` is now required (was not present before)

**Other changes** (non-breaking, all additive):
- ✨ Added `elicitation` client capability
- ✨ Added `title` fields to Tool, Resource, Prompt definitions
- ✨ Added `outputSchema` to Tool definition
- ✨ Added `structuredContent` to CallToolResult

---

## Version Negotiation Rules

All versions follow the same negotiation pattern:

1. **Client sends**: Latest protocol version it supports (e.g., `"2025-06-18"`)
2. **Server responds with**:
   - Same version if supported
   - Earlier compatible version if client's version not supported
   - Latest version server supports if client sent older version
3. **Client must**:
   - Accept server's version if compatible
   - Disconnect if no mutual compatible version exists
4. **Both parties must**:
   - Only use features from negotiated protocol version
   - Respect capability flags for optional features

### Compatibility Matrix

| Client Version | Server Version | Compatible? | Negotiated Version | Notes |
|----------------|----------------|-------------|-------------------|-------|
| 2024-11-05 | 2024-11-05 | ✅ Yes | 2024-11-05 | Perfect match |
| 2024-11-05 | 2025-03-26 | ✅ Yes | 2024-11-05 | Server downgrades |
| 2024-11-05 | 2025-06-18 | ✅ Yes | 2024-11-05 | Server downgrades |
| 2025-03-26 | 2024-11-05 | ✅ Yes | 2024-11-05 | Client downgrades |
| 2025-03-26 | 2025-03-26 | ✅ Yes | 2025-03-26 | Perfect match |
| 2025-03-26 | 2025-06-18 | ✅ Yes | 2025-03-26 | Server downgrades |
| 2025-06-18 | 2024-11-05 | ⚠️ Maybe | 2024-11-05 | Client must handle missing features |
| 2025-06-18 | 2025-03-26 | ⚠️ Maybe | 2025-03-26 | Client must handle ResourceContents.name requirement |
| 2025-06-18 | 2025-06-18 | ✅ Yes | 2025-06-18 | Perfect match |

---

## Error Handling

### Standard JSON-RPC Error Codes (All Versions)

```typescript
enum ErrorCode {
  // JSON-RPC standard errors
  ParseError = -32700,           // Invalid JSON
  InvalidRequest = -32600,       // Invalid request object
  MethodNotFound = -32601,       // Method doesn't exist
  InvalidParams = -32602,        // Invalid method parameters
  InternalError = -32603,        // Internal JSON-RPC error

  // MCP-specific errors (implementation defined)
  // Typically in range -32000 to -32099
}
```

**Changes across versions**: None

---

## Required vs Optional Fields

### Universally Required Fields

#### All Request Messages
- `jsonrpc`: Must be `"2.0"`
- `id`: Required for requests (string or number, not null)
- `method`: Required (string)

#### All Response Messages
- `jsonrpc`: Must be `"2.0"`
- `id`: Required (must match request)
- `result` XOR `error`: Exactly one required

#### Initialize Request
- `params.protocolVersion`: Required
- `params.capabilities`: Required (but may be empty object)
- `params.clientInfo`: Required
  - `clientInfo.name`: Required
  - `clientInfo.version`: Required

#### Initialize Response
- `protocolVersion`: Required
- `capabilities`: Required (but may be empty object)
- `serverInfo`: Required
  - `serverInfo.name`: Required
  - `serverInfo.version`: Required

### Version-Specific Required Fields

#### 2025-06-18 Only
- ⚠️ `ResourceContents.name`: **Required** (not present in earlier versions)
- ⚠️ `ResourceContents.uri`: Required (consistent across all versions)

---

## Timeout Recommendations (All Versions)

1. **Request Timeouts**
   - Establish default timeout for all requests
   - Allow per-request timeout configuration
   - Typical range: 30-120 seconds

2. **Progress Notifications**
   - Can optionally reset timeout when progress received
   - Prevents timeout on long-running operations

3. **Maximum Timeout**
   - Enforce absolute maximum (e.g., 10 minutes)
   - Prevent indefinite hangs

4. **Ping Messages**
   - Can be sent during initialization phase
   - Keep connection alive during idle periods

**Changes across versions**: None

---

## Implementation Recommendations for Translation Layer

### 1. Version Detection
```rust
fn detect_protocol_version(init_request: &InitializeRequest) -> ProtocolVersion {
    match init_request.params.protocol_version.as_str() {
        "2024-11-05" => ProtocolVersion::V20241105,
        "2025-03-26" => ProtocolVersion::V20250326,
        "2025-06-18" => ProtocolVersion::V20250618,
        _ => ProtocolVersion::Unknown,
    }
}
```

### 2. Forward Translation (Older → Newer)
- **2024-11-05 → 2025-03-26**:
  - No changes needed (fully compatible)

- **2024-11-05 → 2025-06-18**:
  - When receiving `ReadResourceResult` from old server:
    - Generate `name` field from `uri` (use last path component or full URI)
    - Leave `title` empty

- **2025-03-26 → 2025-06-18**:
  - Same as above for ResourceContents

### 3. Backward Translation (Newer → Older)
- **2025-06-18 → 2025-03-26**:
  - Strip `title` fields from Tool, Resource, Prompt objects
  - Strip `outputSchema` from Tool
  - Strip `structuredContent` from CallToolResult
  - Strip `elicitation` from ClientCapabilities

- **2025-06-18 → 2024-11-05**:
  - All of the above, plus:
  - Strip `completions` from ServerCapabilities
  - Convert AudioContent to TextContent with description

### 4. Capability Filtering
```rust
fn filter_capabilities_for_version(
    caps: ServerCapabilities,
    version: ProtocolVersion,
) -> ServerCapabilities {
    match version {
        ProtocolVersion::V20241105 => {
            // Remove completions capability
            ServerCapabilities {
                completions: None,
                ..caps
            }
        },
        _ => caps,
    }
}
```

### 5. Content Type Handling
```rust
fn translate_content(content: Content, target_version: ProtocolVersion) -> Content {
    match (content, target_version) {
        (Content::Audio(audio), ProtocolVersion::V20241105) => {
            // Downgrade audio to text description
            Content::Text(TextContent {
                text: format!("[Audio content: {}]", audio.mime_type.unwrap_or_default()),
            })
        },
        _ => content,
    }
}
```

---

## Testing Matrix for Translation Layer

| Test Case | Client Version | Server Version | Expected Behavior |
|-----------|----------------|----------------|-------------------|
| Same version | 2024-11-05 | 2024-11-05 | Pass-through, no translation |
| Same version | 2025-03-26 | 2025-03-26 | Pass-through, no translation |
| Same version | 2025-06-18 | 2025-06-18 | Pass-through, no translation |
| Minor upgrade | 2024-11-05 | 2025-03-26 | Forward translate, add name field to resources |
| Major upgrade | 2024-11-05 | 2025-06-18 | Forward translate, add name/title fields |
| Minor downgrade | 2025-03-26 | 2024-11-05 | Backward translate, strip completions |
| Major downgrade | 2025-06-18 | 2024-11-05 | Backward translate, strip multiple fields |
| Tool with outputSchema | 2025-06-18 | 2024-11-05 | Strip outputSchema, keep inputSchema |
| Tool response with structuredContent | 2025-06-18 | 2024-11-05 | Strip structuredContent, keep content array |
| Audio content | 2025-03-26 | 2024-11-05 | Convert AudioContent to TextContent |
| Resource without name | 2024-11-05 | 2025-06-18 | Generate name from URI |

---

## Appendix: Content Type Evolution

### Text Content (All Versions)
```typescript
interface TextContent {
  type: "text"
  text: string
}
```

### Image Content (All Versions)
```typescript
interface ImageContent {
  type: "image"
  data: string                         // Base64 encoded
  mimeType: string
}
```

### Audio Content (2025-03-26+)
```typescript
interface AudioContent {
  type: "audio"
  data: string                         // Base64 encoded
  mimeType: string
}
```

### Embedded Resource (All Versions)
```typescript
interface EmbeddedResource {
  type: "resource"
  resource: ResourceContents           // Full resource object
}
```

---

## Summary of Key Changes

### What's New in 2025-03-26
1. ✨ `completions` server capability
2. ✨ `AudioContent` content type
3. ✨ `resources/updated` notification

### What's New in 2025-06-18
1. ⚠️ **BREAKING**: `ResourceContents.name` now required
2. ✨ `elicitation` client capability
3. ✨ `title` field added to Tool, Resource, Prompt
4. ✨ `outputSchema` field added to Tool
5. ✨ `structuredContent` field added to CallToolResult

### Migration Path
- **Forward compatibility**: All versions can interoperate
- **Translation required**: When crossing version boundaries
- **Field generation**: Proxy must synthesize missing required fields (e.g., `name`)
- **Field stripping**: Proxy must remove unsupported fields when downgrading

---

## References

- Official Specification: https://modelcontextprotocol.io/specification/
- Schema Repository: https://github.com/modelcontextprotocol/specification/tree/main/schema
- JSON-RPC 2.0: https://www.jsonrpc.org/specification
