# MCP Rust Proxy Server

A fast and efficient Model Context Protocol (MCP) proxy server written in Rust. This proxy aggregates multiple MCP servers and provides a unified interface, with built-in monitoring, health checks, and a web UI for management.

## Features

- **Multi-Server Proxy**: Aggregate multiple MCP servers into a single endpoint
- **Multiple Transports**: Support for stdio, HTTP/SSE, and WebSocket transports
- **Configuration Management**: YAML/JSON configuration with environment variable substitution
- **Server Lifecycle Management**: Start, stop, and restart individual servers
- **Health Monitoring**: Automatic health checks with configurable intervals
- **Web UI Dashboard**: Real-time server status monitoring and control
- **Metrics Collection**: Prometheus-compatible metrics for monitoring
- **Connection Pooling**: Efficient connection management with automatic reconnection
- **Graceful Shutdown**: Clean shutdown of all servers and connections

## Quick Start

1. Create a configuration file `mcp-proxy.yaml`:

```yaml
servers:
  example-server:
    command: "mcp-server-example"
    args: ["--port", "8080"]
    transport:
      type: stdio
    restartOnFailure: true

proxy:
  port: 3000
  host: "0.0.0.0"

webUI:
  enabled: true
  port: 3001
```

2. Run the proxy server:

```bash
cargo run
```

3. Access the web UI at `http://localhost:3001`

## Configuration

The proxy server can be configured using YAML or JSON files. Configuration files are searched in the following order:
- `mcp-proxy.toml`
- `mcp-proxy.json`
- `mcp-proxy.yaml`
- `mcp-proxy.yml`

### Environment Variables

All configuration values support environment variable substitution using the `${VAR}` syntax:

```yaml
servers:
  api-server:
    command: "api-server"
    env:
      API_KEY: "${API_KEY}"
    transport:
      type: httpSse
      url: "${API_URL:-http://localhost:8080}/sse"
```

### Server Configuration

Each server configuration supports:
- `command`: The executable to run
- `args`: Command line arguments
- `env`: Environment variables for the process
- `transport`: Transport configuration (stdio, httpSse, webSocket)
- `restartOnFailure`: Whether to restart on failure (default: true)
- `maxRestarts`: Maximum number of restart attempts (default: 3)
- `restartDelayMs`: Delay between restarts in milliseconds (default: 5000)

### Web UI Configuration

The web UI can be configured with:
- `enabled`: Whether to enable the web UI (default: true)
- `port`: Port to listen on (default: 3001)
- `host`: Host to bind to (default: "0.0.0.0")
- `apiKey`: Optional API key for authentication

## Architecture

The proxy server is built with:
- **Tokio**: Async runtime for high-performance I/O
- **Warp**: Web framework for the proxy and web UI
- **DashMap**: Lock-free concurrent hash maps
- **Prometheus**: Metrics collection and export
- **Serde**: Configuration serialization/deserialization

## Development

### Building

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Project Structure

```
src/
├── config/       # Configuration loading and validation
├── transport/    # Transport implementations (stdio, HTTP/SSE, WebSocket)
├── proxy/        # Core proxy logic and request routing
├── server/       # Server lifecycle management
├── state/        # Application state and metrics
├── web/          # Web UI and REST API
└── main.rs       # Application entry point
```

## License

MIT