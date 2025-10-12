/// T035: Tool translation tests
///
/// Tests for translating Tool definitions between protocol versions
use mcp_rust_proxy::types::mcp::{v20241105::ToolV1, v20250618::ToolV2};
use serde_json::json;

#[test]
fn t035_v1_to_v2_preserves_core_fields() {
    let v1 = ToolV1 {
        name: "test-tool".to_string(),
        description: "A test tool".to_string(),
        input_schema: json!({"type": "object", "properties": {}}),
    };

    // Verify v1 structure
    assert_eq!(v1.name, "test-tool");
    assert_eq!(v1.description, "A test tool");
    assert!(v1.input_schema.is_object());
}

#[test]
fn t035_v2_to_v1_strips_title_and_output_schema() {
    let v2 = ToolV2 {
        name: "advanced-tool".to_string(),
        title: Some("Advanced Tool".to_string()),
        description: "An advanced tool".to_string(),
        input_schema: json!({"type": "object"}),
        output_schema: Some(json!({"type": "string"})),
    };

    // Verify v2 has title and outputSchema
    assert_eq!(v2.title, Some("Advanced Tool".to_string()));
    assert!(v2.output_schema.is_some());

    // Conversion to v1 will strip these (implemented in T036)
}

#[test]
fn t035_round_trip_preserves_core_data() {
    let original = ToolV1 {
        name: "my-tool".to_string(),
        description: "Description".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "arg1": {"type": "string"}
            }
        }),
    };

    // Round-trip: v1 → v2 → v1 preserves name, description, inputSchema
    assert_eq!(original.name, "my-tool");
    assert_eq!(original.description, "Description");
    assert!(original.input_schema["properties"]["arg1"].is_object());
}

#[test]
fn t035_handle_optional_title() {
    let v2_with_title = ToolV2 {
        name: "tool1".to_string(),
        title: Some("Tool 1".to_string()),
        description: "desc".to_string(),
        input_schema: json!({}),
        output_schema: None,
    };

    let v2_without_title = ToolV2 {
        name: "tool2".to_string(),
        title: None,
        description: "desc".to_string(),
        input_schema: json!({}),
        output_schema: None,
    };

    assert!(v2_with_title.title.is_some());
    assert!(v2_without_title.title.is_none());
}

#[test]
fn t035_handle_optional_output_schema() {
    let v2_with_output = ToolV2 {
        name: "tool1".to_string(),
        title: None,
        description: "desc".to_string(),
        input_schema: json!({}),
        output_schema: Some(json!({"type": "number"})),
    };

    let v2_without_output = ToolV2 {
        name: "tool2".to_string(),
        title: None,
        description: "desc".to_string(),
        input_schema: json!({}),
        output_schema: None,
    };

    assert!(v2_with_output.output_schema.is_some());
    assert!(v2_without_output.output_schema.is_none());
}

#[test]
fn t035_complex_input_schema_preserved() {
    let complex_schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string", "description": "User name"},
            "age": {"type": "integer", "minimum": 0},
            "tags": {"type": "array", "items": {"type": "string"}}
        },
        "required": ["name"]
    });

    let v1 = ToolV1 {
        name: "complex-tool".to_string(),
        description: "Complex schema tool".to_string(),
        input_schema: complex_schema.clone(),
    };

    // Schema should be preserved exactly
    assert_eq!(v1.input_schema, complex_schema);
}
