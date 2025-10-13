#!/bin/bash
echo "=== Testing Context7 Usage in Aggregator ===" >&2

{
  sleep 15
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
  sleep 2
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  sleep 2
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"How do I use the @anthropic-ai/sdk library to make API calls?"}}}'
  sleep 70
} | ./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio 2>&1 | tee context7-test.log

echo "" >&2
echo "=== Statistics from Plugin Log ===" >&2
tail -15 ~/.mcp-proxy/plugin-logs/aggregator-plugin.log | grep -E "Aggregation complete|MCP tool"
