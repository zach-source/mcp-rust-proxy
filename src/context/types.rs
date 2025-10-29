//! Core type definitions for context tracing
//!
//! This module defines the fundamental data types used throughout the context tracing framework:
//! - Context units (sources of information)
//! - Responses (AI-generated outputs)
//! - Lineage manifests (provenance records)
//! - Feedback records (quality evaluations)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Context type categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextType {
    /// Tool definitions, MCP plugins, namespaces
    System,
    /// Chat history, project data, memory
    User,
    /// Web results, MCP tool outputs
    External,
    /// Temperature, system prompt, agent goals
    ModelState,
}

/// A discrete piece of information used in generating AI responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUnit {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Category of context
    #[serde(rename = "type")]
    pub r#type: ContextType,
    /// Origin identifier (max 255 chars)
    pub source: String,
    /// Creation time
    pub timestamp: DateTime<Utc>,
    /// Vector DB reference
    pub embedding_id: Option<String>,
    /// Human-readable description (max 500 chars)
    pub summary: Option<String>,
    /// Version number (starts at 1)
    pub version: i32,
    /// Link to previous version
    pub previous_version_id: Option<String>,

    /// Runtime only - aggregate quality score
    #[serde(skip)]
    pub aggregate_score: f32,
    /// Runtime only - number of feedback entries
    #[serde(skip)]
    pub feedback_count: i32,
}

/// Reference to a context unit with its contribution weight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextReference {
    /// UUID of the context unit
    pub context_unit_id: String,
    /// Contribution weight (0.0 to 1.0)
    pub weight: f32,
}

/// An AI-generated output with provenance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Unique identifier (UUID v4, prefixed with `resp_`)
    pub id: String,
    /// Generation time
    pub timestamp: DateTime<Utc>,
    /// Agent identifier
    pub agent: String,
    /// Model version
    pub model: String,
    /// Response size in tokens
    pub token_count: Option<i32>,
    /// Contributing context units
    pub context_units: Vec<ContextReference>,
}

/// Node in the context tree (for lineage manifest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTreeNode {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: ContextType,
    pub source: String,
    pub weight: f32,
    pub embedding_id: Option<String>,
    pub summary: Option<String>,
}

/// Edge in the provenance tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEdge {
    pub from: String,
    pub to: String,
    pub weight: f32,
}

/// Provenance tree showing relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceTree {
    pub root: String,
    pub edges: Vec<ProvenanceEdge>,
}

/// Complete lineage manifest for a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageManifest {
    pub response_id: String,
    pub timestamp: DateTime<Utc>,
    pub agent: String,
    pub model: String,
    pub token_count: Option<i32>,
    pub context_tree: Vec<ContextTreeNode>,
    pub provenance_tree: ProvenanceTree,
}

/// User or system evaluation of response quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    /// Unique identifier
    pub id: String,
    /// Target response
    pub response_id: String,
    /// Feedback submission time
    pub timestamp: DateTime<Utc>,
    /// Quality rating (-1.0 to 1.0)
    pub score: f32,
    /// Optional comment
    pub feedback_text: Option<String>,
    /// User identifier
    pub user_id: Option<String>,
}

/// Feedback submission request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackSubmission {
    pub response_id: String,
    pub score: f32,
    pub feedback_text: Option<String>,
    pub user_id: Option<String>,
}

/// Feedback propagation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackPropagationStatus {
    pub contexts_updated: usize,
    pub avg_score_change: f32,
}

/// Context version for evolution tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextVersion {
    pub id: String,
    pub version: i32,
    pub timestamp: DateTime<Utc>,
    pub summary: Option<String>,
}

/// Evolution history of a context unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionHistory {
    pub current_version: ContextVersion,
    pub history: Vec<ContextVersion>,
}

/// Context impact report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextImpactReport {
    pub context_unit_id: String,
    pub total_responses: usize,
    pub avg_weight: f32,
    pub responses: Vec<ResponseSummary>,
}

/// Summary of a response (for impact reports)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSummary {
    pub response_id: String,
    pub timestamp: DateTime<Utc>,
    pub weight: f32,
    pub agent: String,
}

