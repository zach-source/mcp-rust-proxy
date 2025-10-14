use crate::error::Result;
use crate::state::AppState;
use serde_json::{json, Value};
use std::sync::Arc;

/// Get all proxy-native resources
pub fn get_proxy_resources() -> Vec<Value> {
    vec![
        create_resource(
            "proxy://config",
            "Current proxy configuration (sanitized)",
            "application/json",
        ),
        create_resource(
            "proxy://metrics",
            "Real-time proxy performance metrics",
            "application/json",
        ),
        create_resource(
            "proxy://health",
            "Health status of all backend servers",
            "application/json",
        ),
        create_resource(
            "proxy://topology",
            "Server topology and capability mapping",
            "application/json",
        ),
    ]
}

/// Create a resource definition
fn create_resource(uri: &str, description: &str, mime_type: &str) -> Value {
    json!({
        "uri": uri,
        "name": uri.strip_prefix("proxy://").unwrap_or(uri),
        "description": description,
        "mimeType": mime_type
    })
}

/// Handle reading a proxy resource
pub async fn handle_proxy_resource(uri: &str, state: Arc<AppState>) -> Result<Value> {
    match uri {
        "proxy://config" => get_config_resource(state).await,
        "proxy://metrics" => get_metrics_resource(state).await,
        "proxy://health" => get_health_resource(state).await,
        "proxy://topology" => get_topology_resource(state).await,
        _ => {
            // Check if it's a URI template pattern
            if uri.starts_with("proxy://logs/") {
                get_logs_resource(uri, state).await
            } else if uri.starts_with("proxy://metrics/") {
                get_server_metrics_resource(uri, state).await
            } else if uri.starts_with("proxy://server/") {
                get_server_resource(uri, state).await
            } else {
                Err(crate::error::ProxyError::InvalidRequest(format!(
                    "Unknown proxy resource: {uri}"
                )))
            }
        }
    }
}

/// Get sanitized proxy configuration
async fn get_config_resource(state: Arc<AppState>) -> Result<Value> {
    let config = state.config.read().await;

    // Sanitize sensitive data
    let mut servers_map = serde_json::Map::new();
    for (name, server_config) in &config.servers {
        let transport_type = match &server_config.transport {
            crate::config::schema::TransportConfig::Stdio => "stdio",
            crate::config::schema::TransportConfig::HttpSse { .. } => "httpSse",
            crate::config::schema::TransportConfig::WebSocket { .. } => "webSocket",
        };

        servers_map.insert(
            name.clone(),
            json!({
                "transport": transport_type,
                "enabled": server_config.enabled,
                "restartOnFailure": server_config.restart_on_failure,
                "maxRestarts": server_config.max_restarts,
                "healthCheckEnabled": server_config.health_check.as_ref().and_then(|hc| hc.enabled).unwrap_or(false),
            }),
        );
    }

    Ok(json!({
        "contents": [{
            "uri": "proxy://config",
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&json!({
                "servers": servers_map,
                "proxy": {
                    "host": config.proxy.host,
                    "port": config.proxy.port,
                    "connectionPoolSize": config.proxy.connection_pool_size,
                },
                "webUi": {
                    "enabled": config.web_ui.enabled,
                    "port": config.web_ui.port,
                }
            })).unwrap()
        }]
    }))
}

/// Get proxy performance metrics
async fn get_metrics_resource(state: Arc<AppState>) -> Result<Value> {
    // Build metrics snapshot from Prometheus metrics
    let metrics_data = json!({
        "servers": {
            "total": state.metrics.total_servers.get(),
            "running": state.metrics.running_servers.get(),
            "failed": state.metrics.failed_servers.get(),
        },
        "requests": {
            "total": state.metrics.total_requests.get(),
            "failed": state.metrics.failed_requests.get(),
        },
        "connections": {
            "active": state.metrics.active_connections.get(),
            "errors": state.metrics.connection_errors.get(),
        }
    });

    Ok(json!({
        "contents": [{
            "uri": "proxy://metrics",
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&metrics_data).unwrap()
        }]
    }))
}

/// Get health status of all servers
async fn get_health_resource(state: Arc<AppState>) -> Result<Value> {
    let mut server_health = Vec::new();
    let mut healthy_count = 0;
    let mut failed_count = 0;
    let mut stopped_count = 0;

    // Check config for enabled status
    let config = state.config.read().await;

    for entry in state.servers.iter() {
        let name = entry.key();
        let info = entry.value();
        let state_value = info.state.read().await;
        let restart_count = info.restart_count.read().await;

        let enabled = config
            .servers
            .get(name.as_str())
            .map(|s| s.enabled)
            .unwrap_or(false);

        let health_status = json!({
            "name": name,
            "state": format!("{:?}", *state_value),
            "enabled": enabled,
            "restartCount": *restart_count,
        });

        match *state_value {
            crate::state::ServerState::Running => healthy_count += 1,
            crate::state::ServerState::Failed => failed_count += 1,
            crate::state::ServerState::Stopped => stopped_count += 1,
            _ => {}
        }

        server_health.push(health_status);
    }

    Ok(json!({
        "contents": [{
            "uri": "proxy://health",
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&json!({
                "summary": {
                    "healthy": healthy_count,
                    "failed": failed_count,
                    "stopped": stopped_count,
                    "total": server_health.len()
                },
                "servers": server_health
            })).unwrap()
        }]
    }))
}

