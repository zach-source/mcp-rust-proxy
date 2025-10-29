# Developer Quickstart: Claude API Proxy

**Feature**: 005-claude-api-proxy
**Date**: 2025-10-28

## Overview

This guide provides step-by-step instructions for implementing the Claude API Proxy feature. Follow the phases in order, implementing and testing incrementally.

---

## Prerequisites

Before starting implementation, ensure you have:

1. **Rust Environment**:
   - Rust 1.75+ installed
   - `cargo`, `rustfmt`, `clippy` available
   - Familiarity with async Rust (tokio)

2. **Dependencies Installed**:
   ```bash
   cargo add hyper --features server,client,http1,http2
   cargo add hyper-util
   cargo add rustls --version 0.23
   cargo add rustls-pemfile --version 2
   cargo add rcgen --version 0.13
   cargo add tokio --features full
   cargo add tokio-rustls --version 0.26
   cargo add bytes
   cargo add http
   cargo add tower
   cargo add tracing
   cargo add dashmap --version 6
   cargo add serde_json
   cargo add chrono --features serde
   cargo add uuid --features serde,v4
   ```

3. **Knowledge**:
   - Read [research.md](./research.md) for technical decisions
   - Read [data-model.md](./data-model.md) for entity structure
   - Review [contracts/](./contracts/) for API specifications

---

## Phase 1: Certificate Generation and TLS Setup

### Goal
Implement certificate generation and TLS configuration for MITM proxy.

### Tasks

1. **Create `src/claude_proxy/mod.rs`**:
   ```rust
   pub mod tls_handler;
   pub mod proxy_server;
   pub mod capture;
   pub mod attribution;
   pub mod config;

   pub use tls_handler::TlsHandler;
   pub use proxy_server::ProxyServer;
   pub use config::ClaudeProxyConfig;
   ```

2. **Implement `src/claude_proxy/tls_handler.rs`**:
   - `generate_root_ca()` - One-time CA generation
   - `generate_domain_cert(domain: &str, ca: &Certificate)` - Per-domain cert
   - `load_or_create_ca()` - Load existing or generate new
   - `get_server_config(domain: &str)` - Return rustls::ServerConfig with cached cert
   - `get_client_config()` - Return rustls::ClientConfig for forwarding

   **Key Points**:
   - Store CA in `~/.claude-proxy/ca.crt` and `~/.claude-proxy/ca.key`
   - Cache generated certs in `Arc<DashMap<String, Arc<ServerConfig>>>`
   - Use rcgen for certificate generation
   - Set 1-year validity for CA, 90-day for domain certs

3. **Test Certificate Generation**:
   ```bash
   cargo test tls_handler::tests --test-threads=1
   ```

   Test cases:
   - `test_generate_root_ca()` - Verify CA is valid and serializable
   - `test_generate_domain_cert()` - Verify domain cert is properly signed
   - `test_cert_caching()` - Verify DashMap caching works
   - `test_load_or_create_ca()` - Verify persistence to disk

---

## Phase 2: Basic HTTP Proxy (Without TLS)

### Goal
Implement basic HTTP proxy server that forwards requests without TLS, focusing on capture-and-forward logic.

### Tasks

1. **Create `src/claude_proxy/proxy_server.rs`**:
   ```rust
   pub struct ProxyServer {
       config: ClaudeProxyConfig,
       tls_handler: Arc<TlsHandler>,
       captures: Arc<CaptureStorage>,
   }

   impl ProxyServer {
       pub async fn start(&self) -> Result<(), Error>;
       async fn handle_connection(&self, stream: TcpStream) -> Result<(), Error>;
       async fn proxy_request(&self, req: Request<Body>) -> Result<Response<Body>, Error>;
   }
   ```

2. **Implement Basic Forwarding**:
   - Accept TCP connections on configured port (e.g., 8080 for HTTP testing)
   - Parse HTTP request using hyper
   - Forward to destination (initially just echo back for testing)
   - Return response to client

3. **Test HTTP Proxy**:
   ```bash
   # Start proxy
   cargo run -- --config test-claude-proxy.yaml

   # Test with curl
   curl -x http://localhost:8080 http://httpbin.org/get
   ```

---

