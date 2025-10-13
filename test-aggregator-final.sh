#!/bin/bash
echo "=== Final Aggregator Test with 60s timeout ===" >&2

{
  sleep 15
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
  sleep 2
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  sleep 2
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"Explain React useState hook in one sentence"}}}'
  sleep 70
} | ./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio 2>&1 | tee final-test.log

echo "=== Checking for response ===" >&2
grep '"id":2' final-test.log | jq '.result.content[0].text' 2>/dev/null || echo "No result found"
