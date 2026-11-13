//! Timeline frame extraction and preview rendering.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nodavideo_project::{FrameRate, MediaAsset, MediaType, ProjectDocument};
use nodavideo_timeline::{Clip, Sequence, TrackId, TrackType};

use crate::binaries::ffmpeg_command;
use crate::error::MediaEngineError;
use crate::time_math::{format_seconds, frames_to_seconds, timeline_to_source_frame};

/// Monotonic suffix for temporary JPEGs so concurrent/cancelled extractions never
/// share a partially written output file.
static TEMP_FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Resolve the active top-most visible video or graphic clip at a specific timeline frame.
///
/// This is the pure timeline query. It can return a clip that has no backing media
/// asset (text overlays, generated titles, HyperFrames compositions), so renderers
/// should prefer [`resolve_renderable_video_clip`].
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

/// Whether a clip can produce pixels from an actual media file.
///
/// Text overlays and compositions are created with a fresh `AssetId` that is not
/// registered in the project. Without this check they become the top-most clip and
/// make the preview fail with "asset not found", blanking the video underneath.
fn clip_is_renderable(clip: &Clip, project: &ProjectDocument) -> bool {
    project
        .get_asset(clip.asset_id)
        .is_some_and(|asset| asset.media_type != MediaType::Audio && asset.file_path.exists())
}

/// Resolve the top-most clip at `frame` that can actually be decoded into pixels.
///
/// Overlays and clips with missing assets are skipped rather than blanking the
/// preview; the video/image clip underneath is used instead. Compositing the
/// overlays themselves is a Phase B renderer concern.
pub fn resolve_renderable_video_clip(
    sequence: &Sequence,
    project: &ProjectDocument,
    frame: i64,
) -> Option<(TrackId, Clip)> {
    for track in &sequence.tracks {
        if track.track_type != TrackType::Video || !track.visible {
            continue;
        }
        // Later clips in a track are drawn on top of earlier ones.
        if let Some(clip) = track.clips.iter().rev().find(|clip| {
            clip.enabled && clip.contains_timeline_frame(frame) && clip_is_renderable(clip, project)
        }) {
            return Some((track.id, clip.clone()));
        }
    }
    None
}

/// Stable, filesystem-safe tag for a frame rate, used in the preview cache key.
fn fps_cache_tag(fps: FrameRate) -> String {
    format!("{}-{}", fps.numerator, fps.denominator)
}

/// Extract or retrieve a cached frame for a media asset at a specific source frame.
///
/// The cache key includes the asset frame rate: the same source frame maps to a
/// different timestamp when the asset's frame rate metadata changes.
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

            let fps = asset.frame_rate.unwrap_or(FrameRate::FPS_24);
            let preview_path = previews_dir.join(format!(
                "{}_{}_{}.jpg",
                asset.id,
                source_frame,
                fps_cache_tag(fps)
            ));

            // A cancelled FFmpeg process can leave a zero-byte file behind. Reusing
            // it would pin the preview to a broken frame forever.
            if preview_path
                .metadata()
                .is_ok_and(|meta| meta.is_file() && meta.len() > 0)
            {
                return Ok(preview_path);
            }
            if preview_path.exists() {
                let _ = fs::remove_file(&preview_path);
            }

            let timestamp_sec = frames_to_seconds(source_frame, fps);

            // Write to a unique temporary path, then rename. Readers only ever see
            // a complete JPEG, never a half-written one.
            let temp_path = previews_dir.join(format!(
                "{}_{}_{}_{}_{}.tmp.jpg",
                asset.id,
                source_frame,
                fps_cache_tag(fps),
                std::process::id(),
                TEMP_FRAME_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));

            let output = ffmpeg_command()?
                .arg("-ss")
                .arg(format_seconds(timestamp_sec))
                .arg("-i")
                .arg(&asset.file_path)
                .arg("-frames:v")
                .arg("1")
                .arg("-q:v")
                .arg("2")
                .arg("-y")
                .arg(&temp_path)
                .output()
                .map_err(|e| MediaEngineError::Io {
                    path: asset.file_path.clone(),
                    source: e,
                })?;

            if !output.status.success() || !temp_path.exists() {
                let _ = fs::remove_file(&temp_path);
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(MediaEngineError::FrameExtractionFailed {
                    path: asset.file_path.clone(),
                    reason: format!("FFmpeg failed (code {:?}): {stderr}", output.status.code()),
                });
            }

            fs::rename(&temp_path, &preview_path).map_err(|e| {
                let _ = fs::remove_file(&temp_path);
                MediaEngineError::Io {
                    path: preview_path.clone(),
                    source: e,
                }
            })?;

            Ok(preview_path)
        }
    }
}

