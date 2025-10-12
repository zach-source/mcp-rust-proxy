#[cfg(test)]
mod adapter_tests {
    use mcp_rust_proxy::protocol::{ProtocolAdapter, ProtocolVersion};
    use serde_json::json;

    // Mock adapter for testing trait behavior
    struct MockAdapter {
        source: ProtocolVersion,
        target: ProtocolVersion,
    }

    #[async_trait::async_trait]
    impl ProtocolAdapter for MockAdapter {
        fn source_version(&self) -> ProtocolVersion {
            self.source
        }

        fn target_version(&self) -> ProtocolVersion {
            self.target
        }

        async fn translate_request(
            &self,
            request: serde_json::Value,
        ) -> Result<serde_json::Value, mcp_rust_proxy::protocol::ProtocolError> {
            // Mock: just return the request unchanged
            Ok(request)
        }

        async fn translate_response(
            &self,
            response: serde_json::Value,
        ) -> Result<serde_json::Value, mcp_rust_proxy::protocol::ProtocolError> {
            // Mock: just return the response unchanged
            Ok(response)
        }

        async fn translate_notification(
            &self,
            notification: serde_json::Value,
        ) -> Result<serde_json::Value, mcp_rust_proxy::protocol::ProtocolError> {
            // Mock: just return the notification unchanged
            Ok(notification)
        }
    }

    #[tokio::test]
    async fn test_adapter_reports_source_and_target_versions() {
        let adapter = MockAdapter {
            source: ProtocolVersion::V20250618,
            target: ProtocolVersion::V20241105,
        };

        assert_eq!(adapter.source_version(), ProtocolVersion::V20250618);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20241105);
    }

    #[tokio::test]
    async fn test_translate_request_preserves_jsonrpc_structure() {
        let adapter = MockAdapter {
            source: ProtocolVersion::V20250618,
            target: ProtocolVersion::V20241105,
        };

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        let translated = adapter.translate_request(request.clone()).await.unwrap();

        assert_eq!(translated["jsonrpc"], "2.0");
        assert_eq!(translated["id"], 1);
        assert_eq!(translated["method"], "tools/list");
    }

    #[tokio::test]
    async fn test_translate_response_preserves_jsonrpc_structure() {
        let adapter = MockAdapter {
            source: ProtocolVersion::V20250618,
            target: ProtocolVersion::V20241105,
        };

        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": []
            }
        });

        let translated = adapter.translate_response(response.clone()).await.unwrap();

        assert_eq!(translated["jsonrpc"], "2.0");
        assert_eq!(translated["id"], 1);
        assert!(translated["result"].is_object());
    }

    #[tokio::test]
    async fn test_translate_notification_preserves_jsonrpc_structure() {
        let adapter = MockAdapter {
            source: ProtocolVersion::V20250618,
            target: ProtocolVersion::V20241105,
        };

        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
        });

        let translated = adapter
            .translate_notification(notification.clone())
            .await
            .unwrap();

        assert_eq!(translated["jsonrpc"], "2.0");
        assert_eq!(translated["method"], "notifications/initialized");
    }
}
