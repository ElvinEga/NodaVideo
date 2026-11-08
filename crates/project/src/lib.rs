//! Persistent project document, media asset references, and schema migrations.
//!
//! Owns the canonical, versioned project JSON document according to the NodaVideo
//! architectural contract.

pub mod error;
pub mod id;
pub mod model;
pub mod persistence;
pub mod time;

pub use error::ProjectError;
pub use id::{AssetId, ProjectId};
pub use model::{CURRENT_SCHEMA_VERSION, MediaAsset, MediaAssetStatus, MediaType, ProjectDocument};
pub use time::{FrameRate, TimeRational};
