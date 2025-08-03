use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use std::collections::VecDeque;
use crate::transport::{Connection, Transport, TransportType};
use crate::error::Result;

/// Mock transport for testing
pub struct MockTransport {
    pub transport_type: TransportType,
    pub connection: Arc<MockConnection>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            transport_type: TransportType::Stdio,
            connection: Arc::new(MockConnection::new()),
        }
    }
    
    pub fn with_responses(responses: Vec<Bytes>) -> Self {
        Self {
            transport_type: TransportType::Stdio,
            connection: Arc::new(MockConnection::with_responses(responses)),
        }
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn connect(&self) -> Result<Arc<dyn Connection>> {
        Ok(self.connection.clone() as Arc<dyn Connection>)
    }
    
    fn transport_type(&self) -> TransportType {
        self.transport_type
    }
}

/// Mock connection for testing
pub struct MockConnection {
    pub sent_messages: Arc<Mutex<Vec<Bytes>>>,
    pub responses: Arc<Mutex<VecDeque<Bytes>>>,
    pub closed: Arc<RwLock<bool>>,
    pub recv_tx: Arc<Mutex<Option<mpsc::Sender<Bytes>>>>,
    pub recv_rx: Arc<Mutex<Option<mpsc::Receiver<Bytes>>>>,
}

impl MockConnection {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            sent_messages: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(VecDeque::new())),
            closed: Arc::new(RwLock::new(false)),
            recv_tx: Arc::new(Mutex::new(Some(tx))),
            recv_rx: Arc::new(Mutex::new(Some(rx))),
        }
    }
    
    pub fn with_responses(responses: Vec<Bytes>) -> Self {
        let mut conn = Self::new();
        let mut resp_queue = VecDeque::new();
        for resp in responses {
            resp_queue.push_back(resp);
        }
        conn.responses = Arc::new(Mutex::new(resp_queue));
        conn
    }
    
    pub async fn add_response(&self, response: Bytes) {
        self.responses.lock().await.push_back(response.clone());
        if let Some(tx) = self.recv_tx.lock().await.as_ref() {
            let _ = tx.send(response).await;
        }
    }
    
    pub async fn get_sent_messages(&self) -> Vec<Bytes> {
        self.sent_messages.lock().await.clone()
    }
}

#[async_trait]
impl Connection for MockConnection {
    async fn send(&self, data: Bytes) -> Result<()> {
        if *self.closed.read().await {
            return Err(crate::error::TransportError::Closed.into());
        }
        self.sent_messages.lock().await.push(data);
        Ok(())
    }
    
    async fn recv(&self) -> Result<Bytes> {
        if *self.closed.read().await {
            return Err(crate::error::TransportError::Closed.into());
        }
        
        // First check if we have a response in the queue
        if let Some(response) = self.responses.lock().await.pop_front() {
            return Ok(response);
        }
        
        // Otherwise wait for a response on the channel
        let mut rx = self.recv_rx.lock().await;
        if let Some(rx) = rx.as_mut() {
            match rx.recv().await {
                Some(data) => Ok(data),
                None => Err(crate::error::TransportError::Closed.into()),
            }
        } else {
            Err(crate::error::TransportError::Closed.into())
        }
    }
    
    async fn close(&self) -> Result<()> {
        *self.closed.write().await = true;
        // Close the channel
        self.recv_tx.lock().await.take();
        Ok(())
    }
    
    fn is_closed(&self) -> bool {
        // Use try_read to avoid blocking
        self.closed.try_read().map(|g| *g).unwrap_or(false)
    }
}

/// Create a mock MCP server for testing
pub struct MockMcpServer {
    pub connection: Arc<MockConnection>,
    pub handler: Arc<dyn Fn(Bytes) -> Option<Bytes> + Send + Sync>,
}

impl MockMcpServer {
    pub fn new() -> Self {
        Self {
            connection: Arc::new(MockConnection::new()),
            handler: Arc::new(|_| None),
        }
    }
    
    pub fn with_handler<F>(handler: F) -> Self
    where
        F: Fn(Bytes) -> Option<Bytes> + Send + Sync + 'static,
    {
        Self {
            connection: Arc::new(MockConnection::new()),
            handler: Arc::new(handler),
        }
    }
    
    pub async fn run(self) {
        loop {
            match self.connection.recv().await {
                Ok(request) => {
                    if let Some(response) = (self.handler)(request) {
                        let _ = self.connection.send(response).await;
                    }
                }
                Err(_) => break,
            }
        }
    }
}