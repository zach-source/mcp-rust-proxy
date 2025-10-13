use crate::plugin::chain::PluginChain;
use crate::plugin::schema::{PluginInput, PluginMetadata, PluginPhase};
use crate::state::AppState;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// Get aggregator tools for LLM context optimization
pub fn get_aggregator_tools() -> Vec<Value> {
    vec![create_tool(
        "mcp__proxy__aggregator__context_aggregator",
        "Aggregate and optimize context from multiple MCP servers (context7, serena, etc.) for improved LLM responses with reduced context waste",
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The user query to aggregate context for",
                    "minLength": 1,
                    "maxLength": 10000
                },
                "maxResults": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 30, range: 10-100)",
                    "minimum": 10,
                    "maximum": 100,
                    "default": 30
                },
                "servers": {
                    "type": "array",
                    "description": "Optional list of specific MCP servers to query (default: all configured)",
                    "items": {
                        "type": "string"
                    }
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }),
    )]
}

fn create_tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

/// Handle aggregator tool calls by invoking the JavaScript aggregator plugin
pub async fn handle_aggregator_tool(
    _tool_name: &str,
    arguments: Value,
    state: Arc<AppState>,
) -> std::result::Result<Value, String> {
    // Extract query from arguments
    let query = arguments
        .get("query")
        .and_then(|q| q.as_str())
        .ok_or("Missing query parameter")?;

    // Get optional parameters
    let max_results = arguments
        .get("maxResults")
        .and_then(|m| m.as_u64())
        .unwrap_or(30) as usize;

    let requested_servers = arguments
        .get("servers")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        });

    tracing::info!(
        query_length = query.len(),
        max_results = max_results,
        requested_servers = ?requested_servers,
        "Processing aggregator query"
    );

    // Check if plugin manager is available
    let plugin_manager = state
        .plugin_manager
        .as_ref()
        .ok_or("Plugin manager not initialized")?;

    // Get plugin config
    let config_guard = state.config.read().await;
    let plugin_config = config_guard
        .plugins
        .as_ref()
        .ok_or("Plugins not configured")?
        .clone();
    drop(config_guard);

    // Create plugin chain for aggregator
    let chain = PluginChain::new(
        "aggregator".to_string(),
        PluginPhase::Response, // Reuse Response phase
        plugin_manager.clone(),
        Arc::new(plugin_config),
    );

    // Get MCP server configs to pass to plugin
    let mcp_server_configs = get_mcp_server_configs(&state, requested_servers.clone()).await;

    // Create plugin input with MCP server configs
    let input = PluginInput {
        tool_name: "context_aggregator".to_string(),
        raw_content: query.to_string(),
        max_tokens: Some(max_results as u32 * 100), // Rough estimate: 100 tokens per result
        metadata: PluginMetadata {
            request_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            server_name: "aggregator".to_string(),
            phase: PluginPhase::Response,
            user_query: Some(query.to_string()),
            tool_arguments: Some(arguments.clone()),
            mcp_servers: Some(mcp_server_configs),
        },
    };

    // Execute aggregator plugin
    let output = chain.execute_safe(input).await;

    if !output.continue_ {
        return Err(output
            .error
            .unwrap_or_else(|| "Aggregation failed".to_string()));
    }

    // Return aggregated context
    Ok(json!({
        "content": [{
            "type": "text",
            "text": output.text
        }]
    }))
}

/// Get MCP server configurations from AppState
async fn get_mcp_server_configs(
    state: &Arc<AppState>,
    requested_servers: Option<Vec<String>>,
) -> Vec<Value> {
    let config = state.config.read().await;
    let mut server_configs = Vec::new();

    for (name, server_config) in &config.servers {
        // Filter by requested servers if specified
        if let Some(ref requested) = requested_servers {
            if !requested.contains(name) {
                continue;
            }
        }

        // Only include enabled servers
        if !server_config.enabled {
            continue;
        }

        server_configs.push(json!({
            "name": name,
            "command": server_config.command,
            "args": server_config.args,
            "env": server_config.env
        }));
    }

    server_configs
}
