/// MCP protocol version-specific types
///
/// This module contains type definitions for different versions of the MCP protocol.
/// Each version has its own module with version-specific struct definitions.
pub mod common;
pub mod v20241105;
pub mod v20250326;
pub mod v20250618;

// Re-export common types
pub use common::*;
