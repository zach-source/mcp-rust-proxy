use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolVersion {
    /// MCP Protocol version 2024-11-05 (initial stable release)
    V20241105,
    /// MCP Protocol version 2025-03-26 (adds completions, audio)
    V20250326,
    /// MCP Protocol version 2025-06-18 (adds structured output, titles)
    V20250618,
}

// Stub implementation - will be completed in T006
impl ProtocolVersion {
    pub fn from_string(_s: &str) -> Result<Self, super::error::ProtocolError> {
        todo!("Implementation pending T006")
    }

    pub fn as_str(&self) -> &'static str {
        todo!("Implementation pending T006")
    }
}
