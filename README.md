# MCP Rust Proxy Server

A fast and efficient Model Context Protocol (MCP) proxy server written in Rust. This proxy aggregates multiple MCP servers and provides a unified interface, with built-in monitoring, health checks, and a web UI for management.

**New Features:**
- 🎯 **AI Context Tracing Framework**: Track which context units influenced AI responses
- 🔍 **Self-Aware AI**: LLMs can inspect their own context provenance and quality
- 📊 **Quality Feedback Loop**: Continuous improvement through feedback propagation
- 🌐 **Modern Yew UI**: Built-in web dashboard for monitoring (see [YEW_UI_INTEGRATION.md](YEW_UI_INTEGRATION.md))

## Features

### Core Proxy Features
- **Multi-Server Proxy**: Aggregate multiple MCP servers into a single endpoint
- **Multiple Transports**: Support for stdio, HTTP/SSE, and WebSocket transports
- **Tool Name Prefixing**: Prevent naming conflicts with `mcp__proxy__{server}__{tool}` format
- **File-based Logging**: All server output captured to rotating log files with real-time streaming
- **Configuration Management**: YAML/JSON configuration with environment variable substitution
- **Server Lifecycle Management**: Start, stop, and restart individual servers
- **Health Monitoring**: Automatic health checks with configurable intervals
- **Web UI Dashboard**: Real-time server status monitoring and control
- **Metrics Collection**: Prometheus-compatible metrics for monitoring
- **Connection Pooling**: Efficient connection management with automatic reconnection
- **Graceful Shutdown**: Clean shutdown of all servers and connections

### AI Context Tracing Features (NEW)
- **Provenance Tracking**: Complete lineage manifests showing context → response relationships
- **Multi-Factor Weighting**: Composite scoring (retrieval 40%, recency 30%, type 20%, length 10%)
- **Hybrid Storage**: DashMap in-memory cache + SQLite persistence with WAL mode
- **Bidirectional Queries**: Find responses using a context, or contexts in a response
- **Feedback Propagation**: Quality ratings automatically update all contributing contexts
- **Version Tracking**: Full evolution history for context units
- **Quality Signals**: High/low-rated contexts exposed as MCP resources
- **Self-Improvement**: LLMs can rate their own responses to improve future performance

#### Context Tracing MCP Integration

**5 Tools** for explicit operations:
- `mcp__proxy__tracing__get_trace` - View response lineage
- `mcp__proxy__tracing__query_context_impact` - Assess context impact
- `mcp__proxy__tracing__get_response_contexts` - List contributing contexts
- `mcp__proxy__tracing__get_evolution_history` - Track version history
- `mcp__proxy__tracing__submit_feedback` - Submit quality ratings

**4 Resources** for automatic context enrichment:
- `trace://quality/top-contexts` - High-quality information sources
- `trace://quality/deprecated-contexts` - Low-quality contexts to avoid
- `trace://quality/recent-feedback` - Quality feedback trends
- `trace://stats/cache` - Performance metrics

See [TRACING_TOOLS_QUICKSTART.md](TRACING_TOOLS_QUICKSTART.md) for LLM agent usage guide.

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
cargo run -- --config mcp-proxy.yaml
```

3. Access the web UI at `http://localhost:3001`

## Using with Claude CLI (Stdio Mode)

The proxy can run as an MCP server for Claude CLI, aggregating all your backend servers:

```bash
# Build the proxy
cargo build --release  # or use debug: cargo build

# Run with Claude CLI
claude --mcp-config '{"mcpServers":{"proxy":{"command":"./target/debug/mcp-rust-proxy","args":["--config","mcp-proxy-config.yaml","--stdio"]}}}'
```

**What Claude gets:**
- All tools from all configured backend servers (filesystem, git, memory, etc.)
- Tool names prefixed: `mcp__proxy__{server}__{tool}` to prevent conflicts
- 5 context tracing tools for self-awareness
- 4 context quality resources for automatic context enrichment

**Example tools available:**
- `mcp__proxy__memory__create_entities` - From memory server
- `mcp__proxy__git__commit` - From git server
- `mcp__proxy__tracing__submit_feedback` - Built-in tracing
- And many more...

See [TRACING_TOOLS_QUICKSTART.md](TRACING_TOOLS_QUICKSTART.md) for full agent documentation.

## Using with Claude Code (HTTP Mode)

MCP Rust Proxy works seamlessly with Claude Code to manage multiple MCP servers. Here are some example configurations:

### Example 1: Multiple Tool Servers

```yaml
servers:
  filesystem-server:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/Users/username/projects"]
    transport:
      type: stdio
    env:
      NODE_OPTIONS: "--max-old-space-size=4096"
  
  github-server:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-github"]
    transport:
      type: stdio
    env:
      GITHUB_TOKEN: "${GITHUB_TOKEN}"
  
  postgres-server:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"]
    transport:
      type: stdio

proxy:
  port: 3000
  host: "127.0.0.1"

webUI:
  enabled: true
  port: 3001
```

Then configure Claude Code to use the proxy via MCP remote server:

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

### Example 2: Development Environment

