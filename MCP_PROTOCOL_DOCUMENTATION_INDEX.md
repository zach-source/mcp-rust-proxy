# MCP Protocol Documentation Index

This directory contains comprehensive documentation for implementing Model Context Protocol (MCP) version compatibility and translation in the Rust MCP Proxy.

## Documentation Overview

### 📚 Core Reference Documents

#### 1. [MCP_VERSION_COMPARISON.md](./MCP_VERSION_COMPARISON.md) (23 KB)
**Complete technical specification comparison across all three MCP protocol versions.**

**Contents**:
- Detailed initialization sequence for each version
- Complete message format specifications
- Capability negotiation structures
- Request/response formats for all API endpoints
- Breaking changes summary
- Version negotiation rules
- Error handling patterns
- Content type evolution

**Use this when**: You need the authoritative, detailed specification for any aspect of the protocol across versions.

---

#### 2. [MCP_VERSION_DIFFERENCES_QUICK_REF.md](./MCP_VERSION_DIFFERENCES_QUICK_REF.md) (11 KB)
**Condensed quick-reference guide for developers implementing translation logic.**

**Contents**:
- Protocol version strings
- Breaking changes only (highlighted)
- New features by version (comparison table)
- Translation rules (forward/backward)
- Capability flags matrix
- Message format changes (side-by-side)
- Content type support matrix
- Quick decision tree

**Use this when**: You're actively coding and need quick answers about what changed between versions.

---

#### 3. [MCP_TRANSLATION_TEST_SPEC.md](./MCP_TRANSLATION_TEST_SPEC.md) (24 KB)
**Comprehensive test specification for validating the translation layer.**

**Contents**:
- 10 test suites with detailed test cases
- Test fixtures and sample code
- Version negotiation tests
- Initialize request/response tests
- Resource, tool, prompt API tests
- Content type conversion tests
- Edge cases and error handling
- Performance and stress tests
- Integration test scenarios
- Manual testing checklist

**Use this when**: Implementing or validating the translation layer, writing tests, or debugging translation issues.

---

### 🎯 Planning Documents

#### 4. [MCP_VERSION_COMPATIBILITY_PLAN.md](./MCP_VERSION_COMPATIBILITY_PLAN.md) (12 KB)
**Implementation roadmap for adding multi-version support to the proxy.**

**Contents**:
- Architecture design
- Implementation phases
- Module structure
- Data flow diagrams
- Configuration options
- Error handling strategy
- Testing approach
- Deployment considerations

**Use this when**: Planning the implementation work or reviewing architecture decisions.

---

#### 5. [MCP_PROTOCOL_COMPLIANCE_PLAN.md](./MCP_PROTOCOL_COMPLIANCE_PLAN.md) (7.8 KB)
**Plan for ensuring full MCP protocol compliance.**

**Contents**:
- Protocol requirements checklist
- Validation strategies
- Compliance testing
- Known limitations
- Remediation plan

**Use this when**: Verifying protocol compliance or planning compliance improvements.

---

#### 6. [MCP_ADVANCED_FEATURES_PLAN.md](./MCP_ADVANCED_FEATURES_PLAN.md) (13 KB)
**Roadmap for advanced proxy features beyond basic protocol support.**

**Contents**:
- Feature prioritization
- Load balancing strategies
- Caching mechanisms
- Monitoring and observability
- Security enhancements
- Performance optimizations

**Use this when**: Planning future enhancements beyond basic protocol support.

---

## Key Information by Task

