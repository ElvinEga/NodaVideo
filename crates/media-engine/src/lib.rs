//! Media engine for NodaVideo.
//!
//! Provides FFmpeg stream probing, metadata inspection, thumbnail generation,
//! audio waveform extraction, and SQLite-backed media caching.

pub mod cache;
pub mod error;
pub mod probe;
pub mod thumbnail;
pub mod waveform;

use std::path::{Path, PathBuf};

use nodavideo_project::{MediaAsset, MediaType};

pub use cache::{CachedMediaRecord, MediaCacheDb};
pub use error::MediaEngineError;
pub use probe::{is_ffmpeg_available, is_ffprobe_available, probe_media_file};
pub use thumbnail::generate_thumbnail;
pub use waveform::{WaveformData, generate_waveform};

/// High-level media processor coordinating probing, thumbnailing, waveforms, and caching.
#[derive(Debug, Clone)]
pub struct MediaProcessor {
    pub cache_db: MediaCacheDb,
    pub cache_dir: PathBuf,
}

impl MediaProcessor {
    /// Initialize a MediaProcessor with the designated cache directory.
    pub fn new(cache_dir: &Path) -> Result<Self, MediaEngineError> {
        let db_path = cache_dir.join("media_cache.db");
        let cache_db = MediaCacheDb::open(&db_path)?;

        Ok(Self {
            cache_db,
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Process a media file: probe metadata, generate thumbnail and waveform, and cache results.
    pub fn process_file(&self, path: &Path) -> Result<MediaAsset, MediaEngineError> {
        // 1. Check cache first
        if let Ok(Some(cached)) = self.cache_db.get_by_path(path) {
            return Ok(cached.metadata);
        }

        // 2. Probe file metadata
        let asset = probe_media_file(path)?;

        // 3. Generate thumbnail for Video or Image
        let thumbnail_path = match asset.media_type {
            MediaType::Video | MediaType::Image => generate_thumbnail(
                path,
                asset.id,
                asset.media_type,
                asset.duration,
                &self.cache_dir,
            )
            .ok(),
            MediaType::Audio => None,
        };

        // 4. Generate waveform for Audio or Video with audio
        let waveform_path = if asset.media_type == MediaType::Audio
            || (asset.media_type == MediaType::Video && asset.audio_codec.is_some())
        {
            generate_waveform(path, asset.id, asset.duration, &self.cache_dir)
                .map(|(_, p)| p)
                .ok()
        } else {
            None
        };

        // 5. Store in SQLite cache
        if let Ok(meta) = std::fs::metadata(path) {
            let mtime = meta
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let record = CachedMediaRecord {
                asset_id: asset.id,
                file_path: path.to_path_buf(),
                file_size: asset.file_size,
                file_mtime: mtime,
                metadata: asset.clone(),
                thumbnail_path,
                waveform_path,
                cached_at: chrono::Utc::now().timestamp(),
            };

            let _ = self.cache_db.put(&record);
        }

        Ok(asset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_project::MediaType;

    /// Generate a simple valid 64x64 PNG image file for testing.
    fn create_test_image(path: &Path) {
        let img = image::RgbImage::from_pixel(64, 64, image::Rgb([255, 128, 0]));
        img.save(path).unwrap();
    }

    /// Generate a 1-second 8000Hz 16-bit mono PCM WAV file for testing.
    fn create_test_wav(path: &Path) {
        let sample_rate: u32 = 8000;
        let num_samples: u32 = 8000;
        let byte_rate = sample_rate * 2;
        let data_size = num_samples * 2;
        let file_size = 36 + data_size;

        let mut bytes = Vec::new();
        // RIFF header
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&file_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        // fmt chunk
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // 1 channel
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        // data chunk
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());

        // Simple sine wave
        for i in 0..num_samples {
            let t = (i as f32) / (sample_rate as f32);
            let val = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.75;
            let sample = (val * 32767.0) as i16;
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn test_process_image_file() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("cache");
        let processor = MediaProcessor::new(&cache_dir).unwrap();

        let img_path = dir.path().join("hero.png");
        create_test_image(&img_path);

        let asset = processor.process_file(&img_path).unwrap();
        assert_eq!(asset.media_type, MediaType::Image);
        assert_eq!(asset.width, Some(64));
        assert_eq!(asset.height, Some(64));

        // Verify thumbnail was created
        let thumb_path = cache_dir
            .join("thumbnails")
            .join(format!("{}.jpg", asset.id));
        assert!(thumb_path.exists());

        // Verify cached in SQLite
        let cached = processor.cache_db.get_by_path(&img_path).unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().asset_id, asset.id);
    }

    #[test]
    fn test_process_audio_wav_file() {
        if !is_ffprobe_available() || !is_ffmpeg_available() {
            eprintln!("Skipping audio test because ffmpeg/ffprobe is not available");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("cache");
        let processor = MediaProcessor::new(&cache_dir).unwrap();

        let wav_path = dir.path().join("voice.wav");
        create_test_wav(&wav_path);

        let asset = processor.process_file(&wav_path).unwrap();
        assert_eq!(asset.media_type, MediaType::Audio);
        assert_eq!(asset.audio_channels, Some(1));
        assert_eq!(asset.audio_sample_rate, Some(8000));
        assert!(asset.duration.is_some());

        // Verify waveform was generated
        let wave_path = cache_dir
            .join("waveforms")
            .join(format!("{}.wave", asset.id));
        assert!(wave_path.exists());

        let wave_data = WaveformData::load_from_file(&wave_path).unwrap();
        assert!(!wave_data.peaks.is_empty());
    }
}
