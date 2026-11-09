//! Timeline clip domain model representing placed media segments.

use nodavideo_project::AssetId;
use serde::{Deserialize, Serialize};

use crate::error::TimelineError;
use crate::id::{ClipId, TrackId};

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
        })
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
        };

        Ok((left, right))
    }
}
