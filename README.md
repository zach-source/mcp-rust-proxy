# MCP Rust Proxy

A high-performance Model Context Protocol (MCP) proxy server built in Rust with a Tauri desktop application.

## Project Structure

This project uses a Rust workspace with multiple crates for better modularity:

```
├── crates/
│   ├── mcp-proxy-core/        # Core business logic (protocol, transport, config)
│   ├── mcp-proxy-server/      # Server implementation (proxy, web API, state)
│   ├── mcp-proxy-shared/      # Shared types between frontend and backend
│   └── mcp-proxy-cli/         # Command-line interface
├── tauri-app/                 # Tauri desktop application
│   ├── src/                   # Frontend (TypeScript/React)
│   └── src-tauri/            # Tauri backend (Rust)
├── docs/                      # Documentation
└── configs/examples/          # Example configuration files
```

## Features

- **High Performance**: Built with Rust for maximum performance and reliability
- **Multi-Server Proxy**: Aggregate multiple MCP servers into a single endpoint
- **Multiple Transports**: Support for stdio, HTTP-SSE, and WebSocket
- **Connection Pooling**: Efficient connection management with automatic reconnection
- **Health Monitoring**: Automatic health checks with configurable intervals
- **Desktop Application**: Native desktop app built with Tauri
- **Web UI**: Alternative web interface for browser access
- **File-based Logging**: Comprehensive logging with rotation
- **Server Management**: Start, stop, restart, and disable servers
- **Real-time Updates**: Live server status and log streaming
- **Graceful Shutdown**: Clean shutdown of all servers and connections

## Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- Node.js 18+ (for Tauri frontend)
- pnpm (install with `npm install -g pnpm`)

### Building

```bash
# Build all crates
cargo build --release

# Build with Tauri UI
cd tauri-app
pnpm install
pnpm tauri build
```

### Running

#### CLI Mode
```bash
# Run with configuration file
cargo run --bin mcp-proxy -- --config configs/examples/basic.yaml

# Run with debug logging
cargo run --bin mcp-proxy -- --debug --config configs/examples/basic.yaml
```

#### Desktop Application
```bash
cd tauri-app
pnpm tauri dev
```

## Using with Claude Code

MCP Rust Proxy works seamlessly with Claude Code to manage multiple MCP servers. Configure Claude Code to use the proxy via MCP remote server:

```json
{
  "mcpServers": {
    "rust-proxy": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-remote", "http://localhost:3000"]
    }
  }
}
```

## Configuration

Create a configuration file in YAML format:

```yaml
proxy:
  host: "0.0.0.0"
  port: 3000
  connectionPoolSize: 10

webUI:
  enabled: true
  host: "0.0.0.0"
  port: 3001

servers:
  filesystem-server:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/files"]
    transport:
      type: stdio
    restartOnFailure: true
    maxRestarts: 3
    
  github-server:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-github"]
    transport:
      type: stdio
    env:
      GITHUB_TOKEN: "${GITHUB_TOKEN}"
    healthCheck:
      enabled: true
      intervalSeconds: 30
      timeoutMs: 5000
```

See `configs/examples/` for more configuration examples.

## Development

### Project Layout

- **mcp-proxy-core**: Protocol handling, transport abstractions, configuration parsing
- **mcp-proxy-server**: Server implementation, proxy logic, web API, state management
- **mcp-proxy-shared**: Types shared between frontend and backend
- **mcp-proxy-cli**: Command-line interface
- **tauri-app**: Desktop application with native OS integration

### Running Tests

```bash
# Run all tests
cargo test --all

# Run specific crate tests
cargo test -p mcp-proxy-core
cargo test -p mcp-proxy-server

# Run with coverage
cargo tarpaulin --all
```

### Building Documentation

```bash
# Build and open documentation
cargo doc --all --open
```

## Architecture

The MCP Proxy uses a modular architecture:

1. **Core Layer**: Protocol definitions and transport abstractions
2. **Server Layer**: Proxy implementation and state management
3. **API Layer**: RESTful API and WebSocket connections
4. **UI Layer**: Tauri desktop app or web interface

### Logging System

The proxy captures all server output to rotating log files:
- **Location**: `~/.mcp-proxy/logs/{server-name}/server.log`
- **Format**: `[timestamp] [STDOUT|STDERR] message`
- **Rotation**: Automatic at 10MB, 2-day retention
- **API Access**: 
  - `GET /api/logs/{server}?lines=N&type=stdout|stderr`
  - `GET /api/logs/{server}/stream` (Server-Sent Events)

### Health Monitoring

Configure health checks to monitor server availability:

```yaml
healthChecks:
  critical-server:
    enabled: true
    intervalSeconds: 30
    timeoutMs: 5000
    threshold: 3  # Failures before marking unhealthy
```

### Metrics

Prometheus-compatible metrics available at `/metrics`:
- `mcp_proxy_requests_total`
- `mcp_proxy_request_duration_seconds`
- `mcp_proxy_active_connections`
- `mcp_proxy_server_restarts_total`

## Monitoring and Debugging

### Viewing Logs

1. **Desktop App**: View real-time logs in the UI
2. **Web UI**: Click "Logs" button for any server
3. **Files**: Check `~/.mcp-proxy/logs/{server-name}/server.log`
4. **API**: `curl http://localhost:3001/api/logs/server-name?lines=50`
5. **Stream**: `curl http://localhost:3001/api/logs/server-name/stream`

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit pull requests to our repository.

## License

MIT