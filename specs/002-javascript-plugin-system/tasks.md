# Implementation Tasks: JavaScript Plugin System for MCP Proxy

**Feature Branch**: `002-javascript-plugin-system`
**Date**: 2025-10-10
**Total Tasks**: 45

This document breaks down the JavaScript plugin system implementation into dependency-ordered, atomic tasks organized by user story priority.

---

## Task Overview by User Story

| Phase | User Story | Task Count | Can Start After |
|-------|------------|------------|-----------------|
| Phase 1 | Setup & Infrastructure | 5 tasks | - |
| Phase 2 | Foundational (Prerequisites) | 8 tasks | Phase 1 complete |
| Phase 3 | **US1: Content Curation (P1)** | 12 tasks | Phase 2 complete |
| Phase 4 | **US2: Security Middleware (P2)** | 8 tasks | Phase 3 complete |
| Phase 5 | **US3: Response Transformation (P3)** | 7 tasks | Phase 4 complete |
| Phase 6 | Polish & Integration | 5 tasks | Phase 5 complete |

---

## Phase 1: Setup & Infrastructure

**Goal**: Initialize project structure and dependencies for plugin system.

### T001: Create plugin module structure [P] ✅
**File**: `src/plugin/mod.rs`
```rust
// Create new module with public exports
pub mod manager;
pub mod process;
pub mod schema;
pub mod chain;
pub mod config;

pub use manager::PluginManager;
pub use process::{PluginProcess, ProcessPool};
pub use schema::{PluginInput, PluginOutput, PluginPhase};
pub use chain::PluginChain;
pub use config::PluginConfig;
```
**Dependencies**: None
**Success**: Module compiles, exports are accessible from `src/lib.rs`

---

### T002: Add plugin module to main library [P] ✅
**File**: `src/lib.rs`
```rust
pub mod plugin;
```
**Dependencies**: T001
**Success**: Plugin module is part of library exports

---

### T003: Create test directories [P] ✅
**Directories**:
- `tests/plugin/unit/`
- `tests/plugin/integration/`
- `tests/fixtures/plugins/`
- `examples/plugins/`
- `examples/configs/`

**Dependencies**: None
**Success**: All directories exist with `.gitkeep` or README files

---

### T004: Create example plugin directory structure [P] ✅
**Files**:
- `examples/plugins/package.json` (empty dependencies object)
- `examples/configs/plugin-config.yaml` (minimal config template)

**Dependencies**: T003
**Success**: Directory structure ready for example plugins

---

### T005: Update Cargo.toml dependencies [P] ✅
**File**: `Cargo.toml`

Add if not present:
```toml
[dependencies]
# Already have: tokio, serde_json, nix, prometheus
# No new dependencies needed for MVP (JSON only)
```

**Dependencies**: None
**Success**: `cargo check` passes with no new dependency errors

---

## Phase 2: Foundational Tasks (Prerequisites for All User Stories)

**Goal**: Implement core infrastructure that all user stories depend on.

**🚨 CHECKPOINT**: Phase 1 must be complete before starting Phase 2

---

### T006: [Foundation] Define plugin schema types ✅
**File**: `src/plugin/schema.rs`

Implement:
- `PluginPhase` enum (Request, Response)
- `PluginInput` struct with JSON serialization
- `PluginMetadata` struct
- `PluginOutput` struct with JSON serialization
- `PluginError` enum (Timeout, ProcessFailed, InvalidOutput, etc.)

**Dependencies**: T001
**Success**: All types compile, serde derives work, unit tests pass

---

### T007: [Foundation] Implement plugin configuration types [P] ✅
**File**: `src/plugin/config.rs`

Implement:
- `PluginConfig` struct matching data model
- `ServerPluginAssignment` struct
- Configuration validation methods
- Figment integration for YAML/JSON/TOML parsing

**Dependencies**: T001
**Success**: Config parses from YAML, validation works

---

### T008: [Foundation] Extend main config schema [P] ✅
**File**: `src/config/schema.rs`

Add:
```rust
pub struct ProxyConfig {
    // ... existing fields
    pub plugins: Option<PluginConfig>,
}
```

