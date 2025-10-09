use super::{Connection, Transport, TransportType};
use crate::error::{Result, TransportError};
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct HttpSseTransport {
    url: String,
    headers: HashMap<String, String>,
    timeout_ms: u64,
}

impl HttpSseTransport {
    pub fn new(url: String, headers: HashMap<String, String>, timeout_ms: u64) -> Self {
        Self {
            url,
            headers,
            timeout_ms,
        }
    }
}

#[async_trait]
impl Transport for HttpSseTransport {
    async fn connect(&self) -> Result<Arc<dyn Connection>> {
        // TODO: Implement SSE connection
        // This is a placeholder implementation
        Ok(Arc::new(HttpSseConnection {
            url: self.url.clone(),
            client: Arc::new(reqwest::Client::new()),
            closed: Arc::new(AtomicBool::new(false)),
        }))
    }

    fn transport_type(&self) -> TransportType {
        TransportType::HttpSse
    }
}

pub struct HttpSseConnection {
    url: String,
    client: Arc<reqwest::Client>,
    closed: Arc<AtomicBool>,
}

#[async_trait]
impl Connection for HttpSseConnection {
    async fn send(&self, data: Bytes) -> Result<()> {
        if self.is_closed() {
            return Err(TransportError::Closed.into());
        }

        // TODO: Implement actual SSE send
        let _response = self
            .client
            .post(&format!("{}/message", self.url))
            .body(data)
            .send()
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;

        Ok(())
    }

    async fn recv(&self) -> Result<Bytes> {
        if self.is_closed() {
            return Err(TransportError::Closed.into());
        }

        // TODO: Implement actual SSE receive
        Err(TransportError::InvalidFormat.into())
    }

    async fn close(&self) -> Result<()> {
        self.closed.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}
