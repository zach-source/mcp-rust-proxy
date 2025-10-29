# Data Model: Claude API Proxy

**Feature**: 005-claude-api-proxy
**Date**: 2025-10-28

## Overview

This document defines the data structures for captured Claude API requests, responses, context attribution, and quality feedback. The model leverages the existing context tracing framework in MCP Rust Proxy for storage and querying.

---

## Core Entities

### 1. CapturedRequest

Represents a complete API request intercepted by the proxy.

**Fields**:
- `id`: String (UUID) - Unique identifier for the request
- `timestamp`: DateTime<Utc> - When the proxy received the request
- `url`: String - Full URL (e.g., "https://api.anthropic.com/v1/messages")
- `method`: String - HTTP method (typically "POST")
- `headers`: HashMap<String, String> - HTTP headers (sanitized - no API keys in storage)
- `body`: Vec<u8> - Raw request body (JSON)
- `body_json`: serde_json::Value - Parsed JSON for analysis
- `source_attributions`: Vec<ContextAttribution> - Identified context sources
- `total_tokens`: usize - Total token count for request
- `correlation_id`: String - Links to corresponding CapturedResponse

**Relationships**:
- One-to-one with CapturedResponse (via correlation_id)
- One-to-many with ContextAttribution
- One-to-one with QualityFeedback (optional)

**Validation Rules**:
- `id` must be unique
- `url` must contain "anthropic.com" or "claude.ai"
- `timestamp` must be <= current time
- `body_json` must be valid JSON
- `correlation_id` must match a CapturedResponse

**State Transitions**:
- Created → Stored (after successful capture)
- Stored → Retrieved (when queried via API)
- Stored → Expired (after retention period)

**Storage**:
- SQLite table `captured_requests`
- DashMap cache for recent requests (last 100)

**Indexes**:
- Primary: id
- Secondary: timestamp (for time range queries)
- Secondary: correlation_id (for request-response pairing)

---

### 2. CapturedResponse

Represents the API response received from Claude and forwarded to the CLI.

**Fields**:
- `id`: String (UUID) - Unique identifier for the response
- `correlation_id`: String - Links to corresponding CapturedRequest
- `timestamp`: DateTime<Utc> - When the proxy received the response from Claude API
- `status_code`: u16 - HTTP status code (e.g., 200, 429, 500)
- `headers`: HashMap<String, String> - HTTP response headers
- `body`: Vec<u8> - Raw response body
- `body_json`: serde_json::Value - Parsed JSON response
- `latency_ms`: u64 - Time between request forward and response receipt
- `proxy_latency_ms`: u64 - Additional latency added by proxy operations
- `response_tokens`: usize - Token count for response (from API usage field)

**Relationships**:
- One-to-one with CapturedRequest (via correlation_id)
- One-to-one with QualityFeedback (optional)

**Validation Rules**:
- `correlation_id` must match an existing CapturedRequest
- `status_code` must be valid HTTP status (100-599)
- `timestamp` must be >= corresponding request timestamp
- `latency_ms` must be >= 0

**Storage**:
- SQLite table `captured_responses`
- DashMap cache for recent responses (last 100)

**Indexes**:
- Primary: id
- Secondary: correlation_id (for request-response pairing)
- Secondary: timestamp

---

### 3. ContextAttribution

Metadata identifying which portion of a request context came from which source.

**Fields**:
- `id`: String (UUID) - Unique identifier
- `request_id`: String - Links to CapturedRequest
- `source_type`: SourceType - Enum: User, Framework, McpServer, Skill
- `source_name`: Option<String> - e.g., "context7", "serena", "vectorize" (null for User/Framework)
- `token_count`: usize - Number of tokens from this source
- `content_hash`: String - SHA-256 hash for deduplication
- `message_index`: usize - Position in messages array (0-based)
- `message_role`: String - "user", "assistant", "system" from API request

**Relationships**:
- Many-to-one with CapturedRequest

