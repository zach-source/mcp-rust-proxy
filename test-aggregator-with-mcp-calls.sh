#!/bin/bash
# Test aggregator with queries that FORCE MCP tool usage

echo "=== Aggregator Plugin - MCP Tool Usage Test ===" >&2
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

  # Test 1: Query that requires serena (code search in THIS codebase)
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"What is the exact implementation of the aggregator plugin system prompt in our codebase? Show me the actual code."}}}'
  sleep 60

  # Test 2: Query that requires context7 (specific library docs)
  echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"How does the @anthropic-ai/claude-agent-sdk query function work according to its latest documentation? What are its exact parameters?"}}}'
  sleep 60

  # Test 3: Complex query requiring both servers
  echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"mcp__proxy__aggregator__context_aggregator","arguments":{"query":"Find all references to ProtocolVersion in our Rust codebase and explain how Rust enum pattern matching works in the latest Rust docs"}}}'
  sleep 90

} | ./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio 2>&1 | tee mcp-usage-test.log

echo "" >&2
echo "=== Test Results ===" >&2
echo "" >&2

# Function to extract and display results
display_result() {
  local test_id=$1
  local test_name=$2

  echo "=== $test_name ===" >&2

  # Extract response
  local response=$(grep "\"id\":$test_id" mcp-usage-test.log | jq -r '.result.content[0].text' 2>/dev/null)
  if [ -n "$response" ]; then
    echo "Response preview (first 300 chars):" >&2
    echo "$response" | head -c 300 >&2
    echo "..." >&2
    echo "" >&2
    echo "Full response length: $(echo "$response" | wc -c) bytes" >&2
  else
    echo "No response found" >&2
  fi
  echo "" >&2
}

display_result 2 "Test 1: Serena Code Search"
display_result 3 "Test 2: Context7 Library Docs"
display_result 4 "Test 3: Combined Query"

# Show aggregation statistics
echo "=== Aggregation Statistics ===" >&2
tail -50 ~/.mcp-proxy/plugin-logs/aggregator-plugin.log | grep "Aggregation complete"
echo "" >&2

echo "=== MCP Tool Calls ===" >&2
tail -100 ~/.mcp-proxy/plugin-logs/aggregator-plugin.log | grep "MCP tool"
echo "" >&2

echo "=== Server Selection ===" >&2
grep "Selected MCP servers" mcp-usage-test.log | tail -3
echo "" >&2
