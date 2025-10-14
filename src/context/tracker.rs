//! Context tracker for capturing runtime context during response generation
//!
//! This module provides the ContextTracker which monitors which context units
//! (sources of information) are used during AI response generation and calculates
//! their contribution weights using a multi-factor scoring algorithm.

use crate::context::types::{ContextReference, ContextType, ContextUnit};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Multi-factor weight calculation for context contributions
///
/// This implements the composite scoring algorithm that determines how much
/// each context unit contributed to a response based on:
/// - Retrieval score (0.4 weight) - How relevant the context was to the query
/// - Recency (0.3 weight) - How fresh the context is
/// - Type priority (0.2 weight) - System context weighted higher than external
/// - Length factor (0.1 weight) - Longer contexts may be more informative
///
/// # Algorithm
///
/// For each context unit:
/// 1. Calculate retrieval_score (normalized 0-1)
/// 2. Calculate recency_score based on age: score = e^(-age_hours / 168)
/// 3. Calculate type_score based on ContextType priority
/// 4. Calculate length_score: normalized by max length in batch
/// 5. Composite = 0.4*retrieval + 0.3*recency + 0.2*type + 0.1*length
/// 6. Normalize all weights to sum to exactly 1.0
#[derive(Debug, Clone)]
pub struct WeightCalculator {
    /// Factor weights for composite scoring
    retrieval_weight: f32,
    recency_weight: f32,
    type_weight: f32,
    length_weight: f32,
}

impl Default for WeightCalculator {
    fn default() -> Self {
        Self {
            retrieval_weight: 0.4,
            recency_weight: 0.3,
            type_weight: 0.2,
            length_weight: 0.1,
        }
    }
}

impl WeightCalculator {
    /// Create a new weight calculator with custom factor weights
    ///
    /// # Arguments
    /// * `retrieval_weight` - Weight for retrieval relevance (default 0.4)
    /// * `recency_weight` - Weight for recency (default 0.3)
    /// * `type_weight` - Weight for context type priority (default 0.2)
    /// * `length_weight` - Weight for content length (default 0.1)
    ///
    /// # Panics
    /// Panics if weights don't sum to 1.0 (±0.01)
    pub fn new(
        retrieval_weight: f32,
        recency_weight: f32,
        type_weight: f32,
        length_weight: f32,
    ) -> Self {
        let sum = retrieval_weight + recency_weight + type_weight + length_weight;
        assert!(
            (sum - 1.0).abs() < 0.01,
            "Factor weights must sum to 1.0, got {}",
            sum
        );

        Self {
            retrieval_weight,
            recency_weight,
            type_weight,
            length_weight,
        }
    }

