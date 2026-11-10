//! Audio buffer data structures and frame-to-sample timing conversions.

use nodavideo_project::FrameRate;
use serde::{Deserialize, Serialize};

use crate::error::AudioError;

/// Broadcast standard sample rate (48 kHz).
pub const SAMPLE_RATE_48K: u32 = 48_000;

/// CD standard sample rate (44.1 kHz).
pub const SAMPLE_RATE_44K1: u32 = 44_100;

/// Standard stereo channel count (Left, Right).
pub const CHANNELS_STEREO: usize = 2;

/// Standard mono channel count.
pub const CHANNELS_MONO: usize = 1;

/// Format specification for an audio buffer or stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AudioFormat {
    /// Sample rate in Hertz (e.g. 48000).
    pub sample_rate: u32,
    /// Number of audio channels (e.g. 2 for stereo).
    pub channels: usize,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: SAMPLE_RATE_48K,
            channels: CHANNELS_STEREO,
        }
    }
}

impl AudioFormat {
    /// Create and validate a new audio format.
    pub fn new(sample_rate: u32, channels: usize) -> Result<Self, AudioError> {
        if sample_rate == 0 {
            return Err(AudioError::InvalidSampleRate(sample_rate));
        }
        if channels == 0 {
            return Err(AudioError::InvalidChannels(channels));
        }
        Ok(Self {
            sample_rate,
            channels,
        })
    }

    /// Stereo 48 kHz format.
    #[must_use]
    pub const fn stereo_48k() -> Self {
        Self {
            sample_rate: SAMPLE_RATE_48K,
            channels: CHANNELS_STEREO,
        }
    }

    /// Stereo 44.1 kHz format.
    #[must_use]
    pub const fn stereo_44k1() -> Self {
        Self {
            sample_rate: SAMPLE_RATE_44K1,
            channels: CHANNELS_STEREO,
        }
    }
}

/// An interleaved PCM floating-point audio buffer.
///
/// Samples are stored sequentially per frame: `[L0, R0, L1, R1, ...]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioBuffer {
    /// Format specification for this buffer.
    pub format: AudioFormat,
    /// Interleaved PCM samples normalized to `[-1.0, 1.0]`.
    pub data: Vec<f32>,
}

impl AudioBuffer {
    /// Create a zero-initialized audio buffer with the given format and frame count.
    #[must_use]
    pub fn new(format: AudioFormat, frame_count: usize) -> Self {
        let total_samples = frame_count * format.channels;
        Self {
            format,
            data: vec![0.0; total_samples],
        }
    }

    /// Create an audio buffer from pre-existing interleaved samples.
    pub fn from_interleaved(format: AudioFormat, data: Vec<f32>) -> Result<Self, AudioError> {
        if !data.is_empty() && !data.len().is_multiple_of(format.channels) {
            return Err(AudioError::BufferLengthMismatch {
                expected: (data.len() / format.channels) * format.channels,
                actual: data.len(),
            });
        }
        Ok(Self { format, data })
    }

    /// Number of sample frames in this buffer (i.e. samples per channel).
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.data
            .len()
            .checked_div(self.format.channels)
            .unwrap_or(0)
    }

    /// Read a sample at a given frame and channel index.
    #[must_use]
    pub fn get_sample(&self, frame: usize, channel: usize) -> f32 {
        if channel >= self.format.channels {
            return 0.0;
        }
        let index = frame * self.format.channels + channel;
        self.data.get(index).copied().unwrap_or(0.0)
    }

    /// Set a sample at a given frame and channel index.
    pub fn set_sample(&mut self, frame: usize, channel: usize, val: f32) {
        if channel < self.format.channels {
            let index = frame * self.format.channels + channel;
            if index < self.data.len() {
                self.data[index] = val;
            }
        }
    }

    /// Add a sample value to an existing frame and channel index.
    pub fn add_sample(&mut self, frame: usize, channel: usize, val: f32) {
        if channel < self.format.channels {
            let index = frame * self.format.channels + channel;
            if index < self.data.len() {
                self.data[index] += val;
            }
        }
    }

    /// Scale all samples in the buffer by a constant factor.
    pub fn scale(&mut self, factor: f32) {
        for sample in &mut self.data {
            *sample *= factor;
        }
    }

    /// Clamp all samples to the given bounds (e.g. `[-1.0, 1.0]`).
    pub fn clamp(&mut self, min: f32, max: f32) {
        for sample in &mut self.data {
            *sample = sample.clamp(min, max);
        }
    }

    /// Clear all samples in the buffer to silence (0.0).
    pub fn clear(&mut self) {
        self.data.fill(0.0);
    }
}

/// Calculate the number of audio samples per video frame at a given sample rate and frame rate.
#[must_use]
pub fn samples_per_video_frame(sample_rate: u32, fps: FrameRate) -> usize {
    let rate = sample_rate as f64;
    let fps_f64 = fps.to_f64();
    if fps_f64 <= 0.0 {
        return 0;
    }
    (rate / fps_f64).round() as usize
}

/// Convert a timeline video frame index to an audio sample offset.
#[must_use]
pub fn timeline_frame_to_audio_sample(frame: i64, sample_rate: u32, fps: FrameRate) -> i64 {
    if frame <= 0 {
        return 0;
    }
    let fps_f64 = fps.to_f64();
    if fps_f64 <= 0.0 {
        return 0;
    }
    ((frame as f64 * sample_rate as f64) / fps_f64).round() as i64
}

/// Convert an audio sample offset to the corresponding timeline video frame index.
#[must_use]
pub fn audio_sample_to_timeline_frame(sample: i64, sample_rate: u32, fps: FrameRate) -> i64 {
    if sample <= 0 || sample_rate == 0 {
        return 0;
    }
    let fps_f64 = fps.to_f64();
    ((sample as f64 * fps_f64) / sample_rate as f64).floor() as i64
}
