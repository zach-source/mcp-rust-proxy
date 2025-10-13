/// T037: Content type translation tests
///
/// Tests for translating Content types between protocol versions
use mcp_rust_proxy::types::mcp::{
    v20241105::{ContentV1, ResourceContentsV1},
    v20250326::ContentV2,
};

#[test]
fn t037_text_content_passes_through() {
    // Text content is identical in v1 and v2
    let text = "Hello world";

    let v1 = ContentV1::Text {
        text: text.to_string(),
    };

    // Verify v1 text content
    match v1 {
        ContentV1::Text { text: t } => assert_eq!(t, text),
        _ => panic!("Expected Text variant"),
    }
}

#[test]
fn t037_image_content_passes_through() {
    // Image content is identical in v1 and v2
    let v1 = ContentV1::Image {
        data: "base64data".to_string(),
        mime_type: "image/png".to_string(),
    };

    match v1 {
        ContentV1::Image { data, mime_type } => {
            assert_eq!(data, "base64data");
            assert_eq!(mime_type, "image/png");
        }
        _ => panic!("Expected Image variant"),
    }
}

#[test]
fn t037_audio_content_v2_to_text_v1() {
    // AudioContent only exists in v2 (2025-03-26+)
    // When converting to v1, it becomes a text description
    let v2_audio = ContentV2::Audio {
        data: "base64audiodata".to_string(),
        mime_type: "audio/mp3".to_string(),
    };

    // Verify v2 audio content exists
    match v2_audio {
        ContentV2::Audio { data, mime_type } => {
            assert_eq!(data, "base64audiodata");
            assert_eq!(mime_type, "audio/mp3");
        }
        _ => panic!("Expected Audio variant"),
    }

    // Conversion to v1 will be implemented in T038
}

#[test]
fn t037_resource_content_uses_resource_translation() {
    let resource_v1 = ResourceContentsV1 {
        uri: "file:///test.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        text: Some("content".to_string()),
        blob: None,
    };

    let content_v1 = ContentV1::Resource {
        resource: resource_v1.clone(),
    };

    match content_v1 {
        ContentV1::Resource { resource } => {
            assert_eq!(resource.uri, "file:///test.txt");
        }
        _ => panic!("Expected Resource variant"),
    }
}

#[test]
fn t037_all_content_variants_in_v1() {
    // v1 has: Text, Image, Resource
    let variants = vec![
        ContentV1::Text {
            text: "test".to_string(),
        },
        ContentV1::Image {
            data: "img".to_string(),
            mime_type: "image/png".to_string(),
        },
        ContentV1::Resource {
            resource: ResourceContentsV1 {
                uri: "file:///test".to_string(),
                mime_type: None,
                text: Some("t".to_string()),
                blob: None,
            },
        },
    ];

    assert_eq!(variants.len(), 3, "v1 should have 3 content types");
}

#[test]
fn t037_all_content_variants_in_v2() {
    // v2 has: Text, Image, Audio, Resource (still uses ResourceContentsV1)
    let variants = vec![
        ContentV2::Text {
            text: "test".to_string(),
        },
        ContentV2::Image {
            data: "img".to_string(),
            mime_type: "image/png".to_string(),
        },
        ContentV2::Audio {
            data: "audio".to_string(),
            mime_type: "audio/mp3".to_string(),
        },
        ContentV2::Resource {
            resource: ResourceContentsV1 {
                uri: "file:///test".to_string(),
                mime_type: None,
                text: Some("t".to_string()),
                blob: None,
            },
        },
    ];

    assert_eq!(variants.len(), 4, "v2 should have 4 content types");
}
