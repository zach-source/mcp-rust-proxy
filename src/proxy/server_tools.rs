use crate::state::AppState;
use serde_json::{json, Value};
use std::sync::Arc;

/// Get proxy server management tools
pub fn get_server_tools() -> Vec<Value> {
    vec![
        create_tool(
            "mcp__proxy__server__list",
            "List all backend MCP servers and their status",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        ),
        create_tool(
            "mcp__proxy__server__enable",
            "Enable a stopped backend MCP server",
            json!({
                "type": "object",
                "properties": {
                    "server_name": {
                        "type": "string",
                        "description": "Name of the server to enable"
                    }
                },
                "required": ["server_name"]
            }),
        ),
        create_tool(
            "mcp__proxy__server__disable",
            "Disable a running backend MCP server",
            json!({
                "type": "object",
                "properties": {
                    "server_name": {
                        "type": "string",
                        "description": "Name of the server to disable"
                    }
                },
                "required": ["server_name"]
            }),
        ),
        create_tool(
            "mcp__proxy__server__restart",
            "Restart a backend MCP server",
            json!({
                "type": "object",
                "properties": {
                    "server_name": {
                        "type": "string",
                        "description": "Name of the server to restart"
                    }
                },
                "required": ["server_name"]
            }),
        ),
        create_tool(
            "mcp__proxy__server__start",
            "Start a stopped backend MCP server",
            json!({
                "type": "object",
                "properties": {
                    "server_name": {
                        "type": "string",
                        "description": "Name of the server to start"
                    }
                },
                "required": ["server_name"]
            }),
        ),
        create_tool(
            "mcp__proxy__server__stop",
            "Stop a running backend MCP server",
            json!({
                "type": "object",
                "properties": {
                    "server_name": {
                        "type": "string",
                        "description": "Name of the server to stop"
                    }
                },
                "required": ["server_name"]
            }),
        ),
    ]
}

fn create_tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

/// Handle server management tool calls
pub async fn handle_server_tool(
    tool_name: &str,
    arguments: Value,
    state: Arc<AppState>,
) -> std::result::Result<Value, String> {
    match tool_name {
        "list" => handle_list_servers(state).await,
        "enable" => handle_enable_server(arguments, state).await,
        "disable" => handle_disable_server(arguments, state).await,
        "restart" => handle_restart_server(arguments, state).await,
        "start" => handle_start_server(arguments, state).await,
        "stop" => handle_stop_server(arguments, state).await,
        _ => Err(format!("Unknown server tool: {tool_name}")),
    }
}

async fn handle_list_servers(state: Arc<AppState>) -> std::result::Result<Value, String> {
    let mut servers = Vec::new();

    for entry in state.servers.iter() {
        let name = entry.key();
        let info = entry.value();
        let server_state = info.state.read().await;
        let restart_count = info.restart_count.read().await;

        servers.push(json!({
            "name": name,
            "state": format!("{:?}", *server_state),
            "restart_count": *restart_count,
            "last_access_time": info.last_access_time.read().await.as_ref().map(|t| t.to_rfc3339()),
        }));
    }

    Ok(json!({
        "content": [{"type": "text", "text": serde_json::to_string_pretty(&json!({"servers": servers})).unwrap()}],
        "servers": servers
    }))
}

async fn handle_enable_server(
    arguments: Value,
    state: Arc<AppState>,
) -> std::result::Result<Value, String> {
    let server_name = arguments
        .get("server_name")
        .and_then(|n| n.as_str())
        .ok_or("Missing server_name")?;

    // Update config
    let mut config = state.config.write().await;
    if let Some(server_config) = config.servers.get_mut(server_name) {
        server_config.enabled = true;
        Ok(json!({
            "content": [{"type": "text", "text": format!("Server {} enabled", server_name)}]
        }))
    } else {
        Err(format!("Server {server_name} not found"))
    }
}

async fn handle_disable_server(
    arguments: Value,
    state: Arc<AppState>,
) -> std::result::Result<Value, String> {
    let server_name = arguments
        .get("server_name")
        .and_then(|n| n.as_str())
        .ok_or("Missing server_name")?;

    // Update config
    let mut config = state.config.write().await;
    if let Some(server_config) = config.servers.get_mut(server_name) {
        server_config.enabled = false;
        Ok(json!({
            "content": [{"type": "text", "text": format!("Server {} disabled", server_name)}]
        }))
    } else {
        Err(format!("Server {server_name} not found"))
    }
}

async fn handle_restart_server(
    arguments: Value,
    _state: Arc<AppState>,
) -> std::result::Result<Value, String> {
    let server_name = arguments
        .get("server_name")
        .and_then(|n| n.as_str())
        .ok_or("Missing server_name")?;

    // TODO: Implement actual restart logic via server manager
    Ok(json!({
        "content": [{"type": "text", "text": format!("Server {} restart requested (not yet implemented)", server_name)}]
    }))
}

async fn handle_start_server(
    arguments: Value,
    _state: Arc<AppState>,
) -> std::result::Result<Value, String> {
    let server_name = arguments
        .get("server_name")
        .and_then(|n| n.as_str())
        .ok_or("Missing server_name")?;

    // TODO: Implement actual start logic
    Ok(json!({
        "content": [{"type": "text", "text": format!("Server {} start requested (not yet implemented)", server_name)}]
    }))
}

async fn handle_stop_server(
    arguments: Value,
    _state: Arc<AppState>,
) -> std::result::Result<Value, String> {
    let server_name = arguments
        .get("server_name")
        .and_then(|n| n.as_str())
        .ok_or("Missing server_name")?;

    // TODO: Implement actual stop logic
    Ok(json!({
        "content": [{"type": "text", "text": format!("Server {} stop requested (not yet implemented)", server_name)}]
    }))
}
