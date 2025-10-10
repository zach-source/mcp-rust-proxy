//! Hybrid storage backend with DashMap + SQLite
//!
//! This module provides a persistent storage layer for context tracing data using:
//! - **DashMap**: In-memory cache for hot data (recent responses, high-traffic lookups)
//! - **SQLite**: Persistent storage for complete lineage history
//!
//! # Architecture
//!
//! ```text
//! Write Path:  Request → DashMap (sync) → SQLite (async)
//! Read Path:   Request → DashMap cache → SQLite fallback
//! ```
//!
//! # Concurrency
//!
//! - DashMap provides lock-free concurrent access for cache operations
//! - SQLite uses WAL mode for concurrent reads with single-writer model
//! - Arc wrapper enables safe sharing across async tasks

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::context::types::{ContextUnit, FeedbackRecord, LineageManifest, Response};

/// Storage backend abstraction for context tracing data
///
/// This trait defines the interface for storing and retrieving context tracing data.
/// Implementations must handle concurrent access safely and provide async operations.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    // ========== Context Unit Operations ==========

    /// Store a new context unit or update an existing one
    ///
    /// # Arguments
    /// * `unit` - The context unit to store
    ///
    /// # Returns
    /// * `Ok(())` if stored successfully
    /// * `Err(_)` if storage operation fails
    async fn store_context_unit(&self, unit: &ContextUnit) -> Result<(), StorageError>;

    /// Retrieve a context unit by its ID
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the context unit
    ///
    /// # Returns
    /// * `Ok(Some(unit))` if found
    /// * `Ok(None)` if not found
    /// * `Err(_)` if retrieval operation fails
    async fn get_context_unit(&self, id: &str) -> Result<Option<ContextUnit>, StorageError>;

    /// Update aggregate score and feedback count for a context unit
    ///
    /// This is called during feedback propagation to update context quality metrics.
    ///
    /// # Arguments
    /// * `id` - The context unit ID to update
    /// * `aggregate_score` - New aggregate quality score
    /// * `feedback_count` - New feedback count
    ///
    /// # Returns
    /// * `Ok(())` if updated successfully
    /// * `Err(_)` if update operation fails
    async fn update_context_unit(
        &self,
        id: &str,
        aggregate_score: f32,
        feedback_count: i32,
    ) -> Result<(), StorageError>;

    // ========== Response Operations ==========

    /// Store a new response record
    ///
    /// # Arguments
    /// * `response` - The response to store
    ///
    /// # Returns
    /// * `Ok(())` if stored successfully
    /// * `Err(_)` if storage operation fails
    async fn store_response(&self, response: &Response) -> Result<(), StorageError>;

    /// Retrieve a response by its ID
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the response (must start with "resp_")
    ///
    /// # Returns
    /// * `Ok(Some(response))` if found
    /// * `Ok(None)` if not found
    /// * `Err(_)` if retrieval operation fails
    async fn get_response(&self, id: &str) -> Result<Option<Response>, StorageError>;

    // ========== Lineage Operations ==========

    /// Store a complete lineage manifest
    ///
    /// This stores the provenance tree showing relationships between context units
    /// and the response.
    ///
    /// # Arguments
    /// * `manifest` - The lineage manifest to store
    ///
    /// # Returns
    /// * `Ok(())` if stored successfully
    /// * `Err(_)` if storage operation fails
    async fn store_lineage(&self, manifest: &LineageManifest) -> Result<(), StorageError>;

    /// Query lineage data for a response
    ///
    /// # Arguments
    /// * `response_id` - The response ID to query lineage for
    ///
    /// # Returns
    /// * `Ok(Some(manifest))` if found
    /// * `Ok(None)` if not found
    /// * `Err(_)` if query operation fails
    async fn query_lineage(
        &self,
        response_id: &str,
    ) -> Result<Option<LineageManifest>, StorageError>;

    // ========== Feedback Operations ==========

    /// Store a new feedback record
    ///
    /// # Arguments
    /// * `feedback` - The feedback record to store
    ///
    /// # Returns
    /// * `Ok(())` if stored successfully
    /// * `Err(_)` if storage operation fails
    async fn store_feedback(&self, feedback: &FeedbackRecord) -> Result<(), StorageError>;

    /// Retrieve all feedback for a specific response
    ///
    /// # Arguments
    /// * `response_id` - The response ID to get feedback for
    ///
    /// # Returns
    /// * `Ok(Vec<feedback>)` with all feedback records (empty vec if none)
    /// * `Err(_)` if retrieval operation fails
    async fn get_feedback(&self, response_id: &str) -> Result<Vec<FeedbackRecord>, StorageError>;

    /// Retrieve feedback records within a time range
    ///
    /// Used for analytics and reporting.
    ///
    /// # Arguments
    /// * `start` - Start of time range (inclusive)
    /// * `end` - End of time range (inclusive)
    ///
    /// # Returns
    /// * `Ok(Vec<feedback>)` with matching feedback records
    /// * `Err(_)` if retrieval operation fails
    async fn get_feedback_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<FeedbackRecord>, StorageError>;

    // ========== Evolution/Version Operations ==========

    /// Get complete version chain for a context unit
    ///
    /// Traverses the version chain following previous_version_id links.
    ///
    /// # Arguments
    /// * `context_unit_id` - Any version in the chain
    ///
    /// # Returns
    /// * `Ok(Vec<ContextUnit>)` with all versions ordered from oldest to newest
    /// * `Err(_)` if query fails
    async fn get_context_version_chain(
        &self,
        context_unit_id: &str,
    ) -> Result<Vec<ContextUnit>, StorageError>;

    // ========== Query Operations ==========

    /// Get all responses that used a specific context unit
    ///
    /// # Arguments
    /// * `context_unit_id` - The context unit ID to search for
    /// * `min_weight` - Minimum weight threshold (optional)
    /// * `start_date` - Start date for filtering (optional, inclusive)
    /// * `end_date` - End date for filtering (optional, inclusive)
    /// * `limit` - Maximum number of results (optional)
    ///
    /// # Returns
    /// * `Ok(Vec<ResponseSummary>)` with matching responses
    /// * `Err(_)` if query fails
    async fn get_responses_for_context(
        &self,
        context_unit_id: &str,
        min_weight: Option<f32>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<crate::context::types::ResponseSummary>, StorageError>;

    // ========== Maintenance Operations ==========

    /// Execute retention policy to clean up old data
    ///
    /// Removes data older than the configured retention period (default 90 days).
    /// Should be called periodically (e.g., daily cron job).
    ///
    /// # Arguments
    /// * `retention_days` - Number of days to retain data
    ///
    /// # Returns
    /// * `Ok(count)` with number of records deleted
    /// * `Err(_)` if cleanup operation fails
    async fn cleanup_old_data(&self, retention_days: u32) -> Result<usize, StorageError>;
}

