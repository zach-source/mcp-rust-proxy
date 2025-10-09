#[cfg(test)]
use crate::error::Result;
#[cfg(test)]
use crate::protocol::{JsonRpcId, JsonRpcMessage, JsonRpcResponse, JsonRpcV2Message};
#[cfg(test)]
use crate::transport::{Connection, Transport, TransportType};
#[cfg(test)]
use bytes::Bytes;
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use tokio::sync::{mpsc, RwLock};

#[cfg(test)]
pub struct MockConnection {
    // Channel for sending requests from proxy to mock server
    pub request_tx: mpsc::UnboundedSender<Bytes>,
    pub request_rx: Arc<RwLock<mpsc::UnboundedReceiver<Bytes>>>,
    // Channel for sending responses from mock server to proxy
    pub response_tx: mpsc::UnboundedSender<Bytes>,
    pub response_rx: Arc<RwLock<mpsc::UnboundedReceiver<Bytes>>>,
}

#[cfg(test)]
impl MockConnection {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        Self {
            request_tx,
            request_rx: Arc::new(RwLock::new(request_rx)),
            response_tx,
            response_rx: Arc::new(RwLock::new(response_rx)),
        }
    }

    pub async fn add_response(&self, data: Bytes) {
        let _ = self.response_tx.send(data);
    }

    // Helper method to automatically handle initialization
    pub fn with_auto_initialize(self: Arc<Self>) -> Arc<Self> {
        let conn = self.clone();
        tokio::spawn(async move {
            loop {
                let mut rx = conn.request_rx.write().await;
                match rx.recv().await {
                    Some(request) => {
                        drop(rx); // Drop the lock before processing
                        let request_str = std::str::from_utf8(&request).unwrap();
                        if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(request_str.trim())
                        {
                            if let JsonRpcMessage::V2(JsonRpcV2Message::Request(req)) = msg {
                                if req.method == "initialize" {
                                    // Send initialize response
                                    let response = JsonRpcMessage::V2(JsonRpcV2Message::Response(
                                        JsonRpcResponse {
                                            id: req.id,
                                            result: Some(serde_json::json!({
                                                "protocolVersion": "0.1.0",
                                                "capabilities": {},
                                                "serverInfo": {
                                                    "name": "mock-server",
                                                    "version": "0.1.0"
                                                }
                                            })),
                                            error: None,
                                        },
                                    ));

                                    let response_json =
                                        format!("{}\n", serde_json::to_string(&response).unwrap());
                                    conn.add_response(Bytes::from(response_json)).await;
                                }
                            }
                            // Ignore initialized notification
                        }
                    }
                    None => break,
                }
            }
        });
        self
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl Connection for MockConnection {
    async fn send(&self, data: Bytes) -> Result<()> {
        let _ = self.request_tx.send(data);
        Ok(())
    }

    async fn recv(&self) -> Result<Bytes> {
        let mut rx = self.response_rx.write().await;
        rx.recv().await.ok_or_else(|| {
            crate::error::TransportError::ConnectionFailed("Connection closed".to_string()).into()
        })
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }

    fn is_closed(&self) -> bool {
        false
    }
}

#[cfg(test)]
pub struct MockTransport {
    pub transport_type: TransportType,
    pub connection: Arc<MockConnection>,
}

#[cfg(test)]
#[async_trait::async_trait]
impl Transport for MockTransport {
    async fn connect(&self) -> Result<Arc<dyn Connection>> {
        Ok(self.connection.clone() as Arc<dyn Connection>)
    }

    fn transport_type(&self) -> TransportType {
        self.transport_type.clone()
    }
}
