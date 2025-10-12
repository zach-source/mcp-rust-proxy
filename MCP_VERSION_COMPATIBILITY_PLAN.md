# MCP Protocol Version Compatibility & Translation Plan

## Overview

Implement intelligent protocol version negotiation and translation in MCP Rust Proxy to:
1. Support clients using any MCP protocol version (2024-11-05 → 2025-06-18+)
2. Support backend servers using different protocol versions
3. Automatically translate between protocol versions
4. Maintain backward compatibility while exposing latest features

## Known MCP Protocol Versions

### Version Timeline
1. **"0.1.0"** - Pre-release/alpha format (undocumented)
2. **"2024-11-05"** - Initial public release (Nov 2024)
3. **"2025-03-26"** - Major update (Mar 2025)
4. **"2025-06-18"** - Current latest (Jun 2025)

### Version Differences

#### 2024-11-05 → 2025-03-26
**Added:**
- OAuth 2.1 authorization framework
- Tool annotations (readOnly, destructive, etc.)
- Streamable HTTP transport (replaced HTTP+SSE)
- JSON-RPC batching
- Audio data support
- Completion/autocomplete capability
- `message` field in `ProgressNotification`

**Method Changes:**
- None (methods remained compatible)

**Breaking Changes:**
- Transport layer changed (HTTP+SSE → Streamable HTTP)
- Authorization model introduced

#### 2025-03-26 → 2025-06-18
**Added:**
- Structured tool outputs
- Enhanced OAuth security (Resource Server requirements)
- Elicitation capability (server-initiated user interaction)
- Resource links in tool results
- MCP-Protocol-Version HTTP header requirement

**Removed:**
- JSON-RPC batching (simplified)

**Method Changes:**
- Tools now return structured outputs
- New `elicit` capability

**Breaking Changes:**
- Batching removed
- HTTP header requirements changed

## Current Proxy Status

### What We Support
- ✅ Send `2025-03-26` to backend servers
- ✅ Accept any version in response
- ✅ Basic method routing (tools/list, resources/list, prompts/list)
- ⚠️ No version translation
- ⚠️ No version-specific feature detection

### Compatibility Issues Found

#### Issue 1: Git Server Uses 2024-11-05
- **Observed**: Git returns `"protocolVersion": "2024-11-05"`
- **Impact**: Missing tool annotations, no OAuth support
- **Current Behavior**: Tools work but lack metadata

#### Issue 2: Serena Uses 2025-06-18
- **Observed**: Serena returns `"protocolVersion": "2025-06-18"`
- **Impact**: May use structured outputs, elicitation
- **Current Behavior**: Works but may not expose all features

#### Issue 3: Mixed Version Aggregation
- **Problem**: Aggregating tools from servers with different protocol versions
- **Impact**: Feature set is lowest common denominator
- **Current Behavior**: Pass-through without normalization

## Implementation Plan

### Phase 1: Version Detection & Storage

#### Task 1.1: Store Server Protocol Versions
**File**: `src/state/mod.rs`

Add to `ServerInfo`:
```rust
pub struct ServerInfo {
    // ... existing fields
    pub protocol_version: Arc<RwLock<Option<String>>>,
    pub capabilities: Arc<RwLock<Option<Value>>>,
}
```

#### Task 1.2: Capture Version During Initialization
**File**: `src/transport/pool.rs`

Update `initialize_connection()` to:
1. Store server's returned protocol version
2. Store server's capabilities
3. Log version mismatches

```rust
// After receiving initialize response
let version = resp.result.get("protocolVersion").and_then(|v| v.as_str());
let capabilities = resp.result.get("capabilities");

// Store in ServerInfo
if let Some(info) = self.state.servers.get(server_name) {
    *info.protocol_version.write().await = version.map(|v| v.to_string());
    *info.capabilities.write().await = capabilities.cloned();
}
```

### Phase 2: Version Translation Layer

#### Task 2.1: Create Version Translator
**File**: `src/proxy/version_translator.rs`

