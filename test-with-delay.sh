#!/bin/bash
set -e

# Create named pipes for communication
mkfifo proxy-in proxy-out 2>/dev/null || true

# Start proxy with pipes
./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio < proxy-in > proxy-out 2>&1 &
PROXY_PID=$!

echo "Proxy started (PID: $PROXY_PID), waiting 15 seconds for all servers..." >&2
sleep 15

# Send initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' > proxy-in
sleep 2

# Send initialized notification
echo '{"jsonrpc":"2.0","method":"notifications/initialized"}' > proxy-in
sleep 2

# Request tools list
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' > proxy-in
sleep 3

# Read response and count tools
timeout 2 cat proxy-out | grep '^{"jsonrpc":"2.0","id":2' | jq -r '.result.tools | length' 2>/dev/null || echo "Response parsing failed"

# Cleanup
kill $PROXY_PID 2>/dev/null || true
rm -f proxy-in proxy-out
