//! Canonical EditorCommand domain representation and execution.

use nodavideo_timeline::{Clip, ClipId, Sequence, TimelineError, Track, TrackId};
use serde::{Deserialize, Serialize};

use crate::error::CommandError;

/// Parameters for adding a new clip to a track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddClip {
    pub track_id: TrackId,
    pub clip: Clip,
}

/// Parameters for moving a clip across frames or tracks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveClip {
    pub clip_id: ClipId,
    pub from_track: TrackId,
    pub to_track: TrackId,
    pub old_start_frame: i64,
    pub new_start_frame: i64,
}

/// Parameters for trimming a clip's in or out points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrimClip {
    pub clip_id: ClipId,
    pub track_id: TrackId,
    pub old_start_frame: i64,
    pub new_start_frame: i64,
    pub old_source_in: i64,
    pub new_source_in: i64,
    pub old_source_out: i64,
    pub new_source_out: i64,
}

/// Parameters for splitting a clip at a timeline frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitClip {
    pub original_clip_id: ClipId,
    pub track_id: TrackId,
    pub split_frame: i64,
    pub new_clip_id: ClipId,
    pub original_source_out: i64,
}

/// Parameters for re-merging previously split clips during undo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsplitClip {
    pub original_clip_id: ClipId,
    pub track_id: TrackId,
    pub created_clip_id: ClipId,
    pub original_source_out: i64,
}

/// Parameters for deleting a clip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteClip {
    pub track_id: TrackId,
    pub clip: Clip,
}

/// Parameters for adding a new track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddTrack {
    pub track: Track,
}

/// Parameters for deleting a track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteTrack {
    pub track: Track,
}

/// Parameters for toggling mute on a track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleMute {
    pub track_id: TrackId,
}

/// Parameters for toggling solo on a track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleSolo {
    pub track_id: TrackId,
}

/// Parameters for toggling lock on a track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleLock {
    pub track_id: TrackId,
}

/// Parameters for an atomic batch of commands executing as a single transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchCommand {
    pub name: String,
    pub commands: Vec<EditorCommand>,
}

/// Canonical editor commands governing all project and timeline mutations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorCommand {
    AddClip(AddClip),
    MoveClip(MoveClip),
    TrimClip(TrimClip),
    SplitClip(SplitClip),
    UnsplitClip(UnsplitClip),
    DeleteClip(DeleteClip),
    AddTrack(AddTrack),
    DeleteTrack(DeleteTrack),
    ToggleMute(ToggleMute),
    ToggleSolo(ToggleSolo),
    ToggleLock(ToggleLock),
    Batch(BatchCommand),
}

impl EditorCommand {
    /// Human-readable description of this command for undo/redo UI menus and history lists.
    pub fn description(&self) -> String {
        match self {
            Self::AddClip(cmd) => format!("Add Clip '{}'", cmd.clip.name),
            Self::MoveClip(cmd) => format!("Move Clip to Frame {}", cmd.new_start_frame),
            Self::TrimClip(_) => "Trim Clip".to_string(),
            Self::SplitClip(cmd) => format!("Split Clip at Frame {}", cmd.split_frame),
            Self::UnsplitClip(_) => "Undo Split Clip".to_string(),
            Self::DeleteClip(cmd) => format!("Delete Clip '{}'", cmd.clip.name),
            Self::AddTrack(cmd) => format!("Add Track '{}'", cmd.track.name),
            Self::DeleteTrack(cmd) => format!("Delete Track '{}'", cmd.track.name),
            Self::ToggleMute(_) => "Toggle Track Mute".to_string(),
            Self::ToggleSolo(_) => "Toggle Track Solo".to_string(),
            Self::ToggleLock(_) => "Toggle Track Lock".to_string(),
            Self::Batch(cmd) => cmd.name.clone(),
        }
    }

