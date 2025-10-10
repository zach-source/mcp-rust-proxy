//! Error types for context tracing
//!
//! This module defines the error types used throughout the context tracing framework
//! and provides conversions for integration with the web layer.

use thiserror::Error;

/// Errors that can occur in context tracing operations
#[derive(Debug, Error)]
pub enum ContextError {
    /// Resource not found (response, context unit, etc.)
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid weight value (must be 0.0 to 1.0)
    #[error("Invalid weight: {0}")]
    InvalidWeight(f32),

    /// Invalid score value (must be -1.0 to 1.0)
    #[error("Invalid score: {0}")]
    InvalidScore(f32),

    /// Storage backend error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Validation error (constraints violated)
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Manifest size exceeds limit
    #[error("Manifest too large: {0} bytes (max {1} bytes)")]
    ManifestTooLarge(usize, usize),

    /// No context units in response
    #[error("Response must have at least one context unit")]
    NoContextUnits,

    /// Context tracking session not found
    #[error("Tracking session not found: {0}")]
    SessionNotFound(String),

    /// Version error (invalid version chain)
    #[error("Version error: {0}")]
    VersionError(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

// Conversions from storage errors
impl From<crate::context::storage::StorageError> for ContextError {
    fn from(err: crate::context::storage::StorageError) -> Self {
        ContextError::StorageError(err.to_string())
    }
}

// Conversions from database errors
impl From<rusqlite::Error> for ContextError {
    fn from(err: rusqlite::Error) -> Self {
        ContextError::StorageError(err.to_string())
    }
}

// Conversions from serialization errors
impl From<serde_json::Error> for ContextError {
    fn from(err: serde_json::Error) -> Self {
        ContextError::SerializationError(err.to_string())
    }
}

// Conversion to warp::Rejection for web layer integration
impl warp::reject::Reject for ContextError {}

/// Helper to convert ContextError to appropriate HTTP status
pub fn context_error_to_status(err: &ContextError) -> warp::http::StatusCode {
    use warp::http::StatusCode;

    match err {
        ContextError::NotFound(_) | ContextError::SessionNotFound(_) => StatusCode::NOT_FOUND,
        ContextError::InvalidWeight(_)
        | ContextError::InvalidScore(_)
        | ContextError::ValidationError(_)
        | ContextError::ManifestTooLarge(_, _)
        | ContextError::NoContextUnits
        | ContextError::VersionError(_) => StatusCode::BAD_REQUEST,
        ContextError::StorageError(_)
        | ContextError::SerializationError(_)
        | ContextError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        let err = ContextError::NotFound("resp_123".to_string());
        assert!(err.to_string().contains("Not found"));
        assert!(err.to_string().contains("resp_123"));

        let err = ContextError::InvalidWeight(1.5);
        assert!(err.to_string().contains("Invalid weight"));

        let err = ContextError::ManifestTooLarge(6000, 5120);
        assert!(err.to_string().contains("6000"));
        assert!(err.to_string().contains("5120"));
    }

    #[test]
    fn test_status_codes() {
        use warp::http::StatusCode;

        assert_eq!(
            context_error_to_status(&ContextError::NotFound("test".to_string())),
            StatusCode::NOT_FOUND
        );

        assert_eq!(
            context_error_to_status(&ContextError::InvalidWeight(1.5)),
            StatusCode::BAD_REQUEST
        );

        assert_eq!(
            context_error_to_status(&ContextError::StorageError("db error".to_string())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
