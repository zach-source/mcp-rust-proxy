# Implementation Plan: Claude API Proxy for Context Tracing

**Branch**: `005-claude-api-proxy` | **Date**: 2025-10-28 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/005-claude-api-proxy/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Build a transparent HTTPS proxy that intercepts Claude API traffic, captures complete request/response data with context source attribution, and provides query/feedback mechanisms for improving Claude Code context composition. The proxy passes authentication unchanged and adds <100ms latency while maintaining security.

## Technical Context

**Language/Version**: Rust 1.75+ (leveraging existing MCP Rust Proxy codebase)
**Primary Dependencies**: NEEDS CLARIFICATION - Requires research on Rust HTTPS proxy libraries (hyper, reqwest, or specialized proxy crates)
**TLS/SSL Handling**: NEEDS CLARIFICATION - Certificate generation/trust approach for HTTPS interception
**Storage**: Existing SQLite + DashMap (leverage context tracing framework from MCP Rust Proxy)
**Testing**: cargo test, integration tests with mock Claude API endpoints
**Target Platform**: Local machine (macOS, Linux, Windows) - same environment as Claude CLI
**Project Type**: Single project (extends existing MCP Rust Proxy codebase)
**Performance Goals**: <100ms proxy latency, 100 concurrent requests, 1ms P99 for capture operations
**Constraints**: <200ms p95 end-to-end latency, transparent operation (no CLI changes), fail-open on proxy errors
**Scale/Scope**: 10,000 captured requests, 30-day retention, support for multiple concurrent Claude CLI sessions

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Performance First ✅
- **Requirement**: Arc<DashMap> for state, async with tokio, <100ms latency
- **Compliance**: Proxy designed for <100ms overhead (FR-007), async capture, leverage existing Arc<DashMap> in context storage
- **Evidence**: Success Criteria SC-007 mandates <100ms latency target

### Principle II: Flexibility Over Rigidity ✅
- **Requirement**: Configuration-driven, support multiple transports, extensibility
- **Compliance**: FR-022 enables/disables via config, FR-014 supports different context source types
- **Evidence**: Proxy works with existing Claude CLI without modification (FR-006)

### Principle III: Comprehensive Testing ✅
- **Requirement**: Unit tests, integration tests, protocol compliance tests
- **Compliance**: Will include unit tests for capture logic, integration tests with mock Claude API, edge case handling
- **Evidence**: Each user story has testable acceptance criteria, edge cases documented

### Principle IV: Idiomatic Rust Patterns ✅
- **Requirement**: Result<T, Error>, enums for state, traits for polymorphism
- **Compliance**: Will use Result for all operations, enums for ConnectionState, traits for ProxyHandler
- **Evidence**: Extends existing Rust codebase following established patterns

### Principle V: Structured Logging and Observability ✅
- **Requirement**: Use tracing crate, structured fields, expose metrics
- **Compliance**: FR-011 records detailed timestamps, FR-020 provides query API, will use tracing with request_id/source fields
- **Evidence**: Follows existing logging patterns in MCP Rust Proxy

### Principle VI: Backward Compatibility ✅
- **Requirement**: Support multiple versions, graceful degradation
- **Compliance**: FR-022 allows enabling/disabling without breaking existing functionality, FR-021 fail-open behavior
- **Evidence**: No changes to Claude CLI required (FR-006), proxy is opt-in

### Principle VII: Leverage Context7 and Serena ✅
- **Requirement**: Use MCP tools for development
- **Compliance**: Will use Context7 for researching HTTPS proxy libraries, Serena for navigating codebase
- **Evidence**: Phase 0 research will document library choices using Context7

**GATE STATUS**: ✅ PASSED - All principles satisfied, no violations to justify

## Project Structure

### Documentation (this feature)

```
specs/005-claude-api-proxy/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   ├── query-api.yaml   # OpenAPI spec for captured data query API
│   └── feedback-api.yaml # OpenAPI spec for quality feedback API
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

**Structure Decision**: Extends existing MCP Rust Proxy single project structure. New code will integrate into established patterns.

```
src/
├── claude_proxy/         # NEW - Claude API proxy module
│   ├── mod.rs           # Module definition, public exports
│   ├── proxy_server.rs  # HTTPS proxy server implementation
│   ├── tls_handler.rs   # TLS/SSL certificate handling
│   ├── capture.rs       # Request/response capture logic
│   ├── attribution.rs   # Context source attribution
│   └── config.rs        # Proxy-specific configuration
├── context/             # EXISTING - Extend for Claude API capture
│   ├── storage.rs       # Add captured request/response storage
│   ├── models.rs        # Add CapturedRequest, CapturedResponse entities
│   └── feedback.rs      # NEW - Quality feedback handling
├── web/                 # EXISTING - Add new API endpoints
│   └── api.rs           # Add /api/claude/requests, /api/claude/feedback routes
├── config/              # EXISTING - Extend schema
│   └── schema.rs        # Add ClaudeProxyConfig struct
└── main.rs              # EXISTING - Add Claude proxy startup

tests/
├── integration/
│   └── claude_proxy_tests.rs  # End-to-end proxy tests with mock Claude API
└── unit/
    └── claude_proxy/
        ├── capture_tests.rs
        ├── attribution_tests.rs
        └── tls_tests.rs
```

**Integration Points**:
- Leverage existing `src/context/` for storage (SQLite + DashMap pattern)
- Extend `src/web/api.rs` for new query/feedback endpoints
- Use existing `src/config/` patterns for proxy configuration
- Follow existing `src/transport/` patterns for connection handling

## Complexity Tracking

*No violations - Constitution Check passed without exceptions*
