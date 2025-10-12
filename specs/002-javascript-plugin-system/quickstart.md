# Quick Start Guide: JavaScript Plugin System

**Feature Branch**: `002-javascript-plugin-system`
**Date**: 2025-10-10

Get started with the MCP Proxy JavaScript plugin system in 5 minutes.

---

## Prerequisites

- MCP Rust Proxy installed and configured
- Node.js 18+ installed (`node --version`)
- Basic knowledge of JavaScript/Node.js

---

## Step 1: Create Your First Plugin

Create a simple echo plugin to understand the basics:

```bash
# Create plugin directory
mkdir -p plugins

# Create echo plugin
cat > plugins/echo.js << 'EOF'
#!/usr/bin/env node

async function main() {
  // Read JSON input from stdin
  const input = JSON.parse(await readStdin());

  // Echo back with timestamp
  const output = {
    text: input.rawContent,
    continue: true,
    metadata: {
      processedAt: new Date().toISOString(),
      originalLength: input.rawContent.length
    }
  };

  // Write JSON output to stdout
  console.log(JSON.stringify(output));
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString('utf8');
}

main().catch(err => {
  console.error(JSON.stringify({
    text: "",
    continue: false,
    error: err.message
  }));
  process.exit(1);
});
EOF

# Make executable
chmod +x plugins/echo.js
```

---

## Step 2: Test Plugin Locally

Test your plugin before integrating with the proxy:

```bash
# Test input
echo '{
  "toolName": "test/echo",
  "rawContent": "Hello from plugin!",
  "metadata": {
    "requestId": "test-123",
    "timestamp": "2025-10-10T12:00:00Z",
    "serverName": "test",
    "phase": "response"
  }
}' | node plugins/echo.js
```

**Expected Output**:
```json
{"text":"Hello from plugin!","continue":true,"metadata":{"processedAt":"2025-10-10T12:00:01.234Z","originalLength":19}}
```

---

## Step 3: Configure Proxy

Add plugin configuration to your MCP proxy config file:

### Option A: YAML Configuration (`config.yaml`)

```yaml
# Existing MCP server configuration
servers:
  - name: context7
    command: npx
    args:
      - -y
      - @context7/mcp-server
    # ... rest of server config

# NEW: Plugin configuration
plugins:
  pluginDir: ./plugins              # Directory containing plugin files
  nodeExecutable: node               # Path to Node.js (default: "node")
  maxConcurrentExecutions: 10        # Max concurrent plugin processes
  poolSizePerPlugin: 5               # Warm processes per plugin
  defaultTimeoutMs: 30000            # 30s timeout

  servers:
    context7:                        # Apply plugins to context7 server
      response:                      # Response phase (after server returns)
        - name: echo                 # Plugin name (matches echo.js)
          order: 1                   # Execution order
          enabled: true              # Enable/disable
```

### Option B: JSON Configuration (`config.json`)

```json
{
  "servers": [...],
  "plugins": {
    "pluginDir": "./plugins",
    "servers": {
      "context7": {
        "response": [
          {
            "name": "echo",
            "order": 1,
            "enabled": true
          }
        ]
      }
    }
  }
}
```

---

## Step 4: Start Proxy and Test

```bash
# Start proxy with plugin configuration
cargo run -- --config config.yaml

# In another terminal, test with an MCP client
# The echo plugin will process all responses from context7 server
```

---

## Step 5: Create a Curation Plugin (AI-Powered)

Build a more advanced plugin that uses AI to reduce token usage:

```bash
# Create curation plugin with dependencies
cat > plugins/package.json << 'EOF'
{
  "name": "mcp-plugins",
  "type": "module",
  "dependencies": {
    "@anthropic-ai/sdk": "^0.32.0"
  }
}
EOF

npm install --prefix plugins

cat > plugins/curate.js << 'EOF'
#!/usr/bin/env node

import Anthropic from '@anthropic-ai/sdk';

const client = new Anthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
});

async function main() {
  const input = JSON.parse(await readStdin());

  // Only curate if maxTokens is specified
  if (!input.maxTokens) {
    console.log(JSON.stringify({
      text: input.rawContent,
      continue: true
    }));
    return;
  }

  try {
    // Use Claude to curate content
    const message = await client.messages.create({
      model: 'claude-3-5-sonnet-20241022',
      max_tokens: 4000,
      messages: [{
        role: 'user',
        content: `Curate this documentation to fit within ${input.maxTokens} tokens. Remove boilerplate, keep code examples and key facts.

User query: ${input.metadata.userQuery || 'N/A'}

Documentation:
${input.rawContent}`
      }]
    });

    const curated = message.content[0].text;

    console.log(JSON.stringify({
      text: curated,
      continue: true,
      metadata: {
        reduction: ((1 - curated.length / input.rawContent.length) * 100).toFixed(1) + '%'
      }
    }));

  } catch (error) {
    // On error, return original content
    console.log(JSON.stringify({
      text: input.rawContent,
      continue: false,
      error: `Curation failed: ${error.message}`
    }));
  }
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString('utf8');
}

main().catch(err => {
  console.error(JSON.stringify({
    text: "",
    continue: false,
    error: err.message
  }));
  process.exit(1);
});
EOF

chmod +x plugins/curate.js
```

