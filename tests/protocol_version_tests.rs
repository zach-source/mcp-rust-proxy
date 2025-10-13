use mcp_rust_proxy::protocol::ProtocolVersion;

#[test]
fn test_parse_valid_version_2024_11_05() {
    let (version, supported) = ProtocolVersion::from_string("2024-11-05");
    assert_eq!(version, ProtocolVersion::V20241105);
    assert!(supported, "2024-11-05 should be supported");
}

#[test]
fn test_parse_valid_version_2025_03_26() {
    let (version, supported) = ProtocolVersion::from_string("2025-03-26");
    assert_eq!(version, ProtocolVersion::V20250326);
    assert!(supported, "2025-03-26 should be supported");
}

#[test]
fn test_parse_valid_version_2025_06_18() {
    let (version, supported) = ProtocolVersion::from_string("2025-06-18");
    assert_eq!(version, ProtocolVersion::V20250618);
    assert!(supported, "2025-06-18 should be supported");
}

#[test]
fn test_parse_unsupported_version_future() {
    let (version, supported) = ProtocolVersion::from_string("2026-01-01");
    assert_eq!(
        version,
        ProtocolVersion::V20250326,
        "Should default to V20250326"
    );
    assert!(!supported, "2026-01-01 should not be supported");
}

#[test]
fn test_parse_unsupported_version_invalid() {
    let (version, supported) = ProtocolVersion::from_string("invalid-version");
    assert_eq!(version, ProtocolVersion::V20250326);
    assert!(!supported);
}

#[test]
fn test_round_trip_conversion() {
    let versions = vec!["2024-11-05", "2025-03-26", "2025-06-18"];

    for version_str in versions {
        let (version, supported) = ProtocolVersion::from_string(version_str);
        assert!(supported, "Version {} should be supported", version_str);
        assert_eq!(
            version.as_str(),
            version_str,
            "Round-trip failed for {}",
            version_str
        );
    }
}

#[test]
fn test_supports_audio_content() {
    assert!(!ProtocolVersion::V20241105.supports_audio_content());
    assert!(ProtocolVersion::V20250326.supports_audio_content());
    assert!(ProtocolVersion::V20250618.supports_audio_content());
}

#[test]
fn test_supports_completions() {
    assert!(!ProtocolVersion::V20241105.supports_completions());
    assert!(ProtocolVersion::V20250326.supports_completions());
    assert!(ProtocolVersion::V20250618.supports_completions());
}

#[test]
fn test_requires_resource_name() {
    assert!(!ProtocolVersion::V20241105.requires_resource_name());
    assert!(!ProtocolVersion::V20250326.requires_resource_name());
    assert!(ProtocolVersion::V20250618.requires_resource_name());
}

#[test]
fn test_supports_structured_content() {
    assert!(!ProtocolVersion::V20241105.supports_structured_content());
    assert!(!ProtocolVersion::V20250326.supports_structured_content());
    assert!(ProtocolVersion::V20250618.supports_structured_content());
}

#[test]
fn test_supports_elicitation() {
    assert!(!ProtocolVersion::V20241105.supports_elicitation());
    assert!(!ProtocolVersion::V20250326.supports_elicitation());
    assert!(ProtocolVersion::V20250618.supports_elicitation());
}

#[test]
fn test_supports_title_fields() {
    assert!(!ProtocolVersion::V20241105.supports_title_fields());
    assert!(!ProtocolVersion::V20250326.supports_title_fields());
    assert!(ProtocolVersion::V20250618.supports_title_fields());
}

#[test]
fn test_supports_output_schema() {
    assert!(!ProtocolVersion::V20241105.supports_output_schema());
    assert!(!ProtocolVersion::V20250326.supports_output_schema());
    assert!(ProtocolVersion::V20250618.supports_output_schema());
}
