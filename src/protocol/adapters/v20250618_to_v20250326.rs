/// T045: V20250618 to V20250326 Adapter
///
/// Translates messages from 2025-06-18 format to 2025-03-26 format
/// Key differences: Must strip title fields and outputSchema from tools
use crate::protocol::{ProtocolAdapter, ProtocolError, ProtocolVersion};
use async_trait::async_trait;
use serde_json::Value;

pub struct V20250618ToV20250326Adapter;

impl V20250618ToV20250326Adapter {
    pub fn new() -> Self {
        Self
    }

    /// Translate tools/list response (strip title and outputSchema)
    fn translate_tools_list_response(&self, mut response: Value) -> Result<Value, ProtocolError> {
        if let Some(result) = response.get_mut("result") {
            if let Some(tools) = result.get_mut("tools").and_then(|t| t.as_array_mut()) {
                for tool in tools {
                    if let Some(tool_obj) = tool.as_object_mut() {
                        // Remove title field
                        if let Some(title) = tool_obj.remove("title") {
                            if !title.is_null() && !title.as_str().unwrap_or("").is_empty() {
                                tracing::warn!(
                                    tool_name = tool_obj
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown"),
                                    title = title.as_str().unwrap_or(""),
                                    "Stripping non-empty title during downgrade to V20250326"
                                );
                            }
                        }

                        // Remove outputSchema field
                        tool_obj.remove("outputSchema");
                    }
                }
            }
        }
        Ok(response)
    }

    /// Translate resources/list response (strip title)
    fn translate_resources_list_response(
        &self,
        mut response: Value,
    ) -> Result<Value, ProtocolError> {
        if let Some(result) = response.get_mut("result") {
            if let Some(resources) = result.get_mut("resources").and_then(|r| r.as_array_mut()) {
                for resource in resources {
                    if let Some(resource_obj) = resource.as_object_mut() {
                        resource_obj.remove("title");
                    }
                }
            }
        }
        Ok(response)
    }

    /// Translate resources/read response (strip name and title)
    fn translate_resources_read_response(
        &self,
        mut response: Value,
    ) -> Result<Value, ProtocolError> {
        if let Some(result) = response.get_mut("result") {
            if let Some(contents) = result.get_mut("contents").and_then(|c| c.as_array_mut()) {
                for content in contents {
                    if let Some(content_obj) = content.as_object_mut() {
                        // Remove name and title fields (not in V20250326)
                        content_obj.remove("name");
                        content_obj.remove("title");
                    }
                }
            }
        }
        Ok(response)
    }

    /// Translate tools/call response (strip structuredContent)
    fn translate_tools_call_response(&self, mut response: Value) -> Result<Value, ProtocolError> {
        if let Some(result) = response.get_mut("result") {
            // Remove structuredContent field (not in V20250326)
            if let Some(result_obj) = result.as_object_mut() {
                result_obj.remove("structuredContent");
            }
        }
        Ok(response)
    }
}

#[async_trait]
impl ProtocolAdapter for V20250618ToV20250326Adapter {
    fn source_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250618
    }

    fn target_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250326
    }

    async fn translate_request(&self, request: Value) -> Result<Value, ProtocolError> {
        // Requests are compatible - V20250326 won't send V20250618-specific fields
        Ok(request)
    }

    async fn translate_response(&self, response: Value) -> Result<Value, ProtocolError> {
        // Infer method from response structure
        if let Some(result) = response.get("result") {
            if result.get("tools").is_some() {
                return self.translate_tools_list_response(response);
            }
            if result.get("resources").is_some() {
                return self.translate_resources_list_response(response);
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
        // Notifications are compatible (resources/updated exists in both versions)
        Ok(notification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_adapter_versions() {
        let adapter = V20250618ToV20250326Adapter::new();
        assert_eq!(adapter.source_version(), ProtocolVersion::V20250618);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20250326);
    }

    #[tokio::test]
    async fn test_tools_list_strips_title_and_output_schema() {
        let adapter = V20250618ToV20250326Adapter::new();

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "my-tool",
                        "title": "My Tool",
                        "description": "Does things",
                        "inputSchema": {"type": "object"},
                        "outputSchema": {"type": "string"}
                    }
                ]
            }
        });

        let translated = adapter.translate_response(response).await.unwrap();

        let tool = &translated["result"]["tools"][0];
        assert_eq!(tool["name"], "my-tool");
        assert_eq!(tool["description"], "Does things");
        assert!(tool.get("title").is_none());
        assert!(tool.get("outputSchema").is_none());
    }

    #[tokio::test]
    async fn test_resources_read_strips_name_and_title() {
        let adapter = V20250618ToV20250326Adapter::new();

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
        assert!(content.get("name").is_none());
        assert!(content.get("title").is_none());
    }

    #[tokio::test]
    async fn test_tools_call_strips_structured_content() {
        let adapter = V20250618ToV20250326Adapter::new();

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{"type": "text", "text": "result"}],
                "structuredContent": {"key": "value"}
            }
        });

        let translated = adapter.translate_response(response).await.unwrap();

        assert!(translated["result"].get("content").is_some());
        assert!(translated["result"].get("structuredContent").is_none());
    }

    #[tokio::test]
    async fn test_request_pass_through() {
        let adapter = V20250618ToV20250326Adapter::new();

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": "my-tool", "arguments": {}}
        });

        let translated = adapter.translate_request(request.clone()).await.unwrap();
        assert_eq!(translated, request);
    }
}
