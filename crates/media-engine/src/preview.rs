//! Timeline frame extraction and preview rendering.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use nodavideo_project::{FrameRate, MediaAsset, MediaType, ProjectDocument};
use nodavideo_timeline::{Clip, Sequence, TrackId, TrackType};

use crate::error::MediaEngineError;
use crate::probe::is_ffmpeg_available;

/// Resolve the active top-most visible video or graphic clip at a specific timeline frame.
pub fn resolve_active_video_clip(sequence: &Sequence, frame: i64) -> Option<(TrackId, Clip)> {
    for track in &sequence.tracks {
        if track.track_type == TrackType::Video && track.visible {
            if let Some(clip) = track.clip_at_frame(frame) {
                if clip.enabled {
                    return Some((track.id, clip.clone()));
                }
            }
        }
    }
    None
}

/// Extract or retrieve a cached frame for a media asset at a specific source frame.
pub fn extract_preview_frame(
    asset: &MediaAsset,
    source_frame: i64,
    cache_dir: &Path,
) -> Result<PathBuf, MediaEngineError> {
    if !asset.file_path.exists() {
        return Err(MediaEngineError::FrameExtractionFailed {
            path: asset.file_path.clone(),
            reason: "Source media file does not exist on disk".into(),
        });
    }

    match asset.media_type {
        MediaType::Image => {
            // For images, the source file can be displayed directly
            Ok(asset.file_path.clone())
        }
        MediaType::Audio => Err(MediaEngineError::UnsupportedFormat(
            "Audio assets do not contain video frames".into(),
        )),
        MediaType::Video => {
            let previews_dir = cache_dir.join("previews");
            fs::create_dir_all(&previews_dir).map_err(|e| MediaEngineError::Io {
                path: previews_dir.clone(),
                source: e,
            })?;

            let preview_path = previews_dir.join(format!("{}_{source_frame}.jpg", asset.id));
            if preview_path.exists() {
                return Ok(preview_path);
            }

            if !is_ffmpeg_available() {
                return Err(MediaEngineError::FfmpegNotFound);
            }

            let fps = asset.frame_rate.unwrap_or(FrameRate::FPS_24);
            let fps_f64 = fps.to_f64().max(1.0);
            let timestamp_sec = (source_frame.max(0) as f64) / fps_f64;

            let output = Command::new("ffmpeg")
                .arg("-ss")
                .arg(format!("{timestamp_sec:.4}"))
                .arg("-i")
                .arg(&asset.file_path)
                .arg("-frames:v")
                .arg("1")
                .arg("-q:v")
                .arg("2")
                .arg("-y")
                .arg(&preview_path)
                .output()
                .map_err(|e| MediaEngineError::Io {
                    path: asset.file_path.clone(),
                    source: e,
                })?;

            if !output.status.success() || !preview_path.exists() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(MediaEngineError::FrameExtractionFailed {
                    path: asset.file_path.clone(),
                    reason: format!("FFmpeg failed (code {:?}): {stderr}", output.status.code()),
                });
            }

            Ok(preview_path)
        }
    }
}

/// Render the composited frame at the given timeline frame, returning a path to the preview image if a clip is active.
pub fn render_sequence_frame(
    sequence: &Sequence,
    project: &ProjectDocument,
    timeline_frame: i64,
    cache_dir: &Path,
) -> Result<Option<PathBuf>, MediaEngineError> {
    let Some((_track_id, clip)) = resolve_active_video_clip(sequence, timeline_frame) else {
        return Ok(None);
    };

    let Some(asset) = project.get_asset(clip.asset_id) else {
        return Err(MediaEngineError::FrameExtractionFailed {
            path: PathBuf::from(format!("asset:{}", clip.asset_id)),
            reason: format!(
                "Media asset with ID '{}' not found in project",
                clip.asset_id
            ),
        });
    };

    let source_frame = clip.source_in + (timeline_frame - clip.start_frame);
    let frame_path = extract_preview_frame(asset, source_frame, cache_dir)?;
    Ok(Some(frame_path))
}