```rust
pub struct VersionTranslator {
    client_version: String,
    server_version: String,
}

impl VersionTranslator {
    pub fn new(client_version: String, server_version: String) -> Self;

    // Translate tool from server version to client version
    pub fn translate_tool(&self, tool: Value) -> Value;

    // Translate resource from server version to client version
    pub fn translate_resource(&self, resource: Value) -> Value;

    // Translate tool call result from server version to client version
    pub fn translate_tool_result(&self, result: Value) -> Value;

    // Check if feature is supported in both versions
    pub fn supports_feature(&self, feature: &str) -> bool;
}
```

#### Task 2.2: Version-Specific Feature Mapping
**File**: `src/proxy/version_features.rs`

```rust
pub enum ProtocolFeature {
    OAuth2,
    ToolAnnotations,
    Batching,
    AudioData,
    Completions,
    StructuredOutputs,
    Elicitation,
    ResourceLinks,
}

pub fn get_features_for_version(version: &str) -> HashSet<ProtocolFeature> {
    match version {
        "2024-11-05" => vec![].into_iter().collect(),
        "2025-03-26" => vec![
            ProtocolFeature::OAuth2,
            ProtocolFeature::ToolAnnotations,
            ProtocolFeature::Batching,
            ProtocolFeature::AudioData,
            ProtocolFeature::Completions,
        ].into_iter().collect(),
        "2025-06-18" => vec![
            ProtocolFeature::OAuth2,
            ProtocolFeature::ToolAnnotations,
            ProtocolFeature::AudioData,
            ProtocolFeature::Completions,
            ProtocolFeature::StructuredOutputs,
            ProtocolFeature::Elicitation,
            ProtocolFeature::ResourceLinks,
        ].into_iter().collect(),
        _ => vec![].into_iter().collect(),
    }
}
```

### Phase 3: Translation Implementation

#### Task 3.1: Tool Translation (2024-11-05 → 2025-06-18)
**Upward Translation** (old server → new client):
- Add default tool annotations (`readOnly: false`)
- Ensure `inputSchema` is present
- Add empty `resourceLinks` if missing

**Downward Translation** (new server → old client):
- Strip tool annotations
- Remove `resourceLinks`
- Remove structured output schemas

#### Task 3.2: Tool Result Translation
**For 2025-06-18 Servers** (structured outputs):
```json
// Server returns:
{
  "content": [{
    "type": "text",
    "text": "Result"
  }],
  "isError": false,
  "meta": {
    "resourceLinks": [...]
  }
}

// Translate to 2024-11-05:
{
  "content": [{
    "type": "text",
    "text": "Result"
  }],
  "isError": false
}
```

#### Task 3.3: Capability Translation
**Upward** (old → new):
- Add missing capability stubs
- Set listChanged defaults

**Downward** (new → old):
- Remove unsupported capabilities
- Simplify to common denominator

### Phase 4: Proxy as Version Bridge

#### Task 4.1: Expose Latest Version to Clients
**Current**: Proxy advertises `2025-03-26`
**Enhancement**: Advertise `2025-06-18` or latest
**Benefit**: Clients get modern features even if backends are old

#### Task 4.2: Accept Any Client Version
**Current**: Accept whatever client sends
**Enhancement**:
1. Detect client version from `initialize` request
2. Store client version per connection
3. Translate all responses to client's version
4. Log version mismatches

#### Task 4.3: Per-Server Version Adaptation
When proxying to backend servers:
1. Check server's protocol version from cache
2. Translate request to server's version if needed
3. Translate response back to client's version
4. Handle feature availability gracefully

### Phase 5: Version Negotiation Intelligence

#### Task 5.1: Smart Version Selection
```rust
pub fn negotiate_version(
    client_requested: &str,
    server_supported: Vec<String>
) -> Option<String> {
    // Prefer client's version if server supports it
    if server_supported.contains(&client_requested.to_string()) {
        return Some(client_requested.to_string());
    }

    // Otherwise, find highest common version
    let client_date = parse_version_date(client_requested)?;
    let compatible_versions: Vec<_> = server_supported
        .iter()
        .filter(|v| parse_version_date(v).is_some())
        .collect();

    // Return newest version that's <= client version
    compatible_versions
        .into_iter()
        .map(|v| (v, parse_version_date(v).unwrap()))
        .filter(|(_, date)| date <= &client_date)
        .max_by_key(|(_, date)| *date)
        .map(|(v, _)| v.clone())
}
```

