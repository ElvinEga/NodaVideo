//! Error types for MCP tool execution.

use thiserror::Error;

/// Errors arising during MCP tool schema validation and execution.
#[derive(Debug, Error)]
pub enum McpError {
    /// Requested tool is not recognized.
    #[error("Unknown tool '{0}'")]
    ToolNotFound(String),

    /// Tool arguments failed schema validation or deserialization.
    #[error("Invalid arguments for tool '{tool}': {details}")]
    InvalidArguments { tool: String, details: String },

    /// Command execution through the command system failed.
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Tool requires user approval before it can execute.
    #[error("User approval required for '{tool}': {reason}")]
    ApprovalRequired { tool: String, reason: String },

    /// Tool is recognized but cannot run in this process.
    #[error("Tool '{tool}' is unavailable: {reason}")]
    Unsupported { tool: String, reason: String },

    /// A motion graphics composition operation failed.
    #[error("Composition operation failed: {0}")]
    Composition(#[from] nodavideo_hyperframes_bridge::HyperframesError),
}
