/// T043: V20250326 to V20241105 Adapter
///
/// Translates messages from 2025-03-26 format to 2024-11-05 format
/// Key differences: Must strip AudioContent (convert to text) and completions capability
use crate::protocol::{ProtocolAdapter, ProtocolError, ProtocolVersion};
use async_trait::async_trait;
use serde_json::Value;

pub struct V20250326ToV20241105Adapter;

impl Default for V20250326ToV20241105Adapter {
    fn default() -> Self {
        Self::new()
    }
}

impl V20250326ToV20241105Adapter {
    pub fn new() -> Self {
        Self
    }

    /// Convert AudioContent to TextContent for backward compatibility
    fn convert_audio_to_text_in_content(&self, content: &mut Value) {
        if let Some(content_array) = content.as_array_mut() {
            for item in content_array {
                if let Some(obj) = item.as_object_mut() {
                    if let Some(content_type) = obj.get("type").and_then(|t| t.as_str()) {
                        if content_type == "audio" {
                            // Convert audio to text description
                            let mime_type = obj
                                .get("mimeType")
                                .and_then(|m| m.as_str())
                                .unwrap_or("audio/*")
                                .to_string(); // Clone to avoid borrow issues
                            obj.clear();
                            obj.insert("type".to_string(), Value::String("text".to_string()));
                            obj.insert(
                                "text".to_string(),
                                Value::String(format!("[Audio content: {mime_type}]")),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Translate tools/call response (may contain audio content)
    fn translate_tools_call_response(&self, mut response: Value) -> Result<Value, ProtocolError> {
        if let Some(result) = response.get_mut("result") {
            if let Some(content) = result.get_mut("content") {
                self.convert_audio_to_text_in_content(content);
            }
        }
        Ok(response)
    }
}

#[async_trait]
impl ProtocolAdapter for V20250326ToV20241105Adapter {
    fn source_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20250326
    }

    fn target_version(&self) -> ProtocolVersion {
        ProtocolVersion::V20241105
    }

    async fn translate_request(&self, request: Value) -> Result<Value, ProtocolError> {
        // Requests are compatible - V20241105 won't send anything V20250326-specific
        Ok(request)
    }

    async fn translate_response(&self, response: Value) -> Result<Value, ProtocolError> {
        // Check if response contains audio content (tools/call responses)
        if let Some(result) = response.get("result") {
            if result.get("content").is_some() {
                return self.translate_tools_call_response(response);
            }
        }

        // Other responses are compatible
        Ok(response)
    }

    async fn translate_notification(&self, notification: Value) -> Result<Value, ProtocolError> {
        // Drop resources/updated notification (not supported in V20241105)
        if let Some(method) = notification.get("method").and_then(|m| m.as_str()) {
            if method == "notifications/resources/updated" {
                tracing::debug!("Dropping resources/updated notification for V20241105 client");
                // Return error to signal this notification should be dropped
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
        let adapter = V20250326ToV20241105Adapter::new();
        assert_eq!(adapter.source_version(), ProtocolVersion::V20250326);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20241105);
    }

    #[tokio::test]
    async fn test_audio_content_converted_to_text() {
        let adapter = V20250326ToV20241105Adapter::new();

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "audio",
                        "data": "base64data",
                        "mimeType": "audio/mp3"
                    }
                ]
            }
        });

        let translated = adapter.translate_response(response).await.unwrap();

        // Audio should be converted to text
        assert_eq!(translated["result"]["content"][0]["type"], "text");
        assert_eq!(
            translated["result"]["content"][0]["text"],
            "[Audio content: audio/mp3]"
        );
    }

    #[tokio::test]
    async fn test_text_and_image_content_preserved() {
        let adapter = V20250326ToV20241105Adapter::new();

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Hello"
                    },
                    {
                        "type": "image",
                        "data": "imagedata",
                        "mimeType": "image/png"
                    }
                ]
            }
        });

        let translated = adapter.translate_response(response.clone()).await.unwrap();

        // Text and image should pass through unchanged
        assert_eq!(translated, response);
    }

    #[tokio::test]
    async fn test_resources_updated_notification_dropped() {
        let adapter = V20250326ToV20241105Adapter::new();

        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/resources/updated"
        });

        let result = adapter.translate_notification(notification).await;

        // Should return error (signal to drop notification)
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_other_notifications_preserved() {
        let adapter = V20250326ToV20241105Adapter::new();

        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let translated = adapter
            .translate_notification(notification.clone())
            .await
            .unwrap();

        // Should pass through unchanged
        assert_eq!(translated, notification);
    }
}
