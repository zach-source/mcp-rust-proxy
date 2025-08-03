# MCP Rust Proxy Server

A high-performance Model Context Protocol (MCP) proxy server written in Rust, designed to aggregate multiple MCP servers and provide a unified interface for clients.

## Features

- **Multi-Server Support**: Aggregate multiple MCP servers (stdio, HTTP/SSE, WebSocket transports)
- **Request Routing**: Intelligent routing based on tools, resources, and prompts
- **Health Monitoring**: Automatic health checks using MCP ping protocol
- **Graceful Shutdown**: Proper cleanup of connections and child processes
- **Web UI**: Simple status monitoring and server control interface
- **Environment Variables**: Full support for environment variable substitution in configs
- **Cancellation Support**: Proper handling of MCP request cancellations
- **Connection Pooling**: Efficient connection management with automatic reconnection
- **Metrics**: Prometheus-compatible metrics for monitoring

## Architecture

The proxy is built with:
- **Tokio**: Async runtime for high concurrency
- **Warp**: Fast web framework for the proxy and UI endpoints
- **DashMap**: Lock-free concurrent data structures
- **Serde**: Flexible configuration in YAML/JSON/TOML formats

## Configuration

Create a configuration file (YAML, JSON, or TOML):

```yaml
# proxy configuration
proxy:
  host: "127.0.0.1"
  port: 8080
  connectionPoolSize: 10
  requestTimeoutMs: 30000
  maxConcurrentRequests: 100

# web UI configuration
webUi:
  enabled: true
  host: "127.0.0.1" 
  port: 8081
  apiKey: "optional-api-key"

# health check configuration
healthCheck:
  enabled: true
  intervalSeconds: 30
  timeoutSeconds: 5

# MCP servers
servers:
  time-server:
    command: "mcp-server-time"
    args: []
    env:
      LOG_LEVEL: "${LOG_LEVEL:-info}"
    transport:
      type: stdio
    restartOnFailure: true
    maxRestarts: 3
    restartDelayMs: 1000
```

## Running

```bash
# Run with default config search
cargo run

# Run with specific config file
cargo run -- --config my-config.yaml

# Enable debug logging
cargo run -- --debug
```

## MCP Protocol Support

The proxy implements the Model Context Protocol specification, including:

- **JSON-RPC 2.0** message format
- **Ping/Pong** health checks (per MCP spec)
- **Request cancellation** via `notifications/cancelled`
- **Transport abstraction** for stdio, HTTP/SSE, and WebSocket
- **Proper request/response correlation** with ID matching

## Testing

The project includes comprehensive tests:

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --lib protocol::tests  # Protocol parsing tests
cargo test --lib proxy::tests     # Proxy routing tests
```

## Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Note: On macOS, you may need to install libiconv:
# brew install libiconv
```

## Web UI

When enabled, the web UI provides:
- Server status monitoring
- Start/stop/restart controls
- Real-time health status
- Connection metrics

Access at: `http://localhost:8081` (or configured port)

## Environment Variables

The proxy supports environment variable substitution in configurations using `${VAR}` or `${VAR:-default}` syntax.

## License

[Add your license here]