/// Render the top-most renderable frame at the given timeline frame.
///
/// Returns `Ok(None)` when no clip at this frame has decodable media, so the
/// preview monitor shows its empty state instead of an error.
pub fn render_sequence_frame(
    sequence: &Sequence,
    project: &ProjectDocument,
    timeline_frame: i64,
    cache_dir: &Path,
) -> Result<Option<PathBuf>, MediaEngineError> {
    let Some((_track_id, clip)) = resolve_renderable_video_clip(sequence, project, timeline_frame)
    else {
        return Ok(None);
    };

    let Some(asset) = project.get_asset(clip.asset_id) else {
        // resolve_renderable_video_clip already guarantees this; keep the explicit
        // error so a future change to the resolver fails loudly rather than silently.
        return Err(MediaEngineError::FrameExtractionFailed {
            path: PathBuf::from(format!("asset:{}", clip.asset_id)),
            reason: format!(
                "Media asset with ID '{}' not found in project",
                clip.asset_id
            ),
        });
    };

    let asset_fps = asset.frame_rate.unwrap_or(sequence.fps);
    let source_frame = timeline_to_source_frame(&clip, timeline_frame, asset_fps, sequence.fps);
    let frame_path = extract_preview_frame(asset, source_frame, cache_dir)?;
    Ok(Some(frame_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use nodavideo_project::{AssetId, MediaAssetStatus, TimeRational};
    use nodavideo_timeline::{Clip, ClipId, TextStyle};

    fn test_asset(file_path: PathBuf, media_type: MediaType, fps: FrameRate) -> MediaAsset {
        MediaAsset {
            id: AssetId::new(),
            original_filename: "source.mp4".into(),
            file_path,
            media_type,
            file_size: 1,
            import_timestamp: Utc::now().to_rfc3339(),
            duration: Some(TimeRational::new(2, 1)),
            width: Some(320),
            height: Some(240),
            frame_rate: Some(fps),
            video_codec: Some("h264".into()),
            pixel_format: Some("yuv420p".into()),
            audio_codec: None,
            audio_sample_rate: None,
            audio_channels: None,
            audio_bitrate: None,
            status: MediaAssetStatus::Ready,
            error_message: None,
        }
    }

    fn setup() -> (tempfile::TempDir, ProjectDocument, Sequence, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let media_path = dir.path().join("source.mp4");
        fs::write(&media_path, b"fake").unwrap();
        (
            dir,
            ProjectDocument::new("Preview Test"),
            Sequence::default_1080p24(),
            media_path,
        )
    }

    #[test]
    fn text_overlay_without_asset_is_skipped_for_video_below() {
        let (_dir, mut project, mut sequence, media_path) = setup();
        let asset = test_asset(media_path, MediaType::Video, FrameRate::FPS_24);
        let asset_id = asset.id;
        project.add_asset(asset).unwrap();

        let v2 = sequence.tracks[0].id;
        let v1 = sequence.tracks[1].id;

        let overlay =
            Clip::new_text_overlay(ClipId::new(), v2, "Hello", 0, 48, TextStyle::default())
                .unwrap();
        sequence.add_clip(v2, overlay).unwrap();

        let video = Clip::new(ClipId::new(), asset_id, v1, "Video", 0, 0, 48).unwrap();
        let video_id = video.id;
        sequence.add_clip(v1, video).unwrap();

        let resolved = resolve_renderable_video_clip(&sequence, &project, 10);
        assert_eq!(resolved.map(|(_, clip)| clip.id), Some(video_id));
        // The pure resolver still reports the top-most clip, including overlays.
        assert!(
            resolve_active_video_clip(&sequence, 10)
                .is_some_and(|(_, clip)| clip.text_overlay.is_some())
        );
    }

    #[test]
    fn only_overlay_clips_return_none_instead_of_error() {
        let (_dir, project, mut sequence, _) = setup();
        let v2 = sequence.tracks[0].id;
        let overlay =
            Clip::new_text_overlay(ClipId::new(), v2, "Title", 0, 48, TextStyle::default())
                .unwrap();
        sequence.add_clip(v2, overlay).unwrap();

        assert!(resolve_renderable_video_clip(&sequence, &project, 10).is_none());

        let cache_dir = tempfile::tempdir().unwrap();
        let result = render_sequence_frame(&sequence, &project, 10, cache_dir.path()).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn missing_asset_file_is_not_renderable() {
        let (_dir, mut project, mut sequence, _) = setup();
        let asset = test_asset(
            PathBuf::from("/nonexistent/noda-video-test/missing.mp4"),
            MediaType::Video,
            FrameRate::FPS_24,
        );
        let asset_id = asset.id;
        project.add_asset(asset).unwrap();

        let v1 = sequence.tracks[1].id;
        let clip = Clip::new(ClipId::new(), asset_id, v1, "Missing", 0, 0, 48).unwrap();
        sequence.add_clip(v1, clip).unwrap();

        assert!(resolve_renderable_video_clip(&sequence, &project, 10).is_none());
    }

    #[test]
    fn audio_asset_on_video_track_is_not_renderable() {
        let (_dir, mut project, mut sequence, media_path) = setup();
        let asset = test_asset(media_path, MediaType::Audio, FrameRate::FPS_24);
        let asset_id = asset.id;
        project.add_asset(asset).unwrap();

        let v1 = sequence.tracks[1].id;
        let clip = Clip::new(ClipId::new(), asset_id, v1, "Audio", 0, 0, 48).unwrap();
        sequence.add_clip(v1, clip).unwrap();

        assert!(resolve_renderable_video_clip(&sequence, &project, 10).is_none());
    }

    #[test]
    fn fps_cache_tag_distinguishes_rates() {
        assert_eq!(fps_cache_tag(FrameRate::FPS_24), "24-1");
        assert_eq!(fps_cache_tag(FrameRate::FPS_30), "30-1");
        assert_ne!(
            fps_cache_tag(FrameRate::FPS_29_97),
            fps_cache_tag(FrameRate::FPS_30)
        );
    }

    #[test]
    fn image_assets_are_displayed_without_ffmpeg() {
        let dir = tempfile::tempdir().unwrap();
        let image_path = dir.path().join("still.png");
        fs::write(&image_path, b"fake").unwrap();
        let asset = test_asset(image_path.clone(), MediaType::Image, FrameRate::FPS_24);

        let cache_dir = tempfile::tempdir().unwrap();
        let result = extract_preview_frame(&asset, 7, cache_dir.path()).unwrap();
        assert_eq!(result, image_path);
    }
}
