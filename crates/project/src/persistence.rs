//! Safe atomic project persistence and schema migration validation.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::error::ProjectError;
use crate::model::{CURRENT_SCHEMA_VERSION, ProjectDocument};

impl ProjectDocument {
    /// Save the project document atomically to the designated path.
    ///
    /// Writes first to a temporary file in the same directory, flushes to disk,
    /// and then renames over the target path to prevent data corruption.
    pub fn save_to_file(&self, path: &Path) -> Result<(), ProjectError> {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|err| ProjectError::Io {
            path: parent.to_path_buf(),
            source: err,
        })?;

        let json_bytes = serde_json::to_vec_pretty(self)?;

        // Write to a temporary file in the same directory
        let temp_file_path = parent.join(format!(
            ".{}.tmp",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project")
        ));
        let mut temp_file = fs::File::create(&temp_file_path).map_err(|err| ProjectError::Io {
            path: temp_file_path.clone(),
            source: err,
        })?;

        temp_file
            .write_all(&json_bytes)
            .map_err(|err| ProjectError::Io {
                path: temp_file_path.clone(),
                source: err,
            })?;

        temp_file.sync_all().map_err(|err| ProjectError::Io {
            path: temp_file_path.clone(),
            source: err,
        })?;

        drop(temp_file);

        // Atomically rename into place
        fs::rename(&temp_file_path, path).map_err(|err| ProjectError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;

        Ok(())
    }

    /// Load a project document from a JSON file and verify asset locations.
    pub fn load_from_file(path: &Path) -> Result<Self, ProjectError> {
        let content = fs::read(path).map_err(|err| ProjectError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;

        let mut doc: Self = serde_json::from_slice(&content)?;

        // Verify schema version
        if doc.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ProjectError::UnsupportedSchemaVersion {
                found: doc.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }

        // Verify physical existence of referenced assets
        doc.verify_assets();

        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::AssetId;
    use crate::model::{MediaAsset, MediaAssetStatus, MediaType};
    use crate::time::{FrameRate, TimeRational};

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let proj_path = dir.path().join("test_project.nodavideo");

        let mut doc = ProjectDocument::new("Test Project");
        let asset_id = AssetId::new();
        let asset_path = dir.path().join("clip.mp4");
        fs::write(&asset_path, b"fake video bytes").unwrap();

        let asset = MediaAsset {
            id: asset_id,
            original_filename: "clip.mp4".into(),
            file_path: asset_path,
            media_type: MediaType::Video,
            file_size: 16,
            import_timestamp: "2026-11-08T00:00:00Z".into(),
            duration: Some(TimeRational::new(10, 1)),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(FrameRate::FPS_24),
            video_codec: Some("h264".into()),
            pixel_format: Some("yuv420p".into()),
            audio_codec: Some("aac".into()),
            audio_sample_rate: Some(48000),
            audio_channels: Some(2),
            audio_bitrate: Some(192000),
            status: MediaAssetStatus::Ready,
            error_message: None,
        };

        doc.add_asset(asset).unwrap();
        doc.save_to_file(&proj_path).unwrap();

        let loaded = ProjectDocument::load_from_file(&proj_path).unwrap();
        assert_eq!(loaded.name, "Test Project");
        assert_eq!(loaded.assets.len(), 1);
        assert_eq!(loaded.assets[0].id, asset_id);
        assert_eq!(loaded.assets[0].status, MediaAssetStatus::Ready);
    }

    #[test]
    fn test_missing_asset_detection() {
        let dir = tempfile::tempdir().unwrap();
        let proj_path = dir.path().join("missing.nodavideo");

        let mut doc = ProjectDocument::new("Missing Asset Test");
        let non_existent_path = dir.path().join("deleted.mp4");

        let asset = MediaAsset {
            id: AssetId::new(),
            original_filename: "deleted.mp4".into(),
            file_path: non_existent_path,
            media_type: MediaType::Video,
            file_size: 0,
            import_timestamp: "2026-11-08T00:00:00Z".into(),
            duration: None,
            width: None,
            height: None,
            frame_rate: None,
            video_codec: None,
            pixel_format: None,
            audio_codec: None,
            audio_sample_rate: None,
            audio_channels: None,
            audio_bitrate: None,
            status: MediaAssetStatus::Ready,
            error_message: None,
        };

        doc.add_asset(asset).unwrap();
        doc.save_to_file(&proj_path).unwrap();

        let loaded = ProjectDocument::load_from_file(&proj_path).unwrap();
        assert_eq!(loaded.assets[0].status, MediaAssetStatus::Missing);
    }
}
