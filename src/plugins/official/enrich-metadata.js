#!/usr/bin/env node

/**
 * Metadata Enrichment Plugin
 *
 * Adds custom metadata to responses without modifying the content.
 * Useful for adding processing timestamps, version info, or other tracking data.
 *
 * This plugin demonstrates a pass-through transformation that only adds metadata.
 */

async function main() {
  const input = JSON.parse(await readStdin());

  // Add enrichment metadata
  const enrichment = {
    processedAt: new Date().toISOString(),
    processingTimeMs: Date.now(), // Can be computed if plugin tracks start time
    pluginVersion: '1.0.0',
    serverName: input.metadata.serverName,
    toolName: input.toolName,
    contentLength: input.rawContent.length,
    phase: input.metadata.phase,
  };

  // Pass through content unchanged, only add metadata
  console.log(
    JSON.stringify({
      text: input.rawContent,
      continue: true,
      metadata: enrichment,
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
