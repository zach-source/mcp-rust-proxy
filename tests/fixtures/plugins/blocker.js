#!/usr/bin/env node
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, terminal: false });
rl.on('line', (line) => {
  try {
    const input = JSON.parse(line);
    const output = { text: "BLOCKED", continue: false, metadata: { blocked: true } };
    process.stdout.write(JSON.stringify(output) + '\n');
  } catch (err) {
    const errorOutput = { text: "", continue: false, error: err.message };
    process.stdout.write(JSON.stringify(errorOutput) + '\n');
  }
});
rl.on('close', () => process.exit(0));
