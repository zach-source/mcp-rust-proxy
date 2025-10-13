# MCP Protocol Version Differences - Quick Reference

## Protocol Version Strings
- **2024-11-05**: `"2024-11-05"`
- **2025-03-26**: `"2025-03-26"`
- **2025-06-18**: `"2025-06-18"`

---

## Breaking Changes Only

### 2024-11-05 → 2025-03-26
✅ **NO BREAKING CHANGES** - Fully backward compatible

### 2025-03-26 → 2025-06-18
⚠️ **ONE BREAKING CHANGE**:
- `ResourceContents.name` is now **REQUIRED** (previously not present)
- Impact: `resources/read` response format changed

---

## New Features by Version

### 2025-03-26 (from 2024-11-05)
| Feature | Type | Location |
|---------|------|----------|
| `completions` | Server Capability | `ServerCapabilities.completions` |
| `AudioContent` | Content Type | Tool/Prompt responses |
| `resources/updated` | Notification | Server → Client |

### 2025-06-18 (from 2025-03-26)
| Feature | Type | Location |
|---------|------|----------|
| `elicitation` | Client Capability | `ClientCapabilities.elicitation` |
| `title` | Field | Tool, Resource, Prompt objects |
| `outputSchema` | Field | `Tool.outputSchema` (JSON Schema) |
| `structuredContent` | Field | `CallToolResult.structuredContent` |
| `name` ⚠️ | Required Field | `ResourceContents.name` |

---

## Translation Rules for Proxy

### Forward Translation (Old Client → New Server)

#### 2024-11-05 → 2025-06-18
```
When translating resources/read responses:
  - Generate `name` from `uri` (use filename or full URI)
  - Set `title` to null/undefined
```

### Backward Translation (New Client → Old Server)

#### 2025-06-18 → 2024-11-05
```
Strip these fields:
  - Tool.title
  - Tool.outputSchema
  - Resource.title
  - Prompt.title
  - CallToolResult.structuredContent
  - ClientCapabilities.elicitation
  - ServerCapabilities.completions

Convert AudioContent to TextContent:
  - Replace with: "[Audio content: {mimeType}]"
```

#### 2025-06-18 → 2025-03-26
```
Strip these fields:
  - Tool.title
  - Tool.outputSchema
  - Resource.title
  - Prompt.title
  - CallToolResult.structuredContent
  - ClientCapabilities.elicitation
```

---

## Capability Flags by Version

### ClientCapabilities
| Capability | 2024-11-05 | 2025-03-26 | 2025-06-18 |
|------------|------------|------------|------------|
| `roots` | ✅ | ✅ | ✅ |
| `sampling` | ✅ | ✅ | ✅ |
| `elicitation` | ❌ | ❌ | ✅ |
| `experimental` | ✅ | ✅ | ✅ |

### ServerCapabilities
| Capability | 2024-11-05 | 2025-03-26 | 2025-06-18 |
|------------|------------|------------|------------|
| `logging` | ✅ | ✅ | ✅ |
| `prompts` | ✅ | ✅ | ✅ |
| `resources` | ✅ | ✅ | ✅ |
| `tools` | ✅ | ✅ | ✅ |
| `completions` | ❌ | ✅ | ✅ |
| `experimental` | ✅ | ✅ | ✅ |

---

## Message Format Changes

### Resource Objects

#### 2024-11-05 & 2025-03-26
```typescript
interface Resource {
  uri: string              // Required
  name: string             // Required
  description?: string     // Optional
  mimeType?: string        // Optional
}
```

#### 2025-06-18
```typescript
interface Resource {
  uri: string              // Required
  name: string             // Required
  title?: string           // NEW: Optional
  description?: string     // Optional
  mimeType?: string        // Optional
}
```

### ResourceContents (resources/read response)

#### 2024-11-05 & 2025-03-26
```typescript
interface ResourceContents {
  uri: string              // Required
  mimeType?: string        // Optional
  text?: string            // Optional (one of text/blob)
  blob?: string            // Optional (one of text/blob)
}
```

#### 2025-06-18
```typescript
interface ResourceContents {
  uri: string              // Required
  name: string             // ⚠️ NEW: REQUIRED
  title?: string           // NEW: Optional
  mimeType?: string        // Optional
  text?: string            // Optional (one of text/blob)
  blob?: string            // Optional (one of text/blob)
}
```

### Tool Objects

#### 2024-11-05 & 2025-03-26
```typescript
interface Tool {
  name: string             // Required
  description: string      // Required
  inputSchema: JSONSchema  // Required
}
```

#### 2025-06-18
```typescript
interface Tool {
  name: string             // Required
  title?: string           // NEW: Optional
  description: string      // Required
  inputSchema: JSONSchema  // Required
  outputSchema?: JSONSchema // NEW: Optional
}
```

### CallToolResult

#### 2024-11-05
```typescript
interface CallToolResult {
  content: Content[]       // Text | Image | Resource
  isError?: boolean
}
```

#### 2025-03-26
```typescript
interface CallToolResult {
  content: Content[]       // Text | Image | Audio | Resource
  isError?: boolean
}
```

#### 2025-06-18
```typescript
interface CallToolResult {
  content: Content[]       // Text | Image | Audio | Resource
  structuredContent?: JSONValue  // NEW: Optional
  isError?: boolean
}
```

---

## Content Types Support

| Content Type | 2024-11-05 | 2025-03-26 | 2025-06-18 |
|--------------|------------|------------|------------|
| `TextContent` | ✅ | ✅ | ✅ |
| `ImageContent` | ✅ | ✅ | ✅ |
| `AudioContent` | ❌ | ✅ | ✅ |
| `EmbeddedResource` | ✅ | ✅ | ✅ |

