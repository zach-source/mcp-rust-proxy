use mcp_rust_proxy::protocol::ProtocolVersion;
use serde_json::json;

#[tokio::test]
async fn test_deserialize_valid_initialize_response_with_protocol_version() {
    let response_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "test-server",
                "version": "1.0.0"
            }
        }
    });

    // Parse the response
    let protocol_version = response_json["result"]["protocolVersion"]
        .as_str()
        .expect("protocolVersion field missing");

    let (version, is_supported) = ProtocolVersion::from_string(protocol_version);

    assert!(is_supported);
    assert_eq!(version, ProtocolVersion::V20250326);
}

#[tokio::test]
async fn test_extract_protocol_version_from_response() {
    let response_json = json!({
        "jsonrpc": "2.0",
        "id": "init-1",
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "serverInfo": {
                "name": "backend-server",
                "version": "0.1.0"
            }
        }
    });

    let protocol_version_str = response_json["result"]["protocolVersion"]
        .as_str()
        .expect("Missing protocolVersion");

    let (version, _) = ProtocolVersion::from_string(protocol_version_str);

    assert_eq!(version, ProtocolVersion::V20241105);
    assert_eq!(version.as_str(), "2024-11-05");
}

#[tokio::test]
async fn test_handle_missing_protocol_version_field() {
    let response_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "capabilities": {},
            "serverInfo": {
                "name": "test-server",
                "version": "1.0.0"
            }
        }
    });

    let protocol_version = response_json["result"]["protocolVersion"].as_str();

    assert!(
        protocol_version.is_none(),
        "Should handle missing protocolVersion field"
    );
}

#[tokio::test]
async fn test_handle_malformed_response() {
    let response_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32600,
            "message": "Invalid Request"
        }
    });

    // Should not have result field
    assert!(response_json["result"].is_null());
    assert!(response_json["error"].is_object());
}

#[tokio::test]
async fn test_serialize_initialized_notification() {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    assert_eq!(notification["jsonrpc"], "2.0");
    assert_eq!(notification["method"], "notifications/initialized");
    assert!(notification["id"].is_null());
    assert!(notification["params"].is_null());
}

#[tokio::test]
async fn test_all_supported_versions_parseable() {
    let versions = vec![
        ("2024-11-05", ProtocolVersion::V20241105),
        ("2025-03-26", ProtocolVersion::V20250326),
        ("2025-06-18", ProtocolVersion::V20250618),
    ];

    for (version_str, expected) in versions {
        let (parsed, is_supported) = ProtocolVersion::from_string(version_str);
        assert!(is_supported, "Version {} should be supported", version_str);
        assert_eq!(parsed, expected);
    }
}

#[tokio::test]
async fn test_unsupported_version_returns_default() {
    let (version, is_supported) = ProtocolVersion::from_string("2099-12-31");

    assert!(!is_supported);
    // Should return default version for pass-through
    assert_eq!(version, ProtocolVersion::V20250326);
}
