/// T040: V20241105 to V20250618 Adapter
///
/// Translates messages from 2024-11-05 format to 2025-06-18 format
use crate::protocol::{ProtocolAdapter, ProtocolError, ProtocolVersion};
use async_trait::async_trait;
use serde_json::Value;

pub struct V20241105ToV20250618Adapter;

impl V20241105ToV20250618Adapter {
    pub fn new() -> Self {
        Self
    }

    /// Translate tools/list response (v1 → v2 format)
    fn translate_tools_list_response(&self, mut response: Value) -> Result<Value, ProtocolError> {
        if let Some(result) = response.get_mut("result") {
            if let Some(tools) = result.get_mut("tools").and_then(|t| t.as_array_mut()) {
                for tool in tools {
                    // v1 tools stay as-is, but we can add empty optional fields
                    // title and outputSchema are optional in v2, so omitting them is fine
                }
            }
        }
        Ok(response)
    }

    /// Translate resources/read response (add required 'name' field)
    fn translate_resources_read_response(
        &self,
        mut response: Value,
    ) -> Result<Value, ProtocolError> {
        use crate::protocol::translation::resources::generate_resource_name;

        if let Some(result) = response.get_mut("result") {
            if let Some(contents) = result.get_mut("contents").and_then(|c| c.as_array_mut()) {
                for content in contents {
                    if let Some(content_obj) = content.as_object_mut() {
                        // Generate name from URI if not present
                        if content_obj.get("name").is_none() {
                            if let Some(uri) = content_obj.get("uri").and_then(|u| u.as_str()) {
                                let name = generate_resource_name(uri);
                                content_obj.insert("name".to_string(), Value::String(name));
                            }
                        }
                    }
                }
            }
        }
        Ok(response)
    }

    /// Translate tools/call response (no changes needed, formats are compatible)
    fn translate_tools_call_response(&self, response: Value) -> Result<Value, ProtocolError> {
        // v1 and v2 are compatible for tools/call responses
        // structuredContent is optional in v2, so omitting it is fine
        Ok(response)
    }
}

#[async_trait]
impl ProtocolAdapter for V20241105ToV20250618Adapter {
    fn source_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20241105
    }

    fn target_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250618
    }

    async fn translate_request(&self, request: Value) -> Result<Value, ProtocolError> {
        // Most requests don't need translation (only response formats differ)
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
        // Notifications are compatible between versions
        Ok(notification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_adapter_versions() {
        let adapter = V20241105ToV20250618Adapter::new();
        assert_eq!(adapter.source_version(), ProtocolVersion::V20241105);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20250618);
    }

    #[tokio::test]
    async fn test_tools_list_response_translation() {
        let adapter = V20241105ToV20250618Adapter::new();

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "my-tool",
                        "description": "Does things",
                        "inputSchema": {"type": "object"}
                    }
                ]
            }
        });

        let translated = adapter.translate_response(response.clone()).await.unwrap();

        // Should preserve all fields (v2 is superset of v1)
        assert_eq!(translated["result"]["tools"][0]["name"], "my-tool");
        assert_eq!(
            translated["result"]["tools"][0]["description"],
            "Does things"
        );
    }

    #[tokio::test]
    async fn test_resources_read_adds_name() {
        let adapter = V20241105ToV20250618Adapter::new();

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "contents": [{
                    "uri": "file:///home/user/document.txt",
                    "mimeType": "text/plain",
                    "text": "content"
                }]
            }
        });

        let translated = adapter.translate_response(response).await.unwrap();

        // Should add 'name' field generated from URI
        assert_eq!(translated["result"]["contents"][0]["name"], "document.txt");
        assert_eq!(
            translated["result"]["contents"][0]["uri"],
            "file:///home/user/document.txt"
        );
    }

    #[tokio::test]
    async fn test_request_pass_through() {
        let adapter = V20241105ToV20250618Adapter::new();

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "my-tool",
                "arguments": {}
            }
        });

        let translated = adapter.translate_request(request.clone()).await.unwrap();

        // Requests pass through unchanged (forward translation doesn't modify requests)
        assert_eq!(translated, request);
    }
}
