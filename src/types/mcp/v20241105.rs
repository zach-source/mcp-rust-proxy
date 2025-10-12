/// MCP Protocol Version 2024-11-05 specific types
use serde::{Deserialize, Serialize};

/// Resource contents (v1 format - used in 2024-11-05 and 2025-03-26)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceContentsV1 {
    pub uri: String,

    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>, // Base64 encoded
}

/// Tool definition (v1 format - used in 2024-11-05 and 2025-03-26)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolV1 {
    pub name: String,
    pub description: String,

    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value, // JSON Schema
}

/// Content type (v1 format - used in 2024-11-05)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ContentV1 {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image {
        data: String, // Base64
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    #[serde(rename = "resource")]
    Resource { resource: ResourceContentsV1 },
}

/// CallToolResult (v1 format - used in 2024-11-05)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallToolResultV1 {
    pub content: Vec<ContentV1>,

    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}
