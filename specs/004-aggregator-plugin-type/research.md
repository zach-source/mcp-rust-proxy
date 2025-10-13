# Research: Aggregator Plugin Type

**Feature**: Aggregator Plugin for Multi-Server Query Ranking
**Branch**: `004-aggregator-plugin-type`
**Date**: 2025-10-13

## Overview

This document consolidates research findings for implementing an aggregator plugin that queries multiple MCP servers concurrently, ranks results by relevance, and returns optimized responses to reduce LLM context waste.

---

## Key Research Findings

### 1. Ranking Algorithm Selection and Anthropic API Integration

**Decision**: Use Anthropic API (Claude) for semantic ranking and context optimization via JavaScript plugin approach.

**Rationale**:
1. **Quality**: Semantic understanding via Claude provides superior relevance ranking vs heuristics
2. **Existing Infrastructure**: Proxy already has JavaScript plugin system with Anthropic SDK integration (curation-plugin)
3. **Token Efficiency**: Claude can summarize/optimize results to reduce context waste beyond simple ranking
4. **Reusability**: Leverages existing plugin architecture and API key management
5. **Prompt Engineering**: Can use system prompts to guide aggregation behavior

**Implementation Approach - JavaScript Plugin with Claude Agent SDK**:

The aggregator is a **JavaScript plugin** (like curation-plugin, security-plugin) that uses the Claude Agent SDK (@anthropic-ai/agent npm module). The flow is:

1. **Tool Call**: LLM agent calls `mcp__proxy__aggregator__context_aggregator`
2. **Plugin Invocation**: Rust proxy invokes JavaScript aggregator plugin
3. **MCP Registration**: Plugin registers MCP servers (context7, serena) as tools for Claude Agent
4. **Claude Processing**: Claude Agent SDK receives user query, has access to MCP tools, builds optimized context
5. **Result Return**: Plugin returns Claude's aggregated context to Rust proxy, which forwards to LLM agent

**Alternatives Considered**:
- **Alternative 1**: Rust-native orchestrator + JavaScript ranking
  - **Rejected**: Duplicates work - Claude Agent SDK already handles MCP tool orchestration
- **Alternative 2**: Pure Rust with Anthropic HTTP API
  - **Rejected**: Would need to reimplement Claude Agent SDK's MCP integration in Rust
- **Alternative 3**: Pure heuristic scoring without Claude
  - **Rejected**: Doesn't leverage Claude's semantic understanding for quality aggregation

**Architecture**:
```
LLM Agent
  ↓ calls mcp__proxy__aggregator__context_aggregator(query)
Rust Proxy (plugin manager)
  ↓ invokes JavaScript plugin with query
JavaScript Aggregator Plugin
  ↓ initializes Claude Agent SDK
  ↓ registers MCP servers as tools (context7, serena via stdio)
Claude Agent (via SDK)
  ↓ decides which MCP tools to call
  ↓ queries MCP servers as needed
  ↓ aggregates and optimizes context
JavaScript Plugin
  ↓ returns aggregated context
Rust Proxy
  ↓ returns result to LLM agent
```

**Key Benefit**: Leverages Claude Agent SDK's built-in MCP support instead of reimplementing server orchestration.

**API Key Management**: Reuse existing ANTHROPIC_API_KEY from plugin configuration

---

### 2. MCP Server Access Pattern for JavaScript Plugin

**Decision**: JavaScript plugin creates stdio MCP client connections to configured servers, registering them as tools for Claude Agent SDK.

**Rationale**:
1. **Claude Agent SDK Integration**: SDK natively supports MCP servers as tools via stdio protocol
2. **Existing Pattern**: Matches how curation-plugin would integrate with MCP servers
3. **Server Discovery**: Plugin receives list of MCP server configurations (command, args, env) from Rust proxy
4. **Stdio Protocol**: Each MCP server (context7, serena) spawns as child process, communicates via stdio
5. **SDK Orchestration**: Claude Agent SDK handles MCP initialization, tool discovery, and query routing

**Alternatives Considered**:
- **Alternative 1**: JavaScript plugin calls back to Rust to query MCP servers
  - **Rejected**: Complex IPC, breaks Claude Agent SDK's native MCP support
- **Alternative 2**: Rust pre-queries servers, passes results to JS for ranking only
  - **Rejected**: Loses Claude Agent SDK's intelligent tool selection (Claude decides which servers to query)
- **Alternative 3**: Reuse proxy's existing connections via shared memory
  - **Rejected**: Impossible - stdio connections can't be shared across processes

