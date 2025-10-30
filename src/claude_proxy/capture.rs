//! Request/response capture and storage for Claude API proxy
//!
//! This module handles capturing complete request/response data and storing it
//! persistently for later review and analysis.

use crate::claude_proxy::attribution::AttributionEngine;
use crate::context::types::{CapturedRequest, CapturedResponse, ContextAttribution};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid data: {0}")]
    ValidationError(String),
}

impl From<rusqlite::Error> for CaptureError {
    fn from(err: rusqlite::Error) -> Self {
        CaptureError::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for CaptureError {
    fn from(err: serde_json::Error) -> Self {
        CaptureError::SerializationError(err.to_string())
    }
}

/// Query filters for captured requests
#[derive(Debug, Clone, Default)]
pub struct QueryFilters {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub context_source: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Storage for captured Claude API requests and responses
pub struct CaptureStorage {
    /// Database connection
    db_path: std::path::PathBuf,

    /// Cache of recent requests (last 100)
    request_cache: Arc<DashMap<String, CapturedRequest>>,

    /// Cache of recent responses (last 100)
    response_cache: Arc<DashMap<String, CapturedResponse>>,
}

impl CaptureStorage {
    /// Create a new capture storage instance
    pub fn new(db_path: std::path::PathBuf) -> Result<Self, CaptureError> {
        // Initialize database schema
        {
            let conn = Connection::open(&db_path)?;
            crate::context::storage::initialize_schema(&conn)
                .map_err(|e| CaptureError::DatabaseError(format!("Schema init failed: {}", e)))?;
        }

        Ok(Self {
            db_path,
            request_cache: Arc::new(DashMap::new()),
            response_cache: Arc::new(DashMap::new()),
        })
    }

    /// Get a database connection
    fn get_connection(&self) -> Result<Connection, CaptureError> {
        let conn = Connection::open(&self.db_path)?;
        Ok(conn)
    }

    /// Capture a request and store it with context attributions
    pub async fn capture_request(
        &self,
        url: String,
        method: String,
        mut headers: HashMap<String, String>,
        body: Vec<u8>,
    ) -> Result<String, CaptureError> {
        let request_id = format!("req_{}", Uuid::new_v4());
        let correlation_id = format!("corr_{}", Uuid::new_v4());

        // Sanitize sensitive headers
        Self::sanitize_headers(&mut headers);

        // Parse body as JSON (handle empty bodies for GET requests)
        let body_json: serde_json::Value = if body.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_slice(&body).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to parse request body as JSON, using empty object");
                serde_json::json!({})
            })
        };

        // Generate context attributions
        let mut attributions = AttributionEngine::analyze_request(&body_json);

        // Set request_id for all attributions
        for attr in &mut attributions {
            attr.request_id = request_id.clone();
        }

        // Calculate total tokens from attributions
        let total_tokens: usize = attributions.iter().map(|a| a.token_count).sum();

        let request = CapturedRequest {
            id: request_id.clone(),
            timestamp: Utc::now(),
            url,
            method,
            headers,
            body,
            body_json,
            total_tokens,
            correlation_id: correlation_id.clone(),
        };

        // Store in cache
        self.request_cache
            .insert(request_id.clone(), request.clone());

        // Store in database with attributions (async)
        let db_path = self.db_path.clone();
        let request_clone = request.clone();
        let attributions_clone = attributions.clone();
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;
            let tx = conn.unchecked_transaction()?;

            // Insert request
            let headers_json = serde_json::to_string(&request_clone.headers)?;
            let body_json_str = serde_json::to_string(&request_clone.body_json)?;

            tx.execute(
                "INSERT INTO captured_requests (id, timestamp, url, method, headers, body, body_json, total_tokens, correlation_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    &request_clone.id,
                    request_clone.timestamp.timestamp(),
                    &request_clone.url,
                    &request_clone.method,
                    &headers_json,
                    &request_clone.body,
                    &body_json_str,
                    request_clone.total_tokens,
                    &request_clone.correlation_id,
                ],
            )?;

