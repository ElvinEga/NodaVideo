//! Playhead state, transport controls, and frame navigation.

use nodavideo_project::FrameRate;
use nodavideo_timeline::Sequence;
use serde::{Deserialize, Serialize};

/// State of the timeline playhead and transport playback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayheadState {
    /// Current playhead position in frames.
    pub current_frame: u64,
    /// Whether playback is actively running.
    pub is_playing: bool,
    /// Whether playback should loop upon reaching sequence end.
    pub is_looping: bool,
    /// Playback playback rate multiplier (e.g. 1.0 = normal forward, 2.0 = 2x, -1.0 = reverse).
    pub speed: f32,
    /// Sequence frame rate for timecode conversion.
    pub fps: FrameRate,
}

impl Default for PlayheadState {
    fn default() -> Self {
        Self {
            current_frame: 0,
            is_playing: false,
            is_looping: false,
            speed: 1.0,
            fps: FrameRate::FPS_24,
        }
    }
}

/// Controller managing timeline transport, frame seeking, and cut jumping.
#[derive(Debug, Clone, Default)]
pub struct PlaybackController {
    /// Active playhead state.
    pub state: PlayheadState,
    /// Fractional frame accumulator for fractional time-delta playback.
    fractional_frames: f64,
}

impl PlaybackController {
    /// Create a new playback controller with standard defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific frame rate.
    #[must_use]
    pub fn with_fps(fps: FrameRate) -> Self {
        Self {
            state: PlayheadState {
                fps,
                ..Default::default()
            },
            fractional_frames: 0.0,
        }
    }

    /// Current playhead frame.
    #[must_use]
    pub fn current_frame(&self) -> u64 {
        self.state.current_frame
    }

    /// Whether playback is active.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.state.is_playing
    }

    /// Start playback.
    pub fn play(&mut self) {
        self.state.is_playing = true;
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        self.state.is_playing = false;
        self.fractional_frames = 0.0;
    }

    /// Toggle play / pause.
    pub fn toggle_playback(&mut self) {
        if self.state.is_playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Set looping flag.
    pub fn set_looping(&mut self, looping: bool) {
        self.state.is_looping = looping;
    }

    /// Seek playhead to specific frame, clamped to sequence duration bounds.
    pub fn seek_frame(&mut self, frame: u64, max_frames: u64) {
        self.state.current_frame = frame.min(max_frames);
        self.fractional_frames = 0.0;
    }

    /// Step forward by N frames.
    pub fn step_forward(&mut self, frames: u64, max_frames: u64) {
        let target = self.state.current_frame.saturating_add(frames);
        self.seek_frame(target, max_frames);
    }

    /// Step backward by N frames.
    pub fn step_backward(&mut self, frames: u64) {
        self.state.current_frame = self.state.current_frame.saturating_sub(frames);
        self.fractional_frames = 0.0;
    }

    /// Jump playhead directly to start (frame 0).
    pub fn jump_to_start(&mut self) {
        self.state.current_frame = 0;
        self.fractional_frames = 0.0;
    }

    /// Jump playhead directly to end of sequence.
    pub fn jump_to_end(&mut self, max_frames: u64) {
        self.state.current_frame = max_frames;
        self.fractional_frames = 0.0;
    }

    /// Jump to previous cut point (clip start or clip end) before the current playhead.
    pub fn jump_to_previous_cut(&mut self, sequence: &Sequence) {
        let current = self.state.current_frame as i64;
        let mut cut_points = Vec::new();

        cut_points.push(0);
        for track in &sequence.tracks {
            for clip in &track.clips {
                cut_points.push(clip.start_frame);
                cut_points.push(clip.start_frame + clip.duration_frames());
            }
        }

        cut_points.sort_unstable();
        cut_points.dedup();

        if let Some(&prev_cut) = cut_points.iter().rev().find(|&&cut| cut < current) {
            self.state.current_frame = prev_cut.max(0) as u64;
        } else {
            self.state.current_frame = 0;
        }
        self.fractional_frames = 0.0;
    }

    /// Jump to next cut point (clip start or clip end) after the current playhead.
    pub fn jump_to_next_cut(&mut self, sequence: &Sequence) {
        let current = self.state.current_frame as i64;
        let duration = sequence.duration_frames();
        let mut cut_points = Vec::new();

        cut_points.push(duration);
        for track in &sequence.tracks {
            for clip in &track.clips {
                cut_points.push(clip.start_frame);
                cut_points.push(clip.start_frame + clip.duration_frames());
            }
        }

        cut_points.sort_unstable();
        cut_points.dedup();

        if let Some(&next_cut) = cut_points.iter().find(|&&cut| cut > current) {
            self.state.current_frame = (next_cut.min(duration)).max(0) as u64;
        } else {
            self.state.current_frame = duration.max(0) as u64;
        }
        self.fractional_frames = 0.0;
    }

    /// Advance playhead by real-world delta time in seconds (for playback loop tick).
    ///
    /// Returns the new current frame.
    pub fn advance_delta(&mut self, delta_seconds: f64, max_frames: u64) -> u64 {
        if !self.state.is_playing || max_frames == 0 {
            return self.state.current_frame;
        }

        let fps_f64 = self.state.fps.to_f64();
        let frames_to_advance = delta_seconds * fps_f64 * (self.state.speed as f64);
        self.fractional_frames += frames_to_advance;

        let whole_frames = self.fractional_frames.floor() as i64;
        if whole_frames != 0 {
            self.fractional_frames -= whole_frames as f64;
            let current_signed = self.state.current_frame as i64;
            let next_signed = current_signed + whole_frames;

            if next_signed >= max_frames as i64 {
                if self.state.is_looping {
                    self.state.current_frame = (next_signed % max_frames as i64) as u64;
                } else {
                    self.state.current_frame = max_frames;
                    self.pause();
                }
            } else if next_signed < 0 {
                if self.state.is_looping {
                    self.state.current_frame = max_frames;
                } else {
                    self.state.current_frame = 0;
                    self.pause();
                }
            } else {
                self.state.current_frame = next_signed as u64;
            }
        }

        self.state.current_frame
    }

    /// Formatted SMPTE timecode string for the current playhead: `HH:MM:SS:FF`.
    #[must_use]
    pub fn current_timecode(&self) -> String {
        format_timecode(self.state.current_frame, self.state.fps)
    }

    /// Formatted SMPTE timecode string for sequence duration: `HH:MM:SS:FF`.
    #[must_use]
    pub fn duration_timecode(&self, duration_frames: u64) -> String {
        format_timecode(duration_frames, self.state.fps)
    }
}

/// Helper to format frame numbers into standard SMPTE `HH:MM:SS:FF`.
#[must_use]
pub fn format_timecode(frames: u64, fps: FrameRate) -> String {
    let fps_rate = fps.numerator / fps.denominator;
    let fps_u64 = u64::from(if fps_rate == 0 { 24 } else { fps_rate });

    let total_seconds = frames / fps_u64;
    let rem_frames = frames % fps_u64;

    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = total_seconds / 3600;

    format!("{hours:02}:{minutes:02}:{seconds:02}:{rem_frames:02}")
}