**SourceType Enum**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceType {
    User,       // Direct user input
    Framework,  // System prompt from Claude Code framework
    McpServer,  // Tool result from MCP server
    Skill,      // Skills like vectorize
}
```

**Validation Rules**:
- `request_id` must reference existing CapturedRequest
- If `source_type` is McpServer or Skill, `source_name` must be Some(...)
- `token_count` must be > 0
- `message_index` must be within bounds of request messages array

**Storage**:
- SQLite table `context_attributions`
- No caching (loaded on-demand when querying requests)

**Indexes**:
- Primary: id
- Secondary: request_id (for loading attributions for a request)
- Secondary: source_name (for querying by MCP server)

---

### 4. QualityFeedback

User-submitted rating and comments for a request-response pair.

**Fields**:
- `id`: String (UUID) - Unique identifier
- `request_id`: String - Links to CapturedRequest
- `response_id`: String - Links to CapturedResponse
- `rating`: f64 - Quality score (-1.0 to 1.0)
- `feedback_text`: Option<String> - Optional user comment
- `user_id`: String - Identifier for user (defaults to "local-user")
- `submitted_at`: DateTime<Utc> - When feedback was submitted

**Relationships**:
- One-to-one with CapturedRequest
- One-to-one with CapturedResponse
- Associated with all ContextAttributions of the request (via request_id)

**Validation Rules**:
- `rating` must be between -1.0 and 1.0 inclusive
- `request_id` and `response_id` must reference existing captures
- `submitted_at` must be >= response timestamp
- Only one QualityFeedback per request-response pair

**Storage**:
- SQLite table `quality_feedback`
- Updates aggregate metrics in `context_source_metrics` table

**Indexes**:
- Primary: id
- Unique: request_id (one feedback per request)
- Secondary: rating (for quality reports)

---

### 5. ContextSourceMetrics

Aggregated statistics for each context source (MCP server, skill, etc.).

**Fields**:
- `source_name`: String - Primary key (e.g., "context7", "serena")
- `source_type`: SourceType - Enum value
- `usage_count`: usize - Number of requests using this source
- `total_tokens`: usize - Total tokens contributed across all requests
- `average_tokens`: f64 - Average tokens per request
- `feedback_count`: usize - Number of feedback submissions for this source
- `average_rating`: f64 - Average quality rating (-1.0 to 1.0)
- `last_used`: DateTime<Utc> - Most recent request using this source
- `created_at`: DateTime<Utc> - First request using this source

**Relationships**:
- Aggregates from ContextAttribution and QualityFeedback

**Validation Rules**:
- `source_name` must be unique
- `usage_count` >= `feedback_count`
- `average_rating` must be between -1.0 and 1.0
- `last_used` >= `created_at`

**Storage**:
- SQLite table `context_source_metrics`
- Updated via trigger or async task when feedback is submitted
- DashMap cache for frequently accessed metrics

**Indexes**:
- Primary: source_name
- Secondary: average_rating (for quality rankings)
- Secondary: usage_count (for popularity)

---

## Entity Relationships

```
CapturedRequest (1) ----<correlation_id>---- (1) CapturedResponse
       |                                            |
       | (1)                                    (1) |
       |                                            |
       +----- (1:1 optional) ---- QualityFeedback --+
       |
       | (1:N)
       |
ContextAttribution
       |
       | (N:1)
       |
ContextSourceMetrics
```

---

## Database Schema (SQLite)

### Table: captured_requests

```sql
CREATE TABLE captured_requests (
    id TEXT PRIMARY KEY,
    timestamp INTEGER NOT NULL,  -- Unix timestamp
    url TEXT NOT NULL,
    method TEXT NOT NULL,
    headers TEXT NOT NULL,  -- JSON
    body BLOB NOT NULL,
    body_json TEXT NOT NULL,  -- JSON
    total_tokens INTEGER NOT NULL,
    correlation_id TEXT UNIQUE NOT NULL,
    FOREIGN KEY (correlation_id) REFERENCES captured_responses(id)
);

CREATE INDEX idx_requests_timestamp ON captured_requests(timestamp);
CREATE INDEX idx_requests_correlation ON captured_requests(correlation_id);
```

### Table: captured_responses

```sql
CREATE TABLE captured_responses (
    id TEXT PRIMARY KEY,
    correlation_id TEXT UNIQUE NOT NULL,
    timestamp INTEGER NOT NULL,
    status_code INTEGER NOT NULL,
    headers TEXT NOT NULL,  -- JSON
    body BLOB NOT NULL,
    body_json TEXT NOT NULL,  -- JSON
    latency_ms INTEGER NOT NULL,
    proxy_latency_ms INTEGER NOT NULL,
    response_tokens INTEGER NOT NULL,
    FOREIGN KEY (correlation_id) REFERENCES captured_requests(id)
);