```yaml
servers:
  # Code intelligence server
  code-intel:
    command: "rust-analyzer"
    args: ["--stdio"]
    transport:
      type: stdio
    
  # Database tools
  db-tools:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-sqlite", "./dev.db"]
    transport:
      type: stdio
  
  # Custom project server
  project-server:
    command: "python"
    args: ["./scripts/mcp_server.py"]
    transport:
      type: stdio
    env:
      PROJECT_ROOT: "${PWD}"
      DEBUG: "true"

# Health checks for critical servers
healthChecks:
  code-intel:
    enabled: true
    intervalSeconds: 30
    timeoutMs: 5000
    threshold: 3

proxy:
  port: 3000
  connectionPoolSize: 10
  maxConnectionsPerServer: 5

webUI:
  enabled: true
  port: 3001
  apiKey: "${WEB_UI_API_KEY}"
```

### Example 3: Production Deployment

```yaml
servers:
  api-gateway:
    command: "mcp-api-gateway"
    transport:
      type: webSocket
      url: "ws://api-gateway:8080/mcp"
    restartOnFailure: true
    maxRestarts: 5
    restartDelayMs: 10000
  
  ml-models:
    command: "mcp-ml-server"
    transport:
      type: httpSse
      url: "http://ml-server:9000/sse"
      headers:
        Authorization: "Bearer ${ML_API_KEY}"
    
  vector-db:
    command: "mcp-vector-server"
    args: ["--collection", "production"]
    transport:
      type: stdio
    env:
      PINECONE_API_KEY: "${PINECONE_API_KEY}"
      PINECONE_ENV: "production"

healthChecks:
  api-gateway:
    enabled: true
    intervalSeconds: 10
    timeoutMs: 3000
    threshold: 2
    
  ml-models:
    enabled: true
    intervalSeconds: 30
    timeoutMs: 10000

proxy:
  port: 3000
  host: "0.0.0.0"
  connectionPoolSize: 50
  requestTimeoutMs: 30000

webUI:
  enabled: true
  port: 3001
  host: "0.0.0.0"
  apiKey: "${ADMIN_API_KEY}"

logging:
  level: "info"
  format: "json"
```

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

### Logging System

The proxy captures all server output to rotating log files:
- **Location**: `~/.mcp-proxy/logs/{server-name}/server.log`
- **Format**: `[timestamp] [STDOUT|STDERR] message`
- **Rotation**: Automatic at 10MB, 2-day retention
- **API Access**: 
  - `GET /api/logs/{server}?lines=N&type=stdout|stderr`
  - `GET /api/logs/{server}/stream` (Server-Sent Events)

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
- **Yew**: Rust/WASM framework for the web UI

## Development

### Building with Nix (Recommended)

The project includes a Nix flake for reproducible builds and development environments:

```bash
# Enter development shell with all tools
nix develop

# Build the project
nix build

# Build for specific platforms
nix build .#x86_64-linux
nix build .#aarch64-linux
nix build .#x86_64-darwin    # macOS only
nix build .#aarch64-darwin    # macOS only

# Build Docker image
nix build .#docker

# Run directly
nix run github:zach-source/mcp-rust-proxy
```

#### Using direnv (Automatic Environment)

```bash
# Install direnv: https://direnv.net
direnv allow

# Now all tools are automatically available when you cd into the project
```

#### Setting up Cachix (For Faster Builds)

To use the binary cache for faster builds:

```bash
# Install cachix
nix-env -iA cachix -f https://cachix.org/api/v1/install

# Use the project's cache
cachix use mcp-rust-proxy
```

For maintainers building and pushing to cache:

```bash
# Build and push to cache
nix build .#x86_64-linux | cachix push mcp-rust-proxy
```

### Building with Cargo

```bash
# Build without UI (faster for development)
cargo build --release

# Build with UI (requires trunk)
BUILD_YEW_UI=1 cargo build --release
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
├── logging/      # File-based logging system
├── web/          # Web UI and REST API
└── main.rs       # Application entry point

yew-ui/           # Rust/WASM web UI
├── src/
│   ├── components/  # Yew components
│   ├── api/        # API client and WebSocket handling
│   └── types/      # Shared types
└── style.css       # UI styles
```

## Monitoring and Debugging

### Viewing Logs

1. **Web UI**: Click "Logs" button for any server to view real-time logs
2. **Files**: Check `~/.mcp-proxy/logs/{server-name}/server.log`
3. **API**: `curl http://localhost:3001/api/logs/server-name?lines=50`
4. **Stream**: `curl http://localhost:3001/api/logs/server-name/stream`

### Metrics

Prometheus metrics available at `/metrics`:
- `mcp_proxy_requests_total`
- `mcp_proxy_request_duration_seconds`
- `mcp_proxy_active_connections`
- `mcp_proxy_server_restarts_total`

### Health Checks

Configure health checks to monitor server availability:

```yaml
healthChecks:
  critical-server:
    enabled: true
    intervalSeconds: 30
    timeoutMs: 5000
    threshold: 3  # Failures before marking unhealthy
```

## License

MIT