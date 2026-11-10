//! Timeline transition domain model representing cuts and dissolves between adjacent clips.

use serde::{Deserialize, Serialize};

use crate::error::TimelineError;
use crate::id::{ClipId, TrackId, TransitionId};

/// Types of visual/audio transitions between clips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    /// Smooth crossfade between outgoing and incoming content.
    #[default]
    Crossfade,
    /// Smooth dissolve blending layers.
    Dissolve,
    /// Dip / fade through black.
    FadeToBlack,
    /// Dip / fade through white.
    DipToWhite,
    /// Linear wipe moving from left to right.
    WipeLeft,
    /// Linear wipe moving from right to left.
    WipeRight,
}

/// Placement alignment relative to the cut point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionAlignment {
    /// Centered equally across the cut point (e.g. 12 frames before, 12 frames after for a 24-frame transition).
    #[default]
    Centered,
    /// Starts at the cut point and extends into the incoming clip.
    StartAtCut,
    /// Ends at the cut point, extending backwards into the outgoing clip.
    EndAtCut,
}

/// A transition placed between two adjacent clips on a track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    /// Unique identifier for this transition.
    pub id: TransitionId,
    /// Type of transition effect (crossfade, fade to black, wipe, etc.).
    pub transition_type: TransitionType,
    /// Alignment relative to the edit point.
    pub alignment: TransitionAlignment,
    /// Outgoing clip leading into the transition.
    pub from_clip_id: ClipId,
    /// Incoming clip emerging from the transition.
    pub to_clip_id: ClipId,
    /// Containing track.
    pub track_id: TrackId,
    /// Total duration of the transition in frames.
    pub duration_frames: i64,
    /// Timeline frame where the cut point exists (outgoing clip end / incoming clip start).
    pub cut_frame: i64,
}

impl Transition {
    /// Create a new transition, validating duration and cut frame.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TransitionId,
        transition_type: TransitionType,
        alignment: TransitionAlignment,
        from_clip_id: ClipId,
        to_clip_id: ClipId,
        track_id: TrackId,
        cut_frame: i64,
        duration_frames: i64,
    ) -> Result<Self, TimelineError> {
        if duration_frames <= 0 {
            return Err(TimelineError::InvalidDuration(duration_frames));
        }
        if cut_frame < 0 {
            return Err(TimelineError::InvalidStartFrame(cut_frame));
        }

        Ok(Self {
            id,
            transition_type,
            alignment,
            from_clip_id,
            to_clip_id,
            track_id,
            duration_frames,
            cut_frame,
        })
    }

    /// Calculate the timeline start frame of the transition based on alignment.
    #[must_use]
    pub fn start_frame(&self) -> i64 {
        match self.alignment {
            TransitionAlignment::Centered => self.cut_frame - (self.duration_frames / 2),
            TransitionAlignment::StartAtCut => self.cut_frame,
            TransitionAlignment::EndAtCut => self.cut_frame - self.duration_frames,
        }
    }

    /// Calculate the timeline end frame of the transition (exclusive).
    #[must_use]
    pub fn end_frame(&self) -> i64 {
        self.start_frame() + self.duration_frames
    }

    /// Check if the transition spans across the given timeline frame.
    #[must_use]
    pub fn contains_timeline_frame(&self, frame: i64) -> bool {
        let start = self.start_frame();
        let end = self.end_frame();
        frame >= start && frame < end
    }
}
