/**
 * Aggregator Plugin
 *
 * Uses Claude Agent SDK to aggregate and optimize context from multiple MCP servers
 * (context7, serena, memory, filesystem, etc.) for improved LLM responses.
 *
 * Input: { rawContent: "user query", metadata: { mcpServers: [...] } }
 * Output: { text: "aggregated, optimized context", continue: true }
 */

import { Agent } from '@anthropic-ai/claude-agent-sdk';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

/**
 * Main plugin entry point
 * @param {Object} input - Plugin input from Rust
 * @param {string} input.rawContent - User query to aggregate context for
 * @param {Object} input.metadata - Metadata including MCP server configs
 * @param {Array} input.metadata.mcpServers - List of MCP servers to query
 * @returns {Promise<Object>} Plugin output with aggregated context
 */
export async function process(input) {
  try {
    const userQuery = input.rawContent;
    const mcpServers = input.metadata?.mcpServers || [];

    console.error(
      `[Aggregator Plugin] Processing query: "${userQuery.substring(0, 100)}..."`,
    );
    console.error(
      `[Aggregator Plugin] Configured MCP servers: ${mcpServers.map((s) => s.name).join(', ')}`,
    );

    // Initialize MCP server configs
    const { serverConfigs, serverNames } = await initializeAgent(mcpServers);

    // Build allowed tools list (all tools from all MCP servers)
    const allowedTools = serverNames.flatMap((name) => [
      `mcp__${name}__*`, // Allow all tools from this server
    ]);

    // Import query function from SDK
    const { query } = await import('@anthropic-ai/claude-agent-sdk');

    // Run aggregation via Claude Agent SDK
    let aggregatedResult = '';

    for await (const message of query({
      prompt: userQuery,
      options: {
        mcpServers: serverConfigs,
        allowedTools,
        apiKey: process.env.ANTHROPIC_API_KEY,
      },
    })) {
      if (message.type === 'result' && message.subtype === 'success') {
        aggregatedResult = message.result || '';
      } else if (
        message.type === 'result' &&
        message.subtype === 'error_during_execution'
      ) {
        console.error(`[Aggregator Plugin] Execution error: ${message.error}`);
      }
    }

    console.error(
      `[Aggregator Plugin] Aggregation complete, result length: ${aggregatedResult.length} chars`,
    );

    return {
      text: aggregatedResult,
      continue: true,
      error: null,
    };
  } catch (error) {
    console.error(`[Aggregator Plugin] Error: ${error.message}`);
    return {
      text: '',
      continue: false,
      error: error.message,
    };
  }
}

/**
 * Initialize Claude Agent with MCP servers registered as tools
 */
async function initializeAgent(mcpServers) {
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

  return {
    serverConfigs: mcpServerConfigs,
    serverNames: mcpServers.map((s) => s.name),
  };
}

/**
 * Build system prompt for aggregation behavior
 */
function buildSystemPrompt() {
  return `You are a context aggregation assistant. Your role is to:
1. Query relevant MCP servers to gather information
2. Rank results by relevance to the user's query
3. Combine and optimize the context to reduce waste
4. Return concise, high-quality information for the LLM to use

Focus on quality over quantity. Return only the most relevant information.`;
}
