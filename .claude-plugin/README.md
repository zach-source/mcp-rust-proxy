# MCP Proxy with Context Tracing - Claude Code Plugin

## Overview

Transform Claude into a **self-aware, self-improving AI** with automatic context provenance tracking and quality feedback loops.

This plugin provides:
- 🔍 **Multi-Server MCP Proxy** - Aggregate multiple MCP servers into one
- 📊 **Automatic Context Tracking** - Every tool call traced with full lineage
- 🧠 **Self-Awareness** - Claude can inspect its own context usage
- 📈 **Quality Feedback Loop** - Continuous improvement through ratings
- 🎯 **Session Management** - Group related responses in conversations

## Quick Start

### 1. Install the Plugin

```bash
# Clone or download this repository
git clone https://github.com/zach-source/mcp-rust-proxy
cd mcp-rust-proxy/mcp-proxy-feature-traced-context

# Build the proxy
cargo build

# Install as Claude Code plugin
# (Plugin auto-detected when in this directory)
```

### 2. Configure Your MCP Servers

Edit `mcp-proxy-config.yaml` to add your MCP servers:

```yaml
servers:
  memory:
    command: "mcp-server-memory"
    args: []
    transport:
      type: stdio

  filesystem:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/files"]
    transport:
      type: stdio

contextTracing:
  enabled: true
  sqlitePath: /Users/you/.mcp-proxy/context-tracing.db
```

### 3. Use with Claude

The plugin automatically activates when you run Claude in this directory. All MCP servers will be aggregated through the proxy with automatic context tracking enabled.

## Features

### 10 Tracing Tools

**Session Management:**
- `/mcp-proxy:start-session "user question"` - Begin conversation tracking
- `/mcp-proxy:end-session [score]` - End with optional feedback
- `mcp__proxy__tracing__link_responses` - Chain responses together
- `mcp__proxy__tracing__record_action` - Log custom events
- `mcp__proxy__tracing__get_session_summary` - Session analytics

**Provenance & Quality:**
- `/mcp-proxy:show-trace [response_id]` - View lineage manifests
- `/mcp-proxy:give-feedback <score> <comment>` - Submit quality ratings
- `/mcp-proxy:quality-report` - Analytics and trends
- `mcp__proxy__tracing__query_context_impact` - Assess context impact
- `mcp__proxy__tracing__get_evolution_history` - Version tracking

### 4 Quality Resources

Auto-enriched context (like Context7):
- `trace://quality/top-contexts` - High-rated contexts
- `trace://quality/deprecated-contexts` - Low-rated contexts
- `trace://quality/recent-feedback` - Feedback trends
- `trace://stats/cache` - Performance metrics

### Automatic Hooks

**Session Start:**
- Injects: "🔍 Context Tracing Active"
- Creates session ID
- Logs conversation start

**Post Tool Use:**
- Captures response_id
- Notifies: "📊 Response tracked: resp_xyz"
- Enables easy feedback

**Session End:**
- Prompts for feedback
- Reminds about quality improvement
- Cleans up temp files

## How It Works

### Automatic Tracking

```
You use a tool → Proxy tracks → Response ID generated
                      ↓
                Context recorded from backend
                      ↓
                Lineage manifest stored
                      ↓
                You can query: /mcp-proxy:show-trace
```

### Feedback Loop

```
You rate response: /mcp-proxy:give-feedback 0.9 "Great!"
    ↓
Score propagates to ALL contributing contexts
    ↓
Context aggregate scores updated
    ↓
Future responses prioritize high-scoring contexts
    ↓
Continuous improvement! 🔄
```

## Use Cases

### 1. Understand Your Context Sources

```
/mcp-proxy:show-trace

→ See which docs/tools/memory influenced your response
→ Understand where information came from
→ Verify accuracy of sources
```

### 2. Improve AI Quality Over Time

```
# After good response
/mcp-proxy:give-feedback 0.9 "Code worked perfectly!"

# After poor response
/mcp-proxy:give-feedback -0.5 "Had errors, needed fixes"

→ Contexts learn from feedback
→ Good sources get prioritized
→ Poor sources get deprecated
```

### 3. Track Conversation Sessions

```
/mcp-proxy:start-session "Build a web scraper"

... multiple tool calls ...

/mcp-proxy:end-session 0.8 "Task completed successfully"

→ All responses grouped by session
→ Session-level analytics available
→ Pattern learning across conversations
```

### 4. Multi-Server Aggregation

The proxy automatically aggregates all your MCP servers:

```
Available Tools:
  mcp__proxy__memory__create_entities
  mcp__proxy__filesystem__read_file
  mcp__proxy__git__commit
  mcp__proxy__context7__get_docs
  mcp__proxy__tracing__get_trace
  ... and more!
```

No naming conflicts - everything prefixed!

## Configuration

### Enable/Disable Tracing

In `mcp-proxy-config.yaml`:

```yaml
contextTracing:
  enabled: true  # Set to false to disable
  retentionDays: 90  # How long to keep data
  cacheSize: 10000  # In-memory cache size
```

### Configure Hooks

Hooks are defined in `.claude-plugin/plugin.json` and auto-load when the plugin is active.

## Advanced Features

### Database Schema

5 tables with full ACID guarantees:
- `responses` - All tracked responses
- `context_units` - Information sources
- `lineage` - Context → Response relationships
- `feedback` - Quality ratings
- `lineage_manifests` - Complete provenance trees

### Storage Architecture

- **Hybrid**: DashMap (hot cache) + SQLite (persistence)
- **WAL Mode**: Concurrent reads
- **Indexes**: Optimized queries (< 5s for 100K responses)
- **Retention**: Automatic cleanup after 90 days

### Weight Calculation

Multi-factor composite scoring:
- 40% Retrieval relevance
- 30% Recency (exponential decay)
- 20% Type priority (System > User > External)
- 10% Content length

## Testing

Run integration tests:

```bash
# Unit + integration tests
cargo test

# Automated headless test
./tests/automated_tracing_test.sh

# Real Claude CLI test
./tests/real_claude_integration_test.sh
```

## Documentation

- `README.md` - Main documentation
- `CLAUDE.md` - Agent best practices
- `CLAUDE_HOOKS_INTEGRATION.md` - Hooks details
- `TRACING_TOOLS_QUICKSTART.md` - Quick reference
- `COMPLETE_INTEGRATION_SUMMARY.md` - Full implementation guide

## License

MIT - See LICENSE file

## Contributing

This plugin is part of the MCP Rust Proxy project. Contributions welcome!

---

**Transform Claude into a self-aware, self-improving AI!** 🚀
