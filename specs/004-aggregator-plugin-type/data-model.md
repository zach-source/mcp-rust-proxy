# Data Model: Aggregator Plugin Type

**Feature**: Aggregator Plugin for Multi-Server Query Ranking
**Branch**: `004-aggregator-plugin-type`
**Date**: 2025-10-13

## Overview

This document defines the key entities, data structures, and relationships for the aggregator plugin system.

---

## Core Entities

### 1. AggregatorQuery

**Purpose**: Represents a user query submitted to the aggregator tool for multi-server processing.

**Definition**:
```rust
pub struct AggregatorQuery {
    /// The user's original query/prompt
    pub query_text: String,

    /// Optional context limit (max results to return)
    pub max_results: Option<usize>,

    /// Optional server filter (specific servers to query)
    pub servers: Option<Vec<String>>,

    /// Query metadata
    pub request_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

**Validation Rules**:
- `query_text` must not be empty
- `max_results` must be between 10 and 100 if specified (default: 30)
- `servers` must contain valid server names if specified

---

### 2. ServerResult

**Purpose**: Represents the response from a single MCP server.

**Definition**:
```rust
pub struct ServerResult {
    /// Which server provided this result
    pub server_name: String,

    /// The result content (from MCP tool call)
    pub content: serde_json::Value,

    /// How long this server took to respond
    pub response_time: std::time::Duration,

    /// When this result was retrieved
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Optional error if server query failed
    pub error: Option<String>,
}
```

**Invariants**:
- If `error` is Some, `content` should be empty/null
- `response_time` should never exceed configured timeout (3 seconds)

---

### 3. RankedResult

**Purpose**: A server result with computed relevance score and ranking metadata.

**Definition**:
```rust
pub struct RankedResult {
    /// Original result from server
    pub server_result: ServerResult,

    /// Composite relevance score (0.0 to 1.0)
    pub relevance_score: f32,

    /// Breakdown of score components
    pub score_breakdown: ScoreBreakdown,

    /// Ranking position (1 = most relevant)
    pub rank: usize,
}

pub struct ScoreBreakdown {
    pub keyword_match: f32,    // 0.0-1.0
    pub freshness: f32,         // 0.0-1.0
    pub server_reputation: f32, // 0.0-1.0
    pub length_penalty: f32,    // 0.0-1.0
}
```

**Invariants**:
- `relevance_score` is always in range [0.0, 1.0]
- `rank` starts at 1 (not 0)
- Score components sum to `relevance_score` when weighted

---

### 4. AggregatedResponse

**Purpose**: The final response returned to the LLM agent containing ranked, deduplicated results.

**Definition**:
```rust
pub struct AggregatedResponse {
    /// Top N ranked results
    pub results: Vec<RankedResult>,

    /// Aggregation metadata
    pub metadata: AggregationMetadata,
}

pub struct AggregationMetadata {
    /// How many servers were queried
    pub servers_queried: usize,

    /// How many servers responded successfully
    pub servers_succeeded: usize,

    /// Total results before ranking/dedup
    pub total_results_raw: usize,

    /// Results after deduplication
    pub total_results_dedup: usize,

    /// Final result count returned
    pub results_returned: usize,

    /// Total processing time
    pub processing_time: std::time::Duration,

    /// Server diversity metric (unique servers contributing)
    pub server_diversity: f32, // 0.0-1.0
}
```

---

### 5. AggregatorConfig

**Purpose**: Configuration for aggregator behavior.

**Definition**:
```rust
pub struct AggregatorConfig {
    /// Default maximum results to return
    pub default_max_results: usize, // Default: 30

    /// Timeout for individual server queries
    pub server_timeout_secs: u64, // Default: 3

    /// Total aggregation timeout
    pub total_timeout_secs: u64, // Default: 5

    /// Deduplication similarity threshold
    pub dedup_threshold: f32, // Default: 0.8 (80% similar)

    /// Server selection rules
    pub server_rules: Vec<ServerRule>,

    /// Ranking weights
    pub ranking_weights: RankingWeights,
}

pub struct ServerRule {
    /// Regex pattern to match query
    pub pattern: String,

    /// Servers to query if pattern matches
    pub servers: Vec<String>,
}

pub struct RankingWeights {
    pub keyword_match: f32,    // Default: 0.4
    pub freshness: f32,         // Default: 0.3
    pub server_reputation: f32, // Default: 0.2
    pub length_penalty: f32,    // Default: 0.1
}
```

---

## Data Flow

### Aggregation Flow

```
1. LLM calls aggregator tool with query
2. Aggregator parses query, applies server selection rules
3. For each selected server:
   - Spawn async task
   - Query server with timeout
   - Collect result or error
