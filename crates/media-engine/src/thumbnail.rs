//! Thumbnail generation for video and image media assets.

use std::fs;
use std::path::{Path, PathBuf};

use nodavideo_project::{AssetId, MediaType, TimeRational};

use crate::binaries::ffmpeg_command;
use crate::error::MediaEngineError;

/// Target maximum bounding box dimensions for generated thumbnails.
pub const THUMBNAIL_MAX_DIMENSION: u32 = 320;

/// Generate or retrieve a cached thumbnail for a media asset.
pub fn generate_thumbnail(
    source_path: &Path,
    asset_id: AssetId,
    media_type: MediaType,
    duration: Option<TimeRational>,
    cache_dir: &Path,
) -> Result<PathBuf, MediaEngineError> {
    let thumbs_dir = cache_dir.join("thumbnails");
    fs::create_dir_all(&thumbs_dir).map_err(|err| MediaEngineError::Io {
        path: thumbs_dir.clone(),
        source: err,
    })?;

    let output_path = thumbs_dir.join(format!("{asset_id}.jpg"));

    // If thumbnail already exists and is non-empty, reuse it
    if output_path.exists() {
        if let Ok(meta) = fs::metadata(&output_path) {
            if meta.len() > 0 {
                return Ok(output_path);
            }
        }
    }

    match media_type {
        MediaType::Image => generate_image_thumbnail(source_path, &output_path),
        MediaType::Video => generate_video_thumbnail(source_path, &output_path, duration),
        MediaType::Audio => {
            // Audio files do not generate image thumbnails directly
            Err(MediaEngineError::UnsupportedFormat(
                "Audio assets do not generate image thumbnails".into(),
            ))
        }
    }
}

/// Generate a downscaled thumbnail from an image file preserving aspect ratio.
fn generate_image_thumbnail(
    source_path: &Path,
    output_path: &Path,
) -> Result<PathBuf, MediaEngineError> {
    let img = image::open(source_path).map_err(|err| MediaEngineError::ImageError {
        path: source_path.to_path_buf(),
        reason: format!("Failed to decode image: {err}"),
    })?;

    let thumb = img.thumbnail(THUMBNAIL_MAX_DIMENSION, THUMBNAIL_MAX_DIMENSION);

    thumb
        .save(output_path)
        .map_err(|err| MediaEngineError::ImageError {
            path: output_path.to_path_buf(),
            reason: format!("Failed to save thumbnail: {err}"),
        })?;

    Ok(output_path.to_path_buf())
}

/// Extract a representative video frame as a thumbnail via FFmpeg.
fn generate_video_thumbnail(
    source_path: &Path,
    output_path: &Path,
    duration: Option<TimeRational>,
) -> Result<PathBuf, MediaEngineError> {
    // Pick a representative timestamp: 1.0s or 10% into clip, else 0.0s
    let seek_seconds = match duration {
        Some(d) => {
            let total_sec = d.to_seconds_f64();
            if total_sec > 2.0 {
                1.0
            } else if total_sec > 0.0 {
                total_sec * 0.25
            } else {
                0.0
            }
        }
        None => 0.0,
    };

    let seek_str = format!("{seek_seconds:.3}");

    let output = ffmpeg_command()?
        .arg("-ss")
        .arg(&seek_str)
        .arg("-i")
        .arg(source_path)
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg(format!(
            "scale={THUMBNAIL_MAX_DIMENSION}:{THUMBNAIL_MAX_DIMENSION}:force_original_aspect_ratio=decrease"
        ))
        .arg("-q:v")
        .arg("3")
        .arg("-y")
        .arg(output_path)
        .output()
        .map_err(|err| MediaEngineError::Io {
            path: source_path.to_path_buf(),
            source: err,
        })?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(MediaEngineError::ThumbnailFailed {
            path: source_path.to_path_buf(),
            reason: if err_msg.is_empty() {
                "ffmpeg exited with non-zero status during thumbnail generation".into()
            } else {
                err_msg
            },
        });
    }

    Ok(output_path.to_path_buf())
}
