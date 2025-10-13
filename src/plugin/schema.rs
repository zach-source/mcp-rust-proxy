//! Plugin I/O schema definitions and serialization
//!
//! This module defines the data structures for plugin input/output communication.
//! Plugins communicate with the proxy via stdin/stdout using JSON serialization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Execution phase for plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginPhase {
    /// Request phase (before forwarding to MCP server)
    #[serde(rename = "request")]
    Request,
    /// Response phase (after receiving response from MCP server)
    #[serde(rename = "response")]
    Response,
}

/// Metadata associated with plugin execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMetadata {
    /// Unique request identifier
    pub request_id: String,

    /// ISO 8601 timestamp
    pub timestamp: String,

    /// Name of MCP server
    pub server_name: String,

    /// Execution phase
    pub phase: PluginPhase,

    /// Original user query (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_query: Option<String>,

    /// Original tool arguments (all parameters from the MCP call)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_arguments: Option<serde_json::Value>,

    /// MCP server configurations (for aggregator plugin)
    #[serde(skip_serializing_if = "Option::is_none", rename = "mcpServers")]
    pub mcp_servers: Option<Vec<serde_json::Value>>,
}

/// Input data passed to plugin processes via stdin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInput {
    /// Name of MCP tool being invoked (e.g., "context7/get-docs")
    pub tool_name: String,

    /// Original request/response content
    pub raw_content: String,

    /// Token limit for curation plugins (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Execution context metadata
    pub metadata: PluginMetadata,
}

/// Output data returned by plugin processes via stdout
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginOutput {
    /// Modified content (or original if unchanged)
    pub text: String,

    /// Whether to continue plugin chain execution
    pub continue_: bool,

    /// Plugin-specific metadata (any valid JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Error message if plugin failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Plugin-specific errors
#[derive(Debug, Error)]
pub enum PluginError {
    /// Plugin process timed out
    #[error("Plugin timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Plugin process failed to spawn
    #[error("Failed to spawn plugin process: {reason}")]
    SpawnFailed { reason: String },

    /// Plugin process crashed or exited with non-zero code
    #[error("Plugin process failed with exit code {code}: {stderr}")]
    ProcessFailed { code: i32, stderr: String },

    /// Plugin returned invalid JSON output
    #[error("Plugin returned invalid JSON: {reason}")]
    InvalidOutput { reason: String },

    /// Plugin I/O error (stdin/stdout)
    #[error("Plugin I/O error: {reason}")]
    IoError { reason: String },

    /// Plugin configuration error
    #[error("Plugin configuration error: {reason}")]
    ConfigError { reason: String },

    /// Plugin not found
    #[error("Plugin '{name}' not found")]
    NotFound { name: String },

    /// Semaphore acquisition failed (pool exhausted)
    #[error("Plugin execution pool exhausted")]
    PoolExhausted,
}

impl PluginInput {
    /// Validate plugin input according to schema rules
    pub fn validate(&self) -> Result<(), PluginError> {
        if self.tool_name.is_empty() {
            return Err(PluginError::ConfigError {
                reason: "tool_name cannot be empty".to_string(),
            });
        }

        if let Some(max_tokens) = self.max_tokens {
            if max_tokens == 0 {
                return Err(PluginError::ConfigError {
                    reason: "max_tokens must be greater than 0".to_string(),
                });
            }
        }

        if self.metadata.request_id.is_empty() {
            return Err(PluginError::ConfigError {
                reason: "metadata.request_id cannot be empty".to_string(),
            });
        }

        if self.metadata.server_name.is_empty() {
            return Err(PluginError::ConfigError {
                reason: "metadata.server_name cannot be empty".to_string(),
            });
        }

        Ok(())
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, PluginError> {
        serde_json::to_string(self).map_err(|e| PluginError::IoError {
            reason: format!("Failed to serialize input: {}", e),
        })
    }
}

impl PluginOutput {
    /// Validate plugin output according to schema rules
    pub fn validate(&self) -> Result<(), PluginError> {
        // If error is present, continue must be false
        if self.error.is_some() && self.continue_ {
            return Err(PluginError::InvalidOutput {
                reason: "When error is present, continue must be false".to_string(),
            });
        }

        Ok(())
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, PluginError> {
        let output: PluginOutput =
            serde_json::from_str(json).map_err(|e| PluginError::InvalidOutput {
                reason: format!("Failed to parse output JSON: {}", e),
            })?;

        output.validate()?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_input_validation() {
        let valid_input = PluginInput {
            tool_name: "context7/get-docs".to_string(),
            raw_content: "test content".to_string(),
            max_tokens: Some(1200),
            metadata: PluginMetadata {
                request_id: "req-123".to_string(),
                timestamp: "2025-10-10T12:00:00Z".to_string(),
                server_name: "context7".to_string(),
                phase: PluginPhase::Response,
                user_query: Some("test query".to_string()),
                tool_arguments: None,
            },
        };

        assert!(valid_input.validate().is_ok());

        // Test empty tool_name
        let mut invalid_input = valid_input.clone();
        invalid_input.tool_name = "".to_string();
        assert!(invalid_input.validate().is_err());

        // Test zero max_tokens
        let mut invalid_input = valid_input.clone();
        invalid_input.max_tokens = Some(0);
        assert!(invalid_input.validate().is_err());
    }

    #[test]
    fn test_plugin_output_validation() {
        let valid_output = PluginOutput {
            text: "processed content".to_string(),
            continue_: true,
            metadata: None,
            error: None,
        };

        assert!(valid_output.validate().is_ok());

        // Test error with continue=true (invalid)
        let invalid_output = PluginOutput {
            text: "content".to_string(),
            continue_: true,
            metadata: None,
            error: Some("error occurred".to_string()),
        };

        assert!(invalid_output.validate().is_err());

        // Test error with continue=false (valid)
        let valid_error_output = PluginOutput {
            text: "original content".to_string(),
            continue_: false,
            metadata: None,
            error: Some("error occurred".to_string()),
        };

        assert!(valid_error_output.validate().is_ok());
    }

    #[test]
    fn test_json_serialization() {
        let input = PluginInput {
            tool_name: "test/tool".to_string(),
            raw_content: "content".to_string(),
            max_tokens: None,
            metadata: PluginMetadata {
                request_id: "req-1".to_string(),
                timestamp: "2025-10-10T12:00:00Z".to_string(),
                server_name: "test".to_string(),
                phase: PluginPhase::Request,
                user_query: None,
                tool_arguments: None,
            },
        };

        let json = input.to_json().unwrap();
        assert!(json.contains("\"toolName\":\"test/tool\""));
        assert!(json.contains("\"phase\":\"request\""));
    }

    #[test]
    fn test_json_deserialization() {
        let json = r#"{
            "text": "output text",
            "continue": true,
            "metadata": {"key": "value"}
        }"#;

        let output = PluginOutput::from_json(json).unwrap();
        assert_eq!(output.text, "output text");
        assert!(output.continue_);
        assert!(output.metadata.is_some());
    }
}
