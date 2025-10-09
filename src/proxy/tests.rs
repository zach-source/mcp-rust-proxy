#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::protocol::{
        JsonRpcId, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcV2Message,
    };
    use serde_json::json;

    #[test]
    fn test_request_routing() {
        let router = RequestRouter::new();

        // Register some routes
        router.register_tool("tools/list".to_string(), "server1".to_string());
        router.register_resource("resources/list".to_string(), "server2".to_string());

        // Test tool routing
        assert_eq!(
            router.get_server_for_tool("tools/list"),
            Some("server1".to_string())
        );
        assert_eq!(router.get_server_for_tool("tools/other"), None);

        // Test resource routing
        assert_eq!(
            router.get_server_for_resource("resources/list"),
            Some("server2".to_string())
        );
        assert_eq!(router.get_server_for_resource("resources/other"), None);
    }

    #[test]
    fn test_request_parsing() {
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        let request: JsonRpcMessage = serde_json::from_value(request_json).unwrap();

        if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = request {
            assert_eq!(req.method, "tools/list");
            assert!(matches!(req.id, JsonRpcId::Number(1)));
        } else {
            panic!("Expected request message");
        }
    }

    #[test]
    fn test_cancellation_notification_parsing() {
        let cancel_json = json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {
                "requestId": 42,
                "reason": "User cancelled"
            }
        });

        let message: JsonRpcMessage = serde_json::from_value(cancel_json).unwrap();

        if let JsonRpcMessage::V2(JsonRpcV2Message::Notification(notif)) = message {
            assert_eq!(notif.method, "notifications/cancelled");
            assert!(notif.params.is_some());

            let params = notif.params.unwrap();
            assert_eq!(params["requestId"], 42);
            assert_eq!(params["reason"], "User cancelled");
        } else {
            panic!("Expected notification message");
        }
    }
}
