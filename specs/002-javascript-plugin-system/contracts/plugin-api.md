# Plugin API Contract

**Feature Branch**: `002-javascript-plugin-system`
**Version**: 1.0.0
**Date**: 2025-10-10

This document defines the API contract for JavaScript plugins, including input/output schemas, error handling, and usage examples.

---

## Overview

Plugins are JavaScript modules that run in separate Node.js processes and communicate with the MCP Proxy via stdin/stdout using JSON (MVP) or MessagePack (future) format.

**Communication Protocol**:
- **Input**: Single-line JSON sent to plugin's stdin
- **Output**: Single-line JSON read from plugin's stdout
- **Errors**: Optional error field in output OR non-zero exit code
- **Lifecycle**: One request-response per process invocation (stateless)

---

## Input Schema

### PluginInput (JSON)

```json
{
  "toolName": string,              // Required: MCP tool being invoked (e.g., "context7/get-docs")
  "rawContent": string,            // Required: Original request/response content
  "maxTokens": number | null,      // Optional: Token limit for curation (null if not applicable)
  "metadata": {                    // Required: Execution context
    "requestId": string,           // Required: Unique request identifier
    "timestamp": string,           // Required: ISO 8601 timestamp
    "serverName": string,          // Required: Name of MCP server
    "phase": "request" | "response", // Required: Execution phase
    "userQuery": string | null     // Optional: Original user query (if available)
  }
}
```

###  Validation Rules

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `toolName` | string | Yes | Non-empty string |
| `rawContent` | string | Yes | Any string (can be empty) |
| `maxTokens` | number \| null | No | If present, must be > 0 |
| `metadata.requestId` | string | Yes | Non-empty, unique per request |
| `metadata.timestamp` | string | Yes | Valid ISO 8601 format |
| `metadata.serverName` | string | Yes | Non-empty |
| `metadata.phase` | string | Yes | Exactly "request" or "response" |
| `metadata.userQuery` | string \| null | No | Any string or null |

---

## Output Schema

### PluginOutput (JSON)

```json
{
  "text": string,                  // Required: Modified content (or original if unchanged)
  "continue": boolean,             // Required: true = continue chain, false = stop
  "metadata": object | null,       // Optional: Plugin-specific metadata (any valid JSON)
  "error": string | null           // Optional: Error message if plugin failed
}
```

### Validation Rules

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `text` | string | Yes | Any string |
| `continue` | boolean | Yes | true or false |
| `metadata` | object \| null | No | Any valid JSON object |
| `error` | string \| null | No | If present, `continue` must be false |

### Error Handling Semantics

1. **Success with modification**:
   ```json
   {
     "text": "<modified content>",
     "continue": true,
     "metadata": {"tokensUsed": 1200}
   }
   ```

2. **Success without modification**:
   ```json
   {
     "text": "<original content>",
     "continue": true,
     "metadata": null
   }
   ```

3. **Stop chain (intentional)**:
   ```json
   {
     "text": "<final content>",
     "continue": false,
     "metadata": {"reason": "content already optimal"}
   }
   ```

4. **Plugin failure (graceful)**:
   ```json
   {
     "text": "<original content>",
     "continue": false,
     "error": "AI service unavailable: timeout after 30s"
   }
   ```

5. **Plugin crash**: Non-zero exit code → Proxy falls back to original content

---

## Example Plugins

### 1. Echo Plugin (Minimal Example)

```javascript
#!/usr/bin/env node

// Read input from stdin
const input = JSON.parse(await readStdin());

// Echo back unchanged
const output = {
  text: input.rawContent,
  continue: true,
  metadata: { echoedAt: new Date().toISOString() }
};

console.log(JSON.stringify(output));

// Helper function
async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString('utf8');
}
```

**Usage**:
```bash
echo '{"toolName":"test","rawContent":"hello","metadata":{"requestId":"1","timestamp":"2025-10-10T12:00:00Z","serverName":"test","phase":"response"}}' | node echo-plugin.js
```

**Output**:
```json
{"text":"hello","continue":true,"metadata":{"echoedAt":"2025-10-10T12:00:00.123Z"}}
```

---

### 2. Curation Plugin (AI-Powered Example)

