mod pass_through;
mod v20241105_to_v20250326;
mod v20241105_to_v20250618;
mod v20250326_to_v20241105;
mod v20250326_to_v20250618;
mod v20250618_to_v20241105;
mod v20250618_to_v20250326;

pub use pass_through::PassThroughAdapter;
pub use v20241105_to_v20250326::V20241105ToV20250326Adapter;
pub use v20241105_to_v20250618::V20241105ToV20250618Adapter;
pub use v20250326_to_v20241105::V20250326ToV20241105Adapter;
pub use v20250326_to_v20250618::V20250326ToV20250618Adapter;
pub use v20250618_to_v20241105::V20250618ToV20241105Adapter;
pub use v20250618_to_v20250326::V20250618ToV20250326Adapter;

use crate::protocol::{ProtocolAdapter, ProtocolVersion};
use std::sync::Arc;

/// Factory function to create the appropriate adapter for a version pair
pub fn create_adapter(
    source_version: ProtocolVersion,
    target_version: ProtocolVersion,
) -> Arc<dyn ProtocolAdapter> {
    use ProtocolVersion::*;

    // If versions match, use pass-through adapter (zero overhead)
    if source_version == target_version {
        return Arc::new(PassThroughAdapter::new(source_version));
    }

    // Select appropriate translation adapter based on version pair
    match (source_version, target_version) {
        // V20241105 → other versions
        (V20241105, V20250326) => Arc::new(V20241105ToV20250326Adapter::new()),
        (V20241105, V20250618) => Arc::new(V20241105ToV20250618Adapter::new()),

        // V20250326 → other versions
        (V20250326, V20241105) => Arc::new(V20250326ToV20241105Adapter::new()),
        (V20250326, V20250618) => Arc::new(V20250326ToV20250618Adapter::new()),

        // V20250618 → other versions
        (V20250618, V20241105) => Arc::new(V20250618ToV20241105Adapter::new()),
        (V20250618, V20250326) => Arc::new(V20250618ToV20250326Adapter::new()),

        // Same version cases are handled by the if statement above
        _ => unreachable!("Same-version pairs should have been handled by pass-through check"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_version_returns_pass_through() {
        let adapter = create_adapter(ProtocolVersion::V20250326, ProtocolVersion::V20250326);

        assert_eq!(adapter.source_version(), ProtocolVersion::V20250326);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20250326);
    }

    #[test]
    fn test_all_versions_supported() {
        let versions = vec![
            ProtocolVersion::V20241105,
            ProtocolVersion::V20250326,
            ProtocolVersion::V20250618,
        ];

        for version in versions {
            let adapter = create_adapter(version, version);
            assert_eq!(adapter.source_version(), version);
            assert_eq!(adapter.target_version(), version);
        }
    }

    #[test]
    fn test_all_version_pairs_have_adapters() {
        // T046: Test that all 9 version pair combinations (3x3) have adapters
        let versions = vec![
            ProtocolVersion::V20241105,
            ProtocolVersion::V20250326,
            ProtocolVersion::V20250618,
        ];

        for source in &versions {
            for target in &versions {
                let adapter = create_adapter(*source, *target);
                assert_eq!(adapter.source_version(), *source);
                assert_eq!(adapter.target_version(), *target);
            }
        }
    }

    #[test]
    fn test_translation_adapters_used_for_different_versions() {
        // Test a few specific translation adapter selections
        let adapter = create_adapter(ProtocolVersion::V20241105, ProtocolVersion::V20250618);
        assert_eq!(adapter.source_version(), ProtocolVersion::V20241105);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20250618);

        let adapter = create_adapter(ProtocolVersion::V20250618, ProtocolVersion::V20241105);
        assert_eq!(adapter.source_version(), ProtocolVersion::V20250618);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20241105);

        let adapter = create_adapter(ProtocolVersion::V20250326, ProtocolVersion::V20250618);
        assert_eq!(adapter.source_version(), ProtocolVersion::V20250326);
        assert_eq!(adapter.target_version(), ProtocolVersion::V20250618);
    }
}
