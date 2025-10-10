# Quick Start: Context Tracing Framework

**Feature**: AI Context Provenance & Evolution Framework
**Branch**: `001-context-tracing-framework`
**Date**: 2025-10-09

This guide helps developers integrate and use the context tracing framework in the MCP Rust Proxy.

---

## Overview

The context tracing framework tracks where AI responses come from by:
1. **Capturing** context units (sources of information) during response generation
2. **Recording** lineage manifests showing how contexts influenced each response
3. **Enabling** queries to understand context impact and quality over time

---

## Prerequisites

- Rust stable toolchain (see `rust-toolchain.toml`)
- MCP Rust Proxy codebase cloned
- SQLite 3.35+ (for JSON1 extension support)
- Familiarity with tokio async runtime

---

## Installation

### 1. Add Dependencies

The following dependencies are already in `Cargo.toml`, but verify these are present:

```toml
[dependencies]
# Core (already present)
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.10", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dashmap = "6.0"

# New for context tracing
rusqlite = { version = "0.32", features = ["bundled", "json1"] }
```

### 2. Build the Project

```bash
cargo build --release
```

---

## Basic Usage

### 1. Initialize Context Tracker

In your response generation pipeline (e.g., `src/proxy/handler.rs`):

```rust
use crate::context::tracker::ContextTracker;
use crate::context::types::{ContextUnit, ContextType};

// Create tracker instance
let tracker = ContextTracker::new(state.clone()).await?;

// Start tracking for a new response
let response_id = tracker.start_response(
    "claude-sdk:lazy-broker",
    "claude-3.5-sonnet"
).await?;
```

### 2. Track Context Usage

As your code retrieves context (from memory, tools, etc.), register each context unit:

```rust
// Example: After vector search retrieves memory
let context_unit = ContextUnit {
    id: format!("cu_{}", uuid::Uuid::new_v4()),
    type: ContextType::User,
    source: "memory:project/ripple".to_string(),
    timestamp: Utc::now(),
    embedding_id: Some("vec_3928a".to_string()),
    summary: Some("ArgoCD AppSets with SCM filtering".to_string()),
    version: 1,
    previous_version_id: None,
    aggregate_score: 0.0,
    feedback_count: 0,
};

// Register with tracker (weight calculated automatically)
tracker.add_context(response_id.clone(), context_unit).await?;
```

### 3. Finalize Response

After generating the AI response, finalize tracking to create the lineage manifest:

```rust
// Finalize and persist lineage
let manifest = tracker.finalize_response(
    response_id,
    1984  // token_count
).await?;

// Manifest is automatically persisted to storage
println!("Lineage manifest created: {}", manifest.response_id);
```

---

## Querying Traces

### API Endpoints

Once the framework is running, use these HTTP endpoints:

#### Get Response Trace
```bash
# Retrieve full lineage manifest
curl http://localhost:3001/api/trace/resp_20251009_00123

# Get tree visualization
curl http://localhost:3001/api/trace/resp_20251009_00123?format=tree
```

#### Find Responses by Context
```bash
# Find all responses using a specific memory
curl http://localhost:3001/api/query/by-context/cu_argo_appset_001

# Filter by minimum weight (0.3) and date range
curl "http://localhost:3001/api/query/by-context/cu_argo_appset_001?min_weight=0.3&start_date=2025-10-01T00:00:00Z"
```

#### Track Context Evolution
```bash
# Get version history of a context unit
curl http://localhost:3001/api/query/evolution/cu_argo_appset_001
```

### Programmatic Queries

In Rust code, use the query interface:

```rust
use crate::context::query::QueryService;

let query_service = QueryService::new(state.clone());

// Get trace for a response
let manifest = query_service.get_response_trace("resp_20251009_00123").await?;

// Find all responses using a context
let impact = query_service.query_by_context(
    "cu_argo_appset_001",
    None,  // no weight filter
    None,  // no date filter
    100    // limit
).await?;

println!("Found {} responses using this context", impact.total_responses);
```

---

## Providing Feedback

### Via API

```bash
curl -X POST http://localhost:3001/api/feedback \
  -H "Content-Type: application/json" \
  -d '{
    "response_id": "resp_20251009_00123",
    "score": 0.8,
    "feedback_text": "Accurate and helpful",
    "user_id": "user_42"
  }'
```

### Programmatic

```rust
use crate::context::types::FeedbackRecord;

let feedback = FeedbackRecord {
    id: format!("fb_{}", uuid::Uuid::new_v4()),
    response_id: "resp_20251009_00123".to_string(),
    timestamp: Utc::now(),
    score: 0.8,
    feedback_text: Some("Accurate and helpful".to_string()),
    user_id: Some("user_42".to_string()),
};

tracker.record_feedback(feedback).await?;
```

Feedback automatically propagates to all context units in the response.

---

## Configuration

### Storage Settings

Configure in `mcp-proxy-config.yaml`:

```yaml
context_tracing:
  enabled: true
  storage:
    type: hybrid  # hybrid, sqlite, or postgresql
    sqlite_path: ~/.mcp-proxy/context.db
    cache_size: 10000  # number of responses in hot cache
    retention_days: 90

  weight_calculation:
    method: composite  # composite, retrieval, or uniform
    retrieval_weight: 0.4
    recency_weight: 0.3
    type_weight: 0.2
    length_weight: 0.1
```

### Environment Variables