/// Storage operation errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Database connection or query error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid data or constraint violation
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Cache operation error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

// Implement conversion from rusqlite errors
impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        StorageError::DatabaseError(err.to_string())
    }
}

// Implement conversion from serde_json errors
impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

// ========== SQLite Schema Implementation ==========

/// Initialize SQLite database schema for context tracing
///
/// Creates all required tables with indexes and foreign keys.
/// Configures WAL mode for concurrent access.
///
/// # Arguments
/// * `conn` - SQLite connection to initialize
///
/// # Returns
/// * `Ok(())` if schema initialized successfully
/// * `Err(StorageError)` if initialization fails
pub fn initialize_schema(conn: &rusqlite::Connection) -> Result<(), StorageError> {
    // Enable WAL mode for concurrent reads
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    // Create context_units table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS context_units (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL,
            source TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            embedding_id TEXT,
            summary TEXT,
            version INTEGER NOT NULL DEFAULT 1,
            previous_version_id TEXT,
            aggregate_score REAL NOT NULL DEFAULT 0.0,
            feedback_count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (previous_version_id) REFERENCES context_units(id)
        )",
        [],
    )?;

    // Create indexes for context_units
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_context_units_type ON context_units(type)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_context_units_source ON context_units(source)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_context_units_timestamp ON context_units(timestamp)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_context_units_embedding ON context_units(embedding_id)",
        [],
    )?;

    // Create responses table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS responses (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            agent TEXT NOT NULL,
            model TEXT NOT NULL,
            token_count INTEGER
        )",
        [],
    )?;

    // Create indexes for responses
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_responses_timestamp ON responses(timestamp)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_responses_agent ON responses(agent)",
        [],
    )?;

    // Create lineage junction table (response -> context_unit mapping)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS lineage (
            response_id TEXT NOT NULL,
            context_unit_id TEXT NOT NULL,
            weight REAL NOT NULL,
            PRIMARY KEY (response_id, context_unit_id),
            FOREIGN KEY (response_id) REFERENCES responses(id) ON DELETE CASCADE,
            FOREIGN KEY (context_unit_id) REFERENCES context_units(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes for lineage
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_lineage_context ON lineage(context_unit_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_lineage_response ON lineage(response_id)",
        [],
    )?;

    // Create lineage_manifests table for complete provenance trees
    conn.execute(
        "CREATE TABLE IF NOT EXISTS lineage_manifests (
            response_id TEXT PRIMARY KEY,
            manifest_json TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            FOREIGN KEY (response_id) REFERENCES responses(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create feedback table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS feedback (
            id TEXT PRIMARY KEY,
            response_id TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            score REAL NOT NULL,
            feedback_text TEXT,
            user_id TEXT,
            FOREIGN KEY (response_id) REFERENCES responses(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes for feedback
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_feedback_response ON feedback(response_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_feedback_timestamp ON feedback(timestamp)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_feedback_user ON feedback(user_id)",
        [],
    )?;

    // Create sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            user_query TEXT,
            agent TEXT NOT NULL,
            metadata TEXT,
            session_score REAL
        )",
        [],
    )?;

    // Create indexes for sessions
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(agent)",
        [],
    )?;

    // Create tasks table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            description TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            completed_at TEXT,
            task_score REAL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes for tasks
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tasks_session ON tasks(session_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)",
        [],
    )?;

    // Create task_responses junction table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_responses (
            task_id TEXT NOT NULL,
            response_id TEXT NOT NULL,
            PRIMARY KEY (task_id, response_id),
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (response_id) REFERENCES responses(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Add session_id to responses table (migration)
    conn.execute("ALTER TABLE responses ADD COLUMN session_id TEXT", [])
        .ok(); // Ignore error if column already exists

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_responses_session ON responses(session_id)",
        [],
    )?;

    Ok(())
}