## Phase 3: Request/Response Capture

### Goal
Implement capture logic that stores requests and responses to storage.

### Tasks

1. **Create `src/claude_proxy/capture.rs`**:
   ```rust
   pub struct CaptureStorage {
       db: Arc<SqlitePool>,
       cache: Arc<DashMap<String, CapturedRequest>>,
   }

   impl CaptureStorage {
       pub async fn capture_request(&self, req: &Request<Body>) -> Result<String, Error>;
       pub async fn capture_response(&self, resp: &Response<Body>, req_id: &str) -> Result<(), Error>;
       pub async fn get_request(&self, id: &str) -> Result<Option<CapturedRequest>, Error>;
       pub async fn query_requests(&self, filters: QueryFilters) -> Result<Vec<CapturedRequest>, Error>;
   }
   ```

2. **Extend `src/context/storage.rs`**:
   - Add schema migration for claude_proxy tables (see data-model.md)
   - Implement INSERT/SELECT for captured_requests, captured_responses
   - Add indexes as specified in schema

3. **Implement Capture in Proxy Flow**:
   ```rust
   async fn proxy_request(&self, req: Request<Body>) -> Result<Response<Body>, Error> {
       // 1. Capture request (non-blocking)
       let request_id = self.captures.capture_request(&req).await?;

       // 2. Forward to destination
       let response = self.forward_request(req).await?;

       // 3. Capture response (non-blocking)
       self.captures.capture_response(&response, &request_id).await?;

       // 4. Return response
       Ok(response)
   }
   ```

4. **Test Capture Logic**:
   ```bash
   cargo test capture::tests
   ```

   Test cases:
   - `test_capture_request()` - Verify request is stored in DB
   - `test_capture_response()` - Verify response is linked to request
   - `test_query_by_time_range()` - Verify filtering works
   - `test_concurrent_captures()` - Verify no data corruption

---

## Phase 4: Add HTTPS/TLS Support

### Goal
Enable HTTPS proxy with TLS termination and origination.

### Tasks

1. **Integrate TLS into Proxy Server**:
   ```rust
   async fn handle_connection(&self, stream: TcpStream) -> Result<(), Error> {
       // 1. Perform TLS handshake with client
       let sni = extract_sni_from_client_hello(&stream)?;

       // 2. Get server config for domain
       let server_config = self.tls_handler.get_server_config(&sni).await?;

       // 3. Accept TLS connection
       let tls_stream = tokio_rustls::TlsAcceptor::from(server_config)
           .accept(stream)
           .await?;

       // 4. Handle HTTP over TLS
       self.handle_http_connection(tls_stream).await
   }
   ```

2. **Implement Client-Side TLS (for forwarding)**:
   ```rust
   async fn forward_request(&self, req: Request<Body>) -> Result<Response<Body>, Error> {
       let client_config = self.tls_handler.get_client_config();

       // Connect to api.anthropic.com with TLS
       let connector = hyper_rustls::HttpsConnectorBuilder::new()
           .with_tls_config(client_config)
           .https_only()
           .enable_http1()
           .build();

       let client = hyper::Client::builder().build(connector);
       client.request(req).await
   }
   ```

3. **Add Domain Filtering**:
   ```rust
   fn should_intercept(&self, sni: &str) -> bool {
       sni.ends_with("anthropic.com") || sni.ends_with("claude.ai")
   }
   ```

4. **Test HTTPS Proxy**:
   ```bash
   # Install CA cert in system trust store (macOS example)
   sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ~/.claude-proxy/ca.crt

   # Set proxy environment variable
   export HTTPS_PROXY=https://localhost:8443

   # Test with curl
   curl https://api.anthropic.com/v1/models
   ```

---

## Phase 5: Context Source Attribution

### Goal
Parse captured request JSON and identify context sources.

### Tasks

1. **Create `src/claude_proxy/attribution.rs`**:
   ```rust
   pub struct AttributionEngine;

   impl AttributionEngine {
       pub fn analyze_request(request_json: &serde_json::Value) -> Vec<ContextAttribution>;
       fn identify_source_type(message: &serde_json::Value) -> SourceType;
       fn extract_mcp_server_name(tool_use_id: &str) -> Option<String>;
       fn count_tokens(content: &str) -> usize;
   }
   ```

