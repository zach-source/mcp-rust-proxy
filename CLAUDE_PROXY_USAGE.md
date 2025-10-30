# Using Claude API Proxy to Inspect Context

## Quick Start (3 Steps)

### 1. Start the Proxy

```bash
cd /Users/ztaylor/repos/workspaces/mcp-rust-proxy/main
git checkout 005-claude-api-proxy
cargo run --release -- --config claude-proxy-test.yaml
```

Expected output:
```
✅ Claude API proxy server started bind_address=127.0.0.1:8443
✅ TLS handler initialized ca_cert_path="/Users/ztaylor/.claude-proxy/ca.crt"
✅ Capture storage initialized
```

### 2. Configure Your Client

```bash
# In a new terminal, set proxy for Claude
export HTTP_PROXY=http://localhost:8443

# Verify it's set
echo $HTTP_PROXY
# Should show: http://localhost:8443
```

### 3. Use Claude and Inspect Context

```bash
# Make a request (use curl for testing, or actual Claude CLI)
curl -k --proxy http://localhost:8443 https://api.anthropic.com/v1/messages \
  -H "x-api-key: YOUR_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "What is Rust?"}]
  }'

# View captured requests
curl http://localhost:3001/api/claude/requests | jq

# Get details with context attribution
REQUEST_ID=$(curl -s http://localhost:3001/api/claude/requests | jq -r '.requests[0].id')
curl "http://localhost:3001/api/claude/requests/$REQUEST_ID" | jq
```

---

## What You'll See

### Request Capture

```json
{
  "id": "req_...",
  "timestamp": "2025-10-29T...",
  "url": "/v1/messages",
  "method": "POST",
  "headers": {
    "content-type": "application/json",
    "x-api-key": "[REDACTED]",  // ← API key automatically redacted!
    ...
  },
  "body_json": {
    "model": "claude-3-5-sonnet-20241022",
    "messages": [...],
    "max_tokens": 100
  },
  "total_tokens": 25,  // ← Sum of tokens from all context sources
  "correlation_id": "corr_..."
}
```

### Context Attribution

When you make a request with MCP tool results, you'll see:

```json
{
  "attributions": [
    {
      "id": "attr_...",
      "source_type": "User",
      "source_name": null,
      "token_count": 10,
      "message_role": "user",
      "message_index": 1
    },
    {
      "id": "attr_...",
      "source_type": "McpServer",
      "source_name": "context7",  // ← Identified from tool_use_id!
      "token_count": 450,
      "message_role": "user",
      "message_index": 2
    },
    {
      "id": "attr_...",
      "source_type": "Framework",
      "source_name": null,
      "token_count": 120,
      "message_role": "system"
    }
  ]
}
```

### Response Capture

```json
{
  "response": {
    "id": "resp_...",
    "correlation_id": "corr_...",  // ← Links to request
    "status_code": 200,
    "latency_ms": 450,
    "proxy_latency_ms": 1,  // ← Proxy only added 1ms!
    "response_tokens": 85,
    "body_json": {
      "content": [...],
      "usage": {"input_tokens": 25, "output_tokens": 85}
    }
  }
}
```

---

## Database Queries

```bash
# Connect to database
sqlite3 ~/.mcp-proxy/context.db

# View all captures
SELECT id, url, method, total_tokens,
       datetime(timestamp, 'unixepoch') as time
FROM captured_requests
ORDER BY timestamp DESC;

# See request/response pairs
SELECT
  r.id as request_id,
  r.url,
  r.method,
  r.total_tokens as req_tokens,
  resp.status_code,
  resp.response_tokens as resp_tokens,
  resp.latency_ms
FROM captured_requests r
LEFT JOIN captured_responses resp ON r.correlation_id = resp.correlation_id
ORDER BY r.timestamp DESC;

# Attribution breakdown
SELECT
  source_type,
  source_name,
  COUNT(*) as count,
  SUM(token_count) as total_tokens,
  AVG(token_count) as avg_tokens
FROM context_attributions
GROUP BY source_type, source_name
ORDER BY total_tokens DESC;
```

