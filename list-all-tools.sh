#!/bin/bash
# List all tools from MCP proxy with proper initialization time

echo "Starting MCP Proxy and waiting for all servers to initialize..." >&2

# Start proxy in background and capture PID
./target/debug/mcp-rust-proxy --config mcp-proxy-config.yaml --stdio > proxy-stdio.fifo 2> proxy-stderr.log &
PROXY_PID=$!

# Wait for servers to initialize (30 seconds)
echo "Waiting 30 seconds for all servers to initialize..." >&2
sleep 30

# Send initialize request
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' > proxy-stdin.fifo

# Send initialized notification
sleep 1
echo '{"jsonrpc":"2.0","method":"notifications/initialized"}' > proxy-stdin.fifo

# Request tools list
sleep 1
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' > proxy-stdin.fifo

# Wait for response
sleep 5

# Kill proxy
kill $PROXY_PID 2>/dev/null

# Extract tool count
cat proxy-stdio.fifo | grep -E '^{"jsonrpc":"2.0","id":2' | jq -r '.result.tools | length'