4. Wait for all queries (or timeout)
5. Filter successful results
6. Rank results using composite scoring
7. Deduplicate similar results
8. Return top N results with metadata
```

### Ranking Flow

```
For each ServerResult:
1. Extract keywords from query
2. Count keyword matches in result content
3. Calculate freshness (time since result created)
4. Look up server reputation (from config or defaults)
5. Apply length penalty (very long results penalized)
6. Compute weighted sum = relevance_score
7. Sort by relevance_score descending
```

### Deduplication Flow

```
For each result pair (i, j where i < j):
1. Compute content hash
2. If hashes match: mark j as duplicate
3. Else: compute Levenshtein distance
4. If similarity > threshold: mark j as duplicate
5. Remove all duplicates, keeping highest-ranked instance
```

---

## Performance Characteristics

**Time Complexity**:
- Server queries: O(1) parallel time (max of all server times, ~2-3 seconds)
- Ranking: O(n log n) for sorting (where n = total results, ~100-1000)
- Deduplication: O(n²) pairwise comparison (optimized with early termination)

**Space Complexity**:
- Query: O(1) - single query struct
- Results: O(n) - stores all results before ranking
- Ranked: O(k) - only top k results kept

**Optimization Opportunities**:
- Early termination in dedup (stop after finding duplicate)
- Incremental ranking (rank as results arrive, not batch)
- Result streaming (future enhancement)

---

## Error Handling

```rust
pub enum AggregatorError {
    /// All configured servers failed to respond
    AllServersFailed {
        attempted: Vec<String>,
        errors: Vec<String>,
    },

    /// Query parsing or validation failed
    InvalidQuery {
        query: String,
        reason: String,
    },

    /// Timeout exceeded
    Timeout {
        elapsed: Duration,
        limit: Duration,
    },

    /// Configuration error
    ConfigError {
        field: String,
        reason: String,
    },
}
```

---

## Validation Rules

### Query Validation

1. **Query Text**:
   - Must not be empty
   - Max length: 10,000 characters
   - Must be valid UTF-8

2. **Max Results**:
   - If specified, must be 10-100
   - Default: 30

3. **Server Filter**:
   - Server names must exist in proxy configuration
   - Empty list treated as "all servers"

### Result Validation

1. **Relevance Score**:
   - Must be 0.0 to 1.0
   - Cannot be NaN or infinite

2. **Duplicate Detection**:
   - Similarity threshold: 0.0 to 1.0
   - Default: 0.8 (80% similar = duplicate)

---

## Storage and Persistence

**In-Memory Only** (no persistence):
- AggregatorQuery: Lives during request/response cycle
- ServerResults: Discarded after aggregation completes
- RankedResults: Returned to client, not stored
- AggregatorConfig: Loaded from YAML config at startup

**No Persistent Storage**:
- No query history
- No result caching (for MVP)
- No ranking model training data

---

## Testing Considerations

### Unit Test Targets

1. **Ranking Algorithm**:
   - Test keyword matching with various queries
   - Test freshness scoring with different timestamps
   - Test length penalty for short/long results
   - Verify score composition is correct

2. **Deduplication**:
   - Test exact duplicates removed
   - Test near-duplicates (80%+ similar) removed
   - Test distinct results preserved
   - Edge case: single result, all duplicates

3. **Server Selection**:
   - Test pattern matching for different query types
   - Test fallback to all servers
   - Test disabled servers excluded

### Integration Test Targets

1. Full aggregation with mock servers
2. Partial failure scenarios (some servers timeout)
3. Performance under load (10 servers, 100 results each)
4. Edge cases (all servers fail, all results identical)

---

## Configuration Example

```yaml
aggregator:
  enabled: true
  defaultMaxResults: 30
  serverTimeoutSecs: 3
  totalTimeoutSecs: 5
  dedupThreshold: 0.8

  serverRules:
    - pattern: "documentation|library|package|module"
      servers: ["context7"]
    - pattern: "code|class|function|method|variable"
      servers: ["serena"]
    - pattern: ".*"
      servers: ["context7", "serena", "memory"]

  rankingWeights:
    keywordMatch: 0.4
    freshness: 0.3
    serverReputation: 0.2
    lengthPenalty: 0.1
```
