# Research: Claude API Proxy Technical Decisions

**Feature**: 005-claude-api-proxy
**Date**: 2025-10-28
**Status**: Complete

## Overview

This document captures technical research and decisions made during Phase 0 planning for the Claude API Proxy feature. All NEEDS CLARIFICATION items from the Technical Context have been resolved.

---

## Research Question 1: HTTPS Proxy Implementation Libraries

### Decision

Use **hyper** as the core HTTP library with **rustls** for TLS handling, supplemented by **rcgen** for certificate generation.

### Rationale

- **hyper** provides lowest-level control needed for proxy implementation
- Excellent performance with full HTTP/1.1 and HTTP/2 support
- **rustls** offers pure-Rust TLS implementation with better security defaults than OpenSSL
- Avoids native dependencies while maintaining high performance
- **rcgen** provides simple, efficient X.509 certificate generation for MITM CA

### Alternatives Considered

1. **reqwest** - Higher-level HTTP client, excellent for forwarding but less suitable for implementing the proxy server itself. Could be used for the Claude API forwarding portion.

2. **http-mitm-proxy crate** - Provides ready-made MITM proxy functionality but may be too opinionated for custom capture/analysis needs. Useful as reference implementation.

3. **OpenSSL vs rustls** - OpenSSL has wider compatibility but requires native dependencies. Rustls is pure Rust with better defaults and seamless hyper integration.

### Implementation Notes

**Key Dependencies**:
```toml
[dependencies]
hyper = { version = "1", features = ["server", "client", "http1", "http2"] }
hyper-util = "0.1"
rustls = "0.23"
rustls-pemfile = "2"
rcgen = "0.13"
tokio = { version = "1", features = ["full"] }
tokio-rustls = "0.26"
bytes = "1"
http = "1"
tower = "0.5"
tracing = "0.1"
dashmap = "6"  # For certificate cache
```

**Architecture**:
- Client-facing side: Accept connections from Claude CLI using hyper server + rustls
- Server-facing side: Forward to api.anthropic.com using hyper client + rustls
- Two-sided TLS termination and origination

---

## Research Question 2: TLS/SSL Interception Strategy

### Decision

Generate a **root CA certificate on first run**, then generate **per-domain certificates on-demand**, signed by the CA and cached in memory.

### Rationale

- One-time CA generation minimizes setup complexity
- Per-domain certificates provide proper SNI handling
- Caching avoids regeneration overhead
- Users install CA cert once in system trust store

### Certificate Generation Approach

```rust
// Root CA generation (one-time, persisted to disk)
use rcgen::{CertificateParams, DistinguishedName, KeyPair, IsCa, BasicConstraints};

let mut params = CertificateParams::new(vec!["Claude Proxy CA".to_string()]);
params.distinguished_name = DistinguishedName::new();
params.distinguished_name.push(rcgen::DnType::CommonName, "Claude Proxy CA");
params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

// Generate per-domain certificates on-demand for api.anthropic.com
// Sign with CA, cache in DashMap<String, Certificate>
```

### TLS Interception Flow

1. Listen on local port (e.g., localhost:8443)
2. Accept TLS connection from Claude CLI using generated certificate for api.anthropic.com
3. Decrypt request, capture complete payload for analysis
4. Re-encrypt and forward to real api.anthropic.com with proper authentication headers
5. Decrypt response from Claude API, capture complete payload
6. Re-encrypt and forward back to Claude CLI

### Trust Establishment

**Option A (Recommended)**: Install CA certificate in system trust store
- macOS: Add to Keychain Access
- Linux: Add to /usr/local/share/ca-certificates/
- Windows: Add to Trusted Root Certification Authorities

**Option B**: Environment variable pointing to CA bundle
- Set REQUESTS_CA_BUNDLE or SSL_CERT_FILE
- More portable but requires user configuration

### Performance Optimizations

- **Certificate caching**: Store generated certificates in `Arc<DashMap<String, ServerConfig>>`
- **Connection pooling**: Reuse TLS connections to api.anthropic.com
- **Zero-copy**: Use `Bytes` type for body streaming to avoid unnecessary allocations

---

## Research Question 3: Proxy Configuration Methods

### Decision

Support **environment variable configuration** (HTTP_PROXY/HTTPS_PROXY) as primary method, with optional system proxy integration.

### Rationale

- Environment variables are standard across platforms and tools
- Claude CLI likely respects HTTP_PROXY/HTTPS_PROXY by default
- No modification to Claude CLI binary required
- Easy to enable/disable (set/unset variable)

### Configuration Approach

**Primary Method - Environment Variables**:
```bash
# User sets before running Claude CLI
export HTTPS_PROXY=https://localhost:8443
export HTTP_PROXY=http://localhost:8080  # Fallback for HTTP

# Optional: Point to CA bundle if not in system trust store
export REQUESTS_CA_BUNDLE=~/.claude-proxy/ca.crt
```

**Traffic Filtering**:
- Check SNI (Server Name Indication) during TLS handshake
- Check Host header in HTTP request
- Only intercept traffic destined for `api.anthropic.com`
- Pass through all other HTTPS traffic unchanged

**Port Selection**:
- Default: localhost:8443 (HTTPS proxy)
- Configurable via `ClaudeProxyConfig` in mcp-proxy-config.yaml
- Bind only to localhost for security (no external access)