**Dependencies**: T007
**Success**: Main config accepts plugin configuration, backwards compatible

---

### T009: [Foundation] Implement Node.js process spawning ✅
**File**: `src/plugin/process.rs`

Implement:
- `PluginProcess` struct with `Child`, stdin/stdout/stderr handles
- `spawn()` function using `tokio::process::Command`
- `kill_on_drop(true)` for zombie prevention
- Basic health check (`is_healthy()`)

**Dependencies**: T006
**Success**: Can spawn Node.js process, capture stdio, prevent zombies

---

### T010: [Foundation] Implement timeout mechanism ✅
**File**: `src/plugin/process.rs`

Implement:
- `execute_with_timeout()` async function
- Two-phase shutdown: SIGTERM → wait 5s → SIGKILL
- `tokio::time::timeout` wrapper
- Timeout error handling

**Dependencies**: T009
**Success**: Process killed on timeout, no zombies, proper error returned

---

### T011: [Foundation] Implement plugin I/O communication ✅
**File**: `src/plugin/process.rs`

Implement:
- `write_input()`: JSON serialization → stdin (separate task to avoid deadlock)
- `read_output()`: stdout → JSON deserialization
- `read_stderr()`: error capture
- Newline-based protocol (write `\n`, read line)

**Dependencies**: T010
**Success**: JSON round-trip works, errors captured from stderr

---

### T012: [Foundation] Implement process pool ✅
**File**: `src/plugin/process.rs`

Implement:
- `ProcessPool` struct with `Arc<Mutex<VecDeque<PluginProcess>>>`
- `acquire()`: get from pool or spawn new
- `release()`: return to pool if healthy
- `shutdown()`: graceful termination of all processes

**Dependencies**: T011
**Success**: Pool reuses processes, respects max size, cleans up on drop

---

### T013: [Foundation] Implement concurrency control ✅
**File**: `src/plugin/manager.rs`

Implement:
- `PluginManager` with `Arc<Semaphore>` for global concurrency limit
- Per-plugin process pools
- `execute()` method that acquires permit → gets process → runs → releases

**Dependencies**: T012
**Success**: Semaphore limits concurrent executions, backpressure works

---

**🚨 CHECKPOINT**: Phase 2 complete → Foundation ready for user stories

---

## Phase 3: User Story 1 - Content Curation (P1)

**Goal**: Enable users to configure curation plugins for documentation servers.

**Independent Test**: Configure a curation plugin for Context7, make a documentation query, verify output is reduced by 60-80% while preserving key information.

**Why First**: Primary use case, highest value, demonstrates full plugin flow.

**Dependencies**: Phase 2 complete

---

### T014: [US1] Implement plugin discovery and loading ✅
**File**: `src/plugin/manager.rs`

Implement:
- `discover_plugins()`: scan `plugin_dir` for `.js` files
- `Plugin` struct creation (id, name, path, phase from config)
- Store in `Arc<DashMap<String, Plugin>>`
- Validation (file exists, readable, matches config)

**Dependencies**: T013
**Success**: Plugins discovered from directory, stored in manager, accessible by name

---

### T015: [US1] Implement plugin chain builder ✅
**File**: `src/plugin/chain.rs`

Implement:
- `PluginChain` struct
- `build_chain()`: filter plugins by server, phase, tool
- Sort by order (ascending)
- Filter by enabled status

**Dependencies**: T014
**Success**: Chain correctly orders enabled plugins for a server/phase

---

### T016: [US1] Implement sequential chain execution ✅
**File**: `src/plugin/chain.rs`

Implement:
- `execute_chain()` async method
- Sequential execution: pass output of plugin N to input of plugin N+1
- Early termination if `continue=false`
- Error propagation with plugin name context

**Dependencies**: T015
**Success**: Chain executes in order, stops on continue=false, passes data correctly

---

### T017: [US1] Implement plugin output validation ✅
**File**: `src/plugin/schema.rs`

Implement:
- JSON schema validation for `PluginOutput`
- Required field checking (`text`, `continue`)
- Error field validation (if error, continue must be false)

**Dependencies**: T006
**Success**: Invalid outputs rejected with clear error messages

---

### T018: [US1] Implement graceful error handling ✅
**File**: `src/plugin/chain.rs`

