#!/usr/bin/env node

/**
 * Metadata Enrichment Plugin
 *
 * Adds custom metadata to responses without modifying the content.
 */

const readline = require('readline');

const rl = readline.createInterface({
  input: process.stdin,
  terminal: false,
});

rl.on('line', (line) => {
  try {
    const input = JSON.parse(line);

    // Add enrichment metadata
    const enrichment = {
      processedAt: new Date().toISOString(),
      processingTimeMs: Date.now(),
      pluginVersion: '1.0.0',
      serverName: input.metadata.serverName,
      toolName: input.toolName,
      contentLength: input.rawContent.length,
      phase: input.metadata.phase,
    };

    // Pass through content unchanged, only add metadata
    const output = {
      text: input.rawContent,
      continue: true,
      metadata: enrichment,
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

rl.on('close', () => {
  process.exit(0);
});
