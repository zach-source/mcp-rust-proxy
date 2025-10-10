# Tasks: AI Context Provenance & Evolution Framework

**Input**: Design documents from `/specs/001-context-tracing-framework/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/api-spec.yaml

**Tests**: Not explicitly requested in specification - focusing on implementation tasks only

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions
- Single project structure: `src/` and `tests/` at repository root
- New module: `src/context/` for all context tracing code
- API endpoints: Extend existing `src/web/api.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and dependency setup

- [X] **T001** Add rusqlite dependency to Cargo.toml with features = ["bundled", "json1"]
- [X] **T002** Create `src/context/` directory structure with mod.rs
- [X] **T003** [P] Export context module in `src/lib.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] **T004** [P] [Foundation] Create core type definitions in `src/context/types.rs`:
  - ContextType enum (System, User, External, ModelState)
  - ContextUnit struct with validation
  - ContextReference struct
  - Response struct with validation
  - LineageManifest struct
  - ProvenanceTree and ProvenanceEdge structs
  - FeedbackRecord struct with validation

- [X] **T005** [P] [Foundation] Implement validation functions in `src/context/types.rs`:
  - ContextUnit::validate() method
  - Response::validate() method with weight sum check (1.0 ± 0.01)
  - FeedbackRecord::validate() method
  - Weight normalization utilities

- [X] **T006** [Foundation] Create storage abstraction trait in `src/context/storage.rs`:
  - StorageBackend trait with async methods
  - store_context_unit, get_context_unit, update_context_unit
  - store_response, get_response
  - store_lineage, query_lineage methods
  - store_feedback, get_feedback methods

- [X] **T007** [Foundation] Implement SQLite schema initialization in `src/context/storage.rs`:
  - Create responses table with indexes
  - Create context_units table with indexes
  - Create lineage junction table with foreign keys
  - Create feedback table with indexes
  - WAL mode configuration for concurrency
  - Retention policy setup (90 days default)

- [X] **T008** [P] [Foundation] Implement DashMap cache layer in `src/context/storage.rs`:
  - Create HybridStorage struct with Arc<DashMap> + SQLite connection pool
  - Implement hot cache for recent responses (7 days / 10K items)
  - Cache eviction policy (LRU or time-based)
  - Cache hit/miss tracking

- [X] **T009** [Foundation] Implement hybrid storage backend in `src/context/storage.rs`:
  - Write operations: DashMap + async SQLite write
  - Read operations: Check DashMap → fallback to SQLite
  - Concurrent access handling (Arc wrapper)
  - Connection pooling for SQLite

- [X] **T010** [P] [Foundation] Implement weight calculation in `src/context/tracker.rs`:
  - Multi-factor composite scoring algorithm
  - Retrieval score factor (0.4 weight)
  - Recency factor (0.3 weight)
  - Type weight factor (0.2 weight)
  - Length factor (0.1 weight)
  - Normalization to ensure sum = 1.0

**Checkpoint**: ✅ Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Trace Response Origins (Priority: P1) 🎯 MVP

**Goal**: Enable developers to view complete lineage manifests showing which context units influenced a specific AI response

**Independent Test**: Generate an AI response, call `GET /api/trace/{response_id}`, verify lineage manifest shows all contributing context units with accurate weights summing to 1.0

### Implementation for User Story 1

- [X] **T011** [P] [US1] Implement ContextTracker struct in `src/context/tracker.rs`:
  - start_response() method - generates response UUID, initializes tracking
  - add_context() method - registers context unit for response
  - finalize_response() method - calculates weights, generates manifest, persists
  - Integration with storage layer

- [X] **T012** [P] [US1] Implement manifest generation in `src/context/tracker.rs`:
  - Build LineageManifest JSON from tracked contexts
  - Create provenance tree with edges
  - Validate manifest size < 5KB
  - Serialize to JSON for storage

- [X] **T013** [US1] Implement trace retrieval endpoint `GET /api/trace/{response_id}` in `src/web/api.rs`:
  - Parse response_id path parameter
  - Query storage for lineage manifest
  - Support format parameter (json, tree, compact)
  - Return 404 if response not found
  - Return 200 with LineageManifest JSON

- [X] **T014** [US1] Add format rendering utilities in `src/context/query.rs`:
  - format_as_json() - return raw manifest
  - format_as_tree() - hierarchical ASCII tree visualization
  - format_as_compact() - summary with key stats

- [X] **T015** [US1] Add trace retrieval route to warp router in `src/web/mod.rs`:
  - Register /api/trace/:response_id route
  - Wire to handler function
  - Add CORS headers if needed

- [X] **T016** [US1] Add configuration for context tracing in config schema `src/config/schema.rs`:
  - ContextTracingConfig struct
  - enabled: bool (default true)
  - storage_type: enum (Hybrid, SQLite)
  - sqlite_path: PathBuf
  - cache_size: usize (default 10000)
  - retention_days: u32 (default 90)

- [X] **T017** [US1] Update AppState in `src/state/mod.rs` to include ContextTracker:
  - Add context_tracker: Arc<ContextTracker> field
  - Initialize in state creation
  - Pass storage backend reference

**Checkpoint**: At this point, User Story 1 should be fully functional - developers can retrieve complete trace information for any response

---

## Phase 4: User Story 2 - Query Context Impact (Priority: P2)

**Goal**: Enable administrators to find all responses influenced by a specific context unit to assess the impact of outdated information

**Independent Test**: Store multiple responses using the same context unit, call `GET /api/query/by-context/{context_id}`, verify all responses are returned with correct contribution scores

### Implementation for User Story 2

- [X] **T018** [P] [US2] Implement QueryService struct in `src/context/query.rs`:
  - query_responses_by_context() method - find responses using a context unit
  - query_contexts_by_response() method - find contexts in a response
  - Filter support (min_weight, start_date, end_date, type)
  - Pagination support (limit parameter)

- [X] **T019** [US2] Implement bidirectional query methods in `src/context/storage.rs`:
  - get_responses_for_context() with SQLite recursive CTE
  - get_contexts_for_response() with JOIN query
  - Index utilization for performance (< 5 seconds for 100K responses)
  - Result sorting by contribution weight

- [X] **T020** [US2] Implement context impact endpoint `GET /api/query/by-context/{context_unit_id}` in `src/web/api.rs`:
  - Parse context_unit_id path parameter
  - Parse query parameters (limit, min_weight, start_date, end_date)
  - Call QueryService::query_responses_by_context()
  - Return ContextImpactReport with total count, avg weight, response list
  - Return 404 if context unit not found

- [X] **T021** [US2] Implement response contexts endpoint `GET /api/query/by-response/{response_id}/contexts` in `src/web/api.rs`:
  - Parse response_id path parameter
  - Parse type filter query parameter (optional)
  - Call QueryService::query_contexts_by_response()
  - Return array of ContextTreeNode objects
  - Return 404 if response not found

- [X] **T022** [US2] Add query route handlers to warp router in `src/web/mod.rs`:
  - Register /api/query/by-context/:context_unit_id route
  - Register /api/query/by-response/:response_id/contexts route
  - Wire to handler functions

- [X] **T023** [US2] Implement ContextImpactReport type in `src/context/types.rs`:
  - context_unit_id: String
  - total_responses: usize
  - avg_weight: f32
  - responses: Vec<ResponseSummary>
  - Serialization support

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - developers can trace responses AND administrators can assess context impact

---

## Phase 5: User Story 3 - Track Context Evolution (Priority: P3)

**Goal**: Enable knowledge managers to understand how context units evolve over time through versioning

**Independent Test**: Create multiple versions of a context unit, call `GET /api/query/evolution/{context_unit_id}`, verify version history shows all updates with timestamps

### Implementation for User Story 3

- [X] **T024** [P] [US3] Implement context versioning in `src/context/evolution.rs`:
  - create_context_version() method - creates new version with previous_version_id link
  - get_version_history() method - traverses version chain
  - compare_versions() method - shows differences between versions
  - Version validation (monotonic increase)

- [X] **T025** [US3] Extend storage layer with version queries in `src/context/storage.rs`:
  - get_context_version_chain() - recursive query following previous_version_id
  - get_latest_version() - find head of version chain
  - get_version_at_timestamp() - find version active at specific time
  - Index on previous_version_id for traversal performance

- [X] **T026** [US3] Implement evolution endpoint `GET /api/query/evolution/{context_unit_id}` in `src/web/api.rs`:
  - Parse context_unit_id path parameter (any version)
  - Call evolution::get_version_history()
  - Return EvolutionHistory with current_version and history array
  - Sort by version descending (newest first)
  - Return 404 if context unit not found

- [X] **T027** [US3] Implement EvolutionHistory type in `src/context/types.rs`:
  - current_version: ContextVersion
  - history: Vec<ContextVersion>
  - Serialization support

- [X] **T028** [US3] Implement ContextVersion type in `src/context/types.rs`:
  - id: String
  - version: i32
  - timestamp: DateTime<Utc>
  - summary: Option<String>
  - Serialization support

- [X] **T029** [US3] Add evolution route to warp router in `src/web/mod.rs`:
  - Register /api/query/evolution/:context_unit_id route
  - Wire to handler function

**Checkpoint**: All three user stories should now be independently functional - trace, query impact, and track evolution

---

## Phase 6: User Story 4 - Improve Context Quality Through Feedback (Priority: P4)

**Goal**: Enable model operators to provide feedback on responses that propagates to context units for continuous improvement

**Independent Test**: Submit feedback via `POST /api/feedback`, verify feedback score is applied to all contributing context units and aggregate scores are updated correctly

### Implementation for User Story 4

- [X] **T030** [P] [US4] Implement feedback recording in `src/context/tracker.rs`:
  - record_feedback() method - stores feedback record
  - propagate_feedback() method - updates context unit aggregate scores
  - Aggregate score calculation: (old_score × old_count + new_score × weight) / (old_count + 1)
  - Flag contexts below threshold (-0.5) as deprecated

- [X] **T031** [US4] Extend storage layer with feedback methods in `src/context/storage.rs`:
  - store_feedback() - insert feedback record with foreign key to response
  - get_feedback_for_response() - retrieve all feedback for a response
  - update_context_aggregate_score() - atomic update of context unit scores
  - get_deprecated_contexts() - query contexts below threshold

- [X] **T032** [US4] Implement feedback endpoint `POST /api/feedback` in `src/web/api.rs`:
  - Parse JSON body (FeedbackSubmission)
  - Validate score in range [-1.0, 1.0]
  - Verify response exists
  - Call tracker::record_feedback()
  - Call tracker::propagate_feedback()
  - Return 201 with FeedbackRecord including propagation_status
  - Return 400 for invalid data, 404 for response not found

- [X] **T033** [US4] Implement quality stats endpoint `GET /api/stats/context-quality` in `src/web/api.rs`:
  - Parse query parameters (type, min_score, limit)
  - Query context units with aggregate scores
  - Calculate avg_score across all contexts
  - Return statistics with top/bottom performers
  - Support filtering by ContextType

- [X] **T034** [US4] Implement FeedbackSubmission and propagation types in `src/context/types.rs`:
  - FeedbackSubmission (request body)
  - FeedbackPropagationStatus (contexts_updated, avg_score_change)
  - Validation and serialization support

- [X] **T035** [US4] Add feedback routes to warp router in `src/web/mod.rs`:
  - Register POST /api/feedback route
  - Register GET /api/stats/context-quality route
  - Wire to handler functions

- [X] **T036** [US4] Implement deprecation flagging logic in `src/context/evolution.rs`:
  - check_deprecation_threshold() method
  - mark_as_deprecated() method
  - get_deprecated_contexts_report() method
  - Integration with feedback propagation

**Checkpoint**: All four user stories should now be independently functional - complete context tracing, querying, evolution, and quality feedback

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] **T037** [P] Add error handling and custom error types in `src/context/error.rs`:
  - ContextError enum (NotFound, InvalidWeight, StorageError, ValidationError)
  - From implementations for rusqlite::Error, serde_json::Error
  - Into warp::Rejection implementations

- [X] **T038** [P] Add logging throughout context module:
  - Debug logs for all storage operations
  - Info logs for API endpoint calls
  - Warn logs for validation failures
  - Error logs for storage failures
  - Performance metrics logging (query times, manifest sizes)

- [X] **T039** [P] Add retention policy background task in `src/context/storage.rs`:
  - Spawn tokio task to run daily
  - Delete responses older than configured retention period
  - Cascade delete lineage and feedback records
  - Log deletion statistics

- [X] **T040** [P] Implement configuration validation in `src/config/schema.rs`:
  - Validate sqlite_path is writable
  - Validate cache_size > 0
  - Validate retention_days > 0
  - Return clear error messages for invalid config

- [X] **T041** Add performance monitoring in `src/context/storage.rs`:
  - Track cache hit rate
  - Track query execution times
  - Track manifest generation times
  - Expose metrics via /api/stats/cache endpoint

- [X] **T042** [P] Add comprehensive documentation comments:
  - Module-level docs for src/context/mod.rs
  - Public API docs for all exported types
  - Example usage in doc comments
  - Reference to quickstart.md

- [X] **T043** Update quickstart.md validation:
  - Verify all code examples compile
  - Test API endpoint examples with curl
  - Validate configuration examples
  - Check troubleshooting scenarios

- [X] **T044** Code cleanup and refactoring:
  - Extract common validation logic
  - Remove any TODO/FIXME comments
  - Ensure consistent error handling patterns
  - Run cargo fmt and cargo clippy

- [X] **T045** Security review:
  - SQL injection prevention (use parameterized queries)
  - Input validation on all API endpoints
  - Rate limiting considerations for feedback endpoint
  - Access control (if authentication exists)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phases 3-6)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3 → P4)
- **Polish (Phase 7)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - No dependencies on US1 (independently testable)
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - No dependencies on US1/US2 (independently testable)
- **User Story 4 (P4)**: Can start after Foundational (Phase 2) - Uses US1 storage but independently testable

### Within Each User Story

- Core types before services (types.rs → tracker.rs → query.rs)
- Storage layer before business logic
- Business logic before API endpoints
- API endpoints before route registration
- All tasks within a story can complete independently of other stories

### Parallel Opportunities

**Setup Phase (Phase 1)**:
- T002 and T003 can run in parallel (different files)

**Foundational Phase (Phase 2)**:
- T004 and T005 can run together (same file, related work)
- T008 can run parallel to T004/T005 (different concerns)
- T006 must complete before T007/T009 (interface before implementation)
- T010 can run parallel to T006-T009 (different file)

**User Story 1 (Phase 3)**:
- T011 and T012 can run together (same file, related)
- T014 can run parallel to T011/T012 (different file)
- T016 can run parallel to T011-T014 (different file)
- T013 depends on T011 completing
- T015 depends on T013 completing
- T017 depends on T011-T016 completing

**User Story 2 (Phase 4)**:
- T018 and T019 can run together (related query logic)
- T023 can run parallel to T018/T019 (just type definition)
- T020 and T021 can run parallel (different endpoints)
- T022 depends on T020/T021 completing

**User Story 3 (Phase 5)**:
- T024 and T025 can run together
- T027 and T028 can run parallel to T024/T025 (just types)
- T026 depends on T024/T025
- T029 depends on T026

**User Story 4 (Phase 6)**:
- T030 and T031 can run together
- T034 can run parallel to T030/T031 (just types)
- T032 and T033 can run parallel (different endpoints)
- T035 depends on T032/T033
- T036 can run parallel to T032-T035

**Polish Phase (Phase 7)**:
- T037, T038, T040, T042 can all run in parallel (different files/concerns)
- T039 depends on storage layer being complete
- T041 depends on storage layer being complete
- T043, T044, T045 should be done last (validation/cleanup)

---

## Parallel Example: User Story 1

```bash
# Foundation complete, now starting US1

