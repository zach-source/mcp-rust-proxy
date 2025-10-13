#!/bin/bash
# Test serena specifically loads and provides tools

echo "=== Testing Serena MCP Server with Protocol Version Support ===" >&2
echo "" >&2

# Start proxy and wait for serena to initialize (it can be slow)
echo "Starting proxy and waiting 20 seconds for serena to initialize..." >&2

{
  sleep 20
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
  sleep 2
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  sleep 2
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
  sleep 5
  # Keep connection open
  sleep 5
} | ./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio 2>serena-test-errors.log | tee serena-test-output.log

echo "" >&2
echo "=== Results ===" >&2

# Extract and count serena tools
SERENA_TOOLS=$(grep '^{"jsonrpc":"2.0","id":2' serena-test-output.log | jq -r '.result.tools[] | select(.server == "serena") | .name' 2>/dev/null | wc -l)
TOTAL_TOOLS=$(grep '^{"jsonrpc":"2.0","id":2' serena-test-output.log | jq -r '.result.tools | length' 2>/dev/null)

echo "Total tools: $TOTAL_TOOLS" >&2
echo "Serena tools: $SERENA_TOOLS" >&2

# Check logs for serena initialization
echo "" >&2
echo "=== Serena Initialization Log ===" >&2
grep -i "serena.*initialized\|serena.*protocol" serena-test-errors.log || echo "No serena initialization found" >&2

# Show serena tools
if [ "$SERENA_TOOLS" -gt 0 ]; then
  echo "" >&2
  echo "=== Serena Tools Found ===" >&2
  grep '^{"jsonrpc":"2.0","id":2' serena-test-output.log | jq -r '.result.tools[] | select(.server == "serena") | .name' 2>/dev/null
else
  echo "WARNING: No serena tools found!" >&2
  echo "" >&2
  echo "=== Checking for errors ===" >&2
  grep -i "serena.*error\|serena.*failed" serena-test-errors.log | head -5
fi
