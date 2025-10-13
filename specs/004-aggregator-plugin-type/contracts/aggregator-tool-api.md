# Contract: Aggregator Tool API

**Feature**: Aggregator Plugin Type
**Purpose**: Define the tool interface for the aggregator

---

## Tool Definition

**Tool Name**: `mcp__proxy__aggregator__query`

**Description**: Query multiple MCP servers concurrently and receive ranked, aggregated results optimized for LLM context efficiency.

---

## Input Schema

```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "The user prompt or question to search for across MCP servers",
      "minLength": 1,
      "maxLength": 10000
    },
    "maxResults": {
      "type": "integer",
      "description": "Maximum number of results to return (default: 30)",
      "minimum": 10,
      "maximum": 100,
      "default": 30
    },
    "servers": {
      "type": "array",
      "description": "Optional list of specific servers to query (default: all configured)",
      "items": {
        "type": "string"
      }
    }
  },
  "required": ["query"]
}
```

---

## Output Schema

```json
{
  "type": "object",
  "properties": {
    "results": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "content": {
            "type": "string",
            "description": "The result content from the MCP server"
          },
          "server": {
            "type": "string",
            "description": "Which MCP server provided this result"
          },
          "relevanceScore": {
            "type": "number",
            "description": "Relevance score (0.0 to 1.0)",
            "minimum": 0.0,
            "maximum": 1.0
          },
          "rank": {
            "type": "integer",
            "description": "Ranking position (1 = most relevant)",
            "minimum": 1
          },
          "timestamp": {
            "type": "string",
            "format": "date-time",
            "description": "When this result was retrieved"
          }
        },
        "required": ["content", "server", "relevanceScore", "rank"]
      }
    },
    "metadata": {
      "type": "object",
      "properties": {
        "serversQueried": {
          "type": "integer",
          "description": "Number of servers queried"
        },
        "serversSucceeded": {
          "type": "integer",
          "description": "Number of servers that responded successfully"
        },
        "totalResultsRaw": {
          "type": "integer",
          "description": "Total results before ranking/dedup"
        },
        "totalResultsDedup": {
          "type": "integer",
          "description": "Results after deduplication"
        },
        "resultsReturned": {
          "type": "integer",
          "description": "Final result count returned"
        },
        "processingTimeMs": {
          "type": "integer",
          "description": "Total processing time in milliseconds"
        },
        "serverDiversity": {
          "type": "number",
          "description": "Proportion of queried servers that contributed results",
          "minimum": 0.0,
          "maximum": 1.0
        }
      },
      "required": ["serversQueried", "serversSucceeded", "resultsReturned", "processingTimeMs"]
    }
  },
  "required": ["results", "metadata"]
}
```

---

## Example Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "mcp__proxy__aggregator__query",
    "arguments": {
      "query": "How do I implement React hooks for state management?",
      "maxResults": 20
    }
  }
}
```

---

## Example Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"results\":[{\"content\":\"React hooks documentation: useState allows...\",\"server\":\"context7\",\"relevanceScore\":0.92,\"rank\":1,\"timestamp\":\"2025-10-13T03:00:00Z\"},{\"content\":\"Code example from your project showing useState pattern...\",\"server\":\"serena\",\"relevanceScore\":0.87,\"rank\":2,\"timestamp\":\"2025-10-13T03:00:00Z\"}],\"metadata\":{\"serversQueried\":2,\"serversSucceeded\":2,\"totalResultsRaw\":15,\"totalResultsDedup\":12,\"resultsReturned\":2,\"processingTimeMs\":1234,\"serverDiversity\":1.0}}"
      }
    ]
  }
}
```

---

## Error Responses

### All Servers Failed

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32603,
    "message": "Aggregation failed: all servers unavailable",
    "data": {
      "attemptedServers": ["context7", "serena"],
      "errors": ["context7: timeout after 3s", "serena: connection refused"]
    }
  }
}
```

### Invalid Query

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid query parameters",
    "data": {
      "field": "maxResults",
      "reason": "Must be between 10 and 100, got 500"
    }
  }
}
```

---

## Behavioral Contracts

1. **Concurrent Execution**: All server queries MUST execute in parallel
2. **Timeout Enforcement**: Individual servers timing out MUST NOT block other servers
3. **Graceful Degradation**: If ≥1 server succeeds, return partial results (don't fail)
4. **Deterministic Ranking**: Same query + results MUST produce same ranking
5. **Source Attribution**: Every result MUST include server name
6. **Metadata Accuracy**: Metadata counts MUST match actual processing

---

## Performance Contracts

1. **P90 Latency**: 90% of requests complete within 5 seconds
2. **Server Timeout**: Individual servers timeout after 3 seconds max
3. **Ranking Speed**: Ranking 100 results takes < 50ms
4. **Memory**: < 10MB per active aggregation query