/// Get server topology and capabilities
async fn get_topology_resource(state: Arc<AppState>) -> Result<Value> {
    let mut topology = Vec::new();
    let config = state.config.read().await;

    for entry in state.servers.iter() {
        let name = entry.key();
        let info = entry.value();
        let enabled = config
            .servers
            .get(name.as_str())
            .map(|s| s.enabled)
            .unwrap_or(false);

        topology.push(json!({
            "name": name,
            "enabled": enabled,
            "state": format!("{:?}", *info.state.read().await),
            "toolsCount": 0, // TODO: Cache _tool counts per server
            "hasResources": false, // TODO: Track this during initialization
            "hasPrompts": false, // TODO: Track this during initialization
        }));
    }

    Ok(json!({
        "contents": [{
            "uri": "proxy://topology",
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&json!({
                "servers": topology,
                "totalServers": topology.len()
            })).unwrap()
        }]
    }))
}

/// Get logs for a specific server (URI template pattern)
async fn get_logs_resource(uri: &str, state: Arc<AppState>) -> Result<Value> {
    // Parse: proxy://logs/{server_name}?lines=100
    let server_name = uri
        .strip_prefix("proxy://logs/")
        .and_then(|s| s.split('?').next())
        .ok_or_else(|| crate::error::ProxyError::InvalidRequest("Invalid logs URI".to_string()))?;

    // TODO: Parse query parameters for lines, level, etc.
    // For now, return placeholder
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "text/plain",
            "text": format!("Logs for server '{}' - Feature coming soon!\nUse the Web UI at http://localhost:3001 to view logs.", server_name)
        }]
    }))
}

/// Get metrics for a specific server (URI template pattern)
async fn get_server_metrics_resource(uri: &str, state: Arc<AppState>) -> Result<Value> {
    // Parse: proxy://metrics/{server_name}
    let server_name = uri.strip_prefix("proxy://metrics/").ok_or_else(|| {
        crate::error::ProxyError::InvalidRequest("Invalid metrics URI".to_string())
    })?;

    // TODO: Implement per-server metrics tracking
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": json!({
                "server": server_name,
                "message": "Per-server metrics coming soon!"
            }).to_string()
        }]
    }))
}

/// Get server configuration or capabilities (URI template pattern)
async fn get_server_resource(uri: &str, state: Arc<AppState>) -> Result<Value> {
    // Parse: proxy://server/{server_name}/{config|capabilities}
    let parts: Vec<&str> = uri
        .strip_prefix("proxy://server/")
        .unwrap_or("")
        .split('/')
        .collect();

    if parts.len() != 2 {
        return Err(crate::error::ProxyError::InvalidRequest(
            "Invalid server resource URI. Use proxy://server/{name}/config or proxy://server/{name}/capabilities".to_string()
        ));
    }

    let server_name = parts[0];
    let resource_type = parts[1];

    match resource_type {
        "config" => get_server_config(server_name, state).await,
        "capabilities" => get_server_capabilities(server_name, state).await,
        _ => Err(crate::error::ProxyError::InvalidRequest(format!(
            "Unknown server resource type: {resource_type}. Use 'config' or 'capabilities'"
        ))),
    }
}

async fn get_server_config(server_name: &str, state: Arc<AppState>) -> Result<Value> {
    let config = state.config.read().await;
    let server_config = config
        .servers
        .get(server_name)
        .ok_or_else(|| crate::error::ProxyError::ServerNotFound(server_name.to_string()))?;

    let transport_type = match &server_config.transport {
        crate::config::schema::TransportConfig::Stdio => "stdio",
        crate::config::schema::TransportConfig::HttpSse { .. } => "httpSse",
        crate::config::schema::TransportConfig::WebSocket { .. } => "webSocket",
    };

    Ok(json!({
        "contents": [{
            "uri": format!("proxy://server/{}/config", server_name),
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&json!({
                "name": server_name,
                "transport": transport_type,
                "enabled": server_config.enabled,
                "restartOnFailure": server_config.restart_on_failure,
            })).unwrap()
        }]
    }))
}

async fn get_server_capabilities(server_name: &str, state: Arc<AppState>) -> Result<Value> {
    // TODO: Store capabilities from initialize response
    Ok(json!({
        "contents": [{
            "uri": format!("proxy://server/{}/capabilities", server_name),
            "mimeType": "application/json",
            "text": json!({
                "server": server_name,
                "message": "Capability caching coming soon!"
            }).to_string()
        }]
    }))
}
