/// T041: V20250618 to V20241105 Adapter
///
/// Translates messages from 2025-06-18 format to 2024-11-05 format (backward compatibility)
use crate::protocol::{ProtocolAdapter, ProtocolError, ProtocolVersion};
use async_trait::async_trait;
use serde_json::Value;

pub struct V20250618ToV20241105Adapter;

impl V20250618ToV20241105Adapter {
    pub fn new() -> Self {
        Self
    }

    /// Translate tools/list response (strip title, outputSchema)
    fn translate_tools_list_response(&self, mut response: Value) -> Result<Value, ProtocolError> {
        if let Some(result) = response.get_mut("result") {
            if let Some(tools) = result.get_mut("tools").and_then(|t| t.as_array_mut()) {
                for tool in tools {
                    if let Some(tool_obj) = tool.as_object_mut() {
                        // Strip title field
                        if let Some(title) = tool_obj.remove("title") {
                            if !title.is_null() {
                                tracing::debug!(
                                    tool_name = tool_obj
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("unknown"),
                                    title = title.as_str().unwrap_or(""),
                                    "Stripping title field during v2→v1 translation"
                                );
                            }
                        }

                        // Strip outputSchema field
                        if tool_obj.remove("outputSchema").is_some() {
                            tracing::debug!(
                                tool_name = tool_obj
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown"),
                                "Stripping outputSchema field during v2→v1 translation"
                            );
                        }
                    }
                }
            }
        }
        Ok(response)
    }

    /// Translate resources/read response (strip name, title)
    fn translate_resources_read_response(
        &self,
        mut response: Value,
    ) -> Result<Value, ProtocolError> {
        if let Some(result) = response.get_mut("result") {
            if let Some(contents) = result.get_mut("contents").and_then(|c| c.as_array_mut()) {
                for content in contents {
                    if let Some(content_obj) = content.as_object_mut() {
                        // Strip name field (required in v2, not present in v1)
                        content_obj.remove("name");

                        // Strip title field
                        content_obj.remove("title");
                    }
                }
            }
        }
        Ok(response)
    }

    /// Translate tools/call response (strip structuredContent, convert audio to text)
    fn translate_tools_call_response(&self, mut response: Value) -> Result<Value, ProtocolError> {
        if let Some(result) = response.get_mut("result") {
            // Strip structuredContent field
            if result
                .as_object_mut()
                .and_then(|r| r.remove("structuredContent"))
                .is_some()
            {
                tracing::debug!("Stripping structuredContent field during v2→v1 translation");
            }

            // Convert audio content to text descriptions
            if let Some(content_array) = result.get_mut("content").and_then(|c| c.as_array_mut()) {
                for content in content_array {
                    if let Some(content_obj) = content.as_object_mut() {
                        if content_obj.get("type").and_then(|t| t.as_str()) == Some("audio") {
                            // Convert to text content
                            let mime_type = content_obj
                                .get("mimeType")
                                .and_then(|m| m.as_str())
                                .unwrap_or("unknown");

                            *content_obj = serde_json::json!({
                                "type": "text",
                                "text": format!("[Audio content: {}]", mime_type)
                            })
                            .as_object()
                            .unwrap()
                            .clone();

                            tracing::debug!("Converted audio content to text description");
                        }
                    }
                }
            }
        }
        Ok(response)
    }
}

#[async_trait]
impl ProtocolAdapter for V20250618ToV20241105Adapter {
    fn source_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250618
    }

    fn target_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20241105
    }

    async fn translate_request(&self, request: Value) -> Result<Value, ProtocolError> {
        // Requests from v2 to v1 generally don't need modification
        Ok(request)
    }

    async fn translate_response(&self, response: Value) -> Result<Value, ProtocolError> {
        // Infer method from response structure
        if let Some(result) = response.get("result") {
            if result.get("tools").is_some() {
                return self.translate_tools_list_response(response);
            }
            if result.get("contents").is_some() {
                return self.translate_resources_read_response(response);
            }
            if result.get("content").is_some() {
                return self.translate_tools_call_response(response);
            }
        }

        // For other responses, pass through
        Ok(response)
    }

    async fn translate_notification(&self, notification: Value) -> Result<Value, ProtocolError> {
        // Drop resources/updated notification (not supported in 2024-11-05)
        if let Some(method) = notification.get("method").and_then(|m| m.as_str()) {
            if method == "notifications/resources/updated" {
                tracing::debug!("Dropping resources/updated notification for v1 client");
                return Err(ProtocolError::UnsupportedNotification {
                    method: method.to_string(),
                    version: ProtocolVersion::V20241105,
                });
            }
        }

        Ok(notification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_adapter_versions() {
        let adapter = V20250618ToV20241105Adapter::new();
        assert_eq!(adapter.source_version(), ProtocolVersion::V20250618);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20241105);
    }

    #[tokio::test]
    async fn test_tools_list_strips_fields() {
        let adapter = V20250618ToV20241105Adapter::new();

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [{
                    "name": "my-tool",
                    "title": "My Tool",
                    "description": "Does things",
                    "inputSchema": {"type": "object"},
                    "outputSchema": {"type": "string"}
                }]
            }
        });

        let translated = adapter.translate_response(response).await.unwrap();

        let tool = &translated["result"]["tools"][0];
        assert_eq!(tool["name"], "my-tool");
        assert_eq!(tool["description"], "Does things");
        assert!(tool.get("title").is_none(), "title should be stripped");
        assert!(
            tool.get("outputSchema").is_none(),
            "outputSchema should be stripped"
        );
    }

    #[tokio::test]
    async fn test_resources_read_strips_name_and_title() {
        let adapter = V20250618ToV20241105Adapter::new();

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "contents": [{
                    "uri": "file:///test.txt",
                    "name": "test.txt",
                    "title": "Test File",
                    "mimeType": "text/plain",
                    "text": "content"
                }]
            }
        });

        let translated = adapter.translate_response(response).await.unwrap();

        let content = &translated["result"]["contents"][0];
        assert_eq!(content["uri"], "file:///test.txt");
        assert!(content.get("name").is_none(), "name should be stripped");
        assert!(content.get("title").is_none(), "title should be stripped");
    }

    #[tokio::test]
    async fn test_audio_content_converts_to_text() {
        let adapter = V20250618ToV20241105Adapter::new();

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{
                    "type": "audio",
                    "data": "base64audiodata",
                    "mimeType": "audio/mp3"
                }]
            }
        });

        let translated = adapter.translate_response(response).await.unwrap();

        let content = &translated["result"]["content"][0];
        assert_eq!(content["type"], "text");
        assert_eq!(content["text"], "[Audio content: audio/mp3]");
    }
}
