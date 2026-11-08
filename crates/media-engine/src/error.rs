//! Error types for media engine, FFmpeg probing, thumbnails, and waveforms.

use std::path::PathBuf;
use thiserror::Error;

/// Domain errors encountered during media inspection, decoding, or caching.
#[derive(Debug, Error)]
pub enum MediaEngineError {
    #[error("I/O error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "FFprobe executable not found on system PATH. Please ensure FFmpeg/FFprobe is installed."
    )]
    FfprobeNotFound,

    #[error("FFmpeg executable not found on system PATH. Please ensure FFmpeg is installed.")]
    FfmpegNotFound,

    #[error("Media inspection/probe failed for {path:?}: {reason}")]
    ProbeFailed { path: PathBuf, reason: String },

    #[error("Thumbnail generation failed for {path:?}: {reason}")]
    ThumbnailFailed { path: PathBuf, reason: String },

    #[error("Audio waveform extraction failed for {path:?}: {reason}")]
    WaveformFailed { path: PathBuf, reason: String },

    #[error("SQLite media cache database error: {0}")]
    CacheDb(#[from] rusqlite::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Unsupported media format: {0}")]
    UnsupportedFormat(String),

    #[error("Image processing error for {path:?}: {reason}")]
    ImageError { path: PathBuf, reason: String },
}
