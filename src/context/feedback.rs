//! Quality feedback management for Claude API proxy
//!
//! This module handles user-submitted quality ratings for captured requests
//! and propagates feedback to all contributing context sources.

use crate::context::types::QualityFeedback;
use chrono::Utc;
use rusqlite::Connection;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum FeedbackError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),
}

impl From<rusqlite::Error> for FeedbackError {
    fn from(err: rusqlite::Error) -> Self {
        FeedbackError::DatabaseError(err.to_string())
    }
}

/// Manages quality feedback for captured requests
pub struct FeedbackManager {
    db_path: PathBuf,
}

impl FeedbackManager {
    /// Create a new feedback manager
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Submit quality feedback for a request
    pub async fn submit_feedback(&self, mut feedback: QualityFeedback) -> Result<String, FeedbackError> {
        // Validate
        feedback.validate().map_err(FeedbackError::ValidationError)?;

        // Generate ID if not provided
        if feedback.id.is_empty() {
            feedback.id = format!("fb_{}", Uuid::new_v4());
        }

        let db_path = self.db_path.clone();
        let feedback_clone = feedback.clone();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;

            // Check if feedback already exists for this request
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM quality_feedback WHERE request_id = ?1",
                [&feedback_clone.request_id],
                |row| row.get(0),
            )?;

            if exists {
                return Err(FeedbackError::Conflict(
                    "Feedback already exists for this request".to_string(),
                ));
            }

