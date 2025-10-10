//! MCP tools for interacting with context tracing data
//!
//! This module exposes the context tracing framework as MCP tools that LLM agents
//! can call to query lineage data, submit feedback, and track context evolution.

use crate::context::query::{format_manifest, OutputFormat, QueryFilters, QueryService};
use crate::context::types::FeedbackSubmission;
use crate::state::AppState;
use serde_json::{json, Value};
use std::sync::Arc;

/// Get the list of context tracing resources
pub fn get_tracing_resources() -> Vec<Value> {
    vec![
        json!({
            "uri": "trace://quality/top-contexts",
            "name": "Top Quality Contexts",
            "description": "High-rated context units (aggregate_score > 0.5) - most reliable information sources",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "trace://quality/deprecated-contexts",
            "name": "Deprecated Contexts",
            "description": "Low-rated context units (aggregate_score < -0.5) - potentially outdated or incorrect",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "trace://quality/recent-feedback",
            "name": "Recent Feedback",
            "description": "Last 20 quality feedback submissions across all responses",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "trace://stats/cache",
            "name": "Cache Statistics",
            "description": "Storage cache hit/miss rates and performance metrics",
            "mimeType": "application/json"
        }),
    ]
}

/// Handle reading a context tracing resource
pub async fn handle_tracing_resource(uri: &str, state: Arc<AppState>) -> Result<Value, String> {
    let tracker_guard = state.context_tracker.read().await;
    let tracker = tracker_guard
        .as_ref()
        .ok_or_else(|| "Context tracing is not enabled".to_string())?;

    let storage = tracker.storage();

    match uri {
        "trace://quality/top-contexts" => {
            // TODO: Query contexts with score > 0.5
            // For now, return placeholder
            let data = json!({
                "description": "High-quality context units",
                "threshold": 0.5,
                "contexts": [],
                "note": "Full implementation pending - requires storage query by score"
            });

            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&data).unwrap()
                }]
            }))
        }

        "trace://quality/deprecated-contexts" => {
            let data = json!({
                "description": "Low-quality context units flagged for review",
                "threshold": -0.5,
                "contexts": [],
                "note": "Full implementation pending - requires storage query by score"
            });

            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&data).unwrap()
                }]
            }))
        }

        "trace://quality/recent-feedback" => {
            let now = chrono::Utc::now();
            let week_ago = now - chrono::Duration::days(7);

            let feedback = storage
                .get_feedback_range(week_ago, now)
                .await
                .map_err(|e| format!("Failed to get feedback: {}", e))?;

            let recent: Vec<_> = feedback.into_iter().take(20).collect();

            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&recent).unwrap()
                }]
            }))
        }

        "trace://stats/cache" => {
            // Access cache stats if available (HybridStorage)
            let stats_data = json!({
                "description": "Storage cache performance metrics",
                "note": "Cache statistics available when using HybridStorage backend"
            });

            Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&stats_data).unwrap()
                }]
            }))
        }

        _ => {
            // Check if it's a dynamic resource like trace://response/{id}
            if uri.starts_with("trace://response/") {
                let response_id = uri.strip_prefix("trace://response/").unwrap();
                let manifest = storage
                    .query_lineage(response_id)
                    .await
                    .map_err(|e| format!("Failed to query lineage: {}", e))?
                    .ok_or_else(|| format!("Response {} not found", response_id))?;

                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&manifest).unwrap()
                    }]
                }))
            } else if uri.starts_with("trace://context/") {
                let context_id = uri.strip_prefix("trace://context/").unwrap();
                let context = storage
                    .get_context_unit(context_id)
                    .await
                    .map_err(|e| format!("Failed to get context: {}", e))?
                    .ok_or_else(|| format!("Context {} not found", context_id))?;

                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&context).unwrap()
                    }]
                }))
            } else if uri.starts_with("trace://evolution/") {
                let context_id = uri.strip_prefix("trace://evolution/").unwrap();
                let evolution_service = crate::context::evolution::EvolutionService::new(storage);
                let history = evolution_service
                    .get_version_history(context_id)
                    .await
                    .map_err(|e| format!("Failed to get evolution: {}", e))?;

                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&history).unwrap()
                    }]
                }))
            } else {
                Err(format!("Unknown resource URI: {}", uri))
            }
        }
    }
}

