# Implementation Plan: Aggregator Plugin Type

**Branch**: `004-aggregator-plugin-type` | **Date**: 2025-10-13 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-aggregator-plugin-type/spec.md`

## Summary

Implement an aggregator plugin system that allows LLM agents to query multiple MCP servers (context7, serena, etc.) through a single tool call, receiving ranked and deduplicated results optimized for context efficiency. The aggregator will query configured servers concurrently, rank results by relevance using heuristic scoring, combine and deduplicate responses, and return a prioritized subset that reduces context waste by 40-60% while preserving high-quality information.

**Technical Approach**: Create a new JavaScript plugin (aggregator-plugin) using the Claude Agent SDK (@anthropic-ai/agent npm module). When invoked via the `mcp__proxy__aggregator__context_aggregator` tool, the plugin initializes Claude Agent with configured MCP servers (context7, serena, etc.) registered as tools. Claude Agent SDK handles MCP server communication via stdio, intelligently selects which tools/servers to query based on the user's prompt, aggregates results, and returns optimized context. The Rust proxy manages plugin lifecycle and passes MCP server configurations to the plugin.

**Key Architecture**: JavaScript plugin + Claude Agent SDK pattern (NOT Rust-native). This leverages:
- Claude Agent SDK's built-in MCP client support (stdio protocol)
- SDK's intelligent tool selection and orchestration
- Existing JavaScript plugin infrastructure (process pool, timeout management)
- MCP server configs passed from Rust to JavaScript via plugin metadata

## Technical Context

**Language/Version**:
- Rust 1.75+ (proxy infrastructure)
- Node.js 18+ (JavaScript plugin runtime)
- TypeScript/JavaScript (plugin implementation)

**Primary Dependencies**:
- **Rust**: tokio 1.40, serde 1.0, serde_json 1.0 (plugin manager integration)
- **JavaScript**: @anthropic-ai/agent (Claude Agent SDK), @modelcontextprotocol/sdk (MCP client)
- **Existing**: Plugin process pool, plugin schema types

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
├── plugins/                   # EXISTING: JavaScript plugins directory
│   └── official/
│       └── aggregator-plugin/ # NEW: JavaScript plugin for context aggregation
│           ├── package.json   # Dependencies: @anthropic-ai/agent, @modelcontextprotocol/sdk
│           ├── index.js       # Main plugin entry point
│           ├── agent.js       # Claude Agent SDK initialization
│           └── mcp-client.js  # MCP stdio client setup
├── proxy/                     # EXISTING: Proxy logic
│   ├── aggregator_tools.rs    # NEW: Tool registration and metadata prep
│   └── handler.rs             # MODIFIED: Register mcp__proxy__aggregator__* tools
├── plugin/                    # EXISTING: Plugin manager
│   └── schema.rs              # MODIFIED: Add MCP server config to PluginMetadata
└── config/                    # EXISTING: Configuration
    └── schema.rs              # MODIFIED: Add aggregator plugin config

tests/
├── plugin_aggregator_test.rs  # Integration test for aggregator plugin
└── unit/
    └── aggregator_tools_tests.rs
```

**Structure Decision**: Using **JavaScript Plugin** pattern (like curation-plugin, security-plugin). Aggregator plugin receives MCP server configurations from Rust, spawns stdio connections to servers, registers them with Claude Agent SDK, and lets Claude orchestrate tool usage.

**Key Integration Points**:
1. **Plugin Manager** (`src/plugin/manager.rs`): Invokes aggregator plugin when `mcp__proxy__aggregator__context_aggregator` tool is called
2. **Proxy Handler** (`src/proxy/handler.rs`): Registers aggregator tool, routes calls to plugin manager
3. **Plugin Metadata** (`src/plugin/schema.rs`): Passes MCP server configs (command, args, env) to JavaScript plugin
4. **Configuration** (`src/config/schema.rs`): Aggregator plugin config with server list, timeout, system prompt
5. **Claude Agent SDK** (JavaScript): Handles MCP stdio connections, tool discovery, query orchestration

**Data Flow**:
```
1. LLM calls mcp__proxy__aggregator__context_aggregator(query: "...")
2. Rust proxy → Plugin Manager → spawn aggregator-plugin process
3. Plugin receives: { rawContent: query, metadata: { mcpServers: [...] } }
4. Plugin creates Claude Agent with MCP servers as tools
5. Claude Agent SDK queries MCP servers via stdio as needed
6. Plugin returns optimized context
7. Rust proxy returns result to LLM
```

## Complexity Tracking

*No constitution violations - all design choices follow project conventions and are justified by requirements.*