---

## Initialization Sequence (Unchanged Across All Versions)

```
1. Client → Server: initialize request
   {
     method: "initialize",
     params: {
       protocolVersion: "2025-06-18",  // Version varies
       capabilities: { ... },
       clientInfo: { name, version }
   }

2. Server → Client: initialize response
   {
     result: {
       protocolVersion: "2025-06-18",  // Negotiated version
       capabilities: { ... },
       serverInfo: { name, version }
     }
   }

3. Client → Server: initialized notification
   {
     method: "notifications/initialized"
   }

4. Normal operations begin
```

---

## Version Negotiation

### Rules (Unchanged Across Versions)
1. Client sends **latest** version it supports
2. Server responds with **compatible** version (same or older)
3. Both parties use **negotiated** version going forward
4. Disconnect if no compatible version exists

### Example Negotiation Table
| Client Sends | Server Supports | Server Responds | Result |
|--------------|-----------------|-----------------|--------|
| 2025-06-18 | 2024-11-05 | 2024-11-05 | ✅ Use 2024-11-05 |
| 2025-06-18 | 2025-03-26 | 2025-03-26 | ✅ Use 2025-03-26 |
| 2025-06-18 | 2025-06-18 | 2025-06-18 | ✅ Use 2025-06-18 |
| 2024-11-05 | 2025-06-18 | 2024-11-05 | ✅ Use 2024-11-05 |

---

## Notifications (All Versions Support)

| Notification | Direction | Purpose |
|--------------|-----------|---------|
| `notifications/initialized` | Client → Server | Initialization complete |
| `notifications/resources/list_changed` | Server → Client | Resource list changed |
| `notifications/tools/list_changed` | Server → Client | Tool list changed |
| `notifications/prompts/list_changed` | Server → Client | Prompt list changed |
| `notifications/progress` | Bidirectional | Progress updates |
| `notifications/resources/updated` | Server → Client | Subscribed resource changed (2025-03-26+) |

---

## Testing Checklist for Translation Layer

### Must Test
- [ ] Pass-through when versions match (no translation)
- [ ] Generate `ResourceContents.name` when upgrading to 2025-06-18
- [ ] Strip `title` fields when downgrading from 2025-06-18
- [ ] Strip `outputSchema` when downgrading from 2025-06-18
- [ ] Strip `structuredContent` when downgrading from 2025-06-18
- [ ] Convert `AudioContent` to `TextContent` when downgrading to 2024-11-05
- [ ] Strip `completions` capability when downgrading to 2024-11-05
- [ ] Strip `elicitation` capability when downgrading from 2025-06-18
- [ ] Preserve all other fields during translation
- [ ] Handle missing optional fields gracefully
- [ ] Validate required fields are present after translation

---

## Implementation Priority

### High Priority (Required for Basic Functionality)
1. ✅ Version string detection and negotiation
2. ✅ ResourceContents.name generation/stripping (breaking change)
3. ✅ Pass-through mode for matching versions

### Medium Priority (Common Use Cases)
4. ⚠️ Tool.outputSchema and structuredContent handling
5. ⚠️ Title field stripping/preservation
6. ⚠️ AudioContent conversion

### Low Priority (Edge Cases)
7. 📋 Completions capability filtering
8. 📋 Elicitation capability filtering
9. 📋 Experimental capabilities handling

---

## Common Pitfalls

### ❌ Don't Do This
- Modify pass-through traffic when versions match
- Drop unknown fields (preserve for forward compatibility)
- Assume field order matters (JSON objects are unordered)
- Use string concatenation for JSON manipulation

### ✅ Do This
- Use proper JSON parsing/serialization
- Validate messages after translation
- Log translation operations for debugging
- Handle malformed input gracefully
- Preserve unknown fields in experimental namespaces

---

## Rust Implementation Hints

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolVersion {
    V20241105,
    V20250326,
    V20250618,
}

impl ProtocolVersion {
    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "2024-11-05" => Some(Self::V20241105),
            "2025-03-26" => Some(Self::V20250326),
            "2025-06-18" => Some(Self::V20250618),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V20241105 => "2024-11-05",
            Self::V20250326 => "2025-03-26",
            Self::V20250618 => "2025-06-18",
        }
    }

    pub fn supports_audio_content(&self) -> bool {
        matches!(self, Self::V20250326 | Self::V20250618)
    }

    pub fn supports_completions(&self) -> bool {
        matches!(self, Self::V20250326 | Self::V20250618)
    }

    pub fn requires_resource_name(&self) -> bool {
        matches!(self, Self::V20250618)
    }

    pub fn supports_structured_content(&self) -> bool {
        matches!(self, Self::V20250618)
    }
}
```

---

## Quick Decision Tree

```
When translating a message:

1. Do versions match?
   YES → Pass through unchanged
   NO  → Continue to step 2

2. Is this resources/read response?
   YES → Check if target version is 2025-06-18
         YES → Add 'name' field (generate from URI if missing)
         NO  → Remove 'name' and 'title' fields if present
   NO  → Continue to step 3

3. Is this tools/list or tools/call?
   YES → Check target version
         2025-06-18 → Keep title/outputSchema/structuredContent
         Earlier    → Strip title/outputSchema/structuredContent
   NO  → Continue to step 4

4. Contains AudioContent?
   YES → Check target version
         2024-11-05 → Convert to TextContent description
         Later      → Keep as-is
   NO  → Continue to step 5

5. Is this initialize request/response?
   YES → Filter capabilities based on target version
         2024-11-05 → Strip completions, elicitation
         2025-03-26 → Strip elicitation only
         2025-06-18 → Keep all
   NO  → Pass through (likely no translation needed)
```
