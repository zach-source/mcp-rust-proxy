# Data Model: Context Tracing Framework

**Feature**: AI Context Provenance & Evolution Framework
**Branch**: `001-context-tracing-framework`
**Date**: 2025-10-09
**Status**: Draft

Based on research findings in [research.md](./research.md), this document defines the data model for the context tracing framework using a **Hybrid DashMap + SQLite** storage approach.

---

## Core Entities

### 1. Context Unit (CU)

A discrete piece of information used in generating AI responses.

**Attributes**:

| Field | Type | Required | Description | Validation Rules |
|-------|------|----------|-------------|------------------|
| `id` | String (UUID v4) | Yes | Unique identifier | Valid UUID format |
| `type` | Enum: `ContextType` | Yes | Category of context | One of: System, User, External, ModelState |
| `source` | String | Yes | Origin identifier | Max 255 chars, non-empty |
| `timestamp` | DateTime (ISO 8601) | Yes | Creation time | Valid timestamp, not future |
| `contribution_weight` | Float (0.0-1.0) | Yes (when used) | Influence on response | 0.0 ≤ weight ≤ 1.0 |
| `embedding_id` | Option<String> | No | Vector DB reference | Valid embedding ID or null |
| `summary` | Option<String> | No | Human-readable description | Max 500 chars |
| `version` | Integer | Yes | Version number | Starts at 1, monotonic increase |
| `previous_version_id` | Option<String> | No | Link to previous version | Valid UUID or null |

**ContextType Enum**:
```rust
pub enum ContextType {
    System,      // Tool definitions, MCP plugins, namespaces
    User,        // Chat history, project data, memory
    External,    // Web results, MCP tool outputs
    ModelState,  // Temperature, system prompt, agent goals
}
```

**State Transitions**:
- Created → Active (default state)
- Active → Updated (creates new version, links previous_version_id)
- Active → Deprecated (when feedback score drops below threshold)

**Validation Rules**:
- `contribution_weight` sum across all CUs in a response must equal 1.0 (±0.01 tolerance)
- `version` must increment when creating updated context
- `previous_version_id` must point to valid CU or be null for version 1

---

### 2. Response

An AI-generated output with provenance tracking.

**Attributes**:

| Field | Type | Required | Description | Validation Rules |
|-------|------|----------|-------------|------------------|
| `id` | String (UUID v4) | Yes | Unique identifier | Valid UUID format, prefixed with `resp_` |
| `timestamp` | DateTime (ISO 8601) | Yes | Generation time | Valid timestamp, not future |
| `agent` | String | Yes | Agent identifier | Max 100 chars, non-empty |
| `model` | String | Yes | Model version | Max 100 chars, non-empty |
| `token_count` | Integer | No | Response size | ≥ 0 |
| `context_units` | Vec<ContextReference> | Yes | Contributing contexts | 1-50 items |
| `manifest_json` | String | Yes | Full lineage manifest | Valid JSON, < 5KB |

**ContextReference** (embedded):
```rust
pub struct ContextReference {
    pub context_unit_id: String,  // UUID of CU
    pub weight: f32,               // Contribution weight
}
```

**Validation Rules**:
- `context_units` must contain at least 1 item
- `context_units` weights must sum to 1.0 (±0.01)
- All `context_unit_id` references must exist in ContextUnit table
- `manifest_json` must be valid JSON and < 5KB when serialized

---

### 3. Lineage Manifest

A structured record of how a response was generated (stored as JSON in `Response.manifest_json`).

**Schema**:
```json
{
  "response_id": "resp_20251009_00123",
  "timestamp": "2025-10-09T16:15:00Z",
  "agent": "claude-sdk:lazy-broker",
  "model": "claude-3.5-sonnet",
  "token_count": 1984,
  "context_tree": [
    {
      "id": "cu_argo_appset_001",
      "type": "User",
      "source": "memory:project/ripple",
      "weight": 0.42,
      "embedding_id": "vec_3928a",
      "summary": "ArgoCD AppSets with SCM filtering"
    },
    {
      "id": "cu_values_yaml_002",
      "type": "User",
      "source": "file:argocd/values.yaml",
      "weight": 0.31,
      "embedding_id": "vec_4821b",
      "summary": "Helm values configuration"
    },
    {
      "id": "cu_docs_mcp_003",
      "type": "External",
      "source": "tool:docs_mcp",
      "weight": 0.27,
      "embedding_id": null,
      "summary": "ArgoCD documentation query"
    }
  ],
  "provenance_tree": {
    "root": "resp_20251009_00123",
    "edges": [
      {"from": "cu_argo_appset_001", "to": "resp_20251009_00123", "weight": 0.42},
      {"from": "cu_values_yaml_002", "to": "resp_20251009_00123", "weight": 0.31},
      {"from": "cu_docs_mcp_003", "to": "resp_20251009_00123", "weight": 0.27}
    ]
  }
}
```

