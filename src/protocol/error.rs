use std::time::Duration;
use thiserror::Error;

use super::version::ProtocolVersion;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Unsupported protocol version: {reported_version}. Supported: {supported_versions:?}")]
    UnsupportedVersion {
        reported_version: String,
        supported_versions: Vec<String>,
    },

    #[error(
        "Translation failed from {from_version:?} to {to_version:?} for {message_type}: {details}"
    )]
    TranslationError {
        from_version: ProtocolVersion,
        to_version: ProtocolVersion,
        message_type: String,
        details: String,
    },

    #[error("Missing required field '{field_name}' in {message_type} for {version:?}")]
    MissingRequiredField {
        field_name: String,
        message_type: String,
        version: ProtocolVersion,
    },

    #[error("Initialization timeout for server '{server_name}' after {duration:?}")]
    InitializationTimeout {
        server_name: String,
        duration: Duration,
    },

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