/// Get the list of context tracing tools
pub fn get_tracing_tools() -> Vec<Value> {
    vec![
        // Session Management Tools
        json!({
            "name": "mcp__proxy__tracing__start_session",
            "description": "Start a new tracking session to group related responses in a conversation",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (auto-generated if not provided)"
                    },
                    "user_query": {
                        "type": "string",
                        "description": "The original user question/request"
                    },
                    "metadata": {
                        "type": "object",
                        "description": "Optional metadata about the session"
                    }
                }
            }
        }),
        json!({
            "name": "mcp__proxy__tracing__end_session",
            "description": "End the current tracking session and generate session analytics",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to end (uses current if not provided)"
                    },
                    "auto_feedback": {
                        "type": "boolean",
                        "description": "Automatically submit feedback for session",
                        "default": false
                    },
                    "session_score": {
                        "type": "number",
                        "description": "Overall session quality score",
                        "minimum": -1.0,
                        "maximum": 1.0
                    }
                }
            }
        }),
        json!({
            "name": "mcp__proxy__tracing__record_action",
            "description": "Record a custom action/event in the tracking system",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action_type": {
                        "type": "string",
                        "enum": ["tool_use", "error", "retry", "correction", "success"],
                        "description": "Type of action being recorded"
                    },
                    "response_id": {
                        "type": "string",
                        "description": "Response ID to associate action with"
                    },
                    "metadata": {
                        "type": "object",
                        "description": "Additional action metadata"
                    }
                }
            }
        }),
        json!({
            "name": "mcp__proxy__tracing__link_responses",
            "description": "Create a relationship between responses in a conversation chain",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "parent_response_id": {
                        "type": "string",
                        "description": "Earlier response in the conversation"
                    },
                    "child_response_id": {
                        "type": "string",
                        "description": "Later response that builds on the parent"
                    },
                    "relationship_type": {
                        "type": "string",
                        "enum": ["builds_on", "corrects", "clarifies", "completes"],
                        "description": "Type of relationship",
                        "default": "builds_on"
                    }
                },
                "required": ["parent_response_id", "child_response_id"]
            }
        }),
        json!({
            "name": "mcp__proxy__tracing__get_session_summary",
            "description": "Get summary of all responses and feedback in a session",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to summarize (current if not provided)"
                    },
                    "include_lineage": {
                        "type": "boolean",
                        "description": "Include full lineage for each response",
                        "default": false
                    }
                },
                "required": ["session_id"]
            }
        }),
        // Original Tracing Tools
        json!({
            "name": "mcp__proxy__tracing__get_trace",
            "description": "Get lineage manifest showing which context units influenced a specific AI response",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "response_id": {
                        "type": "string",
                        "description": "The response ID to get lineage for (format: resp_*)"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["json", "tree", "compact"],
                        "description": "Output format (default: json)",
                        "default": "json"
                    }
                },
                "required": ["response_id"]
            }
        }),
        json!({
            "name": "mcp__proxy__tracing__query_context_impact",
            "description": "Find all responses that used a specific context unit to assess impact of context changes",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "context_unit_id": {
                        "type": "string",
                        "description": "The context unit ID to query"
                    },
                    "min_weight": {
                        "type": "number",
                        "description": "Minimum contribution weight (0.0 to 1.0)",
                        "minimum": 0.0,
                        "maximum": 1.0
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of responses to return",
                        "default": 100
                    }
                },
                "required": ["context_unit_id"]
            }
        }),
        json!({
            "name": "mcp__proxy__tracing__get_response_contexts",
            "description": "Get all context units that contributed to a specific response",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "response_id": {
                        "type": "string",
                        "description": "The response ID to query"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["System", "User", "External", "ModelState"],
                        "description": "Filter by context type (optional)"
                    }
                },
                "required": ["response_id"]
            }
        }),
        json!({
            "name": "mcp__proxy__tracing__get_evolution_history",
            "description": "Get version history for a context unit to understand how it evolved over time",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "context_unit_id": {
                        "type": "string",
                        "description": "The context unit ID (any version in the chain)"
                    }
                },
                "required": ["context_unit_id"]
            }
        }),
        json!({
            "name": "mcp__proxy__tracing__submit_feedback",
            "description": "Submit quality feedback on a response that propagates to all contributing context units",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "response_id": {
                        "type": "string",
                        "description": "The response ID to provide feedback on"
                    },
                    "score": {
                        "type": "number",
                        "description": "Quality score from -1.0 (poor) to 1.0 (excellent)",
                        "minimum": -1.0,
                        "maximum": 1.0
                    },
                    "feedback_text": {
                        "type": "string",
                        "description": "Optional comment explaining the feedback"
                    },
                    "user_id": {
                        "type": "string",
                        "description": "Optional user identifier"
                    }
                },
                "required": ["response_id", "score"]
            }
        }),
    ]
}