            // Insert attributions
            for attr in &attributions_clone {
                tx.execute(
                    "INSERT INTO context_attributions (id, request_id, source_type, source_name, token_count, content_hash, message_index, message_role)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        &attr.id,
                        &attr.request_id,
                        &attr.source_type.to_string(),
                        &attr.source_name,
                        attr.token_count,
                        &attr.content_hash,
                        attr.message_index,
                        &attr.message_role,
                    ],
                )?;
            }

            tx.commit()?;

            tracing::info!(
                request_id = %request_clone.id,
                correlation_id = %request_clone.correlation_id,
                url = %request_clone.url,
                attribution_count = attributions_clone.len(),
                total_tokens = request_clone.total_tokens,
                "Captured request with attributions"
            );

            Ok::<_, CaptureError>(())
        })
        .await
        .map_err(|e| CaptureError::DatabaseError(format!("Task join error: {}", e)))?
        .map_err(|e| {
            tracing::warn!(error = %e, "Failed to store request in database");
            e
        })
        .ok(); // Don't fail if DB write fails (fail-open)

        Ok(correlation_id)
    }

    /// Capture a response and link it to the request
    pub async fn capture_response(
        &self,
        correlation_id: &str,
        status_code: u16,
        headers: HashMap<String, String>,
        body: Vec<u8>,
        latency_ms: u64,
        proxy_latency_ms: u64,
    ) -> Result<(), CaptureError> {
        let response_id = format!("resp_{}", Uuid::new_v4());

        // Parse body as JSON
        let body_json: serde_json::Value =
            serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::Value::Null);

        // Extract response_tokens from Claude API usage field
        let response_tokens = body_json
            .get("usage")
            .and_then(|u| u.get("output_tokens"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as usize;

        let response = CapturedResponse {
            id: response_id.clone(),
            correlation_id: correlation_id.to_string(),
            timestamp: Utc::now(),
            status_code,
            headers,
            body,
            body_json,
            latency_ms,
            proxy_latency_ms,
            response_tokens,
        };

        // Store in cache
        self.response_cache
            .insert(response_id.clone(), response.clone());

        // Store in database (async)
        let db_path = self.db_path.clone();
        let response_clone = response.clone();
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;
            let headers_json = serde_json::to_string(&response_clone.headers)?;
            let body_json_str = serde_json::to_string(&response_clone.body_json)?;

            conn.execute(
                "INSERT INTO captured_responses (id, correlation_id, timestamp, status_code, headers, body, body_json, latency_ms, proxy_latency_ms, response_tokens)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    &response_clone.id,
                    &response_clone.correlation_id,
                    response_clone.timestamp.timestamp(),
                    response_clone.status_code,
                    &headers_json,
                    &response_clone.body,
                    &body_json_str,
                    response_clone.latency_ms as i64,
                    response_clone.proxy_latency_ms as i64,
                    response_clone.response_tokens,
                ],
            )?;

            tracing::info!(
                response_id = %response_clone.id,
                correlation_id = %response_clone.correlation_id,
                status_code = response_clone.status_code,
                latency_ms = response_clone.latency_ms,
                "Captured response"
            );

            Ok::<_, CaptureError>(())
        })
        .await
        .map_err(|e| CaptureError::DatabaseError(format!("Task join error: {}", e)))?
        .map_err(|e| {
            tracing::warn!(error = %e, "Failed to store response in database");
            e
        })
        .ok(); // Don't fail if DB write fails (fail-open)

        Ok(())
    }

    /// Store context attributions for a request
    pub async fn store_attributions(
        &self,
        request_id: &str,
        attributions: Vec<ContextAttribution>,
    ) -> Result<(), CaptureError> {
        let db_path = self.db_path.clone();
        let request_id = request_id.to_string();
        let total_tokens: usize = attributions.iter().map(|a| a.token_count).sum();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;

            // Start transaction
            let tx = conn.unchecked_transaction()?;

            // Insert all attributions
            for attr in &attributions {
                tx.execute(
                    "INSERT INTO context_attributions (id, request_id, source_type, source_name, token_count, content_hash, message_index, message_role)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        &attr.id,
                        &attr.request_id,
                        &attr.source_type.to_string(),
                        &attr.source_name,
                        attr.token_count,
                        &attr.content_hash,
                        attr.message_index,
                        &attr.message_role,
                    ],
                )?;
            }

            // Update total_tokens in request
            tx.execute(
                "UPDATE captured_requests SET total_tokens = ?1 WHERE id = ?2",
                rusqlite::params![total_tokens, &request_id],
            )?;

            tx.commit()?;

            tracing::info!(
                request_id = %request_id,
                attribution_count = attributions.len(),
                total_tokens = total_tokens,
                "Stored context attributions"
            );

            Ok::<_, CaptureError>(())
        })
        .await
        .map_err(|e| CaptureError::DatabaseError(format!("Task join error: {}", e)))?
        .map_err(|e| {
            tracing::warn!(error = %e, "Failed to store attributions in database");
            e
        })
        .ok(); // Don't fail if DB write fails (fail-open)

        Ok(())
    }

    /// Get a captured request by ID
    pub async fn get_request(&self, id: &str) -> Result<Option<CapturedRequest>, CaptureError> {
        // Check cache first
        if let Some(request) = self.request_cache.get(id) {
            return Ok(Some(request.clone()));
        }

        // Fallback to database
        let db_path = self.db_path.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;

            let mut stmt = conn.prepare(
                "SELECT id, timestamp, url, method, headers, body, body_json, total_tokens, correlation_id
                 FROM captured_requests WHERE id = ?1",
            )?;

            let request = stmt.query_row([&id], |row| {
                let headers_json: String = row.get(4)?;
                let headers: HashMap<String, String> = serde_json::from_str(&headers_json)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                let body_json_str: String = row.get(6)?;
                let body_json: serde_json::Value = serde_json::from_str(&body_json_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                Ok(CapturedRequest {
                    id: row.get(0)?,
                    timestamp: DateTime::from_timestamp(row.get(1)?, 0).unwrap_or_else(Utc::now),
                    url: row.get(2)?,
                    method: row.get(3)?,
                    headers,
                    body: row.get(5)?,
                    body_json,
                    total_tokens: row.get(7)?,
                    correlation_id: row.get(8)?,
                })
            });

            match request {
                Ok(req) => Ok(Some(req)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(CaptureError::DatabaseError(e.to_string())),
            }
        })
        .await
        .map_err(|e| CaptureError::DatabaseError(format!("Task join error: {}", e)))?
    }

    /// Query captured requests with filters
    pub async fn query_requests(
        &self,
        filters: QueryFilters,
    ) -> Result<Vec<CapturedRequest>, CaptureError> {
        let db_path = self.db_path.clone();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;

            let mut query = "SELECT id, timestamp, url, method, headers, body, body_json, total_tokens, correlation_id FROM captured_requests WHERE 1=1".to_string();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

            if let Some(start) = filters.start_time {
                query.push_str(" AND timestamp >= ?");
                params.push(Box::new(start.timestamp()));
            }
            if let Some(end) = filters.end_time {
                query.push_str(" AND timestamp <= ?");
                params.push(Box::new(end.timestamp()));
            }

            query.push_str(" ORDER BY timestamp DESC");

            if let Some(limit) = filters.limit {
                query.push_str(" LIMIT ?");
                params.push(Box::new(limit));
            }
            if let Some(offset) = filters.offset {
                query.push_str(" OFFSET ?");
                params.push(Box::new(offset));
            }

            let mut stmt = conn.prepare(&query)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

            let requests = stmt
                .query_map(param_refs.as_slice(), |row| {
                    let headers_json: String = row.get(4)?;
                    let headers: HashMap<String, String> = serde_json::from_str(&headers_json)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                    let body_json_str: String = row.get(6)?;
                    let body_json: serde_json::Value = serde_json::from_str(&body_json_str)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                    Ok(CapturedRequest {
                        id: row.get(0)?,
                        timestamp: DateTime::from_timestamp(row.get(1)?, 0).unwrap_or_else(Utc::now),
                        url: row.get(2)?,
                        method: row.get(3)?,
                        headers,
                        body: row.get(5)?,
                        body_json,
                        total_tokens: row.get(7)?,
                        correlation_id: row.get(8)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(requests)
        })
        .await
        .map_err(|e| CaptureError::DatabaseError(format!("Task join error: {}", e)))?
    }

    /// Get attributions for a request
    pub async fn get_attributions_for_request(
        &self,
        request_id: &str,
    ) -> Result<Vec<ContextAttribution>, CaptureError> {
        let db_path = self.db_path.clone();
        let request_id = request_id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;
            let mut stmt = conn.prepare(
                "SELECT id, request_id, source_type, source_name, token_count, content_hash, message_index, message_role
                 FROM context_attributions WHERE request_id = ?1",
            )?;

            let attributions = stmt
                .query_map([&request_id], |row| {
                    let source_type_str: String = row.get(2)?;
                    let source_type = match source_type_str.as_str() {
                        "User" => crate::context::types::SourceType::User,
                        "Framework" => crate::context::types::SourceType::Framework,
                        "McpServer" => crate::context::types::SourceType::McpServer,
                        "Skill" => crate::context::types::SourceType::Skill,
                        _ => crate::context::types::SourceType::Framework,
                    };

                    Ok(ContextAttribution {
                        id: row.get(0)?,
                        request_id: row.get(1)?,
                        source_type,
                        source_name: row.get(3)?,
                        token_count: row.get(4)?,
                        content_hash: row.get(5)?,
                        message_index: row.get(6)?,
                        message_role: row.get(7)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(attributions)
        })
        .await
        .map_err(|e| CaptureError::DatabaseError(format!("Task join error: {}", e)))?
    }

    /// Get response by correlation ID
    pub async fn get_response_by_correlation(
        &self,
        correlation_id: &str,
    ) -> Result<Option<CapturedResponse>, CaptureError> {
        // Check cache first
        if let Some(response) = self
            .response_cache
            .iter()
            .find(|r| r.correlation_id == correlation_id)
        {
            return Ok(Some(response.clone()));
        }

        // Fallback to database
        let db_path = self.db_path.clone();
        let correlation_id = correlation_id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&db_path)?;
            let mut stmt = conn.prepare(
                "SELECT id, correlation_id, timestamp, status_code, headers, body, body_json, latency_ms, proxy_latency_ms, response_tokens
                 FROM captured_responses WHERE correlation_id = ?1",
            )?;

            let response = stmt.query_row([&correlation_id], |row| {
                let headers_json: String = row.get(4)?;
                let headers: HashMap<String, String> = serde_json::from_str(&headers_json)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                let body_json_str: String = row.get(6)?;
                let body_json: serde_json::Value = serde_json::from_str(&body_json_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                Ok(CapturedResponse {
                    id: row.get(0)?,
                    correlation_id: row.get(1)?,
                    timestamp: DateTime::from_timestamp(row.get(2)?, 0).unwrap_or_else(Utc::now),
                    status_code: row.get(3)?,
                    headers,
                    body: row.get(5)?,
                    body_json,
                    latency_ms: row.get(7)?,
                    proxy_latency_ms: row.get(8)?,
                    response_tokens: row.get(9)?,
                })
            });

            match response {
                Ok(resp) => Ok(Some(resp)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(CaptureError::DatabaseError(e.to_string())),
            }
        })
        .await
        .map_err(|e| CaptureError::DatabaseError(format!("Task join error: {}", e)))?
    }

    /// Sanitize sensitive headers (remove API keys)
    fn sanitize_headers(headers: &mut HashMap<String, String>) {
        let sensitive_headers = ["authorization", "x-api-key", "api-key"];

        for key in sensitive_headers {
            if headers.contains_key(key) {
                headers.insert(key.to_string(), "[REDACTED]".to_string());
            }
            // Also check lowercase variants
            let lower_keys: Vec<String> = headers
                .keys()
                .filter(|k| k.to_lowercase() == key)
                .cloned()
                .collect();
            for lower_key in lower_keys {
                headers.insert(lower_key, "[REDACTED]".to_string());
            }
        }
    }
}

impl Clone for CaptureStorage {
    fn clone(&self) -> Self {
        Self {
            db_path: self.db_path.clone(),
            request_cache: self.request_cache.clone(),
            response_cache: self.response_cache.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_capture_request() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        // Initialize schema first
        {
            let conn = Connection::open(&db_path).expect("Failed to open db");
            crate::context::storage::initialize_schema(&conn).expect("Failed to init schema");
        }

        let storage = CaptureStorage::new(db_path).expect("Failed to create storage");

        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert(
            "authorization".to_string(),
            "Bearer sk-test-key".to_string(),
        );

        let body = br#"{"model":"claude-3-5-sonnet-20241022","messages":[]}"#.to_vec();

        let corr_id = storage
            .capture_request(
                "https://api.anthropic.com/v1/messages".to_string(),
                "POST".to_string(),
                headers,
                body,
            )
            .await
            .expect("Failed to capture request");

        assert!(corr_id.starts_with("corr_"));

        // Verify request was cached
        assert_eq!(storage.request_cache.len(), 1);
    }

    #[test]
    fn test_sanitize_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert(
            "Authorization".to_string(),
            "Bearer sk-test-key".to_string(),
        );
        headers.insert("X-API-Key".to_string(), "secret-key".to_string());

        CaptureStorage::sanitize_headers(&mut headers);

        assert_eq!(
            headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            headers.get("Authorization"),
            Some(&"[REDACTED]".to_string())
        );
        assert_eq!(headers.get("X-API-Key"), Some(&"[REDACTED]".to_string()));
    }
}
