//! HTTPS proxy server for intercepting Claude API traffic
//!
//! This module implements a transparent HTTPS proxy that captures and forwards
//! Claude API requests while maintaining authentication and security.

use crate::claude_proxy::{
    capture::CaptureStorage, config::ClaudeProxyConfig, tls_handler::TlsHandler,
};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Forwarding error: {0}")]
    Forward(String),

    #[error("Capture error: {0}")]
    Capture(String),
}

/// Claude API proxy server
pub struct ProxyServer {
    config: ClaudeProxyConfig,
    tls_handler: Arc<TlsHandler>,
    capture_storage: Arc<CaptureStorage>,
}

impl ProxyServer {
    /// Create a new proxy server instance
    pub fn new(
        config: ClaudeProxyConfig,
        tls_handler: Arc<TlsHandler>,
        capture_storage: Arc<CaptureStorage>,
    ) -> Self {
        Self {
            config,
            tls_handler,
            capture_storage,
        }
    }

    /// Start the proxy server
    pub async fn start(self: Arc<Self>) -> Result<(), ProxyError> {
        let addr: SocketAddr = self.config.bind_address.parse().map_err(|e| {
            ProxyError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
        })?;

        let listener = TcpListener::bind(addr).await?;

        tracing::info!(
            bind_address = %addr,
            "Claude API proxy server started"
        );

        loop {
            let (stream, peer_addr) = listener.accept().await?;

            tracing::debug!(
                peer_addr = %peer_addr,
                "Accepted connection"
            );

            let server = self.clone();
            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(stream).await {
                    tracing::warn!(
                        error = %e,
                        peer_addr = %peer_addr,
                        "Connection handling error"
                    );
                }
            });
        }
    }

    /// Handle an incoming TCP connection
    async fn handle_connection(&self, stream: TcpStream) -> Result<(), ProxyError> {
        // TODO: Extract SNI properly - for now, assume api.anthropic.com
        let domain = "api.anthropic.com";

        // Check if we should intercept this domain
        if !Self::should_intercept(domain) {
            tracing::debug!(domain = domain, "Skipping non-Claude API traffic");
            return Ok(());
        }

        // Get TLS server config for domain
        let server_config = self
            .tls_handler
            .get_server_config(domain)
            .map_err(|e| ProxyError::Tls(e.to_string()))?;

        // Accept TLS connection
        let acceptor = TlsAcceptor::from(server_config);
        let tls_stream = acceptor
            .accept(stream)
            .await
            .map_err(|e| ProxyError::Tls(e.to_string()))?;

        tracing::debug!("TLS handshake completed");

        // Handle HTTP over TLS
        let io = TokioIo::new(tls_stream);

        let server = Arc::new(self.clone());
        let service = service_fn(move |req| {
            let server = server.clone();
            async move { server.proxy_request(req).await }
        });

        if let Err(e) = hyper::server::conn::http1::Builder::new()
            .serve_connection(io, service)
            .await
        {
            tracing::warn!(error = %e, "HTTP connection error");
        }

        Ok(())
    }

    /// Proxy a request (capture, forward, capture response)
    async fn proxy_request(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, ProxyError> {
        let start_time = Instant::now();

        // Extract request details
        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let headers: std::collections::HashMap<String, String> = req
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Read request body
        let body_bytes = req
            .into_body()
            .collect()
            .await
            .map_err(|e| ProxyError::Http(e.to_string()))?
            .to_bytes();

        // Capture request (fail-open)
        let capture_start = Instant::now();
        let correlation_id = if self.config.capture_enabled {
            self.capture_storage
                .capture_request(
                    uri.clone(),
                    method.clone(),
                    headers.clone(),
                    body_bytes.to_vec(),
                )
                .await
                .ok()
        } else {
            None
        };
        let capture_latency = capture_start.elapsed();

        // Forward to Claude API
        let response = self
            .forward_to_claude_api(&method, &uri, &headers, body_bytes.clone())
            .await?;

        let total_latency = start_time.elapsed();

        // Capture response (fail-open)
        if let Some(corr_id) = correlation_id {
            let response_headers: std::collections::HashMap<String, String> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            let _ = self
                .capture_storage
                .capture_response(
                    &corr_id,
                    response.status().as_u16(),
                    response_headers,
                    body_bytes.to_vec(),
                    total_latency.as_millis() as u64,
                    capture_latency.as_millis() as u64,
                )
                .await;
        }

        Ok(response)
    }

    /// Forward request to actual Claude API
    async fn forward_to_claude_api(
        &self,
        method: &str,
        uri: &str,
        headers: &std::collections::HashMap<String, String>,
        body: Bytes,
    ) -> Result<Response<Full<Bytes>>, ProxyError> {
        // Build real Claude API URL
        let url = if uri.starts_with("http") {
            uri.to_string()
        } else {
            format!("https://api.anthropic.com{}", uri)
        };

        tracing::debug!(
            method = method,
            url = %url,
            "Forwarding request to Claude API"
        );

        // Create HTTP client (reqwest uses rustls by default in recent versions)
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| ProxyError::Forward(e.to_string()))?;

        // Build request
        let mut request_builder = match method {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "DELETE" => client.delete(&url),
            "PATCH" => client.patch(&url),
            _ => client.post(&url),
        };

        // Add headers (preserving authentication)
        for (key, value) in headers {
            if key.to_lowercase() != "host" {
                // Skip host header
                request_builder = request_builder.header(key, value);
            }
        }

        // Add body
        request_builder = request_builder.body(body.to_vec());

        // Send request
        let response = request_builder
            .send()
            .await
            .map_err(|e| ProxyError::Forward(e.to_string()))?;

        // Convert reqwest::Response to hyper::Response
        let status = response.status();
        let headers = response.headers().clone();
        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| ProxyError::Forward(e.to_string()))?;

        let mut hyper_response = Response::builder()
            .status(status)
            .body(Full::new(body_bytes))
            .map_err(|e| ProxyError::Http(e.to_string()))?;

        // Copy headers
        for (key, value) in headers.iter() {
            hyper_response.headers_mut().insert(key, value.clone());
        }

        tracing::debug!(
            status = status.as_u16(),
            "Received response from Claude API"
        );

        Ok(hyper_response)
    }

    /// Check if a domain should be intercepted (only Claude API traffic)
    fn should_intercept(domain: &str) -> bool {
        domain.ends_with("anthropic.com") || domain.ends_with("claude.ai")
    }
}

impl Clone for ProxyServer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            tls_handler: self.tls_handler.clone(),
            capture_storage: self.capture_storage.clone(),
        }
    }
}
