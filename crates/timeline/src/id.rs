//! Strongly typed opaque identifiers for timeline sequences, tracks, and clips.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use crate::error::TimelineError;

/// Unique identifier for an editable sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SequenceId(Uuid);

impl SequenceId {
    /// Generate a new unique SequenceId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct a SequenceId from an existing UUID.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Return the underlying UUID.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for SequenceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SequenceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SequenceId {
    type Err = TimelineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|err| TimelineError::InvalidId(format!("Invalid SequenceId: {err}")))
    }
}

/// Unique identifier for a timeline track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrackId(Uuid);

impl TrackId {
    /// Generate a new unique TrackId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct a TrackId from an existing UUID.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Return the underlying UUID.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for TrackId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TrackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TrackId {
    type Err = TimelineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|err| TimelineError::InvalidId(format!("Invalid TrackId: {err}")))
    }
}

/// Unique identifier for an individual clip placed on a track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClipId(Uuid);

impl ClipId {
    /// Generate a new unique ClipId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct a ClipId from an existing UUID.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Return the underlying UUID.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ClipId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ClipId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ClipId {
    type Err = TimelineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|err| TimelineError::InvalidId(format!("Invalid ClipId: {err}")))
    }
}
