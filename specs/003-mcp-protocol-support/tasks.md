# Implementation Tasks: MCP Protocol Version Negotiation

**Feature**: MCP Protocol Version Negotiation and Conversion Layer
**Branch**: `003-mcp-protocol-support`
**Date**: 2025-10-12
**Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | **Data Model**: [data-model.md](data-model.md)

---

## Overview

This task list implements multi-version MCP protocol support to enable the proxy to communicate with backend servers using different protocol versions (2024-11-05, 2025-03-26, 2025-06-18). Tasks are organized into 7 phases following TDD principles.

**Key Principles**:
- ✅ Tests written BEFORE implementation (TDD required)
- ✅ Each commit must compile and pass existing tests
- ✅ [P] indicates tasks that can be parallelized
- ✅ Dependencies clearly marked for each task

---

## Phase 1: Setup & Project Initialization

**Goal**: Prepare branch, create directory structure, and establish testing infrastructure.

### T001 [Setup] Create feature branch and initial structure
- **File**: N/A (Git operations)
- **Action**: Create branch `003-mcp-protocol-support` from `main`, create directory structure
- **Commands**:
  ```bash
  git checkout main
  git pull origin main
  git checkout -b 003-mcp-protocol-support
  mkdir -p src/protocol/adapters
  mkdir -p src/protocol/translation
  mkdir -p src/types/mcp
  mkdir -p tests/unit/protocol
  mkdir -p tests/unit/translation
  mkdir -p tests/integration
  mkdir -p tests/compliance
  ```
- **Dependencies**: None
- **Success**: Branch created, directories exist

### T002 [Setup] Create error type definitions
- **File**: `src/protocol/error.rs`
- **Action**: Define `ProtocolError` enum with all error variants from data-model.md
- **Details**: Use `thiserror` for error types, include all variants (UnsupportedVersion, TranslationError, MissingRequiredField, InitializationTimeout, InvalidStateTransition, JsonError, IoError)
- **Dependencies**: T001
- **Success**: Error types compile, derived traits work correctly

### T003 [Setup] Create protocol module scaffolding
- **File**: `src/protocol/mod.rs`
- **Action**: Create public module interface with re-exports for version, adapter, state, handshake, error
- **Dependencies**: T002
- **Success**: Module compiles, imports work from parent modules

### T004 [Setup] Update Cargo.toml dependencies
- **File**: `Cargo.toml`
- **Action**: Verify required dependencies (tokio, serde, serde_json, async-trait, tracing, dashmap, thiserror)
- **Dependencies**: None (can run in parallel)
- **Success**: `cargo build` succeeds with all dependencies

---

## Phase 2: Foundation - Core Types & Infrastructure

**Goal**: Implement foundational types (ProtocolVersion, base traits) with comprehensive tests.

### T005 [Foundation] [TEST] Write ProtocolVersion enum tests
- **File**: `tests/unit/protocol/version_tests.rs`
- **Action**: Write tests for ProtocolVersion::from_string(), as_str(), and all feature detection methods (supports_audio_content, supports_completions, etc.)
- **Test Cases**:
  - Parse valid version strings ("2024-11-05", "2025-03-26", "2025-06-18")
  - Reject invalid version strings (return UnsupportedVersion error)
  - Feature detection returns correct values for each version
  - Round-trip conversion (from_string → as_str → from_string)
- **Dependencies**: T003
- **Success**: Tests compile but fail (no implementation yet)

### T006 [Foundation] Implement ProtocolVersion enum
- **File**: `src/protocol/version.rs`
- **Action**: Implement ProtocolVersion enum with all methods from data-model.md
- **Details**: V20241105, V20250326, V20250618 variants with from_string(), as_str(), and 7 feature detection methods
- **Dependencies**: T005 (test must exist first)
- **Success**: All tests from T005 pass

### T007 [Foundation] [TEST] Write ProtocolAdapter trait tests
- **File**: `tests/unit/protocol/adapter_tests.rs`
- **Action**: Write test scaffolding for ProtocolAdapter trait (mock adapter for testing)
- **Test Cases**:
  - Adapter correctly reports source/target versions
  - translate_request() preserves JSON-RPC structure
  - translate_response() preserves JSON-RPC structure
  - translate_notification() preserves JSON-RPC structure
- **Dependencies**: T003
- **Success**: Test scaffolding compiles (uses mock adapter)

