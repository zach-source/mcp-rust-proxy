#!/usr/bin/env node

/**
 * Curation Plugin - AI-Powered Content Reduction
 *
 * This plugin uses Claude to intelligently reduce documentation size by 60-80%
 * while preserving code examples and key facts relevant to the user's query.
 *
 * Usage:
 *   - Requires ANTHROPIC_API_KEY environment variable
 *   - Only processes responses when maxTokens is specified
 *   - Falls back to original content on errors
 */

import Anthropic from '@anthropic-ai/sdk';
import readline from 'readline';

const client = new Anthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
});

const rl = readline.createInterface({
  input: process.stdin,
  terminal: false,
});

rl.on('line', async (line) => {
  try {
    const input = JSON.parse(line);

    // If no maxTokens specified, pass through unchanged
    if (!input.maxTokens) {
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

    // Use Claude to curate the content
    const message = await client.messages.create({
      model: 'claude-3-5-sonnet-20241022',
      max_tokens: 4000,
      messages: [
        {
          role: 'user',
          content: `You are a documentation curator. Your task is to reduce this documentation to fit within ${input.maxTokens} tokens while preserving the most relevant information.

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

Return ONLY the curated documentation.`,
        },
      ],
    });

    const curatedText = message.content[0].text;
    const reduction = (
      (1 - curatedText.length / input.rawContent.length) *
      100
    ).toFixed(1);

    const output = {
      text: curatedText,
      continue: true,
      metadata: {
        originalLength: input.rawContent.length,
        curatedLength: curatedText.length,
        reduction: `${reduction}%`,
        model: 'claude-3-5-sonnet-20241022',
      },
    };

    process.stdout.write(JSON.stringify(output) + '\n');
  } catch (err) {
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
  process.exit(0);
});
