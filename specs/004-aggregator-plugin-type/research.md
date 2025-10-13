# Research: Aggregator Plugin Type

**Feature**: Aggregator Plugin for Multi-Server Query Ranking
**Branch**: `004-aggregator-plugin-type`
**Date**: 2025-10-13

## Overview

This document consolidates research findings for implementing an aggregator plugin that queries multiple MCP servers concurrently, ranks results by relevance, and returns optimized responses to reduce LLM context waste.

---

## Key Research Findings

### 1. Ranking Algorithm Selection

**Decision**: Use composite heuristic scoring with keyword matching, result freshness, and server reputation.

**Rationale**:
1. **Performance**: Heuristic scoring is fast (< 50ms for 100 results) vs semantic/AI-based ranking
2. **No External Dependencies**: Doesn't require LLM API calls or embedding models
3. **Tunable**: Weights can be adjusted without retraining models
4. **Transparent**: Scoring logic is explainable and debuggable

**Alternatives Considered**:
- **Alternative 1**: Semantic similarity using embeddings
  - **Rejected**: Requires embedding model (adds latency, complexity, cost)
- **Alternative 2**: LLM-based relevance scoring
  - **Rejected**: Too slow (would exceed 5-second target), adds API dependencies
- **Alternative 3**: Simple keyword count only
  - **Rejected**: Too simplistic, doesn't account for result quality or freshness

**Implementation**:
```rust
score = (keyword_match_score * 0.4) + (freshness_score * 0.3) + (server_reputation * 0.2) + (result_length_penalty * 0.1)
```

---

### 2. Concurrent Query Strategy

**Decision**: Use tokio::spawn with timeout per server, collect results with futures::join_all.

**Rationale**:
1. **Parallelism**: Queries all servers simultaneously (reduces total time from sum to max)
2. **Timeout Handling**: Individual timeouts prevent slow servers from blocking aggregation
3. **Partial Success**: Can return results even if some servers fail
4. **Existing Pattern**: Matches proxy's forward_to_all_servers implementation

**Alternatives Considered**:
- **Alternative 1**: Sequential queries (one server at a time)
  - **Rejected**: Would take 10-50 seconds for 10 servers (exceeds 5-second target)
- **Alternative 2**: Stream results as they arrive
  - **Rejected**: Requires ranking after all results collected anyway (can't stream ranked results)

**Implementation Pattern** (from existing code):
```rust
let futures: Vec<_> = servers.iter().map(|server| {
    tokio::spawn(query_server(server, query.clone()))
}).collect();

let results = futures::join_all(futures).await;
```

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