### T008 [Foundation] Define ProtocolAdapter trait
- **File**: `src/protocol/adapter.rs`
- **Action**: Define ProtocolAdapter trait with async methods (source_version, target_version, translate_request, translate_response, translate_notification)
- **Dependencies**: T002, T006
- **Success**: Trait compiles, can be used as trait object

### T009 [Foundation] [TEST] Write PassThroughAdapter tests
- **File**: `tests/unit/protocol/adapter_tests.rs` (add section)
- **Action**: Write tests for PassThroughAdapter (zero-copy, no translation)
- **Test Cases**:
  - Request passes through unchanged (same reference)
  - Response passes through unchanged
  - Notification passes through unchanged
  - Performance benchmark (< 50μs overhead)
- **Dependencies**: T007, T008
- **Success**: Tests compile but fail

### T010 [Foundation] Implement PassThroughAdapter
- **File**: `src/protocol/adapters/pass_through.rs`
- **Action**: Implement PassThroughAdapter for same-version scenarios (zero-copy optimization)
- **Dependencies**: T009
- **Success**: All PassThroughAdapter tests pass

### T011 [Foundation] Create adapter module structure
- **File**: `src/protocol/adapters/mod.rs`
- **Action**: Define adapter module, export PassThroughAdapter, add create_adapter() factory function
- **Dependencies**: T010
- **Success**: Factory function compiles, returns correct adapter types

---

## Phase 3: User Story 1 (P1) - Version Detection & Connection

**Goal**: Implement version negotiation during initialization handshake.

### T012 [US1] [TEST] Write InitializeRequest/Response type tests
- **File**: `tests/unit/protocol/handshake_tests.rs`
- **Action**: Write tests for initialize message parsing and serialization
- **Test Cases**:
  - Deserialize valid InitializeResponse with protocolVersion
  - Extract protocol version from response
  - Handle missing protocolVersion field (error)
  - Handle malformed response (error)
  - Serialize InitializedNotification correctly
- **Dependencies**: T003
- **Success**: Tests compile but fail

### T013 [US1] Define initialization handshake types
- **File**: `src/protocol/handshake.rs`
- **Action**: Define InitializeRequest, InitializeParams, InitializeResponse, InitializeResult, InitializedNotification structs
- **Dependencies**: T012
- **Success**: All handshake type tests pass

### T014 [US1] [TEST] Write ConnectionState state machine tests
- **File**: `tests/unit/protocol/state_tests.rs`
- **Action**: Write tests for ServerConnectionState state transitions
- **Test Cases**:
  - Valid transitions: Connecting → Initializing → SendingInitialized → Ready
  - Invalid transitions return InvalidStateTransition error
  - Protocol version set during received_initialize_response()
  - can_send_request() returns correct values for each state
  - is_ready() returns true only in Ready state
- **Dependencies**: T003
- **Success**: Tests compile but fail

### T015 [US1] Implement ServerConnectionState
- **File**: `src/protocol/state.rs`
- **Action**: Implement ServerConnectionState with state machine logic from data-model.md
- **Details**: ConnectionState enum, state transitions, adapter storage, concurrent access via Arc<Mutex>
- **Dependencies**: T014, T006, T008
- **Success**: All state machine tests pass

### T016 [US1] [TEST] Write InitializationHandshakeTracker tests
- **File**: `tests/unit/protocol/handshake_tests.rs` (add section)
- **Action**: Write tests for handshake timing and timeout detection
- **Test Cases**:
  - Track each handshake phase timing
  - Detect timeout when elapsed > configured timeout
  - Calculate phase durations correctly
  - total_duration() returns correct value
- **Dependencies**: T013
- **Success**: Tests compile but fail

### T017 [US1] Implement InitializationHandshakeTracker
- **File**: `src/protocol/handshake.rs` (add to existing)
- **Action**: Implement handshake timing tracker with mark_*() methods and timeout detection
- **Dependencies**: T016
- **Success**: All handshake tracker tests pass

### T018 [US1] [TEST] Write transport layer integration tests
- **File**: `tests/integration/initialization_tests.rs`
- **Action**: Write integration tests for full initialization sequence
- **Test Cases**:
  - Send initialize request to mock server
  - Receive response with protocol version
  - Send initialized notification
  - State transitions to Ready
  - Subsequent requests allowed after Ready
  - Timeout when server doesn't respond within 60s
