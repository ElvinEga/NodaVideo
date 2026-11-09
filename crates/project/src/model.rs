//! Project document and media asset domain models.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::ProjectError;
use crate::id::{AssetId, ProjectId};
use crate::time::{FrameRate, TimeRational};

/// Current supported schema version for NodaVideo project documents.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Primary classification of imported media assets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Video,
    Audio,
    Image,
}

impl MediaType {
    /// Infer media type from a file path extension.
    pub fn from_extension(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "flv" | "wmv" | "ts" => {
                Some(Self::Video)
            }
            "wav" | "mp3" | "aac" | "flac" | "ogg" | "m4a" | "aiff" | "wma" => Some(Self::Audio),
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tiff" | "svg" => Some(Self::Image),
            _ => None,
        }
    }
}

/// Status of an imported asset's source reference.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", content = "detail", rename_all = "snake_case")]
pub enum MediaAssetStatus {
    /// Asset file exists and metadata is verified.
    #[default]
    Ready,
    /// Source file path could not be found on the filesystem.
    Missing,
    /// Inspection or processing encountered an error.
    Failed(String),
}

/// Metadata and persistent reference for an imported media file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaAsset {
    /// Unique asset ID.
    pub id: AssetId,
    /// Original file name when imported.
    pub original_filename: String,
    /// Absolute or canonical source path to the media file on disk.
    pub file_path: PathBuf,
    /// Media classification (Video, Audio, Image).
    pub media_type: MediaType,
    /// File size in bytes.
    pub file_size: u64,
    /// Import timestamp (ISO 8601).
    pub import_timestamp: String,
    /// Duration of video or audio assets.
    pub duration: Option<TimeRational>,
    /// Frame width for video or image assets.
    pub width: Option<u32>,
    /// Frame height for video or image assets.
    pub height: Option<u32>,
    /// Rational frame rate for video assets.
    pub frame_rate: Option<FrameRate>,
    /// Video codec identifier (e.g. h264, hevc, vp9, prores).
    pub video_codec: Option<String>,
    /// Video pixel format (e.g. yuv420p, yuv422p10le).
    pub pixel_format: Option<String>,
    /// Audio codec identifier (e.g. aac, pcm_s16le, mp3, flac).
    pub audio_codec: Option<String>,
    /// Audio sample rate in Hz (e.g. 48000).
    pub audio_sample_rate: Option<u32>,
    /// Audio channel count (e.g. 1 for mono, 2 for stereo).
    pub audio_channels: Option<u16>,
    /// Audio nominal bitrate in bits per second.
    pub audio_bitrate: Option<u64>,
    /// Current asset status (Ready, Missing, Failed).
    pub status: MediaAssetStatus,
    /// Optional failure error message if inspection failed.
    pub error_message: Option<String>,
}

impl MediaAsset {
    /// Check whether the source file currently exists on disk.
    pub fn source_exists(&self) -> bool {
        self.file_path.exists()
    }

    /// Return a formatted string of the asset's dimensions (e.g. "1920×1080").
    pub fn resolution_string(&self) -> Option<String> {
        match (self.width, self.height) {
            (Some(w), Some(h)) => Some(format!("{w}×{h}")),
            _ => None,
        }
    }
}

/// Canonical versioned JSON document representing a NodaVideo project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDocument {
    /// Schema version for migrations. Must equal `CURRENT_SCHEMA_VERSION`.
    pub schema_version: u32,
    /// Unique project identifier.
    pub id: ProjectId,
    /// User-visible project name.
    pub name: String,
    /// Creation ISO timestamp.
    pub created_at: String,
    /// Last modified ISO timestamp.
    pub modified_at: String,
    /// Imported media assets.
    pub assets: Vec<MediaAsset>,
    /// Optional serialized timeline sequence data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<serde_json::Value>,
}

impl ProjectDocument {
    /// Create a new project document with the current schema version.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: ProjectId::new(),
            name: name.into(),
            created_at: now.clone(),
            modified_at: now,
            assets: Vec::new(),
            timeline: None,
        }
    }

    /// Add an asset to the project, preventing duplicates with matching paths.
    pub fn add_asset(&mut self, asset: MediaAsset) -> Result<(), ProjectError> {
        let asset_path =
            std::fs::canonicalize(&asset.file_path).unwrap_or_else(|_| asset.file_path.clone());

        for existing in &self.assets {
            let existing_path = std::fs::canonicalize(&existing.file_path)
                .unwrap_or_else(|_| existing.file_path.clone());
            if existing_path == asset_path || existing.id == asset.id {
                return Err(ProjectError::DuplicateAsset(asset.original_filename));
            }
        }

        self.assets.push(asset);
        self.modified_at = Utc::now().to_rfc3339();
        Ok(())
    }

    /// Remove an asset by ID from the project.
    pub fn remove_asset(&mut self, id: AssetId) -> Option<MediaAsset> {
        if let Some(pos) = self.assets.iter().position(|a| a.id == id) {
            let removed = self.assets.remove(pos);
            self.modified_at = Utc::now().to_rfc3339();
            Some(removed)
        } else {
            None
        }
    }

    /// Retrieve a reference to an asset by ID.
    pub fn get_asset(&self, id: AssetId) -> Option<&MediaAsset> {
        self.assets.iter().find(|a| a.id == id)
    }

    /// Retrieve a mutable reference to an asset by ID.
    pub fn get_asset_mut(&mut self, id: AssetId) -> Option<&mut MediaAsset> {
        self.assets.iter_mut().find(|a| a.id == id)
    }

    /// Verify physical source file existence for all assets and update status.
    pub fn verify_assets(&mut self) {
        for asset in &mut self.assets {
            if !asset.file_path.exists() {
                asset.status = MediaAssetStatus::Missing;
            } else if asset.status == MediaAssetStatus::Missing {
                asset.status = MediaAssetStatus::Ready;
            }
        }
    }
}

impl Default for ProjectDocument {
    fn default() -> Self {
        Self::new("Untitled Project")
    }
}
