#!/usr/bin/env node

/**
 * Security Plugin - Request Phase Validation
 *
 * This plugin demonstrates request-phase security validation.
 * It blocks requests containing sensitive patterns like passwords, API keys, etc.
 */

async function main() {
  const input = JSON.parse(await readStdin());

  // Only process request phase
  if (input.metadata.phase !== 'request') {
    console.log(
      JSON.stringify({
        text: input.rawContent,
        continue: true,
      }),
    );
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
      console.log(
        JSON.stringify({
          text: '[BLOCKED]',
          continue: false,
          error: `Security violation: Request contains sensitive data matching pattern ${matchedPattern}`,
          metadata: {
            blockedAt: new Date().toISOString(),
            pattern: matchedPattern,
            serverName: input.metadata.serverName,
            toolName: input.toolName,
          },
        }),
      );
      return;
    }
  }

  // Request is safe, pass through unchanged
  console.log(
    JSON.stringify({
      text: input.rawContent,
      continue: true,
      metadata: {
        securityCheck: 'passed',
        checkedAt: new Date().toISOString(),
      },
    }),
  );
}

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString('utf8');
}

main().catch((err) => {
  console.error(
    JSON.stringify({
      text: '',
      continue: false,
      error: err.message,
    }),
  );
  process.exit(1);
});