Implement:
- Catch all plugin errors (timeout, crash, invalid output)
- Return original content on error
- Log error with plugin name, request ID, failure reason
- Ensure proxy doesn't crash on plugin failure

**Dependencies**: T017
**Success**: All error types handled, original content preserved, logged

---

### T019: [US1] Add response-phase hook to proxy handler ✅
**File**: `src/proxy/handler.rs`

Implement:
- After MCP server response received, check for plugin chain
- If chain exists, execute before returning to client
- Pass server name, tool name, response content, user query
- Replace response with plugin output

**Dependencies**: T018
**Success**: Response-phase plugins execute automatically for configured servers

---

### T020: [US1] Create echo test plugin ✅
**File**: `tests/fixtures/plugins/echo.js`

Implement:
```javascript
#!/usr/bin/env node
// Read JSON from stdin, echo back with metadata
```

**Dependencies**: T003
**Success**: Standalone plugin works, passes basic I/O test

---

### T021: [US1] Create curation example plugin ✅
**File**: `examples/plugins/curation-plugin.js`

Implement:
- Read PluginInput from stdin
- If no maxTokens, pass through unchanged
- If maxTokens set, use Claude SDK to curate content
- Return PluginOutput with reduced text
- Include metadata (original length, curated length, reduction %)

**Dependencies**: T004, T020
**Success**: Plugin reduces 50KB doc to <10KB, preserves code examples

---

### T022: [US1] Write integration test for curation flow ✅
**File**: `tests/plugin_curation_test.rs`

Test:
1. Load plugin config with curation plugin for context7
2. Mock 50KB documentation response
3. Execute plugin chain
4. Assert output is 60-80% smaller
5. Assert no facts invented (compare content)

**Dependencies**: T021
**Success**: End-to-end curation test passes

---

### T023: [US1] Add plugin execution metrics ✅
**File**: `src/plugin/manager.rs`

Implement:
- Prometheus metrics: executions_total, execution_duration, errors_total, timeouts_total
- Increment on each execution
- Label by plugin name, server, phase

**Dependencies**: T019
**Success**: Metrics visible in /metrics endpoint, accurate counts

---

### T024: [US1] Create example config with curation ✅
**File**: `examples/configs/plugin-config.yaml`

```yaml
plugins:
  pluginDir: ./examples/plugins
  servers:
    context7:
      response:
        - name: curation-plugin
          order: 1
          enabled: true
          timeoutMs: 45000
```

**Dependencies**: T021
**Success**: Config loads, curation plugin assigned to context7

---

### T025: [US1] Write quickstart documentation update ✅
**File**: PLUGIN_QUICKSTART.md

Document:
- How to create first plugin
- How to configure for a server
- How to test locally
- Curation example

**Dependencies**: T024
**Success**: New user can follow guide and set up curation plugin in 5 minutes

---

**🚨 CHECKPOINT US1**: Content curation working end-to-end
- Users can configure curation plugins ✅
- Documentation responses reduced 60-80% ✅
- No facts invented ✅
- System handles failures gracefully ✅

---

## Phase 4: User Story 2 - Security Middleware (P2)

**Goal**: Enable users to configure security plugins for request validation.

**Independent Test**: Configure a security plugin that blocks requests with sensitive data, attempt such a request, verify it's blocked with appropriate error.

**Why Second**: Security is critical, builds on curation infrastructure, adds request-phase support.

**Dependencies**: Phase 3 (US1) complete

---

### T026: [US2] Add request-phase hook to proxy handler ✅
**File**: `src/proxy/handler.rs`

Implement:
- Before forwarding to MCP server, check for request-phase plugin chain
- If chain exists, execute on request content
- If plugin returns `continue=false` or error, block request
- Return error to client without calling MCP server

**Dependencies**: T019
**Success**: Request-phase plugins execute, can block requests before server

---

### T027: [US2] Implement request blocking logic ✅
**File**: `src/proxy/handler.rs`

Add:
- `apply_request_plugins()` method
- If any plugin returns `continue=false`, halt request
- Return blocked request error to client with plugin name

**Dependencies**: T026
**Success**: Blocked requests don't reach MCP server, clear error returned

