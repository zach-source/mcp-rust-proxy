use mcp_proxy_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisabledServers {
    pub servers: HashSet<String>,
}

impl DisabledServers {
    pub fn new() -> Self {
        Self {
            servers: HashSet::new(),
        }
    }

    pub async fn load() -> Result<Self> {
        let path = Self::state_file_path();

        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path).await?;
        let state: DisabledServers = serde_json::from_str(&content)?;
        Ok(state)
    }

    pub async fn save(&self) -> Result<()> {
        let path = Self::state_file_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content).await?;
        Ok(())
    }

    pub fn is_disabled(&self, server_name: &str) -> bool {
        self.servers.contains(server_name)
    }

    pub async fn toggle(&mut self, server_name: &str) -> Result<bool> {
        let disabled = if self.servers.contains(server_name) {
            self.servers.remove(server_name);
            false
        } else {
            self.servers.insert(server_name.to_string());
            true
        };

        self.save().await?;
        Ok(disabled)
    }

    fn state_file_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".mcp-proxy");
        path.push("disabled-servers.json");
        path
    }
}