#### Task 5.2: Version Fallback Strategy
If translation fails:
1. **Log warning** with version details
2. **Pass through unchanged** (best effort)
3. **Include metadata** in error responses about version incompatibility
4. **Suggest upgrade** paths in error messages

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_translate_tool_2024_to_2025() {
    let tool_old = json!({
        "name": "test_tool",
        "description": "Test",
        "inputSchema": {...}
    });

    let translator = VersionTranslator::new("2025-06-18", "2024-11-05");
    let tool_new = translator.translate_tool(tool_old);

    assert!(tool_new.get("readOnly").is_some());
}

#[test]
fn test_version_negotiation() {
    let client = "2025-03-26";
    let server_versions = vec!["2024-11-05", "2025-03-26"];
    let agreed = negotiate_version(client, server_versions);
    assert_eq!(agreed, Some("2025-03-26".to_string()));
}
```

### Integration Tests
1. Connect old client (2024-11-05) to proxy → new server (2025-06-18)
2. Connect new client (2025-06-18) to proxy → old server (2024-11-05)
3. Connect mixed versions, verify all tools work
4. Test feature detection and graceful degradation

### Compatibility Matrix
| Client Version | Server Version | Expected Behavior |
|----------------|----------------|-------------------|
| 2024-11-05 | 2024-11-05 | Pass-through |
| 2024-11-05 | 2025-06-18 | Downward translation |
| 2025-06-18 | 2024-11-05 | Upward translation |
| 2025-06-18 | 2025-06-18 | Pass-through |
| 2025-03-26 | Mixed | Translate per-server |

## Implementation Timeline

### Week 1: Foundation
- [ ] Add protocol_version and capabilities fields to ServerInfo
- [ ] Capture and store versions during initialization
- [ ] Add version comparison utilities
- [ ] Document version differences

### Week 2: Translation Layer
- [ ] Create VersionTranslator module
- [ ] Implement tool translation (up and down)
- [ ] Implement result translation
- [ ] Add feature detection

### Week 3: Integration
- [ ] Integrate translator into handler
- [ ] Add per-connection version tracking
- [ ] Implement smart version negotiation
- [ ] Add version mismatch logging

### Week 4: Testing & Polish
- [ ] Write unit tests for translation
- [ ] Integration tests with real servers
- [ ] Performance benchmarks
- [ ] Documentation and examples

## Benefits

### For Users
- **Seamless Compatibility**: Connect any client to any server
- **Future Proof**: New features automatically available
- **No Breaking Changes**: Old servers continue to work
- **Clear Errors**: Helpful messages when features unavailable

### For Proxy
- **Differentiation**: Only proxy with multi-version support
- **Flexibility**: Support old and new servers simultaneously
- **Longevity**: Won't break when new versions released
- **Intelligence**: Can expose features backends don't have

### For Ecosystem
- **Migration Path**: Smooth upgrade from old to new versions
- **Interoperability**: Mix and match server versions
- **Innovation**: Proxy can add features via translation
- **Stability**: No forced upgrades

## Success Criteria

- [ ] Support all versions from 2024-11-05 onwards
- [ ] Zero breaking changes for existing clients/servers
- [ ] < 5ms translation overhead per request
- [ ] 100% test coverage for version translation
- [ ] Clear documentation of supported features per version
- [ ] Graceful degradation when features unavailable

## Open Questions

1. Should we support "0.1.0" format or require migration to date format?
2. How to handle future versions we don't know about yet?
3. Should translation be opt-in or always-on?
4. What's the performance impact of per-request translation?
5. Should we cache translated tools or translate on-demand?

## References

- [MCP Specification 2024-11-05](https://modelcontextprotocol.info/specification/2024-11-05/)
- [MCP Specification 2025-03-26](https://modelcontextprotocol.io/specification/2025-03-26)
- [MCP Specification 2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18)
- [Version Negotiation RFC](https://modelcontextprotocol.info/specification/2024-11-05/basic/versioning/)
