mod pass_through;

pub use pass_through::PassThroughAdapter;

use crate::protocol::{ProtocolAdapter, ProtocolVersion};
use std::sync::Arc;

/// Factory function to create the appropriate adapter for a version pair
pub fn create_adapter(
    source_version: ProtocolVersion,
    target_version: ProtocolVersion,
) -> Arc<dyn ProtocolAdapter> {
    // If versions match, use pass-through adapter (zero overhead)
    if source_version == target_version {
        return Arc::new(PassThroughAdapter::new(source_version));
    }

    // TODO: Add translation adapters for different version pairs in later tasks
    // For now, use pass-through for all cases
    tracing::warn!(
        source = ?source_version,
        target = ?target_version,
        "No translation adapter available for version pair, using pass-through"
    );
    Arc::new(PassThroughAdapter::new(source_version))
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
}
