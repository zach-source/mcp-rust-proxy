/// MCP Protocol Version 2025-03-26 specific types
use serde::{Deserialize, Serialize};

// Re-use v1 types for resources and tools (no changes in this version)
pub use super::v20241105::{ResourceContentsV1, ToolV1};

/// Content type (v2 format - adds AudioContent in 2025-03-26)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ContentV2 {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "image")]
    Image {
        data: String, // Base64
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    #[serde(rename = "audio")]
    Audio {
        data: String, // Base64
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    #[serde(rename = "resource")]
    Resource {
        resource: ResourceContentsV1, // Still uses v1 format
    },
}

/// CallToolResult (v2 format - uses ContentV2 with audio support)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallToolResultV2 {
    pub content: Vec<ContentV2>,

    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}
