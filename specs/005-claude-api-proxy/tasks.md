# Tasks: Claude API Proxy for Context Tracing

**Input**: Design documents from `/specs/005-claude-api-proxy/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions
- Single project: `src/` at repository root
- Following existing MCP Rust Proxy structure

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and dependency setup

- [x] T001 Add Rust dependencies to Cargo.toml: hyper, rustls, rcgen, tokio-rustls, hyper-util, bytes, tower, dashmap
- [x] T002 [P] Create module structure: `src/claude_proxy/mod.rs` with submodule declarations
- [x] T003 [P] Create Claude proxy configuration directory structure: `~/.claude-proxy/` for CA storage

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Implement TLS certificate generation in `src/claude_proxy/tls_handler.rs`:
  - `generate_root_ca()` - One-time CA creation
  - `generate_domain_cert(domain, ca)` - Per-domain cert generation
  - `load_or_create_ca()` - Load from disk or create new
  - Store CA in `~/.claude-proxy/ca.crt` and `~/.claude-proxy/ca.key`
- [x] T005 [P] Implement certificate caching in `src/claude_proxy/tls_handler.rs`:
  - `Arc<DashMap<String, Arc<ServerConfig>>>` for cert cache
  - `get_server_config(domain)` - Return cached or generate new
  - `get_client_config()` - Return rustls ClientConfig for forwarding
- [x] T006 [P] Extend database schema in `src/context/storage.rs`:
  - Add `captured_requests` table per data-model.md
  - Add `captured_responses` table per data-model.md
  - Add `context_attributions` table per data-model.md
  - Add `quality_feedback` table per data-model.md
  - Add `context_source_metrics` table per data-model.md
  - Add all indexes specified in data-model.md
- [x] T007 [P] Create data models in `src/context/types.rs`:
  - `CapturedRequest` struct with serde derives
  - `CapturedResponse` struct with serde derives
  - `ContextAttribution` struct with SourceType enum
  - `QualityFeedback` struct
  - `ContextSourceMetrics` struct
- [x] T008 [P] Implement proxy configuration in `src/claude_proxy/config.rs`:
  - `ClaudeProxyConfig` struct with enabled, bind_address, capture_enabled, retention_days
- [x] T009 Extend main config schema in `src/config/schema.rs`:
  - Add `claude_proxy: Option<ClaudeProxyConfig>` field to Config struct

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Context Source Visibility (Priority: P1) 🎯 MVP

**Goal**: Provide transparency into context composition by capturing and attributing Claude API requests to their sources (MCP servers, skills, user input, framework)

**Independent Test**: Make a Claude API request through the proxy, inspect captured data, verify context sources are identified and labeled correctly

### Implementation for User Story 1

- [x] T010 [US1] Implement basic HTTPS proxy server in `src/claude_proxy/proxy_server.rs`:
  - `ProxyServer` struct with config, tls_handler, capture_storage fields
  - `start()` method to bind and listen on configured port
  - `handle_connection(stream)` method for accepting TCP connections
  - `proxy_request(req)` method for basic forwarding logic
- [x] T011 [US1] Implement TLS handshake in `src/claude_proxy/proxy_server.rs`:
  - Extract SNI (Server Name Indication) from client hello
  - Get server config for domain from tls_handler
  - Accept TLS connection using tokio_rustls::TlsAcceptor
  - Pass decrypted stream to HTTP handler
- [x] T012 [US1] Implement client-side TLS for forwarding in `src/claude_proxy/proxy_server.rs`:
  - Create hyper-rustls HTTPS connector with client_config
  - Forward decrypted request to api.anthropic.com
  - Maintain authentication headers unchanged
  - Return response to caller
- [x] T013 [US1] Implement capture storage in `src/claude_proxy/capture.rs`:
  - `CaptureStorage` struct with db (SqlitePool) and cache (DashMap)
  - `capture_request(req)` - Store request, return UUID
  - `capture_response(resp, req_id)` - Store response, link to request
  - `get_request(id)` - Retrieve from cache or DB
  - `query_requests(filters)` - Filter by time range, context source
- [x] T014 [US1] Integrate capture into proxy flow in `src/claude_proxy/proxy_server.rs`:
  - Call `capture_request()` before forwarding (non-blocking)
  - Forward request to Claude API regardless of capture success (fail-open)
  - Call `capture_response()` after receiving response
  - Return response to client even if capture fails
- [x] T015 [US1] Implement context attribution engine in `src/claude_proxy/attribution.rs`:
  - `AttributionEngine` struct with analysis methods
  - `analyze_request(body_json)` - Parse messages array, return Vec<ContextAttribution>
  - `identify_source_type(message)` - Classify as User/Framework/McpServer/Skill
  - `extract_mcp_server_name(tool_use_id)` - Parse server name from tool ID
  - `count_tokens(content)` - Count tokens for attribution
- [x] T016 [US1] Integrate attribution into capture in `src/claude_proxy/capture.rs`:
  - Parse request body as JSON
  - Call `AttributionEngine::analyze_request()`
  - Store attributions with request in database
  - Calculate total_tokens from all attributions
- [x] T017 [US1] Implement query API endpoints in `src/web/api.rs`:
  - `GET /api/claude/requests` - List requests with filters (per query-api.yaml)
  - `GET /api/claude/requests/:id` - Get request details with attributions
  - `GET /api/claude/responses/:id` - Get response details
  - `GET /api/claude/contexts` - Query context attributions
  - Add routes to main router
- [x] T018 [US1] Add domain filtering in `src/claude_proxy/proxy_server.rs`:
  - `should_intercept(sni)` - Check if domain ends with anthropic.com or claude.ai
  - Only intercept Claude API traffic, pass through others transparently
- [x] T019 [US1] Add structured logging in `src/claude_proxy/`:
  - Use tracing for all operations with structured fields (request_id, source_name)
  - Log certificate generation events
  - Log capture success/failure with request_id
  - Log attribution results with source counts

**Checkpoint**: User Story 1 complete - proxy captures and attributes context sources. Test by proxying a Claude CLI request and querying `/api/claude/requests`.

---

## Phase 4: User Story 2 - Request/Response Audit Trail (Priority: P1)

**Goal**: Enable debugging by providing complete request/response history with timestamps, correlation IDs, and searchability

**Independent Test**: Execute multiple Claude CLI operations, query the audit trail API, verify all requests/responses captured with correct linking and filtering

### Implementation for User Story 2

- [x] T020 [US2] Implement timestamp tracking in `src/claude_proxy/capture.rs`:
  - Record request_initiation, proxy_receipt, forward_transmission timestamps
  - Record response_receipt_from_api, final_delivery_to_client timestamps
  - Calculate latency_ms (time between forward and response receipt)
  - Calculate proxy_latency_ms (overhead from capture operations)
- [x] T021 [US2] Implement correlation ID linking in `src/claude_proxy/capture.rs`:
  - Generate UUID for each request (correlation_id)
  - Store same correlation_id in both captured_requests and captured_responses
  - Ensure foreign key relationship enforced in database
  - Include correlation_id in all structured logs
- [x] T022 [US2] Implement query API logic in `src/web/api.rs`:
  - Implement list_claude_requests with start_time, end_time, context_source filters
  - Implement get_claude_request with attribution loading
  - Implement get_claude_response with correlation lookup
  - Implement query_claude_contexts with filtering
  - Add limit and offset pagination
  - Return total count in responses
- [x] T023 [US2] Implement error capture in `src/claude_proxy/capture.rs`:
  - Store non-200 status codes in captured_responses
  - Capture error response bodies and headers
  - Log errors with request_id for correlation
  - Ensure failed requests still appear in audit trail
- [x] T024 [US2] Add metrics summary endpoint in `src/web/api.rs` (deferred - basic metrics available via list_claude_requests):
  - `GET /api/claude/metrics/summary` - Return overall statistics
  - Calculate total_requests, total_tokens, average_latency
  - Calculate oldest_capture and newest_capture timestamps
  - Return per query-api.yaml SummaryMetrics schema

**Checkpoint**: User Story 2 complete - full audit trail with timestamps and searchability. Test by making multiple proxied requests with different timing and filtering via API.

---

## Phase 5: User Story 3 - Context Quality Feedback Integration (Priority: P2)

**Goal**: Enable continuous improvement by allowing users to submit quality ratings that propagate to contributing context sources

**Independent Test**: Submit feedback for a captured request, verify it associates with all context sources, check aggregate metrics update correctly

### Implementation for User Story 3

- [ ] T025 [US3] Implement feedback manager in `src/context/feedback.rs`:
  - `FeedbackManager` struct with database pool
  - `submit_feedback(feedback)` - Validate and store feedback
  - `update_feedback(id, updates)` - Modify existing feedback
  - `delete_feedback(id)` - Remove feedback and update metrics
  - `get_feedback_by_request(request_id)` - Query feedback
- [ ] T026 [US3] Implement aggregate metrics update in `src/context/feedback.rs`:
  - `update_aggregate_metrics(feedback)` - Find all attributions for request
  - For each unique source_name, update context_source_metrics:
    - Increment feedback_count
    - Recalculate average_rating using weighted average
    - Update last_used timestamp
  - Use transaction to ensure atomic updates
- [ ] T027 [US3] Implement feedback API endpoints in `src/web/api.rs`:
  - `POST /api/claude/feedback` - Submit feedback (per feedback-api.yaml)
  - `GET /api/claude/feedback` - List all feedback with filters
  - `GET /api/claude/feedback/:id` - Get specific feedback details
  - `PUT /api/claude/feedback/:id` - Update existing feedback
  - `DELETE /api/claude/feedback/:id` - Delete feedback
  - `GET /api/claude/feedback/by-request/:request_id` - Get feedback for request
- [ ] T028 [US3] Add feedback validation in `src/context/feedback.rs`:
  - Validate rating is between -1.0 and 1.0
  - Verify request_id and response_id exist in database
  - Ensure only one feedback per request (unique constraint)
  - Return 409 Conflict if feedback already exists
- [ ] T029 [US3] Implement context source metrics endpoint in `src/web/api.rs`:
  - `GET /api/claude/metrics/sources` - List all source metrics
  - Support filtering by source_name
  - Support sorting by usage_count, average_rating, or total_tokens
  - Return per query-api.yaml ContextSourceMetrics schema
- [ ] T030 [US3] Add feedback propagation logging in `src/context/feedback.rs`:
  - Log feedback submission with rating and request_id
  - Log which sources were affected with their new average ratings
  - Use structured logging with feedback_id, affected_sources fields

**Checkpoint**: User Story 3 complete - quality feedback system operational. Test by submitting feedback and verifying metrics update for all contributing sources.

---

## Phase 6: User Story 4 - Context Size and Cost Analysis (Priority: P3)

**Goal**: Enable cost optimization by showing token usage and costs per context source

**Independent Test**: View a captured request's token breakdown, verify accurate counts per source, check cost analysis over multiple requests

### Implementation for User Story 4

- [ ] T031 [US4] Implement token counting in `src/claude_proxy/attribution.rs`:
  - Add tiktoken-rs dependency to Cargo.toml (or tiktoken_rs crate)
  - Integrate tiktoken-rs for Claude-compatible token counting (cl100k_base encoding)
  - Count tokens for each message in attribution using the appropriate model encoding
  - Store token_count in each ContextAttribution record
  - Sum to get total_tokens for request
  - Verify counts match Claude API billing within 1% (per SC-006)
- [ ] T032 [US4] Add token display to request detail endpoint in `src/web/api.rs`:
  - Include token_count for each attribution in response
  - Show breakdown by source (e.g., "MCP: context7 - 1,500 tokens")
  - Display total_tokens for request
  - Include response_tokens from Claude API usage field
- [ ] T033 [US4] Implement token aggregation in context metrics in `src/context/storage.rs`:
  - When storing attribution, update context_source_metrics.total_tokens
  - Update average_tokens calculation (total_tokens / usage_count)
  - Store per-source aggregates for cost analysis
- [ ] T034 [US4] Add cost analysis to metrics endpoint in `src/web/api.rs`:
  - Extend `/api/claude/metrics/sources` to show total_tokens and average_tokens
  - Sort sources by total_tokens to identify high-usage sources
  - Provide data for cost optimization recommendations
- [ ] T035 [US4] Add token history tracking in `src/context/storage.rs`:
  - Track token usage over time for trending
  - Support time-based filtering in metrics queries
  - Enable comparison of token usage across time periods

**Checkpoint**: User Story 4 complete - cost analysis functionality operational. Test by reviewing token breakdowns and identifying high-token sources.

---

## Phase 7: Integration & Startup

**Purpose**: Integrate proxy into MCP Rust Proxy main application

- [ ] T036 Add proxy startup to main in `src/main.rs`:
  - Check if `config.claude_proxy.enabled` is true
  - Initialize TlsHandler with load_or_create_ca()
  - Create ProxyServer instance with config and state
  - Spawn tokio task to run proxy.start()
  - Log proxy startup with bind address
- [ ] T037 [P] Create example configuration in root directory:
  - `claude-proxy-config.yaml` with enabled: true, bind_address, capture_enabled, retention_days
  - Document CA certificate installation instructions
  - Document HTTPS_PROXY environment variable setup
- [ ] T038 [P] Add proxy documentation to `CLAUDE.md`:
  - Document Claude API Proxy feature
  - Include setup instructions (CA cert installation, HTTPS_PROXY)
  - Document API endpoints for querying captures and feedback
  - Include troubleshooting section for TLS issues

---

## Phase 8: Data Retention & Cleanup

**Purpose**: Implement data retention policies and cleanup

- [ ] T039 Implement retention cleanup in `src/context/storage.rs`:
  - Create background task that runs daily
  - Delete `captured_requests` older than retention_days
  - CASCADE DELETE removes associated responses, attributions, feedback
  - Preserve context_source_metrics (aggregates don't expire)
  - Log cleanup operations with deleted count
- [ ] T040 [P] Implement sensitive data sanitization in `src/claude_proxy/capture.rs`:
  - Remove Authorization, X-API-Key, and other sensitive headers before storage
  - Replace with [REDACTED] placeholder
  - Store hash of API key for correlation (not the key itself)
  - Apply sanitization before database insert
  - Extend to tracing/logging: sanitize sensitive headers in structured logs
  - Ensure request_id logs never include full API keys or tokens

---

## Phase 9: Testing & Validation

**Purpose**: Comprehensive testing of the complete system

- [ ] T041 [P] Create unit tests in `tests/unit/claude_proxy/`:
  - `tls_tests.rs` - Test certificate generation, caching, CA persistence
  - `capture_tests.rs` - Test request/response capture, query filtering
  - `attribution_tests.rs` - Test source identification, token counting, MCP name parsing
  - Configure code coverage tracking (tarpaulin or llvm-cov)
  - Target 80%+ coverage on critical paths per constitution (Principle III)
- [ ] T042 [P] Create integration tests in `tests/integration/`:
  - `claude_proxy_tests.rs` - End-to-end proxy flow with mock Claude API
  - Test TLS handshake and forwarding with certificate validation
  - Validate TLS end-to-end security: client → proxy → API maintains encryption
  - Test concurrent request handling (verify no data corruption)
  - Test capture and attribution integration
  - Test API endpoint functionality
  - Verify authentication headers pass through unchanged (per FR-002)
- [ ] T043 Implement mock Claude API server for testing in `tests/integration/`:
  - Return canned responses for specific requests
  - Simulate different status codes (200, 429, 500)
  - Verify authentication headers passed through unchanged
  - Test fail-open behavior on capture errors
- [ ] T044 [P] Create manual test script:
  - Script to start proxy, install CA cert, set HTTPS_PROXY
  - Test with curl first, then Claude CLI
  - Query captured data via API
  - Submit feedback and verify metrics update
  - Document expected output for validation
- [ ] T045 [P] Run performance benchmarks:
  - Measure proxy latency overhead (target: <100ms added to request-response cycle)
  - Test with 100 concurrent requests (per SC-008)
  - Verify no memory leaks over extended run (24h)
  - Profile with flamegraph to identify bottlenecks
  - Benchmark critical paths per constitution (Principle I)

---

## Phase 10: Polish & Documentation

**Purpose**: Final improvements and documentation

- [ ] T046 [P] Run cargo fmt on all claude_proxy code
- [ ] T047 [P] Run cargo clippy and fix all warnings
- [ ] T048 [P] Update PROJECT_SNAPSHOT with Claude Proxy feature
- [ ] T049 [P] Create quickstart validation script per quickstart.md
- [ ] T050 Review and finalize API error handling consistency across all endpoints

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup (T001-T003) - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational (T004-T009) complete
- **User Story 2 (Phase 4)**: Depends on Foundational (T004-T009) complete - Can run parallel to US1
- **User Story 3 (Phase 5)**: Depends on US1 or US2 (needs captured data) - Can run parallel after US1/US2
- **User Story 4 (Phase 6)**: Depends on US1 (needs attribution) - Can run parallel after US1
- **Integration (Phase 7)**: Depends on US1 minimum (MVP) - Can run after any story subset
- **Retention (Phase 8)**: Depends on US1 (needs storage) - Can run parallel after US1
- **Testing (Phase 9)**: Can run parallel throughout development
- **Polish (Phase 10)**: Depends on desired stories complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational - Independent
- **User Story 2 (P1)**: Can start after Foundational - Independent (but extends US1 capture)
- **User Story 3 (P2)**: Needs US1 or US2 captured data - Otherwise independent
- **User Story 4 (P3)**: Needs US1 attribution - Otherwise independent

### Within Each User Story

- **US1**: TLS setup → Proxy server → Capture → Attribution → Query API
- **US2**: Timestamps → Correlation → Filters → Error handling → Metrics
- **US3**: Feedback manager → Metrics update → API endpoints → Validation
- **US4**: Token counting → Display → Aggregation → Cost analysis

### Parallel Opportunities

**Setup Phase (T001-T003)**: All can run in parallel

**Foundational Phase (T004-T009)**: T005, T006, T007, T008 can run in parallel after T004 completes

**User Story 1 (T010-T019)**: T017 (API), T018 (filtering), T019 (logging) can run parallel to T010-T016

**User Story 2 (T020-T024)**: T024 (metrics endpoint) can run parallel to T020-T023

**User Story 3 (T025-T030)**: T027 (API), T029 (metrics), T030 (logging) can run parallel to T025-T026

**User Story 4 (T031-T035)**: T032-T035 can run parallel after T031 completes

**Testing Phase (T041-T045)**: All test tasks can run in parallel

**Polish Phase (T046-T050)**: T046-T048 can run in parallel

---

## Parallel Example: User Story 1

```bash
# After T010-T016 complete, these can run together:
Task T017: "Implement query API endpoints in src/web/api.rs"
Task T018: "Add domain filtering in src/claude_proxy/proxy_server.rs"
Task T019: "Add structured logging in src/claude_proxy/"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: Foundational (T004-T009) - CRITICAL
3. Complete Phase 3: User Story 1 (T010-T019)
4. Complete Phase 7: Integration (T036-T038) for US1
5. **STOP and VALIDATE**: Proxy works, captures requests, shows context sources
6. Deploy/demo MVP