2. **Implement Attribution Rules** (see research.md):
   - Parse `messages` array
   - Identify system prompt as Framework
   - Identify user messages as User
   - Parse tool_result content for MCP server names
   - Count tokens per attribution

3. **Integrate Attribution into Capture**:
   ```rust
   async fn capture_request(&self, req: &Request<Body>) -> Result<String, Error> {
       let body_json: serde_json::Value = serde_json::from_slice(&body_bytes)?;

       // Generate attributions
       let attributions = AttributionEngine::analyze_request(&body_json);

       // Store request with attributions
       let request_id = self.store_request(req, body_json, attributions).await?;

       Ok(request_id)
   }
   ```

4. **Test Attribution Logic**:
   ```bash
   cargo test attribution::tests
   ```

   Test cases:
   - `test_identify_user_message()` - Verify User classification
   - `test_identify_mcp_tool_result()` - Verify McpServer extraction
   - `test_extract_server_name_from_tool_id()` - Verify name parsing
   - `test_token_counting()` - Verify token accuracy

---

## Phase 6: Query API Implementation

### Goal
Implement REST API endpoints for querying captured data.

### Tasks

1. **Extend `src/web/api.rs`**:
   ```rust
   pub fn claude_proxy_routes() -> Router {
       Router::new()
           .route("/api/claude/requests", get(list_requests))
           .route("/api/claude/requests/:id", get(get_request))
           .route("/api/claude/responses/:id", get(get_response))
           .route("/api/claude/contexts", get(query_contexts))
           .route("/api/claude/metrics/sources", get(get_source_metrics))
           .route("/api/claude/metrics/summary", get(get_summary))
   }
   ```

2. **Implement Handlers** (following contracts/query-api.yaml):
   - Parse query parameters
   - Call CaptureStorage methods
   - Serialize to JSON
   - Handle errors with appropriate status codes

3. **Test API Endpoints**:
   ```bash
   # Start server
   cargo run -- --config mcp-proxy-config.yaml

   # Test query
   curl http://localhost:3001/api/claude/requests?limit=10
   curl http://localhost:3001/api/claude/metrics/sources
   ```

---

## Phase 7: Feedback API Implementation

### Goal
Implement feedback submission and aggregate metrics updates.

### Tasks

1. **Create `src/context/feedback.rs`**:
   ```rust
   pub struct FeedbackManager {
       db: Arc<SqlitePool>,
   }

   impl FeedbackManager {
       pub async fn submit_feedback(&self, feedback: QualityFeedback) -> Result<(), Error>;
       pub async fn update_feedback(&self, id: &str, updates: FeedbackUpdate) -> Result<(), Error>;
       pub async fn delete_feedback(&self, id: &str) -> Result<(), Error>;
       async fn update_aggregate_metrics(&self, feedback: &QualityFeedback) -> Result<(), Error>;
   }
   ```

2. **Implement Aggregate Metrics Updates**:
   - When feedback is submitted, find all context_attributions for that request
   - Update context_source_metrics for each affected source
   - Recalculate average_rating, feedback_count

3. **Add Feedback Routes to API**:
   ```rust
   .route("/api/claude/feedback", post(submit_feedback).get(list_feedback))
   .route("/api/claude/feedback/:id", get(get_feedback).put(update_feedback).delete(delete_feedback))
   ```

4. **Test Feedback Flow**:
   ```bash
   # Submit feedback
   curl -X POST http://localhost:3001/api/claude/feedback \
     -H "Content-Type: application/json" \
     -d '{"request_id": "abc123", "rating": 0.8, "feedback_text": "Great context"}'

   # Verify metrics updated
   curl http://localhost:3001/api/claude/metrics/sources?source_name=context7
   ```

---

## Phase 8: Configuration and Integration

### Goal
Add proxy configuration to MCP Rust Proxy config schema and integrate startup.

### Tasks

