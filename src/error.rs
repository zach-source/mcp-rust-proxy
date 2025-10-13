use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Server error: {0}")]
    Server(#[from] ServerError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Timeout error")]
    Timeout,

    #[error("Server not found: {0}")]
    ServerNotFound(String),

    #[error("Server not ready: {0}")]
    ServerNotReady(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Invalid response")]
    InvalidResponse,

    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Environment variable error: {0}")]
    EnvVar(String),
}

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Transport closed")]
    Closed,

    #[error("Invalid message format")]
    InvalidFormat,
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Failed to start server: {0}")]
    StartFailed(String),

    #[error("Server crashed: {0}")]
    Crashed(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Health check failed")]
    HealthCheckFailed,
}

#[derive(Error, Debug)]
pub enum PoolError {
    #[error("Server not found: {0}")]
    ServerNotFound(String),

    #[error("Pool exhausted")]
    Exhausted,

    #[error("Connection error: {0}")]
    Connection(String),
}

#[derive(Error, Debug)]
pub enum HealthError {
    #[error("Unhealthy server")]
    Unhealthy,

    #[error("Health check timeout")]
    Timeout,

    #[error("Invalid response")]
    InvalidResponse,
}

pub type Result<T> = std::result::Result<T, ProxyError>;

impl warp::reject::Reject for ProxyError {}
