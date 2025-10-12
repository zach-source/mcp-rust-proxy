use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "jsonrpc")]
pub enum JsonRpcMessage {
    #[serde(rename = "2.0")]
    V2(JsonRpcV2Message),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcV2Message {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub id: JsonRpcId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(i64),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// MCP-specific message types
pub mod mcp {
    use super::*;

    pub const PING_METHOD: &str = "ping";

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PingRequest {}

    #[derive(Debug, Serialize, Deserialize)]
    pub struct PingResponse {}

    pub fn create_ping_request(id: JsonRpcId) -> JsonRpcMessage {
        JsonRpcMessage::V2(JsonRpcV2Message::Request(JsonRpcRequest {
            id,
            method: PING_METHOD.to_string(),
            params: Some(serde_json::json!({})),
        }))
    }
}

// Protocol version support modules
pub mod adapter;
pub mod error;
pub mod handshake;
pub mod state;
pub mod version;

// Re-exports for convenience
pub use adapter::ProtocolAdapter;
pub use error::ProtocolError;
pub use version::ProtocolVersion;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