# Launch parallel tasks first:
Task T011+T012: "Implement ContextTracker and manifest generation in src/context/tracker.rs"
Task T014: "Implement format rendering utilities in src/context/query.rs"
Task T016: "Add configuration in src/config/schema.rs"

# After T011 completes:
Task T013: "Implement trace retrieval endpoint in src/web/api.rs"

# After T013 completes:
Task T015: "Add route to warp router in src/web/mod.rs"

# After all above complete:
Task T017: "Update AppState to include ContextTracker"
```

---

## Parallel Example: User Story 2

```bash
# US1 complete, now starting US2 in parallel

# Launch parallel tasks:
Task T018+T019: "Implement QueryService and storage queries"
Task T023: "Implement ContextImpactReport type"

# After T018/T019 complete, launch parallel endpoints:
Task T020: "Implement by-context endpoint"
Task T021: "Implement by-response endpoint"

# After T020/T021 complete:
Task T022: "Add routes to router"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. ✅ Complete Phase 1: Setup (T001-T003)
2. ✅ Complete Phase 2: Foundational (T004-T010) - CRITICAL blocking phase
3. ✅ Complete Phase 3: User Story 1 (T011-T017)
4. **STOP and VALIDATE**:
   - Test trace retrieval with real responses
   - Verify weight calculations sum to 1.0
   - Validate manifest JSON structure
   - Check performance (< 2 second retrieval)
