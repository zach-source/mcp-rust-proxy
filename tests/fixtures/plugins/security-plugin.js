#!/usr/bin/env node

/**
 * Security Plugin - Request Phase Validation
 *
 * This plugin demonstrates request-phase security validation.
 * It blocks requests containing sensitive patterns like passwords, API keys, etc.
 */

const readline = require('readline');

const rl = readline.createInterface({
  input: process.stdin,
  terminal: false,
});

rl.on('line', (line) => {
  try {
    const input = JSON.parse(line);

    // Only process request phase
    if (input.metadata.phase !== 'request') {
      const output = {
        text: input.rawContent,
        continue: true,
      };
      process.stdout.write(JSON.stringify(output) + '\n');
      return;
    }

    // Define sensitive patterns to block
    const sensitivePatterns = [
      /password\s*[:=]\s*[^\s]+/i,
      /api[_-]?key\s*[:=]\s*[^\s]+/i,
      /secret\s*[:=]\s*[^\s]+/i,
      /token\s*[:=]\s*[^\s]+/i,
      /bearer\s+[a-zA-Z0-9\-._~+/]+=*/i,
      /-----BEGIN (RSA |DSA )?PRIVATE KEY-----/i,
    ];

    // Check for sensitive data in the request
    for (const pattern of sensitivePatterns) {
      if (pattern.test(input.rawContent)) {
        const matchedPattern = pattern.toString();
        const output = {
          text: '[BLOCKED]',
          continue: false,
          error: `Security violation: Request contains sensitive data matching pattern ${matchedPattern}`,
          metadata: {
            blockedAt: new Date().toISOString(),
            pattern: matchedPattern,
            serverName: input.metadata.serverName,
            toolName: input.toolName,
          },
        };
        process.stdout.write(JSON.stringify(output) + '\n');
        return;
      }
    }

    // Request is safe, pass through unchanged
    const output = {
      text: input.rawContent,
      continue: true,
      metadata: {
        securityCheck: 'passed',
        checkedAt: new Date().toISOString(),
      },
    };
    process.stdout.write(JSON.stringify(output) + '\n');
  } catch (err) {
    const errorOutput = {
      text: '',
      continue: false,
      error: err.message,
    };
    process.stdout.write(JSON.stringify(errorOutput) + '\n');
  }
});

// Keep process alive until stdin closes
rl.on('close', () => {
  process.exit(0);
});
