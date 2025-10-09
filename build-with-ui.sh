#!/bin/bash

echo "Building MCP Rust Proxy with Yew UI..."

# Build with Yew UI enabled
BUILD_YEW_UI=1 cargo build --release

echo "Build complete! Run with: ./target/release/mcp-rust-proxy"