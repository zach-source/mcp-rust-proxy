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

impl ProtocolVersion {
    /// Parse version string from initialize message
    ///
    /// If version is unsupported, logs a warning and returns the proxy's
    /// default version (V20250326) to enable pass-through operation.
    /// Returns (version, is_supported) tuple.
    pub fn from_string(s: &str) -> (Self, bool) {
        match s {
            "2024-11-05" => (Self::V20241105, true),
            "2025-03-26" => (Self::V20250326, true),
            "2025-06-18" => (Self::V20250618, true),
            unsupported => {
                tracing::warn!(
                    reported_version = unsupported,
                    supported_versions = ?["2024-11-05", "2025-03-26", "2025-06-18"],
                    "Backend server reports unsupported protocol version, using pass-through mode"
                );
                (Self::V20250326, false)
            }
        }
    }

    /// Get version string for initialize messages
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V20241105 => "2024-11-05",
            Self::V20250326 => "2025-03-26",
            Self::V20250618 => "2025-06-18",
        }
    }

    /// Check if this version supports audio content
    pub fn supports_audio_content(&self) -> bool {
        matches!(self, Self::V20250326 | Self::V20250618)
    }

    /// Check if this version supports completions capability
    pub fn supports_completions(&self) -> bool {
        matches!(self, Self::V20250326 | Self::V20250618)
    }

    /// Check if this version requires ResourceContents.name field
    pub fn requires_resource_name(&self) -> bool {
        matches!(self, Self::V20250618)
    }

    /// Check if this version supports structured content output
    pub fn supports_structured_content(&self) -> bool {
        matches!(self, Self::V20250618)
    }

    /// Check if this version supports elicitation capability
    pub fn supports_elicitation(&self) -> bool {
        matches!(self, Self::V20250618)
    }

    /// Check if this version supports title fields
    pub fn supports_title_fields(&self) -> bool {
        matches!(self, Self::V20250618)
    }

    /// Check if this version supports output schema in tools
    pub fn supports_output_schema(&self) -> bool {
        matches!(self, Self::V20250618)
    }
}