5. Deploy/demo if ready - **You now have a working MVP!**

### Incremental Delivery

1. **Foundation** (Phases 1-2) → Core infrastructure ready
2. **MVP** (Phase 3 - US1) → Test independently → Deploy/Demo ✅
3. **Query Impact** (Phase 4 - US2) → Test independently → Deploy/Demo
4. **Evolution Tracking** (Phase 5 - US3) → Test independently → Deploy/Demo
5. **Feedback System** (Phase 6 - US4) → Test independently → Deploy/Demo
6. **Polish** (Phase 7) → Final quality pass
7. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. **Team completes Foundational together** (Phases 1-2)
2. **Once Foundational is done, stories proceed in parallel**:
   - Developer A: User Story 1 (Phase 3) - 7 tasks
   - Developer B: User Story 2 (Phase 4) - 6 tasks
   - Developer C: User Story 3 (Phase 5) - 6 tasks
   - Developer D: User Story 4 (Phase 6) - 7 tasks
3. **Stories complete and integrate independently**
4. **Team converges on Polish** (Phase 7)

---

## Task Count Summary

- **Phase 1 (Setup)**: 3 tasks
- **Phase 2 (Foundational)**: 7 tasks ⚠️ CRITICAL PATH
- **Phase 3 (User Story 1 - P1)**: 7 tasks 🎯 MVP
- **Phase 4 (User Story 2 - P2)**: 6 tasks
- **Phase 5 (User Story 3 - P3)**: 6 tasks
- **Phase 6 (User Story 4 - P4)**: 7 tasks
- **Phase 7 (Polish)**: 9 tasks

**Total**: 45 tasks

### Per User Story

- **US1 (Trace Response Origins)**: 7 tasks - MVP scope
- **US2 (Query Context Impact)**: 6 tasks
- **US3 (Track Context Evolution)**: 6 tasks
- **US4 (Improve Context Quality)**: 7 tasks

### Parallel Opportunities

- **Setup**: 2 parallel groups
- **Foundational**: 3-4 parallel groups (within phase)
- **User Stories**: 4 stories can run completely in parallel after Foundational
- **Polish**: 6-7 parallel tasks

---

## Notes

- [P] tasks = different files or independent concerns, can run in parallel
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Foundational phase is CRITICAL - must complete before any user story work
- MVP scope is User Story 1 only (7 implementation tasks after foundation)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Run `cargo fmt` and `cargo clippy` frequently during development
- Avoid: vague tasks, same-file conflicts, cross-story dependencies that break independence
