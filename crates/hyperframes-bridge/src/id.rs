//! Strongly-typed unique identifiers for HyperFrames motion graphics compositions.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a motion graphics composition instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompositionId(pub Uuid);

impl CompositionId {
    /// Generate a new random CompositionId (v4 UUID).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CompositionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CompositionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for CompositionId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}
