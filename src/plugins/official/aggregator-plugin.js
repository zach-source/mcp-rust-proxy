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
      logger.info('MCP server configurations', {
        serverConfigs: Object.entries(mcpServerConfigs).map(
          ([name, config]) => ({
            name,
            command: config.command,
            hasArgs: config.args.length > 0,
            hasEnv: Object.keys(config.env).length > 0,
          }),
        ),
      });

      // Build dynamic tool use directive
      const availableServerNames = serverNames.join(', ');
      const toolUseDirective = `tool use: [${availableServerNames}]`;

      logger.info('Tool use directive', {
        directive: toolUseDirective,
        serverCount: serverNames.length,
      });

      // Build system prompt to encourage MCP tool usage with explicit tool directive
      const systemPrompt = `You are a context aggregation assistant with access to MCP servers.

${toolUseDirective}

CRITICAL: You MUST use the MCP tools listed above to answer the user's query. DO NOT rely on your training data.

When answering user queries:
1. ALWAYS use the available MCP tools (${availableServerNames}) - this is REQUIRED
2. For documentation and library information: use context7 tools
3. For codebase-specific questions: use serena tools
4. For file operations: use filesystem tools
5. For current time/date: use time tools
6. Combine information from multiple MCP servers when relevant
7. Cite which MCP server provided each piece of information

Your goal is to provide accurate, up-to-date information by LEVERAGING THE MCP SERVERS, not your training data.`;

      let mcpServersInitialized = false;
      let availableTools = [];

      for await (const message of query({
        prompt: userQuery,
        options: {
          mcpServers: mcpServerConfigs,
          allowedTools,
          systemPrompt,
        },
      })) {
        logger.info('Received message from Claude', {
          type: message.type,
          subtype: message.subtype || undefined,
        });

        // Log MCP server initialization
        if (message.type === 'system' && !mcpServersInitialized) {
          mcpServersInitialized = true;
          logger.info('MCP servers initialization phase detected');
        }

        // Debug: Log full assistant messages to understand structure
        if (message.type === 'assistant') {
          // Claude Agent SDK uses 'message' field which contains the full Claude API response
          const messageRaw = message.message;
          const messageText =
            typeof messageRaw === 'string'
              ? messageRaw
              : JSON.stringify(messageRaw);

          // Check for tool uses in the content array
          let toolUses = [];
          if (
            typeof messageRaw === 'object' &&
            messageRaw.content &&
            Array.isArray(messageRaw.content)
          ) {
            toolUses = messageRaw.content.filter((c) => c.type === 'tool_use');
          }

          logger.info('Assistant message', {
            hasMessage: !!message.message,
            messageType: typeof message.message,
            messageIsArray: Array.isArray(message.message),
            messageLength: messageText ? messageText.length : 0,
            hasTools: toolUses.length > 0,
            toolCount: toolUses.length,
            messageKeys: Object.keys(message),
          });

          // Always log the message text to see what Claude is actually saying
          if (messageText) {
            logger.info('Assistant text', {
              length: messageText.length,
              preview:
                messageText.length > 0
                  ? messageText.substring(0, 300)
                  : '(empty)',
              fullText:
                messageText.length < 500
                  ? messageText
                  : `${messageText.substring(0, 500)}... (truncated)`,
            });
          }

          // Log and COUNT any tool uses detected
          if (toolUses.length > 0) {
            toolCallCount += toolUses.length; // INCREMENT the counter!
            logger.info('Assistant requested tools', {
              toolCount: toolUses.length,
              tools: toolUses.map((t) => t.name),
              totalToolCalls: toolCallCount,
            });
          }
        }

        // Log tool availability from user messages (tool results)
        if (message.type === 'user') {
          // User messages have tool results in message.message.content (similar to assistant)
          const messageRaw = message.message;

          // Extract tool results from the message object
          let toolResults = [];
          if (
            typeof messageRaw === 'object' &&
            messageRaw.content &&
            Array.isArray(messageRaw.content)
          ) {
            toolResults = messageRaw.content.filter(
              (c) => c.type === 'tool_result',
            );
          }

          // Debug log user message structure
          logger.info('User message', {
            hasMessage: !!message.message,
            messageType: typeof message.message,
            hasContentArray:
              typeof messageRaw === 'object' &&
              Array.isArray(messageRaw.content),
            toolResultCount: toolResults.length,
            messageKeys: Object.keys(message),
          });

          if (toolResults.length > 0) {
            // Track raw MCP response sizes
            toolResults.forEach((result) => {
              const resultSize = JSON.stringify(result.content || '').length;
              totalRawResponseBytes += resultSize;
              logger.info('Tool result received', {
                toolId: result.tool_use_id,
                size: resultSize,
                totalRawBytes: totalRawResponseBytes,
              });
            });

            logger.info('All tool results summary', {
              resultCount: toolResults.length,
              totalBytes: totalRawResponseBytes,
            });
          }
        }

        // Legacy tracking - these message types don't appear in Claude Agent SDK
        // Keeping for compatibility but tools are actually in assistant.message.content
        if (message.type === 'tool_use') {
          toolCallCount++;
          logger.info('Direct tool_use message (legacy)', {
            tool: message.name,
            callNumber: toolCallCount,
          });
        }

        if (message.type === 'tool_result') {
          const resultSize = JSON.stringify(message.content || '').length;
          totalRawResponseBytes += resultSize;
          logger.info('Direct tool_result message (legacy)', {
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