- **Dependencies**: T015, T017
- **Success**: Tests compile with mock server setup

### T019 [US1] [P] Integrate ServerConnectionState into HTTP-SSE transport
- **File**: `src/transport/http_sse.rs`
- **Action**: Add ServerConnectionState to HTTP-SSE transport, implement initialization sequence
- **Details**: Create state on connection, send initialize, wait for response, extract version, send initialized notification
- **Dependencies**: T015, T018
- **Success**: HTTP-SSE transport completes initialization, tests pass

### T020 [US1] [P] Integrate ServerConnectionState into WebSocket transport
- **File**: `src/transport/websocket.rs`
- **Action**: Add ServerConnectionState to WebSocket transport, implement initialization sequence
- **Details**: Same as T019 but for WebSocket connections
- **Dependencies**: T015, T018
- **Success**: WebSocket transport completes initialization, tests pass

### T021 [US1] Update shared state management
- **File**: `src/state/mod.rs`
- **Action**: Add protocol_version and connection_state fields to server state storage (DashMap)
- **Dependencies**: T015
- **Success**: State stores and retrieves protocol version per server

### T022 [US1] [TEST] Write multi-server version negotiation tests
- **File**: `tests/integration/version_negotiation_tests.rs`
- **Action**: Write tests for multiple servers with different protocol versions
- **Test Cases**:
  - 3 mock servers with different versions (2024-11-05, 2025-03-26, 2025-06-18)
  - All servers initialize successfully
  - Each server's protocol version stored correctly
  - All servers reach Ready state
  - No cross-contamination of versions
- **Dependencies**: T019, T020, T021
- **Success**: Tests compile with mock servers

### T023 [US1] Verify US1 acceptance criteria
- **File**: `tests/integration/version_negotiation_tests.rs` (add assertions)
- **Action**: Ensure all US1 acceptance scenarios pass
- **Dependencies**: T022
- **Success**: All 4 US1 acceptance scenarios verified

---

## Phase 4: User Story 2 (P1) - Reliable Initialization Sequence

**Goal**: Enforce initialization state machine to prevent premature requests.

### T024 [US2] [TEST] Write request gating tests
- **File**: `tests/integration/initialization_tests.rs` (add section)
- **Action**: Write tests for request blocking before initialization completes
- **Test Cases**:
  - tools/list request blocked when state is Connecting
  - tools/list request blocked when state is Initializing
  - tools/list request blocked when state is SendingInitialized
  - tools/list request allowed when state is Ready
  - initialize request only allowed in Connecting state
- **Dependencies**: T018
- **Success**: Tests compile but fail

### T025 [US2] Implement request gating in proxy handler
- **File**: `src/proxy/handler.rs`
- **Action**: Add state checks before forwarding requests to backend servers
- **Details**: Check ServerConnectionState.can_send_request(method) before forwarding, return error or queue if not ready
- **Dependencies**: T024, T021
- **Success**: Request gating tests pass

### T026 [US2] [TEST] Write slow initialization tests
- **File**: `tests/integration/initialization_tests.rs` (add section)
- **Action**: Write tests for servers that take 30+ seconds to initialize
- **Test Cases**:
  - Mock server delays initialize response by 30 seconds
  - Proxy waits patiently without timeout (within 60s limit)
  - Requests queued during initialization are processed after Ready
  - Timeout occurs if server takes > 60 seconds
- **Dependencies**: T024
- **Success**: Tests compile with delayed mock server

### T027 [US2] Implement request queuing during initialization
- **File**: `src/proxy/router.rs`
- **Action**: Add request queue per server, hold requests until Ready state
- **Details**: Queue requests when server not Ready, process queue on state transition to Ready, respect timeout
- **Dependencies**: T026
- **Success**: Slow initialization tests pass

### T028 [US2] [TEST] Write concurrent client tests
- **File**: `tests/integration/initialization_tests.rs` (add section)
- **Action**: Write tests for multiple clients connecting simultaneously
- **Test Cases**:
  - 10 clients send tools/list simultaneously
  - Backend servers still initializing
  - All requests queued correctly
  - All requests processed after initialization
  - No race conditions or deadlocks
- **Dependencies**: T026
- **Success**: Tests compile with multi-client setup

