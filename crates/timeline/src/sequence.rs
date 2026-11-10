//! Editable sequence domain model coordinating multitrack composition.

use nodavideo_project::{FrameRate, ProjectDocument, TimeRational};
use serde::{Deserialize, Serialize};

use crate::clip::Clip;
use crate::error::TimelineError;
use crate::id::{ClipId, SequenceId, TrackId, TransitionId};
use crate::track::{Track, TrackType};
use crate::transition::Transition;

/// Editable video composition sequence containing multitrack lanes and timing settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sequence {
    /// Unique sequence identifier.
    pub id: SequenceId,
    /// Human-readable sequence name.
    pub name: String,
    /// Rational timeline frame rate.
    pub fps: FrameRate,
    /// Canvas frame width in pixels.
    pub width: u32,
    /// Canvas frame height in pixels.
    pub height: u32,
    /// Internal timebase units per second (default 1_000_000).
    pub timebase: u32,
    /// Ordered collection of timeline tracks.
    pub tracks: Vec<Track>,
}

impl Default for Sequence {
    fn default() -> Self {
        Self::default_1080p24()
    }
}

impl Sequence {
    /// Create a standard 1080p 24fps sequence with initial V2, V1, A1, A2 tracks.
    pub fn default_1080p24() -> Self {
        let v2 = Track::new(TrackId::new(), "V2", TrackType::Video, 0);
        let v1 = Track::new(TrackId::new(), "V1", TrackType::Video, 1);
        let a1 = Track::new(TrackId::new(), "A1", TrackType::Audio, 2);
        let a2 = Track::new(TrackId::new(), "A2", TrackType::Audio, 3);

        Self {
            id: SequenceId::new(),
            name: "Main Sequence".into(),
            fps: FrameRate::FPS_24,
            width: 1920,
            height: 1080,
            timebase: 1_000_000,
            tracks: vec![v2, v1, a1, a2],
        }
    }

    /// Total timeline duration in frames (furthest clip end across all tracks).
    pub fn duration_frames(&self) -> i64 {
        self.tracks
            .iter()
            .map(Track::duration_frames)
            .max()
            .unwrap_or(0)
    }

    /// Format any frame number into SMPTE timecode at this sequence's FPS.
    pub fn frame_to_timecode(&self, frame: i64) -> String {
        let rational = TimeRational::from_frames(frame, self.fps);
        rational.to_timecode(self.fps)
    }

    /// Sequence duration formatted as SMPTE timecode.
    pub fn duration_timecode(&self) -> String {
        self.frame_to_timecode(self.duration_frames())
    }

    /// Add a new track to the sequence.
    pub fn add_track(&mut self, track: Track) -> Result<(), TimelineError> {
        if self.tracks.iter().any(|t| t.id == track.id) {
            return Err(TimelineError::DuplicateTrackId(track.id));
        }
        self.tracks.push(track);
        Ok(())
    }

    /// Remove a track by ID.
    pub fn remove_track(&mut self, track_id: TrackId) -> Result<Track, TimelineError> {
        let pos = self
            .tracks
            .iter()
            .position(|t| t.id == track_id)
            .ok_or(TimelineError::TrackNotFound(track_id))?;
        Ok(self.tracks.remove(pos))
    }