**Validation Rules**:
- `context_tree` weights must sum to 1.0 (±0.01)
- All `id` fields must reference valid Context Units
- Total JSON size must be < 5KB
- `provenance_tree.edges` must match `context_tree` entries

---

### 4. Feedback Record

User or system evaluation of response quality.

**Attributes**:

| Field | Type | Required | Description | Validation Rules |
|-------|------|----------|-------------|------------------|
| `id` | String (UUID v4) | Yes | Unique identifier | Valid UUID format |
| `response_id` | String | Yes | Target response | Must reference valid Response |
| `timestamp` | DateTime (ISO 8601) | Yes | Feedback time | Valid timestamp, ≥ response.timestamp |
| `score` | Float (-1.0 to 1.0) | Yes | Quality rating | -1.0 (bad) to 1.0 (good), 0.0 (neutral) |
| `feedback_text` | Option<String> | No | Optional comment | Max 1000 chars |
| `user_id` | Option<String> | No | User identifier | Max 100 chars |

**Validation Rules**:
- `score` must be in range [-1.0, 1.0]
- `response_id` must reference existing Response
- `timestamp` must be ≥ referenced response timestamp
- Each `response_id` can have multiple feedback records

**Feedback Propagation**:
When feedback is submitted:
1. Record is created with `response_id` link
2. Feedback score propagates to all `context_units` in that response
3. Each CU's aggregate score is updated: `new_score = (old_score * old_count + feedback_score * cu_weight) / (old_count + 1)`
4. If CU aggregate score drops below threshold (-0.5), flag as deprecated

---

### 5. Context Graph

A network structure representing relationships between Context Units and Responses.

**Nodes**:
- Response nodes (type: Response, id: `resp_*`)
- Context Unit nodes (type: ContextUnit, id: `cu_*`)

**Edges**:

| Edge Type | From | To | Attributes | Description |
|-----------|------|-----|-----------|-------------|
| `USED_IN` | ContextUnit | Response | `weight: f32` | CU contributed to response generation |
| `DERIVED_FROM` | ContextUnit | ContextUnit | `version: i32` | CU is updated version of another CU |
| `UPDATED_BY` | ContextUnit | Response | `timestamp: DateTime` | Response triggered CU update |

**Graph Queries**:
- **Forward traversal**: Given ContextUnit, find all Responses using it (via `USED_IN`)
- **Backward traversal**: Given Response, find all ContextUnits (via reverse `USED_IN`)
- **Evolution tracking**: Given ContextUnit, find all versions (via `DERIVED_FROM`)
- **Impact analysis**: Given deprecated ContextUnit, find all affected Responses

---

## Storage Layer Design

### Hybrid Approach: DashMap + SQLite

**In-Memory (DashMap)**:
- Hot cache for recent responses (last 7 days or 10K responses)
- Fast concurrent access without locks
- Keys: `response_id` → `Response` and `context_unit_id` → `ContextUnit`

**Persistent (SQLite)**:
- Complete historical data (90 days retention)
- Enables complex graph queries via recursive CTEs
- Background writes to avoid blocking response generation

### Tables

#### `responses` table
```sql
CREATE TABLE responses (
    id TEXT PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    agent TEXT NOT NULL,
    model TEXT NOT NULL,
    token_count INTEGER,
    manifest_json TEXT NOT NULL
);

CREATE INDEX idx_responses_timestamp ON responses(timestamp);
```

#### `context_units` table
```sql
CREATE TABLE context_units (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL CHECK (type IN ('System', 'User', 'External', 'ModelState')),
    source TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    summary TEXT,
    embedding_id TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    previous_version_id TEXT,
    aggregate_score REAL DEFAULT 0.0,
    feedback_count INTEGER DEFAULT 0,
    FOREIGN KEY (previous_version_id) REFERENCES context_units(id)
);

CREATE INDEX idx_context_type ON context_units(type);
CREATE INDEX idx_context_version ON context_units(previous_version_id);
```

