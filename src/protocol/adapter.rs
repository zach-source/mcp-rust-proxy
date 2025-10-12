use async_trait::async_trait;
use serde_json::Value;

use super::error::ProtocolError;
use super::version::ProtocolVersion;

/// Trait for protocol version adapters that translate messages between different MCP versions
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// Get the source protocol version this adapter handles
    fn source_version(&self) -> ProtocolVersion;

    /// Get the target protocol version this adapter produces
    fn target_version(&self) -> ProtocolVersion;

    /// Translate a request from source to target version
    async fn translate_request(&self, request: Value) -> Result<Value, ProtocolError>;

    /// Translate a response from target back to source version
    async fn translate_response(&self, response: Value) -> Result<Value, ProtocolError>;

    /// Translate a notification from target back to source version
    async fn translate_notification(&self, notification: Value) -> Result<Value, ProtocolError>;
}
