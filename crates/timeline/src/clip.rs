//! Timeline clip domain model representing placed media segments.

use nodavideo_project::AssetId;
use serde::{Deserialize, Serialize};

use crate::error::TimelineError;
use crate::id::{ClipId, TrackId};
use crate::text::{TextOverlay, TextStyle};

/// Spatial and visual transform properties for a video/image/composition clip.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClipTransform {
    /// Horizontal translation offset in pixels (0.0 = centered).
    pub position_x: f64,
    /// Vertical translation offset in pixels (0.0 = centered).
    pub position_y: f64,
    /// Scale factor along horizontal axis (1.0 = 100%).
    pub scale_x: f64,
    /// Scale factor along vertical axis (1.0 = 100%).
    pub scale_y: f64,
    /// Rotation in degrees clockwise.
    pub rotation_degrees: f64,
    /// Visual opacity (0.0 to 1.0).
    pub opacity: f64,
}

impl Default for ClipTransform {
    fn default() -> Self {
        Self {
            position_x: 0.0,
            position_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation_degrees: 0.0,
            opacity: 1.0,
        }
    }
}

impl Eq for ClipTransform {}
impl PartialEq for ClipTransform {
    fn eq(&self, other: &Self) -> bool {
        self.position_x.to_bits() == other.position_x.to_bits()
            && self.position_y.to_bits() == other.position_y.to_bits()
            && self.scale_x.to_bits() == other.scale_x.to_bits()
            && self.scale_y.to_bits() == other.scale_y.to_bits()
            && self.rotation_degrees.to_bits() == other.rotation_degrees.to_bits()
            && self.opacity.to_bits() == other.opacity.to_bits()
    }
}

impl ClipTransform {
    /// Whether this transform is identical to the default identity state.
    pub fn is_identity(&self) -> bool {
        self == &Self::default()
    }

    /// Return a clamped version ensuring opacity is within [0.0, 1.0].
    #[must_use]
    pub fn clamped(&self) -> Self {
        Self {
            position_x: self.position_x,
            position_y: self.position_y,
            scale_x: self.scale_x.max(0.001),
            scale_y: self.scale_y.max(0.001),
            rotation_degrees: self.rotation_degrees % 360.0,
            opacity: self.opacity.clamp(0.0, 1.0),
        }
    }
}

/// Audio properties for a clip segment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClipAudioParams {
    /// Volume gain in decibels (0.0 = unity gain, negative = attenuate, positive = boost).
    pub volume_db: f64,
    /// Stereo panning (-1.0 = full left, 0.0 = center, +1.0 = full right).
    pub pan: f64,
}

impl Default for ClipAudioParams {
    fn default() -> Self {
        Self {
            volume_db: 0.0,
            pan: 0.0,
        }
    }
}

impl Eq for ClipAudioParams {}
impl PartialEq for ClipAudioParams {
    fn eq(&self, other: &Self) -> bool {
        self.volume_db.to_bits() == other.volume_db.to_bits()
            && self.pan.to_bits() == other.pan.to_bits()
    }
}

impl ClipAudioParams {
    /// Whether this audio parameter is identical to the default unity gain & center pan.
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    /// Return a clamped version ensuring pan is within [-1.0, 1.0] and volume within [-100.0, 24.0] dB.
    #[must_use]
    pub fn clamped(&self) -> Self {
        Self {
            volume_db: self.volume_db.clamp(-100.0, 24.0),
            pan: self.pan.clamp(-1.0, 1.0),
        }
    }
}

/// Placed segment of a media asset positioned on a timeline track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clip {
    /// Unique identifier for this clip instance.
    pub id: ClipId,
    /// Reference to the underlying imported media asset.
    pub asset_id: AssetId,
    /// Reference to the containing track.
    pub track_id: TrackId,
    /// Human-readable label (defaults to media asset name).
    pub name: String,
    /// Timeline position in frames where clip playback begins.
    pub start_frame: i64,
    /// In-point frame within the source media asset.
    pub source_in: i64,
    /// Out-point frame within the source media asset (exclusive).
    pub source_out: i64,
    /// Whether the clip is active and audible/visible.
    pub enabled: bool,
    /// Spatial and visual transform attributes.
    #[serde(default)]
    pub transform: ClipTransform,
    /// Audio level and panning attributes.
    #[serde(default)]
    pub audio_params: ClipAudioParams,
    /// Optional text overlay and caption attributes.
    #[serde(default)]
    pub text_overlay: Option<TextOverlay>,
}

impl Clip {
    /// Create a new clip, validating timing boundaries.
    pub fn new(
        id: ClipId,
        asset_id: AssetId,
        track_id: TrackId,
        name: impl Into<String>,
        start_frame: i64,
        source_in: i64,
        source_out: i64,
    ) -> Result<Self, TimelineError> {
        if start_frame < 0 {
            return Err(TimelineError::InvalidStartFrame(start_frame));
        }
        if source_in < 0 || source_out <= source_in {
            return Err(TimelineError::InvalidTrim {
                source_in,
                source_out,
            });
        }

        Ok(Self {
            id,
            asset_id,
            track_id,
            name: name.into(),
            start_frame,
            source_in,
            source_out,
            enabled: true,
            transform: ClipTransform::default(),
            audio_params: ClipAudioParams::default(),
            text_overlay: None,
        })
    }

