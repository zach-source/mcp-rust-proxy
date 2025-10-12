/// T036: Tool translation helpers
///
/// Functions for converting Tool definitions between protocol versions
use crate::types::mcp::{v20241105::ToolV1, v20250618::ToolV2};

/// Convert ToolV1 to ToolV2
///
/// Adds optional title and outputSchema fields (set to None for v1)
pub fn tool_v1_to_v2(v1: ToolV1) -> ToolV2 {
    ToolV2 {
        name: v1.name,
        title: None, // v1 doesn't have title
        description: v1.description,
        input_schema: v1.input_schema,
        output_schema: None, // v1 doesn't have outputSchema
    }
}

/// Convert ToolV2 to ToolV1
///
/// Strips title and outputSchema fields (not in v1)
pub fn tool_v2_to_v1(v2: ToolV2) -> ToolV1 {
    ToolV1 {
        name: v2.name,
        description: v2.description,
        input_schema: v2.input_schema,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_v1_to_v2_conversion() {
        let v1 = ToolV1 {
            name: "my-tool".to_string(),
            description: "Does things".to_string(),
            input_schema: json!({"type": "object"}),
        };

        let v2 = tool_v1_to_v2(v1.clone());

        assert_eq!(v2.name, v1.name);
        assert_eq!(v2.description, v1.description);
        assert_eq!(v2.input_schema, v1.input_schema);
        assert_eq!(v2.title, None);
        assert_eq!(v2.output_schema, None);
    }

    #[test]
    fn test_v2_to_v1_strips_extra_fields() {
        let v2 = ToolV2 {
            name: "advanced-tool".to_string(),
            title: Some("Advanced Tool".to_string()),
            description: "Advanced functionality".to_string(),
            input_schema: json!({"type": "object"}),
            output_schema: Some(json!({"type": "string"})),
        };

        let v1 = tool_v2_to_v1(v2.clone());

        assert_eq!(v1.name, v2.name);
        assert_eq!(v1.description, v2.description);
        assert_eq!(v1.input_schema, v2.input_schema);
        // title and outputSchema are stripped
    }

    #[test]
    fn test_round_trip_preserves_v1_data() {
        let original = ToolV1 {
            name: "round-trip-tool".to_string(),
            description: "Test round trip".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "param": {"type": "string"}
                }
            }),
        };

        let v2 = tool_v1_to_v2(original.clone());
        let round_trip = tool_v2_to_v1(v2);

        assert_eq!(round_trip.name, original.name);
        assert_eq!(round_trip.description, original.description);
        assert_eq!(round_trip.input_schema, original.input_schema);
    }

    #[test]
    fn test_v2_without_optional_fields() {
        let v2_minimal = ToolV2 {
            name: "minimal".to_string(),
            title: None,
            description: "Minimal tool".to_string(),
            input_schema: json!({}),
            output_schema: None,
        };

        let v1 = tool_v2_to_v1(v2_minimal);

        assert_eq!(v1.name, "minimal");
        assert_eq!(v1.description, "Minimal tool");
    }
}