### Task: Implement Version Detection
**Primary**: [MCP_VERSION_COMPARISON.md § Version Negotiation](#)
**Quick Ref**: [MCP_VERSION_DIFFERENCES_QUICK_REF.md § Version Negotiation](#)
**Tests**: [MCP_TRANSLATION_TEST_SPEC.md § Test Suite 1](#)

### Task: Handle ResourceContents.name (Breaking Change)
**Primary**: [MCP_VERSION_COMPARISON.md § Resources API → resources/read Response](#)
**Quick Ref**: [MCP_VERSION_DIFFERENCES_QUICK_REF.md § Breaking Changes Only](#)
**Tests**: [MCP_TRANSLATION_TEST_SPEC.md § Test Suite 3.2](#)

### Task: Translate Tool Definitions
**Primary**: [MCP_VERSION_COMPARISON.md § Tools API](#)
**Quick Ref**: [MCP_VERSION_DIFFERENCES_QUICK_REF.md § Message Format Changes → Tool Objects](#)
**Tests**: [MCP_TRANSLATION_TEST_SPEC.md § Test Suite 4](#)

### Task: Convert AudioContent
**Primary**: [MCP_VERSION_COMPARISON.md § Content Type Evolution](#)
**Quick Ref**: [MCP_VERSION_DIFFERENCES_QUICK_REF.md § Content Types Support](#)
**Tests**: [MCP_TRANSLATION_TEST_SPEC.md § Test Suite 5.1](#)

### Task: Filter Capabilities
**Primary**: [MCP_VERSION_COMPARISON.md § Capability Negotiation](#)
**Quick Ref**: [MCP_VERSION_DIFFERENCES_QUICK_REF.md § Capability Flags by Version](#)
**Tests**: [MCP_TRANSLATION_TEST_SPEC.md § Test Suite 2](#)

### Task: Handle Notifications
**Primary**: [MCP_VERSION_COMPARISON.md § Notifications](#)
**Quick Ref**: [MCP_VERSION_DIFFERENCES_QUICK_REF.md § Notifications](#)
**Tests**: [MCP_TRANSLATION_TEST_SPEC.md § Test Suite 7](#)

---

## Protocol Versions Summary

### 2024-11-05 (Initial Release)
- ✅ Base protocol with JSON-RPC 2.0
- ✅ Resources, Tools, Prompts APIs
- ✅ Client roots and sampling capabilities
- ✅ Server logging, prompts, resources, tools capabilities
- ✅ Text, Image, EmbeddedResource content types
- ✅ Core notifications (initialized, list_changed, progress)

### 2025-03-26 (First Update)
- ✨ Added `completions` server capability
- ✨ Added `AudioContent` content type
- ✨ Added `resources/updated` notification
- ✅ Fully backward compatible with 2024-11-05

### 2025-06-18 (Latest)
- ⚠️ **BREAKING**: `ResourceContents.name` now required
- ✨ Added `elicitation` client capability
- ✨ Added `title` fields to Tool, Resource, Prompt
- ✨ Added `outputSchema` to Tool definition
- ✨ Added `structuredContent` to CallToolResult
- ⚠️ Partial breaking changes from 2025-03-26

---

## Critical Breaking Changes

### Only One Breaking Change Across All Versions:
**2025-06-18**: `ResourceContents.name` is now required

**Impact**:
- `resources/read` responses must include a `name` field
- Older servers (2024-11-05, 2025-03-26) don't provide this field
- Translation layer must generate `name` from `uri` when upgrading

**Mitigation**:
```rust
fn generate_name_from_uri(uri: &str) -> String {
    uri.split('/').last()
       .unwrap_or(uri)
       .to_string()
}
```

---

## Translation Decision Matrix

| Scenario | Action | Document Reference |
|----------|--------|-------------------|
| Client = Server version | Pass through unchanged | Quick Ref § Quick Decision Tree |
| Forward: 2024 → 2025 | Add missing fields (name, title) | Comparison § Forward Translation |
| Backward: 2025 → 2024 | Strip unsupported fields | Comparison § Backward Translation |
| AudioContent to 2024 | Convert to TextContent | Test Spec § Test Suite 5.1 |
| Capabilities mismatch | Filter by target version | Comparison § Capability Negotiation |

---

## Implementation Checklist

### Phase 1: Foundation
- [ ] Read [MCP_VERSION_COMPARISON.md](./MCP_VERSION_COMPARISON.md) fully
- [ ] Review [MCP_VERSION_COMPATIBILITY_PLAN.md](./MCP_VERSION_COMPATIBILITY_PLAN.md) architecture
- [ ] Set up test environment from [MCP_TRANSLATION_TEST_SPEC.md](./MCP_TRANSLATION_TEST_SPEC.md)

### Phase 2: Core Translation
- [ ] Implement version detection (Test Suite 1)
- [ ] Implement pass-through for matching versions (Test Suite 1)
- [ ] Implement ResourceContents.name handling (Test Suite 3.2)
- [ ] Implement capability filtering (Test Suite 2)

### Phase 3: Content Translation
- [ ] Implement AudioContent conversion (Test Suite 5.1)
- [ ] Implement title field stripping/adding (Test Suite 3, 4, 6)
- [ ] Implement outputSchema/structuredContent handling (Test Suite 4)

### Phase 4: Integration
- [ ] Full initialize handshake test (Test Suite 10.1)
- [ ] tools/call round trip test (Test Suite 10.2)
- [ ] Notification handling (Test Suite 7)

### Phase 5: Edge Cases
- [ ] Malformed message handling (Test Suite 8)
- [ ] Unknown field preservation (Test Suite 8.4)
- [ ] Performance stress tests (Test Suite 9)

### Phase 6: Production
- [ ] Manual testing with real clients/servers
- [ ] Monitor translation performance
- [ ] Document any issues or deviations
- [ ] Update tests based on findings

---

## Common Questions

### Q: Are there any breaking changes between 2024-11-05 and 2025-03-26?
**A**: No, 2025-03-26 is fully backward compatible.

### Q: What's the main breaking change in 2025-06-18?
**A**: `ResourceContents.name` is now required. See [MCP_VERSION_DIFFERENCES_QUICK_REF.md § Breaking Changes](#).

### Q: How do I know which fields to strip when translating backward?
**A**: Use the translation rules in [MCP_VERSION_DIFFERENCES_QUICK_REF.md § Translation Rules](#).

### Q: Can I pass through messages if versions are different?
**A**: Only if the versions match exactly. Otherwise, translation is required. See [Quick Decision Tree](#).

### Q: What happens if I receive AudioContent but the target is 2024-11-05?
**A**: Convert it to TextContent with a description. See [MCP_TRANSLATION_TEST_SPEC.md § Test 5.1.1](#).

### Q: How do I generate the required `name` field for ResourceContents?
**A**: Extract the filename from the URI (last path component). See [MCP_TRANSLATION_TEST_SPEC.md § Test 3.2.2](#).

---

## Document Relationships

```
MCP_VERSION_COMPARISON.md (Authoritative Spec)
    ↓ Summarized in
MCP_VERSION_DIFFERENCES_QUICK_REF.md (Quick Ref)
    ↓ Tested by
MCP_TRANSLATION_TEST_SPEC.md (Test Spec)
    ↓ Guides implementation in
MCP_VERSION_COMPATIBILITY_PLAN.md (Implementation Plan)
    ↓ Ensures compliance via
MCP_PROTOCOL_COMPLIANCE_PLAN.md (Compliance Plan)
    ↓ Future features in
MCP_ADVANCED_FEATURES_PLAN.md (Feature Roadmap)
```

---

## Revision History

| Date | Version | Changes |
|------|---------|---------|
| 2025-10-12 | 1.0 | Initial documentation set created |

---

## Contributing

When updating these documents:

1. **Always update MCP_VERSION_COMPARISON.md first** (authoritative source)
2. Update quick reference and test spec to match
3. Keep version numbers and dates consistent
4. Add test cases for any new scenarios
5. Update this index if adding new documents

---

## External References

- Official MCP Specification: https://modelcontextprotocol.io/specification/
- MCP Schema Repository: https://github.com/modelcontextprotocol/specification
- JSON-RPC 2.0 Spec: https://www.jsonrpc.org/specification

---

## Quick Start

### New to MCP Protocol Translation?
1. Start with [MCP_VERSION_DIFFERENCES_QUICK_REF.md](./MCP_VERSION_DIFFERENCES_QUICK_REF.md)
2. Review the Breaking Changes section
3. Read the Quick Decision Tree
4. Refer to [MCP_VERSION_COMPARISON.md](./MCP_VERSION_COMPARISON.md) for details

### Implementing Translation Logic?
1. Read [MCP_VERSION_COMPATIBILITY_PLAN.md](./MCP_VERSION_COMPATIBILITY_PLAN.md) architecture
2. Use [MCP_VERSION_DIFFERENCES_QUICK_REF.md](./MCP_VERSION_DIFFERENCES_QUICK_REF.md) for coding
3. Write tests from [MCP_TRANSLATION_TEST_SPEC.md](./MCP_TRANSLATION_TEST_SPEC.md)
4. Validate against [MCP_VERSION_COMPARISON.md](./MCP_VERSION_COMPARISON.md)

### Debugging Issues?
1. Check [MCP_VERSION_DIFFERENCES_QUICK_REF.md § Common Pitfalls](#)
2. Review [MCP_TRANSLATION_TEST_SPEC.md § Test Suite 8 (Edge Cases)](#)
3. Consult [MCP_VERSION_COMPARISON.md § Error Handling](#)

### Planning Features?
1. Review [MCP_ADVANCED_FEATURES_PLAN.md](./MCP_ADVANCED_FEATURES_PLAN.md)
2. Check [MCP_PROTOCOL_COMPLIANCE_PLAN.md](./MCP_PROTOCOL_COMPLIANCE_PLAN.md) for gaps
3. Ensure compatibility with all protocol versions
