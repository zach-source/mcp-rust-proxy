# Quickstart Guide: Aggregator Plugin

**Feature**: Aggregator Plugin Type
**Audience**: Developers and LLM agents using the aggregator

## Overview

The aggregator plugin enables LLM agents to query multiple MCP servers simultaneously and receive ranked, deduplicated results optimized for context efficiency.

---

## Quick Example

### Basic Usage (LLM Agent)

```javascript
// Call the aggregator tool with a query
const result = await callTool("mcp__proxy__aggregator__query", {
  query: "How do I use React hooks for state management?",
  maxResults: 20
});

// Result contains ranked information from multiple servers
console.log(result.metadata.serversQueried); // e.g., 2 (context7 + serena)
console.log(result.results.length); // e.g., 20 (top results)
console.log(result.results[0].server); // e.g., "context7"
console.log(result.results[0].relevanceScore); // e.g., 0.92
```

---

## Configuration

### Enable Aggregator in mcp-proxy-config.yaml

```yaml
aggregator:
  enabled: true
  defaultMaxResults: 30
  serverTimeoutSecs: 3
  totalTimeoutSecs: 5
  dedupThreshold: 0.8

  # Server selection rules (queries matched top-to-bottom)
  serverRules:
    - pattern: "documentation|docs|library|package|npm|pip"
      servers: ["context7"]

    - pattern: "code|function|class|method|implementation"
      servers: ["serena"]

    - pattern: ".*"  # fallback: query all
      servers: ["context7", "serena", "memory", "filesystem"]

  # Ranking weights (must sum to 1.0)
  rankingWeights:
    keywordMatch: 0.4      # How well result matches query keywords
    freshness: 0.3          # How recent the result is
    serverReputation: 0.2   # Server quality score
    lengthPenalty: 0.1      # Penalty for very long results
```

---

## Usage Examples

### Example 1: Documentation Query

```javascript
const result = await callTool("mcp__proxy__aggregator__query", {
  query: "TypeScript type guards documentation"
});

// Returns results from context7 (matched "documentation" pattern)
// Results ranked by relevance to "TypeScript type guards"
```

### Example 2: Code Analysis Query

```javascript
const result = await callTool("mcp__proxy__aggregator__query", {
  query: "Find implementations of authentication middleware",
  servers: ["serena", "filesystem"]  // Query specific servers
});

// Only queries serena and filesystem (overrides pattern matching)
```

### Example 3: Large Context Query

```javascript
const result = await callTool("mcp__proxy__aggregator__query", {
  query: "Complete guide to React performance optimization",
  maxResults: 50  // Request more results for comprehensive topic
});

// Returns top 50 results from all matching servers
// Metadata shows reduction: e.g., 120 raw → 80 dedup → 50 returned
```

---

## Understanding Results

### Result Structure

Each result includes:
- **content**: The actual information from the MCP server
- **server**: Which server provided it (e.g., "context7", "serena")
- **relevanceScore**: 0.0-1.0 indicating how well it matches the query
- **rank**: Position in ranked list (1 = most relevant)

### Metadata

- **serversQueried**: How many servers were asked
- **serversSucceeded**: How many responded (rest failed/timeout)
- **totalResultsRaw**: Results before deduplication
- **totalResultsDedup**: After removing duplicates
- **resultsReturned**: Final count (after limiting to maxResults)
- **processingTimeMs**: Total time taken
- **serverDiversity**: 0.0-1.0 indicating result source variety

---

## Troubleshooting

### No Results Returned

**Symptom**: `results` array is empty

**Possible Causes**:
1. All configured servers are down → Check server status
2. Query doesn't match any server rules → Check pattern matching
3. All results filtered out → Lower dedup threshold or increase maxResults

**Solution**: Check metadata.serversSucceeded - if 0, servers are unavailable

### Low Relevance Scores

**Symptom**: All results have scores < 0.5

**Possible Causes**:
1. Query keywords don't match result content
2. Results are very old (low freshness score)
3. Server reputation is low

**Solution**: Rephrase query with different keywords or increase maxResults to see more candidates

### Slow Response Times

**Symptom**: processingTimeMs > 5000 (5 seconds)

**Possible Causes**:
1. Too many servers configured (increase parallelism or reduce servers)
2. Servers responding slowly (check individual server health)
3. Large result sets (reduce maxResults or tune dedup)

**Solution**: Check which servers are slow in logs, consider excluding them

---

## Advanced Configuration

### Custom Server Reputation

```yaml
aggregator:
  serverReputation:
    context7: 0.9    # High quality documentation
    serena: 0.85     # Good code analysis
    memory: 0.7      # User-created content (variable quality)
    filesystem: 0.6  # Raw file content
```

### Query-Specific Timeouts

```yaml
aggregator:
  serverTimeouts:
    context7: 5      # Documentation queries can be slower
    serena: 10       # Code analysis is slow
    default: 3       # All other servers
```

---

## Integration with LLM Workflows

### Pattern: Pre-Query Aggregation

```
1. User asks question
2. LLM agent calls aggregator tool first
3. Aggregator returns ranked, relevant context
4. LLM agent uses aggregated context in response
5. Response quality improved, context waste reduced
```

### Pattern: Fallback to Individual Servers

```
1. Try aggregator first
2. If insufficient results, query specific server directly
3. Use aggregator metadata to decide which server to query
```

---

## Performance Expectations

- **Typical query**: 2-3 seconds (waiting for servers)
- **Fast query** (cached servers): 500ms-1s
- **Slow query** (10 servers, large results): 4-5 seconds
- **Context reduction**: 40-60% smaller than raw results
- **Quality improvement**: Top 30 results vs random 30 = measurable relevance increase

---

## Next Steps

After implementing aggregator:
1. Test with real queries against your MCP servers
2. Tune ranking weights based on result quality
3. Adjust server selection patterns for your use case
4. Monitor processingTimeMs and optimize slow paths