    /// Compute the exact inverse command required to revert this action.
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            Self::AddClip(cmd) => Self::DeleteClip(DeleteClip {
                track_id: cmd.track_id,
                clip: cmd.clip.clone(),
            }),
            Self::DeleteClip(cmd) => Self::AddClip(AddClip {
                track_id: cmd.track_id,
                clip: cmd.clip.clone(),
            }),
            Self::MoveClip(cmd) => Self::MoveClip(MoveClip {
                clip_id: cmd.clip_id,
                from_track: cmd.to_track,
                to_track: cmd.from_track,
                old_start_frame: cmd.new_start_frame,
                new_start_frame: cmd.old_start_frame,
            }),
            Self::TrimClip(cmd) => Self::TrimClip(TrimClip {
                clip_id: cmd.clip_id,
                track_id: cmd.track_id,
                old_start_frame: cmd.new_start_frame,
                new_start_frame: cmd.old_start_frame,
                old_source_in: cmd.new_source_in,
                new_source_in: cmd.old_source_in,
                old_source_out: cmd.new_source_out,
                new_source_out: cmd.old_source_out,
            }),
            Self::SplitClip(cmd) => Self::UnsplitClip(UnsplitClip {
                original_clip_id: cmd.original_clip_id,
                track_id: cmd.track_id,
                created_clip_id: cmd.new_clip_id,
                original_source_out: cmd.original_source_out,
            }),
            Self::UnsplitClip(cmd) => Self::SplitClip(SplitClip {
                original_clip_id: cmd.original_clip_id,
                track_id: cmd.track_id,
                split_frame: 0, // Not used in inverse
                new_clip_id: cmd.created_clip_id,
                original_source_out: cmd.original_source_out,
            }),
            Self::AddTrack(cmd) => Self::DeleteTrack(DeleteTrack {
                track: cmd.track.clone(),
            }),
            Self::DeleteTrack(cmd) => Self::AddTrack(AddTrack {
                track: cmd.track.clone(),
            }),
            Self::ToggleMute(cmd) => Self::ToggleMute(cmd.clone()),
            Self::ToggleSolo(cmd) => Self::ToggleSolo(cmd.clone()),
            Self::ToggleLock(cmd) => Self::ToggleLock(cmd.clone()),
            Self::Batch(cmd) => {
                let inverse_commands = cmd.commands.iter().rev().map(|c| c.inverse()).collect();
                Self::Batch(BatchCommand {
                    name: format!("Undo {}", cmd.name),
                    commands: inverse_commands,
                })
            }
        }
    }

    /// Execute the command against the sequence state deterministically.
    pub fn apply(&self, sequence: &mut Sequence) -> Result<(), CommandError> {
        match self {
            Self::AddClip(cmd) => {
                sequence.add_clip(cmd.track_id, cmd.clip.clone())?;
            }
            Self::DeleteClip(cmd) => {
                sequence.delete_clip(cmd.clip.id)?;
            }
            Self::MoveClip(cmd) => {
                sequence.move_clip(cmd.clip_id, cmd.to_track, cmd.new_start_frame)?;
            }
            Self::TrimClip(cmd) => {
                let track = sequence
                    .find_track_for_clip_mut(cmd.clip_id)
                    .ok_or(TimelineError::ClipNotFound(cmd.clip_id))?;
                let clip = track
                    .get_clip_mut(cmd.clip_id)
                    .ok_or(TimelineError::ClipNotFound(cmd.clip_id))?;
                clip.start_frame = cmd.new_start_frame;
                clip.source_in = cmd.new_source_in;
                clip.source_out = cmd.new_source_out;
                track.sort_clips();
            }
            Self::SplitClip(cmd) => {
                sequence.split_clip(cmd.original_clip_id, cmd.split_frame, cmd.new_clip_id)?;
            }
            Self::UnsplitClip(cmd) => {
                let track = sequence
                    .get_track_mut(cmd.track_id)
                    .ok_or(TimelineError::TrackNotFound(cmd.track_id))?;
                // Remove the right segment
                track.remove_clip(cmd.created_clip_id)?;
                // Restore the original clip's source_out
                let original = track
                    .get_clip_mut(cmd.original_clip_id)
                    .ok_or(TimelineError::ClipNotFound(cmd.original_clip_id))?;
                original.source_out = cmd.original_source_out;
            }
            Self::AddTrack(cmd) => {
                sequence.add_track(cmd.track.clone())?;
            }
            Self::DeleteTrack(cmd) => {
                sequence.remove_track(cmd.track.id)?;
            }
            Self::ToggleMute(cmd) => {
                let track = sequence
                    .get_track_mut(cmd.track_id)
                    .ok_or(TimelineError::TrackNotFound(cmd.track_id))?;
                track.muted = !track.muted;
            }
            Self::ToggleSolo(cmd) => {
                let track = sequence
                    .get_track_mut(cmd.track_id)
                    .ok_or(TimelineError::TrackNotFound(cmd.track_id))?;
                track.soloed = !track.soloed;
            }
            Self::ToggleLock(cmd) => {
                let track = sequence
                    .get_track_mut(cmd.track_id)
                    .ok_or(TimelineError::TrackNotFound(cmd.track_id))?;
                track.locked = !track.locked;
            }
            Self::Batch(cmd) => {
                let mut applied_inverses: Vec<Self> = Vec::new();
                for c in &cmd.commands {
                    if let Err(err) = c.apply(sequence) {
                        for inv in applied_inverses.into_iter().rev() {
                            let _ = inv.apply(sequence);
                        }
                        return Err(err);
                    }
                    applied_inverses.push(c.inverse());
                }
            }
        }
        Ok(())
    }
}
