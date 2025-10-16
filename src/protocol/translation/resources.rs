/// T034: Resource translation helpers
///
/// Functions for converting ResourceContents between protocol versions
use crate::types::mcp::{v20241105::ResourceContentsV1, v20250618::ResourceContentsV2};

/// Generate a resource name from a URI
///
/// Priority:
/// 1. Last path component if URI is path-like (e.g., file:///path/to/doc.txt → "doc.txt")
/// 2. Full URI if not path-like (e.g., custom://resource-id → "custom://resource-id")
pub fn generate_resource_name(uri: &str) -> String {
    // Try parsing as URL
    if let Ok(parsed) = url::Url::parse(uri) {
        // Get last path segment
        if let Some(mut segments) = parsed.path_segments() {
            if let Some(last) = segments.next_back() {
                if !last.is_empty() {
                    return last.to_string();
                }
            }
        }
    }

    // Fallback: use full URI
    uri.to_string()
}

/// Convert ResourceContentsV1 to ResourceContentsV2
///
/// Generates the required 'name' field from the URI
pub fn resource_v1_to_v2(v1: ResourceContentsV1) -> ResourceContentsV2 {
    ResourceContentsV2 {
        name: generate_resource_name(&v1.uri),
        uri: v1.uri,
        title: None, // v1 doesn't have title
        mime_type: v1.mime_type,
        text: v1.text,
        blob: v1.blob,
    }
}

/// Convert ResourceContentsV2 to ResourceContentsV1
///
/// Strips the 'name' and 'title' fields (not in v1)
pub fn resource_v2_to_v1(v2: ResourceContentsV2) -> ResourceContentsV1 {
    ResourceContentsV1 {
        uri: v2.uri,
        mime_type: v2.mime_type,
        text: v2.text,
        blob: v2.blob,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_name_from_file_uri() {
        assert_eq!(
            generate_resource_name("file:///home/user/document.txt"),
            "document.txt"
        );
    }

    #[test]
    fn test_generate_name_from_http_url() {
        assert_eq!(
            generate_resource_name("https://example.com/api/resource.json"),
            "resource.json"
        );
    }

    #[test]
    fn test_generate_name_from_custom_scheme() {
        assert_eq!(
            generate_resource_name("custom://unique-id-12345"),
            "custom://unique-id-12345"
        );
    }

    #[test]
    fn test_generate_name_from_root_path() {
        assert_eq!(generate_resource_name("file:///"), "file:///");
    }

    #[test]
    fn test_v1_to_v2_conversion() {
        let v1 = ResourceContentsV1 {
            uri: "file:///test.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            text: Some("content".to_string()),
            blob: None,
        };

        let v2 = resource_v1_to_v2(v1.clone());

        assert_eq!(v2.uri, v1.uri);
        assert_eq!(v2.name, "test.txt");
        assert_eq!(v2.mime_type, v1.mime_type);
        assert_eq!(v2.text, v1.text);
        assert_eq!(v2.blob, v1.blob);
        assert_eq!(v2.title, None);
    }

    #[test]
    fn test_v2_to_v1_strips_name_and_title() {
        let v2 = ResourceContentsV2 {
            uri: "file:///doc.md".to_string(),
            name: "doc.md".to_string(),
            title: Some("Documentation".to_string()),
            mime_type: Some("text/markdown".to_string()),
            text: Some("# Title".to_string()),
            blob: None,
        };

        let v1 = resource_v2_to_v1(v2.clone());

        assert_eq!(v1.uri, v2.uri);
        assert_eq!(v1.mime_type, v2.mime_type);
        assert_eq!(v1.text, v2.text);
        assert_eq!(v1.blob, v2.blob);
        // name and title not present in v1
    }

    #[test]
    fn test_round_trip_preserves_v1_data() {
        let original = ResourceContentsV1 {
            uri: "file:///data.json".to_string(),
            mime_type: Some("application/json".to_string()),
            text: Some(r#"{"key": "value"}"#.to_string()),
            blob: None,
        };

        let v2 = resource_v1_to_v2(original.clone());
        let round_trip = resource_v2_to_v1(v2);

        assert_eq!(round_trip.uri, original.uri);
        assert_eq!(round_trip.mime_type, original.mime_type);
        assert_eq!(round_trip.text, original.text);
        assert_eq!(round_trip.blob, original.blob);
    }
}
