/// T045: V20250326 to V20250618 Adapter
///
/// Translates messages from 2025-03-26 format to 2025-06-18 format
/// Key differences: V20250618 requires ResourceContents.name field and adds title/outputSchema
use crate::protocol::{ProtocolAdapter, ProtocolError, ProtocolVersion};
use async_trait::async_trait;
use serde_json::Value;

pub struct V20250326ToV20250618Adapter;

impl V20250326ToV20250618Adapter {
    pub fn new() -> Self {
        Self
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
}

#[async_trait]
impl ProtocolAdapter for V20250326ToV20250618Adapter {
    fn source_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250326
    }

    fn target_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250618
    }

    async fn translate_request(&self, request: Value) -> Result<Value, ProtocolError> {
        // Requests are compatible between V20250326 and V20250618
        Ok(request)
    }

    async fn translate_response(&self, response: Value) -> Result<Value, ProtocolError> {
        // Check if this is a resources/read response (needs name field)
        if let Some(result) = response.get("result") {
            if result.get("contents").is_some() {
                return self.translate_resources_read_response(response);
            }
        }

        // Other responses are compatible (title and outputSchema are optional additions)
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
        let adapter = V20250326ToV20250618Adapter::new();
        assert_eq!(adapter.source_version(), ProtocolVersion::V20250326);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20250618);
    }

    #[tokio::test]
    async fn test_resources_read_adds_name() {
        let adapter = V20250326ToV20250618Adapter::new();

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
    async fn test_tools_list_response_compatible() {
        let adapter = V20250326ToV20250618Adapter::new();

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

        // Should pass through (title and outputSchema are optional in V20250618)
        assert_eq!(translated, response);
    }

    #[tokio::test]
    async fn test_request_pass_through() {
        let adapter = V20250326ToV20250618Adapter::new();

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

        // Requests pass through unchanged
        assert_eq!(translated, request);
    }
}