```javascript
#!/usr/bin/env node

import Anthropic from '@anthropic-ai/sdk';

const client = new Anthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
});

async function main() {
  // Read input
  const input = JSON.parse(await readStdin());

  // Validate max tokens
  if (!input.maxTokens || input.maxTokens <= 0) {
    throw new Error('maxTokens is required for curation plugin');
  }

  try {
    // Call Claude to curate content
    const message = await client.messages.create({
      model: 'claude-3-5-sonnet-20241022',
      max_tokens: 4000,
      messages: [{
        role: 'user',
        content: `You are a documentation curator. Given the following documentation and a token limit, extract only the most relevant information that answers the user's query.

User Query: ${input.metadata.userQuery || 'General overview'}
Token Limit: ${input.maxTokens}

Documentation:
${input.rawContent}

Return ONLY the curated documentation, removing boilerplate, navigation elements, and redundant information. Preserve code examples and key facts.`
      }]
    });

    const curatedText = message.content[0].text;

    // Return curated output
    const output = {
      text: curatedText,
      continue: true,
      metadata: {
        originalLength: input.rawContent.length,
        curatedLength: curatedText.length,
        reduction: ((1 - curatedText.length / input.rawContent.length) * 100).toFixed(1) + '%'
      }
    };

    console.log(JSON.stringify(output));

  } catch (error) {
    // Graceful failure: return original content
    const errorOutput = {
      text: input.rawContent,
      continue: false,
      error: `Curation failed: ${error.message}`
    };

    console.log(JSON.stringify(errorOutput));
    process.exit(0); // Exit cleanly even on error
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
```

**package.json**:
```json
{
  "type": "module",
  "dependencies": {
    "@anthropic-ai/sdk": "^0.32.0"
  }
}
```

---

### 3. Security Plugin (Request Validation Example)

```javascript
#!/usr/bin/env node

async function main() {
  const input = JSON.parse(await readStdin());

  // Only process request phase
  if (input.metadata.phase !== 'request') {
    console.log(JSON.stringify({
      text: input.rawContent,
      continue: true
    }));
    return;
  }

  // Check for prohibited patterns
  const prohibitedPatterns = [
    /\bpassword\b/i,
    /\bsecret\b/i,
    /\bapi[_-]?key\b/i,
    /\btoken\b/i
  ];

  const hasSensitiveData = prohibitedPatterns.some(pattern =>
    pattern.test(input.rawContent)
  );

  if (hasSensitiveData) {
    // Block the request
    console.log(JSON.stringify({
      text: "[BLOCKED] Request contains potentially sensitive information",
      continue: false,
      error: "Security policy violation: sensitive data detected in request"
    }));
    return;
  }

  // Safe to proceed
  console.log(JSON.stringify({
    text: input.rawContent,
    continue: true,
    metadata: { securityCheck: "passed" }
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
```

---

## Testing Plugin Contract

### Unit Test Example (Node.js Native Test Runner)

```javascript
import { test } from 'node:test';
import assert from 'node:assert';
import { execFileSync } from 'node:child_process';

test('curation plugin reduces content size', () => {
  const input = {
    toolName: 'context7/get-docs',
    rawContent: 'A'.repeat(50000), // 50KB of content
    maxTokens: 1200,
    metadata: {
      requestId: 'test-123',
      timestamp: new Date().toISOString(),
      serverName: 'context7',
      phase: 'response',
      userQuery: 'test query'
    }
  };

  const output = execFileSync('node', ['./curation-plugin.js'], {
    input: JSON.stringify(input),
    encoding: 'utf-8',
    env: { ...process.env, ANTHROPIC_API_KEY: 'test-key' }
  });

  const result = JSON.parse(output);

  assert.strictEqual(typeof result.text, 'string');
  assert.strictEqual(result.continue, true);
  assert(result.text.length < input.rawContent.length);
});

test('security plugin blocks sensitive data', () => {
  const input = {
    toolName: 'test/tool',
    rawContent: 'Please use password: secret123',
    metadata: {
      requestId: 'test-456',
      timestamp: new Date().toISOString(),
      serverName: 'test',
      phase: 'request'
    }
  };

  const output = execFileSync('node', ['./security-plugin.js'], {
    input: JSON.stringify(input),
    encoding: 'utf-8'
  });

  const result = JSON.parse(output);

  assert.strictEqual(result.continue, false);
  assert(result.error.includes('sensitive data'));
});

test('echo plugin preserves input', () => {
  const input = {
    toolName: 'test/echo',
    rawContent: 'Hello, World!',
    metadata: {
      requestId: 'test-789',
      timestamp: new Date().toISOString(),
      serverName: 'test',
      phase: 'response'
    }
  };

  const output = execFileSync('node', ['./echo-plugin.js'], {
    input: JSON.stringify(input),
    encoding: 'utf-8'
  });

  const result = JSON.parse(output);

  assert.strictEqual(result.text, 'Hello, World!');
  assert.strictEqual(result.continue, true);
});
```