    /// Find an immutable reference to a track by ID.
    pub fn get_track(&self, track_id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == track_id)
    }

    /// Find a mutable reference to a track by ID.
    pub fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == track_id)
    }

    /// Locate which track contains the given clip ID.
    pub fn find_track_for_clip(&self, clip_id: ClipId) -> Option<&Track> {
        self.tracks.iter().find(|t| t.contains_clip(clip_id))
    }

    /// Locate which track contains the given clip ID (mutable).
    pub fn find_track_for_clip_mut(&mut self, clip_id: ClipId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.contains_clip(clip_id))
    }

    /// Retrieve clip reference by clip ID.
    pub fn get_clip(&self, clip_id: ClipId) -> Option<&Clip> {
        for track in &self.tracks {
            if let Some(clip) = track.get_clip(clip_id) {
                return Some(clip);
            }
        }
        None
    }

    /// Retrieve mutable clip reference by clip ID.
    pub fn get_clip_mut(&mut self, clip_id: ClipId) -> Option<&mut Clip> {
        for track in &mut self.tracks {
            if let Some(clip) = track.get_clip_mut(clip_id) {
                return Some(clip);
            }
        }
        None
    }

    /// Add a clip to a specific track.
    pub fn add_clip(&mut self, track_id: TrackId, clip: Clip) -> Result<(), TimelineError> {
        let track = self
            .get_track_mut(track_id)
            .ok_or(TimelineError::TrackNotFound(track_id))?;
        track.add_clip(clip)
    }

    /// Move a clip to a (possibly different) track and a new start frame.
    pub fn move_clip(
        &mut self,
        clip_id: ClipId,
        new_track_id: TrackId,
        new_start_frame: i64,
    ) -> Result<(), TimelineError> {
        if new_start_frame < 0 {
            return Err(TimelineError::InvalidStartFrame(new_start_frame));
        }

        // Verify target track exists and is not locked
        let target_track = self
            .get_track(new_track_id)
            .ok_or(TimelineError::TrackNotFound(new_track_id))?;
        if target_track.locked {
            return Err(TimelineError::TrackLocked(new_track_id));
        }
        let target_type = target_track.track_type;

        // Locate current source track and remove clip
        let source_track = self
            .find_track_for_clip_mut(clip_id)
            .ok_or(TimelineError::ClipNotFound(clip_id))?;

        if source_track.track_type != target_type {
            return Err(TimelineError::TrackTypeMismatch {
                expected: format!("{target_type:?}"),
                found: format!("{:?}", source_track.track_type),
            });
        }

        let mut clip = source_track.remove_clip(clip_id)?;
        clip.track_id = new_track_id;
        clip.start_frame = new_start_frame;

        // Insert into target track
        let dest_track = self
            .get_track_mut(new_track_id)
            .expect("target track already verified");
        dest_track.add_clip(clip)?;
        Ok(())
    }

    /// Trim in-point of a clip.
    pub fn trim_clip_in(
        &mut self,
        clip_id: ClipId,
        new_source_in: i64,
    ) -> Result<(), TimelineError> {
        let track = self
            .find_track_for_clip_mut(clip_id)
            .ok_or(TimelineError::ClipNotFound(clip_id))?;
        if track.locked {
            return Err(TimelineError::TrackLocked(track.id));
        }
        let clip = track.get_clip_mut(clip_id).expect("clip verified");
        clip.trim_in(new_source_in)?;
        track.sort_clips();
        Ok(())
    }

    /// Trim out-point of a clip.
    pub fn trim_clip_out(
        &mut self,
        clip_id: ClipId,
        new_source_out: i64,
    ) -> Result<(), TimelineError> {
        let track = self
            .find_track_for_clip_mut(clip_id)
            .ok_or(TimelineError::ClipNotFound(clip_id))?;
        if track.locked {
            return Err(TimelineError::TrackLocked(track.id));
        }
        let clip = track.get_clip_mut(clip_id).expect("clip verified");
        clip.trim_out(new_source_out)?;
        Ok(())
    }

    /// Split a clip at a timeline frame into two adjacent clips.
    pub fn split_clip(
        &mut self,
        clip_id: ClipId,
        split_frame: i64,
        new_clip_id: ClipId,
    ) -> Result<(Clip, Clip), TimelineError> {
        let track = self
            .find_track_for_clip_mut(clip_id)
            .ok_or(TimelineError::ClipNotFound(clip_id))?;
        if track.locked {
            return Err(TimelineError::TrackLocked(track.id));
        }

        let clip = track.get_clip(clip_id).expect("clip verified");
        let (left, right) = clip.split(split_frame, new_clip_id)?;

        // Update existing clip to left, and insert right
        let clip_mut = track.get_clip_mut(clip_id).expect("clip verified");
        *clip_mut = left.clone();
        track.add_clip(right.clone())?;

        Ok((left, right))
    }

    /// Delete a clip from its track.
    pub fn delete_clip(&mut self, clip_id: ClipId) -> Result<Clip, TimelineError> {
        let track = self
            .find_track_for_clip_mut(clip_id)
            .ok_or(TimelineError::ClipNotFound(clip_id))?;
        track.remove_clip(clip_id)
    }

    /// Delete a clip and ripple subsequent clips on that track backwards.
    pub fn ripple_delete_clip(&mut self, clip_id: ClipId) -> Result<Clip, TimelineError> {
        let track = self
            .find_track_for_clip_mut(clip_id)
            .ok_or(TimelineError::ClipNotFound(clip_id))?;
        track.ripple_remove_clip(clip_id)
    }

    /// Add a transition to a specific track.
    pub fn add_transition(
        &mut self,
        track_id: TrackId,
        transition: Transition,
    ) -> Result<(), TimelineError> {
        let track = self
            .get_track_mut(track_id)
            .ok_or(TimelineError::TrackNotFound(track_id))?;
        track.add_transition(transition)
    }

    /// Remove a transition by ID from its track.
    pub fn remove_transition(
        &mut self,
        transition_id: TransitionId,
    ) -> Result<(TrackId, Transition), TimelineError> {
        let track = self
            .find_track_for_transition_mut(transition_id)
            .ok_or(TimelineError::TransitionNotFound(transition_id))?;
        let track_id = track.id;
        let trans = track.remove_transition(transition_id)?;
        Ok((track_id, trans))
    }

    /// Find the track containing a specific transition.
    pub fn find_track_for_transition(&self, transition_id: TransitionId) -> Option<&Track> {
        self.tracks
            .iter()
            .find(|t| t.transitions.iter().any(|tr| tr.id == transition_id))
    }

    /// Find mutable track containing a specific transition.
    pub fn find_track_for_transition_mut(
        &mut self,
        transition_id: TransitionId,
    ) -> Option<&mut Track> {
        self.tracks
            .iter_mut()
            .find(|t| t.transitions.iter().any(|tr| tr.id == transition_id))
    }

    /// Get a transition by ID across all tracks.
    pub fn get_transition(&self, transition_id: TransitionId) -> Option<&Transition> {
        self.tracks
            .iter()
            .find_map(|t| t.get_transition(transition_id))
    }

    /// Attach this sequence to a project document as its timeline payload.
    pub fn attach_to_project(&self, project: &mut ProjectDocument) -> Result<(), TimelineError> {
        project.timeline = Some(serde_json::to_value(self)?);
        Ok(())
    }

    /// Extract and deserialize the sequence from a project document, if present.
    pub fn extract_from_project(project: &ProjectDocument) -> Result<Option<Self>, TimelineError> {
        match &project.timeline {
            Some(val) => Ok(Some(serde_json::from_value(val.clone())?)),
            None => Ok(None),
        }
    }
}
