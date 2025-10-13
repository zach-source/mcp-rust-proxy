# Implementation Plan: Aggregator Plugin Type

**Branch**: `004-aggregator-plugin-type` | **Date**: 2025-10-13 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-aggregator-plugin-type/spec.md`

## Summary

Implement an aggregator plugin system that allows LLM agents to query multiple MCP servers (context7, serena, etc.) through a single tool call, receiving ranked and deduplicated results optimized for context efficiency. The aggregator will query configured servers concurrently, rank results by relevance using heuristic scoring, combine and deduplicate responses, and return a prioritized subset that reduces context waste by 40-60% while preserving high-quality information.

**Technical Approach**: Extend the existing JavaScript plugin system with a new "aggregator" plugin type. Aggregator plugins receive user queries, dispatch concurrent requests to configured MCP servers via the proxy's connection pool, apply heuristic ranking (keyword matching, result freshness, server reputation), deduplicate similar results, and return top-N ranked items. Configuration stored in existing plugin config format, with server selection rules and ranking weights.

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
├── plugin/                    # EXISTING: Plugin system
│   ├── aggregator/            # NEW: Aggregator plugin implementation
│   │   ├── mod.rs
│   │   ├── query.rs           # Query parsing and server selection
│   │   ├── ranking.rs         # Heuristic ranking algorithms
│   │   ├── dedup.rs           # Result deduplication
│   │   └── aggregation.rs     # Main aggregation orchestration
│   ├── mod.rs                 # MODIFIED: Register aggregator plugin type
│   ├── chain.rs               # EXISTING: Plugin execution chain
│   └── schema.rs              # MODIFIED: Add AggregatorPlugin type
├── proxy/                     # EXISTING: Proxy logic
│   ├── aggregator_tools.rs    # NEW: Aggregator tool handler (like tracing_tools.rs)
│   └── handler.rs             # MODIFIED: Register aggregator tool
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

**Structure Decision**: Using **Single Project** structure (extends existing proxy). Aggregator is a new plugin type integrated into the existing plugin system architecture, following the pattern of security-plugin and curation-plugin but with multi-server query orchestration.

**Key Integration Points**:
1. **Plugin system**: New plugin type extends existing PluginPhase enum
2. **Proxy handler**: New aggregator tool handler (similar to tracing_tools, server_tools)
3. **Connection pool**: Reuse existing pool for querying backend servers
4. **Configuration**: Extends existing plugins section with aggregator-specific settings

## Complexity Tracking

*No constitution violations - all design choices follow project conventions and are justified by requirements.*
