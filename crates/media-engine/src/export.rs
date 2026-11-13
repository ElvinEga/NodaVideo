//! Multitrack video and audio export engine generating broadcast-standard MP4.

use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

use nodavideo_project::{AssetId, FrameRate, ProjectDocument};
use nodavideo_timeline::{Clip, Sequence, TrackType};
use serde::{Deserialize, Serialize};

use crate::binaries::ffmpeg_command;
use crate::error::MediaEngineError;
use crate::time_math::{format_seconds, frames_to_seconds, retiming_factor};

/// Parameters governing sequence export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportSettings {
    /// Destination MP4 file path.
    pub output_path: PathBuf,
    /// Export frame width in pixels.
    pub width: u32,
    /// Export frame height in pixels.
    pub height: u32,
    /// Export frame rate.
    pub fps: FrameRate,
    /// Target video bitrate in kilobits per second.
    pub video_bitrate_kbps: u32,
    /// Target audio bitrate in kilobits per second.
    pub audio_bitrate_kbps: u32,
    /// Video codec identifier (default: "libx264").
    pub video_codec: String,
    /// Audio codec identifier (default: "aac").
    pub audio_codec: String,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("output.mp4"),
            width: 1920,
            height: 1080,
            fps: FrameRate::FPS_24,
            video_bitrate_kbps: 8000,
            audio_bitrate_kbps: 192,
            video_codec: "libx264".into(),
            audio_codec: "aac".into(),
        }
    }
}

/// Statistics and metadata returned upon successful export completion.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportReport {
    pub output_path: PathBuf,
    pub duration_seconds: f64,
    pub file_size_bytes: u64,
    pub total_clips_exported: usize,
}

