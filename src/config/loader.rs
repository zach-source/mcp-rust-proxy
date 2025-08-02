use figment::{Figment, providers::{Format, Toml, Json, Yaml, Env}};
use crate::error::{ConfigError, Result};
use super::schema::Config;
use std::path::Path;

pub async fn load_from_env_or_file() -> Result<Config> {
    let config: Config = Figment::new()
        // Try to load from various config files
        .merge(Toml::file("mcp-proxy.toml"))
        .merge(Json::file("mcp-proxy.json"))
        .merge(Yaml::file("mcp-proxy.yaml"))
        .merge(Yaml::file("mcp-proxy.yml"))
        // Override with environment variables (MCP_PROXY_ prefix)
        .merge(Env::prefixed("MCP_PROXY_").split("_"))
        // Allow individual server env vars (MCP_SERVER_<NAME>_*)
        .merge(Env::raw().filter(|k| k.starts_with("MCP_SERVER_")))
        .extract()
        .map_err(|e| ConfigError::Parse(e.to_string()))?;
    
    // Validate configuration
    validate(&config)?;
    
    // Apply environment variable substitutions
    let config = apply_env_substitutions(config)?;
    
    Ok(config)
}

pub fn validate(config: &Config) -> Result<()> {
    // Validate ports
    if config.proxy.port == config.web_ui.port && config.web_ui.enabled {
        return Err(ConfigError::Validation(
            "Proxy and Web UI ports must be different".into()
        ).into());
    }
    
    // Validate server configs
    for (name, server) in &config.servers {
        if server.command.is_empty() {
            return Err(ConfigError::Validation(
                format!("Server '{}' has empty command", name)
            ).into());
        }
        
        // Validate transport config
        match &server.transport {
            super::schema::TransportConfig::HttpSse { url, .. } => {
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Err(ConfigError::Validation(
                        format!("Server '{}' has invalid HTTP URL", name)
                    ).into());
                }
            }
            super::schema::TransportConfig::WebSocket { url, .. } => {
                if !url.starts_with("ws://") && !url.starts_with("wss://") {
                    return Err(ConfigError::Validation(
                        format!("Server '{}' has invalid WebSocket URL", name)
                    ).into());
                }
            }
            _ => {}
        }
    }
    
    // Validate connection pool size
    if config.proxy.connection_pool_size == 0 {
        return Err(ConfigError::Validation(
            "Connection pool size must be greater than 0".into()
        ).into());
    }
    
    Ok(())
}

fn apply_env_substitutions(mut config: Config) -> Result<Config> {
    
    // Process server configurations
    for (_, server) in config.servers.iter_mut() {
        // Substitute in args
        for arg in &mut server.args {
            *arg = substitute_env_vars(arg)?;
        }
        
        // Substitute in env vars
        for (_, value) in &mut server.env {
            *value = substitute_env_vars(value)?;
        }
        
        // Substitute in transport URLs
        match &mut server.transport {
            super::schema::TransportConfig::HttpSse { url, headers, .. } => {
                *url = substitute_env_vars(url)?;
                for (_, header_value) in headers {
                    *header_value = substitute_env_vars(header_value)?;
                }
            }
            super::schema::TransportConfig::WebSocket { url, .. } => {
                *url = substitute_env_vars(url)?;
            }
            _ => {}
        }
    }
    
    // Substitute API key if present
    if let Some(api_key) = &mut config.web_ui.api_key {
        *api_key = substitute_env_vars(api_key)?;
    }
    
    Ok(config)
}

fn substitute_env_vars(input: &str) -> Result<String> {
    let mut result = input.to_string();
    let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
    
    for cap in re.captures_iter(input) {
        let var_name = &cap[1];
        match std::env::var(var_name) {
            Ok(value) => {
                result = result.replace(&cap[0], &value);
            }
            Err(_) => {
                // Check if there's a default value (e.g., ${VAR:-default})
                if let Some((name, default)) = var_name.split_once(":-") {
                    match std::env::var(name) {
                        Ok(value) => result = result.replace(&cap[0], &value),
                        Err(_) => result = result.replace(&cap[0], default),
                    }
                } else {
                    return Err(ConfigError::EnvVar(
                        format!("Environment variable '{}' not found", var_name)
                    ).into());
                }
            }
        }
    }
    
    Ok(result)
}

pub async fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Config> {
    let path = path.as_ref();
    
    let config = if path.extension().and_then(|e| e.to_str()) == Some("toml") {
        Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("MCP_PROXY_").split("_"))
            .extract()
    } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
        Figment::new()
            .merge(Json::file(path))
            .merge(Env::prefixed("MCP_PROXY_").split("_"))
            .extract()
    } else if matches!(path.extension().and_then(|e| e.to_str()), Some("yaml") | Some("yml")) {
        Figment::new()
            .merge(Yaml::file(path))
            .merge(Env::prefixed("MCP_PROXY_").split("_"))
            .extract()
    } else {
        return Err(ConfigError::Parse(
            "Unsupported config file format. Use .toml, .json, .yaml, or .yml".into()
        ).into());
    };
    
    let config = config.map_err(|e| ConfigError::Parse(e.to_string()))?;
    validate(&config)?;
    let config = apply_env_substitutions(config)?;
    
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_env_substitution() {
        std::env::set_var("TEST_VAR", "test_value");
        
        let result = substitute_env_vars("Hello ${TEST_VAR}!").unwrap();
        assert_eq!(result, "Hello test_value!");
        
        let result = substitute_env_vars("${MISSING:-default}").unwrap();
        assert_eq!(result, "default");
        
        std::env::remove_var("TEST_VAR");
    }
}