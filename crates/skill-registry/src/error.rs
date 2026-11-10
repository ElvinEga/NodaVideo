//! Error types for skill package discovery and loading.

use std::path::PathBuf;
use thiserror::Error;

/// Errors arising during skill package discovery, parsing, or loading.
#[derive(Debug, Error)]
pub enum SkillError {
    /// Skill with specified ID was not found.
    #[error("Skill '{0}' was not found in registry")]
    NotFound(String),

    /// SKILL.md format or frontmatter was invalid.
    #[error("Invalid skill document at '{path}': {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    /// File I/O failure.
    #[error("I/O error at '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
