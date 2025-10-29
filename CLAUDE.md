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
- **Protocol Version Support** (`src/protocol/`): Multi-version MCP protocol support with automatic translation
- **Web UI** (`yew-ui/`): Rust/WASM frontend built with Yew framework

### Protocol Version Support
- **Supported Versions**: 2024-11-05, 2025-03-26, 2025-06-18
- **Auto-Detection**: Protocol version detected during initialize handshake
- **Bidirectional Translation**: Automatic message translation between any version pair
- **Zero-Copy Optimization**: Pass-through mode when versions match
- **API Integration**: Protocol version visible in `/api/servers` and `/api/servers/{name}` responses
- **Structured Logging**: Version negotiation logged with structured fields (server_name, protocol_version)
- **Deprecation Warnings**: Logs WARN for servers using 2024-11-05 (deprecated)

### Aggregator Plugin (Claude Agent SDK Integration)
- **Plugin Location**: `src/plugins/official/aggregator-plugin.js`
- **Rust Integration**: `src/proxy/aggregator_tools.rs`
- **Purpose**: Intelligent multi-server context aggregation using Claude to query multiple MCP servers
- **Key Features**:
  - Uses `@anthropic-ai/claude-agent-sdk` to orchestrate queries across multiple MCP servers
  - Enforces MCP tool usage over training data with explicit `tool use: [servers]` directives
  - Intelligent server selection based on query keywords:
    - **context7**: Default for documentation/library information
    - **serena**: Auto-selected for code-related queries (function, class, method, etc.)
    - **filesystem**: Auto-selected for file operations (file, directory, path, etc.)
    - **fetch**: Auto-selected for web operations (url, website, http, etc.)
  - Comprehensive logging for debugging tool usage and server selection
  - Tracks aggregation statistics (response sizes, processing time, tool calls)

**Testing Aggregator**:
- `test-aggregator-e2e.sh` - Basic end-to-end test with code and documentation queries
- `test-aggregator-with-mcp-calls.sh` - Comprehensive test verifying MCP tool usage
- Exposed as MCP tool: `mcp__proxy__aggregator__context_aggregator`

**Key Considerations**:
- System prompt emphasizes CRITICAL requirement to use MCP tools
- Plugin spawns actual MCP server processes (same configs as proxy)
- Logs stored at `~/.mcp-proxy/plugin-logs/aggregator-plugin.log`
- Requires Claude API key in environment or config

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
- Use `test-aggregator-e2e.sh` for testing aggregator plugin end-to-end
- Use `test-aggregator-with-mcp-calls.sh` for verifying MCP tool usage in aggregator

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
- **Current Test Status**: 108 tests passing, 0 failed, 2 ignored

## Recent Major Features (Latest First)
1. **Aggregator Plugin Enhancements** (commit 090ef67)
   - Enhanced MCP tool usage enforcement with explicit directives
   - Expanded server selection heuristics (filesystem, fetch)
   - Added comprehensive test scripts

2. **Aggregator Plugin Integration** (commit 3718f95)
   - Claude Agent SDK integration for multi-server context optimization
   - Intelligent query routing based on content analysis
   - Full logging and statistics tracking

3. **MCP Protocol Version Support** (commits dec54ac - f05df5e)
   - Multi-version protocol support (2024-11-05, 2025-03-26, 2025-06-18)
   - Automatic bidirectional translation
   - 258 total tests including performance benchmarks

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

### Protocol Version Issues
- **Check server version**: Look for "Protocol version negotiated successfully" in logs with `protocol_version` field
- **Version mismatch**: Proxy automatically translates between versions - no manual intervention needed
- **Missing tools**: If tools count is low, check if servers completed initialization (state should be "Ready")
- **Deprecation warnings**: Servers using 2024-11-05 will show WARN logs suggesting upgrade to 2025-06-18
- **Debug translation**: Enable DEBUG logs to see translation operations with `source_version` and `target_version` fields

## Testing with Playwright MCP
Use the Playwright MCP server to test UI functionality:
1. Ensure the server is running
2. Use `mcp__playwright__browser_navigate` to load the UI
3. Use `mcp__playwright__browser_click` to interact with elements
4. Use `mcp__playwright__browser_snapshot` to verify state

## Context Tracing Tools for LLM Agents

When running as an MCP server (with `--stdio` flag), the proxy exposes context tracing tools that enable LLM agents to track provenance, query impact, and submit quality feedback.

### Available Tracing Tools

#### 1. `mcp__proxy__tracing__submit_feedback`
**When to use:** After completing a task or generating a response, submit feedback to improve context quality.

