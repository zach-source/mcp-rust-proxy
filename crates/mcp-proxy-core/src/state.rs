// Temporary stub types - will be moved to server crate
use chrono::{DateTime, Utc};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}

#[derive(Debug)]
pub struct ServerInfo {
    pub name: String,
    pub logger: Option<Arc<ServerLogger>>,
}

impl ServerInfo {
    pub fn broadcast_log(&self, _entry: LogEntry) {
        // Stub implementation
    }
}

#[derive(Debug)]
pub struct ServerLogger;

impl ServerLogger {
    pub async fn write_stderr(&self, _line: &str) -> Result<(), std::io::Error> {
        Ok(())
    }

    pub async fn write_stdout(&self, _line: &str) -> Result<(), std::io::Error> {
        Ok(())
    }
}
