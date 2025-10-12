# Official MCP Rust Proxy Plugins

This directory contains officially maintained plugins that ship with MCP Rust Proxy. These plugins are production-ready, tested, and recommended for common use cases.

## Plugin Inventory

### 1. Curation Plugin (`curation-plugin.js`)

**Purpose**: AI-powered documentation content reduction

**Features:**
- Reduces documentation size by 60-80% using Claude
- Preserves code examples and key facts
- Considers user query context for relevance
- Graceful fallback on errors

**Phase**: Response
**Requirements**: None (uses Claude Agent SDK with CLI session)

**Configuration:**
```yaml
plugins:
  servers:
    context7:
      response:
        - name: curation-plugin
          order: 1
          enabled: true
          timeoutMs: 60000  # AI calls need longer timeout
```

---

### 2. Security Plugin (`security-plugin.js`)

**Purpose**: Request validation and sensitive data blocking

**Features:**
- Blocks passwords, API keys, secrets, tokens
- Detects bearer tokens and private keys
- Only processes request phase
- Audit logging to stderr

**Phase**: Request  
**Requirements**: None

**Configuration:**
```yaml
plugins:
  servers:
    filesystem:
      request:
        - name: security-plugin
          order: 1
          enabled: true
          timeoutMs: 5000
```

**Detected Patterns:**
- `password: value`
- `api_key: value` or `api-key: value`
- `secret: value`
- `token: value`
- `Bearer <token>`
- `-----BEGIN PRIVATE KEY-----`

---

### 3. Prompt Injection Scanner (`prompt-injection-scanner.js`)

**Purpose**: Detect and sanitize prompt injection attacks in LLM responses

**Features:**
- Scans response output for injection patterns
- Detects system prompt overrides
- Identifies role manipulation attempts
- Catches command injection and jailbreak patterns
- Sanitizes detected patterns (permissive mode)
- Can be configured to block (strict mode)

**Phase**: Response  
**Requirements**: None

**Configuration:**
```yaml
plugins:
  servers:
    llm-server:
      response:
        - name: prompt-injection-scanner
          order: 2  # After curation
          enabled: true
          timeoutMs: 5000
```

**Detected Patterns:**
- "Ignore previous instructions"
- "You are now..."
- "Act as..."
- "DAN mode", "Developer mode"
- Command injection: `sudo`, `rm -rf`, `$(...)`, backticks
- Data exfiltration: "Send to", "POST to"
- Context poisoning: "Forget everything", "Reset context"

---

## Installation

### 1. Install Dependencies

```bash
cd src/plugins/official
npm install
```

This installs:
- `@anthropic-ai/claude-agent-sdk` - For curation-plugin (uses CLI session, no API key needed)

### 2. Configure Logging (Optional)

Plugins automatically log to:
- **Console**: stderr (always enabled)
- **Log files**: `~/.mcp-proxy/plugin-logs/{plugin-name}.log`

Control logging behavior:
```bash
# Set log level (DEBUG, INFO, WARN, ERROR)
export PLUGIN_LOG_LEVEL=DEBUG

# View logs in real-time
tail -f ~/.mcp-proxy/plugin-logs/*.log

# Or monitor all plugin activity
cargo run -- --config config.yaml 2>&1 | grep "\[security-plugin\|curation-plugin\|prompt-injection-scanner\]"
```

**Log Features:**
- Structured JSON metadata for easy parsing
- Automatic rotation at 10MB
- Keeps last 5 rotated files
- Per-plugin log files
- Performance metrics (duration, content length)

### 3. Configure Plugins

Use the configuration examples in `examples/configs/official-plugins.yaml` or add to your existing config:

```yaml
plugins:
  pluginDir: ./src/plugins/official
  servers:
    your-server:
      request:
        - name: security-plugin
          order: 1
          enabled: true
      response:
        - name: curation-plugin
          order: 1
          enabled: true
        - name: prompt-injection-scanner
          order: 2
          enabled: true
```

---

## Security Considerations

### Defense in Depth

Recommended plugin layering:

**Request Path:**
```
Client Request 
  → security-plugin (validate input)
  → MCP Server
```

**Response Path:**
```
MCP Server Response
  → curation-plugin (reduce size)
  → prompt-injection-scanner (sanitize output)
  → Client
```

### Audit Logging

All official security plugins log to stderr:

```bash
# Monitor security events
cargo run -- --config config.yaml 2>&1 | grep "\[SECURITY\]"
```

### Production Deployment

See `docs/PLUGIN_DEPLOYMENT.md` for:
- Performance tuning
- Monitoring setup
- Incident response
- Capacity planning

---

## Testing

All official plugins are tested in:
- `tests/plugin_curation_test.rs`
- `tests/plugin_security_test.rs`
- `tests/plugin_prompt_injection_test.rs`

Run tests:
```bash
cargo test plugin_
```

---

## Maintenance

These plugins are officially maintained and will receive:
- Security updates
- Performance improvements
- Bug fixes
- Compatibility updates

For issues or feature requests, open a GitHub issue.

---

## Version

**Plugin Version**: 1.0.0  
**Compatible with**: MCP Rust Proxy v0.1.0+  
**Last Updated**: 2025-10-11
