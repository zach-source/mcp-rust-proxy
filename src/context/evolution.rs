//! Context versioning and evolution tracking
//!
//! This module provides functionality for tracking how context units evolve over time
//! through versioning. Each context unit can have multiple versions linked by
//! `previous_version_id`, forming a version chain.

use crate::context::storage::StorageBackend;
use crate::context::types::{ContextUnit, ContextVersion, EvolutionHistory};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Service for managing context evolution and versioning
pub struct EvolutionService {
    storage: Arc<dyn StorageBackend>,
}

impl EvolutionService {
    /// Create a new evolution service
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self { storage }
    }

    /// Create a new version of an existing context unit
    ///
    /// # Arguments
    /// * `previous_context` - The existing context unit to create a new version of
    /// * `new_summary` - Updated summary for the new version
    /// * `new_source` - Updated source identifier (optional, defaults to previous)
    ///
    /// # Returns
    /// * `Ok(ContextUnit)` with the new version
    /// * `Err(String)` if creation fails
    pub async fn create_context_version(
        &self,
        previous_context: &ContextUnit,
        new_summary: Option<String>,
        new_source: Option<String>,
    ) -> Result<ContextUnit, String> {
        // Validate previous context
        previous_context
            .validate()
            .map_err(|e| format!("Invalid previous context: {e}"))?;

        // Create new version
        let new_version = ContextUnit {
            id: format!("cu_{}", Uuid::new_v4()),
            r#type: previous_context.r#type,
            source: new_source.unwrap_or_else(|| previous_context.source.clone()),
            timestamp: Utc::now(),
            embedding_id: None, // New version gets new embedding
            summary: new_summary.or_else(|| previous_context.summary.clone()),
            version: previous_context.version + 1,
            previous_version_id: Some(previous_context.id.clone()),
            aggregate_score: previous_context.aggregate_score, // Inherit score
            feedback_count: previous_context.feedback_count,   // Inherit count
        };

        // Validate new version
        new_version
            .validate()
            .map_err(|e| format!("Invalid new version: {e}"))?;

        // Ensure version is monotonically increasing
        if new_version.version <= previous_context.version {
            return Err(format!(
                "Version must be monotonically increasing: {} -> {}",
                previous_context.version, new_version.version
            ));
        }

        // Store new version
        self.storage
            .store_context_unit(&new_version)
            .await
            .map_err(|e| format!("Failed to store new version: {e}"))?;

        Ok(new_version)
    }

    /// Get complete version history for a context unit
    ///
    /// Traverses the version chain from the given context unit back to the first version.
    ///
    /// # Arguments
    /// * `context_unit_id` - Any version in the chain
    ///
    /// # Returns
    /// * `Ok(EvolutionHistory)` with current version and full history
    /// * `Err(String)` if retrieval fails
    pub async fn get_version_history(
        &self,
        context_unit_id: &str,
    ) -> Result<EvolutionHistory, String> {
        // Get the starting context
        let current = self
            .storage
            .get_context_unit(context_unit_id)
            .await
            .map_err(|e| format!("Failed to get context: {e}"))?
            .ok_or_else(|| format!("Context unit {context_unit_id} not found"))?;

        // Traverse version chain using storage method
        let chain = self
            .storage
            .get_context_version_chain(context_unit_id)
            .await
            .map_err(|e| format!("Failed to get version chain: {e}"))?;

        // Convert to ContextVersion objects
        let history: Vec<ContextVersion> = chain
            .iter()
            .map(|ctx| ContextVersion {
                id: ctx.id.clone(),
                version: ctx.version,
                timestamp: ctx.timestamp,
                summary: ctx.summary.clone(),
            })
            .collect();

        // Current version is the one we started with
        let current_version = ContextVersion {
            id: current.id.clone(),
            version: current.version,
            timestamp: current.timestamp,
            summary: current.summary.clone(),
        };

        Ok(EvolutionHistory {
            current_version,
            history,
        })
    }

    /// Compare two versions of a context unit
    ///
    /// # Arguments
    /// * `version_a_id` - First version ID
    /// * `version_b_id` - Second version ID
    ///
    /// # Returns
    /// * `Ok((version_a, version_b))` tuple for comparison
    /// * `Err(String)` if either version not found
    pub async fn compare_versions(
        &self,
        version_a_id: &str,
        version_b_id: &str,
    ) -> Result<(ContextUnit, ContextUnit), String> {
        let version_a = self
            .storage
            .get_context_unit(version_a_id)
            .await
            .map_err(|e| format!("Failed to get version A: {e}"))?
            .ok_or_else(|| format!("Version {version_a_id} not found"))?;

        let version_b = self
            .storage
            .get_context_unit(version_b_id)
            .await
            .map_err(|e| format!("Failed to get version B: {e}"))?
            .ok_or_else(|| format!("Version {version_b_id} not found"))?;

        Ok((version_a, version_b))
    }

    /// Check if a context unit is below deprecation threshold
    ///
    /// # Arguments
    /// * `context` - The context unit to check
    /// * `threshold` - Deprecation threshold (default: -0.5)
    ///
    /// # Returns
    /// * `true` if context score is below threshold
    pub fn check_deprecation_threshold(
        &self,
        context: &ContextUnit,
        threshold: Option<f32>,
    ) -> bool {
        let threshold = threshold.unwrap_or(-0.5);
        context.feedback_count > 0 && context.aggregate_score < threshold
    }

    /// Get all deprecated contexts (score below threshold)
    ///
    /// # Arguments
    /// * `threshold` - Deprecation threshold (default: -0.5)
    ///
    /// # Returns
    /// * `Ok(Vec<ContextUnit>)` with all deprecated contexts
    /// * `Err(String)` if query fails
    pub async fn get_deprecated_contexts(
        &self,
        _threshold: Option<f32>,
    ) -> Result<Vec<ContextUnit>, String> {
        // Note: This would require a new storage method to query by score
        // For now, return empty vec as placeholder
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::types::ContextType;

    // Helper to create test context
    fn create_test_context(id: &str, version: i32, prev_id: Option<String>) -> ContextUnit {
        ContextUnit {
            id: id.to_string(),
            r#type: ContextType::User,
            source: "test".to_string(),
            timestamp: Utc::now(),
            embedding_id: None,
            summary: Some(format!("Version {version}")),
            version,
            previous_version_id: prev_id,
            aggregate_score: 0.0,
            feedback_count: 0,
        }
    }

    #[test]
    fn test_version_validation() {
        let v1 = create_test_context("ctx_v1", 1, None);
        assert!(v1.validate().is_ok());

        let v2 = create_test_context("ctx_v2", 2, Some("ctx_v1".to_string()));
        assert!(v2.validate().is_ok());
        assert_eq!(v2.version, 2);
        assert_eq!(v2.previous_version_id, Some("ctx_v1".to_string()));
    }
}
