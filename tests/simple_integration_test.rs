#[cfg(test)]
mod simple_tests {
    use mcp_rust_proxy::protocol::{mcp, JsonRpcId, JsonRpcMessage, JsonRpcV2Message};
    use serde_json::json;

    #[test]
    fn test_protocol_ping_request_creation() {
        let ping = mcp::create_ping_request(JsonRpcId::Number(1));
        let json_str = serde_json::to_string(&ping).unwrap();
        assert!(json_str.contains("ping"));
        assert!(json_str.contains("\"id\":1"));
    }

    #[test]
    fn test_protocol_ping_response_parsing() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {}
        });

        let response: JsonRpcMessage = serde_json::from_value(response_json).unwrap();

        match response {
            JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) => {
                assert!(matches!(resp.id, JsonRpcId::Number(1)));
                assert!(resp.result.is_some());
                assert!(resp.error.is_none());
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_protocol_cancellation_notification() {
        let cancel_json = json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {
                "requestId": 42,
                "reason": "User cancelled"
            }
        });

        let message: JsonRpcMessage = serde_json::from_value(cancel_json).unwrap();

        match message {
            JsonRpcMessage::V2(JsonRpcV2Message::Notification(notif)) => {
                assert_eq!(notif.method, "notifications/cancelled");
                let params = notif.params.unwrap();
                assert_eq!(params["requestId"], 42);
                assert_eq!(params["reason"], "User cancelled");
            }
            _ => panic!("Expected notification"),
        }
    }
}