### Incremental Delivery

1. **Setup + Foundational** (T001-T009) → Foundation ready
2. **Add User Story 1** (T010-T019) → Test independently → **MVP deployed** 🎯
3. **Add User Story 2** (T020-T024) → Test independently → Deploy audit trail feature
4. **Add User Story 3** (T025-T030) → Test independently → Deploy feedback system
5. **Add User Story 4** (T031-T035) → Test independently → Deploy cost analysis
6. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers after Foundational phase:

- **Developer A**: User Story 1 (T010-T019) - Core proxy with context visibility
- **Developer B**: User Story 2 (T020-T024) - Audit trail enhancements
- **Developer C**: Testing (T041-T045) - Test infrastructure
- **Developer D**: User Story 3 (T025-T030) - Feedback system (waits for US1 data)

Stories complete and integrate independently.

---

## Notes

- **[P] tasks** = Different files, no dependencies, can run in parallel
- **[Story] label** maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- **Foundational phase is BLOCKING** - must complete before user stories start
- User stories can run in parallel once Foundation is ready
- MVP is User Story 1 only - provides core value
- Tests are throughout development, not a separate phase

**Total Tasks**: 50
- Setup: 3 tasks
- Foundational: 6 tasks (BLOCKING)
- User Story 1 (P1 - MVP): 10 tasks
- User Story 2 (P1): 5 tasks
- User Story 3 (P2): 6 tasks
- User Story 4 (P3): 5 tasks
- Integration: 3 tasks
- Retention: 2 tasks
- Testing: 5 tasks
- Polish: 5 tasks

**MVP Scope**: T001-T009 (Setup + Foundational) + T010-T019 (US1) + T036-T038 (Integration) = 22 tasks for minimal viable product