/// Handle a context tracing tool call
pub async fn handle_tracing_tool(
    tool_name: &str,
    arguments: Value,
    state: Arc<AppState>,
) -> Result<Value, String> {
    let tracker_guard = state.context_tracker.read().await;
    let tracker = tracker_guard
        .as_ref()
        .ok_or_else(|| "Context tracing is not enabled".to_string())?;

    let storage = tracker.storage();

    match tool_name {
        "start_session" => {
            let session_id = arguments
                .get("session_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("session_{}", uuid::Uuid::new_v4()));

            let user_query = arguments
                .get("user_query")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Store session ID in a file for hooks to access
            let session_file = std::path::Path::new("/tmp/mcp-proxy-current-session");
            std::fs::write(session_file, &session_id).ok();

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Session started: {}\n\nAll tool calls in this conversation will be grouped under this session.\nUser query: {}",
                        session_id,
                        user_query.unwrap_or_else(|| "Not provided".to_string())
                    )
                }],
                "session_id": session_id
            }))
        }

        "end_session" => {
            let session_from_file = std::fs::read_to_string("/tmp/mcp-proxy-current-session").ok();
            let session_id = arguments
                .get("session_id")
                .and_then(|v| v.as_str())
                .or_else(|| session_from_file.as_deref())
                .unwrap_or("unknown");

            let auto_feedback = arguments
                .get("auto_feedback")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Clean up session file
            std::fs::remove_file("/tmp/mcp-proxy-current-session").ok();
            std::fs::remove_file("/tmp/mcp-proxy-last-response-id").ok();

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Session ended: {}\n\nAuto-feedback: {}\n\nUse /mcp-proxy:quality-report to see session impact.",
                        session_id,
                        auto_feedback
                    )
                }]
            }))
        }

        "record_action" => {
            let action_type = arguments
                .get("action_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let response_id = arguments.get("response_id").and_then(|v| v.as_str());

            // For now, just log the action
            // Future: Store actions in a separate table
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Action recorded: {}\nResponse: {}\n\nThis feature is currently logging-only. Full action tracking coming soon.",
                        action_type,
                        response_id.unwrap_or("none")
                    )
                }]
            }))
        }

        "link_responses" => {
            let parent = arguments
                .get("parent_response_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing parent_response_id".to_string())?;

            let child = arguments
                .get("child_response_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing child_response_id".to_string())?;

            let rel_type = arguments
                .get("relationship_type")
                .and_then(|v| v.as_str())
                .unwrap_or("builds_on");

            // Future: Store relationship in database
            // For now, document the relationship
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Response relationship created:\n  {} --[{}]--> {}\n\nThis creates a conversation chain showing how responses build on each other.\n\nFull relationship storage coming soon.",
                        parent,
                        rel_type,
                        child
                    )
                }]
            }))
        }

        "get_session_summary" => {
            let session_id = arguments
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing session_id".to_string())?;

            // Future: Query all responses in session
            // For now, return placeholder
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Session Summary: {}\n\nThis feature requires session-response mapping in storage.\nComing soon: Full session analytics with response chains, feedback trends, and quality metrics.",
                        session_id
                    )
                }]
            }))
        }

        "get_trace" => {
            let response_id = arguments
                .get("response_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing response_id".to_string())?;

            let format_str = arguments
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("json");
            let format = OutputFormat::from_str(format_str).unwrap_or(OutputFormat::Json);

            let manifest = storage
                .query_lineage(response_id)
                .await
                .map_err(|e| format!("Failed to query lineage: {}", e))?
                .ok_or_else(|| format!("Response {} not found", response_id))?;

            let formatted = format_manifest(&manifest, format)
                .map_err(|e| format!("Failed to format manifest: {}", e))?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": formatted
                }]
            }))
        }

        "query_context_impact" => {
            let context_unit_id = arguments
                .get("context_unit_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing context_unit_id".to_string())?;

            let filters = QueryFilters {
                min_weight: arguments
                    .get("min_weight")
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32),
                limit: arguments
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|l| l as usize),
                start_date: None,
                end_date: None,
                context_type: None,
            };

            let query_service = QueryService::new(storage);
            let report = query_service
                .query_responses_by_context(context_unit_id, Some(filters))
                .await
                .map_err(|e| format!("Failed to query impact: {}", e))?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&report).unwrap()
                }]
            }))
        }

        "get_response_contexts" => {
            let response_id = arguments
                .get("response_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing response_id".to_string())?;

            let type_filter =
                arguments
                    .get("type")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "System" => Some(crate::context::types::ContextType::System),
                        "User" => Some(crate::context::types::ContextType::User),
                        "External" => Some(crate::context::types::ContextType::External),
                        "ModelState" => Some(crate::context::types::ContextType::ModelState),
                        _ => None,
                    });

            let query_service = QueryService::new(storage);
            let contexts = query_service
                .query_contexts_by_response(response_id, type_filter)
                .await
                .map_err(|e| format!("Failed to query contexts: {}", e))?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&contexts).unwrap()
                }]
            }))
        }

        "get_evolution_history" => {
            let context_unit_id = arguments
                .get("context_unit_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing context_unit_id".to_string())?;

            let evolution_service = crate::context::evolution::EvolutionService::new(storage);
            let history = evolution_service
                .get_version_history(context_unit_id)
                .await
                .map_err(|e| format!("Failed to get evolution history: {}", e))?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&history).unwrap()
                }]
            }))
        }

        "submit_feedback" => {
            let response_id = arguments
                .get("response_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing response_id".to_string())?;

            let score = arguments
                .get("score")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| "Missing score".to_string())? as f32;

            let feedback_text = arguments
                .get("feedback_text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let user_id = arguments
                .get("user_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let propagation_status = tracker
                .record_feedback(response_id, score, feedback_text, user_id)
                .await
                .map_err(|e| format!("Failed to record feedback: {}", e))?;

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Feedback submitted successfully!\n\nPropagation: {} contexts updated, avg score change: {:.3}",
                        propagation_status.contexts_updated,
                        propagation_status.avg_score_change
                    )
                }],
                "propagation": propagation_status
            }))
        }

        _ => Err(format!("Unknown tracing tool: {}", tool_name)),
    }
}
