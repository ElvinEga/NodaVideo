//! Timeline selection and playhead navigation state.

use std::collections::HashSet;

use crate::id::ClipId;

/// State tracking selected timeline clips, playhead frame, and in/out markers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TimelineSelection {
    /// Set of currently selected clip IDs.
    pub selected_clips: HashSet<ClipId>,
    /// Current playhead position in frames (>= 0).
    pub playhead_frame: i64,
    /// Optional sequence in-point marker in frames.
    pub in_point: Option<i64>,
    /// Optional sequence out-point marker in frames.
    pub out_point: Option<i64>,
}

impl TimelineSelection {
    /// Create a new selection with playhead at frame 0 and no selected clips.
    pub fn new() -> Self {
        Self::default()
    }

    /// Select a single clip, clearing any previous selection.
    pub fn select_single(&mut self, clip_id: ClipId) {
        self.selected_clips.clear();
        self.selected_clips.insert(clip_id);
    }

    /// Toggle selection state for a clip (for Shift+Click or Cmd+Click multi-select).
    pub fn toggle_select(&mut self, clip_id: ClipId) {
        if self.selected_clips.contains(&clip_id) {
            self.selected_clips.remove(&clip_id);
        } else {
            self.selected_clips.insert(clip_id);
        }
    }

    /// Remove a clip from selection.
    pub fn deselect(&mut self, clip_id: ClipId) {
        self.selected_clips.remove(&clip_id);
    }

    /// Clear all clip selections.
    pub fn clear_selection(&mut self) {
        self.selected_clips.clear();
    }

    /// Check if a specific clip is selected.
    pub fn is_selected(&self, clip_id: ClipId) -> bool {
        self.selected_clips.contains(&clip_id)
    }

    /// Get the primary selected clip (first item if single/multi-selected).
    pub fn primary_selected(&self) -> Option<ClipId> {
        self.selected_clips.iter().copied().next()
    }

    /// Set playhead frame position, clamping at 0.
    pub fn set_playhead(&mut self, frame: i64) {
        self.playhead_frame = frame.max(0);
    }

    /// Set sequence in-point marker.
    pub fn set_in_point(&mut self, frame: Option<i64>) {
        self.in_point = frame.map(|f| f.max(0));
    }

    /// Set sequence out-point marker.
    pub fn set_out_point(&mut self, frame: Option<i64>) {
        self.out_point = frame.map(|f| f.max(0));
    }
}
