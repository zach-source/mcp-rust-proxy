#!/bin/bash
# End-to-end test for aggregator plugin

echo "=== Aggregator Plugin End-to-End Test ===" >&2
echo "" >&2

# Start proxy and wait for all servers to initialize
{
  sleep 20

  # Initialize
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
  sleep 2

  # Send initialized notification
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  sleep 2

  # Test 1: Code-related query (should trigger context7 + serena)
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"Find the implementation of protocol version negotiation in our codebase"}}}'
  sleep 60

  # Test 2: Documentation query (should trigger only context7)
  echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"Explain TypeScript async/await syntax"}}}'
  sleep 60

} | ./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio 2>&1 | tee e2e-test-output.log

echo "" >&2
echo "=== Test Results ===" >&2
echo "" >&2

# Extract Test 1 response
echo "Test 1 (Code query - should use context7 + serena):" >&2
grep '"id":2' e2e-test-output.log | jq -r '.result.content[0].text' 2>/dev/null | head -c 200
echo "" >&2
echo "..." >&2
echo "" >&2

# Extract Test 2 response
echo "Test 2 (Documentation query - should use context7 only):" >&2
grep '"id":3' e2e-test-output.log | jq -r '.result.content[0].text' 2>/dev/null | head -c 200
echo "" >&2
echo "..." >&2
echo "" >&2

# Show statistics from plugin logs
echo "=== Statistics ===" >&2
tail -20 ~/.mcp-proxy/plugin-logs/aggregator-plugin.log | grep "Aggregation complete"
echo "" >&2

# Check which servers were selected
echo "=== Server Selection ===" >&2
grep "Selected MCP servers" e2e-test-output.log | tail -2
