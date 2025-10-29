# MCP Rust Proxy - Project Snapshot
**Date**: October 28, 2025
**Branch**: main
**Commit**: 090ef67

## Current State Summary

### ✅ Project Health
- **Build Status**: ✅ Passing (with Yew UI)
- **Test Status**: ✅ 108 tests passing, 0 failed, 2 ignored
- **Git Status**: Clean (all changes committed and pushed)
- **Remote**: Up to date with origin/main

### 🎯 Core Capabilities

#### 1. Multi-Server MCP Proxy
- Aggregates multiple MCP servers into unified interface
- Supports stdio, HTTP-SSE, and WebSocket transports
- Connection pooling with automatic reconnection
- Health monitoring with configurable intervals
- Tool name prefixing: `mcp__proxy__{server}__{tool}`

#### 2. Protocol Version Support
- **Versions Supported**: 2024-11-05, 2025-03-26, 2025-06-18
- Auto-detection during initialize handshake
- Bidirectional translation between any version pair
- Zero-copy optimization when versions match
- Deprecation warnings for 2024-11-05

#### 3. AI Context Tracing Framework
- **Provenance Tracking**: Complete lineage manifests
- **Feedback System**: Quality ratings propagate to contributing contexts
- **Storage**: Hybrid DashMap + SQLite with WAL mode
- **5 MCP Tools** for explicit operations
- **4 MCP Resources** for automatic context enrichment
- **Claude Code Hooks**: Session management and feedback collection

#### 4. Aggregator Plugin (NEW - Latest Feature)
- **Technology**: Claude Agent SDK + Node.js plugin
- **Purpose**: Intelligent multi-server context aggregation
- **Smart Server Selection**:
  - `context7`: Default (documentation/libraries)
  - `serena`: Code-related queries
  - `filesystem`: File operations
  - `fetch`: Web/URL operations
- **Tool Usage Enforcement**: Explicit directives to use MCP tools over training data
- **Comprehensive Logging**: Tool usage tracking, server selection reasoning
- **Exposed As**: `mcp__proxy__aggregator__context_aggregator` MCP tool

#### 5. Web UI (Yew/WASM)
- Real-time server status via WebSocket
- Log streaming via Server-Sent Events
- Built with Yew framework (Rust → WASM)
- Automatic rebuild with `BUILD_YEW_UI=1`

### 📁 Key Files & Locations

#### Core Source
```
src/
├── proxy/
│   ├── mod.rs                    # Main proxy request handler
│   └── aggregator_tools.rs       # Aggregator server selection logic
├── server/                       # Server lifecycle management
├── transport/                    # stdio, HTTP-SSE, WebSocket
├── protocol/                     # Version translation
├── state/                        # DashMap-based state
├── context/                      # Context tracing framework
├── web/                          # API endpoints
└── plugins/
    └── official/
        └── aggregator-plugin.js  # Claude SDK integration
```

#### Configuration & Testing
```
mcp-proxy-config.yaml              # Main config
test-aggregator-e2e.sh             # Aggregator end-to-end tests
test-aggregator-with-mcp-calls.sh  # MCP tool usage verification
mock-logging-server.py             # Log streaming test server
```

#### Logs
```
~/.mcp-proxy/logs/{server}/server.log       # Server stdout/stderr
~/.mcp-proxy/plugin-logs/aggregator-plugin.log  # Plugin logs
```

### 🔧 Recent Commits (Last 10)

```
090ef67  feat(aggregator): Enhance MCP tool usage enforcement and server selection
3718f95  feat: Aggregator Plugin - Claude Agent SDK integration (#2)
f05df5e  Merge branch '003-mcp-protocol-support'
e83ee71  chore: Remove test scripts
bebfb47  fix(protocol): Correct initialized notification method name
dec54ac  feat(protocol): T063-T066 Performance benchmarks complete - 258 tests
3257586  feat(protocol): T057-T072 Polish complete - documentation updated
c6aa651  feat(protocol): T050-T056 US4 complete - version reporting in API
baf5d22  feat(protocol): T048-T049 US3 complete - adapter integration
00dc0a1  feat(protocol): T047 Adapter integration complete
```

### 🎨 Architecture Highlights

#### Plugin System
- Plugins written in JavaScript/Node.js
- Communicate via stdio (JSON-RPC)
- Can spawn and manage MCP servers
- Aggregator plugin uses Claude Agent SDK to orchestrate queries

#### Server Selection Intelligence
Query content analysis determines which MCP servers to use:
- **Code keywords**: `code, function, class, method, implementation, bug, error, test, variable, struct, enum, interface` → Add serena
- **File keywords**: `file, read, write, directory, folder, path` → Add filesystem
- **Web keywords**: `url, website, web, http, fetch, download` → Add fetch
- **Default**: Always include context7

#### Tool Usage Enforcement
System prompt includes:
```
tool use: [server1, server2, ...]

CRITICAL: You MUST use the MCP tools listed above to answer the user's query.
DO NOT rely on your training data.
```

### 📊 Metrics & Performance

- **258 total tests** (including protocol performance benchmarks)
- **108 unit/integration tests** currently passing
- **2 ignored tests** (require test infrastructure modifications)
- Log rotation at 10MB, 2-day retention
- Prometheus-compatible metrics collection

### 🔮 Known Limitations & TODOs

#### Current Gaps
1. Aggregator plugin Claude responses don't always use MCP tools (monitoring ongoing)
2. Some protocol compliance tests stubbed (T069-T072)
3. Health check test ignored (requires mock time advancement)

#### Future Enhancements
- More sophisticated query analysis for server selection
- Caching layer for repeated aggregator queries
- Support for additional MCP protocol versions as they're released
- Enhanced UI with query builder for aggregator

### 🚀 Quick Start Commands

```bash
# Build
cargo build

# Build with UI
BUILD_YEW_UI=1 cargo build

# Run proxy
cargo run -- --config mcp-proxy-config.yaml

# Run as MCP server (stdio mode)
cargo run -- --config mcp-proxy-config.yaml --stdio

# Run tests
cargo test

# Test aggregator
./test-aggregator-e2e.sh
./test-aggregator-with-mcp-calls.sh

# Format code
cargo fmt
npx prettier --write src/plugins/**/*.js
```

### 📚 Documentation Files

- `README.md` - Main project documentation
- `CLAUDE.md` - Development guide (just updated)
- `MCP_PROTOCOL_COMPLIANCE_PLAN.md` - Protocol implementation details
- `MCP_VERSION_COMPATIBILITY_PLAN.md` - Version support strategy
- `MCP_ADVANCED_FEATURES_PLAN.md` - Advanced feature roadmap
- `CLAUDE_HOOKS_INTEGRATION.md` - Claude Code hooks documentation
- `YEW_UI_INTEGRATION.md` - Web UI architecture

### 🎯 Next Steps / Recommended Focus

1. **Monitor Aggregator Effectiveness**: Run test scripts, verify MCP tool usage
2. **Expand Test Coverage**: Address ignored tests, add more integration tests
3. **Performance Optimization**: Profile aggregator queries, optimize context tracing
4. **UI Enhancements**: Add aggregator query interface to web UI
5. **Documentation**: Keep CLAUDE.md updated as features evolve

---

**Generated**: October 28, 2025
**Tool**: Claude Code (Sonnet 4.5)
**Purpose**: Project state preservation for future sessions
