/// T043: V20241105 to V20250326 Adapter
///
/// Translates messages from 2024-11-05 format to 2025-03-26 format
/// Key differences: V20250326 adds AudioContent and completions capability
use crate::protocol::{ProtocolAdapter, ProtocolError, ProtocolVersion};
use async_trait::async_trait;
use serde_json::Value;

pub struct V20241105ToV20250326Adapter;

impl V20241105ToV20250326Adapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProtocolAdapter for V20241105ToV20250326Adapter {
    fn source_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20241105
    }

    fn target_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250326
    }

    async fn translate_request(&self, request: Value) -> Result<Value, ProtocolError> {
        // Requests are compatible between V20241105 and V20250326
        // V20250326 adds completions capability, but that's in responses not requests
        Ok(request)
    }

    async fn translate_response(&self, response: Value) -> Result<Value, ProtocolError> {
        // Responses are compatible - V20250326 is a superset of V20241105
        // V20241105 responses work fine in V20250326 context
        // AudioContent and completions capability are optional additions
        Ok(response)
    }

    async fn translate_notification(&self, notification: Value) -> Result<Value, ProtocolError> {
        // Notifications are compatible between versions
        // V20250326 adds resources/updated notification, but V20241105 won't send it
        Ok(notification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_adapter_versions() {
        let adapter = V20241105ToV20250326Adapter::new();
        assert_eq!(adapter.source_version(), ProtocolVersion::V20241105);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20250326);
    }

    #[tokio::test]
    async fn test_tools_list_response_compatible() {
        let adapter = V20241105ToV20250326Adapter::new();

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

        // Should pass through unchanged - formats are compatible
        assert_eq!(translated, response);
    }

    #[tokio::test]
    async fn test_resources_read_response_compatible() {
        let adapter = V20241105ToV20250326Adapter::new();

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

        let translated = adapter.translate_response(response.clone()).await.unwrap();

        // Should pass through unchanged - name field not required in V20250326
        assert_eq!(translated, response);
    }

    #[tokio::test]
    async fn test_request_pass_through() {
        let adapter = V20241105ToV20250326Adapter::new();

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
