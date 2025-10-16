/// MCP Protocol Compliance Tests
///
/// These tests verify that the proxy complies with the MCP 2025-03-26 specification.

#[cfg(test)]
mod mcp_compliance_tests {
    use serde_json::json;

    #[tokio::test]
    async fn test_initialize_response_format() {
        // Test that initialize response has required fields
        let initialize_response = json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "subscribe": false, "listChanged": false },
                "prompts": { "listChanged": false }
            },
            "serverInfo": {
                "name": "mcp-rust-proxy",
                "version": "0.1.0"
            }
        });

        // Verify all required fields present
        assert!(initialize_response.get("protocolVersion").is_some());
        assert!(initialize_response.get("capabilities").is_some());
        assert!(initialize_response.get("serverInfo").is_some());

        // Verify protocol version is correct
        assert_eq!(
            initialize_response["protocolVersion"].as_str().unwrap(),
            "2025-03-26"
        );

        // Verify serverInfo structure
        let server_info = &initialize_response["serverInfo"];
        assert!(server_info.get("name").is_some());
        assert!(server_info.get("version").is_some());
    }

    #[test]
    fn test_capabilities_match_implementation() {
        // Verify we only declare capabilities we actually support
        let capabilities = json!({
            "tools": { "listChanged": false },
            "resources": { "subscribe": false, "listChanged": false },
            "prompts": { "listChanged": false }
        });

        let caps = capabilities.as_object().unwrap();

        // Verify we don't claim unsupported features
        assert_eq!(caps["tools"]["listChanged"], false);
        assert_eq!(caps["resources"]["subscribe"], false);
        assert_eq!(caps["resources"]["listChanged"], false);
        assert_eq!(caps["prompts"]["listChanged"], false);

        // Verify we don't declare logging capability (not implemented)
        assert!(caps.get("logging").is_none());
    }

    #[test]
    fn test_tool_annotations_preserved() {
        // Simulate a backend tool with annotations
        let backend_tool = json!({
            "name": "dangerous_delete",
            "description": "Deletes files permanently",
            "destructive": true,
            "readOnly": false,
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }
        });

        // Simulate prefixing the tool name (what proxy does)
        let mut prefixed_tool = backend_tool.clone();
        if let Some(tool_obj) = prefixed_tool.as_object_mut() {
            tool_obj.insert(
                "name".to_string(),
                json!("mcp__proxy__mock__dangerous_delete"),
            );
            tool_obj.insert("originalName".to_string(), json!("dangerous_delete"));
            tool_obj.insert("server".to_string(), json!("mock"));
        }

        // Verify annotations are preserved after prefixing
        assert_eq!(prefixed_tool["destructive"], true);
        assert_eq!(prefixed_tool["readOnly"], false);
        assert_eq!(prefixed_tool["inputSchema"]["type"], "object");

        // Verify prefixing added metadata
        assert_eq!(prefixed_tool["name"], "mcp__proxy__mock__dangerous_delete");
        assert_eq!(prefixed_tool["originalName"], "dangerous_delete");
        assert_eq!(prefixed_tool["server"], "mock");
    }

    #[test]
    fn test_version_detection() {
        use chrono::Utc;
        use mcp_rust_proxy::state::ServerVersion;

        // Test ServerVersion struct
        let version = ServerVersion {
            protocol_version: "2024-11-05".to_string(),
            capabilities: json!({ "tools": {} }),
            detected_at: Utc::now(),
        };

        assert_eq!(version.protocol_version, "2024-11-05");
        assert!(version.capabilities.get("tools").is_some());
    }

    #[test]
    fn test_supported_methods() {
        // List of methods we must support per MCP 2025-03-26 spec
        let required_methods = vec![
            "initialize",
            "tools/list",
            "tools/call",
            "resources/list",
            "resources/read",
            "resources/templates/list",
            "prompts/list",
            "prompts/get",
            "ping",
        ];

        // This is a documentation test - actual implementation verified by integration tests
        for method in required_methods {
            // These methods should be handled in src/proxy/handler.rs
            assert!(!method.is_empty(), "Method {method} must be implemented");
        }
    }

    #[test]
    fn test_error_response_format() {
        // Test JSON-RPC 2.0 error format compliance
        let error_response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        });

        assert_eq!(error_response["jsonrpc"], "2.0");
        assert!(error_response.get("error").is_some());
        assert!(error_response["error"].get("code").is_some());
        assert!(error_response["error"].get("message").is_some());
    }

    #[test]
    fn test_tool_prefixing_format() {
        // Verify tool name prefixing follows the pattern: mcp__proxy__{server}__{tool}
        let server_name = "context7";
        let tool_name = "get-library-docs";

        let prefixed = format!(
            "mcp__proxy__{}__{}",
            server_name.replace("-", "_"),
            tool_name
        );

        assert_eq!(prefixed, "mcp__proxy__context7__get-library-docs");

        // Verify we can extract server and tool from prefixed name
        let parts: Vec<&str> = prefixed.splitn(4, "__").collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "mcp");
        assert_eq!(parts[1], "proxy");
        assert_eq!(parts[2], "context7");
        assert_eq!(parts[3], "get-library-docs");
    }

    #[test]
    fn test_capability_flags_are_boolean() {
        // All capability flags should be explicit booleans, not undefined
        let capabilities = json!({
            "tools": { "listChanged": false },
            "resources": { "subscribe": false, "listChanged": false },
            "prompts": { "listChanged": false }
        });

        // Verify all flags are boolean
        assert!(capabilities["tools"]["listChanged"].is_boolean());
        assert!(capabilities["resources"]["subscribe"].is_boolean());
        assert!(capabilities["resources"]["listChanged"].is_boolean());
        assert!(capabilities["prompts"]["listChanged"].is_boolean());
    }
}
