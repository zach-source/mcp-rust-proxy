# Context Tracing Tools - Quick Reference for LLM Agents

## TL;DR
Submit feedback after successful responses to improve context quality over time.

## Quick Command

```bash
claude --mcp-config '{"mcpServers":{"proxy":{"command":"'$(pwd)'/target/debug/mcp-rust-proxy","args":["--config","'$(pwd)'/mcp-proxy-config.yaml","--stdio"]}}}'
```

## 5 Tools Available

### 1. Submit Feedback (Most Important)
**Tool:** `mcp__proxy__tracing__submit_feedback`

**Use after:** Every successful response to users

**Example:**
```json
{
  "name": "mcp__proxy__tracing__submit_feedback",
  "arguments": {
    "response_id": "resp_abc123",
    "score": 0.8,
    "feedback_text": "Accurate code that solved the problem"
  }
}
```

**Score Guide:**
- `1.0 to 0.7` = Excellent
- `0.6 to 0.3` = Good
- `0.2 to -0.2` = Neutral
- `-0.3 to -0.6` = Poor (errors)
- `-0.7 to -1.0` = Bad (very wrong)

### 2. Get Trace
**Tool:** `mcp__proxy__tracing__get_trace`

**Use when:** Debugging why a response was generated

**Example:**
```json
{
  "name": "mcp__proxy__tracing__get_trace",
  "arguments": {
    "response_id": "resp_abc123",
    "format": "tree"
  }
}
```

### 3. Query Context Impact
**Tool:** `mcp__proxy__tracing__query_context_impact`

**Use when:** Checking which responses depend on a context before updating it

**Example:**
```json
{
  "name": "mcp__proxy__tracing__query_context_impact",
  "arguments": {
    "context_unit_id": "ctx_docs_v2",
    "min_weight": 0.3
  }
}
```

### 4. Get Response Contexts
**Tool:** `mcp__proxy__tracing__get_response_contexts`

**Use when:** Analyzing which contexts contributed to a response

**Example:**
```json
{
  "name": "mcp__proxy__tracing__get_response_contexts",
  "arguments": {
    "response_id": "resp_abc123",
    "type": "External"
  }
}
```

### 5. Get Evolution History
**Tool:** `mcp__proxy__tracing__get_evolution_history`

**Use when:** Tracking how a context changed over time

**Example:**
```json
{
  "name": "mcp__proxy__tracing__get_evolution_history",
  "arguments": {
    "context_unit_id": "ctx_api_spec"
  }
}
```

## Agent Best Practices

### DO:
✅ Submit feedback after completing user tasks
✅ Use descriptive feedback_text explaining your reasoning
✅ Submit positive feedback for accurate, helpful responses
✅ Submit negative feedback when you detect errors
✅ Query context impact before making breaking changes

### DON'T:
❌ Submit feedback for intermediate reasoning steps
❌ Submit feedback without a clear quality assessment
❌ Ignore feedback opportunities (they improve the system)
❌ Use extreme scores (-1.0 or 1.0) without strong justification

## How Feedback Propagates

```
Submit feedback on Response
    ↓
Feedback score: 0.8
    ↓
Response used 3 contexts:
  - ctx_1 (weight 0.5) → receives 0.8 * 0.5 = 0.4 score contribution
  - ctx_2 (weight 0.3) → receives 0.8 * 0.3 = 0.24 score contribution
  - ctx_3 (weight 0.2) → receives 0.8 * 0.2 = 0.16 score contribution
    ↓
Each context's aggregate score updated via weighted average
    ↓
Low-scoring contexts (< -0.5) flagged for deprecation
```

## Integration Example

```bash
# List all available tools (including tracing tools)
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | \
  ./target/debug/mcp-rust-proxy --config test-proxy.yaml --stdio

# Submit feedback after a successful response
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"mcp__proxy__tracing__submit_feedback","arguments":{"response_id":"resp_test","score":0.9,"feedback_text":"Excellent response"}}}' | \
  ./target/debug/mcp-rust-proxy --config test-proxy.yaml --stdio
```

## Architecture

The proxy now provides:
- **Backend MCP servers** (filesystem, git, memory, etc.) - prefixed as `mcp__proxy__{server}__{tool}`
- **Context tracing tools** (5 tools) - prefixed as `mcp__proxy__tracing__{tool}`
- **Unified namespace** - no naming conflicts
- **Quality feedback loop** - continuous improvement via agent feedback

This enables LLMs to be **self-aware** and **self-improving** by understanding and rating their own context usage!