---

### T028: [US2] Create security example plugin ✅
**File**: `examples/plugins/security-plugin.js`

Implement:
- Check for sensitive patterns (password, secret, api_key, token)
- If found, return `continue=false` with security error
- If phase is response, pass through unchanged

**Dependencies**: T004
**Success**: Plugin blocks requests with sensitive data, allows safe requests

---

### T029: [US2] Write integration test for security blocking ✅
**File**: `tests/plugin_security_test.rs`

Test:
1. Load plugin config with security plugin for filesystem server
2. Send request with "password: secret123"
3. Assert request blocked before reaching server
4. Assert error includes security violation message

**Dependencies**: T028
**Success**: Security test passes, request properly blocked

---

### T030: [US2] Create logging example plugin [P] ✅
**File**: `examples/plugins/logging-plugin.js`

Implement:
- Log all requests to stderr with timestamp, server, tool
- Pass through unchanged (continue=true)
- Demonstrate audit use case

**Dependencies**: T004
**Success**: Plugin logs sensitive operations, doesn't modify data

---

### T031: [US2] Add example security config ✅
**File**: `examples/configs/security-config.yaml`

```yaml
plugins:
  pluginDir: ./examples/plugins
  servers:
    filesystem:
      request:
        - name: security-plugin
          order: 1
          enabled: true
        - name: logging-plugin
          order: 2
          enabled: true
```

**Dependencies**: T028, T030
**Success**: Config loads, security + logging plugins chain correctly

---

### T032: [US2] Write unit tests for phase detection ✅
**File**: `tests/plugin_unit_tests.rs` (includes `tests/plugin/unit/phase_tests.rs`)

Test:
- Plugin correctly identifies request vs response phase
- Only request-phase plugins execute on requests
- Only response-phase plugins execute on responses

**Dependencies**: T026
**Success**: Phase isolation verified, no cross-execution

---

### T033: [US2] Document security plugin patterns ✅
**File**: PLUGIN_QUICKSTART.md (updated)

Document:
- How to write security validation plugins
- Request-phase vs response-phase
- Blocking requests with continue=false
- Audit logging patterns

**Dependencies**: T031
**Success**: Security plugin documentation clear and accurate

---

**🚨 CHECKPOINT US2**: Security middleware working
- Requests can be blocked by security plugins ✅
- Sensitive data detected and rejected ✅
- Audit logging captures operations ✅
- Request phase fully functional ✅

---

## Phase 5: User Story 3 - Response Transformation (P3)

**Goal**: Enable users to chain multiple plugins for custom transformations.

**Independent Test**: Configure 2+ transformation plugins, make a request, verify each plugin's transformation is applied in sequence.

**Why Third**: Demonstrates plugin chaining, less critical than curation and security.

**Dependencies**: Phase 4 (US2) complete

---

### T034: [US3] Implement metadata passthrough ✅
**File**: `src/plugin/chain.rs`

Implement:
- Collect metadata from each plugin in chain
- Aggregate in final response
- Preserve metadata from all plugins, not just last

**Dependencies**: T016
**Success**: Metadata from all chained plugins visible in final output

---

### T035: [US3] Create path normalization plugin ✅
**File**: `examples/plugins/path-normalizer.js`

Implement:
- Detect file paths in response
- Convert to platform-specific format (e.g., Windows → Unix)
- Return transformed text with metadata (paths_normalized: count)

**Dependencies**: T004
**Success**: Plugin transforms paths, reports count in metadata

---

### T036: [US3] Create metadata enrichment plugin [P] ✅
**File**: `examples/plugins/enrich-metadata.js`

Implement:
- Add custom metadata to response
- Timestamp, processing info, etc.
- Pass through content unchanged

**Dependencies**: T004
**Success**: Plugin adds metadata without modifying content

---

### T037: [US3] Write integration test for chaining ✅
**File**: `tests/plugin_chaining_test.rs`

Test:
1. Configure 3-plugin chain (echo → path-normalizer → enrichment)
2. Send request through chain
3. Assert each plugin's transformation applied
4. Assert execution order correct
5. Assert metadata from all plugins present

