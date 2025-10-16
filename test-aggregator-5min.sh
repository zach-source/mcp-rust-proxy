#!/bin/bash
echo "=== Aggregator Test - 5 Minute Window ===" >&2
echo "Server init: 15s, Tool call wait: 4m 30s" >&2

{
  sleep 15
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
  sleep 2
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  sleep 2
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"Explain React useState hook briefly"}}}'
  sleep 270
} | ./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio 2>&1 | tee aggregator-5min.log

echo "" >&2
echo "=== Results ===" >&2
grep '"id":2' aggregator-5min.log | head -1 | jq -r '.result.content[0].text' 2>/dev/null || echo "Checking for error..."
grep '"id":2' aggregator-5min.log | head -1 | jq '.error.message' 2>/dev/null