/// Execute retention policy to clean up old data
///
/// # Arguments
/// * `conn` - SQLite connection
/// * `retention_days` - Number of days to retain data (default 90)
///
/// # Returns
/// * `Ok(count)` with number of records deleted
/// * `Err(StorageError)` if cleanup fails
pub fn cleanup_old_data(
    conn: &rusqlite::Connection,
    retention_days: u32,
) -> Result<usize, StorageError> {
    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
    let cutoff_str = cutoff_date.to_rfc3339();

    // Delete old responses (cascades to lineage and lineage_manifests)
    let deleted = conn.execute("DELETE FROM responses WHERE timestamp < ?1", [&cutoff_str])?;

    // Delete old feedback
    conn.execute("DELETE FROM feedback WHERE timestamp < ?1", [&cutoff_str])?;

    // Delete orphaned context units (no longer referenced by any response)
    conn.execute(
        "DELETE FROM context_units
         WHERE id NOT IN (SELECT DISTINCT context_unit_id FROM lineage)
         AND timestamp < ?1",
        [&cutoff_str],
    )?;

    Ok(deleted)
}

// ========== Hybrid Storage Implementation ==========

use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Cache entry with timestamp for TTL tracking
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    inserted_at: DateTime<Utc>,
}

/// Hybrid storage backend combining DashMap cache with SQLite persistence
///
/// # Architecture
/// - Hot cache: DashMap for recent responses (configurable TTL and size)
/// - Cold storage: SQLite for complete history
/// - Write path: Update cache + async write to SQLite
/// - Read path: Check cache → fallback to SQLite
pub struct HybridStorage {
    /// In-memory cache for responses
    response_cache: Arc<DashMap<String, CacheEntry<Response>>>,
    /// In-memory cache for context units
    context_cache: Arc<DashMap<String, CacheEntry<ContextUnit>>>,
    /// In-memory cache for lineage manifests
    lineage_cache: Arc<DashMap<String, CacheEntry<LineageManifest>>>,
    /// SQLite connection (wrapped in Mutex for async access)
    db: Arc<Mutex<rusqlite::Connection>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache statistics
    stats: Arc<CacheStats>,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in cache
    pub max_entries: usize,
    /// Time-to-live for cache entries (in seconds)
    pub ttl_seconds: i64,
    /// Cache eviction strategy
    pub eviction_strategy: EvictionStrategy,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            ttl_seconds: 7 * 24 * 60 * 60, // 7 days
            eviction_strategy: EvictionStrategy::TimeBasedLRU,
        }
    }
}

