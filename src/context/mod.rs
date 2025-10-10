//! Context Tracing Framework
//!
//! This module implements AI context provenance and evolution tracking for the MCP Rust Proxy.
//! It provides functionality to track which context units (sources of information) influenced
//! each AI response, enabling transparency, auditability, and continuous improvement.
//!
//! # Architecture
//!
//! The context tracing framework uses a hybrid storage approach:
//! - **DashMap**: In-memory cache for hot data (recent responses)
//! - **SQLite**: Persistent storage for complete lineage history
//!
//! # Core Components
//!
//! - [`types`]: Core data types (ContextUnit, Response, LineageManifest, FeedbackRecord)
//! - [`storage`]: Storage backend with hybrid DashMap + SQLite implementation
//! - [`tracker`]: Runtime hooks for capturing context during response generation
//! - [`query`]: Query interface for lineage data retrieval
//! - [`evolution`]: Context versioning and evolution tracking
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mcp_rust_proxy::context::tracker::ContextTracker;
//! use mcp_rust_proxy::context::types::{ContextUnit, ContextType};
//! use mcp_rust_proxy::context::storage::{HybridStorage, StorageBackend};
//! use std::sync::Arc;
//! use std::path::PathBuf;
//! use chrono::Utc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize storage backend
//! let storage = HybridStorage::new(PathBuf::from("context.db"), None).await?;
//! let storage: Arc<dyn StorageBackend> = Arc::new(storage);
//!
//! // Initialize tracker
//! let tracker = ContextTracker::new(storage);
//!
//! // Start tracking a response
//! let response_id = tracker.start_response("agent".to_string(), "model".to_string()).await?;
//!
//! // Create and add context units as they're retrieved
//! let context_unit = ContextUnit {
//!     id: "ctx_1".to_string(),
//!     r#type: ContextType::User,
//!     source: "chat_history".to_string(),
//!     timestamp: Utc::now(),
//!     embedding_id: None,
//!     summary: Some("User message".to_string()),
//!     version: 1,
//!     previous_version_id: None,
//!     aggregate_score: 0.0,
//!     feedback_count: 0,
//! };
//! tracker.add_context(response_id.clone(), context_unit, Some(0.8)).await?;
//!
//! // Finalize and generate lineage manifest
//! let manifest = tracker.finalize_response(response_id, Some(1984)).await?;
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod evolution;
pub mod query;
pub mod storage;
pub mod tracker;
pub mod types;
