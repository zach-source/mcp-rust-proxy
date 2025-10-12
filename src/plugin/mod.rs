//! JavaScript Plugin System for MCP Proxy
//!
//! This module implements a plugin system that allows end users to run custom middleware
//! for different MCP servers. Plugins run in isolated Node.js processes and communicate
//! via stdin/stdout using JSON or MessagePack.
//!
//! # Architecture
//!
//! - **Process Isolation**: Each plugin runs in a separate Node.js process
//! - **IPC via stdio**: Communication using stdin/stdout with JSON serialization
//! - **Fail-fast**: Plugin failures cause the entire request to fail with detailed errors
//! - **Process Pooling**: Reuse warm Node.js processes to reduce spawn overhead
//! - **Concurrency Control**: Global semaphore to prevent resource exhaustion
//!
//! # Modules
//!
//! - `manager`: Plugin lifecycle management and concurrency control
//! - `process`: Node.js process spawning, IPC, and process pooling
//! - `schema`: Plugin I/O schema definitions and serialization
//! - `chain`: Plugin chaining logic and sequential execution
//! - `config`: Plugin configuration parsing and validation

pub mod chain;
pub mod config;
pub mod manager;
pub mod process;
pub mod schema;

pub use chain::PluginChain;
pub use config::PluginConfig;
pub use manager::PluginManager;
pub use process::{PluginProcess, ProcessPool};
pub use schema::{PluginInput, PluginOutput, PluginPhase};
