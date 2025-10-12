#!/usr/bin/env node

/**
 * Path Normalization Plugin
 *
 * Detects file paths in response text and converts them to platform-specific format.
 */

const readline = require('readline');

const rl = readline.createInterface({
  input: process.stdin,
  terminal: false,
});

rl.on('line', (line) => {
  try {
    const input = JSON.parse(line);

    const windowsPathPattern =
      /[A-Z]:\\(?:[^\\/:*?"<>|\r\n]+\\)*[^\\/:*?"<>|\r\n]*/g;

    let transformedText = input.rawContent;
    let pathsNormalized = 0;

    const hasWindowsPaths = windowsPathPattern.test(input.rawContent);
    const targetFormat = hasWindowsPaths ? 'unix' : 'current';

    if (targetFormat === 'unix') {
      transformedText = transformedText.replace(windowsPathPattern, (match) => {
        pathsNormalized++;
        return match.replace(/\\/g, '/').replace(/^[A-Z]:/, '');
      });
    }

    const output = {
      text: transformedText,
      continue: true,
      metadata: {
        pathsNormalized,
        targetFormat,
        processedAt: new Date().toISOString(),
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

rl.on('close', () => {
  process.exit(0);
});
