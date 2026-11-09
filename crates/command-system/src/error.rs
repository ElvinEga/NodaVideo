//! Command system error types.

use nodavideo_timeline::TimelineError;
use thiserror::Error;

/// Errors arising during command execution, undo, or redo.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommandError {
    /// Underlying timeline operation failed.
    #[error(transparent)]
    Timeline(#[from] TimelineError),

    /// Attempted to undo when history undo stack is empty.
    #[error("No command available to undo")]
    NoCommandToUndo,

    /// Attempted to redo when history redo stack is empty.
    #[error("No command available to redo")]
    NoCommandToRedo,

    /// Command validation or execution failed.
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
}