    /// Create a new clip hosting a text overlay or subtitle.
    pub fn new_text_overlay(
        id: ClipId,
        track_id: TrackId,
        text: impl Into<String>,
        start_frame: i64,
        duration_frames: i64,
        style: TextStyle,
    ) -> Result<Self, TimelineError> {
        let text_str = text.into();
        let label = format!("Text: {}", text_str.chars().take(20).collect::<String>());
        let mut clip = Self::new(
            id,
            AssetId::new(),
            track_id,
            label,
            start_frame,
            0,
            duration_frames,
        )?;
        clip.text_overlay = Some(TextOverlay::with_style(text_str, style));
        Ok(clip)
    }

    /// Derived duration of the clip in integer frames.
    pub const fn duration_frames(&self) -> i64 {
        self.source_out - self.source_in
    }

    /// Derived end frame on the timeline (exclusive).
    pub const fn end_frame(&self) -> i64 {
        self.start_frame + self.duration_frames()
    }

    /// Returns true if the given timeline frame falls within `[start_frame, end_frame)`.
    pub const fn contains_timeline_frame(&self, frame: i64) -> bool {
        self.start_frame <= frame && frame < self.end_frame()
    }

    /// Map a timeline frame to the corresponding source media frame.
    pub fn timeline_to_source_frame(&self, frame: i64) -> Option<i64> {
        if self.contains_timeline_frame(frame) {
            Some(self.source_in + (frame - self.start_frame))
        } else {
            None
        }
    }

    /// Map a source media frame to the corresponding timeline frame.
    pub fn source_to_timeline_frame(&self, source_frame: i64) -> Option<i64> {
        if source_frame >= self.source_in && source_frame < self.source_out {
            Some(self.start_frame + (source_frame - self.source_in))
        } else {
            None
        }
    }

    /// Set a new timeline start frame, keeping duration fixed.
    pub fn set_start_frame(&mut self, new_start_frame: i64) -> Result<(), TimelineError> {
        if new_start_frame < 0 {
            return Err(TimelineError::InvalidStartFrame(new_start_frame));
        }
        self.start_frame = new_start_frame;
        Ok(())
    }

    /// Trim in-point: changes `source_in` and shifts `start_frame` forward/backward
    /// so the clip's tail (`end_frame`) remains stationary.
    pub fn trim_in(&mut self, new_source_in: i64) -> Result<(), TimelineError> {
        if new_source_in < 0 || new_source_in >= self.source_out {
            return Err(TimelineError::InvalidTrim {
                source_in: new_source_in,
                source_out: self.source_out,
            });
        }

        let delta = new_source_in - self.source_in;
        let new_start_frame = self.start_frame + delta;
        if new_start_frame < 0 {
            return Err(TimelineError::InvalidStartFrame(new_start_frame));
        }

        self.source_in = new_source_in;
        self.start_frame = new_start_frame;
        Ok(())
    }

    /// Trim out-point: changes `source_out`, modifying clip duration while keeping `start_frame` fixed.
    pub fn trim_out(&mut self, new_source_out: i64) -> Result<(), TimelineError> {
        if new_source_out <= self.source_in {
            return Err(TimelineError::InvalidTrim {
                source_in: self.source_in,
                source_out: new_source_out,
            });
        }
        self.source_out = new_source_out;
        Ok(())
    }

    /// Split the clip at a timeline frame strictly within `(start_frame, end_frame)`.
    /// Returns the left segment (retaining current ID) and the right segment (assigned `new_clip_id`).
    pub fn split(
        &self,
        split_frame: i64,
        new_clip_id: ClipId,
    ) -> Result<(Self, Self), TimelineError> {
        if split_frame <= self.start_frame || split_frame >= self.end_frame() {
            return Err(TimelineError::InvalidSplitPosition {
                split_frame,
                start: self.start_frame,
                end: self.end_frame(),
            });
        }

        let split_offset = split_frame - self.start_frame;
        let mid_source = self.source_in + split_offset;

        let left = Self {
            id: self.id,
            asset_id: self.asset_id,
            track_id: self.track_id,
            name: self.name.clone(),
            start_frame: self.start_frame,
            source_in: self.source_in,
            source_out: mid_source,
            enabled: self.enabled,
            transform: self.transform,
            audio_params: self.audio_params,
            text_overlay: self.text_overlay.clone(),
        };

        let right = Self {
            id: new_clip_id,
            asset_id: self.asset_id,
            track_id: self.track_id,
            name: self.name.clone(),
            start_frame: split_frame,
            source_in: mid_source,
            source_out: self.source_out,
            enabled: self.enabled,
            transform: self.transform,
            audio_params: self.audio_params,
            text_overlay: self.text_overlay.clone(),
        };

        Ok((left, right))
    }

    /// Update spatial and visual transform attributes.
    pub fn set_transform(&mut self, transform: ClipTransform) {
        self.transform = transform.clamped();
    }

    /// Reset spatial transform back to identity.
    pub fn reset_transform(&mut self) {
        self.transform = ClipTransform::default();
    }

    /// Update audio volume and panning attributes.
    pub fn set_audio_params(&mut self, audio_params: ClipAudioParams) {
        self.audio_params = audio_params.clamped();
    }

    /// Reset audio parameters back to defaults (0.0 dB, 0 pan).
    pub fn reset_audio_params(&mut self) {
        self.audio_params = ClipAudioParams::default();
    }
}
