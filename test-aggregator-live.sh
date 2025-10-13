#!/bin/bash
# Test aggregator with live API key

echo "=== Testing Aggregator Plugin with Live Claude Agent SDK ===" >&2

{
  sleep 15
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
  sleep 2
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  sleep 2
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"Explain React hooks briefly"}}}'
  sleep 30
} | ./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio 2>&1 | tee aggregator-live-output.log

echo "" >&2
echo "=== Test Complete ===" >&2
