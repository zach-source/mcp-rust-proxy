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
    const query = input.rawContent;
    const mcpServers = input.metadata?.mcpServers || [];

    console.error(
      `[Aggregator Plugin] Processing query: "${query.substring(0, 100)}..."`,
    );
    console.error(
      `[Aggregator Plugin] Configured MCP servers: ${mcpServers.map((s) => s.name).join(', ')}`,
    );

    // Initialize Claude Agent SDK with MCP servers
    const agent = await initializeAgent(mcpServers);

    // Run aggregation via Claude Agent
    const result = await agent.run({
      query,
      systemPrompt: buildSystemPrompt(),
    });

    console.error(
      `[Aggregator Plugin] Aggregation complete, result length: ${result.length} chars`,
    );

    return {
      text: result,
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
  // TODO T006: Implement full Agent initialization with MCP clients
  // For now, return placeholder
  return {
    run: async ({ query }) => {
      return `Aggregated context for: ${query}\n(Agent initialization pending T006)`;
    },
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
