//! Media metadata inspection and stream probing via FFprobe and native image readers.

use chrono::Utc;
use serde::Deserialize;
use std::path::Path;

use nodavideo_project::{
    AssetId, FrameRate, MediaAsset, MediaAssetStatus, MediaType, TimeRational,
};

use crate::binaries::{ffprobe_command, is_ffprobe_available};
use crate::error::MediaEngineError;

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    pix_fmt: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u16>,
    bit_rate: Option<String>,
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    #[allow(dead_code)]
    size: Option<String>,
    #[allow(dead_code)]
    bit_rate: Option<String>,
}

/// Helper to parse frame rates in fractional formats like "24/1" or "30000/1001".
fn parse_rational_fps(s: &str) -> Option<FrameRate> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 {
        let num: u32 = parts[0].trim().parse().ok()?;
        let den: u32 = parts[1].trim().parse().ok()?;
        if num > 0 && den > 0 {
            return Some(FrameRate::new(num, den));
        }
    } else if let Ok(val) = s.trim().parse::<f64>() {
        if val > 0.0 {
            return Some(FrameRate::new((val * 1000.0).round() as u32, 1000));
        }
    }
    None
}

/// Inspect a media file on disk and construct a fully populated `MediaAsset`.
pub fn probe_media_file(path: &Path) -> Result<MediaAsset, MediaEngineError> {
    if !path.exists() {
        return Err(MediaEngineError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File does not exist"),
        });
    }

    let file_size = std::fs::metadata(path)
        .map_err(|err| MediaEngineError::Io {
            path: path.to_path_buf(),
            source: err,
        })?
        .len();

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let media_type = MediaType::from_extension(path).ok_or_else(|| {
        MediaEngineError::UnsupportedFormat(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string(),
        )
    })?;

    let asset_id = AssetId::new();
    let import_timestamp = Utc::now().to_rfc3339();

    // Fast-path inspection for static images using pure-Rust image reader
    if media_type == MediaType::Image {
        match image::image_dimensions(path) {
            Ok((w, h)) => {
                return Ok(MediaAsset {
                    id: asset_id,
                    original_filename: filename,
                    file_path: path.to_path_buf(),
                    media_type: MediaType::Image,
                    file_size,
                    import_timestamp,
                    duration: None,
                    width: Some(w),
                    height: Some(h),
                    frame_rate: None,
                    video_codec: path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase()),
                    pixel_format: None,
                    audio_codec: None,
                    audio_sample_rate: None,
                    audio_channels: None,
                    audio_bitrate: None,
                    status: MediaAssetStatus::Ready,
                    error_message: None,
                });
            }
            Err(err) => {
                // If pure-Rust image reading fails and ffprobe is available, fallback to ffprobe
                if !is_ffprobe_available() {
                    return Err(MediaEngineError::ImageError {
                        path: path.to_path_buf(),
                        reason: err.to_string(),
                    });
                }
            }
        }
    }

    // Probe via FFprobe for video, audio, or fallback
    if !is_ffprobe_available() {
        return Err(MediaEngineError::FfprobeNotFound);
    }

    let output = ffprobe_command()?
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(path)
        .output()
        .map_err(|err| MediaEngineError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(MediaEngineError::ProbeFailed {
            path: path.to_path_buf(),
            reason: if err_msg.is_empty() {
                "ffprobe exited with non-zero status".into()
            } else {
                err_msg
            },
        });
    }

    let parsed: FfprobeOutput =
        serde_json::from_slice(&output.stdout).map_err(|err| MediaEngineError::ProbeFailed {
            path: path.to_path_buf(),
            reason: format!("Failed to parse ffprobe JSON output: {err}"),
        })?;

    let mut duration_sec: Option<f64> = None;
    if let Some(ref fmt) = parsed.format {
        if let Some(ref dur_str) = fmt.duration {
            duration_sec = dur_str.parse::<f64>().ok();
        }
    }

    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut frame_rate: Option<FrameRate> = None;
    let mut video_codec: Option<String> = None;
    let mut pixel_format: Option<String> = None;

    let mut audio_codec: Option<String> = None;
    let mut audio_sample_rate: Option<u32> = None;
    let mut audio_channels: Option<u16> = None;
    let mut audio_bitrate: Option<u64> = None;

    for stream in &parsed.streams {
        match stream.codec_type.as_deref() {
            Some("video") if width.is_none() => {
                width = stream.width;
                height = stream.height;
                video_codec.clone_from(&stream.codec_name);
                pixel_format.clone_from(&stream.pix_fmt);

                if let Some(ref r_fps) = stream.r_frame_rate {
                    frame_rate = parse_rational_fps(r_fps);
                }
                if frame_rate.is_none() {
                    if let Some(ref avg_fps) = stream.avg_frame_rate {
                        frame_rate = parse_rational_fps(avg_fps);
                    }
                }
                if duration_sec.is_none() {
                    if let Some(ref dur_str) = stream.duration {
                        duration_sec = dur_str.parse::<f64>().ok();
                    }
                }
            }
            Some("audio") if audio_codec.is_none() => {
                audio_codec.clone_from(&stream.codec_name);
                audio_channels = stream.channels;
                if let Some(ref sr) = stream.sample_rate {
                    audio_sample_rate = sr.parse::<u32>().ok();
                }
                if let Some(ref br) = stream.bit_rate {
                    audio_bitrate = br.parse::<u64>().ok();
                }
                if duration_sec.is_none() {
                    if let Some(ref dur_str) = stream.duration {
                        duration_sec = dur_str.parse::<f64>().ok();
                    }
                }
            }
            _ => {}
        }
    }

    let duration = duration_sec.map(|sec| TimeRational::from_seconds_f64(sec, 1000));

    Ok(MediaAsset {
        id: asset_id,
        original_filename: filename,
        file_path: path.to_path_buf(),
        media_type,
        file_size,
        import_timestamp,
        duration,
        width,
        height,
        frame_rate,
        video_codec,
        pixel_format,
        audio_codec,
        audio_sample_rate,
        audio_channels,
        audio_bitrate,
        status: MediaAssetStatus::Ready,
        error_message: None,
    })
}