    /// Calculate weights for a batch of context units
    ///
    /// # Arguments
    /// * `contexts` - List of (context_unit, retrieval_score) pairs
    /// * `current_time` - Current timestamp for recency calculation
    ///
    /// # Returns
    /// Vec of ContextReference with normalized weights summing to 1.0
    pub fn calculate_weights(
        &self,
        contexts: &[(ContextUnit, f32)],
        current_time: DateTime<Utc>,
    ) -> Vec<ContextReference> {
        if contexts.is_empty() {
            return Vec::new();
        }

        // Calculate max length for normalization
        let max_length = contexts
            .iter()
            .map(|(ctx, _)| ctx.summary.as_ref().map(|s| s.len()).unwrap_or(0))
            .max()
            .unwrap_or(1) as f32;

        // Calculate composite scores for each context
        let scores: Vec<(String, f32)> = contexts
            .iter()
            .map(|(ctx, retrieval_score)| {
                let recency_score = self.calculate_recency_score(&ctx.timestamp, current_time);
                let type_score = self.calculate_type_score(ctx.r#type);
                let length_score = self.calculate_length_score(ctx, max_length);

                let composite_score = self.retrieval_weight * retrieval_score
                    + self.recency_weight * recency_score
                    + self.type_weight * type_score
                    + self.length_weight * length_score;

                (ctx.id.clone(), composite_score)
            })
            .collect();

        // Normalize weights to sum to 1.0
        let total_score: f32 = scores.iter().map(|(_, score)| score).sum();

        if total_score > 0.0 {
            scores
                .iter()
                .map(|(id, score)| ContextReference {
                    context_unit_id: id.clone(),
                    weight: score / total_score,
                })
                .collect()
        } else {
            // Fallback: equal weights if all scores are 0
            let equal_weight = 1.0 / contexts.len() as f32;
            contexts
                .iter()
                .map(|(ctx, _)| ContextReference {
                    context_unit_id: ctx.id.clone(),
                    weight: equal_weight,
                })
                .collect()
        }
    }

    /// Calculate recency score using exponential decay
    ///
    /// score = e^(-age_hours / 168)
    /// - 168 hours = 1 week half-life
    /// - Score ranges from 1.0 (brand new) to ~0.0 (very old)
    fn calculate_recency_score(
        &self,
        context_time: &DateTime<Utc>,
        current_time: DateTime<Utc>,
    ) -> f32 {
        let age = current_time.signed_duration_since(*context_time);
        let age_hours = age.num_hours() as f32;

        // Exponential decay with 1-week half-life
        let decay_constant = 168.0; // hours
        (-age_hours / decay_constant).exp()
    }

    /// Calculate type priority score
    ///
    /// Priority order (highest to lowest):
    /// - System: 1.0 (tool definitions, schemas)
    /// - ModelState: 0.8 (prompts, parameters)
    /// - User: 0.6 (chat history, memory)
    /// - External: 0.4 (web results, API responses)
    fn calculate_type_score(&self, context_type: ContextType) -> f32 {
        match context_type {
            ContextType::System => 1.0,
            ContextType::ModelState => 0.8,
            ContextType::User => 0.6,
            ContextType::External => 0.4,
        }
    }

    /// Calculate length score (normalized by max length in batch)
    ///
    /// Longer summaries may contain more information, but diminishing returns
    fn calculate_length_score(&self, context: &ContextUnit, max_length: f32) -> f32 {
        if max_length <= 0.0 {
            return 0.5;
        }

        let length = context.summary.as_ref().map(|s| s.len()).unwrap_or(0) as f32;

        // Normalize to 0-1 range
        length / max_length
    }
}

/// Context tracking state for a single response being generated
#[derive(Debug)]
pub struct ResponseTracking {
    /// Response ID
    pub response_id: String,
    /// Agent identifier
    pub agent: String,
    /// Model identifier
    pub model: String,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// Context units with retrieval scores
    pub contexts: HashMap<String, (ContextUnit, f32)>,
}

impl ResponseTracking {
    /// Create a new response tracking session
    pub fn new(response_id: String, agent: String, model: String) -> Self {
        Self {
            response_id,
            agent,
            model,
            started_at: Utc::now(),
            contexts: HashMap::new(),
        }
    }

    /// Add a context unit to this response with its retrieval score
    ///
    /// # Arguments
    /// * `context` - The context unit that was used
    /// * `retrieval_score` - How relevant this context was (0.0 to 1.0)
    pub fn add_context(&mut self, context: ContextUnit, retrieval_score: f32) {
        self.contexts
            .insert(context.id.clone(), (context, retrieval_score));
    }

