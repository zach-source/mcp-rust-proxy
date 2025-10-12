/// MCP Protocol Version 2025-06-18 specific types
use serde::{Deserialize, Serialize};

// Re-use ContentV2 from v20250326 (audio support carries forward)
pub use super::v20250326::ContentV2;

/// Resource contents (v2 format - adds required 'name' field in 2025-06-18)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceContentsV2 {
    pub uri: String,
    pub name: String, // REQUIRED in 2025-06-18

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>, // Base64 encoded
}

/// Tool definition (v2 format - adds title and outputSchema in 2025-06-18)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolV2 {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    pub description: String,

    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,

    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

/// CallToolResult (v3 format - adds structuredContent in 2025-06-18)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallToolResultV3 {
    pub content: Vec<ContentV2>,

    #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<serde_json::Value>,

    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}
