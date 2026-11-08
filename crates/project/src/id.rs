//! Strongly typed, opaque identifiers for projects and media assets.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

use crate::error::ProjectError;

/// Unique identifier for a NodaVideo project document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(Uuid);

impl ProjectId {
    /// Generate a new unique ProjectId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct a ProjectId from an existing UUID.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Return the underlying UUID.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ProjectId {
    type Err = ProjectError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|err| ProjectError::InvalidDocument(format!("Invalid ProjectId: {err}")))
    }
}

/// Unique identifier for an imported media asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(Uuid);

impl AssetId {
    /// Generate a new unique AssetId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct an AssetId from an existing UUID.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Return the underlying UUID.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for AssetId {
    type Err = ProjectError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|err| ProjectError::InvalidDocument(format!("Invalid AssetId: {err}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_generation_and_string_roundtrip() {
        let id = AssetId::new();
        let s = id.to_string();
        let parsed: AssetId = s.parse().unwrap();
        assert_eq!(id, parsed);

        let proj_id = ProjectId::new();
        let p_str = proj_id.to_string();
        let p_parsed: ProjectId = p_str.parse().unwrap();
        assert_eq!(proj_id, p_parsed);
    }
}