### Detecting Claude API Traffic

```rust
// During TLS handshake
if let Some(sni) = client_hello.server_name() {
    if sni.ends_with("anthropic.com") || sni.ends_with("claude.ai") {
        // Route through capture-and-forward logic
    } else {
        // Pass through transparently
    }
}
```

---

## Research Question 4: Capture-and-Forward Pattern Best Practices

### Decision

Use **async buffering** with **atomic capture** operations and **fail-open error handling**.

### Rationale

- Async patterns prevent blocking on I/O during capture/forward
- Atomic capture ensures no partial data if capture fails
- Fail-open maintains Claude CLI functionality even if proxy has issues
- Zero-copy where possible minimizes latency overhead

### Implementation Pattern

```rust
enum ProxyAction {
    Forward,      // Normal proxy operation with capture
    Passthrough,  // If capture fails, still forward unchanged
    Block,        // For non-Claude traffic (shouldn't happen with filtering)
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Error> {
    // 1. Attempt to capture request
    let capture_result = capture_request(&req).await;

    // 2. Forward regardless of capture success (fail-open)
    let response = forward_to_claude_api(req).await?;

    // 3. Attempt to capture response
    let capture_response_result = capture_response(&response).await;

    // 4. Store captures if both succeeded
    if let (Ok(captured_req), Ok(captured_resp)) = (capture_result, capture_response_result) {
        store_capture_pair(captured_req, captured_resp).await;
    }

    // 5. Return response to CLI (even if capture failed)
    Ok(response)
}
```

### Performance Optimizations

**Buffering Strategy**:
- Use `hyper::body::to_bytes()` to buffer request/response bodies
- Clone body bytes for capture (cheap with `Bytes` type - Arc internally)
- Stream to destination without waiting for capture completion

**Concurrency**:
- Tokio tasks for parallel capture and forward operations
- Connection pooling to api.anthropic.com (reuse TLS sessions)
- DashMap for lock-free concurrent access to captures

**Error Handling**:
- Log capture failures with tracing::warn!
- Never block forward operations due to capture issues
- Provide metric for capture success rate

---

## Research Question 5: Context Source Attribution

### Decision

Parse Claude API request body (JSON) to identify context boundaries based on **message roles** and **system prompt markers**.

### Rationale

- Claude API uses structured JSON format with clear message roles
- System prompts are distinct from user messages
- MCP tool results have identifiable structure
- Can infer source from message content patterns

### Attribution Strategy

**Request Structure** (Claude Messages API):
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "messages": [
    {"role": "user", "content": "..."},
    {"role": "assistant", "content": "..."},
    {"role": "user", "content": [
      {"type": "tool_result", "tool_use_id": "...", "content": "..."}
    ]}
  ],
  "system": "You are Claude Code with access to...",
  "tools": [...]
}
```

**Attribution Rules**:
1. **System Prompt**: Identify as "Framework: Claude Code" (from `system` field)
2. **User Messages**: Identify as "User Input" (role=user, no tool_result)
3. **Tool Results**: Parse `tool_use_id` to extract MCP server name from tool name prefix
   - Example: `mcp__proxy__context7__get_docs` → "MCP: context7"
4. **Skills/Vectorize**: Identify by checking for vector search markers in system prompt

**Token Counting**:
- Use `tiktoken-rs` or similar to count tokens per message
- Attribute tokens to identified source
- Aggregate for metrics

**Implementation Notes**:
```rust
struct ContextAttribution {
    source_type: SourceType,  // User, MCP, Skill, Framework
    source_name: Option<String>,  // e.g., "context7", "serena", "vectorize"
    token_count: usize,
    content_hash: String,  // For deduplication
}

enum SourceType {
    User,
    Framework,  // System prompt from Claude Code
    McpServer,  // MCP tool result
    Skill,      // Skills like vectorize
}
```

---

## Testing Strategy

### Unit Tests
- Certificate generation and caching
- Request/response capture logic
- Context attribution parsing
- Token counting accuracy

### Integration Tests
- Mock Claude API server (returns canned responses)
- Verify end-to-end flow: CLI → Proxy → Mock API → Proxy → CLI
- Test authentication pass-through
- Test concurrent requests

### Manual Testing
- Use curl with --proxy flag to test basic forwarding
- Use actual Claude CLI with HTTPS_PROXY set
- Verify captured data in storage
- Monitor latency with tracing logs

---

## Open Questions / Future Enhancements

**Resolved in this research**:
- ✅ Primary Dependencies (hyper, rustls, rcgen)
- ✅ TLS/SSL Handling (CA + per-domain certs)
- ✅ Proxy Configuration (environment variables)
- ✅ Capture-and-forward pattern (fail-open async)
- ✅ Context attribution (parse JSON, identify sources)

**Future Enhancements** (Out of scope for initial implementation):
- Support for HTTP/3 (QUIC protocol)
- Distributed tracing integration (OpenTelemetry)
- Real-time streaming capture (for large responses)
- Compression handling (transparent decompression for capture)

---

**Research Complete**: All technical unknowns from Phase 0 have been resolved. Ready to proceed to Phase 1 (Design & Contracts).