1. **Extend `src/config/schema.rs`**:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ClaudeProxyConfig {
       pub enabled: bool,
       pub bind_address: String,  // e.g., "127.0.0.1:8443"
       pub ca_cert_path: Option<PathBuf>,  // Optional: use custom CA
       pub capture_enabled: bool,
       pub retention_days: u32,
   }

   // Add to main Config struct
   pub struct Config {
       // ... existing fields
       pub claude_proxy: Option<ClaudeProxyConfig>,
   }
   ```

2. **Update `src/main.rs`**:
   ```rust
   #[tokio::main]
   async fn main() -> Result<(), Box<dyn std::error::Error>> {
       // ... existing initialization

       // Start Claude API proxy if enabled
       if let Some(proxy_config) = &config.claude_proxy {
           if proxy_config.enabled {
               let proxy = ProxyServer::new(proxy_config.clone(), state.clone()).await?;
               tokio::spawn(async move {
                   if let Err(e) = proxy.start().await {
                       tracing::error!("Claude proxy error: {}", e);
                   }
               });
           }
       }

       // ... rest of main
   }
   ```

3. **Create Example Config**:
   ```yaml
   # mcp-proxy-config.yaml
   claude_proxy:
     enabled: true
     bind_address: "127.0.0.1:8443"
     capture_enabled: true
     retention_days: 30
   ```

---

## Phase 9: Testing and Validation

### Goal
Comprehensive testing of the entire proxy system.

### Test Strategy

1. **Unit Tests**:
   - Certificate generation and caching
   - Request/response capture
   - Context attribution logic
   - Feedback aggregation

2. **Integration Tests**:
   - End-to-end proxy flow with mock Claude API
   - Concurrent request handling
   - TLS handshake and forwarding
   - API endpoint functionality

3. **Manual Testing**:
   ```bash
   # 1. Start proxy
   cargo run -- --config mcp-proxy-config.yaml

   # 2. Install CA cert (one-time)
   # macOS
   sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ~/.claude-proxy/ca.crt

   # Linux
   sudo cp ~/.claude-proxy/ca.crt /usr/local/share/ca-certificates/claude-proxy.crt
   sudo update-ca-certificates

   # 3. Configure Claude CLI to use proxy
   export HTTPS_PROXY=https://localhost:8443

   # 4. Use Claude CLI normally
   claude "What is Rust?"

   # 5. Query captured data
   curl http://localhost:3001/api/claude/requests?limit=1
   curl http://localhost:3001/api/claude/metrics/summary

   # 6. Submit feedback
   curl -X POST http://localhost:3001/api/claude/feedback \
     -H "Content-Type: application/json" \
     -d '{"request_id": "<from query>", "rating": 0.9}'
   ```

4. **Performance Testing**:
   - Measure latency overhead (should be <100ms)
   - Test with 100 concurrent requests
   - Verify no memory leaks over extended run

---

## Common Issues and Solutions

### Issue: TLS Handshake Failures
**Solution**: Verify CA cert is installed in system trust store. Check `~/.claude-proxy/ca.crt` exists and is valid.

### Issue: Proxy Not Intercepting Traffic
**Solution**: Ensure `HTTPS_PROXY` environment variable is set. Verify proxy is listening on configured port with `netstat -an | grep 8443`.

### Issue: Context Attribution Missing MCP Server Names
**Solution**: Check that tool_use_id format matches expectations (e.g., `mcp__proxy__context7__tool_name`). Update parsing logic if format differs.

### Issue: High Latency
**Solution**:
- Verify certificate caching is working (check DashMap)
- Enable connection pooling for forwarding client
- Profile with `cargo flamegraph` to find bottlenecks

---

## Next Steps

After completing all phases:

1. Run full test suite: `cargo test`
2. Run clippy: `cargo clippy`
3. Format code: `cargo fmt`
4. Update CLAUDE.md with proxy documentation
5. Create PR for review

---

**Implementation Tips**:

- Work incrementally - each phase builds on the previous
- Test after each phase before moving forward
- Use tracing logs liberally for debugging
- Commit working code frequently
- Follow existing code patterns in MCP Rust Proxy

**Resources**:
- [research.md](./research.md) - Technical decisions and library choices
- [data-model.md](./data-model.md) - Database schema and entity definitions
- [contracts/](./contracts/) - API specifications (OpenAPI)
- [MCP Rust Proxy CLAUDE.md](../../../CLAUDE.md) - Project development guide
