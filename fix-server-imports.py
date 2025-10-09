#!/usr/bin/env python3
import os
import re

# Define the replacements
replacements = [
    # Error imports
    (r"use crate::error::Result;", "use mcp_proxy_core::Result;"),
    (
        r"use crate::error::\{Result, ServerError\};",
        "use mcp_proxy_core::{Result, error::ServerError};",
    ),
    (
        r"use crate::error::\{ProxyError, Result\};",
        "use mcp_proxy_core::{ProxyError, Result};",
    ),
    (r"use crate::error::HealthError;", "use mcp_proxy_core::error::HealthError;"),
    (r"use crate::error::ProxyError;", "use mcp_proxy_core::ProxyError;"),
    # Config imports
    (r"use crate::config::ServerConfig;", "use mcp_proxy_core::config::ServerConfig;"),
    (r"use crate::config::Config;", "use mcp_proxy_core::Config;"),
    (r"use crate::config::", "use mcp_proxy_core::config::"),
    # Transport imports
    (
        r"use crate::transport::\{create_transport, Transport\};",
        "use mcp_proxy_core::transport::{create_transport, Transport};",
    ),
    (r"use crate::transport::", "use mcp_proxy_core::transport::"),
    # Protocol imports
    (
        r"use crate::protocol::\{mcp, JsonRpcId, JsonRpcMessage, JsonRpcV2Message\};",
        "use mcp_proxy_core::protocol::{mcp, JsonRpcId, JsonRpcMessage, JsonRpcV2Message};",
    ),
    (r"use crate::protocol::", "use mcp_proxy_core::protocol::"),
    # Inline crate references
    (r"crate::error::ProxyError", "mcp_proxy_core::ProxyError"),
    (r"crate::error::ConfigError", "mcp_proxy_core::error::ConfigError"),
    (r"crate::error::ServerError", "mcp_proxy_core::error::ServerError"),
    (r"crate::config::Config", "mcp_proxy_core::Config"),
    (r"crate::config::validate", "mcp_proxy_core::config::validate"),
    (
        r"crate::transport::pool::ConnectionPool",
        "mcp_proxy_core::transport::pool::ConnectionPool",
    ),
]


def fix_imports(filepath):
    """Fix imports in a single file."""
    try:
        with open(filepath, "r") as f:
            content = f.read()

        original = content
        for pattern, replacement in replacements:
            content = re.sub(pattern, replacement, content)

        if content != original:
            with open(filepath, "w") as f:
                f.write(content)
            print(f"Fixed: {filepath}")
            return True
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
    return False


def main():
    server_src = "crates/mcp-proxy-server/src"
    fixed_count = 0

    for root, dirs, files in os.walk(server_src):
        for file in files:
            if file.endswith(".rs"):
                filepath = os.path.join(root, file)
                if fix_imports(filepath):
                    fixed_count += 1

    print(f"\nFixed {fixed_count} files")
    print("Running cargo fmt...")
    os.system("cargo fmt --all")


if __name__ == "__main__":
    main()
