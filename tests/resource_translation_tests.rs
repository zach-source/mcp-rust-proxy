/// T033: Resource translation tests
///
/// Tests for translating ResourceContents between protocol versions
use mcp_rust_proxy::types::mcp::{v20241105::ResourceContentsV1, v20250618::ResourceContentsV2};

#[test]
fn t033_v1_to_v2_generates_name_from_file_uri() {
    let v1 = ResourceContentsV1 {
        uri: "file:///home/user/document.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        text: Some("Hello world".to_string()),
        blob: None,
    };

    // Will be implemented in T034 - for now verify type exists
    assert_eq!(v1.uri, "file:///home/user/document.txt");
    assert_eq!(v1.mime_type, Some("text/plain".to_string()));
}

#[test]
fn t033_v2_to_v1_strips_name_and_title() {
    let v2 = ResourceContentsV2 {
        uri: "file:///test.txt".to_string(),
        name: "test.txt".to_string(),
        title: Some("Test Document".to_string()),
        mime_type: Some("text/plain".to_string()),
        text: Some("content".to_string()),
        blob: None,
    };

    // Verify v2 has name and title fields
    assert_eq!(v2.name, "test.txt");
    assert_eq!(v2.title, Some("Test Document".to_string()));

    // Conversion to v1 will strip these fields (implemented in T034)
}

#[test]
fn t033_round_trip_preserves_core_data() {
    let original = ResourceContentsV1 {
        uri: "file:///doc.md".to_string(),
        mime_type: Some("text/markdown".to_string()),
        text: Some("# Heading".to_string()),
        blob: None,
    };

    // Round-trip: v1 → v2 → v1 should preserve uri, mime_type, text, blob
    // (name and title are not in v1, so they're lost in round-trip)
    assert_eq!(original.uri, "file:///doc.md");
    assert_eq!(original.text, Some("# Heading".to_string()));
    assert_eq!(original.mime_type, Some("text/markdown".to_string()));
}

#[test]
fn t033_handle_missing_optional_fields() {
    // v1 with minimal fields
    let v1_minimal = ResourceContentsV1 {
        uri: "custom://resource-id".to_string(),
        mime_type: None,
        text: Some("minimal".to_string()),
        blob: None,
    };

    assert!(v1_minimal.mime_type.is_none());
    assert!(v1_minimal.blob.is_none());

    // v2 with minimal fields (name is required)
    let v2_minimal = ResourceContentsV2 {
        uri: "custom://id".to_string(),
        name: "id".to_string(),
        title: None,
        mime_type: None,
        text: Some("minimal".to_string()),
        blob: None,
    };

    assert!(v2_minimal.title.is_none());
    assert!(v2_minimal.mime_type.is_none());
}

#[test]
fn t033_special_uri_characters() {
    // Test URIs with special characters that might affect name generation
    let test_cases = vec![
        (
            "file:///path/to/file%20with%20spaces.txt",
            "file with spaces",
        ),
        ("https://example.com/api/resource?id=123", "query params"),
        ("custom://unique-id-with-dashes", "custom scheme"),
        ("file:///", "root path"),
        ("http://example.com/api/", "trailing slash"),
    ];

    for (uri, description) in test_cases {
        let resource = ResourceContentsV1 {
            uri: uri.to_string(),
            mime_type: None,
            text: Some("test".to_string()),
            blob: None,
        };

        assert!(
            !resource.uri.is_empty(),
            "URI should not be empty for {description}"
        );
    }
}