**Dependencies**: T035, T036
**Success**: Chain test passes, transformations applied in order, metadata complete

---

### T038: [US3] Create example chaining config ✅
**File**: `examples/configs/chaining-config.yaml`

```yaml
plugins:
  pluginDir: ./examples/plugins
  servers:
    context7:
      response:
        - name: curation-plugin
          order: 1
        - name: path-normalizer
          order: 2
        - name: enrich-metadata
          order: 3
```

**Dependencies**: T035, T036
**Success**: Config loads, 3-plugin chain executes correctly

---

### T039: [US3] Write unit tests for chain termination ✅
**File**: `tests/plugin/unit/chain_tests.rs`

Test:
- Chain stops when plugin returns continue=false
- Remaining plugins not executed
- Output from last executed plugin returned

**Dependencies**: T016
**Success**: Chain termination logic verified (6 unit tests passing)

---

### T040: [US3] Document plugin chaining patterns ✅
**File**: PLUGIN_QUICKSTART.md (updated)

Document:
- How to chain multiple plugins
- Execution order (by `order` field)
- When to use continue=false
- Metadata aggregation

**Dependencies**: T038
**Success**: Chaining documentation complete and clear

---

**🚨 CHECKPOINT US3**: Response transformation working
- Multiple plugins chain correctly ✅
- Transformations applied in order ✅
- Metadata from all plugins preserved ✅
- Chain termination works as expected ✅

---

## Phase 6: Polish & Cross-Cutting Concerns

**Goal**: Finalize system with monitoring, error handling, and documentation.

**Dependencies**: All user stories (Phase 3, 4, 5) complete

---

### T041: Implement comprehensive error logging ✅
**File**: `src/plugin/manager.rs`

Implement:
- Structured logging with tracing spans
- Log every plugin execution (success/failure/timeout)
- Include: plugin name, server, tool, duration, input size, output size
- Error details for failures

**Dependencies**: T023
**Success**: All plugin executions logged with full context, easy to debug

---

### T042: Add plugin pool metrics [P] ✅
**File**: `src/state/metrics.rs`

Implement:
- Prometheus metrics: pool_size, pool_available, spawned_total, killed_total
- Zombie process detection (warn if count > 10)
- Pool utilization tracking

**Dependencies**: T023
**Success**: Pool metrics visible, zombie detection works

---

### T043: Write end-to-end integration tests [P] ✅
**File**: `tests/plugin_end_to_end_test.rs`

Test all user stories combined:
1. Complete plugin system integration
2. Error handling for all error types
3. Plugin discovery and counting
4. Verify all plugins execute correctly

**Dependencies**: T037, T029, T022
**Success**: Full integration test passes, all scenarios covered

---

### T044: Create production deployment guide ✅
**File**: `docs/PLUGIN_DEPLOYMENT.md`

Document:
- Production configuration best practices
- Performance tuning (pool sizes, timeouts)
- Monitoring and alerting setup
- Troubleshooting common issues
- Security considerations

**Dependencies**: T041, T042
**Success**: Production deployment guide complete

---

### T045: Final polish and documentation review ✅
**File**: Various docs (CHANGELOG.md, PLUGIN_QUICKSTART.md, README.md)

Tasks:
- Review all plugin documentation for accuracy
- Add examples to documentation
- Create CHANGELOG entry
- Ensure all TODOs addressed or documented

**Dependencies**: T044
**Success**: All documentation polished, ready for release

---

**🚨 FINAL CHECKPOINT**: System complete and production-ready
- All user stories implemented ✅
- Comprehensive test coverage ✅
- Monitoring and metrics in place ✅
- Documentation complete ✅

---

## Task Dependencies Graph

```
Phase 1 (Setup)
T001 → T002
T003 → T004
T005 (independent)

Phase 2 (Foundation)
T001 → T006 → T009 → T010 → T011 → T012 → T013
T001 → T007 → T008

Phase 3 (US1 - Curation)
T013 → T014 → T015 → T016 → T017 → T018 → T019 → T022
T003 → T020
T004, T020 → T021 → T022
T019 → T023
T021 → T024 → T025

Phase 4 (US2 - Security)
T019 → T026 → T027 → T029
T004 → T028 → T029
T004 → T030
T028, T030 → T031 → T033
T026 → T032

Phase 5 (US3 - Transformation)
T016 → T034
T004 → T035, T036
T035, T036 → T037 → T038 → T040
T016 → T039

Phase 6 (Polish)
T023 → T041
T023 → T042
T037, T029, T022 → T043
T041, T042 → T044 → T045
```

