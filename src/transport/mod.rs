use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use crate::error::Result;

pub mod stdio;
pub mod http_sse;
pub mod websocket;
pub mod pool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportType {
    Stdio,
    HttpSse,
    WebSocket,
}

#[async_trait]
pub trait Transport: Send + Sync + 'static {
    async fn connect(&self) -> Result<Arc<dyn Connection>>;
    fn transport_type(&self) -> TransportType;
}

#[async_trait]
pub trait Connection: Send + Sync + 'static {
    async fn send(&self, data: Bytes) -> Result<()>;
    async fn recv(&self) -> Result<Bytes>;
    async fn close(&self) -> Result<()>;
    fn is_closed(&self) -> bool;
}

pub fn create_transport(
    config: &crate::config::TransportConfig,
    server_config: &crate::config::ServerConfig,
) -> Result<Arc<dyn Transport>> {
    match config {
        crate::config::TransportConfig::Stdio => {
            let mut transport = stdio::StdioTransport::new();
            transport = transport.with_command(
                server_config.command.clone(),
                server_config.args.clone(),
            );
            if !server_config.env.is_empty() {
                transport = transport.with_env(server_config.env.clone());
            }
            if let Some(ref working_dir) = server_config.working_directory {
                transport = transport.with_working_dir(working_dir.clone());
            }
            Ok(Arc::new(transport))
        }
        crate::config::TransportConfig::HttpSse { url, headers, timeout_ms } => {
            Ok(Arc::new(http_sse::HttpSseTransport::new(
                url.clone(),
                headers.clone(),
                *timeout_ms,
            )))
        }
        crate::config::TransportConfig::WebSocket { url, protocols, auto_reconnect } => {
            Ok(Arc::new(websocket::WebSocketTransport::new(
                url.clone(),
                protocols.clone(),
                *auto_reconnect,
            )))
        }
    }
}