//! Error types for audio processing and playback synchronization.

use thiserror::Error;

/// Domain errors occurring within the audio engine.
#[derive(Debug, Error)]
pub enum AudioError {
    /// Invalid sample rate (must be greater than 0, e.g. 44100 or 48000 Hz).
    #[error("Invalid sample rate: {0} Hz")]
    InvalidSampleRate(u32),

    /// Invalid channel count (must be at least 1).
    #[error("Invalid channel count: {0}")]
    InvalidChannels(usize),

    /// Buffer length or channel count mismatch during processing or copying.
    #[error("Buffer length mismatch: expected {expected}, got {actual}")]
    BufferLengthMismatch {
        /// Expected sample length.
        expected: usize,
        /// Actual sample length.
        actual: usize,
    },

    /// Generic audio processing or decoding failure.
    #[error("Audio processing failure: {0}")]
    ProcessingFailed(String),
}
