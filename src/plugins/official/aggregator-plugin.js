#!/usr/bin/env node

/**
 * Aggregator Plugin
 *
 * Uses Claude Agent SDK to aggregate and optimize context from multiple MCP servers
 * (context7, serena, memory, filesystem, etc.) for improved LLM responses.
 */

import { query } from '@anthropic-ai/claude-agent-sdk';
import readline from 'readline';
import { createLogger } from './logger.js';

const logger = createLogger('aggregator-plugin');

// Create readline interface for stdin/stdout communication
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false,
});

logger.info('Aggregator plugin started');

// Process each input line from Rust
rl.on('line', (line) => {
  // Use setImmediate to ensure async processing doesn't block readline
  setImmediate(async () => {
    const startTime = Date.now();

    try {
      const input = JSON.parse(line);
      const userQuery = input.rawContent;
      const mcpServers = input.metadata?.mcpServers || [];

      logger.info('Processing aggregation query', {
        queryLength: userQuery.length,
        serverCount: mcpServers.length,
        servers: mcpServers.map((s) => s.name),
      });

      // Build MCP server configs for Claude Agent SDK
      const mcpServerConfigs = {};
      for (const server of mcpServers) {
        mcpServerConfigs[server.name] = {
          type: 'stdio',
          command: server.command,
          args: server.args || [],
          env: server.env || {},
        };
      }

      // Build allowed tools (all tools from configured servers)
      const serverNames = mcpServers.map((s) => s.name);
      const allowedTools = serverNames.flatMap((name) => [`mcp__${name}__*`]);

      logger.info('Configured MCP servers', {
        servers: Object.keys(mcpServerConfigs),
        allowedTools: allowedTools.length,
      });

      // Check for API key
      if (!process.env.ANTHROPIC_API_KEY) {
        throw new Error(
          'ANTHROPIC_API_KEY environment variable not set. Required for aggregator plugin.',
        );
      }

      // Run aggregation via Claude Agent SDK
      let aggregatedResult = '';
      let success = false;
      let toolCallCount = 0;
      let totalRawResponseBytes = 0;

      logger.info('Starting Claude Agent SDK query...');

      for await (const message of query({
        prompt: userQuery,
        options: {
          mcpServers: mcpServerConfigs,
          allowedTools,
        },
      })) {
        logger.info('Received message from Claude', { type: message.type });

        // Track MCP tool calls
        if (message.type === 'tool_use') {
          toolCallCount++;
          logger.info('MCP tool called', {
            tool: message.name,
            callNumber: toolCallCount,
          });
        }

        // Track raw MCP response sizes
        if (message.type === 'tool_result') {
          const resultSize = JSON.stringify(message.content || '').length;
          totalRawResponseBytes += resultSize;
          logger.info('MCP tool result', {
            size: resultSize,
            totalRawBytes: totalRawResponseBytes,
          });
        }

        if (message.type === 'result' && message.subtype === 'success') {
          aggregatedResult = message.result || '';
          success = true;
        } else if (
          message.type === 'result' &&
          message.subtype === 'error_during_execution'
        ) {
          logger.error('Claude Agent execution error', {
            error: message.error,
          });
        }
      }

      const duration = Date.now() - startTime;
      const aggregatedBytes = aggregatedResult.length;
      const reductionPercent =
        totalRawResponseBytes > 0
          ? Math.round(
              ((totalRawResponseBytes - aggregatedBytes) /
                totalRawResponseBytes) *
                100,
            )
          : 0;

      logger.info('Aggregation complete', {
        durationMs: duration,
        mcpToolCalls: toolCallCount,
        rawMcpBytes: totalRawResponseBytes,
        aggregatedBytes,
        reduction: `${reductionPercent}%`,
        success,
      });

      const output = {
        text: aggregatedResult,
        continue: true,
        metadata: {
          serversQueried: mcpServers.length,
          mcpToolCalls: toolCallCount,
          rawMcpBytes: totalRawResponseBytes,
          aggregatedBytes,
          reductionPercent,
          durationMs: duration,
        },
      };

      process.stdout.write(JSON.stringify(output) + '\n');
    } catch (err) {
      const duration = Date.now() - startTime;

      logger.error('Aggregation failed', {
        error: err.message,
        stack: err.stack,
        durationMs: duration,
      });

      // On error, return error response
      const errorOutput = {
        text: '',
        continue: false,
        error: `Aggregation failed: ${err.message}`,
      };
      process.stdout.write(JSON.stringify(errorOutput) + '\n');
    }
  });
});

// Handle plugin shutdown
rl.on('close', () => {
  logger.info('Aggregator plugin shutting down');
  process.exit(0);
});