---

## Advanced Usage

### Filter by Time Range

```bash
# Requests in last hour
START=$(date -u -v-1H +"%Y-%m-%dT%H:%M:%SZ")
curl "http://localhost:3001/api/claude/requests?start_time=$START"

# Requests from specific time period
curl "http://localhost:3001/api/claude/requests?start_time=2025-10-29T00:00:00Z&end_time=2025-10-29T23:59:59Z"
```

### Filter by Context Source

```bash
# Only requests that used context7
curl "http://localhost:3001/api/claude/requests?context_source=context7"
```

### Pagination

```bash
# First 10 results
curl "http://localhost:3001/api/claude/requests?limit=10&offset=0"

# Next 10 results
curl "http://localhost:3001/api/claude/requests?limit=10&offset=10"
```

---

## Troubleshooting

### "Connection refused" error

The proxy isn't running. Start it:
```bash
./target/release/mcp-rust-proxy --config claude-proxy-test.yaml
```

### "SSL certificate problem: self-signed certificate"

This is expected! The proxy uses a self-signed CA. Either:

**Option A**: Use `-k` flag with curl (skip verification)
```bash
curl -k --proxy http://localhost:8443 ...
```

**Option B**: Trust the CA certificate (recommended for Claude CLI)
```bash
# macOS
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain \
  ~/.claude-proxy/ca.crt

# Linux
sudo cp ~/.claude-proxy/ca.crt /usr/local/share/ca-certificates/claude-proxy.crt
sudo update-ca-certificates
```

### No requests captured

1. Verify proxy is set:
   ```bash
   echo $HTTP_PROXY  # Should show http://localhost:8443
   ```

2. Verify capture is enabled:
   ```bash
   grep "captureEnabled" claude-proxy-test.yaml
   # Should show: captureEnabled: true
   ```

3. Check database:
   ```bash
   sqlite3 ~/.mcp-proxy/context.db "SELECT COUNT(*) FROM captured_requests;"
   ```

### Query API returns empty

Database might be empty. Make a test request first:
```bash
curl -k --proxy http://localhost:8443 https://api.anthropic.com/v1/models
```

Then query again:
```bash
curl http://localhost:3001/api/claude/requests
```

---

## Real-World Use Case

**Scenario**: You want to see what context Claude Code is sending when it uses MCP servers.

```bash
# 1. Start proxy
./target/release/mcp-rust-proxy --config claude-proxy-test.yaml

# 2. In another terminal, configure Claude Code to use proxy
export HTTP_PROXY=http://localhost:8443

# 3. Use Claude Code with MCP servers enabled
# (Run your normal Claude Code workflow)

# 4. After interactions, query what was sent:
curl http://localhost:3001/api/claude/requests | jq '.requests[] | {
  url,
  method,
  total_tokens,
  timestamp
}'

# 5. Deep dive into a specific request:
REQUEST_ID=$(curl -s http://localhost:3001/api/claude/requests | jq -r '.requests[0].id')
curl "http://localhost:3001/api/claude/requests/$REQUEST_ID" | jq '{
  request: .request | {url, method, total_tokens},
  attributions: .attributions | map({
    source_type,
    source_name,
    token_count,
    message_role
  }),
  response: .response | {status_code, latency_ms, response_tokens}
}'
```

**Expected Output**:
You'll see exactly which MCP servers contributed context, how many tokens each added, and what role they played in the conversation!

---

## Tips

1. **Test with curl first** before using with Claude CLI to verify proxy works
2. **Use `-k` flag** to skip certificate verification during testing
3. **Check logs** if capture isn't working: proxy logs show "captured successfully"
4. **Query database directly** if API has issues: `sqlite3 ~/.mcp-proxy/context.db`
5. **Monitor latency** in captured_responses table: proxy_latency_ms column

---

**Ready to see what context Claude is really getting? Start the proxy and inspect!** 🔍
