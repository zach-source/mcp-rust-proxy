#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;
    
    #[test]
    fn test_ping_request_serialization() {
        let ping_request = mcp::create_ping_request(JsonRpcId::Number(1));
        let json = serde_json::to_value(&ping_request).unwrap();
        
        assert_eq!(json["jsonrpc"], "2.0");
        if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = ping_request {
            assert_eq!(req.method, "ping");
            assert_eq!(req.params, Some(json!({})));
            match req.id {
                JsonRpcId::Number(n) => assert_eq!(n, 1),
                _ => panic!("Expected number ID"),
            }
        } else {
            panic!("Expected request message");
        }
    }
    
    #[test]
    fn test_ping_response_deserialization() {
        let response_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {}
        });
        
        let response: JsonRpcMessage = serde_json::from_value(response_json).unwrap();
        
        if let JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) = response {
            match resp.id {
                JsonRpcId::Number(n) => assert_eq!(n, 1),
                _ => panic!("Expected number ID"),
            }
            assert!(resp.result.is_some());
            assert!(resp.error.is_none());
        } else {
            panic!("Expected response message");
        }
    }
    
    #[test]
    fn test_error_response_deserialization() {
        let error_json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        });
        
        let response: JsonRpcMessage = serde_json::from_value(error_json).unwrap();
        
        if let JsonRpcMessage::V2(JsonRpcV2Message::Response(resp)) = response {
            assert!(resp.result.is_none());
            assert!(resp.error.is_some());
            
            let error = resp.error.unwrap();
            assert_eq!(error.code, -32601);
            assert_eq!(error.message, "Method not found");
        } else {
            panic!("Expected response message");
        }
    }
    
    #[test]
    fn test_notification_deserialization() {
        let notification_json = json!({
            "jsonrpc": "2.0",
            "method": "progress",
            "params": {
                "token": "abc123",
                "value": 50
            }
        });
        
        let message: JsonRpcMessage = serde_json::from_value(notification_json).unwrap();
        
        if let JsonRpcMessage::V2(JsonRpcV2Message::Notification(notif)) = message {
            assert_eq!(notif.method, "progress");
            assert!(notif.params.is_some());
            
            let params = notif.params.unwrap();
            assert_eq!(params["token"], "abc123");
            assert_eq!(params["value"], 50);
        } else {
            panic!("Expected notification message");
        }
    }
    
    #[test]
    fn test_string_id_support() {
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": "custom-id-123",
            "method": "test",
            "params": {}
        });
        
        let request: JsonRpcMessage = serde_json::from_value(request_json).unwrap();
        
        if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = request {
            match req.id {
                JsonRpcId::String(s) => assert_eq!(s, "custom-id-123"),
                _ => panic!("Expected string ID"),
            }
        } else {
            panic!("Expected request message");
        }
    }
    
    #[test]
    fn test_mcp_cancel_notification() {
        // Test MCP cancellation notification format
        let cancel_json = json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {
                "requestId": 42,
                "reason": "User cancelled operation"
            }
        });
        
        let message: JsonRpcMessage = serde_json::from_value(cancel_json).unwrap();
        
        if let JsonRpcMessage::V2(JsonRpcV2Message::Notification(notif)) = message {
            assert_eq!(notif.method, "notifications/cancelled");
            let params = notif.params.unwrap();
            assert_eq!(params["requestId"], 42);
            assert_eq!(params["reason"], "User cancelled operation");
        } else {
            panic!("Expected notification message");
        }
    }
}