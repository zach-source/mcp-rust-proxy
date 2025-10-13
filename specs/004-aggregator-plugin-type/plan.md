# Implementation Plan: Aggregator Plugin Type

**Branch**: `004-aggregator-plugin-type` | **Date**: 2025-10-13 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-aggregator-plugin-type/spec.md`

## Summary

Implement an aggregator plugin system that allows LLM agents to query multiple MCP servers (context7, serena, etc.) through a single tool call, receiving ranked and deduplicated results optimized for context efficiency. The aggregator will query configured servers concurrently, rank results by relevance using heuristic scoring, combine and deduplicate responses, and return a prioritized subset that reduces context waste by 40-60% while preserving high-quality information.

**Technical Approach**: Create a new aggregator tool handler (Rust-native, not JavaScript plugin) that queries configured MCP servers through the proxy's existing stdio connection pool. The aggregator receives user queries, dispatches concurrent requests to selected MCP servers using the same connection pool that serves normal proxy requests, collects responses, applies heuristic ranking (keyword matching, result freshness, server reputation), deduplicates similar results, and returns top-N ranked items. Server selection and ranking configuration stored in existing config file format.

**Key Constraint**: The aggregator MUST reuse the proxy's existing stdio connections to MCP servers (context7, serena, etc.) rather than creating separate connections. This ensures protocol version compatibility is maintained and connection pooling benefits are preserved.

## Technical Context

**Language/Version**: Rust 1.75+

**Primary Dependencies**:
- tokio 1.40 (async runtime, concurrent server queries)
- serde 1.0, serde_json 1.0 (JSON serialization for MCP messages)
- dashmap 6.0 (concurrent state for aggregation tracking)
- regex 1.10 (keyword matching for ranking)
- futures 0.3 (concurrent query execution with join_all)

**Storage**: In-memory only (aggregation results not persisted, query-response lifecycle only)

**Testing**: cargo test (unit tests for ranking algorithms, integration tests with mock MCP servers)

**Target Platform**: Linux/macOS server (same as proxy core)

**Project Type**: Single backend service (extends existing mcp-rust-proxy)

**Performance Goals**:
- Query processing: < 5 seconds P90 (including MCP server round trips)
- Ranking algorithm: < 50ms for 100 results
- Concurrent server queries: Up to 10 servers in parallel
- Result aggregation: < 100ms for combining/dedup

**Constraints**:
- Must integrate with existing plugin system architecture
- Cannot block proxy's main request/response flow
- Aggregation must work with partial results (some servers failing)
- Memory footprint: < 10MB per active aggregation query

**Scale/Scope**:
- Support querying 3-10 MCP servers per aggregation
- Handle 10-100 results per server (1000 total before dedup)
- Reduce to 20-50 top results (40-60% reduction)
- Process 10-50 aggregation requests per minute

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Constitution Reference**: `.specify/memory/constitution.md` v1.1.0 (ratified 2025-10-12)

### Principle I: Performance First ✅

- Concurrent MCP server queries using tokio::spawn (parallel execution)
- Heuristic ranking (no heavy AI processing, < 50ms for 100 results)
- Result streaming to avoid buffering all responses
- Configurable limits to prevent unbounded aggregation

### Principle II: Flexibility Over Rigidity ✅

- Works with any MCP server (context7, serena, or custom servers)
- Configurable server selection per query type
- Pluggable ranking algorithms (extendable in future)
- Graceful degradation when servers unavailable

### Principle III: Comprehensive Testing ✅

- Unit tests for ranking algorithms
- Integration tests with mock MCP servers
- Failure mode testing (partial server failures)
- Performance benchmarks for aggregation

### Principle IV: Idiomatic Rust Patterns ✅

- AggregatorPlugin struct implementing plugin trait
- Result<T, AggregatorError> for fallible operations
- Async/await for concurrent server queries
- Strong typing for ranking scores and metadata

### Principle V: Structured Logging and Observability ✅

- Log each server query with timing
- Log ranking decisions (top results selected)
- Structured fields: query, servers_queried, results_count, processing_time
- Expose aggregation metrics via API

### Principle VI: Backward Compatibility ✅

- Extends existing plugin system (no breaking changes)
- New aggregator tool is opt-in (doesn't affect existing tools)
- Configuration is additive (backward compatible with current plugin config)

### Principle VII: Leverage Context7 and Serena ✅

- Feature itself enables better use of these servers!
- Will use Context7 for researching ranking algorithms
- Will use Serena for understanding existing plugin architecture

## Project Structure

### Documentation (this feature)

```
specs/004-aggregator-plugin-type/
├── spec.md              # Feature specification (already exists)
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (ranking algorithms, MCP query patterns)
├── data-model.md        # Phase 1 output (AggregatorQuery, ServerResult, RankingScore entities)
├── quickstart.md        # Phase 1 output (how to configure and use aggregator)
├── contracts/           # Phase 1 output
│   ├── aggregator-tool-api.md      # Tool interface contract
│   ├── ranking-algorithm.md        # Ranking behavior specification
│   └── server-query-protocol.md    # How aggregator queries MCP servers
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created yet)
```

### Source Code (repository root)

```
src/
├── aggregator/                # NEW: Aggregator system (Rust-native, not JavaScript plugin)
│   ├── mod.rs                 # Public API
│   ├── query.rs               # Query parsing and server selection
│   ├── ranking.rs             # Heuristic ranking algorithms
│   ├── dedup.rs               # Result deduplication
│   └── orchestrator.rs        # Main aggregation orchestration using connection pool
├── proxy/                     # EXISTING: Proxy logic
│   ├── aggregator_tools.rs    # NEW: Aggregator tool handler (like tracing_tools.rs)
│   └── handler.rs             # MODIFIED: Register aggregator tool in tools/call handler
└── config/                    # EXISTING: Configuration
    └── schema.rs              # MODIFIED: Add aggregator config section

tests/
├── unit/
│   └── aggregator/
│       ├── ranking_tests.rs
│       ├── dedup_tests.rs
│       └── query_tests.rs
└── integration/
    └── aggregator_integration_tests.rs
```

**Structure Decision**: Using **Single Project** structure (extends existing proxy). Aggregator is implemented as a Rust-native tool handler (NOT a JavaScript plugin) following the pattern of `tracing_tools.rs` and `server_tools.rs`. This allows direct access to the proxy's connection pool and state.

**Key Integration Points**:
1. **Connection Pool** (`src/transport/pool.rs`): Aggregator uses existing stdio connections via `pool.get(server_name)` to query MCP servers
2. **Proxy handler** (`src/proxy/handler.rs`): New aggregator tool registered in tools/call handler (pattern: `mcp__proxy__aggregator__*`)
3. **State** (`src/state/mod.rs`): Access to server list and connection states via AppState
4. **Configuration** (`src/config/schema.rs`): New aggregator section in config YAML
5. **Protocol Adapters**: Aggregator benefits from existing protocol version translation (queries work across all MCP versions)

## Complexity Tracking

*No constitution violations - all design choices follow project conventions and are justified by requirements.*
