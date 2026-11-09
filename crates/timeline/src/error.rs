//! Timeline domain error types.

use thiserror::Error;

use crate::id::{ClipId, TrackId};

/// Errors encountered in pure timeline operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TimelineError {
    /// Referenced track was not found in the sequence.
    #[error("Track with ID '{0}' not found")]
    TrackNotFound(TrackId),

    /// Referenced clip was not found in any track.
    #[error("Clip with ID '{0}' not found")]
    ClipNotFound(ClipId),

    /// Track with this ID already exists in sequence.
    #[error("Track with ID '{0}' already exists")]
    DuplicateTrackId(TrackId),

    /// Clip with this ID already exists in track.
    #[error("Clip with ID '{0}' already exists")]
    DuplicateClipId(ClipId),

    /// Track is locked and cannot be mutated.
    #[error("Track '{0}' is locked")]
    TrackLocked(TrackId),

    /// Split position is invalid (must be strictly inside clip: start < split < end).
    #[error("Cannot split clip at frame {split_frame}: bounds are [{start}, {end})")]
    InvalidSplitPosition {
        split_frame: i64,
        start: i64,
        end: i64,
    },

    /// Trim boundaries are invalid (source_out must be strictly greater than source_in).
    #[error("Invalid trim range: source_in ({source_in}) must be < source_out ({source_out})")]
    InvalidTrim { source_in: i64, source_out: i64 },

    /// Timeline start frame cannot be negative.
    #[error("Invalid start frame {0}: start frame cannot be negative")]
    InvalidStartFrame(i64),

    /// Clip duration must be at least 1 frame.
    #[error("Invalid clip duration {0}: must be at least 1 frame")]
    InvalidDuration(i64),

    /// Incompatible asset media type for target track.
    #[error("Incompatible track type: track expects {expected}, but asset is {found}")]
    TrackTypeMismatch { expected: String, found: String },

    /// Malformed ID string representation.
    #[error("Invalid identifier: {0}")]
    InvalidId(String),

    /// Serialization or deserialization failure.
    #[error("Timeline serialization error: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for TimelineError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}
