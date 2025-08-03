#!/bin/bash

# Test the MCP proxy with a simple request

# List tools request
echo "Testing list tools request..."
curl -X POST http://localhost:3000 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list",
    "params": {}
  }'

echo ""
echo ""

# Ping request
echo "Testing ping request..."
curl -X POST http://localhost:3000 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "ping",
    "params": {}
  }'