/// Session tracking for grouping related responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: String,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Session end time (None if still active)
    pub ended_at: Option<DateTime<Utc>>,
    /// Original user query/request
    pub user_query: Option<String>,
    /// Agent identifier
    pub agent: String,
    /// Session-level metadata
    pub metadata: Option<String>,
    /// Overall session score (average of task scores)
    pub session_score: Option<f32>,
}

/// Task within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: String,
    /// Session this task belongs to
    pub session_id: String,
    /// Task description
    pub description: String,
    /// Task status
    pub status: TaskStatus,
    /// When task was created
    pub created_at: DateTime<Utc>,
    /// When task was completed (None if not done)
    pub completed_at: Option<DateTime<Utc>>,
    /// Response IDs associated with this task
    pub response_ids: Vec<String>,
    /// Task quality score
    pub task_score: Option<f32>,
}

/// Task status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending
    Pending,
    /// Task is in progress
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task cancelled
    Cancelled,
}

/// Session summary with tasks and responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session: Session,
    pub tasks: Vec<Task>,
    pub total_responses: usize,
    pub total_contexts: usize,
    pub average_score: f32,
}

impl ContextUnit {
    /// Validate context unit constraints
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Context unit ID cannot be empty".to_string());
        }
        if self.source.is_empty() || self.source.len() > 255 {
            return Err("Source must be 1-255 characters".to_string());
        }
        if self.version < 1 {
            return Err("Version must be ≥ 1".to_string());
        }
        if let Some(summary) = &self.summary {
            if summary.len() > 500 {
                return Err("Summary must be ≤ 500 characters".to_string());
            }
        }
        Ok(())
    }
}

impl Response {
    /// Validate response constraints
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Response ID cannot be empty".to_string());
        }
        if !self.id.starts_with("resp_") {
            return Err("Response ID must start with 'resp_'".to_string());
        }
        if self.context_units.is_empty() {
            return Err("Response must have at least 1 context unit".to_string());
        }
        if self.context_units.len() > 50 {
            return Err("Response cannot have more than 50 context units".to_string());
        }

        // Validate weight sum
        let total_weight: f32 = self.context_units.iter().map(|c| c.weight).sum();
        if (total_weight - 1.0).abs() > 0.01 {
            return Err(format!(
                "Context weights must sum to 1.0 (got {total_weight})"
            ));
        }

        Ok(())
    }
}

impl FeedbackRecord {
    /// Validate feedback constraints
    pub fn validate(&self) -> Result<(), String> {
        if self.score < -1.0 || self.score > 1.0 {
            return Err("Feedback score must be in range [-1.0, 1.0]".to_string());
        }
        if self.response_id.is_empty() {
            return Err("Response ID is required".to_string());
        }
        if let Some(text) = &self.feedback_text {
            if text.len() > 1000 {
                return Err("Feedback text must be ≤ 1000 characters".to_string());
            }
        }
        Ok(())
    }
}

/// Normalize weights to sum to exactly 1.0
pub fn normalize_weights(weights: &mut [f32]) {
    let total: f32 = weights.iter().sum();
    if total > 0.0 {
        for weight in weights.iter_mut() {
            *weight /= total;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_unit_validation() {
        let mut cu = ContextUnit {
            id: "cu_test".to_string(),
            r#type: ContextType::User,
            source: "test".to_string(),
            timestamp: Utc::now(),
            embedding_id: None,
            summary: None,
            version: 1,
            previous_version_id: None,
            aggregate_score: 0.0,
            feedback_count: 0,
        };

        assert!(cu.validate().is_ok());

        cu.version = 0;
        assert!(cu.validate().is_err());
    }

    #[test]
    fn test_normalize_weights() {
        let mut weights = vec![0.2, 0.3, 0.5];
        normalize_weights(&mut weights);
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }
}

// ============================================================================
// Claude API Proxy Data Models
// ============================================================================

/// Type of context source in Claude API requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    /// Direct user input
    User,
    /// System prompt from Claude Code framework
    Framework,
    /// Tool result from MCP server
    McpServer,
    /// Skills like vectorize
    Skill,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::User => write!(f, "User"),
            SourceType::Framework => write!(f, "Framework"),
            SourceType::McpServer => write!(f, "McpServer"),
            SourceType::Skill => write!(f, "Skill"),
        }
    }
}

