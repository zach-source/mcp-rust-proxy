//! Query interface and formatting utilities for lineage data
//!
//! This module provides utilities for querying and formatting lineage manifests
//! in different output formats (JSON, tree visualization, compact summary).

use crate::context::types::LineageManifest;
use serde_json;

/// Output format for lineage manifests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Raw JSON format
    Json,
    /// ASCII tree visualization
    Tree,
    /// Compact summary with key statistics
    Compact,
}

impl OutputFormat {
    /// Parse output format from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "tree" => Some(Self::Tree),
            "compact" => Some(Self::Compact),
            _ => None,
        }
    }
}

/// Format lineage manifest as JSON string
///
/// Returns the raw LineageManifest serialized as pretty-printed JSON.
///
/// # Arguments
/// * `manifest` - The lineage manifest to format
///
/// # Returns
/// * `Ok(String)` with JSON representation
/// * `Err(String)` if serialization fails
pub fn format_as_json(manifest: &LineageManifest) -> Result<String, String> {
    serde_json::to_string_pretty(manifest).map_err(|e| format!("JSON serialization failed: {e}"))
}

/// Format lineage manifest as ASCII tree visualization
///
/// Creates a hierarchical tree showing the response and its contributing contexts.
///
/// # Example Output
/// ```text
/// Response: resp_abc123
/// ├── Agent: mcp-proxy
/// ├── Model: claude-3-5-sonnet
/// ├── Tokens: 1984
/// └── Contexts (3):
///     ├── [45%] ctx_1 (System) - MCP tool definitions
///     ├── [35%] ctx_2 (User) - Chat history
///     └── [20%] ctx_3 (External) - Web search results
/// ```
pub fn format_as_tree(manifest: &LineageManifest) -> Result<String, String> {
    let mut output = String::new();

    // Header
    output.push_str(&format!("Response: {}\n", manifest.response_id));
    output.push_str(&format!("├── Agent: {}\n", manifest.agent));
    output.push_str(&format!("├── Model: {}\n", manifest.model));
    output.push_str(&format!(
        "├── Tokens: {}\n",
        manifest
            .token_count
            .map(|t| t.to_string())
            .unwrap_or_else(|| "N/A".to_string())
    ));
    output.push_str(&format!(
        "└── Contexts ({}):\n",
        manifest.context_tree.len()
    ));

    // Context tree (sorted by weight descending)
    let mut contexts = manifest.context_tree.clone();
    contexts.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());

    for (i, ctx) in contexts.iter().enumerate() {
        let is_last = i == contexts.len() - 1;
        let prefix = if is_last { "└──" } else { "├──" };

        let summary = ctx
            .summary
            .as_ref()
            .map(|s| {
                if s.len() > 50 {
                    format!("{s:.50}...")
                } else {
                    s.clone()
                }
            })
            .unwrap_or_else(|| ctx.source.clone());

        output.push_str(&format!(
            "    {} [{:3.0}%] {} ({:?}) - {}\n",
            prefix,
            ctx.weight * 100.0,
            ctx.id,
            ctx.r#type,
            summary
        ));
    }

    Ok(output)
}

