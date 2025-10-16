use crate::config::{Config, TransportConfig};
use crate::error::Result;
use crate::protocol::{mcp, JsonRpcId, JsonRpcMessage, JsonRpcV2Message};
use crate::transport::create_transport;
use bytes::Bytes;
use serde_json::json;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

pub async fn run_config_check(config: Config, ping: bool) -> Result<()> {
    info!("Checking configuration...");

    // Validate configuration
    crate::config::validate(&config)?;
    info!("✓ Configuration is valid");

    // Check servers
    info!("Checking {} server(s)...", config.servers.len());

    let mut all_ok = true;

    for (name, server_config) in &config.servers {
        info!("\nChecking server: {}", name);
        info!(
            "  Command: {} {}",
            server_config.command,
            server_config.args.join(" ")
        );
        info!("  Transport: {:?}", server_config.transport);

        if ping && matches!(server_config.transport, TransportConfig::Stdio) {
            match test_server_ping(name, server_config).await {
                Ok(()) => {
                    info!("  ✓ Ping test passed");
                }
                Err(e) => {
                    error!("  ✗ Ping test failed: {}", e);
                    all_ok = false;
                }
            }
        } else if ping && !matches!(server_config.transport, TransportConfig::Stdio) {
            warn!("  ! Ping test skipped (only supported for stdio transport)");
        }
    }

    if all_ok {
        info!("\n✓ All checks passed");
        Ok(())
    } else {
        Err(crate::error::ProxyError::Config(
            crate::error::ConfigError::Validation("One or more servers failed checks".to_string()),
        ))
    }
}

async fn test_server_ping(name: &str, config: &crate::config::ServerConfig) -> Result<()> {
    // Create transport
    let transport = create_transport(&config.transport, config, None)?;

    // Connect
    let connection = timeout(Duration::from_secs(10), transport.connect())
        .await
        .map_err(|_| crate::error::ProxyError::Timeout)?
        .map_err(|e| {
            error!("Failed to connect to server {}: {}", name, e);
            e
        })?;

    // Send initialize request
    let init_request =
        JsonRpcMessage::V2(JsonRpcV2Message::Request(crate::protocol::JsonRpcRequest {
            id: JsonRpcId::Number(1),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "0.1.0",
                "capabilities": {},
                "clientInfo": {
                    "name": "mcp-rust-proxy",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
        }));

    let request_json = serde_json::to_string(&init_request)?;
    connection
        .send(Bytes::from(format!("{request_json}\n")))
        .await?;

    // Wait for initialize response
    let response_bytes = timeout(Duration::from_secs(5), connection.recv())
        .await
        .map_err(|_| crate::error::ProxyError::Timeout)??;

    let response_str = std::str::from_utf8(&response_bytes)
        .map_err(|_| crate::error::ProxyError::InvalidResponse)?;
    let _response: JsonRpcMessage = serde_json::from_str(response_str.trim())?;

    // Send ping request
    let ping_request = mcp::create_ping_request(JsonRpcId::Number(2));
    let ping_json = serde_json::to_string(&ping_request)?;
    connection
        .send(Bytes::from(format!("{ping_json}\n")))
        .await?;

    // Wait for ping response
    let ping_response_bytes = timeout(Duration::from_secs(5), connection.recv())
        .await
        .map_err(|_| crate::error::ProxyError::Timeout)??;

    let ping_response_str = std::str::from_utf8(&ping_response_bytes)
        .map_err(|_| crate::error::ProxyError::InvalidResponse)?;
    let ping_response: JsonRpcMessage = serde_json::from_str(ping_response_str.trim())?;

    // Check if ping was successful
    match ping_response {
        JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) => {
            if resp.error.is_some() {
                return Err(crate::error::ProxyError::InvalidResponse);
            }
            Ok(())
        }
        _ => Err(crate::error::ProxyError::InvalidResponse),
    }
}
