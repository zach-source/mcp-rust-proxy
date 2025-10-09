#!/bin/bash

# Fix imports in mcp-proxy-server to use mcp-proxy-core

echo "Fixing imports in mcp-proxy-server..."

# Fix error imports
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/use crate::error::/use mcp_proxy_core::error::/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/use crate::error/use mcp_proxy_core/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/crate::error::/mcp_proxy_core::error::/g' {} \;

# Fix config imports
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/use crate::config::/use mcp_proxy_core::config::/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/use crate::config/use mcp_proxy_core::config/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/crate::config::/mcp_proxy_core::config::/g' {} \;

# Fix transport imports  
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/use crate::transport::/use mcp_proxy_core::transport::/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/use crate::transport/use mcp_proxy_core::transport/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/crate::transport::/mcp_proxy_core::transport::/g' {} \;

# Fix protocol imports
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/use crate::protocol::/use mcp_proxy_core::protocol::/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/use crate::protocol/use mcp_proxy_core::protocol/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/crate::protocol::/mcp_proxy_core::protocol::/g' {} \;

# Clean up double colons that might result
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/mcp_proxy_core::::error/mcp_proxy_core::error/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/mcp_proxy_core::::config/mcp_proxy_core::config/g' {} \;

# Fix specific ProxyError and Result imports
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/mcp_proxy_core::error::{ProxyError, Result}/mcp_proxy_core::{ProxyError, Result}/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/mcp_proxy_core::error::Result/mcp_proxy_core::Result/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/mcp_proxy_core::error::ProxyError/mcp_proxy_core::ProxyError/g' {} \;

# Fix ConfigError, ServerError, HealthError imports
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/mcp_proxy_core::error::ConfigError/mcp_proxy_core::error::ConfigError/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/mcp_proxy_core::error::ServerError/mcp_proxy_core::error::ServerError/g' {} \;
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/mcp_proxy_core::error::HealthError/mcp_proxy_core::error::HealthError/g' {} \;

# Fix bytes import
find crates/mcp-proxy-server/src -name "*.rs" -exec sed -i '' 's/bytes::Bytes/bytes::Bytes/g' {} \;

echo "Imports fixed. Running cargo fmt..."
cargo fmt --all

echo "Done!"