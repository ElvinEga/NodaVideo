//! Monotonic audio playback synchronization clock and drift compensation.

use nodavideo_project::FrameRate;

use crate::buffer::{
    SAMPLE_RATE_48K, audio_sample_to_timeline_frame, timeline_frame_to_audio_sample,
};

/// Master synchronization clock driven by audio sample progression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioClock {
    /// Audio sample rate in Hertz (e.g. 48000).
    pub sample_rate: u32,
    /// Current sample position in the audio stream.
    pub current_sample: u64,
    /// Playback state indicator.
    pub is_playing: bool,
}

impl Default for AudioClock {
    fn default() -> Self {
        Self {
            sample_rate: SAMPLE_RATE_48K,
            current_sample: 0,
            is_playing: false,
        }
    }
}

impl AudioClock {
    /// Create a new audio clock at the specified sample rate.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate.max(1),
            current_sample: 0,
            is_playing: false,
        }
    }

    /// Start or resume audio playback.
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// Pause audio playback.
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Advance the audio clock by the given number of processed sample frames.
    pub fn advance_samples(&mut self, frames: usize) {
        if self.is_playing {
            self.current_sample = self.current_sample.saturating_add(frames as u64);
        }
    }

    /// Seek the audio clock to a specific timeline video frame.
    pub fn seek_to_frame(&mut self, frame: i64, fps: FrameRate) {
        let sample = timeline_frame_to_audio_sample(frame, self.sample_rate, fps);
        self.current_sample = sample.max(0) as u64;
    }

    /// Reset the audio clock to sample 0.
    pub fn reset(&mut self) {
        self.current_sample = 0;
        self.is_playing = false;
    }

    /// Current timeline video frame corresponding to the audio clock position.
    #[must_use]
    pub fn current_timeline_frame(&self, fps: FrameRate) -> i64 {
        audio_sample_to_timeline_frame(self.current_sample as i64, self.sample_rate, fps)
    }

    /// Current timeline playback position in seconds as a floating-point value.
    #[must_use]
    pub fn current_time_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.current_sample as f64 / self.sample_rate as f64
        }
    }

    /// Calculate the synchronization drift in integer video frames between visual rendering and audio clock.
    ///
    /// - Returns a positive number if visual rendering is ahead of audio.
    /// - Returns a negative number if visual rendering is lagging behind audio.
    #[must_use]
    pub fn drift_frames(&self, visual_frame: i64, fps: FrameRate) -> i64 {
        let audio_frame = self.current_timeline_frame(fps);
        visual_frame - audio_frame
    }
}
