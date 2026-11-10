//! NodaVideo unified command system and undo/redo architecture.
//!
//! As defined in `AGENTS.md`, every state mutation — human or AI — goes through
//! `EditorCommand`. Command execution is deterministic, reversible, and transactional.

pub mod command;
pub mod error;
pub mod history;
pub mod staged;

pub use command::{
    AddClip, AddComposition, AddTrack, BatchCommand, DeleteClip, DeleteTrack, EditorCommand,
    MoveClip, SplitClip, ToggleLock, ToggleMute, ToggleSolo, TrimClip, UnsplitClip,
};
pub use error::CommandError;
pub use history::{CommandHistory, HistoryEntry};
pub use staged::{ChangesetDiff, ChangesetStatus, StagedChangeset};

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_project::AssetId;
    use nodavideo_timeline::{Clip, ClipId, Sequence};

    #[test]
    fn test_add_and_delete_clip_undo_redo() {
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let track_id = sequence.tracks[1].id;
        let asset_id = AssetId::new();
        let clip_id = ClipId::new();

        let clip = Clip::new(clip_id, asset_id, track_id, "Clip 1", 0, 0, 100).unwrap();

        // 1. Execute AddClip
        let add_cmd = EditorCommand::AddClip(AddClip { track_id, clip });
        history.execute(add_cmd, &mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips.len(), 1);
        assert_eq!(sequence.duration_frames(), 100);
        assert!(history.can_undo());
        assert!(!history.can_redo());

        // 2. Undo AddClip -> clip is removed
        let undone = history.undo(&mut sequence).unwrap();
        assert_eq!(undone, Some("Add Clip 'Clip 1'".to_string()));
        assert_eq!(sequence.tracks[1].clips.len(), 0);
        assert_eq!(sequence.duration_frames(), 0);
        assert!(!history.can_undo());
        assert!(history.can_redo());

        // 3. Redo AddClip -> clip is restored
        let redone = history.redo(&mut sequence).unwrap();
        assert_eq!(redone, Some("Add Clip 'Clip 1'".to_string()));
        assert_eq!(sequence.tracks[1].clips.len(), 1);
        assert_eq!(sequence.duration_frames(), 100);
    }

    #[test]
    fn test_move_clip_undo_redo() {
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let v1_id = sequence.tracks[1].id;
        let v2_id = sequence.tracks[0].id;
        let clip_id = ClipId::new();

        let clip = Clip::new(clip_id, AssetId::new(), v1_id, "Clip", 0, 0, 50).unwrap();
        sequence.add_clip(v1_id, clip).unwrap();

        // Move to V2 at frame 40
        let move_cmd = EditorCommand::MoveClip(MoveClip {
            clip_id,
            from_track: v1_id,
            to_track: v2_id,
            old_start_frame: 0,
            new_start_frame: 40,
        });
        history.execute(move_cmd, &mut sequence).unwrap();
        assert_eq!(sequence.tracks[0].clips.len(), 1);
        assert_eq!(sequence.tracks[1].clips.len(), 0);
        assert_eq!(sequence.tracks[0].clips[0].start_frame, 40);

        // Undo move
        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[0].clips.len(), 0);
        assert_eq!(sequence.tracks[1].clips.len(), 1);
        assert_eq!(sequence.tracks[1].clips[0].start_frame, 0);

        // Redo move
        history.redo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[0].clips.len(), 1);
        assert_eq!(sequence.tracks[0].clips[0].start_frame, 40);
    }

    #[test]
    fn test_split_clip_undo_redo() {
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let v1_id = sequence.tracks[1].id;
        let original_clip_id = ClipId::new();
        let new_clip_id = ClipId::new();

        let clip = Clip::new(original_clip_id, AssetId::new(), v1_id, "Clip", 0, 0, 100).unwrap();
        sequence.add_clip(v1_id, clip).unwrap();

        // Split at frame 40
        let split_cmd = EditorCommand::SplitClip(SplitClip {
            original_clip_id,
            track_id: v1_id,
            split_frame: 40,
            new_clip_id,
            original_source_out: 100,
        });
        history.execute(split_cmd, &mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips.len(), 2);
        assert_eq!(sequence.tracks[1].clips[0].duration_frames(), 40);
        assert_eq!(sequence.tracks[1].clips[1].duration_frames(), 60);

        // Undo split: re-merges back to single clip with duration 100
        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips.len(), 1);
        assert_eq!(sequence.tracks[1].clips[0].id, original_clip_id);
        assert_eq!(sequence.tracks[1].clips[0].duration_frames(), 100);

        // Redo split
        history.redo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips.len(), 2);
    }

    #[test]
    fn test_toggle_track_properties_undo() {
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let v1_id = sequence.tracks[1].id;

        assert!(!sequence.tracks[1].muted);
        assert!(!sequence.tracks[1].locked);

        // Mute
        let mute_cmd = EditorCommand::ToggleMute(ToggleMute { track_id: v1_id });
        history.execute(mute_cmd, &mut sequence).unwrap();
        assert!(sequence.tracks[1].muted);

        // Undo mute
        history.undo(&mut sequence).unwrap();
        assert!(!sequence.tracks[1].muted);

        // Lock
        let lock_cmd = EditorCommand::ToggleLock(ToggleLock { track_id: v1_id });
        history.execute(lock_cmd, &mut sequence).unwrap();
        assert!(sequence.tracks[1].locked);

        // Undo lock
        history.undo(&mut sequence).unwrap();
        assert!(!sequence.tracks[1].locked);
    }

    #[test]
    fn test_staged_changeset_speculative_and_commit() {
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let track_id = sequence.tracks[1].id;
        let asset_id = AssetId::new();

        let clip1 = Clip::new(ClipId::new(), asset_id, track_id, "Clip 1", 0, 0, 50).unwrap();
        let clip2 = Clip::new(ClipId::new(), asset_id, track_id, "Clip 2", 50, 0, 50).unwrap();

        let mut staged = StagedChangeset::new(
            "Smart Cuts",
            "Auto-generated sequence arrangement with 2 clips",
        )
        .with_skill("smart-cuts");

        staged.add_command(EditorCommand::AddClip(AddClip {
            track_id,
            clip: clip1,
        }));
        staged.add_command(EditorCommand::AddClip(AddClip {
            track_id,
            clip: clip2,
        }));

        // 1. Speculative preview should reflect changes without altering the original sequence
        let diff = staged.compute_diff(&sequence).unwrap();
        assert_eq!(diff.original_clip_count, 0);
        assert_eq!(diff.staged_clip_count, 2);
        assert_eq!(diff.original_duration_frames, 0);
        assert_eq!(diff.staged_duration_frames, 100);
        assert_eq!(diff.commands_count, 2);
        assert_eq!(diff.tracks_affected, 1);
        assert_eq!(sequence.tracks[1].clips.len(), 0);

        // 2. Commit the staged proposal into active sequence through history
        let committed = history.commit_staged(staged, &mut sequence).unwrap();
        assert_eq!(committed.status, ChangesetStatus::Committed);
        assert_eq!(sequence.tracks[1].clips.len(), 2);
        assert_eq!(sequence.duration_frames(), 100);

        // 3. Unified undo reverts entire batch in one step
        assert!(history.can_undo());
        let undone = history.undo(&mut sequence).unwrap();
        assert_eq!(undone, Some("Smart Cuts".to_string()));
        assert_eq!(sequence.tracks[1].clips.len(), 0);
        assert_eq!(sequence.duration_frames(), 0);

        // 4. Redo restores entire batch in one step
        let redone = history.redo(&mut sequence).unwrap();
        assert_eq!(redone, Some("Smart Cuts".to_string()));
        assert_eq!(sequence.tracks[1].clips.len(), 2);
        assert_eq!(sequence.duration_frames(), 100);
    }

    #[test]
    fn test_add_composition_command() {
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let track_id = sequence.tracks[0].id; // V2 track for graphics overlay

        let clip = Clip::new(
            ClipId::new(),
            AssetId::new(),
            track_id,
            "Lower Third - Alex",
            0,
            0,
            72,
        )
        .unwrap();

        let cmd = EditorCommand::AddComposition(AddComposition {
            track_id,
            clip,
            composition_id: "comp-123".into(),
            template_id: "lower-third-modern".into(),
        });

        history.execute(cmd, &mut sequence).unwrap();
        assert_eq!(sequence.tracks[0].clips.len(), 1);
        assert_eq!(sequence.duration_frames(), 72);

        // Undo
        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[0].clips.len(), 0);
        assert_eq!(sequence.duration_frames(), 0);
    }
}
