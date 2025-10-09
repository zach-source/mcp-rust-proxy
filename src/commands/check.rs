use crate::config::{Config, ServerConfig};
use crate::error::{ProxyError, Result};
use crate::protocol::{
    JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcV2Message,
};
use crate::transport::create_transport;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, warn};

pub async fn run_config_check(config: Config, test_ping: bool) -> Result<()> {
    info!("Running configuration check...");
    info!("Found {} servers in configuration", config.servers.len());

    // Display global health check configuration
    info!("\nGlobal health check configuration:");
    info!("  Enabled: {}", config.health_check.enabled);
    info!(
        "  Interval: {} seconds",
        config.health_check.interval_seconds
    );
    info!("  Timeout: {} seconds", config.health_check.timeout_seconds);
    info!("  Max attempts: {}", config.health_check.max_attempts);
    info!(
        "  Retry interval: {} seconds",
        config.health_check.retry_interval_seconds
    );

    let mut all_passed = true;

    for (name, server_config) in &config.servers {
        info!("\n--- Checking server: {} ---", name);
        info!("Command: {}", server_config.command);
        info!("Args: {:?}", server_config.args);

        // Check if health checks are enabled for this server
        let effective_hc = config.get_server_health_check(name);
        if let Some(hc_config) = &effective_hc {
            info!("Health check configuration (effective):");
            info!("  Interval: {} seconds", hc_config.interval_seconds);
            info!("  Timeout: {} seconds", hc_config.timeout_seconds);
            info!("  Max attempts: {}", hc_config.max_attempts);
            info!(
                "  Retry interval: {} seconds",
                hc_config.retry_interval_seconds
            );
        } else {
            info!("Health checks: DISABLED");
        }

        if test_ping {
            match test_server_ping(name, server_config).await {
                Ok(true) => info!("✓ Ping test: PASSED"),
                Ok(false) => {
                    warn!("✗ Ping test: NOT SUPPORTED");
                    all_passed = false;
                }
                Err(e) => {
                    error!("✗ Ping test: FAILED - {}", e);
                    all_passed = false;
                }
            }
        }
    }

    if test_ping {
        info!("\n--- Summary ---");
        if all_passed {
            info!("All ping tests passed!");
        } else {
            warn!("Some servers failed ping tests or don't support ping.");
            warn!("Consider disabling health checks for servers that don't support ping.");
        }
    }

    Ok(())
}

async fn test_server_ping(name: &str, config: &ServerConfig) -> Result<bool> {
    info!("Starting server for ping test...");

    // Create transport
    let transport = create_transport(&config.transport, config, None)?;
    let connection = transport.connect().await?;

    // Initialize the connection
    let init_params = serde_json::json!({
        "protocolVersion": "1.0",
        "clientInfo": {
            "name": "mcp-rust-proxy-check",
            "version": "0.1.0"
        }
    });
    let init_msg = JsonRpcMessage::V2(JsonRpcV2Message::Request(JsonRpcRequest {
        method: "initialize".to_string(),
        params: Some(init_params),
        id: JsonRpcId::Number(0),
    }));
    let request_json = serde_json::to_string(&init_msg)?;
    connection
        .send(bytes::Bytes::from(format!("{}\n", request_json)))
        .await?;

    // Wait for initialize response
    let response_bytes = match timeout(Duration::from_secs(5), connection.recv()).await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(ProxyError::Timeout),
    };
    let response_str =
        std::str::from_utf8(&response_bytes).map_err(|_| ProxyError::InvalidResponse)?;
    let _: JsonRpcMessage = serde_json::from_str(response_str.trim())?;

    // Send initialized notification
    let initialized_msg = JsonRpcMessage::V2(JsonRpcV2Message::Notification(JsonRpcNotification {
        method: "initialized".to_string(),
        params: None,
    }));
    let notif_json = serde_json::to_string(&initialized_msg)?;
    connection
        .send(bytes::Bytes::from(format!("{}\n", notif_json)))
        .await?;

    // Give the server time to process initialized
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Now test ping
    info!("Testing ping support...");
    let ping_msg = JsonRpcMessage::V2(JsonRpcV2Message::Request(JsonRpcRequest {
        method: "ping".to_string(),
        params: None,
        id: JsonRpcId::Number(1),
    }));
    let ping_json = serde_json::to_string(&ping_msg)?;
    connection
        .send(bytes::Bytes::from(format!("{}\n", ping_json)))
        .await?;

    // Wait for ping response with timeout
    match timeout(Duration::from_secs(2), connection.recv()).await {
        Ok(Ok(response_bytes)) => {
            let response_str =
                std::str::from_utf8(&response_bytes).map_err(|_| ProxyError::InvalidResponse)?;
            match serde_json::from_str::<JsonRpcMessage>(response_str.trim()) {
                Ok(JsonRpcMessage::V2(JsonRpcV2Message::Response(resp))) => {
                    if resp.error.is_some() {
                        // Server responded with error (doesn't support ping)
                        Ok(false)
                    } else {
                        // Server supports ping
                        Ok(true)
                    }
                }
                _ => Ok(false),
            }
        }
        Ok(Err(_)) => {
            // Transport error
            Err(ProxyError::Transport(
                crate::error::TransportError::ConnectionFailed(
                    "Transport error during ping test".to_string(),
                ),
            ))
        }
        Err(_) => {
            // Timeout - server doesn't support ping
            Ok(false)
        }
    }
}
