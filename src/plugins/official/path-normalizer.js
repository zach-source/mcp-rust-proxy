#!/usr/bin/env node

/**
 * Path Normalization Plugin
 *
 * Detects file paths in response text and converts them to platform-specific format.
 * Useful for cross-platform compatibility when responses contain file paths.
 *
 * Example transformations:
 * - Windows → Unix: C:\Users\foo\file.txt → /Users/foo/file.txt
 * - Unix → Windows: /home/user/file → C:\home\user\file (if needed)
 */

async function main() {
  const input = JSON.parse(await readStdin());

  // Path patterns to detect
  const windowsPathPattern =
    /[A-Z]:\\(?:[^\\/:*?"<>|\r\n]+\\)*[^\\/:*?"<>|\r\n]*/g;
  const unixPathPattern = /\/(?:[^\/\0]+\/)*[^\/\0]*/g;

  let transformedText = input.rawContent;
  let pathsNormalized = 0;

  // Detect platform from first match or use Unix as default
  const hasWindowsPaths = windowsPathPattern.test(input.rawContent);
  const targetFormat = hasWindowsPaths ? 'unix' : 'current';

  if (targetFormat === 'unix') {
    // Convert Windows paths to Unix format
    transformedText = transformedText.replace(windowsPathPattern, (match) => {
      pathsNormalized++;
      // C:\Users\foo\file.txt → /Users/foo/file.txt
      return match.replace(/\\/g, '/').replace(/^[A-Z]:/, '');
    });
  }

  // For demonstration, also normalize path separators
  const finalText = transformedText;

  console.log(
    JSON.stringify({
      text: finalText,
      continue: true,
      metadata: {
        pathsNormalized,
        targetFormat,
        processedAt: new Date().toISOString(),
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