            // Verify request and response exist
            let request_exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM captured_requests WHERE id = ?1",
                [&feedback_clone.request_id],
                |row| row.get(0),
            )?;

            if !request_exists {
                return Err(FeedbackError::NotFound("Request not found".to_string()));
            }

            // Get response_id from correlation
            let response_id: Option<String> = conn
                .query_row(
                    "SELECT id FROM captured_responses WHERE correlation_id = (
                        SELECT correlation_id FROM captured_requests WHERE id = ?1
                    )",
                    [&feedback_clone.request_id],
                    |row| row.get(0),
                )
                .ok();

            let response_id = response_id.ok_or_else(|| FeedbackError::NotFound("Response not found".to_string()))?;

            // Insert feedback
            conn.execute(
                "INSERT INTO quality_feedback (id, request_id, response_id, rating, feedback_text, user_id, submitted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    &feedback_clone.id,
                    &feedback_clone.request_id,
                    &response_id,
                    feedback_clone.rating,
                    &feedback_clone.feedback_text,
                    &feedback_clone.user_id,
                    feedback_clone.submitted_at.timestamp(),
                ],
            )?;

            tracing::info!(
                feedback_id = %feedback_clone.id,
                request_id = %feedback_clone.request_id,
                rating = feedback_clone.rating,
                "Quality feedback submitted"
            );

            Ok(feedback_clone.id)
        })
        .await
        .map_err(|e| FeedbackError::DatabaseError(format!("Task join error: {}", e)))?
    }

    /// Update aggregate metrics for all context sources in a request
    pub async fn update_aggregate_metrics(&self, feedback: &QualityFeedback) -> Result<(), FeedbackError> {
        let db_path = self.db_path.clone();
        let request_id = feedback.request_id.clone();
        let rating = feedback.rating;

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;
            let tx = conn.unchecked_transaction()?;

            // Get all attributions for this request
            let mut stmt = tx.prepare(
                "SELECT source_name FROM context_attributions WHERE request_id = ?1 AND source_name IS NOT NULL",
            )?;

            let source_names: Vec<String> = stmt
                .query_map([&request_id], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;

            // Update metrics for each source
            for source_name in &source_names {
                // Get current metrics
                let current: Option<(i64, f64)> = tx
                    .query_row(
                        "SELECT feedback_count, average_rating FROM context_source_metrics WHERE source_name = ?1",
                        [source_name],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .ok();

                let (new_count, new_avg) = if let Some((count, avg)) = current {
                    // Update existing
                    let new_count = count + 1;
                    let new_avg = ((avg * count as f64) + rating) / new_count as f64;
                    (new_count, new_avg)
                } else {
                    // Create new entry
                    (1, rating)
                };

                // Upsert metrics
                tx.execute(
                    "INSERT INTO context_source_metrics (source_name, source_type, usage_count, feedback_count, average_rating, total_tokens, average_tokens, last_used, created_at)
                     VALUES (?1, 'McpServer', 0, ?2, ?3, 0, 0.0, ?4, ?4)
                     ON CONFLICT(source_name) DO UPDATE SET
                        feedback_count = ?2,
                        average_rating = ?3,
                        last_used = ?4",
                    rusqlite::params![
                        source_name,
                        new_count,
                        new_avg,
                        Utc::now().timestamp(),
                    ],
                )?;

                tracing::info!(
                    source_name = %source_name,
                    new_count = new_count,
                    new_avg = new_avg,
                    "Updated context source metrics"
                );
            }

            tx.commit()?;

            tracing::info!(
                request_id = %request_id,
                affected_sources = source_names.len(),
                "Feedback propagated to context sources"
            );

            Ok(())
        })
        .await
        .map_err(|e| FeedbackError::DatabaseError(format!("Task join error: {}", e)))?
    }

    /// Get feedback by request ID
    pub async fn get_feedback_by_request(&self, request_id: &str) -> Result<Option<QualityFeedback>, FeedbackError> {
        let db_path = self.db_path.clone();
        let request_id = request_id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;
            let mut stmt = conn.prepare(
                "SELECT id, request_id, response_id, rating, feedback_text, user_id, submitted_at
                 FROM quality_feedback WHERE request_id = ?1",
            )?;

            let feedback = stmt.query_row([&request_id], |row| {
                Ok(QualityFeedback {
                    id: row.get(0)?,
                    request_id: row.get(1)?,
                    response_id: row.get(2)?,
                    rating: row.get(3)?,
                    feedback_text: row.get(4)?,
                    user_id: row.get(5)?,
                    submitted_at: chrono::DateTime::from_timestamp(row.get(6)?, 0).unwrap_or_else(Utc::now),
                })
            });

            match feedback {
                Ok(fb) => Ok(Some(fb)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(FeedbackError::DatabaseError(e.to_string())),
            }
        })
        .await
        .map_err(|e| FeedbackError::DatabaseError(format!("Task join error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_submit_feedback() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        // Initialize schema
        {
            let conn = Connection::open(&db_path).expect("Failed to open db");
            crate::context::storage::initialize_schema(&conn).expect("Failed to init schema");

            // Insert test request and response
            conn.execute(
                "INSERT INTO captured_requests (id, timestamp, url, method, headers, body, body_json, total_tokens, correlation_id)
                 VALUES ('req_test', 0, '/v1/models', 'GET', '{}', '', '{}', 0, 'corr_test')",
                [],
            ).expect("Failed to insert request");

            conn.execute(
                "INSERT INTO captured_responses (id, correlation_id, timestamp, status_code, headers, body, body_json, latency_ms, proxy_latency_ms, response_tokens)
                 VALUES ('resp_test', 'corr_test', 0, 200, '{}', '', '{}', 100, 1, 0)",
                [],
            ).expect("Failed to insert response");
        }

        let manager = FeedbackManager::new(db_path);

        let feedback = QualityFeedback {
            id: String::new(),
            request_id: "req_test".to_string(),
            response_id: "resp_test".to_string(),
            rating: 0.8,
            feedback_text: Some("Great response".to_string()),
            user_id: "test-user".to_string(),
            submitted_at: Utc::now(),
        };

        let fb_id = manager.submit_feedback(feedback).await.expect("Failed to submit feedback");
        assert!(fb_id.starts_with("fb_"));
    }

    #[test]
    fn test_feedback_validation() {
        let feedback = QualityFeedback {
            id: "fb_test".to_string(),
            request_id: "req_test".to_string(),
            response_id: "resp_test".to_string(),
            rating: 0.5,
            feedback_text: None,
            user_id: "user".to_string(),
            submitted_at: Utc::now(),
        };

        assert!(feedback.validate().is_ok());

        let mut invalid = feedback.clone();
        invalid.rating = 1.5;
        assert!(invalid.validate().is_err());

        let mut invalid2 = feedback.clone();
        invalid2.request_id = String::new();
        assert!(invalid2.validate().is_err());
    }
}