/// Format lineage manifest as compact summary
///
/// Returns a concise summary with key statistics.
///
/// # Example Output
/// ```text
/// Response resp_abc123 (agent: mcp-proxy, model: claude-3-5-sonnet)
/// Generated: 2025-10-09T12:34:56Z
/// Tokens: 1984
/// Contexts: 3 total (System: 1, User: 1, External: 1)
/// Top contributors:
///   - ctx_1 (45.0%): MCP tool definitions
///   - ctx_2 (35.0%): Chat history
///   - ctx_3 (20.0%): Web search results
/// ```
pub fn format_as_compact(manifest: &LineageManifest) -> Result<String, String> {
    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "Response {} (agent: {}, model: {})\n",
        manifest.response_id, manifest.agent, manifest.model
    ));
    output.push_str(&format!("Generated: {}\n", manifest.timestamp.to_rfc3339()));
    output.push_str(&format!(
        "Tokens: {}\n",
        manifest
            .token_count
            .map(|t| t.to_string())
            .unwrap_or_else(|| "N/A".to_string())
    ));

    // Context statistics
    let total = manifest.context_tree.len();
    let system_count = manifest
        .context_tree
        .iter()
        .filter(|c| matches!(c.r#type, crate::context::types::ContextType::System))
        .count();
    let user_count = manifest
        .context_tree
        .iter()
        .filter(|c| matches!(c.r#type, crate::context::types::ContextType::User))
        .count();
    let external_count = manifest
        .context_tree
        .iter()
        .filter(|c| matches!(c.r#type, crate::context::types::ContextType::External))
        .count();
    let model_state_count = manifest
        .context_tree
        .iter()
        .filter(|c| matches!(c.r#type, crate::context::types::ContextType::ModelState))
        .count();

    output.push_str(&format!(
        "Contexts: {total} total (System: {system_count}, User: {user_count}, External: {external_count}, ModelState: {model_state_count})\n"
    ));

    // Top 3 contributors
    let mut contexts = manifest.context_tree.clone();
    contexts.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());

    output.push_str("Top contributors:\n");
    for ctx in contexts.iter().take(3) {
        let summary = ctx
            .summary
            .as_ref()
            .map(|s| {
                if s.len() > 60 {
                    format!("{s:.60}...")
                } else {
                    s.clone()
                }
            })
            .unwrap_or_else(|| ctx.source.clone());

        output.push_str(&format!(
            "  - {} ({:.1}%): {}\n",
            ctx.id,
            ctx.weight * 100.0,
            summary
        ));
    }

    Ok(output)
}

/// Format lineage manifest according to the specified format
///
/// # Arguments
/// * `manifest` - The lineage manifest to format
/// * `format` - Desired output format
///
/// # Returns
/// * `Ok(String)` with formatted output
/// * `Err(String)` if formatting fails
pub fn format_manifest(manifest: &LineageManifest, format: OutputFormat) -> Result<String, String> {
    match format {
        OutputFormat::Json => format_as_json(manifest),
        OutputFormat::Tree => format_as_tree(manifest),
        OutputFormat::Compact => format_as_compact(manifest),
    }
}

// ========== Query Service ==========

use crate::context::storage::StorageBackend;
use crate::context::types::{ContextImpactReport, ContextType};
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Query filters for context/response queries
#[derive(Debug, Clone)]
pub struct QueryFilters {
    /// Minimum weight threshold (0.0 to 1.0)
    pub min_weight: Option<f32>,
    /// Start date for filtering (inclusive)
    pub start_date: Option<DateTime<Utc>>,
    /// End date for filtering (inclusive)
    pub end_date: Option<DateTime<Utc>>,
    /// Context type filter
    pub context_type: Option<ContextType>,
    /// Maximum results to return
    pub limit: Option<usize>,
}

impl Default for QueryFilters {
    fn default() -> Self {
        Self {
            min_weight: None,
            start_date: None,
            end_date: None,
            context_type: None,
            limit: Some(100), // Default limit
        }
    }
}

/// Service for querying context tracing data
pub struct QueryService {
    storage: Arc<dyn StorageBackend>,
}

impl QueryService {
    /// Create a new query service
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self { storage }
    }

    /// Find all responses that used a specific context unit
    ///
    /// # Arguments
    /// * `context_unit_id` - The context unit ID to search for
    /// * `filters` - Optional filters for the query
    ///
    /// # Returns
    /// * `Ok(ContextImpactReport)` with all matching responses
    /// * `Err(String)` if query fails
    pub async fn query_responses_by_context(
        &self,
        context_unit_id: &str,
        filters: Option<QueryFilters>,
    ) -> Result<ContextImpactReport, String> {
        let filters = filters.unwrap_or_default();

        // Get all responses that reference this context unit
        let responses = self
            .storage
            .get_responses_for_context(
                context_unit_id,
                filters.min_weight,
                filters.start_date,
                filters.end_date,
                filters.limit,
            )
            .await
            .map_err(|e| format!("Failed to query responses: {e}"))?;

        // Calculate statistics
        let total_responses = responses.len();
        let avg_weight = if total_responses > 0 {
            responses.iter().map(|r| r.weight).sum::<f32>() / total_responses as f32
        } else {
            0.0
        };

        Ok(ContextImpactReport {
            context_unit_id: context_unit_id.to_string(),
            total_responses,
            avg_weight,
            responses,
        })
    }

    /// Find all contexts used in a specific response
    ///
    /// # Arguments
    /// * `response_id` - The response ID to search for
    /// * `type_filter` - Optional context type filter
    ///
    /// # Returns
    /// * `Ok(Vec<ContextTreeNode>)` with all matching contexts
    /// * `Err(String)` if query fails
    pub async fn query_contexts_by_response(
        &self,
        response_id: &str,
        type_filter: Option<ContextType>,
    ) -> Result<Vec<crate::context::types::ContextTreeNode>, String> {
        // Get the lineage manifest which contains context tree
        let manifest = self
            .storage
            .query_lineage(response_id)
            .await
            .map_err(|e| format!("Failed to query lineage: {e}"))?
            .ok_or_else(|| format!("Response {response_id} not found"))?;

        // Filter by type if specified
        let contexts = if let Some(filter_type) = type_filter {
            manifest
                .context_tree
                .into_iter()
                .filter(|ctx| ctx.r#type == filter_type)
                .collect()
        } else {
            manifest.context_tree
        };

        Ok(contexts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::types::{
        ContextTreeNode, ContextType, LineageManifest, ProvenanceEdge, ProvenanceTree,
    };
    use chrono::Utc;

    fn create_test_manifest() -> LineageManifest {
        LineageManifest {
            response_id: "resp_test".to_string(),
            timestamp: Utc::now(),
            agent: "test-agent".to_string(),
            model: "test-model".to_string(),
            token_count: Some(100),
            context_tree: vec![
                ContextTreeNode {
                    id: "ctx_1".to_string(),
                    r#type: ContextType::System,
                    source: "test".to_string(),
                    weight: 0.6,
                    embedding_id: None,
                    summary: Some("System context".to_string()),
                },
                ContextTreeNode {
                    id: "ctx_2".to_string(),
                    r#type: ContextType::User,
                    source: "test".to_string(),
                    weight: 0.4,
                    embedding_id: None,
                    summary: Some("User context".to_string()),
                },
            ],
            provenance_tree: ProvenanceTree {
                root: "resp_test".to_string(),
                edges: vec![
                    ProvenanceEdge {
                        from: "resp_test".to_string(),
                        to: "ctx_1".to_string(),
                        weight: 0.6,
                    },
                    ProvenanceEdge {
                        from: "resp_test".to_string(),
                        to: "ctx_2".to_string(),
                        weight: 0.4,
                    },
                ],
            },
        }
    }

    #[test]
    fn test_format_as_json() {
        let manifest = create_test_manifest();
        let result = format_as_json(&manifest);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("resp_test"));
        assert!(json.contains("ctx_1"));
    }

    #[test]
    fn test_format_as_tree() {
        let manifest = create_test_manifest();
        let result = format_as_tree(&manifest);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(tree.contains("Response: resp_test"));
        assert!(tree.contains("60%"));
        assert!(tree.contains("40%"));
    }

    #[test]
    fn test_format_as_compact() {
        let manifest = create_test_manifest();
        let result = format_as_compact(&manifest);
        assert!(result.is_ok());
        let compact = result.unwrap();
        assert!(compact.contains("resp_test"));
        assert!(compact.contains("Tokens: 100"));
        assert!(compact.contains("Top contributors"));
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("tree"), Some(OutputFormat::Tree));
        assert_eq!(
            OutputFormat::from_str("compact"),
            Some(OutputFormat::Compact)
        );
        assert_eq!(OutputFormat::from_str("invalid"), None);
    }
}