```bash
# Override storage backend
export CONTEXT_STORAGE=sqlite

# Override retention period
export CONTEXT_RETENTION_DAYS=180

# Enable debug logging
export RUST_LOG=mcp_rust_proxy::context=debug
```

---

## Testing

### Run Unit Tests

```bash
# All context framework tests
cargo test --package mcp-rust-proxy --lib context

# Specific test modules
cargo test --test context_tracker_tests
cargo test --test context_storage_tests
cargo test --test context_query_tests
```

### Integration Tests

```bash
# Full integration test suite
cargo test --test context_integration_tests

# Test with real SQLite database
cargo test --test context_integration_tests --features integration
```

### Manual Testing

```bash
# Start proxy with debug logging
RUST_LOG=debug cargo run -- --config mcp-proxy-config.yaml

# Generate a traced response (in another terminal)
curl -X POST http://localhost:3000/mcp/tools/call \
  -H "Content-Type: application/json" \
  -d '{"name": "search", "arguments": {"query": "ArgoCD"}}'

# Retrieve the trace
RESPONSE_ID=$(curl http://localhost:3001/api/stats/recent | jq -r '.responses[0].id')
curl http://localhost:3001/api/trace/$RESPONSE_ID
```

---

## Performance Considerations

### Cache Warmth

The hybrid storage approach keeps recent responses in memory:

- **First 7 days**: Sub-second queries (DashMap cache)
- **8-90 days**: 2-5 second queries (SQLite disk)
- **After 90 days**: Responses are automatically purged

### Query Optimization

For best performance:

1. **Use time filters** when querying large datasets
2. **Enable indexes** on frequently queried fields
3. **Batch feedback** submissions when possible
4. **Monitor cache hit rate** via metrics endpoint

```bash
# Check cache performance
curl http://localhost:3001/api/stats/cache
```

### Resource Usage

Expected resource consumption:

- **Memory**: ~40 MB (10K response cache) + base proxy overhead
- **Disk**: ~4 KB per response × retention period
- **CPU**: < 100ms per response for tracking overhead

---

## Troubleshooting

### Issue: Trace Not Found

**Symptom**: `GET /api/trace/{id}` returns 404

**Solutions**:
1. Verify response was generated after framework enabled
2. Check retention policy hasn't purged old responses
3. Verify SQLite database exists at configured path
4. Check logs for storage errors: `RUST_LOG=debug`

### Issue: Weights Don't Sum to 1.0

**Symptom**: Validation error when finalizing response

**Solutions**:
1. Ensure all context units have weights assigned
2. Verify weight calculation method in config
3. Check for floating-point precision issues (tolerance is ±0.01)

### Issue: Slow Queries

**Symptom**: Queries take > 5 seconds

**Solutions**:
1. Check cache hit rate (should be > 80% for recent data)
2. Verify indexes exist on lineage table
3. Add time filters to narrow result set
4. Consider migrating to PostgreSQL for > 1M responses

### Issue: Database Locked

**Symptom**: SQLite BUSY error

**Solutions**:
1. Increase WAL mode timeout in config
2. Reduce concurrent write load
3. Switch to PostgreSQL for high concurrency

---

## Advanced Usage

### Custom Weight Calculation

Implement your own weight calculator:

```rust
use crate::context::tracker::WeightCalculator;

pub struct CustomWeightCalculator;

impl WeightCalculator for CustomWeightCalculator {
    fn calculate_weights(&self, contexts: &[ContextUnit]) -> Vec<f32> {
        // Your custom logic here
        // Must return weights that sum to 1.0
        vec![]
    }
}

// Register custom calculator
tracker.set_weight_calculator(Box::new(CustomWeightCalculator));
```

### Context Versioning

When updating a context unit:

```rust
// Load existing context
let old_context = storage.get_context_unit("cu_argo_appset_001").await?;

// Create new version
let new_context = ContextUnit {
    id: format!("cu_{}", uuid::Uuid::new_v4()),
    type: old_context.type.clone(),
    source: old_context.source.clone(),
    timestamp: Utc::now(),
    embedding_id: Some("vec_new_id".to_string()),
    summary: Some("Updated ArgoCD AppSets info".to_string()),
    version: old_context.version + 1,
    previous_version_id: Some(old_context.id.clone()),
    aggregate_score: old_context.aggregate_score,
    feedback_count: old_context.feedback_count,
};

storage.store_context_unit(new_context).await?;
```

### Export Lineage Data

```rust
use crate::context::export::LineageExporter;

let exporter = LineageExporter::new(storage);

// Export to JSON file
exporter.export_to_file(
    "lineage_export.json",
    Some("2025-10-01"),  // start date
    Some("2025-10-31")   // end date
).await?;

// Export for external analysis
let csv_data = exporter.export_to_csv().await?;
```

---

## Next Steps

1. **Review** the [data model documentation](./data-model.md) for schema details
2. **Explore** the [API specification](./contracts/api-spec.yaml) for all endpoints
3. **Read** the [research document](./research.md) for architecture decisions
4. **Run** `/speckit.tasks` to see implementation tasks
5. **Contribute** improvements via pull requests

---

## Support

- **Issues**: Report bugs at https://github.com/zach-source/mcp-rust-proxy/issues
- **Discussions**: Join community at https://github.com/zach-source/mcp-rust-proxy/discussions
- **Docs**: Full documentation at https://mcp-rust-proxy.readthedocs.io

---

**Last Updated**: 2025-10-09
**Framework Version**: 1.0.0 (MVP)
