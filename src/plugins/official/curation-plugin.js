#!/usr/bin/env node

/**
 * Curation Plugin - AI-Powered Content Reduction
 *
 * This plugin uses Claude Agent SDK to intelligently reduce documentation size by 60-80%
 * while preserving code examples and key facts relevant to the user's query.
 *
 * Usage:
 *   - Uses Claude Agent SDK (no API key required when running in CLI context)
 *   - Only processes responses when maxTokens is specified
 *   - Falls back to original content on errors
 */

import { query } from '@anthropic-ai/claude-agent-sdk';
import readline from 'readline';
import { createLogger } from './logger.js';

const logger = createLogger('curation-plugin');

const rl = readline.createInterface({
  input: process.stdin,
  terminal: false,
});

logger.info('Curation plugin initialized', {
  sdk: 'claude-agent-sdk',
});

rl.on('line', async (line) => {
  const startTime = Date.now();

  try {
    const input = JSON.parse(line);

    logger.debug('Processing response', {
      requestId: input.metadata.requestId,
      serverName: input.metadata.serverName,
      toolName: input.toolName,
      contentLength: input.rawContent.length,
      maxTokens: input.maxTokens || 'not specified',
      toolArguments: input.metadata.toolArguments
        ? Object.keys(input.metadata.toolArguments)
        : [],
    });

    // If no maxTokens specified, pass through unchanged
    if (!input.maxTokens) {
      logger.info('Skipping curation - no maxTokens specified', {
        requestId: input.metadata.requestId,
        availableArguments: input.metadata.toolArguments || {},
      });
      const output = {
        text: input.rawContent,
        continue: true,
        metadata: {
          skipped: true,
          reason: 'No maxTokens specified',
        },
      };
      process.stdout.write(JSON.stringify(output) + '\n');
      return;
    }

    logger.info('Starting AI curation', {
      requestId: input.metadata.requestId,
      targetTokens: input.maxTokens,
      originalLength: input.rawContent.length,
    });

    // Use Claude Agent SDK to curate the content
    const prompt = `You are a documentation curator. Your task is to reduce this documentation to fit within ${input.maxTokens} tokens while preserving the most relevant information.

User Query: ${input.metadata.userQuery || 'General overview'}
Token Limit: ${input.maxTokens}

Documentation:
${input.rawContent}

Instructions:
- Remove boilerplate, navigation elements, and redundant information
- Preserve ALL code examples and their explanations
- Keep key facts and technical details
- Maintain the overall structure and flow
- Do not invent any information not in the original

Return ONLY the curated documentation.`;

    const response = await query({ prompt });

    // Collect streaming response
    let curatedText = '';
    for await (const message of response) {
      if (typeof message === 'string') {
        curatedText += message;
      } else if (message.type === 'text') {
        curatedText += message.text;
      }
    }
    const duration = Date.now() - startTime;
    const reduction = (
      (1 - curatedText.length / input.rawContent.length) *
      100
    ).toFixed(1);

    logger.info('Curation completed successfully', {
      requestId: input.metadata.requestId,
      originalLength: input.rawContent.length,
      curatedLength: curatedText.length,
      reduction: `${reduction}%`,
      durationMs: duration,
    });

    // Rotate log file periodically
    logger.rotate();

    const output = {
      text: curatedText,
      continue: true,
      metadata: {
        originalLength: input.rawContent.length,
        curatedLength: curatedText.length,
        reduction: `${reduction}%`,
        sdk: 'claude-agent-sdk',
      },
    };

    process.stdout.write(JSON.stringify(output) + '\n');
  } catch (err) {
    const duration = Date.now() - startTime;

    logger.error('Curation failed', {
      error: err.message,
      stack: err.stack,
      durationMs: duration,
    });

    // On error, return original content
    const input = JSON.parse(line);
    const errorOutput = {
      text: input.rawContent,
      continue: false,
      error: `Curation failed: ${err.message}`,
    };
    process.stdout.write(JSON.stringify(errorOutput) + '\n');
  }
});

// Keep process alive
rl.on('close', () => {
  logger.info('Curation plugin shutting down');
  process.exit(0);
});
