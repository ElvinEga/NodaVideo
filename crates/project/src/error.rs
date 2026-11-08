//! Error types for project serialization, migration, and asset validation.

use std::path::PathBuf;
use thiserror::Error;

/// Domain errors encountered during project lifecycle and media reference operations.
#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("I/O error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Serialization / JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Unsupported project schema version: {found} (supported: {supported})")]
    UnsupportedSchemaVersion { found: u32, supported: u32 },

    #[error("Asset already exists in project: {0}")]
    DuplicateAsset(String),

    #[error("Asset not found in project: {0}")]
    AssetNotFound(String),

    #[error("Invalid project file: {0}")]
    InvalidDocument(String),

    #[error("Invalid timecode or rational: {0}")]
    InvalidTime(String),
}