---

## Parallel Execution Opportunities

### Phase 1 (All tasks can run in parallel)
- T001, T002, T003, T004, T005 → **5 tasks in parallel**

### Phase 2
- After T001: T006 and T007 in parallel → **2 tasks**
- After T006: T009 (sequential path for process management)
- After T007: T008 in parallel with T009-T013 → **opportunistic**

### Phase 3 (US1)
- T020 (test plugin) parallel with T014-T019 → **background task**
- T023 (metrics) parallel with T024-T025 → **2 tasks in parallel**

### Phase 4 (US2)
- T030 (logging plugin) parallel with T028 (security plugin) → **2 tasks**
- T032 (unit tests) parallel with T031, T033 → **opportunistic**

### Phase 5 (US3)
- T035, T036 (plugins) in parallel → **2 tasks**
- T039 (unit tests) parallel with T037-T040 → **opportunistic**

### Phase 6
- T041, T042 in parallel → **2 tasks**
- T043 parallel with T044 → **2 tasks**

---

## Implementation Strategy

### MVP Scope (Recommended First Release)
**Ship after Phase 3 (US1) complete**:
- ✅ Basic plugin infrastructure
- ✅ JSON I/O protocol
- ✅ Process spawning & timeout
- ✅ Response-phase plugins
- ✅ Curation use case (primary value)
- ✅ Error handling & logging
- ✅ Example plugins & docs

**Defer to v1.1**:
- US2 (Security) - important but can follow
- US3 (Transformation) - nice to have

**Defer to v2.0**:
- MessagePack support
- Hot-reloading
- Advanced monitoring

### Incremental Delivery Plan

**Week 1**: Phase 1 + 2 (Setup + Foundation)
- All infrastructure ready
- Can spawn plugins, basic I/O works

**Week 2**: Phase 3 (US1 - Curation)
- End-to-end curation working
- **MVP complete, shippable**

**Week 3**: Phase 4 (US2 - Security)
- Request-phase support added
- Security validated

**Week 4**: Phase 5 + 6 (US3 + Polish)
- Chaining refined
- Production-ready
- **Full v1.0 release**

---

## Testing Strategy

### Unit Tests (Per Task)
- Schema validation (T006, T017)
- Phase detection (T032)
- Chain termination (T039)

### Integration Tests (Per User Story)
- Curation flow (T022)
- Security blocking (T029)
- Plugin chaining (T037)

### End-to-End Tests (Final)
- Full system integration (T043)

### Manual Testing Checklist
- [ ] Echo plugin works standalone
- [ ] Curation reduces content 60-80%
- [ ] Security blocks sensitive requests
- [ ] Chain executes in correct order
- [ ] Errors logged with full context
- [ ] Metrics accurate
- [ ] No zombie processes
- [ ] Proxy doesn't crash on plugin failures

---

## Success Criteria Mapping

| Success Criterion | Validated By |
|-------------------|--------------|
| **SC-001**: 60-80% reduction | T022 (curation integration test) |
| **SC-002**: <500ms latency | T023 (metrics), T041 (logging) |
| **SC-003**: No crashes on 100% failures | T018 (error handling), T043 (e2e) |
| **SC-004**: Configure without restart | T024 (example config load) |
| **SC-005**: 100% errors caught | T018 (graceful handling), T041 (logging) |
| **SC-006**: Chain 5 plugins no degradation | T037 (chaining test) |

---

**Total Estimated Effort**: 4 weeks (1 developer)
- Phase 1: 1 day
- Phase 2: 4 days
- Phase 3 (US1): 5 days ← **MVP milestone**
- Phase 4 (US2): 3 days
- Phase 5 (US3): 2 days
- Phase 6: 1 day

**Generated**: 2025-10-10
**Ready for**: Implementation via `/speckit.implement` or manual execution
