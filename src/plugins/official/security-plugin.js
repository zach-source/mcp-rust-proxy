#!/usr/bin/env node

/**
 * Security Plugin - Request Phase Validation
 *
 * Blocks requests containing sensitive patterns like passwords, API keys, etc.
 * Includes comprehensive logging to both console and file.
 */

import readline from 'readline';
import { createLogger } from './logger.js';

const logger = createLogger('security-plugin');

const rl = readline.createInterface({
  input: process.stdin,
  terminal: false,
});

logger.info('Security plugin initialized', {
  logLevel: process.env.PLUGIN_LOG_LEVEL || 'INFO',
  logDir: logger.logDir,
});

rl.on('line', (line) => {
  const startTime = Date.now();

  try {
    const input = JSON.parse(line);

    logger.debug('Processing request', {
      requestId: input.metadata.requestId,
      serverName: input.metadata.serverName,
      toolName: input.toolName,
      phase: input.metadata.phase,
      contentLength: input.rawContent.length,
    });

    // Only process request phase
    if (input.metadata.phase !== 'request') {
      logger.debug('Skipping non-request phase', {
        phase: input.metadata.phase,
      });
      const output = {
        text: input.rawContent,
        continue: true,
      };
      process.stdout.write(JSON.stringify(output) + '\n');
      return;
    }

    // Define sensitive patterns to block
    const sensitivePatterns = [
      { pattern: /password\s*[:=]\s*[^\s]+/i, name: 'password' },
      { pattern: /api[_-]?key\s*[:=]\s*[^\s]+/i, name: 'api_key' },
      { pattern: /secret\s*[:=]\s*[^\s]+/i, name: 'secret' },
      { pattern: /token\s*[:=]\s*[^\s]+/i, name: 'token' },
      { pattern: /bearer\s+[a-zA-Z0-9\-._~+/]+=*/i, name: 'bearer_token' },
      {
        pattern: /-----BEGIN (RSA |DSA )?PRIVATE KEY-----/i,
        name: 'private_key',
      },
    ];

    // Check for sensitive data in the request
    for (const { pattern, name } of sensitivePatterns) {
      if (pattern.test(input.rawContent)) {
        const duration = Date.now() - startTime;

        logger.warn('Security violation detected', {
          requestId: input.metadata.requestId,
          serverName: input.metadata.serverName,
          toolName: input.toolName,
          patternType: name,
          durationMs: duration,
        });

        // Rotate log file periodically
        logger.rotate();

        const output = {
          text: '[BLOCKED]',
          continue: false,
          error: `Security violation: Request contains sensitive data matching pattern ${pattern.toString()}`,
          metadata: {
            blockedAt: new Date().toISOString(),
            pattern: pattern.toString(),
            patternType: name,
            serverName: input.metadata.serverName,
            toolName: input.toolName,
          },
        };
        process.stdout.write(JSON.stringify(output) + '\n');
        return;
      }
    }

    // Request is safe, pass through unchanged
    const duration = Date.now() - startTime;

    logger.info('Security check passed', {
      requestId: input.metadata.requestId,
      serverName: input.metadata.serverName,
      toolName: input.toolName,
      durationMs: duration,
    });

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
    const duration = Date.now() - startTime;

    logger.error('Plugin execution failed', {
      error: err.message,
      stack: err.stack,
      durationMs: duration,
    });

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
  logger.info('Security plugin shutting down');
  process.exit(0);
});
