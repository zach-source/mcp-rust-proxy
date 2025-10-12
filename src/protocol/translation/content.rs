/// T038: Content type translation helpers
///
/// Functions for converting Content types between protocol versions
use crate::types::mcp::{v20241105::ContentV1, v20250326::ContentV2};

/// Convert ContentV1 to ContentV2
///
/// All v1 content types map directly to v2 (v2 is a superset)
pub fn content_v1_to_v2(v1: ContentV1) -> ContentV2 {
    match v1 {
        ContentV1::Text { text } => ContentV2::Text { text },
        ContentV1::Image { data, mime_type } => ContentV2::Image { data, mime_type },
        ContentV1::Resource { resource } => ContentV2::Resource { resource },
    }
}

/// Convert ContentV2 to ContentV1
///
/// AudioContent is converted to a text description (audio not supported in v1)
pub fn content_v2_to_v1(v2: ContentV2) -> ContentV1 {
    match v2 {
        ContentV2::Text { text } => ContentV1::Text { text },
        ContentV2::Image { data, mime_type } => ContentV1::Image { data, mime_type },
        ContentV2::Audio { mime_type, .. } => {
            // Convert audio to text description (audio not supported in v1)
            ContentV1::Text {
                text: format!("[Audio content: {}]", mime_type),
            }
        }
        ContentV2::Resource { resource } => {
            // ResourceContentsV1 is used in both v1 and v2 (2025-03-26)
            ContentV1::Resource { resource }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_content_bidirectional() {
        let v1 = ContentV1::Text {
            text: "Hello".to_string(),
        };

        let v2 = content_v1_to_v2(v1.clone());
        let round_trip = content_v2_to_v1(v2);

        match (v1, round_trip) {
            (ContentV1::Text { text: t1 }, ContentV1::Text { text: t2 }) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("Text content should round-trip"),
        }
    }

    #[test]
    fn test_image_content_bidirectional() {
        let v1 = ContentV1::Image {
            data: "imagedata".to_string(),
            mime_type: "image/jpeg".to_string(),
        };

        let v2 = content_v1_to_v2(v1.clone());
        let round_trip = content_v2_to_v1(v2);

        match (v1, round_trip) {
            (
                ContentV1::Image {
                    data: d1,
                    mime_type: m1,
                },
                ContentV1::Image {
                    data: d2,
                    mime_type: m2,
                },
            ) => {
                assert_eq!(d1, d2);
                assert_eq!(m1, m2);
            }
            _ => panic!("Image content should round-trip"),
        }
    }

    #[test]
    fn test_audio_content_converts_to_text() {
        let v2_audio = ContentV2::Audio {
            data: "audiodata".to_string(),
            mime_type: "audio/wav".to_string(),
        };

        let v1 = content_v2_to_v1(v2_audio);

        match v1 {
            ContentV1::Text { text } => {
                assert_eq!(text, "[Audio content: audio/wav]");
            }
            _ => panic!("Audio should convert to Text"),
        }
    }

    #[test]
    fn test_resource_content_translation() {
        use crate::types::mcp::v20241105::ResourceContentsV1;

        let resource_v1 = ResourceContentsV1 {
            uri: "file:///doc.pdf".to_string(),
            mime_type: Some("application/pdf".to_string()),
            text: None,
            blob: Some("base64data".to_string()),
        };

        let v1 = ContentV1::Resource {
            resource: resource_v1.clone(),
        };

        let v2 = content_v1_to_v2(v1.clone());

        // Verify resource passes through unchanged (v2 uses ResourceContentsV1)
        match v2 {
            ContentV2::Resource { resource } => {
                assert_eq!(resource.uri, "file:///doc.pdf");
                assert_eq!(resource.mime_type, Some("application/pdf".to_string()));
                assert_eq!(resource.blob, Some("base64data".to_string()));
            }
            _ => panic!("Expected Resource variant"),
        }
    }

    #[test]
    fn test_content_array_conversion() {
        let v1_array = vec![
            ContentV1::Text {
                text: "text".to_string(),
            },
            ContentV1::Image {
                data: "img".to_string(),
                mime_type: "image/png".to_string(),
            },
        ];

        let v2_array: Vec<ContentV2> = v1_array.into_iter().map(content_v1_to_v2).collect();

        assert_eq!(v2_array.len(), 2);
    }
}
