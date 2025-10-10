# Installing MCP Proxy Tracing as a Claude Code Plugin

## Installation Methods

### Method 1: Local Plugin (Recommended for Development)

The plugin automatically activates when you run Claude Code in this directory:

```bash
cd /path/to/mcp-proxy-feature-traced-context
claude

# Plugin features automatically available:
# - All slash commands (/mcp-proxy:*)
# - All hooks (session-start, post-tool-use, session-end)
# - MCP server with 10 tracing tools + 4 resources
```

### Method 2: Install from Local Path

```bash
# In Claude Code
/plugin install mcp-proxy-tracing@local /path/to/mcp-proxy-feature-traced-context

# Restart Claude Code
# Plugin now available in all directories
```

### Method 3: Install from Git Repository (Future)

Once published to a marketplace:

```bash
/plugin install mcp-proxy-tracing@github
```

## Prerequisites

### 1. Build the Proxy

```bash
cd /path/to/mcp-proxy-feature-traced-context
cargo build
```

This creates `target/debug/mcp-rust-proxy` binary.

### 2. Configure MCP Servers

Edit `mcp-proxy-config.yaml` to add your MCP servers.

### 3. Create Database Directory

```bash
mkdir -p ~/.mcp-proxy
```

## Verification

### Check Plugin Loaded

```bash
claude

# In Claude, type:
/plugin list

# Should show: mcp-proxy-tracing
```

### Check MCP Server Active

```bash
# List tools - should see tracing tools
/tools

# Should show:
# - mcp__proxy__tracing__start_session
# - mcp__proxy__tracing__get_trace
# - mcp__proxy__tracing__submit_feedback
# ... and more
```

### Check Slash Commands

```bash
# Type / to see available commands
/

# Should show:
# /mcp-proxy:give-feedback
# /mcp-proxy:show-trace
# /mcp-proxy:start-session
# /mcp-proxy:end-session
# /mcp-proxy:quality-report
```

### Check Hooks Active

```bash
# Start a new session
# Should see: "🔍 Context Tracing Active"

# Use any backend tool
# Should see: "📊 Response tracked: resp_xyz..."
```

## Plugin Structure

```
.claude-plugin/
├── plugin.json           Plugin manifest
├── README.md            Plugin documentation
└── commands/
    ├── mcp-proxy-give-feedback.md
    ├── mcp-proxy-show-trace.md
    ├── mcp-proxy-start-session.md
    ├── mcp-proxy-end-session.md
    └── mcp-proxy-quality-report.md

.claude/
├── settings.json        Hook configuration
└── hooks/
    ├── session-start.sh
    ├── post-tool-use.sh
    └── session-end.sh
```

## Configuration

### Plugin Settings

The plugin respects these settings in `.claude-plugin/plugin.json`:

```json
{
  "settings": {
    "contextTracing": {
      "enabled": true,
      "autoNotifications": true,
      "feedbackPrompts": true,
      "sessionTracking": true
    }
  }
}
```

### MCP Server Configuration

The proxy reads from `mcp-proxy-config.yaml`:

```yaml
contextTracing:
  enabled: true
  storageType: hybrid
  sqlitePath: ~/.mcp-proxy/context-tracing.db
  cacheSize: 10000
  cacheTtlSeconds: 604800  # 7 days
  retentionDays: 90
```

## Usage Examples

### Example 1: Simple Session with Feedback

```
You: /mcp-proxy:start-session "Create a Python script"
Claude: Session started: session_abc123

You: Write a Python script that...
Claude: [Creates script using tools]
       📊 Response tracked: resp_xyz789

You: /mcp-proxy:give-feedback 0.9 "Perfect, worked first try!"
Claude: Feedback submitted! 1 context updated, score +0.9

You: /mcp-proxy:end-session 0.9 "Great session"
Claude: Session ended. 3 responses tracked.
```

### Example 2: Quality-Driven Development

```
You: /mcp-proxy:quality-report
Claude: Top Contexts:
        - ctx_python_docs (score: 0.85, 12 uses)
        - ctx_git_commands (score: 0.72, 8 uses)

        Deprecated:
        - ctx_old_api (score: -0.6, 5 uses) ← Needs update!

You: Thanks! Let me update that old API context.
```

### Example 3: Lineage Investigation

```
You: Why did you suggest that specific approach?
Claude: Let me check...

Claude: /mcp-proxy:show-trace

Claude: My response was influenced by:
        - ctx_design_patterns (45%) - System context
        - ctx_your_codebase (35%) - User files
        - ctx_documentation (20%) - External docs

        I prioritized design patterns based on their high quality score (0.88).
```

## Troubleshooting

### Plugin Not Loading

- Ensure `plugin.json` is valid JSON
- Check Claude Code version >= 1.0.0
- Try restarting Claude Code

### MCP Server Not Starting

- Run `cargo build` to compile proxy
- Check logs: `~/.mcp-proxy/session-log.txt`
- Verify config path in `plugin.json`

### Tracking Not Working

- Check database exists: `~/.mcp-proxy/context-tracing.db`
- Verify `contextTracing.enabled: true` in config
- Check logs for initialization errors

### Hooks Not Triggering

- Ensure hooks are executable: `chmod +x .claude/hooks/*.sh`
- Check `.claude/settings.json` is valid
- Verify hook paths in `plugin.json`

## Development

### Testing the Plugin

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test context_integration_test

# Automated tracing tests
./tests/automated_tracing_test.sh

# Real Claude tests (requires Claude CLI)
./tests/real_claude_integration_test.sh
```

### Plugin Development Workflow

1. Modify plugin files
2. Rebuild: `cargo build`
3. Restart Claude Code
4. Test changes

## Support

- Issues: https://github.com/zach-source/mcp-rust-proxy/issues
- Documentation: See docs in repository
- Examples: See `tests/` directory

---

**Make Claude self-aware and self-improving!** 🚀