**Best practices:**
- Submit positive feedback (0.5 to 1.0) when responses are accurate and helpful
- Submit negative feedback (-1.0 to -0.5) when responses contain errors or outdated info
- Submit neutral feedback (-0.2 to 0.2) when responses are acceptable but not exceptional
- Always include `feedback_text` explaining why (helps with debugging)

**Example usage:**
```json
{
  "name": "mcp__proxy__tracing__submit_feedback",
  "arguments": {
    "response_id": "resp_abc123",
    "score": 0.9,
    "feedback_text": "Response provided accurate technical information with working code examples",
    "user_id": "claude-assistant"
  }
}
```

**When NOT to use:**
- Don't submit feedback for your own intermediate thoughts/reasoning
- Only submit for final responses delivered to users
- Skip if response_id is not available

#### 2. `mcp__proxy__tracing__query_context_impact`
**When to use:** Before updating documentation or context, check which past responses depend on it.

**Example:**
```json
{
  "name": "mcp__proxy__tracing__query_context_impact",
  "arguments": {
    "context_unit_id": "ctx_api_docs_v2",
    "min_weight": 0.3,
    "limit": 50
  }
}
```

**Returns:** List of all responses influenced by this context, with contribution weights.

#### 3. `mcp__proxy__tracing__get_trace`
**When to use:** Debug why a specific response was generated by viewing its lineage.

**Formats:**
- `json`: Full structured data
- `tree`: ASCII visualization (best for human review)
- `compact`: Summary with top contributors

**Example:**
```json
{
  "name": "mcp__proxy__tracing__get_trace",
  "arguments": {
    "response_id": "resp_xyz789",
    "format": "tree"
  }
}
```

#### 4. `mcp__proxy__tracing__get_response_contexts`
**When to use:** Inspect which contexts contributed to a response (filtered by type).

**Example:**
```json
{
  "name": "mcp__proxy__tracing__get_response_contexts",
  "arguments": {
    "response_id": "resp_xyz789",
    "type": "External"
  }
}
```

#### 5. `mcp__proxy__tracing__get_evolution_history`
**When to use:** Track how a context unit evolved over time.

**Example:**
```json
{
  "name": "mcp__proxy__tracing__get_evolution_history",
  "arguments": {
    "context_unit_id": "ctx_system_docs"
  }
}
```

### Feedback Workflow for Agents

**Recommended pattern:**
1. Complete user task successfully
2. If a response_id is available, submit feedback reflecting quality
3. Use positive scores for helpful, accurate responses
4. Use negative scores if you detect errors or hallucinations
5. Periodically query impact of frequently-used contexts to identify quality issues

**Scoring guidelines:**
- **+1.0 to +0.7**: Excellent - accurate, complete, very helpful
- **+0.6 to +0.3**: Good - accurate and useful
- **+0.2 to -0.2**: Neutral - acceptable but unremarkable
- **-0.3 to -0.6**: Poor - contains minor errors or outdated info
- **-0.7 to -1.0**: Bad - significantly wrong, misleading, or harmful

**Impact:**
- Feedback propagates to ALL context units that contributed to the response
- Scores are weighted by each context's contribution (higher weight = more impact)
- Aggregate scores guide future context selection and deprecation

## Claude Code Hooks Integration

This project includes Claude Code hooks for automatic session management and feedback collection.

### Available Slash Commands

**`/mcp-proxy:give-feedback <score> [comment]`**
- Submit feedback on your most recent response
- Score: -1.0 (poor) to 1.0 (excellent)
- Example: `/mcp-proxy:give-feedback 0.8 "Accurate and helpful"`

**`/mcp-proxy:show-trace [response_id] [format]`**
- Display lineage for last response or specific response_id
- Formats: json, tree, compact
- Example: `/mcp-proxy:show-trace tree`

**`/mcp-proxy:quality-report`**
- Generate analytics on context quality
- Shows top performers and contexts needing review

### Automatic Hooks

**Session Start:**
- Injects message: "🔍 Context Tracing Active"
- Creates session ID for grouping
- Logs session start

**Post Tool Use:**
- Captures response_id after each backend tool call
- Notifies: "📊 Response tracked: resp_xyz..."
- Enables easy feedback via slash command

**Session End:**
- Prompts for feedback on last response
- Reminds about quality improvement
- Cleans up temp files

See [CLAUDE_HOOKS_INTEGRATION.md](CLAUDE_HOOKS_INTEGRATION.md) for complete documentation.