/// Cache eviction strategy
#[derive(Debug, Clone, Copy)]
pub enum EvictionStrategy {
    /// Time-based least recently used
    TimeBasedLRU,
    /// Strict LRU (not yet implemented)
    StrictLRU,
}

/// Cache statistics for monitoring
#[derive(Debug, Default)]
pub struct CacheStats {
    hits: Arc<Mutex<u64>>,
    misses: Arc<Mutex<u64>>,
    evictions: Arc<Mutex<u64>>,
}

impl CacheStats {
    async fn record_hit(&self) {
        let mut hits = self.hits.lock().await;
        *hits += 1;
    }

    async fn record_miss(&self) {
        let mut misses = self.misses.lock().await;
        *misses += 1;
    }

    async fn record_eviction(&self) {
        let mut evictions = self.evictions.lock().await;
        *evictions += 1;
    }

    pub async fn get_stats(&self) -> (u64, u64, u64) {
        let hits = *self.hits.lock().await;
        let misses = *self.misses.lock().await;
        let evictions = *self.evictions.lock().await;
        (hits, misses, evictions)
    }
}

impl HybridStorage {
    /// Create a new hybrid storage instance
    ///
    /// # Arguments
    /// * `db_path` - Path to SQLite database file
    /// * `config` - Cache configuration (optional, uses defaults if None)
    ///
    /// # Returns
    /// * `Ok(HybridStorage)` if initialized successfully
    /// * `Err(StorageError)` if initialization fails
    pub async fn new(db_path: PathBuf, config: Option<CacheConfig>) -> Result<Self, StorageError> {
        let config = config.unwrap_or_default();

        // Open SQLite connection
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Initialize schema
        initialize_schema(&conn)?;

        Ok(Self {
            response_cache: Arc::new(DashMap::new()),
            context_cache: Arc::new(DashMap::new()),
            lineage_cache: Arc::new(DashMap::new()),
            db: Arc::new(Mutex::new(conn)),
            config,
            stats: Arc::new(CacheStats::default()),
        })
    }

    /// Check if cache entry is still valid (not expired)
    fn is_cache_valid<T>(&self, entry: &CacheEntry<T>) -> bool {
        let age = chrono::Utc::now().signed_duration_since(entry.inserted_at);
        age.num_seconds() < self.config.ttl_seconds
    }

