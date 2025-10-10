#!/bin/bash
# Test script for stdio mode proxy

set -e

echo "Starting proxy in stdio mode..."
./target/debug/mcp-rust-proxy --config test-proxy.yaml --stdio 2>/tmp/proxy-stderr.log &
PROXY_PID=$!

# Give it time to start
sleep 3

echo "Sending tools/list request..."
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | nc localhost 3000 2>/dev/null || {
    # If nc doesn't work, try direct stdio
    echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | ./target/debug/mcp-rust-proxy --config test-proxy.yaml --stdio 2>/tmp/proxy-stderr.log
}

# Clean up
kill $PROXY_PID 2>/dev/null || true

echo ""
echo "Check /tmp/proxy-stderr.log for proxy logs"
