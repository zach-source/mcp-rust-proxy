#!/usr/bin/env node

/**
 * Logging Plugin - Request Phase Audit Logging
 *
 * This plugin demonstrates audit logging for sensitive operations.
 * It logs all requests to stderr and passes them through unchanged.
 */

async function main() {
  const input = JSON.parse(await readStdin());

  // Log to stderr (not captured as plugin output)
  const logEntry = {
    timestamp: new Date().toISOString(),
    phase: input.metadata.phase,
    serverName: input.metadata.serverName,
    toolName: input.toolName,
    requestId: input.metadata.requestId,
    contentLength: input.rawContent.length,
  };

  console.error(`[AUDIT] ${JSON.stringify(logEntry)}`);

  // Pass through unchanged (continue=true)
  console.log(
    JSON.stringify({
      text: input.rawContent,
      continue: true,
      metadata: {
        logged: true,
        loggedAt: new Date().toISOString(),
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
