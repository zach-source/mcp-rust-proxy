#!/usr/bin/env node

/**
 * Echo Plugin - Test fixture
 *
 * Simple plugin that echoes back the input content unchanged.
 * Used for testing the basic plugin I/O flow.
 */

const readline = require('readline');

const rl = readline.createInterface({
  input: process.stdin,
  terminal: false,
});

rl.on('line', (line) => {
  try {
    const input = JSON.parse(line);

    const output = {
      text: input.rawContent,
      continue: true,
      metadata: {
        echoed: true,
        timestamp: new Date().toISOString(),
        originalLength: input.rawContent.length,
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
