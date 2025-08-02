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

pub fn create_transport(config: &crate::config::TransportConfig) -> Result<Arc<dyn Transport>> {
    match config {
        crate::config::TransportConfig::Stdio => {
            Ok(Arc::new(stdio::StdioTransport::new()))
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