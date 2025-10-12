use async_trait::async_trait;
use serde_json::Value;

use crate::protocol::{ProtocolAdapter, ProtocolError, ProtocolVersion};

/// Pass-through adapter for when source and target versions are the same.
/// This is a zero-copy optimization that returns messages unchanged.
pub struct PassThroughAdapter {
    version: ProtocolVersion,
}

impl PassThroughAdapter {
    pub fn new(version: ProtocolVersion) -> Self {
        Self { version }
    }
}

#[async_trait]
impl ProtocolAdapter for PassThroughAdapter {
    fn source_version(&self) -> ProtocolVersion {
        self.version
    }

    fn target_version(&self) -> ProtocolVersion {
        self.version
    }

    async fn translate_request(&self, request: Value) -> Result<Value, ProtocolError> {
        // Zero-copy pass-through: no translation needed
        Ok(request)
    }

    async fn translate_response(&self, response: Value) -> Result<Value, ProtocolError> {
        // Zero-copy pass-through: no translation needed
        Ok(response)
    }

    async fn translate_notification(&self, notification: Value) -> Result<Value, ProtocolError> {
        // Zero-copy pass-through: no translation needed
        Ok(notification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_pass_through_preserves_data() {
        let adapter = PassThroughAdapter::new(ProtocolVersion::V20250326);

        let data = json!({"test": "data", "number": 42});
        let result = adapter.translate_request(data.clone()).await.unwrap();

        assert_eq!(result, data);
    }
}
