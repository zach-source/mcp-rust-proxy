use anyhow::Result;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use crate::events::{EventEmitter, ProxyEvent};

const MAX_LOG_LINES: usize = 10000;
const LOG_POLL_INTERVAL_MS: u64 = 100;

pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub message: String,
    pub server: String,
}

pub struct LogManager {
    logs: Arc<RwLock<VecDeque<LogEntry>>>,
    log_dir: PathBuf,
    emitter: EventEmitter,
}

impl LogManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let log_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mcp-proxy")
            .join("logs");

        Self {
            logs: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_LOG_LINES))),
            log_dir,
            emitter: EventEmitter::new(app_handle),
        }
    }

    pub async fn start_streaming(&self, server_name: String) -> Result<()> {
        let log_file = self.log_dir.join(&server_name).join("server.log");

        if !log_file.exists() {
            return Err(anyhow::anyhow!(
                "Log file not found for server: {}",
                server_name
            ));
        }

        let logs = self.logs.clone();
        let emitter = self.emitter.clone();
        let server = server_name.clone();

        tokio::spawn(async move {
            if let Err(e) = tail_log_file(log_file, logs, emitter, server).await {
                tracing::error!("Error tailing log file: {}", e);
            }
        });

        let _ = self.emitter.emit(ProxyEvent::LogStreamStarted {
            server: server_name,
        });

        Ok(())
    }

    pub async fn stop_streaming(&self, server_name: String) -> Result<()> {
        let _ = self.emitter.emit(ProxyEvent::LogStreamStopped {
            server: server_name,
        });
        Ok(())
    }

    pub async fn get_recent_logs(&self, server_name: &str, lines: usize) -> Vec<String> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|entry| entry.server == server_name)
            .rev()
            .take(lines)
            .map(|entry| {
                format!(
                    "[{}] [{}] {}",
                    entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                    entry.level,
                    entry.message
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub async fn clear_logs(&self, server_name: Option<&str>) {
        let mut logs = self.logs.write().await;
        if let Some(server) = server_name {
            logs.retain(|entry| entry.server != server);
        } else {
            logs.clear();
        }
    }
}

async fn tail_log_file(
    path: PathBuf,
    logs: Arc<RwLock<VecDeque<LogEntry>>>,
    emitter: EventEmitter,
    server_name: String,
) -> Result<()> {
    let file = File::open(&path).await?;
    let mut reader = BufReader::new(file);

    // Seek to end of file initially
    reader.seek(std::io::SeekFrom::End(0)).await?;

    let mut line = String::new();
    let mut ticker = interval(Duration::from_millis(LOG_POLL_INTERVAL_MS));

    loop {
        ticker.tick().await;

        while reader.read_line(&mut line).await? > 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                // Parse log line
                let (level, message) = parse_log_line(trimmed);

                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: level.clone(),
                    message: message.clone(),
                    server: server_name.clone(),
                };

                // Store in buffer
                {
                    let mut logs_guard = logs.write().await;
                    if logs_guard.len() >= MAX_LOG_LINES {
                        logs_guard.pop_front();
                    }
                    logs_guard.push_back(entry);
                }

                // Emit event
                let _ = emitter.emit(ProxyEvent::LogEntry {
                    server: server_name.clone(),
                    level,
                    message,
                });
            }
            line.clear();
        }
    }
}

fn parse_log_line(line: &str) -> (String, String) {
    // Try to extract log level from common patterns
    if let Some(pos) = line.find("[ERROR]") {
        return ("ERROR".to_string(), line[pos + 7..].trim().to_string());
    }
    if let Some(pos) = line.find("[WARN]") {
        return ("WARN".to_string(), line[pos + 6..].trim().to_string());
    }
    if let Some(pos) = line.find("[INFO]") {
        return ("INFO".to_string(), line[pos + 6..].trim().to_string());
    }
    if let Some(pos) = line.find("[DEBUG]") {
        return ("DEBUG".to_string(), line[pos + 7..].trim().to_string());
    }

    // Default to INFO level
    ("INFO".to_string(), line.to_string())
}

// Log rotation support
pub struct LogRotator {
    max_size_bytes: u64,
    max_age_days: u32,
    log_dir: PathBuf,
}

impl LogRotator {
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            max_size_bytes: 10 * 1024 * 1024, // 10MB
            max_age_days: 2,
            log_dir,
        }
    }

    pub async fn rotate_if_needed(&self, server_name: &str) -> Result<()> {
        let log_file = self.log_dir.join(server_name).join("server.log");

        if !log_file.exists() {
            return Ok(());
        }

        let metadata = tokio::fs::metadata(&log_file).await?;

        // Check size
        if metadata.len() > self.max_size_bytes {
            self.rotate_file(&log_file).await?;
        }

        // Clean old rotated files
        self.clean_old_logs(server_name).await?;

        Ok(())
    }

    async fn rotate_file(&self, log_file: &PathBuf) -> Result<()> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let rotated_name = format!("{}.{}", log_file.display(), timestamp);
        tokio::fs::rename(log_file, rotated_name).await?;
        Ok(())
    }

    async fn clean_old_logs(&self, server_name: &str) -> Result<()> {
        let server_log_dir = self.log_dir.join(server_name);
        let cutoff = chrono::Utc::now() - chrono::Duration::days(self.max_age_days as i64);

        let mut entries = tokio::fs::read_dir(&server_log_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: chrono::DateTime<chrono::Utc> = modified.into();
                    if modified_time < cutoff {
                        let _ = tokio::fs::remove_file(entry.path()).await;
                    }
                }
            }
        }

        Ok(())
    }
}
