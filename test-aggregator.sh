#!/bin/bash
# Test the aggregator plugin

echo "=== Testing Aggregator Plugin ===" >&2
echo "" >&2

# Start proxy and wait for initialization
echo "Starting proxy, waiting 15 seconds for servers..." >&2

{
  sleep 15
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
  sleep 2
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  sleep 2

  # Call aggregator tool
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"How to use React hooks for state management?"}}}'

  sleep 20
} | ./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio 2>aggregator-test-errors.log | tee aggregator-test-output.log

echo "" >&2
echo "=== Test Complete ===" >&2
echo "Check aggregator-test-output.log for results" >&2
echo "Check aggregator-test-errors.log for server logs" >&2
