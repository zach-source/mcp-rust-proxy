# MCP Protocol Compliance Plan

## Executive Summary
Ensure MCP Rust Proxy fully complies with the Model Context Protocol (MCP) specification version **2025-03-26** (latest stable) and **2025-06-18** (current).

## Current Status

### ✅ What We Have Correct
1. **Protocol Version**: Updated to `2025-03-26` in initialization (src/transport/pool.rs:53)
2. **Initialize Handshake**: Properly implements `initialize` request + `initialized` notification
3. **Tool Aggregation**: Fixed `list_tools()` to aggregate from backend servers
4. **Method Support**: Handles both old (`"list"`) and new (`"tools/list"`) method formats
5. **Tool Prefixing**: Properly prefixes backend tools with `mcp__proxy__{server}__`

### ⚠️ Potential Compliance Issues

## MCP Specification 2025-03-26 / 2025-06-18

### Core JSON-RPC Methods

#### 1. Lifecycle Methods
- **initialize** (Request) ✅
  - Client sends: `{method: "initialize", params: {protocolVersion, capabilities, clientInfo}}`
  - Server responds with: `{protocolVersion, capabilities, serverInfo, instructions?}`
  - **Action**: Verify our initialize response format matches spec

- **initialized** (Notification) ✅
  - Client sends after receiving initialize response
  - No response expected
  - **Status**: Implemented in pool.rs:99-111

#### 2. Tools Methods
- **tools/list** (Request) ✅
  - Returns: `{tools: [{name, description, inputSchema}]}`
  - **Status**: Implemented and fixed

- **tools/call** (Request) ✅
  - Params: `{name, arguments}`
  - Returns: `{content: [...], isError?}`
  - **Action**: Verify response format compliance

#### 3. Resources Methods
- **resources/list** (Request) ✅
  - Returns: `{resources: [{uri, name, description?, mimeType?}]}`
  - **Status**: Implemented

- **resources/read** (Request) ✅
  - Params: `{uri}`
  - Returns: `{contents: [{uri, mimeType?, text?, blob?}]}`
  - **Action**: Verify we support both `read` and `resources/read`

- **resources/templates/list** (Request) ❓
  - **Status**: NOT IMPLEMENTED
  - **Action**: Check if this is in 2025-06-18 spec

- **resources/subscribe** (Request) ❓
  - **Status**: NOT IMPLEMENTED
  - **Action**: Implement if capability declared

- **resources/unsubscribe** (Request) ❓
  - **Status**: NOT IMPLEMENTED
  - **Action**: Implement if capability declared

#### 4. Prompts Methods
- **prompts/list** (Request) ✅
  - Returns: `{prompts: [{name, description?, arguments?}]}`
  - **Status**: Implemented

- **prompts/get** (Request) ❓
  - Params: `{name, arguments?}`
  - Returns: `{description?, messages: [...]}`
  - **Status**: NOT IMPLEMENTED
  - **Action**: Add forwarding support

#### 5. Logging Methods (Server → Client)
- **logging/setLevel** (Request) ❓
  - **Status**: NOT IMPLEMENTED
  - **Action**: Review if proxy should support this

#### 6. Completion Methods
- **completion/complete** (Request) ❓
  - For argument autocompletion
  - **Status**: NOT IMPLEMENTED (2025-03-26 feature)
  - **Action**: Check if needed for proxy

#### 7. Ping Method
- **ping** (Request) ✅
  - Returns: `{}`
  - **Status**: Implemented in handler.rs:119

### Capabilities Declaration

#### During Initialize, Server Must Declare:

```json
{
  "capabilities": {
    "tools": { "listChanged"?: boolean },
    "resources": {
      "subscribe"?: boolean,
      "listChanged"?: boolean
    },
    "prompts": { "listChanged"?: boolean },
    "logging": {},
    "experimental": {}
  }
}
```

**Action Items:**
1. **Review Initialize Response**: Check what capabilities we currently declare
2. **Match Capabilities to Implementation**: Only declare what we actually support
3. **Add Missing Capabilities**: Implement or declare as unsupported

