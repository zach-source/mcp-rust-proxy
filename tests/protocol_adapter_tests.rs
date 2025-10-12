use mcp_rust_proxy::protocol::{ProtocolAdapter, ProtocolError, ProtocolVersion};
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
    ) -> Result<serde_json::Value, ProtocolError> {
        // Mock: just return the request unchanged
        Ok(request)
    }

    async fn translate_response(
        &self,
        response: serde_json::Value,
    ) -> Result<serde_json::Value, ProtocolError> {
        // Mock: just return the response unchanged
        Ok(response)
    }

    async fn translate_notification(
        &self,
        notification: serde_json::Value,
    ) -> Result<serde_json::Value, ProtocolError> {
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

// PassThroughAdapter tests (T009)
mod pass_through_adapter_tests {
    use super::*;
    use mcp_rust_proxy::protocol::adapters::PassThroughAdapter;
    use std::time::Instant;

    #[tokio::test]
    async fn test_pass_through_request_unchanged() {
        let adapter = PassThroughAdapter::new(ProtocolVersion::V20250326);

        let request = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/list",
            "params": {}
        });

        let translated = adapter.translate_request(request.clone()).await.unwrap();

        // Should be identical
        assert_eq!(translated, request);
    }

    #[tokio::test]
    async fn test_pass_through_response_unchanged() {
        let adapter = PassThroughAdapter::new(ProtocolVersion::V20250326);

        let response = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "result": {
                "tools": [
                    {
                        "name": "test-tool",
                        "description": "A test tool",
                        "inputSchema": {"type": "object"}
                    }
                ]
            }
        });

        let translated = adapter.translate_response(response.clone()).await.unwrap();

        // Should be identical
        assert_eq!(translated, response);
    }

    #[tokio::test]
    async fn test_pass_through_notification_unchanged() {
        let adapter = PassThroughAdapter::new(ProtocolVersion::V20250326);

        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let translated = adapter
            .translate_notification(notification.clone())
            .await
            .unwrap();

        // Should be identical
        assert_eq!(translated, notification);
    }

    #[tokio::test]
    async fn test_pass_through_performance_benchmark() {
        let adapter = PassThroughAdapter::new(ProtocolVersion::V20250618);

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "test-tool",
                "arguments": {"input": "test"}
            }
        });

        // Warm up
        for _ in 0..10 {
            let _ = adapter.translate_request(request.clone()).await;
        }

        // Benchmark
        let iterations = 1000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = adapter.translate_request(request.clone()).await.unwrap();
        }

        let elapsed = start.elapsed();
        let avg_micros = elapsed.as_micros() / iterations;

        println!("PassThroughAdapter average overhead: {}μs", avg_micros);

        // Should be < 50μs per operation
        assert!(
            avg_micros < 50,
            "PassThroughAdapter overhead too high: {}μs (expected < 50μs)",
            avg_micros
        );
    }
}
