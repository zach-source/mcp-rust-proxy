# Implementation Plan: MCP Protocol Version Negotiation

**Branch**: `003-mcp-protocol-support` | **Date**: 2025-10-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-mcp-protocol-support/spec.md`

## Summary

Implement multi-version protocol support for the MCP Rust Proxy to enable seamless communication with backend MCP servers using different protocol versions (2024-11-05, 2025-03-26, and 2025-06-18). The proxy will automatically detect each server's protocol version during initialization and translate messages bidirectionally to ensure compatibility. This addresses the critical issue where 5 out of 9 backend servers crash due to protocol mismatches and incorrect initialization sequencing, resulting in only 42 tools available instead of the expected 76-86 tools.

**Technical Approach**: Implement the Adapter pattern with protocol version detection during the initialize handshake. Create bidirectional translation adapters for each version pair, with pass-through optimization for matching versions. Enforce strict initialization state machine (initialize → response → initialized → ready) to prevent premature requests that crash servers.

## Technical Context

**Language/Version**: Rust 1.75+

**Primary Dependencies**:
- tokio 1.40 (async runtime)
- serde 1.0, serde_json 1.0 (JSON serialization)
- async-trait 0.1 (trait async methods)
- tracing 0.1 (structured logging)
- dashmap 6.0 (concurrent state management)

**Storage**: In-memory only (protocol version state per connection, no persistence needed)

**Testing**: cargo test (unit tests), integration tests with mock MCP servers

**Target Platform**: Linux/macOS server (cross-platform Rust)

**Project Type**: Single backend service

**Performance Goals**:
- Protocol version detection: < 100ms per server
- Message translation: < 1ms P99 latency per message
- Pass-through (same version): Zero overhead (no-op)
- Support 9+ concurrent backend servers

**Constraints**:
- Must not break existing functionality
- Must support all transport types (stdio, HTTP-SSE, WebSocket)
- Initialization timeout: 60 seconds (configurable)
- Zero downtime deployment (stateless protocol negotiation)

**Scale/Scope**:
- 3 protocol versions supported initially
- 9+ backend MCP servers per proxy instance
- 76-86 total tools from all servers
- Expected message rate: 10-100 messages/second per server

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Note**: This project does not have a constitution file at `/Users/ztaylor/repos/workspaces/mcp-rust-proxy/main/.specify/constitution.md`. Using default guidelines from project's CLAUDE.md.

### Guidelines from CLAUDE.md

✅ **Simplicity**:
- Using established Adapter pattern (well-known design pattern)
- Clear separation: version detection, state management, translation
- No premature abstraction (only 3 versions to support)

✅ **Incremental Progress**:
- Can implement versions one at a time
- Each adapter is independently testable
- Feature can be enabled per-server (gradual rollout)

✅ **Learning from Existing Code**:
- Follows existing transport abstraction pattern (src/transport/)
- Reuses ServerState pattern (src/state/)
- Matches error handling conventions (thiserror)

✅ **Clear Intent**:
- ProtocolVersion enum makes versions explicit
- ProtocolAdapter trait documents contract
- State machine enforces initialization sequence

### Constitution Gates

Since no formal constitution exists, using pragmatic gates:

1. ✅ **Does this add new external dependencies?**
   - No new external dependencies (uses existing tokio, serde, etc.)

2. ✅ **Does this introduce new abstractions?**
   - Yes, but justified: ProtocolAdapter trait needed for polymorphic translation
   - ProtocolVersion enum needed to represent versions type-safely

3. ✅ **Does this follow project conventions?**
   - Yes: Matches existing module structure (src/protocol/)
   - Yes: Uses thiserror for errors
   - Yes: Uses tracing for logging
   - Yes: Uses async-trait for async traits

4. ✅ **Is this the simplest solution?**
   - Yes: Adapter pattern is well-understood and appropriate
   - Alternative (runtime reflection) would be more complex
   - Alternative (separate proxy instances) would waste resources

## Project Structure

### Documentation (this feature)

```
specs/003-mcp-protocol-support/
├── spec.md              # Feature specification (already exists)
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Research findings and decisions
├── data-model.md        # Core entities and state machines
├── quickstart.md        # Developer guide with examples
├── contracts/           # API and behavior contracts
│   ├── protocol-adapter-api.md
│   ├── version-detection.md
│   └── translation-rules.md
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created yet)
```

### Source Code (repository root)

```
src/
├── protocol/                    # NEW: Protocol version support
│   ├── mod.rs                   # Public API (ProtocolVersion, create_adapter)
│   ├── version.rs               # ProtocolVersion enum + feature detection
│   ├── adapter.rs               # ProtocolAdapter trait definition
│   ├── adapters/                # Concrete adapter implementations
│   │   ├── mod.rs
│   │   ├── pass_through.rs      # PassThroughAdapter (same version)
│   │   ├── v20241105_to_v20250326.rs
│   │   ├── v20241105_to_v20250618.rs
│   │   ├── v20250326_to_v20241105.rs
│   │   ├── v20250326_to_v20250618.rs
│   │   ├── v20250618_to_v20241105.rs
│   │   └── v20250618_to_v20250326.rs
│   ├── state.rs                 # ServerConnectionState (state machine)
│   ├── handshake.rs             # InitializationHandshake tracking
│   ├── translation/             # Translation helper functions
│   │   ├── mod.rs
│   │   ├── tools.rs             # Tool translation helpers
│   │   ├── resources.rs         # Resource translation helpers
│   │   ├── prompts.rs           # Prompt translation helpers
│   │   └── content.rs           # Content type translation
│   └── error.rs                 # ProtocolError types
├── transport/                   # EXISTING: Transport layer
│   ├── mod.rs                   # MODIFIED: Use ServerConnectionState
│   ├── stdio.rs                 # MODIFIED: Protocol negotiation
│   ├── http_sse.rs              # MODIFIED: Protocol negotiation
│   └── websocket.rs             # MODIFIED: Protocol negotiation
├── proxy/                       # EXISTING: Proxy request handlers
│   ├── handler.rs               # MODIFIED: Use adapters for translation
│   ├── router.rs                # MODIFIED: Check server ready state
│   └── ...
├── state/                       # EXISTING: Shared state
│   └── mod.rs                   # MODIFIED: Store protocol version per server
└── types/                       # EXISTING: Type definitions
    ├── jsonrpc.rs               # EXISTING: JSON-RPC message types
    └── mcp/                     # NEW: Version-specific MCP types
        ├── mod.rs
        ├── common.rs            # Shared types across versions
        ├── v20241105.rs         # 2024-11-05 specific types
        ├── v20250326.rs         # 2025-03-26 specific types
        └── v20250618.rs         # 2025-06-18 specific types