    /// Evict expired entries from cache if over max size
    async fn evict_if_needed<T: Clone>(&self, cache: &Arc<DashMap<String, CacheEntry<T>>>) {
        if cache.len() > self.config.max_entries {
            let now = chrono::Utc::now();

            // Collect expired keys
            let expired_keys: Vec<String> = cache
                .iter()
                .filter(|entry| {
                    let age = now.signed_duration_since(entry.value().inserted_at);
                    age.num_seconds() >= self.config.ttl_seconds
                })
                .map(|entry| entry.key().clone())
                .collect();

            // Remove expired entries
            for key in expired_keys {
                cache.remove(&key);
                self.stats.record_eviction().await;
            }

            // If still over limit, remove oldest entries
            if cache.len() > self.config.max_entries {
                let mut entries: Vec<_> = cache
                    .iter()
                    .map(|e| (e.key().clone(), e.value().inserted_at))
                    .collect();

                entries.sort_by_key(|(_, ts)| *ts);

                let to_remove = cache.len() - self.config.max_entries;
                for (key, _) in entries.iter().take(to_remove) {
                    cache.remove(key);
                    self.stats.record_eviction().await;
                }
            }
        }
    }
}

#[async_trait]
impl StorageBackend for HybridStorage {
    async fn store_context_unit(&self, unit: &ContextUnit) -> Result<(), StorageError> {
        // Validate before storing
        unit.validate()
            .map_err(|e| StorageError::ValidationError(e))?;

        // Update cache
        self.context_cache.insert(
            unit.id.clone(),
            CacheEntry {
                data: unit.clone(),
                inserted_at: chrono::Utc::now(),
            },
        );

        // Evict if needed
        self.evict_if_needed(&self.context_cache).await;

        // Persist to SQLite
        let db = self.db.lock().await;
        db.execute(
            "INSERT OR REPLACE INTO context_units
             (id, type, source, timestamp, embedding_id, summary, version, previous_version_id, aggregate_score, feedback_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                &unit.id,
                format!("{:?}", unit.r#type),
                &unit.source,
                unit.timestamp.to_rfc3339(),
                &unit.embedding_id,
                &unit.summary,
                unit.version,
                &unit.previous_version_id,
                unit.aggregate_score,
                unit.feedback_count,
            ],
        )?;

        Ok(())
    }

    async fn get_context_unit(&self, id: &str) -> Result<Option<ContextUnit>, StorageError> {
        // Check cache first
        if let Some(entry) = self.context_cache.get(id) {
            if self.is_cache_valid(&entry) {
                self.stats.record_hit().await;
                return Ok(Some(entry.data.clone()));
            } else {
                // Remove expired entry
                drop(entry);
                self.context_cache.remove(id);
            }
        }

        self.stats.record_miss().await;

        // Fallback to SQLite
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, type, source, timestamp, embedding_id, summary, version, previous_version_id, aggregate_score, feedback_count
             FROM context_units WHERE id = ?1",
        )?;

        let result = stmt.query_row([id], |row| {
            Ok(ContextUnit {
                id: row.get(0)?,
                r#type: match row.get::<_, String>(1)?.as_str() {
                    "System" => crate::context::types::ContextType::System,
                    "User" => crate::context::types::ContextType::User,
                    "External" => crate::context::types::ContextType::External,
                    "ModelState" => crate::context::types::ContextType::ModelState,
                    _ => crate::context::types::ContextType::User,
                },
                source: row.get(2)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                embedding_id: row.get(4)?,
                summary: row.get(5)?,
                version: row.get(6)?,
                previous_version_id: row.get(7)?,
                aggregate_score: row.get(8)?,
                feedback_count: row.get(9)?,
            })
        });