### T029 [US2] Verify concurrent safety
- **File**: `src/proxy/router.rs` (add locks/synchronization)
- **Action**: Ensure thread-safe request queuing and state transitions
- **Details**: Use Arc<DashMap> for concurrent state access, verify no data races with Miri or loom
- **Dependencies**: T028
- **Success**: Concurrent client tests pass, no data races

### T030 [US2] Verify US2 acceptance criteria
- **File**: `tests/integration/initialization_tests.rs` (add assertions)
- **Action**: Ensure all US2 acceptance scenarios pass
- **Dependencies**: T029
- **Success**: All 4 US2 acceptance scenarios verified

---

## Phase 5: User Story 3 (P2) - Protocol Translation

**Goal**: Implement bidirectional message translation between protocol versions.

### T031 [US3] Define version-specific MCP types
- **File**: `src/types/mcp/v20241105.rs`, `src/types/mcp/v20250326.rs`, `src/types/mcp/v20250618.rs`
- **Action**: Define version-specific structs (ResourceContentsV1/V2, ToolV1/V2, ContentV1/V2, CallToolResult variants)
- **Dependencies**: T006
- **Success**: All type definitions compile

### T032 [US3] Define common MCP types
- **File**: `src/types/mcp/common.rs`
- **Action**: Define shared types across all versions (Implementation, Capabilities, etc.)
- **Dependencies**: T031
- **Success**: Common types compile, used by version-specific types

### T033 [US3] [TEST] Write resource translation tests
- **File**: `tests/unit/translation/resources_tests.rs`
- **Action**: Write tests for ResourceContents translation between versions
- **Test Cases**:
  - V1 → V2: generate name from URI
  - V2 → V1: strip name and title fields
  - Round-trip: V1 → V2 → V1 preserves data
  - Handle missing optional fields
  - Handle special URI characters in name generation
- **Dependencies**: T031
- **Success**: Tests compile but fail

### T034 [US3] Implement resource translation helpers
- **File**: `src/protocol/translation/resources.rs`
- **Action**: Implement ResourceContents conversion (from_v1, to_v1, generate_resource_name)
- **Dependencies**: T033
- **Success**: Resource translation tests pass

### T035 [US3] [TEST] Write tool translation tests
- **File**: `tests/unit/translation/tools_tests.rs`
- **Action**: Write tests for Tool translation between versions
- **Test Cases**:
  - V1 → V2: preserve name, description, inputSchema
  - V2 → V1: strip title and outputSchema
  - Round-trip preserves data
  - Handle optional title field
  - Handle optional outputSchema field
- **Dependencies**: T031
- **Success**: Tests compile but fail

### T036 [US3] Implement tool translation helpers
- **File**: `src/protocol/translation/tools.rs`
- **Action**: Implement Tool conversion (from_v1, to_v1, from_v2, to_v2)
- **Dependencies**: T035
- **Success**: Tool translation tests pass

### T037 [US3] [TEST] Write content type translation tests
- **File**: `tests/unit/translation/content_tests.rs`
- **Action**: Write tests for Content type translation
- **Test Cases**:
  - Text content passes through unchanged
  - Image content passes through unchanged
  - Audio content (V2) → Text description (V1)
  - Resource content uses ResourceContents translation
  - CallToolResult with structuredContent handling
- **Dependencies**: T031, T034
- **Success**: Tests compile but fail

### T038 [US3] Implement content type translation helpers
- **File**: `src/protocol/translation/content.rs`
- **Action**: Implement Content conversion (ContentV1 ↔ ContentV2, handle Audio fallback)
- **Dependencies**: T037
- **Success**: Content translation tests pass

### T039 [US3] [TEST] Write V20241105 ↔ V20250618 adapter tests
- **File**: `tests/unit/protocol/adapter_v20241105_v20250618_tests.rs`
- **Action**: Write comprehensive tests for version pair adapters
- **Test Cases**:
  - tools/list request/response translation
  - resources/read request/response translation
  - tools/call request/response translation
  - Preserve JSON-RPC structure (id, jsonrpc, method)
  - Handle missing fields gracefully
  - Performance benchmark (< 1ms P99)
- **Dependencies**: T034, T036, T038
- **Success**: Tests compile but fail

### T040 [US3] Implement V20241105ToV20250618Adapter
- **File**: `src/protocol/adapters/v20241105_to_v20250618.rs`
- **Action**: Implement adapter using translation helpers
- **Dependencies**: T039
- **Success**: Forward direction tests pass

