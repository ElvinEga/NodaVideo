//! Error types for HyperFrames composition templates and rendering.

use thiserror::Error;

/// Errors arising during composition configuration and template rendering.
#[derive(Debug, Error)]
pub enum HyperframesError {
    /// Requested motion graphics template was not found.
    #[error("Motion graphics template '{0}' not found")]
    TemplateNotFound(String),

    /// A required parameter was missing or failed validation.
    #[error("Invalid template parameter '{name}': {reason}")]
    InvalidParameter { name: String, reason: String },

    /// Headless renderer process or filtergraph execution failed.
    #[error("Failed to render composition: {0}")]
    RenderFailed(String),

    /// File I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