#### `lineage` table (junction for graph relationships)
```sql
CREATE TABLE lineage (
    response_id TEXT NOT NULL,
    context_unit_id TEXT NOT NULL,
    weight REAL NOT NULL CHECK (weight >= 0.0 AND weight <= 1.0),
    PRIMARY KEY (response_id, context_unit_id),
    FOREIGN KEY (response_id) REFERENCES responses(id) ON DELETE CASCADE,
    FOREIGN KEY (context_unit_id) REFERENCES context_units(id)
);

CREATE INDEX idx_lineage_response ON lineage(response_id);
CREATE INDEX idx_lineage_context ON lineage(context_unit_id);
```

#### `feedback` table
```sql
CREATE TABLE feedback (
    id TEXT PRIMARY KEY,
    response_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    score REAL NOT NULL CHECK (score >= -1.0 AND score <= 1.0),
    feedback_text TEXT,
    user_id TEXT,
    FOREIGN KEY (response_id) REFERENCES responses(id) ON DELETE CASCADE
);

CREATE INDEX idx_feedback_response ON feedback(response_id);
CREATE INDEX idx_feedback_timestamp ON feedback(timestamp);
```

---

## Rust Type Definitions

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextType {
    System,
    User,
    External,
    ModelState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUnit {
    pub id: String,
    pub r#type: ContextType,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub embedding_id: Option<String>,
    pub summary: Option<String>,
    pub version: i32,
    pub previous_version_id: Option<String>,

    // Runtime only (not persisted in manifest)
    #[serde(skip)]
    pub aggregate_score: f32,
    #[serde(skip)]
    pub feedback_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextReference {
    pub context_unit_id: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub agent: String,
    pub model: String,
    pub token_count: Option<i32>,
    pub context_units: Vec<ContextReference>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTreeNode {
    pub id: String,
    pub r#type: ContextType,
    pub source: String,
    pub weight: f32,
    pub embedding_id: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceTree {
    pub root: String,
    pub edges: Vec<ProvenanceEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEdge {
    pub from: String,
    pub to: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub id: String,
    pub response_id: String,
    pub timestamp: DateTime<Utc>,
    pub score: f32,
    pub feedback_text: Option<String>,
    pub user_id: Option<String>,
}
```

---

## Validation Functions

```rust
impl ContextUnit {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() || !self.id.starts_with("cu_") {
            return Err("Invalid context unit ID format".to_string());
        }
        if self.source.is_empty() || self.source.len() > 255 {
            return Err("Source must be 1-255 characters".to_string());
        }
        if self.version < 1 {
            return Err("Version must be ≥ 1".to_string());
        }
        Ok(())
    }
}

impl Response {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() || !self.id.starts_with("resp_") {
            return Err("Invalid response ID format".to_string());
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
            return Err(format!("Context weights must sum to 1.0 (got {})", total_weight));
        }

        Ok(())
    }
}

impl FeedbackRecord {
    pub fn validate(&self) -> Result<(), String> {
        if self.score < -1.0 || self.score > 1.0 {
            return Err("Feedback score must be in range [-1.0, 1.0]".to_string());
        }
        if self.response_id.is_empty() {
            return Err("Response ID is required".to_string());
        }
        Ok(())
    }
}
```

---

## Size Estimates

**Per Response**:
- Base Response struct: ~200 bytes
- Lineage manifest JSON: ~2-4 KB (for 20 context units)
- SQLite row overhead: ~100 bytes
- **Total**: ~2.3-4.3 KB ✅ (meets < 5KB requirement)

**For 100K Responses**:
- In-memory cache (10K responses): ~40 MB
- SQLite database (100K responses): ~300-400 MB
- **Total**: ~340-440 MB

**For 1M Responses** (future scale):
- SQLite database: ~3-4 GB
- In-memory cache (still 10K): ~40 MB
- Consider migration to PostgreSQL at this scale

---

## Next Steps

1. Implement Rust types in `src/context/types.rs`
2. Implement storage layer in `src/context/storage.rs`
3. Create validation tests in `tests/context/storage_tests.rs`
4. Generate API contracts in `contracts/` directory
