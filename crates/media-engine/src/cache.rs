//! SQLite-backed media metadata, thumbnail, and waveform index cache.

use chrono::Utc;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use nodavideo_project::{AssetId, MediaAsset};

use crate::error::MediaEngineError;

/// Cached media record stored in SQLite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedMediaRecord {
    pub asset_id: AssetId,
    pub file_path: PathBuf,
    pub file_size: u64,
    pub file_mtime: i64,
    pub metadata: MediaAsset,
    pub thumbnail_path: Option<PathBuf>,
    pub waveform_path: Option<PathBuf>,
    pub cached_at: i64,
}

/// Thread-safe SQLite media cache index.
#[derive(Debug, Clone)]
pub struct MediaCacheDb {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl MediaCacheDb {
    /// Open or create the SQLite media cache database.
    pub fn open(db_path: &Path) -> Result<Self, MediaEngineError> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|err| MediaEngineError::Io {
                path: parent.to_path_buf(),
                source: err,
            })?;
        }

        let conn = Connection::open(db_path)?;

        // Initialize schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS media_cache (
                asset_id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                file_size INTEGER NOT NULL,
                file_mtime INTEGER NOT NULL,
                metadata_json TEXT NOT NULL,
                thumbnail_path TEXT,
                waveform_path TEXT,
                cached_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_media_cache_path ON media_cache(file_path);",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: db_path.to_path_buf(),
        })
    }

    /// Return the path to the database file.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Retrieve a cached record if the source file exists and has not changed size or mtime.
    pub fn get_by_path(
        &self,
        file_path: &Path,
    ) -> Result<Option<CachedMediaRecord>, MediaEngineError> {
        let meta = match fs::metadata(file_path) {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        let current_size = meta.len();
        let current_mtime = meta
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let path_str = file_path.to_string_lossy().to_string();
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT asset_id, file_path, file_size, file_mtime, metadata_json, thumbnail_path, waveform_path, cached_at
             FROM media_cache WHERE file_path = ?1",
        )?;

        let mut rows = stmt.query(params![path_str])?;

        if let Some(row) = rows.next()? {
            let file_size: u64 = row.get(2)?;
            let file_mtime: i64 = row.get(3)?;

            // If file has been modified or resized, invalidate cache
            if file_size != current_size || file_mtime != current_mtime {
                drop(rows);
                drop(stmt);
                conn.execute(
                    "DELETE FROM media_cache WHERE file_path = ?1",
                    params![path_str],
                )?;
                return Ok(None);
            }

            let asset_id_str: String = row.get(0)?;
            let asset_id: AssetId =
                asset_id_str
                    .parse()
                    .map_err(|err| MediaEngineError::ProbeFailed {
                        path: file_path.to_path_buf(),
                        reason: format!("Corrupted cached asset ID: {err}"),
                    })?;

            let metadata_json: String = row.get(4)?;
            let metadata: MediaAsset = serde_json::from_str(&metadata_json)?;

            let thumbnail_path: Option<String> = row.get(5)?;
            let waveform_path: Option<String> = row.get(6)?;
            let cached_at: i64 = row.get(7)?;

            Ok(Some(CachedMediaRecord {
                asset_id,
                file_path: file_path.to_path_buf(),
                file_size,
                file_mtime,
                metadata,
                thumbnail_path: thumbnail_path.map(PathBuf::from),
                waveform_path: waveform_path.map(PathBuf::from),
                cached_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Insert or update a cached media record.
    pub fn put(&self, record: &CachedMediaRecord) -> Result<(), MediaEngineError> {
        let conn = self.conn.lock().unwrap();
        let metadata_json = serde_json::to_string(&record.metadata)?;
        let path_str = record.file_path.to_string_lossy().to_string();
        let asset_id_str = record.asset_id.to_string();
        let thumb_str = record
            .thumbnail_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
        let wave_str = record
            .waveform_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO media_cache (
                asset_id, file_path, file_size, file_mtime, metadata_json, thumbnail_path, waveform_path, cached_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(file_path) DO UPDATE SET
                asset_id = excluded.asset_id,
                file_size = excluded.file_size,
                file_mtime = excluded.file_mtime,
                metadata_json = excluded.metadata_json,
                thumbnail_path = excluded.thumbnail_path,
                waveform_path = excluded.waveform_path,
                cached_at = excluded.cached_at;",
            params![
                asset_id_str,
                path_str,
                record.file_size,
                record.file_mtime,
                metadata_json,
                thumb_str,
                wave_str,
                now
            ],
        )?;

        Ok(())
    }

    /// Invalidate cache for a specific file path.
    pub fn invalidate(&self, file_path: &Path) -> Result<(), MediaEngineError> {
        let path_str = file_path.to_string_lossy().to_string();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM media_cache WHERE file_path = ?1",
            params![path_str],
        )?;
        Ok(())
    }

    /// Clear all records in the cache database.
    pub fn clear(&self) -> Result<(), MediaEngineError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM media_cache", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_project::{MediaAssetStatus, MediaType};

    #[test]
    fn test_sqlite_cache_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("cache.db");
        let cache = MediaCacheDb::open(&db_path).unwrap();

        let file_path = dir.path().join("video.mp4");
        fs::write(&file_path, b"test payload 123").unwrap();
        let meta = fs::metadata(&file_path).unwrap();
        let mtime = meta
            .modified()
            .unwrap()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let asset_id = AssetId::new();
        let asset = MediaAsset {
            id: asset_id,
            original_filename: "video.mp4".into(),
            file_path: file_path.clone(),
            media_type: MediaType::Video,
            file_size: meta.len(),
            import_timestamp: "2026-11-08T00:00:00Z".into(),
            duration: None,
            width: Some(1920),
            height: Some(1080),
            frame_rate: None,
            video_codec: Some("h264".into()),
            pixel_format: None,
            audio_codec: None,
            audio_sample_rate: None,
            audio_channels: None,
            audio_bitrate: None,
            status: MediaAssetStatus::Ready,
            error_message: None,
        };

        let record = CachedMediaRecord {
            asset_id,
            file_path: file_path.clone(),
            file_size: meta.len(),
            file_mtime: mtime,
            metadata: asset,
            thumbnail_path: Some(dir.path().join("thumb.jpg")),
            waveform_path: None,
            cached_at: 1000,
        };

        cache.put(&record).unwrap();

        let retrieved = cache.get_by_path(&file_path).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().asset_id, asset_id);

        // Modifying the file should invalidate the cache automatically
        fs::write(&file_path, b"changed payload larger").unwrap();
        let invalidated = cache.get_by_path(&file_path).unwrap();
        assert!(invalidated.is_none());
    }
}
