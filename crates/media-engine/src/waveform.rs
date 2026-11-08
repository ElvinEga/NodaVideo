//! Audio waveform extraction and normalization.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use nodavideo_project::{AssetId, TimeRational};

use crate::error::MediaEngineError;
use crate::probe::is_ffmpeg_available;

/// Points generated per second of audio for timeline visualization.
pub const WAVEFORM_POINTS_PER_SECOND: u32 = 100;
/// Decoded internal audio sample rate for waveform calculation.
const WAVEFORM_SAMPLE_RATE: u32 = 8000;
/// Samples per peak bucket.
const SAMPLES_PER_POINT: usize = (WAVEFORM_SAMPLE_RATE / WAVEFORM_POINTS_PER_SECOND) as usize;

/// Normalized audio waveform peak data cached on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaveformData {
    /// Associated media asset ID.
    pub asset_id: AssetId,
    /// Duration of the audio stream.
    pub duration: TimeRational,
    /// Number of peak data points per second.
    pub points_per_second: u32,
    /// Normalized amplitude peaks (0.0 .. 1.0).
    pub peaks: Vec<f32>,
}

impl WaveformData {
    /// Save waveform data to disk.
    pub fn save_to_file(&self, path: &Path) -> Result<(), MediaEngineError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| MediaEngineError::Io {
                path: parent.to_path_buf(),
                source: err,
            })?;
        }

        let bytes = serde_json::to_vec(self).map_err(MediaEngineError::Json)?;
        fs::write(path, bytes).map_err(|err| MediaEngineError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;

        Ok(())
    }

    /// Load cached waveform data from disk.
    pub fn load_from_file(path: &Path) -> Result<Self, MediaEngineError> {
        let bytes = fs::read(path).map_err(|err| MediaEngineError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;

        serde_json::from_slice(&bytes).map_err(MediaEngineError::Json)
    }
}

/// Extract normalized audio waveform data via FFmpeg and cache to disk.
pub fn generate_waveform(
    source_path: &Path,
    asset_id: AssetId,
    duration: Option<TimeRational>,
    cache_dir: &Path,
) -> Result<(WaveformData, PathBuf), MediaEngineError> {
    let waves_dir = cache_dir.join("waveforms");
    fs::create_dir_all(&waves_dir).map_err(|err| MediaEngineError::Io {
        path: waves_dir.clone(),
        source: err,
    })?;

    let output_path = waves_dir.join(format!("{asset_id}.wave"));

    // If cached waveform already exists, load and return it
    if output_path.exists() {
        if let Ok(cached) = WaveformData::load_from_file(&output_path) {
            return Ok((cached, output_path));
        }
    }

    if !is_ffmpeg_available() {
        return Err(MediaEngineError::FfmpegNotFound);
    }

    // Decode audio to mono 8kHz 32-bit float raw PCM to stdout
    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(source_path)
        .arg("-vn")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg(WAVEFORM_SAMPLE_RATE.to_string())
        .arg("-f")
        .arg("f32le")
        .arg("-")
        .output()
        .map_err(|err| MediaEngineError::Io {
            path: source_path.to_path_buf(),
            source: err,
        })?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(MediaEngineError::WaveformFailed {
            path: source_path.to_path_buf(),
            reason: if err_msg.is_empty() {
                "ffmpeg exited with non-zero status during waveform extraction".into()
            } else {
                err_msg
            },
        });
    }

    let raw_bytes = output.stdout;
    let float_count = raw_bytes.len() / 4;
    let mut samples = Vec::with_capacity(float_count);

    for chunk in raw_bytes.chunks_exact(4) {
        let bytes: [u8; 4] = [chunk[0], chunk[1], chunk[2], chunk[3]];
        let sample = f32::from_le_bytes(bytes);
        samples.push(sample.abs());
    }

    // Compute peaks per bucket
    let mut peaks = Vec::new();
    let mut global_max: f32 = 0.0;

    for chunk in samples.chunks(SAMPLES_PER_POINT) {
        let mut bucket_max: f32 = 0.0;
        for &s in chunk {
            if s > bucket_max {
                bucket_max = s;
            }
        }
        if bucket_max > global_max {
            global_max = bucket_max;
        }
        peaks.push(bucket_max);
    }

    // Normalize peaks to 0.0 .. 1.0 range
    if global_max > 0.0 {
        for p in &mut peaks {
            *p /= global_max;
        }
    }

    let audio_duration =
        duration.unwrap_or_else(|| TimeRational::new(samples.len() as i64, WAVEFORM_SAMPLE_RATE));

    let waveform = WaveformData {
        asset_id,
        duration: audio_duration,
        points_per_second: WAVEFORM_POINTS_PER_SECOND,
        peaks,
    };

    waveform.save_to_file(&output_path)?;

    Ok((waveform, output_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waveform_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wave");

        let wave = WaveformData {
            asset_id: AssetId::new(),
            duration: TimeRational::new(5, 1),
            points_per_second: 100,
            peaks: vec![0.0, 0.25, 0.5, 0.75, 1.0],
        };

        wave.save_to_file(&path).unwrap();
        let loaded = WaveformData::load_from_file(&path).unwrap();

        assert_eq!(wave, loaded);
    }
}