**Implementation Pattern** (JavaScript plugin):
```javascript
import { Agent } from '@anthropic-ai/agent';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

// Plugin receives MCP server configs from Rust
const servers = input.metadata.mcpServers; // e.g., [{name: "context7", command: "/path", args: []}]

// Create stdio clients for each MCP server
const mcpClients = servers.map(server => ({
  name: server.name,
  transport: new StdioClientTransport({
    command: server.command,
    args: server.args,
    env: server.env
  })
}));

// Initialize Claude Agent with MCP servers as tools
const agent = new Agent({
  apiKey: process.env.ANTHROPIC_API_KEY,
  mcpServers: mcpClients
});

// Agent now has access to all MCP server tools
const result = await agent.run(input.rawContent);  // User query
return { text: result };  // Aggregated, optimized context
```

**Critical Note**: Plugin spawns **separate** stdio connections to MCP servers (not reusing proxy's connections). This is necessary because:
- JavaScript plugin runs in separate Node.js process
- Claude Agent SDK manages its own MCP client lifecycle
- Allows SDK to handle protocol negotiation and tool discovery automatically

---

### 3. Deduplication Strategy

**Decision**: Use content hash comparison for exact duplicates, Levenshtein distance for near-duplicates (similarity > 80%).

**Rationale**:
1. **Exact Duplicates**: Common when multiple servers index the same documentation
2. **Near Duplicates**: Slight formatting differences shouldn't count as unique results
3. **Tunable Threshold**: 80% similarity is configurable based on use case
4. **Moderate Cost**: Pairwise comparison is O(n²) but acceptable for < 1000 results

**Alternatives Considered**:
- **Alternative 1**: Hash-only deduplication
  - **Rejected**: Misses near-duplicates with minor formatting differences
- **Alternative 2**: MinHash or LSH
  - **Rejected**: Over-engineered for MVP, adds dependency complexity
- **Alternative 3**: No deduplication
  - **Rejected**: Wastes context on redundant information

---

### 4. Result Size Limiting

**Decision**: Limit to top 30 results by default, configurable per query (10-100 range).

**Rationale**:
1. **Context Budget**: 30 results ≈ 3000-6000 tokens (reasonable for most queries)
2. **Quality Focus**: Top 30 captures high-relevance results, diminishing returns after that
3. **Configurable**: Users can request more/less based on query complexity
4. **40-60% Reduction**: Typical reduction from 50-100 raw results to 30 ranked results

**Calculation**:
- Typical server returns 10-20 results
- 10 servers × 15 results = 150 total
- After dedup: ~100 unique results
- Top 30 = 70% reduction ✅ (meets 40-60% target)

---

### 5. Server Selection Logic

**Decision**: Use regex pattern matching on query text to determine which servers to query.

**Rationale**:
1. **Simple**: Pattern-based rules are easy to configure and understand
2. **Fast**: Regex matching is sub-millisecond for typical queries
3. **Flexible**: Can add new patterns without code changes
4. **Fallback**: Queries all servers if no pattern matches

**Alternatives Considered**:
- **Alternative 1**: Query all servers always
  - **Rejected**: Wastes time on irrelevant servers (e.g., querying browser automation for code questions)
- **Alternative 2**: ML-based query classification
  - **Rejected**: Adds complexity and latency for marginal benefit

**Configuration Example**:
```yaml
aggregator:
  serverRules:
    - pattern: "documentation|docs|library|package"
      servers: ["context7"]
    - pattern: "code|function|class|method"
      servers: ["serena"]
    - pattern: ".*"  # fallback
      servers: ["context7", "serena", "memory"]
```

---

### 6. Error Handling Strategy

**Decision**: Fail-fast per server, graceful degradation for overall aggregation.

**Rationale**:
1. **Individual Timeout**: 3-second timeout per server prevents blocking
2. **Collect Successes**: Return aggregated results from servers that succeeded
3. **Log Failures**: Warn about failed servers but don't fail entire aggregation
4. **Minimum Threshold**: Require at least 1 server to succeed (else return error)

**Implementation**:
```rust
// Timeout per server
tokio::time::timeout(Duration::from_secs(3), query_server(server)).await

// Aggregate successes
for result in results {
    match result {
        Ok(server_result) => aggregated.push(server_result),
        Err(e) => tracing::warn!("Server {} failed: {}", server, e),
    }
}

if aggregated.is_empty() {
    return Err(AggregatorError::AllServersFailed);
}
```

---

## Implementation Decisions Summary

| Decision Area | Choice | Confidence |
|--------------|--------|------------|
| Ranking Algorithm | Composite heuristic scoring | High |
| Concurrency | tokio::spawn with join_all | High |
| Deduplication | Hash + Levenshtein distance | Medium |
| Result Limit | Top 30 results (configurable) | High |
| Server Selection | Regex pattern matching | High |
| Error Handling | Graceful degradation | High |

---

## Open Questions

None - all technical decisions resolved during research phase.

---

## References

- MCP Specification: https://modelcontextprotocol.io/specification/
- Existing plugin system: `src/plugin/` (security-plugin, curation-plugin patterns)
- Connection pool: `src/transport/pool.rs` (for querying servers)
- Tool handlers: `src/proxy/tracing_tools.rs`, `src/proxy/server_tools.rs` (patterns to follow)
