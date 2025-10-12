#!/usr/bin/env node

/**
 * Prompt Injection Scanner - Response Security Plugin
 *
 * Detects and blocks potential prompt injection attacks in LLM responses.
 * This plugin scans response-phase output for common prompt injection patterns
 * that could manipulate downstream LLM behavior.
 *
 * Detects:
 * - System prompt overrides ("Ignore previous instructions", "New instructions:")
 * - Role manipulation ("You are now...", "Act as...")
 * - Command injection attempts ("Execute:", "Run:", "sudo", "rm -rf")
 * - Jailbreak patterns ("DAN mode", "Developer mode", "Unrestricted")
 * - Data exfiltration attempts ("Send to:", "POST to:", "curl")
 * - Context poisoning ("Forget everything", "Reset context")
 *
 * Usage:
 *   Configure as response-phase plugin to scan outputs before returning to client
 */

import readline from 'readline';
import { createLogger } from './logger.js';

const logger = createLogger('prompt-injection-scanner');

const rl = readline.createInterface({
  input: process.stdin,
  terminal: false,
});

logger.info('Prompt injection scanner initialized');

// Prompt injection patterns (defensive security only)
const INJECTION_PATTERNS = [
  // System prompt overrides
  /ignore\s+(all\s+)?(previous|prior|earlier)\s+(instructions|prompts|commands)/i,
  /disregard\s+(previous|prior)\s+(instructions|prompts)/i,
  /new\s+(instructions|system\s+prompt|role):/i,
  /override\s+(instructions|system|prompt)/i,

  // Role manipulation
  /you\s+are\s+now\s+(a|an|the)\s+/i,
  /act\s+as\s+(if\s+you\s+are\s+)?(a|an)\s+/i,
  /pretend\s+(to\s+be|you\s+are)\s+/i,
  /roleplay\s+as\s+/i,

  // Command injection (defensive detection)
  /^\s*(sudo|rm\s+-rf|chmod|chown|curl\s+-X\s+POST)/m,
  /;\s*(rm|sudo|curl|wget|nc|netcat)/,
  /\$\(.*\)/, // Command substitution
  /`.*`/, // Backtick execution

  // Jailbreak attempts
  /DAN\s+mode/i,
  /developer\s+mode\s+(enabled|activated)/i,
  /unrestricted\s+mode/i,
  /jailbreak/i,
  /bypass\s+(safety|restrictions|guidelines)/i,

  // Data exfiltration
  /send\s+(this\s+)?(to|via)\s+(https?:\/\/|email|webhook)/i,
  /POST\s+.*\s+to\s+https?:\/\//i,
  /exfiltrate|steal\s+data/i,

  // Context poisoning
  /forget\s+(everything|all|previous)/i,
  /reset\s+(context|memory|conversation)/i,
  /clear\s+(history|context|memory)/i,
];

rl.on('line', async (line) => {
  const startTime = Date.now();

  try {
    const input = JSON.parse(line);

    logger.debug('Processing response', {
      requestId: input.metadata.requestId,
      serverName: input.metadata.serverName,
      toolName: input.toolName,
      phase: input.metadata.phase,
      contentLength: input.rawContent.length,
    });

    // Only scan response phase
    if (input.metadata.phase !== 'response') {
      logger.debug('Skipping non-response phase', {
        phase: input.metadata.phase,
      });
      const output = {
        text: input.rawContent,
        continue: true,
        metadata: {
          scanner: 'skipped',
          reason: 'not response phase',
        },
      };
      process.stdout.write(JSON.stringify(output) + '\n');
      return;
    }

    // Scan for injection patterns
    const detections = [];
    const matches = []; // Store actual matched text for logging

    for (const pattern of INJECTION_PATTERNS) {
      const match = input.rawContent.match(pattern);
      if (match) {
        detections.push({
          pattern: pattern.source,
          matched: true,
        });

        // Store the matched text (truncate if too long)
        const matchedText = match[0].substring(0, 100);
        matches.push({
          pattern: pattern.source,
          text: matchedText,
          index: match.index,
        });
      }
    }

    // If suspicious patterns detected, sanitize or block
    if (detections.length > 0) {
      const duration = Date.now() - startTime;

      // Log detailed information about what was detected
      logger.warn('Prompt injection detected', {
        requestId: input.metadata.requestId,
        serverName: input.metadata.serverName,
        toolName: input.toolName,
        detectionsCount: detections.length,
        durationMs: duration,
        patterns: matches.map((m) => m.pattern),
      });

      // Log each match with details
      matches.forEach((match, index) => {
        logger.warn(`Detection ${index + 1}: Pattern matched`, {
          pattern: match.pattern,
          matchedText: match.text,
          position: match.index,
        });
      });

      // Rotate log file periodically
      logger.rotate();

      // Option 1: Block the response (strict mode)
      // const output = {
      //   text: "[BLOCKED: Potential prompt injection detected]",
      //   continue: false,
      //   error: "Security: Response contains potential prompt injection patterns",
      //   metadata: {
      //     scanner: 'prompt-injection',
      //     detections: detections.length,
      //     blocked: true,
      //   },
      // };

      // Option 2: Sanitize and warn (permissive mode)
      let sanitizedText = input.rawContent;

      // Sanitize by adding context markers
      sanitizedText = sanitizedText.replace(
        /ignore\s+(all\s+)?(previous|prior|earlier)\s+(instructions|prompts|commands)/gi,
        '[SANITIZED: instruction override attempt]',
      );

      const output = {
        text: sanitizedText,
        continue: true,
        metadata: {
          scanner: 'prompt-injection',
          detections: detections.length,
          sanitized: true,
          warning: 'Response contained suspicious patterns and was sanitized',
        },
      };

      process.stdout.write(JSON.stringify(output) + '\n');
      return;
    }

    // No injection detected - pass through
    const duration = Date.now() - startTime;

    logger.info('Response scanned clean', {
      requestId: input.metadata.requestId,
      serverName: input.metadata.serverName,
      toolName: input.toolName,
      durationMs: duration,
    });

    const output = {
      text: input.rawContent,
      continue: true,
      metadata: {
        scanner: 'prompt-injection',
        scannedAt: new Date().toISOString(),
        detections: 0,
        status: 'clean',
      },
    };

    process.stdout.write(JSON.stringify(output) + '\n');
  } catch (err) {
    const duration = Date.now() - startTime;

    logger.error('Scanner execution failed', {
      error: err.message,
      stack: err.stack,
      durationMs: duration,
    });

    // On error, return original content
    try {
      const input = JSON.parse(line);
      const errorOutput = {
        text: input.rawContent,
        continue: true, // Don't block on scanner errors
        metadata: {
          scanner: 'prompt-injection',
          error: err.message,
        },
      };
      process.stdout.write(JSON.stringify(errorOutput) + '\n');
    } catch {
      logger.error('Critical failure - cannot parse input');
      process.exit(1);
    }
  }
});

// Keep process alive
rl.on('close', () => {
  logger.info('Prompt injection scanner shutting down');
  process.exit(0);
});
