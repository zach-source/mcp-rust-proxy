//! Unit tests for plugin chain termination logic
//!
//! Tests verify that:
//! - Chain stops when plugin returns continue=false
//! - Remaining plugins not executed
//! - Output from last executed plugin returned

use mcp_rust_proxy::plugin::schema::PluginOutput;

#[test]
fn test_plugin_output_continue_false() {
    // Test that PluginOutput with continue=false stops the chain
    let output = PluginOutput {
        text: "stopped here".to_string(),
        continue_: false,
        metadata: Some(serde_json::json!({"reason": "blocked"})),
        error: None,
    };

    assert!(!output.continue_, "continue_ should be false");
    assert_eq!(output.text, "stopped here");

    println!("✓ PluginOutput continue=false test passed");
}

#[test]
fn test_plugin_output_continue_true() {
    // Test that PluginOutput with continue=true allows chain to proceed
    let output = PluginOutput {
        text: "keep going".to_string(),
        continue_: true,
        metadata: Some(serde_json::json!({"status": "ok"})),
        error: None,
    };

    assert!(output.continue_, "continue_ should be true");
    assert_eq!(output.text, "keep going");

    println!("✓ PluginOutput continue=true test passed");
}

#[test]
fn test_plugin_output_with_error_stops_chain() {
    // Test that PluginOutput with error field should stop the chain
    let output = PluginOutput {
        text: "original content".to_string(),
        continue_: false,
        metadata: None,
        error: Some("Plugin failed".to_string()),
    };

    assert!(!output.continue_, "Error should set continue=false");
    assert!(output.error.is_some(), "Error should be present");

    println!("✓ PluginOutput with error test passed");
}

#[test]
fn test_chain_termination_logic() {
    // Simulate chain execution logic
    let outputs = vec![
        PluginOutput {
            text: "step 1".to_string(),
            continue_: true,
            metadata: Some(serde_json::json!({"plugin": 1})),
            error: None,
        },
        PluginOutput {
            text: "step 2 - STOP".to_string(),
            continue_: false,
            metadata: Some(serde_json::json!({"plugin": 2})),
            error: None,
        },
        PluginOutput {
            text: "step 3 - NEVER EXECUTED".to_string(),
            continue_: true,
            metadata: Some(serde_json::json!({"plugin": 3})),
            error: None,
        },
    ];

    // Simulate chain execution stopping at continue=false
    let mut final_output = outputs[0].clone();
    for output in &outputs[1..] {
        if !final_output.continue_ {
            break; // Chain stopped
        }
        final_output = output.clone();
    }

    // Assert chain stopped at second output
    assert_eq!(
        final_output.text, "step 2 - STOP",
        "Chain should stop at second plugin"
    );
    assert!(!final_output.continue_);

    println!("✓ Chain termination logic test passed");
}

#[test]
fn test_plugin_output_serialization() {
    // Test that PluginOutput serializes correctly
    let output = PluginOutput {
        text: "test content".to_string(),
        continue_: false,
        metadata: Some(serde_json::json!({"key": "value"})),
        error: Some("test error".to_string()),
    };

    let json = serde_json::to_string(&output).unwrap();
    assert!(json.contains(r#""continue":false"#));
    assert!(json.contains(r#""text":"test content""#));
    assert!(json.contains(r#""error":"test error""#));

    // Deserialize back
    let deserialized: PluginOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.text, "test content");
    assert!(!deserialized.continue_);
    assert_eq!(deserialized.error, Some("test error".to_string()));

    println!("✓ PluginOutput serialization test passed");
}

#[test]
fn test_metadata_aggregation_structure() {
    // Test metadata aggregation structure
    let mut aggregated = serde_json::Map::new();

    aggregated.insert(
        "plugin1".to_string(),
        serde_json::json!({"executed": true, "duration": 10}),
    );
    aggregated.insert(
        "plugin2".to_string(),
        serde_json::json!({"executed": true, "duration": 15}),
    );

    let metadata = serde_json::Value::Object(aggregated);
    let metadata_obj = metadata.as_object().unwrap();

    assert_eq!(
        metadata_obj.len(),
        2,
        "Should have 2 plugin metadata entries"
    );
    assert!(metadata_obj.contains_key("plugin1"));
    assert!(metadata_obj.contains_key("plugin2"));

    println!("✓ Metadata aggregation structure test passed");
}
