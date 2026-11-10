//! Error types for agent runtime operations.

use std::path::PathBuf;
use thiserror::Error;

use crate::id::SessionId;

/// Errors emitted during agent provider operations, session management, and protocol streams.
#[derive(Debug, Error)]
pub enum AgentRuntimeError {
    /// Specified agent session not found.
    #[error("Agent session '{0}' was not found")]
    SessionNotFound(SessionId),

    /// Requested agent provider is not registered or unavailable.
    #[error("Agent provider '{0}' is unavailable: {1}")]
    ProviderUnavailable(String, String),

    /// Process launch or execution failed.
    #[error("Failed to execute process '{program}': {source}")]
    ProcessFailed {
        program: String,
        #[source]
        source: std::io::Error,
    },

    /// JSON-RPC protocol error occurred.
    #[error("JSON-RPC error (code {code}): {message}")]
    JsonRpcError {
        code: i64,
        message: String,
        data: Option<serde_json::Value>,
    },

    /// Protocol or stream decoding error.
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// Serialization/deserialization failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error occurred.
    #[error("I/O error at '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Operation was cancelled.
    #[error("Operation was cancelled")]
    Cancelled,

    /// General operational failure.
    #[error("Agent error: {0}")]
    Other(String),
}