/// Captured Claude API request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedRequest {
    /// Unique identifier (UUID)
    pub id: String,
    /// When the proxy received the request
    pub timestamp: DateTime<Utc>,
    /// Full URL (e.g., "https://api.anthropic.com/v1/messages")
    pub url: String,
    /// HTTP method (typically "POST")
    pub method: String,
    /// HTTP headers (sanitized - no API keys)
    pub headers: std::collections::HashMap<String, String>,
    /// Raw request body
    pub body: Vec<u8>,
    /// Parsed JSON for analysis
    pub body_json: serde_json::Value,
    /// Total token count for request
    pub total_tokens: usize,
    /// Links to corresponding CapturedResponse
    pub correlation_id: String,
}

/// Captured Claude API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedResponse {
    /// Unique identifier (UUID)
    pub id: String,
    /// Links to corresponding CapturedRequest
    pub correlation_id: String,
    /// When the proxy received the response from Claude API
    pub timestamp: DateTime<Utc>,
    /// HTTP status code
    pub status_code: u16,
    /// HTTP response headers
    pub headers: std::collections::HashMap<String, String>,
    /// Raw response body
    pub body: Vec<u8>,
    /// Parsed JSON response
    pub body_json: serde_json::Value,
    /// Time between request forward and response receipt (ms)
    pub latency_ms: u64,
    /// Additional latency added by proxy operations (ms)
    pub proxy_latency_ms: u64,
    /// Token count for response (from API usage field)
    pub response_tokens: usize,
}

/// Context source attribution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAttribution {
    /// Unique identifier (UUID)
    pub id: String,
    /// Links to CapturedRequest
    pub request_id: String,
    /// Type of source
    pub source_type: SourceType,
    /// Source name (e.g., "context7", "serena", "vectorize")
    pub source_name: Option<String>,
    /// Number of tokens from this source
    pub token_count: usize,
    /// SHA-256 hash for deduplication
    pub content_hash: String,
    /// Position in messages array (0-based)
    pub message_index: usize,
    /// Message role from API request
    pub message_role: String,
}

/// User-submitted quality feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFeedback {
    /// Unique identifier (UUID)
    pub id: String,
    /// Links to CapturedRequest
    pub request_id: String,
    /// Links to CapturedResponse
    pub response_id: String,
    /// Quality score (-1.0 to 1.0)
    pub rating: f64,
    /// Optional user comment
    pub feedback_text: Option<String>,
    /// User identifier
    pub user_id: String,
    /// When feedback was submitted
    pub submitted_at: DateTime<Utc>,
}

/// Aggregated statistics for a context source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSourceMetrics {
    /// Source name (e.g., "context7", "serena")
    pub source_name: String,
    /// Source type
    pub source_type: SourceType,
    /// Number of requests using this source
    pub usage_count: usize,
    /// Total tokens contributed across all requests
    pub total_tokens: usize,
    /// Average tokens per request
    pub average_tokens: f64,
    /// Number of feedback submissions for this source
    pub feedback_count: usize,
    /// Average quality rating (-1.0 to 1.0)
    pub average_rating: f64,
    /// Most recent request using this source
    pub last_used: DateTime<Utc>,
    /// First request using this source
    pub created_at: DateTime<Utc>,
}

impl QualityFeedback {
    /// Validate feedback constraints
    pub fn validate(&self) -> Result<(), String> {
        if self.rating < -1.0 || self.rating > 1.0 {
            return Err("Feedback rating must be in range [-1.0, 1.0]".to_string());
        }
        if self.request_id.is_empty() {
            return Err("Request ID is required".to_string());
        }
        if self.response_id.is_empty() {
            return Err("Response ID is required".to_string());
        }
        if let Some(text) = &self.feedback_text {
            if text.len() > 2000 {
                return Err("Feedback text must be ≤ 2000 characters".to_string());
            }
        }
        Ok(())
    }
}

impl ContextAttribution {
    /// Validate attribution constraints
    pub fn validate(&self) -> Result<(), String> {
        if self.request_id.is_empty() {
            return Err("Request ID is required".to_string());
        }
        if matches!(self.source_type, SourceType::McpServer | SourceType::Skill)
            && self.source_name.is_none()
        {
            return Err("Source name required for McpServer and Skill types".to_string());
        }
        if self.token_count == 0 {
            return Err("Token count must be > 0".to_string());
        }
        Ok(())
    }
}
