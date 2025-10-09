# MCP Rust Proxy - Claude Development Guide

## Project Overview
MCP Rust Proxy is a high-performance Model Context Protocol (MCP) proxy server built in Rust. It provides connection pooling, health monitoring, and a web UI for managing multiple MCP servers.

## Project Structure
- This is a git work tree folder structure, the root folder has no code; the sub folders are of branches, create branches and merge to main when tested and ready
- Main development happens in feature branches (e.g., `feature/rust-mcp-proxy`)

## Key Components

### Core Architecture
- **Proxy Server** (`src/proxy/`): Handles incoming MCP requests and routes them to appropriate backend servers
- **Server Management** (`src/server/`): Manages MCP server lifecycle, health checks, and restarts
- **Transport Layer** (`src/transport/`): Supports stdio, HTTP-SSE, and WebSocket transports
- **State Management** (`src/state/`): Centralized state using DashMap for thread-safe concurrent access
- **Web UI** (`yew-ui/`): Rust/WASM frontend built with Yew framework

### Logging System
- **File-based Logging**: All MCP server stdout/stderr output is captured to rotating log files
- **Log Location**: `~/.mcp-proxy/logs/{server-name}/server.log`
- **Log Format**: `[timestamp] [STDOUT|STDERR] message`
- **Rotation**: Automatic rotation at 10MB file size, 2-day retention
- **Streaming**: Real-time log streaming via Server-Sent Events (SSE)
- **API Endpoints**:
  - `GET /api/logs/{server}?lines=N&type=stdout|stderr` - Fetch last N lines
  - `GET /api/logs/{server}/stream?type=stdout|stderr` - SSE stream for real-time logs

### Configuration
- **Config Files**: Supports YAML, JSON, TOML formats
- **Server Configs**: Define MCP servers with transport type, command, args, env vars
- **Health Checks**: Optional health monitoring with configurable intervals and thresholds
- **Connection Pooling**: Configurable pool sizes and connection limits

## Development Guidelines

### Building and Running
```bash
# Build without UI (faster for development)
cargo build

# Build with UI (requires trunk)
BUILD_YEW_UI=1 cargo build

# Run with config file
cargo run -- --config mcp-proxy-config.yaml

# Run tests
cargo test
```

### Testing MCP Servers
- Use `mock-logging-server.py` for testing log streaming
- Use `test-mock-logging.yaml` config for a pre-configured test setup
- Mock server generates continuous log output to test streaming functionality

### UI Development
- UI is built with Yew (Rust/WASM framework)
- Located in `yew-ui/` directory
- Uses WebSocket for real-time server status updates
- Uses SSE for log streaming from files
- Automatic rebuild when `BUILD_YEW_UI=1` is set

### Code Style
- Use idiomatic Rust patterns
- Prefer `Arc<DashMap>` for concurrent state
- Use `tokio` for async operations
- Use `tracing` for structured logging
- Keep error handling explicit with `Result<T, Error>`

### Common Tasks

#### Adding a New Transport
1. Implement the transport in `src/transport/`
2. Add to `TransportType` enum in `src/config/schema.rs`
3. Update `create_transport()` in `src/transport/mod.rs`
4. Add configuration parsing

#### Adding API Endpoints
1. Add handler function in `src/web/api.rs`
2. Add route in appropriate `*_routes()` function
3. Update types if needed
4. Document the endpoint

#### Debugging Logs
1. Check server logs at `~/.mcp-proxy/logs/{server-name}/server.log`
2. Use the web UI logs modal for real-time viewing
3. Use `curl http://localhost:3001/api/logs/{server}/stream` for raw SSE stream
4. Enable debug logging with `--debug` flag

## Important Notes
- **includeCoAuthoredBy** is set to false for git commits
- Always test with both stdio and WebSocket transports
- Ensure backwards compatibility with existing MCP servers
- Log files are automatically cleaned up after 2 days
- The proxy maintains persistent connections to MCP servers for performance

## Common Issues

### UI Not Updating
- Check WebSocket connection in browser console
- Verify server is running on expected ports (proxy: 3000, UI: 3001)
- Check for CORS issues if running from different origins

### Logs Not Appearing
- Verify log directory exists and has write permissions
- Check if MCP server is actually producing output
- Ensure ServerLogger is properly initialized in lifecycle.rs

### Build Issues
- If Yew UI fails to build, install trunk: `cargo install trunk --locked`
- For lock file version errors, delete `Cargo.lock` and rebuild
- Ensure rustc version is recent (1.70+ recommended)

## Testing with Playwright MCP
Use the Playwright MCP server to test UI functionality:
1. Ensure the server is running
2. Use `mcp__playwright__browser_navigate` to load the UI
3. Use `mcp__playwright__browser_click` to interact with elements
4. Use `mcp__playwright__browser_snapshot` to verify state