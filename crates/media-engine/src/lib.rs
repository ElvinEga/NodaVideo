//! Media engine for NodaVideo.
//!
//! Provides FFmpeg stream probing, metadata inspection, thumbnail generation,
//! audio waveform extraction, and SQLite-backed media caching.

pub mod cache;
pub mod error;
pub mod export;
pub mod preview;
pub mod probe;
pub mod thumbnail;
pub mod waveform;

use std::path::{Path, PathBuf};

use nodavideo_project::{AssetId, MediaAsset, MediaType, ProjectDocument};
use nodavideo_timeline::Sequence;

pub use cache::{CachedMediaRecord, MediaCacheDb};
pub use error::MediaEngineError;
pub use export::{ExportReport, ExportSettings, export_sequence};
pub use preview::{extract_preview_frame, render_sequence_frame, resolve_active_video_clip};
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

    /// Render the preview frame for a timeline sequence at a given playhead frame.
    pub fn render_preview(
        &self,
        sequence: &Sequence,
        project: &ProjectDocument,
        frame: i64,
    ) -> Result<Option<PathBuf>, MediaEngineError> {
        render_sequence_frame(sequence, project, frame, &self.cache_dir)
    }

    /// Export a timeline sequence to an MP4 video file.
    pub fn export(
        &self,
        sequence: &Sequence,
        project: &ProjectDocument,
        settings: &ExportSettings,
    ) -> Result<ExportReport, MediaEngineError> {
        export_sequence(sequence, project, settings)
    }

    /// Retrieve cached waveform data for an asset ID if it exists on disk.
    #[must_use]
    pub fn get_waveform(&self, asset_id: AssetId) -> Option<WaveformData> {
        let path = self
            .cache_dir
            .join("waveforms")
            .join(format!("{asset_id}.json"));
        if path.exists() {
            WaveformData::load_from_file(&path).ok()
        } else {
            None
        }
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

    #[test]
    fn test_preview_and_export_pipeline() {
        if !is_ffmpeg_available() || !is_ffprobe_available() {
            eprintln!("Skipping export pipeline test: ffmpeg/ffprobe not available");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("cache");
        let processor = MediaProcessor::new(&cache_dir).unwrap();

        // 1. Generate a small synthetic 2-second video with FFmpeg for testing
        let video_path = dir.path().join("source.mp4");
        let status = std::process::Command::new("ffmpeg")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("testsrc=size=320x240:rate=24:duration=2")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("sine=frequency=440:duration=2")
            .arg("-c:v")
            .arg("libx264")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-c:a")
            .arg("aac")
            .arg("-t")
            .arg("2")
            .arg("-y")
            .arg(&video_path)
            .status()
            .unwrap();
        assert!(status.success());

        // 2. Process file into asset & add to project
        let asset = processor.process_file(&video_path).unwrap();
        let mut project = ProjectDocument::new("Export Test");
        let asset_id = asset.id;
        project.add_asset(asset).unwrap();

        // 3. Create sequence and place clip on V1
        let mut sequence = Sequence::default_1080p24();
        let v1_id = sequence.tracks[1].id;
        let clip_id = nodavideo_timeline::ClipId::new();

        // 24 frames (1 second), trimmed from source in=0 to source out=24
        let clip = nodavideo_timeline::Clip::new(clip_id, asset_id, v1_id, "Video Clip", 0, 0, 24)
            .unwrap();
        sequence.add_clip(v1_id, clip).unwrap();

        // 4. Test preview extraction at frame 12
        let preview_opt = processor.render_preview(&sequence, &project, 12).unwrap();
        assert!(preview_opt.is_some());
        let preview_path = preview_opt.unwrap();
        assert!(preview_path.exists());

        // 5. Test export to MP4
        let export_path = dir.path().join("exported_timeline.mp4");
        let export_settings = ExportSettings {
            output_path: export_path.clone(),
            width: 320,
            height: 240,
            fps: nodavideo_project::FrameRate::FPS_24,
            video_bitrate_kbps: 1000,
            audio_bitrate_kbps: 128,
            video_codec: "libx264".into(),
            audio_codec: "aac".into(),
        };

        let report = processor
            .export(&sequence, &project, &export_settings)
            .unwrap();
        assert_eq!(report.output_path, export_path);
        assert!(report.file_size_bytes > 0);
        assert!(export_path.exists());

        // 6. Verify exported MP4 via ffprobe
        let probed = probe_media_file(&export_path).unwrap();
        assert_eq!(probed.media_type, MediaType::Video);
        assert_eq!(probed.width, Some(320));
        assert_eq!(probed.height, Some(240));
        assert!(probed.duration.is_some());
    }
}