### T041 [US3] Implement V20250618ToV20241105Adapter
- **File**: `src/protocol/adapters/v20250618_to_v20241105.rs`
- **Action**: Implement reverse adapter
- **Dependencies**: T039
- **Success**: Reverse direction tests pass

### T042 [US3] [TEST] [P] Write V20241105 ↔ V20250326 adapter tests
- **File**: `tests/unit/protocol/adapter_v20241105_v20250326_tests.rs`
- **Action**: Write tests for this version pair (similar to T039)
- **Dependencies**: T034, T036, T038
- **Success**: Tests compile but fail

### T043 [US3] [P] Implement V20241105 ↔ V20250326 adapters
- **File**: `src/protocol/adapters/v20241105_to_v20250326.rs`, `src/protocol/adapters/v20250326_to_v20241105.rs`
- **Action**: Implement both directions
- **Dependencies**: T042
- **Success**: Both adapter tests pass

### T044 [US3] [TEST] [P] Write V20250326 ↔ V20250618 adapter tests
- **File**: `tests/unit/protocol/adapter_v20250326_v20250618_tests.rs`
- **Action**: Write tests for this version pair
- **Dependencies**: T034, T036, T038
- **Success**: Tests compile but fail

### T045 [US3] [P] Implement V20250326 ↔ V20250618 adapters
- **File**: `src/protocol/adapters/v20250326_to_v20250618.rs`, `src/protocol/adapters/v20250618_to_v20250326.rs`
- **Action**: Implement both directions
- **Dependencies**: T044
- **Success**: Both adapter tests pass

### T046 [US3] Update adapter factory with all adapters
- **File**: `src/protocol/adapters/mod.rs`
- **Action**: Update create_adapter() to handle all version pairs (9 combinations: 3×3 including pass-through)
- **Dependencies**: T040, T041, T043, T045
- **Success**: Factory returns correct adapter for any version pair

### T047 [US3] Integrate adapters into proxy handler
- **File**: `src/proxy/handler.rs`
- **Action**: Use adapter to translate requests before forwarding, translate responses before returning
- **Details**: Get adapter from ServerConnectionState, call translate_request/response
- **Dependencies**: T046, T025
- **Success**: Requests translated correctly end-to-end

### T048 [US3] [TEST] Write end-to-end translation tests
- **File**: `tests/integration/translation_tests.rs`
- **Action**: Write integration tests with mock servers
- **Test Cases**:
  - Client (2025-03-26) → Proxy → Server (2024-11-05)
  - Client (2024-11-05) → Proxy → Server (2025-06-18)
  - Client (2025-06-18) → Proxy → Server (2025-03-26)
  - All three versions in parallel (3 servers)
  - Tools/list, resources/read, tools/call work across versions
- **Dependencies**: T047
- **Success**: Tests compile with mock servers

### T049 [US3] Verify US3 acceptance criteria
- **File**: `tests/integration/translation_tests.rs` (add assertions)
- **Action**: Ensure all US3 acceptance scenarios pass
- **Dependencies**: T048
- **Success**: All 3 US3 acceptance scenarios verified

---

## Phase 6: User Story 4 (P3) - Version Compatibility Reporting

**Goal**: Add visibility into protocol versions and compatibility status.

### T050 [US4] [TEST] Write version reporting API tests
- **File**: `tests/integration/version_reporting_tests.rs`
- **Action**: Write tests for version information in API responses
- **Test Cases**:
  - GET /api/server/{name}/status includes protocolVersion field
  - GET /api/servers includes protocolVersion for each server
  - Version shown correctly for all three versions
  - Unknown/failed servers show appropriate status
- **Dependencies**: T021
- **Success**: Tests compile but fail

### T051 [US4] Add protocol version to server status API
- **File**: `src/web/api.rs`
- **Action**: Include protocol_version in server status responses
- **Dependencies**: T050, T021
- **Success**: Version reporting tests pass

### T052 [US4] [TEST] Write version mismatch logging tests
- **File**: `tests/integration/version_reporting_tests.rs` (add section)
- **Action**: Write tests for structured logging of version events
- **Test Cases**:
  - INFO log on successful version negotiation
  - WARN log when server uses oldest supported version
  - DEBUG log for translation operations
  - Structured fields: server_name, protocol_version, source_version, target_version