/// Export a timeline sequence to an MP4 video file using FFmpeg filtergraph.
pub fn export_sequence(
    sequence: &Sequence,
    project: &ProjectDocument,
    settings: &ExportSettings,
) -> Result<ExportReport, MediaEngineError> {
    let duration_frames = sequence.duration_frames();
    if duration_frames <= 0 {
        return Err(MediaEngineError::ExportFailed {
            reason: "Cannot export empty timeline sequence: no clips placed".into(),
        });
    }

    let seq_fps = settings.fps.to_f64().max(1.0);
    let total_duration_sec = frames_to_seconds(duration_frames, settings.fps);

    // Collect active video clips (ordered from bottom track to top track so higher tracks overlay on top).
    // The third element is the asset's frame rate, used to convert source-frame values
    // (`source_in`, `duration_frames`) into source seconds and to retime onto the sequence.
    let mut video_clips: Vec<(usize, Clip, FrameRate)> = Vec::new();
    // Collect active audio clips
    let mut audio_clips: Vec<(usize, Clip, FrameRate)> = Vec::new();

    // Map assets to unique FFmpeg input indexes
    let mut asset_input_indices: HashMap<AssetId, usize> = HashMap::new();
    let mut input_paths: Vec<PathBuf> = Vec::new();

    // Collect video tracks in reverse order (V1 before V2 so V2 overlays on V1)
    let video_tracks: Vec<_> = sequence
        .tracks
        .iter()
        .filter(|t| t.track_type == TrackType::Video && t.visible)
        .rev()
        .collect();

    for track in video_tracks {
        for clip in &track.clips {
            if !clip.enabled {
                continue;
            }
            if let Some(asset) = project.get_asset(clip.asset_id) {
                if !asset.file_path.exists() {
                    return Err(MediaEngineError::ExportFailed {
                        reason: format!("Missing media asset file: {}", asset.file_path.display()),
                    });
                }
                let input_idx = *asset_input_indices.entry(clip.asset_id).or_insert_with(|| {
                    input_paths.push(asset.file_path.clone());
                    input_paths.len() - 1
                });
                let asset_fps = asset.frame_rate.unwrap_or(settings.fps);
                video_clips.push((input_idx, clip.clone(), asset_fps));
            }
        }
    }

    // Collect audio tracks
    for track in &sequence.tracks {
        if track.track_type == TrackType::Audio && !track.muted {
            for clip in &track.clips {
                if !clip.enabled {
                    continue;
                }
                if let Some(asset) = project.get_asset(clip.asset_id) {
                    if !asset.file_path.exists() {
                        return Err(MediaEngineError::ExportFailed {
                            reason: format!(
                                "Missing media asset file: {}",
                                asset.file_path.display()
                            ),
                        });
                    }
                    let input_idx =
                        *asset_input_indices.entry(clip.asset_id).or_insert_with(|| {
                            input_paths.push(asset.file_path.clone());
                            input_paths.len() - 1
                        });
                    let asset_fps = asset.frame_rate.unwrap_or(settings.fps);
                    audio_clips.push((input_idx, clip.clone(), asset_fps));
                }
            }
        }
    }

    // Build FFmpeg command using the centrally resolved executable
    let mut cmd = ffmpeg_command()?;

    // 1. Inputs
    for path in &input_paths {
        cmd.arg("-i").arg(path);
    }

    // 2. Filtergraph construction
    let mut filtergraph = String::new();

    // Video canvas: start with black background matching target resolution, fps, and duration
    let _ = write!(
        filtergraph,
        "color=c=black:s={}x{}:r={}:d={}[vbase];",
        settings.width,
        settings.height,
        seq_fps,
        format_seconds(total_duration_sec)
    );

    let mut current_video_link = "vbase".to_string();

    for (k, (input_idx, clip, asset_fps)) in video_clips.iter().enumerate() {
        // Timeline frames use the SEQUENCE rate; source-frame offsets use the ASSET rate.
        let start_sec = frames_to_seconds(clip.start_frame, settings.fps);
        let end_sec = frames_to_seconds(clip.end_frame(), settings.fps);
        let source_in_sec = frames_to_seconds(clip.source_in, *asset_fps);
        let source_dur_sec = frames_to_seconds(clip.duration_frames(), *asset_fps);
        // Retime the trimmed source so its on-screen duration matches the timeline
        // exactly when asset fps != sequence fps. Equal rates produce a factor of 1.0.
        let factor = retiming_factor(*asset_fps, settings.fps);

        // Process clip segment: trim, retime + offset PTS, scale to fit, pad to canvas center
        let processed_label = format!("vclip_{k}");
        let _ = write!(
            filtergraph,
            "[{input_idx}:v]trim=start={}:duration={},setpts=(PTS-STARTPTS)*{}+{}/TB,scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,format=yuv420p[{processed_label}];",
            format_seconds(source_in_sec),
            format_seconds(source_dur_sec),
            format_seconds(factor),
            format_seconds(start_sec),
            settings.width,
            settings.height,
            settings.width,
            settings.height
        );

        let next_video_link = if k == video_clips.len() - 1 {
            "vfinal".to_string()
        } else {
            format!("voverlay_{k}")
        };

        let _ = write!(
            filtergraph,
            "[{current_video_link}][{processed_label}]overlay=x=0:y=0:enable='between(t,{},{})'[{}];",
            format_seconds(start_sec),
            format_seconds(end_sec),
            next_video_link
        );

        current_video_link = next_video_link;
    }

    if video_clips.is_empty() {
        current_video_link = "vbase".to_string();
    }

    // Audio mixing
    let mut has_audio = false;
    let mut audio_labels = Vec::new();

    if !audio_clips.is_empty() {
        has_audio = true;
        // Start with silent base audio
        let _ = write!(
            filtergraph,
            "anullsrc=channel_layout=stereo:sample_rate=48000:d={}[abase];",
            format_seconds(total_duration_sec)
        );
        audio_labels.push("[abase]".to_string());

        for (j, (input_idx, clip, asset_fps)) in audio_clips.iter().enumerate() {
            let start_sec = frames_to_seconds(clip.start_frame, settings.fps);
            let delay_ms = (start_sec * 1000.0).round() as i64;
            let source_in_sec = frames_to_seconds(clip.source_in, *asset_fps);
            let source_dur_sec = frames_to_seconds(clip.duration_frames(), *asset_fps);
            let factor = retiming_factor(*asset_fps, settings.fps);

            // `atempo` keeps audio in sync with the retimed video. FFmpeg's single
            // instance accepts 0.5..=2.0; outside that range audio retiming is
            // deferred to the Phase B decode engine (documented limitation).
            let tempo_filter = if (factor - 1.0).abs() > 1e-6 && (0.5..=2.0).contains(&factor) {
                format!("atempo={},", format_seconds(factor))
            } else {
                String::new()
            };

            let audio_label = format!("aclip_{j}");
            let _ = write!(
                filtergraph,
                "[{input_idx}:a]atrim=start={}:duration={},asetpts=PTS-STARTPTS,{tempo_filter}adelay={delay_ms}|{delay_ms},aformat=sample_fmts=fltp:sample_rates=48000:channel_layouts=stereo[{audio_label}];",
                format_seconds(source_in_sec),
                format_seconds(source_dur_sec),
            );
            audio_labels.push(format!("[{audio_label}]"));
        }

        let mix_inputs = audio_labels.len();
        let mix_str = audio_labels.join("");
        // normalize=0 prevents the silent base track from dividing every real
        // clip's amplitude by the number of inputs.
        let _ = write!(
            filtergraph,
            "{mix_str}amix=inputs={mix_inputs}:duration=first:dropout_transition=0:normalize=0[afinal];"
        );
    }

    // Remove trailing semicolon if any
    if filtergraph.ends_with(';') {
        filtergraph.pop();
    }

    // Force a constant frame rate output so the MP4 has exactly the sequence fps,
    // independent of the source streams' timestamps.
    let _ = write!(
        filtergraph,
        ";[{current_video_link}]fps={},format=yuv420p[vout]",
        format_seconds(seq_fps)
    );

    // Filter complex argument
    cmd.arg("-filter_complex").arg(&filtergraph);

    // Map output streams
    cmd.arg("-map").arg("[vout]");
    if has_audio {
        cmd.arg("-map").arg("[afinal]");
    }

    // Video codec and bitrate
    cmd.arg("-c:v")
        .arg(&settings.video_codec)
        .arg("-b:v")
        .arg(format!("{}k", settings.video_bitrate_kbps))
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-r")
        .arg(format_seconds(seq_fps))
        .arg("-fps_mode")
        .arg("cfr");

    // Audio codec and bitrate
    if has_audio {
        cmd.arg("-c:a")
            .arg(&settings.audio_codec)
            .arg("-b:a")
            .arg(format!("{}k", settings.audio_bitrate_kbps));
    }

    // Ensure parent directory exists
    if let Some(parent) = settings.output_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Global flags
    cmd.arg("-movflags")
        .arg("+faststart")
        .arg("-t")
        .arg(format_seconds(total_duration_sec))
        .arg("-y")
        .arg(&settings.output_path);

    // Execute export process
    let output = cmd.output().map_err(|e| MediaEngineError::Io {
        path: settings.output_path.clone(),
        source: e,
    })?;

    if !output.status.success() || !settings.output_path.exists() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MediaEngineError::ExportFailed {
            reason: format!(
                "FFmpeg export process exited with error ({:?}): {stderr}",
                output.status.code()
            ),
        });
    }

    let file_size_bytes = fs::metadata(&settings.output_path).map_or(0, |m| m.len());

    Ok(ExportReport {
        output_path: settings.output_path.clone(),
        duration_seconds: total_duration_sec,
        file_size_bytes,
        total_clips_exported: video_clips.len() + audio_clips.len(),
    })
}