**Run tests**:
```bash
node --test examples/plugins/tests/
```

---

## Configuration Schema

### YAML Configuration

```yaml
plugins:
  # Global plugin settings
  pluginDir: ./plugins              # Directory containing .js plugin files
  nodeExecutable: node               # Path to Node.js binary

  # Concurrency & Performance
  maxConcurrentExecutions: 10        # Global semaphore limit
  poolSizePerPlugin: 5               # Warm processes per plugin
  defaultTimeoutMs: 30000            # 30s default timeout

  # Server-specific plugin assignments
  servers:
    context7:
      response:                      # Response phase plugins
        - name: curation-plugin
          order: 1
          enabled: true
          timeoutMs: 45000           # Override default timeout

    filesystem:
      request:                       # Request phase plugins
        - name: security-plugin
          order: 1
          enabled: true
      response:
        - name: path-normalizer
          order: 1
          enabled: true
```

### JSON Configuration

```json
{
  "plugins": {
    "pluginDir": "./plugins",
    "nodeExecutable": "node",
    "maxConcurrentExecutions": 10,
    "poolSizePerPlugin": 5,
    "defaultTimeoutMs": 30000,
    "servers": {
      "context7": {
        "response": [
          {
            "name": "curation-plugin",
            "order": 1,
            "enabled": true,
            "timeoutMs": 45000
          }
        ]
      }
    }
  }
}
```

---

## Error Codes & Handling

### Plugin-Level Errors

| Error Type | Plugin Behavior | Proxy Behavior |
|------------|-----------------|----------------|
| **Malformed JSON output** | N/A | Return original content, log error |
| **Missing required fields** | N/A | Return original content, log error |
| **Non-zero exit code** | Exit with code | Return original content, log error |
| **Timeout exceeded** | Killed by proxy | Return original content, log error |
| **Process crash** | N/A | Return original content, log error |
| **Plugin returns error field** | Exit 0, error in JSON | Return original content (from plugin), log error |

### Example Error Responses

**Timeout**:
```
Proxy logs: "Plugin 'curation-plugin' timed out after 30000ms"
Response to client: Original content unchanged
```

**Malformed Output**:
```
Proxy logs: "Plugin 'transform-plugin' returned invalid JSON: unexpected token at line 1"
Response to client: Original content unchanged
```

**Plugin-Reported Error**:
```
Plugin output: {"text":"<original>","continue":false,"error":"API key missing"}
Proxy logs: "Plugin 'curation-plugin' reported error: API key missing"
Response to client: Original content unchanged
```

---

## Best Practices

### 1. Always Return Valid JSON
```javascript
// ✅ Good
console.log(JSON.stringify(output));

// ❌ Bad
console.log(output);  // Logs [object Object]
```

### 2. Handle Errors Gracefully
```javascript
try {
  // Plugin logic
} catch (error) {
  console.log(JSON.stringify({
    text: input.rawContent,  // Return original on error
    continue: false,
    error: error.message
  }));
  process.exit(0);  // Clean exit even on error
}
```

### 3. Validate Input
```javascript
if (!input.maxTokens) {
  throw new Error('maxTokens required');
}
```

### 4. Use Environment Variables for Secrets
```javascript
const apiKey = process.env.ANTHROPIC_API_KEY;
if (!apiKey) {
  throw new Error('ANTHROPIC_API_KEY not set');
}
```

### 5. Log to stderr for Debugging
```javascript
console.error('[DEBUG] Processing request:', input.metadata.requestId);
console.log(JSON.stringify(output)); // stdout for result
```

---

## Versioning & Compatibility

**Current Version**: 1.0.0

### Backward Compatibility Guarantees

- JSON format will always be supported (even if MessagePack added)
- Required fields will never be removed
- New optional fields may be added (plugins should ignore unknown fields)
- Error handling semantics will remain consistent

### Future Additions (V2)

- MessagePack binary format support
- Streaming input/output for large payloads
- Bidirectional communication for long-running plugins
- Plugin metadata endpoint for capability discovery

---

**Contract Version**: 1.0.0
**Last Updated**: 2025-10-10
**Maintained By**: MCP Rust Proxy Team