- **Dependencies**: T050
- **Success**: Tests verify log output

### T053 [US4] Implement structured version logging
- **File**: `src/protocol/state.rs`, `src/protocol/adapters/*.rs`
- **Action**: Add tracing instrumentation for version events
- **Details**: Use tracing::info!, tracing::warn!, tracing::debug! with structured fields
- **Dependencies**: T052
- **Success**: Logging tests pass

### T054 [US4] [TEST] Write version deprecation warning tests
- **File**: `tests/integration/version_reporting_tests.rs` (add section)
- **Action**: Write tests for deprecation notifications
- **Test Cases**:
  - Server using 2024-11-05 triggers deprecation WARN
  - Deprecation message includes timeline
  - Newer versions don't trigger warning
- **Dependencies**: T052
- **Success**: Tests compile but fail

### T055 [US4] Implement version deprecation notifications
- **File**: `src/protocol/version.rs`
- **Action**: Add is_deprecated() method, log warnings when old versions detected
- **Dependencies**: T054
- **Success**: Deprecation warning tests pass

### T056 [US4] Verify US4 acceptance criteria
- **File**: `tests/integration/version_reporting_tests.rs` (add assertions)
- **Action**: Ensure all US4 acceptance scenarios pass
- **Dependencies**: T051, T053, T055
- **Success**: All 3 US4 acceptance scenarios verified

---

## Phase 7: Polish, Integration & Documentation

**Goal**: Complete testing, performance validation, and documentation.

### T057 [Polish] [TEST] Write protocol compliance tests for 2024-11-05
- **File**: `tests/compliance/v20241105_compliance.rs`
- **Action**: Write compliance tests against MCP spec for version 2024-11-05
- **Test Cases**:
  - Initialize sequence follows spec
  - Message formats match spec
  - Required fields present
  - Optional fields handled correctly
- **Dependencies**: T049
- **Success**: Tests compile but fail

### T058 [Polish] [P] Fix 2024-11-05 compliance issues
- **File**: Various (based on test failures)
- **Action**: Fix any spec compliance issues found
- **Dependencies**: T057
- **Success**: All 2024-11-05 compliance tests pass

### T059 [Polish] [TEST] [P] Write protocol compliance tests for 2025-03-26
- **File**: `tests/compliance/v20250326_compliance.rs`
- **Action**: Write compliance tests for version 2025-03-26
- **Dependencies**: T049
- **Success**: Tests compile

### T060 [Polish] [P] Fix 2025-03-26 compliance issues
- **File**: Various
- **Action**: Fix compliance issues
- **Dependencies**: T059
- **Success**: All 2025-03-26 compliance tests pass

### T061 [Polish] [TEST] [P] Write protocol compliance tests for 2025-06-18
- **File**: `tests/compliance/v20250618_compliance.rs`
- **Action**: Write compliance tests for version 2025-06-18
- **Dependencies**: T049
- **Success**: Tests compile

### T062 [Polish] [P] Fix 2025-06-18 compliance issues
- **File**: Various
- **Action**: Fix compliance issues
- **Dependencies**: T061
- **Success**: All 2025-06-18 compliance tests pass

### T063 [Polish] [TEST] Write performance benchmarks
- **File**: `benches/protocol_benchmarks.rs`
- **Action**: Create criterion benchmarks for adapters
- **Benchmarks**:
  - PassThroughAdapter (< 50μs)
  - Translation adapters (< 1ms P99)
  - Version detection (< 100ms)
  - Full initialization sequence (< 60s)
- **Dependencies**: T049
- **Success**: Benchmarks compile and run

### T064 [Polish] Optimize adapter performance
- **File**: Various adapter files
- **Action**: Optimize any adapters that don't meet performance goals
- **Dependencies**: T063
- **Success**: All benchmarks meet target performance

### T065 [Polish] [TEST] Write multi-version stress test
- **File**: `tests/integration/multi_version_stress_test.rs`
- **Action**: Write stress test with 9+ servers, mixed versions, concurrent requests
- **Test Cases**:
  - 9 mock servers (3 of each version)
  - 100 concurrent clients
  - 1000 requests per client
  - All tools available (76-86 total)
  - No crashes or connection failures
  - < 5% error rate
- **Dependencies**: T049
- **Success**: Tests compile

