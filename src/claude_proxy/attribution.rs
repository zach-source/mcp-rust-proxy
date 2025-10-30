//! Context source attribution for Claude API requests
//!
//! This module analyzes Claude API request payloads to identify which context came from
//! which source (MCP servers, skills, user input, framework system prompts).

use crate::context::types::{ContextAttribution, SourceType};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Attribution engine for analyzing Claude API requests
pub struct AttributionEngine;

impl AttributionEngine {
    /// Analyze a Claude API request and generate context attributions
    pub fn analyze_request(request_json: &serde_json::Value) -> Vec<ContextAttribution> {
        let mut attributions = Vec::new();

        // Analyze system prompt
        if let Some(system) = request_json.get("system") {
            if let Some(system_str) = system.as_str() {
                let attribution = ContextAttribution {
                    id: format!("attr_{}", Uuid::new_v4()),
                    request_id: String::new(), // Will be set when storing
                    source_type: SourceType::Framework,
                    source_name: None,
                    token_count: Self::estimate_tokens(system_str),
                    content_hash: Self::hash_content(system_str),
                    message_index: 0,
                    message_role: "system".to_string(),
                };
                attributions.push(attribution);
            }
        }

        // Analyze messages array
        if let Some(messages) = request_json.get("messages").and_then(|m| m.as_array()) {
            for (index, message) in messages.iter().enumerate() {
                if let Some(role) = message.get("role").and_then(|r| r.as_str()) {
                    let (source_type, source_name) = Self::identify_source_type(message);

                    // Extract content for token counting
                    let content = message.get("content").unwrap_or(&serde_json::Value::Null);
                    let content_str = Self::extract_content_string(content);

                    let attribution = ContextAttribution {
                        id: format!("attr_{}", Uuid::new_v4()),
                        request_id: String::new(), // Will be set when storing
                        source_type,
                        source_name,
                        token_count: Self::estimate_tokens(&content_str),
                        content_hash: Self::hash_content(&content_str),
                        message_index: index + 1, // +1 because system is index 0
                        message_role: role.to_string(),
                    };
                    attributions.push(attribution);
                }
            }
        }

        attributions
    }

    /// Identify the source type and name from a message
    fn identify_source_type(message: &serde_json::Value) -> (SourceType, Option<String>) {
        let role = message.get("role").and_then(|r| r.as_str()).unwrap_or("");

        // Check for tool results (MCP server)
        if let Some(content) = message.get("content") {
            if let Some(content_array) = content.as_array() {
                for item in content_array {
                    if item.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                        if let Some(tool_use_id) = item.get("tool_use_id").and_then(|t| t.as_str())
                        {
                            if let Some(server_name) = Self::extract_mcp_server_name(tool_use_id) {
                                return (SourceType::McpServer, Some(server_name));
                            }
                        }
                    }
                }
            }
        }

        // Check for skills in content (vectorize, etc.)
        if let Some(content_str) = message.get("content").and_then(|c| c.as_str()) {
            if content_str.contains("vectorize") || content_str.contains("vector search") {
                return (SourceType::Skill, Some("vectorize".to_string()));
            }
        }

        // Default: User input for user role messages
        if role == "user" {
            (SourceType::User, None)
        } else {
            (SourceType::Framework, None)
        }
    }

    /// Extract MCP server name from tool_use_id
    /// Format: "tool_xyz123" or with prefix like "mcp__proxy__context7__get_docs"
    fn extract_mcp_server_name(tool_use_id: &str) -> Option<String> {
        // Check if it contains mcp server prefix pattern
        if tool_use_id.contains("mcp__") {
            let parts: Vec<&str> = tool_use_id.split("__").collect();
            if parts.len() >= 3 {
                // Format: mcp__proxy__servername__toolname
                return Some(parts[2].to_string());
            }
        }
        None
    }

    /// Estimate token count for content (simple word-based estimation)
    /// TODO: T031 will replace this with tiktoken-rs for accurate counting
    fn estimate_tokens(content: &str) -> usize {
        // Rough estimation: ~0.75 tokens per word
        let words = content.split_whitespace().count();
        (words as f64 * 0.75).ceil() as usize
    }

    /// Hash content for deduplication
    fn hash_content(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Extract content string from content field (handles string or array formats)
    fn extract_content_string(content: &serde_json::Value) -> String {
        match content {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(arr) => arr
                .iter()
                .filter_map(|item| {
                    item.get("text")
                        .or_else(|| item.get("content"))
                        .and_then(|v| v.as_str())
                })
                .collect::<Vec<_>>()
                .join(" "),
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_identify_user_message() {
        let message = json!({
            "role": "user",
            "content": "Hello, how are you?"
        });

        let (source_type, source_name) = AttributionEngine::identify_source_type(&message);
        assert_eq!(source_type, SourceType::User);
        assert_eq!(source_name, None);
    }

    #[test]
    fn test_identify_mcp_tool_result() {
        let message = json!({
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "mcp__proxy__context7__get_docs",
                "content": "Documentation content"
            }]
        });

        let (source_type, source_name) = AttributionEngine::identify_source_type(&message);
        assert_eq!(source_type, SourceType::McpServer);
        assert_eq!(source_name, Some("context7".to_string()));
    }

    #[test]
    fn test_extract_server_name_from_tool_id() {
        let name = AttributionEngine::extract_mcp_server_name("mcp__proxy__serena__find_symbol");
        assert_eq!(name, Some("serena".to_string()));

        let name = AttributionEngine::extract_mcp_server_name("mcp__proxy__context7__get_docs");
        assert_eq!(name, Some("context7".to_string()));

        let name = AttributionEngine::extract_mcp_server_name("regular_tool_id");
        assert_eq!(name, None);
    }

    #[test]
    fn test_token_estimation() {
        let content = "This is a test message with several words";
        let tokens = AttributionEngine::estimate_tokens(content);
        assert!(tokens > 0);
        assert!(tokens < content.len()); // Tokens should be less than character count
    }

    #[test]
    fn test_analyze_request() {
        let request = json!({
            "model": "claude-3-5-sonnet-20241022",
            "system": "You are a helpful assistant",
            "messages": [
                {
                    "role": "user",
                    "content": "What is Rust?"
                },
                {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": "mcp__proxy__context7__get_docs",
                        "content": "Rust is a systems programming language"
                    }]
                }
            ]
        });

        let attributions = AttributionEngine::analyze_request(&request);

        // Should have 3 attributions: system prompt, user message, mcp tool result
        assert_eq!(attributions.len(), 3);

        // Check system prompt
        assert_eq!(attributions[0].source_type, SourceType::Framework);
        assert_eq!(attributions[0].message_role, "system");

        // Check user message
        assert_eq!(attributions[1].source_type, SourceType::User);

        // Check MCP tool result
        assert_eq!(attributions[2].source_type, SourceType::McpServer);
        assert_eq!(attributions[2].source_name, Some("context7".to_string()));
    }
}