Update config to use curation plugin:

```yaml
plugins:
  pluginDir: ./plugins
  servers:
    context7:
      response:
        - name: curate              # Use curation plugin
          order: 1
          enabled: true
          timeoutMs: 45000           # Override timeout for AI calls
```

Set API key and restart:

```bash
export ANTHROPIC_API_KEY=your_api_key_here
cargo run -- --config config.yaml
```

---

## Common Patterns

### 1. Chain Multiple Plugins

```yaml
plugins:
  servers:
    context7:
      response:
        - name: curate              # First: reduce size
          order: 1
          enabled: true
        - name: format              # Second: format markdown
          order: 2
          enabled: true
```

Plugins execute in `order`, each receiving output of the previous.

### 2. Request Phase Security Plugin

```javascript
// plugins/security.js
async function main() {
  const input = JSON.parse(await readStdin());

  if (input.metadata.phase !== 'request') {
    console.log(JSON.stringify({ text: input.rawContent, continue: true }));
    return;
  }

  // Block requests with sensitive patterns
  if (/password|secret|api[_-]?key/i.test(input.rawContent)) {
    console.log(JSON.stringify({
      text: "[BLOCKED]",
      continue: false,
      error: "Sensitive data detected"
    }));
    return;
  }

  console.log(JSON.stringify({ text: input.rawContent, continue: true }));
}
```

### 3. Conditional Processing

```javascript
// Only process specific tools
if (input.toolName !== 'context7/get-docs') {
  console.log(JSON.stringify({ text: input.rawContent, continue: true }));
  return;
}

// Process only if content is large
if (input.rawContent.length < 10000) {
  console.log(JSON.stringify({ text: input.rawContent, continue: true }));
  return;
}

// ... actual plugin logic
```

---

## Testing Your Plugins

### Unit Tests with Node.js Native Test Runner

```bash
# Create test file
cat > plugins/test.js << 'EOF'
import { test } from 'node:test';
import assert from 'node:assert';
import { execFileSync } from 'node:child_process';

test('echo plugin preserves content', () => {
  const input = {
    toolName: 'test',
    rawContent: 'Test content',
    metadata: {
      requestId: '1',
      timestamp: new Date().toISOString(),
      serverName: 'test',
      phase: 'response'
    }
  };

  const output = execFileSync('node', ['./echo.js'], {
    input: JSON.stringify(input),
    encoding: 'utf-8',
    cwd: './plugins'
  });

  const result = JSON.parse(output);
  assert.strictEqual(result.text, 'Test content');
  assert.strictEqual(result.continue, true);
});
EOF

# Run tests
node --test plugins/test.js
```

---

## Debugging Tips

### 1. Use stderr for Debug Logs

```javascript
console.error('[DEBUG] Received input:', JSON.stringify(input, null, 2));
console.log(JSON.stringify(output));  // stdout for result only
```

### 2. Check Proxy Logs

```bash
# Proxy logs show plugin execution details
cargo run -- --config config.yaml 2>&1 | grep plugin
```

### 3. Test Plugin Isolation

```bash
# Test plugin directly with sample input
cat test-input.json | node plugins/your-plugin.js
```

---

## Troubleshooting

### Plugin Not Executing

**Check**:
1. Plugin file exists in `pluginDir` ✓
2. Plugin name in config matches filename (without `.js`) ✓
3. Plugin is `enabled: true` ✓
4. Server name matches configured MCP server ✓

### Timeout Errors

**Solutions**:
- Increase timeout in config: `timeoutMs: 60000`
- Optimize plugin logic (reduce AI calls)
- Use process pooling (configured automatically)

### Invalid JSON Output

**Fix**:
```javascript
// ❌ Bad
console.log(output);

// ✅ Good
console.log(JSON.stringify(output));
```

### Plugin Errors Not Visible

**Solution**: Check stderr output:
```bash
cargo run -- --config config.yaml 2>&1 | tee proxy.log
```

---

## Next Steps

1. **Review [Plugin API Contract](./contracts/plugin-api.md)** for full schema details
2. **Explore [Data Model](./data-model.md)** to understand plugin architecture
3. **Read [Research](./research.md)** for implementation best practices
4. **Check [Implementation Plan](./plan.md)** for roadmap and phases

---

## Production Checklist

Before deploying plugins to production:

- [ ] All plugins tested with unit tests
- [ ] Error handling verified (plugin returns original on failure)
- [ ] Timeout values tuned for expected workload
- [ ] API keys stored securely (environment variables, not hardcoded)
- [ ] Proxy logs monitored for plugin errors
- [ ] Process pool sizes configured appropriately
- [ ] Plugin execution metrics tracked (latency, error rate)

---

**Quick Start Complete!** You now have a working plugin system. Happy plugin development! 🚀
