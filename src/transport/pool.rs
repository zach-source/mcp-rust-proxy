use super::{Connection, Transport};
use crate::error::{PoolError, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ConnectionPool {
    connections: DashMap<String, Arc<dyn Connection>>,
    transports: DashMap<String, Arc<dyn Transport>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            transports: DashMap::new(),
        }
    }

    pub async fn add_server(
        &self,
        server_name: String,
        transport: Arc<dyn Transport>,
    ) -> Result<()> {
        // Store the transport for reconnection
        self.transports
            .insert(server_name.clone(), transport.clone());

        // Create initial connection
        tracing::debug!("Creating connection for server: {}", server_name);
        let connection = transport.connect().await?;

        // Perform MCP initialization handshake
        self.initialize_connection(&server_name, &connection)
            .await?;

        self.connections.insert(server_name, connection);

        Ok(())
    }

    async fn initialize_connection(
        &self,
        server_name: &str,
        conn: &Arc<dyn Connection>,
    ) -> Result<()> {
        use crate::protocol::{JsonRpcId, JsonRpcMessage, JsonRpcRequest, JsonRpcV2Message};

        // Step 1: Send initialize request
        let initialize_request = JsonRpcMessage::V2(JsonRpcV2Message::Request(JsonRpcRequest {
            id: JsonRpcId::Number(1),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {
                    "name": "mcp-rust-proxy",
                    "version": "0.1.0"
                }
            })),
        }));

        let request_json = serde_json::to_string(&initialize_request)?;
        let request_bytes = bytes::Bytes::from(format!("{}\n", request_json));

        tracing::debug!("Sending initialize request to {}", server_name);
        conn.send(request_bytes).await?;

        // Wait for initialize response
        let response_bytes = conn.recv().await?;
        let response_str = std::str::from_utf8(&response_bytes)
            .map_err(|_e| crate::error::TransportError::InvalidFormat)?;
        let response: JsonRpcMessage = serde_json::from_str(response_str.trim())?;

        // Verify we got a successful response
        match response {
            JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) => {
                if resp.error.is_some() {
                    return Err(crate::error::TransportError::ConnectionFailed(format!(
                        "Initialize failed for {}: {:?}",
                        server_name, resp.error
                    ))
                    .into());
                }
                tracing::debug!(
                    "Received initialize response from {}: {:?}",
                    server_name,
                    resp.result
                );
            }
            _ => {
                return Err(crate::error::TransportError::ConnectionFailed(format!(
                    "Invalid initialize response from {}",
                    server_name
                ))
                .into());
            }
        }

        // Step 2: Send initialized notification
        let initialized_notification = JsonRpcMessage::V2(JsonRpcV2Message::Notification(
            crate::protocol::JsonRpcNotification {
                method: "initialized".to_string(),
                params: None,
            },
        ));

        let notification_json = serde_json::to_string(&initialized_notification)?;
        let notification_bytes = bytes::Bytes::from(format!("{}\n", notification_json));

        tracing::debug!("Sending initialized notification to {}", server_name);
        conn.send(notification_bytes).await?;

        tracing::info!("Successfully initialized MCP connection to {}", server_name);
        Ok(())
    }

    pub async fn get(&self, server_name: &str) -> Result<Arc<dyn Connection>> {
        // Check if we have an existing connection
        if let Some(conn) = self.connections.get(server_name) {
            if !conn.is_closed() {
                return Ok(conn.clone());
            }
            // Connection is closed, remove it
            drop(conn);
            self.connections.remove(server_name);
        }

        // Try to reconnect
        if let Some(transport) = self.transports.get(server_name) {
            let connection = transport.connect().await?;

            // Initialize the reconnected connection
            self.initialize_connection(server_name, &connection).await?;

            self.connections
                .insert(server_name.to_string(), connection.clone());
            Ok(connection)
        } else {
            Err(PoolError::ServerNotFound(server_name.to_string()).into())
        }
    }

    pub fn remove(&self, server_name: &str) {
        self.connections.remove(server_name);
        self.transports.remove(server_name);
    }

    pub async fn close_all(&self) -> Result<()> {
        for conn in self.connections.iter() {
            let _ = conn.value().close().await;
        }
        self.connections.clear();
        Ok(())
    }
}
