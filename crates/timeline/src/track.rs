//! Timeline track domain model holding ordered clips.

use serde::{Deserialize, Serialize};

use crate::clip::Clip;
use crate::error::TimelineError;
use crate::id::{ClipId, TrackId, TransitionId};
use crate::transition::Transition;

/// Classification of timeline track content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackType {
    /// Video track (hosts video clips and graphic overlays).
    Video,
    /// Audio track (hosts audio clips and extracted video audio).
    Audio,
}

/// A timeline track containing placed media clips.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    /// Unique identifier for this track.
    pub id: TrackId,
    /// Display label (e.g. "V1", "V2", "A1", "A2").
    pub name: String,
    /// Type of media clips hosted on this track.
    pub track_type: TrackType,
    /// Visual and compositing order (lower index = rendered above or listed earlier).
    pub order: usize,
    /// Whether audio from this track is muted.
    pub muted: bool,
    /// Whether this track is soloed during playback.
    pub soloed: bool,
    /// Whether this track is locked to prevent edits.
    pub locked: bool,
    /// Whether this track is visible in the preview monitor.
    pub visible: bool,
    /// Ordered list of clips placed on this track.
    pub clips: Vec<Clip>,
    /// Transitions placed on edit cuts between adjacent clips in this track.
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

impl Track {
    /// Create a new empty track.
    pub fn new(id: TrackId, name: impl Into<String>, track_type: TrackType, order: usize) -> Self {
        Self {
            id,
            name: name.into(),
            track_type,
            order,
            muted: false,
            soloed: false,
            locked: false,
            visible: true,
            clips: Vec::new(),
            transitions: Vec::new(),
        }
    }

    /// Derived duration of the track in frames (end frame of the latest clip).
    pub fn duration_frames(&self) -> i64 {
        self.clips.iter().map(Clip::end_frame).max().unwrap_or(0)
    }

    /// Find an immutable reference to a clip by ID.
    pub fn get_clip(&self, id: ClipId) -> Option<&Clip> {
        self.clips.iter().find(|c| c.id == id)
    }

    /// Find a mutable reference to a clip by ID.
    pub fn get_clip_mut(&mut self, id: ClipId) -> Option<&mut Clip> {
        self.clips.iter_mut().find(|c| c.id == id)
    }

    /// Check if a clip with the given ID exists on this track.
    pub fn contains_clip(&self, id: ClipId) -> bool {
        self.clips.iter().any(|c| c.id == id)
    }

    /// Add a clip to this track, maintaining start_frame sort order.
    pub fn add_clip(&mut self, clip: Clip) -> Result<(), TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked(self.id));
        }
        if self.contains_clip(clip.id) {
            return Err(TimelineError::DuplicateClipId(clip.id));
        }

        self.clips.push(clip);
        self.sort_clips();
        Ok(())
    }

    /// Remove a clip by ID from this track.
    pub fn remove_clip(&mut self, id: ClipId) -> Result<Clip, TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked(self.id));
        }
        let pos = self
            .clips
            .iter()
            .position(|c| c.id == id)
            .ok_or(TimelineError::ClipNotFound(id))?;

        // Clean up attached transitions
        self.transitions
            .retain(|t| t.from_clip_id != id && t.to_clip_id != id);

        Ok(self.clips.remove(pos))
    }

    /// Remove a clip and ripple-shift subsequent clips on this track backwards.
    pub fn ripple_remove_clip(&mut self, id: ClipId) -> Result<Clip, TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked(self.id));
        }
        let pos = self
            .clips
            .iter()
            .position(|c| c.id == id)
            .ok_or(TimelineError::ClipNotFound(id))?;

        // Clean up attached transitions
        self.transitions
            .retain(|t| t.from_clip_id != id && t.to_clip_id != id);

        let removed = self.clips.remove(pos);
        let duration = removed.duration_frames();
        let removed_end = removed.end_frame();

        for clip in &mut self.clips {
            if clip.start_frame >= removed_end {
                clip.start_frame = (clip.start_frame - duration).max(0);
            }
        }

        self.sort_clips();
        Ok(removed)
    }

    /// Add a transition between two adjacent clips on this track.
    pub fn add_transition(&mut self, transition: Transition) -> Result<(), TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked(self.id));
        }
        if self.transitions.iter().any(|t| t.id == transition.id) {
            return Err(TimelineError::DuplicateTransitionId(transition.id));
        }

        // Verify from_clip and to_clip exist and are adjacent
        let from_clip = self
            .get_clip(transition.from_clip_id)
            .ok_or(TimelineError::ClipNotFound(transition.from_clip_id))?;
        let to_clip = self
            .get_clip(transition.to_clip_id)
            .ok_or(TimelineError::ClipNotFound(transition.to_clip_id))?;

        if from_clip.end_frame() != to_clip.start_frame {
            return Err(TimelineError::InvalidTransitionClips(format!(
                "Clips '{}' and '{}' are not adjacent (gap from {} to {})",
                from_clip.id,
                to_clip.id,
                from_clip.end_frame(),
                to_clip.start_frame
            )));
        }

        if transition.cut_frame != from_clip.end_frame() {
            return Err(TimelineError::InvalidTransitionClips(format!(
                "Cut frame {} does not match boundary {}",
                transition.cut_frame,
                from_clip.end_frame()
            )));
        }

        self.transitions.push(transition);
        Ok(())
    }

    /// Remove a transition by ID from this track.
    pub fn remove_transition(&mut self, id: TransitionId) -> Result<Transition, TimelineError> {
        if self.locked {
            return Err(TimelineError::TrackLocked(self.id));
        }
        let pos = self
            .transitions
            .iter()
            .position(|t| t.id == id)
            .ok_or(TimelineError::TransitionNotFound(id))?;

        Ok(self.transitions.remove(pos))
    }

    /// Find an immutable reference to a transition by ID.
    pub fn get_transition(&self, id: TransitionId) -> Option<&Transition> {
        self.transitions.iter().find(|t| t.id == id)
    }

    /// Return the clip at a specific timeline frame, if one exists.
    pub fn clip_at_frame(&self, frame: i64) -> Option<&Clip> {
        self.clips.iter().find(|c| c.contains_timeline_frame(frame))
    }

    /// Sort clips by starting frame.
    pub fn sort_clips(&mut self) {
        self.clips.sort_by_key(|c| c.start_frame);
    }
}
