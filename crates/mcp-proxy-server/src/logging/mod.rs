use chrono::{DateTime, Local, Utc};
use mcp_proxy_core::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024; // 10MB
const LOG_ROTATION_DAYS: i64 = 2;

pub struct ServerLogger {
    log_dir: PathBuf,
    server_name: String,
    log_writer: Arc<Mutex<File>>,
    current_log_size: Arc<Mutex<u64>>,
}

impl ServerLogger {
    pub async fn new(server_name: String, base_log_dir: Option<PathBuf>) -> Result<Self> {
        // Use provided directory or default to ~/.mcp-proxy/logs
        let log_dir = if let Some(dir) = base_log_dir {
            dir
        } else {
            let home = dirs::home_dir().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine home directory",
                )
            })?;
            home.join(".mcp-proxy").join("logs")
        };

        // Create log directory if it doesn't exist
        fs::create_dir_all(&log_dir)?;

        // Create server-specific directory
        let server_log_dir = log_dir.join(&server_name);
        fs::create_dir_all(&server_log_dir)?;

        // Clean up old logs
        Self::cleanup_old_logs(&server_log_dir).await?;

        // Create initial combined log file
        let log_path = Self::get_log_path(&server_log_dir);

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .await?;

        // Get initial file size
        let log_size = log_file.metadata().await?.len();

        Ok(Self {
            log_dir: server_log_dir,
            server_name,
            log_writer: Arc::new(Mutex::new(log_file)),
            current_log_size: Arc::new(Mutex::new(log_size)),
        })
    }

    pub async fn write_stdout(&self, data: &str) -> Result<()> {
        self.write_log(data, "STDOUT").await
    }

    pub async fn write_stderr(&self, data: &str) -> Result<()> {
        self.write_log(data, "STDERR").await
    }

    async fn write_log(&self, data: &str, stream_type: &str) -> Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!("[{}] [{}] {}\n", timestamp, stream_type, data);
        let log_bytes = log_line.as_bytes();

        let mut size = self.current_log_size.lock().await;

        // Check if rotation is needed
        if *size + log_bytes.len() as u64 > MAX_LOG_SIZE {
            // Rotate the log file
            let mut writer_guard = self.log_writer.lock().await;
            writer_guard.flush().await?;
            drop(writer_guard);

            self.rotate_log().await?;

            // Create new file
            let new_path = Self::get_log_path(&self.log_dir);
            let new_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&new_path)
                .await?;

            let mut writer_guard = self.log_writer.lock().await;
            *writer_guard = new_file;
            *size = 0;
        }

        // Write the log line
        let mut writer_guard = self.log_writer.lock().await;
        writer_guard.write_all(log_bytes).await?;
        writer_guard.flush().await?;

        *size += log_bytes.len() as u64;

        Ok(())
    }

    async fn rotate_log(&self) -> Result<()> {
        let current_path = Self::get_log_path(&self.log_dir);
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let rotated_path = self
            .log_dir
            .join(format!("{}.{}.log", self.server_name, timestamp));

        if current_path.exists() {
            tokio::fs::rename(&current_path, &rotated_path).await?;
        }

        Ok(())
    }

    async fn cleanup_old_logs(log_dir: &Path) -> Result<()> {
        let cutoff_time = Utc::now() - chrono::Duration::days(LOG_ROTATION_DAYS);

        let mut entries = tokio::fs::read_dir(log_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        let modified_time: DateTime<Utc> = modified.into();
                        if modified_time < cutoff_time {
                            // Delete old log file
                            let _ = tokio::fs::remove_file(&path).await;
                            tracing::debug!("Deleted old log file: {:?}", path);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn get_log_path(log_dir: &Path) -> PathBuf {
        log_dir.join("server.log")
    }

    pub async fn flush(&self) -> Result<()> {
        let mut log_writer = self.log_writer.lock().await;
        log_writer.flush().await?;
        Ok(())
    }
}