        match result {
            Ok(unit) => {
                // Cache for future reads
                self.context_cache.insert(
                    id.to_string(),
                    CacheEntry {
                        data: unit.clone(),
                        inserted_at: chrono::Utc::now(),
                    },
                );
                Ok(Some(unit))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::DatabaseError(e.to_string())),
        }
    }

    async fn update_context_unit(
        &self,
        id: &str,
        aggregate_score: f32,
        feedback_count: i32,
    ) -> Result<(), StorageError> {
        // Update cache if present
        if let Some(mut entry) = self.context_cache.get_mut(id) {
            entry.data.aggregate_score = aggregate_score;
            entry.data.feedback_count = feedback_count;
        }

        // Update SQLite
        let db = self.db.lock().await;
        db.execute(
            "UPDATE context_units SET aggregate_score = ?1, feedback_count = ?2 WHERE id = ?3",
            rusqlite::params![aggregate_score, feedback_count, id],
        )?;

        Ok(())
    }

    async fn store_response(&self, response: &Response) -> Result<(), StorageError> {
        // Validate before storing
        response
            .validate()
            .map_err(|e| StorageError::ValidationError(e))?;

        // Update cache
        self.response_cache.insert(
            response.id.clone(),
            CacheEntry {
                data: response.clone(),
                inserted_at: chrono::Utc::now(),
            },
        );

        // Evict if needed
        self.evict_if_needed(&self.response_cache).await;

        // Persist to SQLite
        let db = self.db.lock().await;
        db.execute(
            "INSERT OR REPLACE INTO responses (id, timestamp, agent, model, token_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                &response.id,
                response.timestamp.to_rfc3339(),
                &response.agent,
                &response.model,
                response.token_count,
            ],
        )?;

        // Store lineage relationships
        for ctx_ref in &response.context_units {
            db.execute(
                "INSERT OR REPLACE INTO lineage (response_id, context_unit_id, weight)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![&response.id, &ctx_ref.context_unit_id, ctx_ref.weight],
            )?;
        }

        Ok(())
    }

    async fn get_response(&self, id: &str) -> Result<Option<Response>, StorageError> {
        // Check cache first
        if let Some(entry) = self.response_cache.get(id) {
            if self.is_cache_valid(&entry) {
                self.stats.record_hit().await;
                return Ok(Some(entry.data.clone()));
            } else {
                drop(entry);
                self.response_cache.remove(id);
            }
        }

        self.stats.record_miss().await;

        // Fallback to SQLite
        let db = self.db.lock().await;

        // Get response metadata
        let mut stmt = db.prepare(
            "SELECT id, timestamp, agent, model, token_count FROM responses WHERE id = ?1",
        )?;

        let response_result = stmt.query_row([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i32>>(4)?,
            ))
        });

        match response_result {
            Ok((id, timestamp_str, agent, model, token_count)) => {
                // Get lineage data
                let mut lineage_stmt = db.prepare(
                    "SELECT context_unit_id, weight FROM lineage WHERE response_id = ?1",
                )?;

                let context_units: Vec<crate::context::types::ContextReference> = lineage_stmt
                    .query_map([&id], |row| {
                        Ok(crate::context::types::ContextReference {
                            context_unit_id: row.get(0)?,
                            weight: row.get(1)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;

                let response = Response {
                    id,
                    timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    agent,
                    model,
                    token_count,
                    context_units,
                };

                // Cache for future reads
                self.response_cache.insert(
                    response.id.clone(),
                    CacheEntry {
                        data: response.clone(),
                        inserted_at: chrono::Utc::now(),
                    },
                );

                Ok(Some(response))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::DatabaseError(e.to_string())),
        }
    }

    async fn store_lineage(&self, manifest: &LineageManifest) -> Result<(), StorageError> {
        // Update cache
        self.lineage_cache.insert(
            manifest.response_id.clone(),
            CacheEntry {
                data: manifest.clone(),
                inserted_at: chrono::Utc::now(),
            },
        );

        // Evict if needed
        self.evict_if_needed(&self.lineage_cache).await;

        // Serialize manifest to JSON
        let manifest_json = serde_json::to_string(manifest)?;

        // Persist to SQLite
        let db = self.db.lock().await;
        db.execute(
            "INSERT OR REPLACE INTO lineage_manifests (response_id, manifest_json, timestamp)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![
                &manifest.response_id,
                &manifest_json,
                manifest.timestamp.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    async fn query_lineage(
        &self,
        response_id: &str,
    ) -> Result<Option<LineageManifest>, StorageError> {
        // Check cache first
        if let Some(entry) = self.lineage_cache.get(response_id) {
            if self.is_cache_valid(&entry) {
                self.stats.record_hit().await;
                return Ok(Some(entry.data.clone()));
            } else {
                drop(entry);
                self.lineage_cache.remove(response_id);
            }
        }

        self.stats.record_miss().await;

        // Fallback to SQLite
        let db = self.db.lock().await;
        let mut stmt =
            db.prepare("SELECT manifest_json FROM lineage_manifests WHERE response_id = ?1")?;

        let result = stmt.query_row([response_id], |row| {
            let json_str: String = row.get(0)?;
            Ok(json_str)
        });

        match result {
            Ok(json_str) => {
                let manifest: LineageManifest = serde_json::from_str(&json_str)?;

                // Cache for future reads
                self.lineage_cache.insert(
                    response_id.to_string(),
                    CacheEntry {
                        data: manifest.clone(),
                        inserted_at: chrono::Utc::now(),
                    },
                );

                Ok(Some(manifest))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::DatabaseError(e.to_string())),
        }
    }

    async fn store_feedback(&self, feedback: &FeedbackRecord) -> Result<(), StorageError> {
        // Validate before storing
        feedback
            .validate()
            .map_err(|e| StorageError::ValidationError(e))?;

        // Persist to SQLite (feedback not cached)
        let db = self.db.lock().await;
        db.execute(
            "INSERT INTO feedback (id, response_id, timestamp, score, feedback_text, user_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &feedback.id,
                &feedback.response_id,
                feedback.timestamp.to_rfc3339(),
                feedback.score,
                &feedback.feedback_text,
                &feedback.user_id,
            ],
        )?;

        Ok(())
    }

    async fn get_feedback(&self, response_id: &str) -> Result<Vec<FeedbackRecord>, StorageError> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, response_id, timestamp, score, feedback_text, user_id
             FROM feedback WHERE response_id = ?1 ORDER BY timestamp DESC",
        )?;

        let feedback: Vec<FeedbackRecord> = stmt
            .query_map([response_id], |row| {
                Ok(FeedbackRecord {
                    id: row.get(0)?,
                    response_id: row.get(1)?,
                    timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    score: row.get(3)?,
                    feedback_text: row.get(4)?,
                    user_id: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(feedback)
    }

    async fn get_feedback_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<FeedbackRecord>, StorageError> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, response_id, timestamp, score, feedback_text, user_id
             FROM feedback WHERE timestamp >= ?1 AND timestamp <= ?2 ORDER BY timestamp DESC",
        )?;

        let feedback: Vec<FeedbackRecord> = stmt
            .query_map([start.to_rfc3339(), end.to_rfc3339()], |row| {
                Ok(FeedbackRecord {
                    id: row.get(0)?,
                    response_id: row.get(1)?,
                    timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    score: row.get(3)?,
                    feedback_text: row.get(4)?,
                    user_id: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(feedback)
    }

    async fn get_context_version_chain(
        &self,
        context_unit_id: &str,
    ) -> Result<Vec<ContextUnit>, StorageError> {
        let db = self.db.lock().await;

        // Use recursive CTE to traverse version chain
        let sql = "
            WITH RECURSIVE version_chain AS (
                -- Base case: start with the given context unit
                SELECT * FROM context_units WHERE id = ?1
                UNION ALL
                -- Recursive case: follow previous_version_id
                SELECT cu.*
                FROM context_units cu
                INNER JOIN version_chain vc ON cu.id = vc.previous_version_id
            )
            SELECT id, type, source, timestamp, embedding_id, summary, version, previous_version_id, aggregate_score, feedback_count
            FROM version_chain
            ORDER BY version ASC";

        let mut stmt = db.prepare(sql)?;

        let results: Vec<ContextUnit> = stmt
            .query_map([context_unit_id], |row| {
                Ok(ContextUnit {
                    id: row.get(0)?,
                    r#type: match row.get::<_, String>(1)?.as_str() {
                        "System" => crate::context::types::ContextType::System,
                        "User" => crate::context::types::ContextType::User,
                        "External" => crate::context::types::ContextType::External,
                        "ModelState" => crate::context::types::ContextType::ModelState,
                        _ => crate::context::types::ContextType::User,
                    },
                    source: row.get(2)?,
                    timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    embedding_id: row.get(4)?,
                    summary: row.get(5)?,
                    version: row.get(6)?,
                    previous_version_id: row.get(7)?,
                    aggregate_score: row.get(8)?,
                    feedback_count: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    async fn get_responses_for_context(
        &self,
        context_unit_id: &str,
        min_weight: Option<f32>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<crate::context::types::ResponseSummary>, StorageError> {
        let db = self.db.lock().await;

        // Build the query dynamically based on filters
        let mut sql = String::from(
            "SELECT r.id, r.timestamp, r.agent, l.weight
             FROM responses r
             INNER JOIN lineage l ON r.id = l.response_id
             WHERE l.context_unit_id = ?1",
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(context_unit_id.to_string())];

        if let Some(min_w) = min_weight {
            sql.push_str(" AND l.weight >= ?");
            params.push(Box::new(min_w));
        }

        if let Some(start) = start_date {
            sql.push_str(" AND r.timestamp >= ?");
            params.push(Box::new(start.to_rfc3339()));
        }

        if let Some(end) = end_date {
            sql.push_str(" AND r.timestamp <= ?");
            params.push(Box::new(end.to_rfc3339()));
        }

        sql.push_str(" ORDER BY l.weight DESC");

        if let Some(lim) = limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(lim as i64));
        }

        let mut stmt = db.prepare(&sql)?;

        let param_refs: Vec<&dyn rusqlite::ToSql> = params
            .iter()
            .map(|p| p.as_ref() as &dyn rusqlite::ToSql)
            .collect();

        let results: Vec<crate::context::types::ResponseSummary> = stmt
            .query_map(&param_refs[..], |row| {
                Ok(crate::context::types::ResponseSummary {
                    response_id: row.get(0)?,
                    timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                    agent: row.get(2)?,
                    weight: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    async fn cleanup_old_data(&self, retention_days: u32) -> Result<usize, StorageError> {
        let db = self.db.lock().await;
        cleanup_old_data(&db, retention_days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_schema() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        initialize_schema(&conn).unwrap();

        // Verify tables exist
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"context_units".to_string()));
        assert!(tables.contains(&"responses".to_string()));
        assert!(tables.contains(&"lineage".to_string()));
        assert!(tables.contains(&"lineage_manifests".to_string()));
        assert!(tables.contains(&"feedback".to_string()));
    }

    #[test]
    fn test_schema_wal_mode() {
        // Note: WAL mode is not available for in-memory databases
        // Test with a temporary file instead
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_wal.db");

        // Clean up any existing test database
        let _ = std::fs::remove_file(&db_path);

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        initialize_schema(&conn).unwrap();

        // Verify WAL mode is enabled
        let journal_mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_lowercase(), "wal");

        // Clean up
        drop(conn);
        let _ = std::fs::remove_file(&db_path);
    }
}