## Implementation Tasks

### High Priority

#### Task 1: Audit Initialize Response
- **File**: `src/transport/pool.rs:48-60`
- **Action**:
  - Verify we return `serverInfo` with `name` and `version`
  - Check capabilities declaration matches our support
  - Test with MCP spec validator if available

#### Task 2: Verify tools/call Response Format
- **File**: `src/proxy/handler.rs:297-389`
- **Current**: Returns `result` directly from backend
- **Spec**: Should return `{content: [...], isError?: boolean}`
- **Action**: Check if backend servers already return correct format, or if we need to transform

#### Task 3: Add Missing Resource Methods
- **Files**:
  - `src/proxy/handler.rs` (add method handlers)
  - `src/proxy/mod.rs` (add types if needed)
- **Methods to Add**:
  - `resources/templates/list`
  - `resources/subscribe` (if we declare capability)
  - `resources/unsubscribe` (if we declare capability)

#### Task 4: Add Missing Prompt Methods
- **File**: `src/proxy/handler.rs`
- **Method**: `prompts/get`
- **Action**: Forward to backend servers that support it

### Medium Priority

#### Task 5: Protocol Version Negotiation
- **Current**: We send `2025-03-26`, accept whatever server returns
- **Spec**: If server doesn't support our version, it responds with one it supports
- **Action**: Handle version mismatch gracefully, log warnings

#### Task 6: JSON-RPC Batching Support
- **Status**: NOT IMPLEMENTED
- **Spec**: 2025-03-26 added batching, 2025-06-18 removed it
- **Action**: Skip (removed in latest spec)

#### Task 7: Notifications Support
- **servers can send**: `notifications/resources/updated`, `notifications/resources/list_changed`
- **Action**: Review if proxy should forward these from backends to clients

### Low Priority

#### Task 8: OAuth 2.1 Authorization
- **Spec**: 2025-06-18 added comprehensive OAuth support
- **Status**: NOT IMPLEMENTED
- **Action**: Future enhancement for secure deployments

#### Task 9: Tool Annotations
- **Spec**: 2025-03-26 added tool behavior annotations (readOnly, destructive, etc.)
- **Status**: PASS-THROUGH (backends provide this)
- **Action**: Verify we don't strip these fields when aggregating

#### Task 10: Audio Data Support
- **Spec**: 2025-03-26 added audio support in tool responses
- **Status**: PASS-THROUGH
- **Action**: Verify binary data handling

## Testing Plan

### Unit Tests
1. Test initialize with various protocol versions
2. Test all supported methods return correct JSON-RPC format
3. Test method routing (old vs new format)
4. Test error responses match spec

### Integration Tests
1. Test with official MCP SDK clients
2. Test with Claude Code (stdio mode)
3. Test with various backend servers
4. Test capability negotiation

### Compliance Testing
1. Use MCP specification test suite (if available)
2. Validate JSON-RPC 2.0 compliance
3. Test edge cases (missing params, invalid methods, etc.)

## Success Criteria

- [x] All core methods from 2025-03-26 spec supported
- [x] Initialize response matches spec format (returns protocolVersion, capabilities, serverInfo)
- [x] Tools/call response format verified (backends return {content, isError})
- [x] HTTP endpoint works with all backend tools (103 tools aggregated)
- [x] Task 1: Initialize response handler added
- [x] Task 2: Tools/call format verified correct
- [x] Task 3: resources/templates/list method added
- [x] Task 4: prompts/get method added
- [ ] All tests passing (pending)
- [ ] Documentation updated (pending)
- [ ] Claude Code (stdio) works with all backend tools (pending validation)

## Timeline

- **Week 1**: Audit current implementation, document gaps
- **Week 2**: Fix high-priority issues (initialize, tools/call)
- **Week 3**: Add missing resource/prompt methods
- **Week 4**: Testing and documentation

## References

- [MCP Specification 2025-03-26](https://modelcontextprotocol.io/specification/2025-03-26)
- [MCP Specification 2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [MCP GitHub Repository](https://github.com/modelcontextprotocol/modelcontextprotocol)
