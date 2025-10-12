use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Runtime overrides for server configuration (per-project)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerOverrides {
    /// Project directory this override file belongs to
    pub project: PathBuf,

    /// Per-server overrides
    pub overrides: HashMap<String, ServerOverride>,

    /// Last modification timestamp
    pub last_modified: DateTime<Utc>,
}

/// Override settings for a specific server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerOverride {
    /// Override enabled state (None = use base config)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    // Future: add more overridable fields
    // pub max_restarts: Option<u32>,
    // pub initialization_delay_ms: Option<u64>,
}

impl ServerOverrides {
    /// Create empty overrides for a project
    pub fn new(project: PathBuf) -> Self {
        Self {
            project,
            overrides: HashMap::new(),
            last_modified: Utc::now(),
        }
    }

    /// Load overrides from file
    pub async fn load(path: &Path) -> crate::error::Result<Self> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            crate::error::ConfigError::Parse(format!("Failed to read overrides file: {}", e))
        })?;

        serde_json::from_str(&content).map_err(|e| {
            crate::error::ConfigError::Parse(format!("Failed to parse overrides JSON: {}", e))
                .into()
        })
    }

    /// Save overrides to file
    pub async fn save(&self, path: &Path) -> crate::error::Result<()> {
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            crate::error::ConfigError::Parse(format!("Failed to serialize overrides: {}", e))
        })?;

        tokio::fs::write(path, content).await.map_err(|e| {
            crate::error::ConfigError::Parse(format!("Failed to write overrides file: {}", e))
        })?;

        tracing::info!("Saved server overrides to {}", path.display());
        Ok(())
    }

    /// Set enabled override for a server
    pub fn set_enabled(&mut self, server_name: String, enabled: bool) {
        self.overrides
            .entry(server_name)
            .or_insert_with(|| ServerOverride { enabled: None })
            .enabled = Some(enabled);
        self.last_modified = Utc::now();
    }

    /// Remove override for a server (revert to base config)
    pub fn remove_override(&mut self, server_name: &str) {
        self.overrides.remove(server_name);
        self.last_modified = Utc::now();
    }

    /// Get effective enabled state (override or None if using base)
    pub fn get_enabled(&self, server_name: &str) -> Option<bool> {
        self.overrides.get(server_name).and_then(|o| o.enabled)
    }
}

/// Apply overrides to base configuration
pub fn apply_overrides(config: &mut super::schema::Config, overrides: &ServerOverrides) {
    for (server_name, override_settings) in &overrides.overrides {
        if let Some(server_config) = config.servers.get_mut(server_name) {
            if let Some(enabled) = override_settings.enabled {
                tracing::debug!(
                    "Applying override for server '{}': enabled = {}",
                    server_name,
                    enabled
                );
                server_config.enabled = enabled;
            }
        }
    }
}

/// Detect current project directory
pub fn detect_project_dir() -> crate::error::Result<PathBuf> {
    // Try environment variable first
    if let Ok(project) = std::env::var("MCP_PROXY_PROJECT_DIR") {
        return Ok(PathBuf::from(project));
    }

    // Fall back to current working directory
    std::env::current_dir().map_err(|e| {
        crate::error::ConfigError::Parse(format!("Failed to get current directory: {}", e)).into()
    })
}

/// Get path to overrides file for current project
pub fn get_overrides_path() -> crate::error::Result<PathBuf> {
    let project_dir = detect_project_dir()?;
    Ok(project_dir.join(".mcp-proxy-overrides.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_overrides_creation() {
        let overrides = ServerOverrides::new(PathBuf::from("/test/project"));
        assert_eq!(overrides.project, PathBuf::from("/test/project"));
        assert_eq!(overrides.overrides.len(), 0);
    }

    #[test]
    fn test_set_enabled_override() {
        let mut overrides = ServerOverrides::new(PathBuf::from("/test"));
        overrides.set_enabled("serena".to_string(), false);

        assert_eq!(overrides.get_enabled("serena"), Some(false));
        assert_eq!(overrides.get_enabled("other"), None);
    }

    #[test]
    fn test_remove_override() {
        let mut overrides = ServerOverrides::new(PathBuf::from("/test"));
        overrides.set_enabled("serena".to_string(), false);
        assert_eq!(overrides.get_enabled("serena"), Some(false));

        overrides.remove_override("serena");
        assert_eq!(overrides.get_enabled("serena"), None);
    }
}