### T066 [Polish] Fix stress test issues
- **File**: Various
- **Action**: Fix any issues found during stress testing
- **Dependencies**: T065
- **Success**: Stress test passes reliably

### T067 [Polish] Update CLAUDE.md with protocol version info
- **File**: `CLAUDE.md`
- **Action**: Document protocol version support, how to debug version issues
- **Dependencies**: T056
- **Success**: Documentation complete and accurate

### T068 [Polish] Create protocol version debugging guide
- **File**: `specs/003-mcp-protocol-support/debugging.md`
- **Action**: Write guide for troubleshooting version-related issues
- **Content**: How to check server version, interpret logs, diagnose translation errors
- **Dependencies**: T067
- **Success**: Debugging guide complete

### T069 [Polish] Update README with version support
- **File**: `README.md`
- **Action**: Add section about multi-version protocol support
- **Dependencies**: T067
- **Success**: README updated

### T070 [Polish] Run full test suite
- **File**: N/A (testing)
- **Action**: Run `cargo test --all` and verify all tests pass
- **Dependencies**: T058, T060, T062, T064, T066
- **Success**: All tests pass (unit, integration, compliance)

### T071 [Polish] Run formatter and linter
- **File**: N/A (code quality)
- **Action**: Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`
- **Dependencies**: T070
- **Success**: No formatting or linting issues

### T072 [Polish] Final integration test with real MCP servers
- **File**: N/A (manual testing)
- **Action**: Test with real backend MCP servers using different versions
- **Details**: Configure 3+ real servers, verify all provide tools, no crashes
- **Dependencies**: T071
- **Success**: All 9+ servers connect, provide tools, no errors

---

## Task Summary

**Total Tasks**: 72
**Phase Breakdown**:
- Phase 1 (Setup): 4 tasks
- Phase 2 (Foundation): 7 tasks
- Phase 3 (US1 - P1): 12 tasks
- Phase 4 (US2 - P1): 7 tasks
- Phase 5 (US3 - P2): 19 tasks
- Phase 6 (US4 - P3): 7 tasks
- Phase 7 (Polish): 16 tasks

**Parallel Opportunities**: 12 tasks marked [P]

**Estimated Complexity**:
- Small (< 2 hours): 35 tasks (tests, simple implementations)
- Medium (2-4 hours): 28 tasks (adapters, integration)
- Large (4-8 hours): 9 tasks (complex integration, stress testing)

**Critical Path** (longest dependency chain):
T001 → T002 → T003 → T005 → T006 → T008 → T010 → T011 → T015 → T018 → T019 → T021 → T025 → T047 → T048 → T049 → T070 → T071 → T072

**Success Criteria Verification**:
- SC-001 to SC-008: Verified in T023 (US1), T030 (US2), T049 (US3), T056 (US4), T072 (final)

---

## Development Notes

**TDD Workflow**:
1. Write test (T00X [TEST])
2. Verify test compiles and fails
3. Implement feature (T00X+1)
4. Verify test passes
5. Commit with clear message

**Commit Strategy**:
- Commit after each test passes
- Commit message format: "feat(protocol): T00X - Description"
- Reference spec.md user stories in commits
- Keep commits small and focused

**Testing Strategy**:
- Unit tests for each component in isolation
- Integration tests for end-to-end scenarios
- Compliance tests for spec adherence
- Performance benchmarks for optimization
- Stress tests for reliability

**Performance Monitoring**:
- PassThroughAdapter: < 50μs overhead
- Translation adapters: < 1ms P99 latency
- Version detection: < 100ms per server
- Full initialization: < 60s per server

**Quality Gates**:
- All tests pass before merge
- No compiler warnings
- No clippy warnings
- Code formatted with rustfmt
- All 4 user stories verified
- All 8 success criteria met

---

## Dependencies Between Phases

```
Phase 1 (Setup)
    ↓
Phase 2 (Foundation)
    ↓
Phase 3 (US1 - Version Detection) ←─── Critical for all other phases
    ↓
Phase 4 (US2 - Initialization) ←─── Depends on US1
    ↓
Phase 5 (US3 - Translation) ←─── Depends on US1, US2
    ↓
Phase 6 (US4 - Reporting) ←─── Depends on US1, US3
    ↓
Phase 7 (Polish) ←─── Depends on all phases
```

Each phase must be completed before the next phase begins, except where marked [P] for parallel execution within a phase.
