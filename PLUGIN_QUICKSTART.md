# Plugin System Quick Start Guide

Get started with the MCP Rust Proxy JavaScript plugin system in 5 minutes.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Your First Plugin](#your-first-plugin)
- [Testing Your Plugin](#testing-your-plugin)
- [Configuring the Proxy](#configuring-the-proxy)
- [Common Use Cases](#common-use-cases)
- [Next Steps](#next-steps)

## Prerequisites

- MCP Rust Proxy installed and running
- Node.js 18+ (`node --version`)
- Basic JavaScript knowledge

## Your First Plugin

### 1. Create Plugin Directory

```bash
mkdir -p plugins
```

### 2. Create a Simple Echo Plugin

```bash
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

chmod +x plugins/echo.js
```

## Testing Your Plugin

### Test Locally Before Integration

```bash
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

**Expected Output:**
```json
{"text":"Hello from plugin!","continue":true,"metadata":{"processedAt":"2025-10-10T12:00:01Z","originalLength":19}}
```

## Configuring the Proxy

### Add Plugin Configuration

Add this to your MCP proxy config file (`config.yaml`):

```yaml
# Existing MCP server configuration
servers:
  - name: context7
    command: npx
    args: ["-y", "@context7/mcp-server"]
    # ... rest of config

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

### Start the Proxy

```bash
cargo run -- --config config.yaml
```

## Common Use Cases

### 1. Content Curation (AI-Powered)

Reduce documentation size by 60-80% using AI:

```bash
# Install dependencies
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

# Create curation plugin
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

**Update config:**
```yaml
plugins:
  servers:
    context7:
      response:
        - name: curate
          order: 1
          enabled: true
          timeoutMs: 45000  # Longer timeout for AI calls
```

**Set API key and run:**
```bash
export ANTHROPIC_API_KEY=your_api_key_here
cargo run -- --config config.yaml
```

### 2. Security Middleware & Request Validation

Block requests containing sensitive data before they reach MCP servers:

```bash
cat > plugins/security.js << 'EOF'
#!/usr/bin/env node

/**
 * Security Plugin - Request Phase Validation
 *
 * This plugin blocks requests containing sensitive patterns like:
 * - Passwords (password: value)
 * - API keys (api_key: value, api-key: value)
 * - Secrets (secret: value)
 * - Tokens (token: value, bearer tokens)
 * - Private keys (-----BEGIN PRIVATE KEY-----)
 */

async function main() {
  const input = JSON.parse(await readStdin());

  // Only process request phase - security doesn't apply to responses
  if (input.metadata.phase !== 'request') {
    console.log(JSON.stringify({ text: input.rawContent, continue: true }));
    return;
  }

  // Define sensitive patterns to detect
  const sensitivePatterns = [
    /password\s*[:=]\s*[^\s]+/i,
    /api[_-]?key\s*[:=]\s*[^\s]+/i,
    /secret\s*[:=]\s*[^\s]+/i,
    /token\s*[:=]\s*[^\s]+/i,
    /bearer\s+[a-zA-Z0-9\-._~+/]+=*/i,
    /-----BEGIN (RSA |DSA )?PRIVATE KEY-----/i,
  ];

  // Check for sensitive data patterns
  for (const pattern of sensitivePatterns) {
    if (pattern.test(input.rawContent)) {
      // Log to stderr for audit trail
      console.error(`[SECURITY] Blocked request with pattern: ${pattern.source}`);

      // Block the request
      console.log(JSON.stringify({
        text: "[BLOCKED]",
        continue: false,
        error: `Security violation: Request contains sensitive data matching pattern ${pattern}`,
        metadata: {
          blockedAt: new Date().toISOString(),
          serverName: input.metadata.serverName,
          toolName: input.toolName
        }
      }));
      return;
    }
  }

  // Request is safe, pass through unchanged
  console.log(JSON.stringify({
    text: input.rawContent,
    continue: true,
    metadata: {
      securityCheck: 'passed',
      checkedAt: new Date().toISOString()
    }
  }));
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

chmod +x plugins/security.js
```

**Config for security:**
```yaml
plugins:
  servers:
    filesystem:
      request:  # Request phase (before forwarding)
        - name: security
          order: 1
          enabled: true
          timeoutMs: 5000  # Quick validation
```

#### Security Plugin Patterns

**Pattern 1: Audit Logging**
```javascript
// Log all sensitive operations to stderr for audit trail
async function main() {
  const input = JSON.parse(await readStdin());

  const auditLog = {
    timestamp: new Date().toISOString(),
    phase: input.metadata.phase,
    serverName: input.metadata.serverName,
    toolName: input.toolName,
    requestId: input.metadata.requestId,
    contentLength: input.rawContent.length
  };

  console.error(`[AUDIT] ${JSON.stringify(auditLog)}`);

  // Pass through unchanged (continue=true)
  console.log(JSON.stringify({
    text: input.rawContent,
    continue: true,
    metadata: { logged: true, loggedAt: new Date().toISOString() }
  }));
}
```

**Pattern 2: Combined Security & Logging**
```yaml
plugins:
  servers:
    filesystem:
      request:
        - name: security      # First: validate security
          order: 1
          enabled: true
        - name: logging       # Second: log if allowed
          order: 2
          enabled: true
```

**Pattern 3: Per-Tool Security Rules**
```javascript
async function main() {
  const input = JSON.parse(await readStdin());

  if (input.metadata.phase !== 'request') {
    console.log(JSON.stringify({ text: input.rawContent, continue: true }));
    return;
  }

  // Apply different rules per tool
  if (input.toolName.includes('write')) {
    // Strict validation for write operations
    if (/\.(exe|sh|bat|cmd)$/i.test(input.rawContent)) {
      console.log(JSON.stringify({
        text: "[BLOCKED]",
        continue: false,
        error: "Executable file write blocked"
      }));
      return;
    }
  }

  if (input.toolName.includes('network')) {
    // Block suspicious network patterns
    if (/192\.168\.|10\.|127\.0\.0\.1/i.test(input.rawContent)) {
      console.log(JSON.stringify({
        text: "[BLOCKED]",
        continue: false,
        error: "Private IP access blocked"
      }));
      return;
    }
  }

  console.log(JSON.stringify({ text: input.rawContent, continue: true }));
}
```

**Security Best Practices:**
1. **Request Phase Only**: Security plugins should only process request phase
2. **Fast Validation**: Keep timeout short (5-10s) for security checks
3. **Audit Trail**: Always log blocked requests to stderr with context
4. **Clear Errors**: Provide helpful error messages explaining why request was blocked
5. **Defense in Depth**: Combine multiple security plugins for layered protection
6. **Phase Detection**: Always check `input.metadata.phase` to skip responses

### 3. Plugin Chaining & Transformations

Chain multiple plugins together for complex transformations. Each plugin receives the output of the previous plugin as its input, enabling powerful composition patterns.

#### Basic Chaining Example

```yaml
plugins:
  servers:
    context7:
      response:
        - name: curate           # First: reduce size with AI
          order: 1
          enabled: true
        - name: path-normalizer  # Second: normalize file paths
          order: 2
          enabled: true
        - name: enrich-metadata  # Third: add metadata
          order: 3
          enabled: true
```

**Execution Flow:**
1. Context7 returns 50KB documentation
2. `curate` reduces to 10KB (output → next input)
3. `path-normalizer` converts Windows paths to Unix format
4. `enrich-metadata` adds processing timestamp
5. Final output returned to client with aggregated metadata

#### Path Normalization Plugin

```bash
cat > plugins/path-normalizer.js << 'EOF'
#!/usr/bin/env node

async function main() {
  const input = JSON.parse(await readStdin());

  let transformedText = input.rawContent;
  let pathsNormalized = 0;

  // Convert Windows paths to Unix format
  const windowsPathPattern = /[A-Z]:\\(?:[^\\/:*?"<>|\r\n]+\\)*[^\\/:*?"<>|\r\n]*/g;
  transformedText = transformedText.replace(windowsPathPattern, (match) => {
    pathsNormalized++;
    // C:\Users\foo\file.txt → /Users/foo/file.txt
    return match.replace(/\\/g, '/').replace(/^[A-Z]:/, '');
  });

  console.log(JSON.stringify({
    text: transformedText,
    continue: true,
    metadata: {
      pathsNormalized,
      processedAt: new Date().toISOString()
    }
  }));
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return Buffer.concat(chunks).toString('utf8');
}

main().catch(err => {
  console.error(JSON.stringify({ text: "", continue: false, error: err.message }));
  process.exit(1);
});
EOF

chmod +x plugins/path-normalizer.js
```

#### Metadata Enrichment Plugin

```bash
cat > plugins/enrich-metadata.js << 'EOF'
#!/usr/bin/env node

async function main() {
  const input = JSON.parse(await readStdin());

  // Add enrichment metadata (pass content through unchanged)
  const enrichment = {
    processedAt: new Date().toISOString(),
    pluginVersion: '1.0.0',
    serverName: input.metadata.serverName,
    toolName: input.toolName,
    contentLength: input.rawContent.length,
    phase: input.metadata.phase
  };

  console.log(JSON.stringify({
    text: input.rawContent,  // Content unchanged
    continue: true,
    metadata: enrichment
  }));
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return Buffer.concat(chunks).toString('utf8');
}

main().catch(err => {
  console.error(JSON.stringify({ text: "", continue: false, error: err.message }));
  process.exit(1);
});
EOF

chmod +x plugins/enrich-metadata.js
```

#### Chaining Patterns

**Pattern 1: Sequential Transformation**
```yaml
# Each plugin transforms the output of the previous plugin
plugins:
  servers:
    context7:
      response:
        - name: curate           # Step 1: Reduce content
          order: 1
        - name: path-normalizer  # Step 2: Transform paths
          order: 2
        - name: enrich-metadata  # Step 3: Add metadata
          order: 3
```

**Pattern 2: Early Termination**
```javascript
// A plugin can stop the chain by returning continue=false
async function main() {
  const input = JSON.parse(await readStdin());

  if (shouldBlock(input)) {
    console.log(JSON.stringify({
      text: "[STOPPED]",
      continue: false,  // Chain stops here
      metadata: { reason: "condition met" }
    }));
    return;
  }

  // Continue to next plugin
  console.log(JSON.stringify({ text: input.rawContent, continue: true }));
}
```

**Pattern 3: Conditional Processing**
```javascript
// Only process specific content types
async function main() {
  const input = JSON.parse(await readStdin());

  // Skip if content is small
  if (input.rawContent.length < 10000) {
    console.log(JSON.stringify({
      text: input.rawContent,
      continue: true,
      metadata: { skipped: true, reason: "content too small" }
    }));
    return;
  }

  // Process large content
  const processed = heavyTransformation(input.rawContent);
  console.log(JSON.stringify({
    text: processed,
    continue: true,
    metadata: { transformed: true }
  }));
}
```

#### Metadata Aggregation

When plugins are chained, metadata from **all executed plugins** is aggregated:

```json
{
  "text": "final transformed content",
  "metadata": {
    "curate": {
      "reduction": "75.3%",
      "originalLength": 50000,
      "curatedLength": 12350
    },
    "path-normalizer": {
      "pathsNormalized": 5,
      "processedAt": "2025-10-10T12:00:01Z"
    },
    "enrich-metadata": {
      "processedAt": "2025-10-10T12:00:02Z",
      "pluginVersion": "1.0.0",
      "contentLength": 12350
    }
  }
}
```

Each plugin's metadata is preserved under a key matching the plugin name.

#### Chaining Best Practices

1. **Order Matters**: Plugins execute in ascending order (1, 2, 3, ...)
2. **Input = Previous Output**: Each plugin receives the `text` from the previous plugin as `rawContent`
3. **Metadata Preserved**: All plugin metadata is collected and included in final response
4. **Early Exit**: Set `continue=false` to stop the chain (useful for validation/blocking)
5. **Error Handling**: If a plugin fails, original content is preserved
6. **Performance**: Keep chains short (3-5 plugins) for best latency
7. **Timeout Tuning**: Set appropriate timeouts per plugin based on workload

#### When to Use Chaining

**Good Use Cases:**
- ✅ Curate → Normalize → Enrich (complementary transformations)
- ✅ Validate → Log → Transform (security + audit + processing)
- ✅ Filter → Reduce → Format (progressive refinement)

**Avoid:**
- ❌ Too many plugins (>5) - increases latency
- ❌ Conflicting transformations (format → un-format)
- ❌ Redundant processing (two curators in sequence)

## Official Plugins

MCP Rust Proxy ships with three officially maintained plugins in `src/plugins/official/`:

### 1. Curation Plugin
**Purpose**: AI-powered documentation reduction (60-80%)
**Phase**: Response
**Requirements**: ANTHROPIC_API_KEY

### 2. Security Plugin
**Purpose**: Block sensitive data in requests (passwords, API keys, tokens)
**Phase**: Request
**Requirements**: None

### 3. Prompt Injection Scanner
**Purpose**: Detect and sanitize prompt injection attacks in LLM responses
**Phase**: Response
**Requirements**: None

**Quick Start with Official Plugins:**
```yaml
plugins:
  pluginDir: ./src/plugins/official
  servers:
    context7:
      response:
        - name: curation-plugin
          order: 1
          enabled: true
        - name: prompt-injection-scanner  # Protection against prompt injection
          order: 2
          enabled: true
    filesystem:
      request:
        - name: security-plugin
          order: 1
          enabled: true
```

See `src/plugins/official/README.md` for complete documentation.

## Next Steps

### Learn More
- **[Official Plugins](src/plugins/official/README.md)** - Production-ready plugins
- **[Plugin API Contract](specs/002-javascript-plugin-system/contracts/plugin-api.md)** - Full schema details
- **[Data Model](specs/002-javascript-plugin-system/data-model.md)** - Plugin architecture
- **[Research](specs/002-javascript-plugin-system/research.md)** - Implementation best practices
- **[Production Deployment](docs/PLUGIN_DEPLOYMENT.md)** - Deployment guide

### Production Checklist
- [ ] All plugins tested with unit tests
- [ ] Error handling verified (plugin returns original on failure)
- [ ] Timeout values tuned for expected workload
- [ ] API keys stored securely (environment variables)
- [ ] Proxy logs monitored for plugin errors
- [ ] Process pool sizes configured appropriately
- [ ] Plugin execution metrics tracked

### Debugging Tips

**1. Use stderr for debug logs:**
```javascript
console.error('[DEBUG] Received input:', JSON.stringify(input, null, 2));
console.log(JSON.stringify(output));  // stdout for result only
```

**2. Check proxy logs:**
```bash
cargo run -- --config config.yaml 2>&1 | grep plugin
```

**3. Test plugin isolation:**
```bash
cat test-input.json | node plugins/your-plugin.js
```

## Troubleshooting

### Plugin Not Executing
**Check:**
1. Plugin file exists in `pluginDir` ✓
2. Plugin name in config matches filename (without `.js`) ✓
3. Plugin is `enabled: true` ✓
4. Server name matches configured MCP server ✓

### Timeout Errors
**Solutions:**
- Increase timeout in config: `timeoutMs: 60000`
- Optimize plugin logic (reduce AI calls)
- Process pooling is configured automatically

### Invalid JSON Output
**Fix:**
```javascript
// ❌ Bad
console.log(output);

// ✅ Good
console.log(JSON.stringify(output));
```

## Plugin I/O Contract

### Input Format
```json
{
  "toolName": "context7/get-docs",
  "rawContent": "content to process",
  "maxTokens": 1200,
  "metadata": {
    "requestId": "req-123",
    "timestamp": "2025-10-10T12:00:00Z",
    "serverName": "context7",
    "phase": "response",
    "userQuery": "user's question"
  }
}
```

### Output Format
```json
{
  "text": "processed content",
  "continue": true,
  "metadata": {
    "your": "custom metadata"
  },
  "error": "optional error message"
}
```

### Rules
- If `error` is present, `continue` must be `false`
- If `continue` is `false`, plugin chain stops
- On any error, original content is preserved

---

**Happy Plugin Development! 🚀**

For support, see [project documentation](CLAUDE.md) or open an issue.
