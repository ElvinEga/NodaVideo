//! Selection state tracking for timeline tracks, clips, and transitions.

use nodavideo_timeline::{ClipId, TrackId, TransitionId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Current selection state in the editor timeline.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionState {
    /// Selected clip IDs.
    pub selected_clips: HashSet<ClipId>,
    /// Active/focused track ID.
    pub selected_track: Option<TrackId>,
    /// Selected transition ID, if any.
    pub selected_transition: Option<TransitionId>,
}

impl SelectionState {
    /// Create a new empty selection state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Select a single clip, clearing any previous clip selection.
    pub fn select_clip(&mut self, clip_id: ClipId) {
        self.selected_clips.clear();
        self.selected_clips.insert(clip_id);
        self.selected_transition = None;
    }

    /// Toggle selection state of a clip (multi-select / Shift-click).
    pub fn toggle_clip_selection(&mut self, clip_id: ClipId) {
        if self.selected_clips.contains(&clip_id) {
            self.selected_clips.remove(&clip_id);
        } else {
            self.selected_clips.insert(clip_id);
        }
        self.selected_transition = None;
    }

    /// Add a collection of clips to selection.
    pub fn select_multiple_clips(&mut self, clips: impl IntoIterator<Item = ClipId>) {
        self.selected_clips.clear();
        self.selected_clips.extend(clips);
        self.selected_transition = None;
    }

    /// Select a track.
    pub fn select_track(&mut self, track_id: Option<TrackId>) {
        self.selected_track = track_id;
    }

    /// Select an edit cut transition.
    pub fn select_transition(&mut self, transition_id: Option<TransitionId>) {
        self.selected_transition = transition_id;
        if transition_id.is_some() {
            self.selected_clips.clear();
        }
    }

    /// Clear all selections.
    pub fn clear(&mut self) {
        self.selected_clips.clear();
        self.selected_track = None;
        self.selected_transition = None;
    }

    /// Whether any item is selected.
    #[must_use]
    pub fn has_selection(&self) -> bool {
        !self.selected_clips.is_empty()
            || self.selected_track.is_some()
            || self.selected_transition.is_some()
    }

    /// Whether a specific clip is selected.
    #[must_use]
    pub fn is_clip_selected(&self, clip_id: &ClipId) -> bool {
        self.selected_clips.contains(clip_id)
    }

    /// Return primary selected clip ID if at least one clip is selected.
    #[must_use]
    pub fn primary_clip(&self) -> Option<ClipId> {
        self.selected_clips.iter().copied().next()
    }
}
