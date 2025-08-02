use dashmap::DashMap;
use std::sync::Arc;
use crate::error::{PoolError, Result};
use super::{Transport, Connection};

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
        self.transports.insert(server_name.clone(), transport.clone());
        
        // Create initial connection
        let connection = transport.connect().await?;
        self.connections.insert(server_name, connection);
        
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
            self.connections.insert(server_name.to_string(), connection.clone());
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