//! Error types for editor session and orchestration operations.

use nodavideo_command_system::CommandError;
use nodavideo_project::ProjectError;
use nodavideo_timeline::TimelineError;
use std::path::PathBuf;
use thiserror::Error;

/// Core errors occurring during editor orchestration, session management, or playback.
#[derive(Debug, Error)]
pub enum EditorError {
    /// Command execution failure.
    #[error("Command execution error: {0}")]
    Command(#[from] CommandError),

    /// Timeline structural or arithmetic error.
    #[error("Timeline error: {0}")]
    Timeline(#[from] TimelineError),

    /// Project document persistence or schema error.
    #[error("Project error: {0}")]
    Project(#[from] ProjectError),

    /// File or path I/O error.
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Asset resolution error.
    #[error("Asset with ID {0} not found in project")]
    AssetNotFound(String),

    /// Track resolution error.
    #[error("Track with ID {0} not found in sequence")]
    TrackNotFound(String),

    /// Clip resolution error.
    #[error("Clip with ID {0} not found in sequence")]
    ClipNotFound(String),

    /// No active staged changeset available.
    #[error("No staged changeset is currently pending")]
    NoStagedChangeset,

    /// Playback or frame bounds error.
    #[error("Invalid playhead frame {frame} (maximum allowed {max_frame})")]
    InvalidPlayheadFrame { frame: u64, max_frame: u64 },

    /// Media export error.
    #[error("Export pipeline failure: {0}")]
    ExportFailed(String),

    /// General operational failure.
    #[error("{0}")]
    Other(String),
}