CREATE INDEX idx_responses_timestamp ON captured_responses(timestamp);
CREATE INDEX idx_responses_correlation ON captured_responses(correlation_id);
```

### Table: context_attributions

```sql
CREATE TABLE context_attributions (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    source_type TEXT NOT NULL,  -- Enum as string
    source_name TEXT,
    token_count INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    message_index INTEGER NOT NULL,
    message_role TEXT NOT NULL,
    FOREIGN KEY (request_id) REFERENCES captured_requests(id) ON DELETE CASCADE
);

CREATE INDEX idx_attributions_request ON context_attributions(request_id);
CREATE INDEX idx_attributions_source ON context_attributions(source_name);
```

### Table: quality_feedback

```sql
CREATE TABLE quality_feedback (
    id TEXT PRIMARY KEY,
    request_id TEXT UNIQUE NOT NULL,
    response_id TEXT NOT NULL,
    rating REAL NOT NULL CHECK(rating >= -1.0 AND rating <= 1.0),
    feedback_text TEXT,
    user_id TEXT NOT NULL,
    submitted_at INTEGER NOT NULL,
    FOREIGN KEY (request_id) REFERENCES captured_requests(id) ON DELETE CASCADE,
    FOREIGN KEY (response_id) REFERENCES captured_responses(id) ON DELETE CASCADE
);

CREATE INDEX idx_feedback_rating ON quality_feedback(rating);
```

### Table: context_source_metrics

```sql
CREATE TABLE context_source_metrics (
    source_name TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    usage_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    average_tokens REAL NOT NULL DEFAULT 0.0,
    feedback_count INTEGER NOT NULL DEFAULT 0,
    average_rating REAL NOT NULL DEFAULT 0.0,
    last_used INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_metrics_rating ON context_source_metrics(average_rating);
CREATE INDEX idx_metrics_usage ON context_source_metrics(usage_count);
```

---

## Rust Type Definitions

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedRequest {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub body_json: serde_json::Value,
    pub total_tokens: usize,
    pub correlation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedResponse {
    pub id: String,
    pub correlation_id: String,
    pub timestamp: DateTime<Utc>,
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub body_json: serde_json::Value,
    pub latency_ms: u64,
    pub proxy_latency_ms: u64,
    pub response_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceType {
    User,
    Framework,
    McpServer,
    Skill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAttribution {
    pub id: String,
    pub request_id: String,
    pub source_type: SourceType,
    pub source_name: Option<String>,
    pub token_count: usize,
    pub content_hash: String,
    pub message_index: usize,
    pub message_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityFeedback {
    pub id: String,
    pub request_id: String,
    pub response_id: String,
    pub rating: f64,
    pub feedback_text: Option<String>,
    pub user_id: String,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSourceMetrics {
    pub source_name: String,
    pub source_type: SourceType,
    pub usage_count: usize,
    pub total_tokens: usize,
    pub average_tokens: f64,
    pub feedback_count: usize,
    pub average_rating: f64,
    pub last_used: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
```

---

## Data Retention Policy

**Retention Period**: 30 days (configurable)

**Cleanup Strategy**:
- Background task runs daily
- Deletes `captured_requests` older than retention period
- CASCADE DELETE removes associated `captured_responses`, `context_attributions`, and `quality_feedback`
- `context_source_metrics` are preserved (aggregates don't have timestamps)

**Storage Estimates**:
- Average request: ~10KB (with context)
- Average response: ~5KB
- 10,000 captures: ~150MB
- With 30-day retention and moderate usage: ~500MB max

---

## Sensitive Data Handling

**Sanitization Rules**:
- API keys removed from headers before storage (Authorization, X-API-Key, etc.)
- Replace with `[REDACTED]` placeholder
- Store hash of API key for correlation, not the key itself
- User messages and responses stored as-is (user owns their data)

**Security**:
- Database file encrypted at rest (OS-level encryption recommended)
- Access via API requires authentication (future enhancement)
- Log redaction for sensitive fields in tracing output

---

**Data Model Complete**: Ready to implement storage layer and API contracts in Phase 1.