tests/
├── unit/                        # Unit tests
│   ├── protocol/
│   │   ├── version_tests.rs     # ProtocolVersion tests
│   │   ├── adapter_tests.rs     # Individual adapter tests
│   │   └── state_tests.rs       # State machine tests
│   └── translation/
│       ├── tools_tests.rs
│       ├── resources_tests.rs
│       └── content_tests.rs
├── integration/                 # Integration tests
│   ├── initialization_tests.rs  # Full init sequence
│   ├── version_negotiation_tests.rs
│   └── multi_version_tests.rs   # Multiple servers with different versions
└── compliance/                  # Protocol compliance tests
    ├── v20241105_compliance.rs
    ├── v20250326_compliance.rs
    └── v20250618_compliance.rs
```

**Structure Decision**: Using **Single Project** structure. This is a backend server with modular architecture. The new `protocol/` module encapsulates all version-related logic and integrates cleanly with existing `transport/` and `proxy/` modules. This follows the existing project convention of feature-based modules (e.g., `context/`, `transport/`, `proxy/`).

**Key Integration Points**:
1. **transport layer**: Handles initialize handshake, detects version, creates adapter
2. **proxy handler**: Uses adapter to translate requests/responses
3. **state management**: Stores protocol version and adapter per server connection
4. **error handling**: New ProtocolError type integrates with existing error types

## Complexity Tracking

*No constitution violations - all design choices follow project conventions and are justified by requirements.*