    /// Calculate final weights for all tracked contexts
    pub fn calculate_weights(&self) -> Vec<ContextReference> {
        let calculator = WeightCalculator::default();
        let contexts: Vec<_> = self.contexts.values().cloned().collect();
        calculator.calculate_weights(&contexts, Utc::now())
    }
}

// ========== Context Tracker (Main Interface) ==========

use crate::context::storage::StorageBackend;
use crate::context::types::{
    ContextTreeNode, LineageManifest, ProvenanceEdge, ProvenanceTree, Response,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Main context tracking coordinator
///
/// This struct provides the public API for tracking context usage during
/// AI response generation. It manages active response tracking sessions
/// and persists lineage data to storage.
///
/// # Thread Safety
/// ContextTracker is thread-safe and can be shared across async tasks using Arc.
pub struct ContextTracker {
    /// Storage backend for persistence
    storage: Arc<dyn StorageBackend>,
    /// Active tracking sessions (response_id -> ResponseTracking)
    active_sessions: Arc<RwLock<HashMap<String, ResponseTracking>>>,
}

impl ContextTracker {
    /// Create a new context tracker
    ///
    /// # Arguments
    /// * `storage` - Storage backend for persisting lineage data
    ///
    /// # Returns
    /// New ContextTracker instance
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self {
            storage,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start tracking a new response
    ///
    /// Generates a unique response ID and initializes a tracking session.
    ///
    /// # Arguments
    /// * `agent` - Agent identifier (e.g., "mcp-proxy", "claude-assistant")
    /// * `model` - Model identifier (e.g., "claude-3-5-sonnet-20241022")
    ///
    /// # Returns
    /// * `Ok(response_id)` with the generated response ID (format: "resp_{uuid}")
    /// * `Err(String)` if initialization fails
    pub async fn start_response(&self, agent: String, model: String) -> Result<String, String> {
        let response_id = format!("resp_{}", Uuid::new_v4());

        let session = ResponseTracking::new(response_id.clone(), agent, model);

        let mut sessions = self.active_sessions.write().await;
        sessions.insert(response_id.clone(), session);

        Ok(response_id)
    }

    /// Add a context unit to an active response tracking session
    ///
    /// # Arguments
    /// * `response_id` - The response ID from start_response()
    /// * `context` - The context unit that was used
    /// * `retrieval_score` - Relevance score (0.0 to 1.0, default 0.5 if unknown)
    ///
    /// # Returns
    /// * `Ok(())` if context added successfully
    /// * `Err(String)` if response_id not found or context invalid
    pub async fn add_context(
        &self,
        response_id: String,
        context: ContextUnit,
        retrieval_score: Option<f32>,
    ) -> Result<(), String> {
        // Validate context unit
        context
            .validate()
            .map_err(|e| format!("Invalid context unit: {}", e))?;

        let score = retrieval_score.unwrap_or(0.5).clamp(0.0, 1.0);

        let mut sessions = self.active_sessions.write().await;
        let session = sessions
            .get_mut(&response_id)
            .ok_or_else(|| format!("Response {} not found", response_id))?;

        // Store context unit to storage
        self.storage
            .store_context_unit(&context)
            .await
            .map_err(|e| format!("Failed to store context unit: {}", e))?;

        session.add_context(context, score);

        Ok(())
    }

    /// Finalize response tracking and generate lineage manifest
    ///
    /// Calculates final weights, generates provenance tree, persists to storage,
    /// and removes the tracking session.
    ///
    /// # Arguments
    /// * `response_id` - The response ID to finalize
    /// * `token_count` - Optional token count for the response
    ///
    /// # Returns
    /// * `Ok(LineageManifest)` with the generated manifest
    /// * `Err(String)` if finalization fails
    pub async fn finalize_response(
        &self,
        response_id: String,
        token_count: Option<i32>,
    ) -> Result<LineageManifest, String> {
        // Remove session from active tracking
        let session = {
            let mut sessions = self.active_sessions.write().await;
            sessions
                .remove(&response_id)
                .ok_or_else(|| format!("Response {} not found", response_id))?
        };

        // Calculate final weights
        let context_refs = session.calculate_weights();

        // Validate we have at least one context
        if context_refs.is_empty() {
            return Err("Response must have at least one context unit".to_string());
        }

        // Create Response record
        let response = Response {
            id: response_id.clone(),
            timestamp: Utc::now(),
            agent: session.agent.clone(),
            model: session.model.clone(),
            token_count,
            context_units: context_refs.clone(),
        };

        // Validate response
        response
            .validate()
            .map_err(|e| format!("Invalid response: {}", e))?;

        // Store response
        self.storage
            .store_response(&response)
            .await
            .map_err(|e| format!("Failed to store response: {}", e))?;

        // Build lineage manifest
        let manifest = self.build_lineage_manifest(&response, &session).await?;

        // Validate manifest size
        let manifest_json = serde_json::to_string(&manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

        if manifest_json.len() > 5 * 1024 {
            return Err(format!(
                "Manifest too large: {} bytes (max 5KB)",
                manifest_json.len()
            ));
        }

        // Store lineage manifest
        self.storage
            .store_lineage(&manifest)
            .await
            .map_err(|e| format!("Failed to store lineage: {}", e))?;

        Ok(manifest)
    }

    /// Build lineage manifest from response and tracking session
    async fn build_lineage_manifest(
        &self,
        response: &Response,
        session: &ResponseTracking,
    ) -> Result<LineageManifest, String> {
        // Build context tree nodes
        let mut context_tree = Vec::new();
        for (ctx, _score) in session.contexts.values() {
            context_tree.push(ContextTreeNode {
                id: ctx.id.clone(),
                r#type: ctx.r#type,
                source: ctx.source.clone(),
                weight: response
                    .context_units
                    .iter()
                    .find(|r| r.context_unit_id == ctx.id)
                    .map(|r| r.weight)
                    .unwrap_or(0.0),
                embedding_id: ctx.embedding_id.clone(),
                summary: ctx.summary.clone(),
            });
        }

        // Build provenance tree edges (response -> context units)
        let edges: Vec<ProvenanceEdge> = response
            .context_units
            .iter()
            .map(|ctx_ref| ProvenanceEdge {
                from: response.id.clone(),
                to: ctx_ref.context_unit_id.clone(),
                weight: ctx_ref.weight,
            })
            .collect();

        let provenance_tree = ProvenanceTree {
            root: response.id.clone(),
            edges,
        };

        Ok(LineageManifest {
            response_id: response.id.clone(),
            timestamp: response.timestamp,
            agent: response.agent.clone(),
            model: response.model.clone(),
            token_count: response.token_count,
            context_tree,
            provenance_tree,
        })
    }

    /// Get storage backend (for testing/introspection)
    pub fn storage(&self) -> Arc<dyn StorageBackend> {
        self.storage.clone()
    }

    /// Record feedback for a response and propagate to context units
    ///
    /// # Arguments
    /// * `response_id` - The response to provide feedback on
    /// * `score` - Feedback score (-1.0 to 1.0)
    /// * `feedback_text` - Optional feedback comment
    /// * `user_id` - Optional user identifier
    ///
    /// # Returns
    /// * `Ok(FeedbackPropagationStatus)` with propagation statistics
    /// * `Err(String)` if recording or propagation fails
    pub async fn record_feedback(
        &self,
        response_id: &str,
        score: f32,
        feedback_text: Option<String>,
        user_id: Option<String>,
    ) -> Result<crate::context::types::FeedbackPropagationStatus, String> {
        use crate::context::types::FeedbackRecord;

        // Validate score range
        if score < -1.0 || score > 1.0 {
            return Err(format!("Score must be in range [-1.0, 1.0], got {}", score));
        }

        // Get the response to find its contributing contexts
        let response = self
            .storage
            .get_response(response_id)
            .await
            .map_err(|e| format!("Failed to get response: {}", e))?
            .ok_or_else(|| format!("Response {} not found", response_id))?;

        // Create feedback record
        let feedback = FeedbackRecord {
            id: format!("fb_{}", Uuid::new_v4()),
            response_id: response_id.to_string(),
            timestamp: Utc::now(),
            score,
            feedback_text,
            user_id,
        };

        // Validate and store feedback
        feedback
            .validate()
            .map_err(|e| format!("Invalid feedback: {}", e))?;

        self.storage
            .store_feedback(&feedback)
            .await
            .map_err(|e| format!("Failed to store feedback: {}", e))?;

        // Propagate feedback to all contributing context units
        let status = self.propagate_feedback(&response, score).await?;

        Ok(status)
    }

    /// Propagate feedback score to all contributing context units
    ///
    /// Updates aggregate scores using weighted average:
    /// new_score = (old_score * old_count + feedback_score * weight) / (old_count + 1)
    ///
    /// # Arguments
    /// * `response` - The response that received feedback
    /// * `feedback_score` - The feedback score to propagate
    ///
    /// # Returns
    /// * `Ok(FeedbackPropagationStatus)` with update statistics
    /// * `Err(String)` if propagation fails
    async fn propagate_feedback(
        &self,
        response: &Response,
        feedback_score: f32,
    ) -> Result<crate::context::types::FeedbackPropagationStatus, String> {
        let mut contexts_updated = 0;
        let mut total_score_change = 0.0;

        for ctx_ref in &response.context_units {
            // Get current context
            let context = self
                .storage
                .get_context_unit(&ctx_ref.context_unit_id)
                .await
                .map_err(|e| format!("Failed to get context: {}", e))?
                .ok_or_else(|| format!("Context unit {} not found", ctx_ref.context_unit_id))?;

            // Calculate new aggregate score using weighted average
            let old_score = context.aggregate_score;
            let old_count = context.feedback_count;
            let weight = ctx_ref.weight;

            let new_score = if old_count > 0 {
                (old_score * old_count as f32 + feedback_score * weight) / (old_count as f32 + 1.0)
            } else {
                feedback_score * weight
            };

            let new_count = old_count + 1;

            // Update context unit
            self.storage
                .update_context_unit(&ctx_ref.context_unit_id, new_score, new_count)
                .await
                .map_err(|e| format!("Failed to update context: {}", e))?;

            contexts_updated += 1;
            total_score_change += new_score - old_score;
        }

        let avg_score_change = if contexts_updated > 0 {
            total_score_change / contexts_updated as f32
        } else {
            0.0
        };

        Ok(crate::context::types::FeedbackPropagationStatus {
            contexts_updated,
            avg_score_change,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::types::ContextType;

    fn create_test_context(id: &str, context_type: ContextType, summary_len: usize) -> ContextUnit {
        ContextUnit {
            id: id.to_string(),
            r#type: context_type,
            source: "test".to_string(),
            timestamp: Utc::now(),
            embedding_id: None,
            summary: Some("x".repeat(summary_len)),
            version: 1,
            previous_version_id: None,
            aggregate_score: 0.0,
            feedback_count: 0,
        }
    }

    #[test]
    fn test_weight_calculator_normalization() {
        let calc = WeightCalculator::default();
        let now = Utc::now();

        let contexts = vec![
            (create_test_context("ctx1", ContextType::System, 100), 0.9),
            (create_test_context("ctx2", ContextType::User, 50), 0.7),
            (create_test_context("ctx3", ContextType::External, 25), 0.5),
        ];

        let weights = calc.calculate_weights(&contexts, now);

        // Verify sum equals 1.0
        let sum: f32 = weights.iter().map(|w| w.weight).sum();
        assert!((sum - 1.0).abs() < 0.001, "Weights sum to {}, not 1.0", sum);

        // Verify we have correct number of weights
        assert_eq!(weights.len(), 3);
    }

    #[test]
    fn test_recency_score_decay() {
        let calc = WeightCalculator::default();
        let now = Utc::now();

        // Recent context should have higher score
        let recent_score = calc.calculate_recency_score(&now, now);
        assert!((recent_score - 1.0).abs() < 0.01);

        // Week-old context should have ~0.37 score (e^-1)
        let week_ago = now - chrono::Duration::weeks(1);
        let week_score = calc.calculate_recency_score(&week_ago, now);
        assert!(week_score > 0.3 && week_score < 0.4);
    }

    #[test]
    fn test_type_priority() {
        let calc = WeightCalculator::default();

        assert_eq!(calc.calculate_type_score(ContextType::System), 1.0);
        assert_eq!(calc.calculate_type_score(ContextType::ModelState), 0.8);
        assert_eq!(calc.calculate_type_score(ContextType::User), 0.6);
        assert_eq!(calc.calculate_type_score(ContextType::External), 0.4);
    }

    #[test]
    fn test_response_tracking() {
        let mut tracking = ResponseTracking::new(
            "resp_test".to_string(),
            "test-agent".to_string(),
            "test-model".to_string(),
        );

        let ctx1 = create_test_context("ctx1", ContextType::System, 100);
        let ctx2 = create_test_context("ctx2", ContextType::User, 50);

        tracking.add_context(ctx1, 0.9);
        tracking.add_context(ctx2, 0.7);

        let weights = tracking.calculate_weights();
        assert_eq!(weights.len(), 2);

        let sum: f32 = weights.iter().map(|w| w.weight).sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_empty_contexts() {
        let calc = WeightCalculator::default();
        let weights = calc.calculate_weights(&[], Utc::now());
        assert_eq!(weights.len(), 0);
    }
}
