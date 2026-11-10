//! NodaVideo unified command system and undo/redo architecture.
//!
//! As defined in `AGENTS.md`, every state mutation — human or AI — goes through
//! `EditorCommand`. Command execution is deterministic, reversible, and transactional.

pub mod command;
pub mod error;
pub mod history;
pub mod staged;

pub use command::{
    AddClip, AddComposition, AddTrack, AddTransition, BatchCommand, DeleteClip, DeleteTrack,
    EditorCommand, MoveClip, RemoveTransition, SetClipAudio, SetClipTransform, SplitClip,
    ToggleLock, ToggleMute, ToggleSolo, TrimClip, UnsplitClip,
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

    #[test]
    fn test_set_clip_transform_and_audio_undo_redo() {
        use nodavideo_timeline::{ClipAudioParams, ClipTransform};

        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let track_id = sequence.tracks[1].id;
        let clip_id = ClipId::new();

        let clip = Clip::new(clip_id, AssetId::new(), track_id, "Video Clip", 0, 0, 48).unwrap();
        history
            .execute(
                EditorCommand::AddClip(AddClip { track_id, clip }),
                &mut sequence,
            )
            .unwrap();

        let initial_transform = sequence.tracks[1].clips[0].transform;
        let target_transform = ClipTransform {
            position_x: 200.0,
            position_y: 100.0,
            scale_x: 1.25,
            scale_y: 1.25,
            rotation_degrees: 90.0,
            opacity: 0.75,
        };

        // 1. Execute SetClipTransform
        let transform_cmd = EditorCommand::SetClipTransform(SetClipTransform {
            clip_id,
            old_transform: initial_transform,
            new_transform: target_transform,
        });
        history.execute(transform_cmd, &mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips[0].transform, target_transform);

        // 2. Undo SetClipTransform
        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips[0].transform, initial_transform);

        // 3. Redo SetClipTransform
        history.redo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips[0].transform, target_transform);

        // 4. Execute SetClipAudio
        let initial_audio = sequence.tracks[1].clips[0].audio_params;
        let target_audio = ClipAudioParams {
            volume_db: -12.0,
            pan: 0.4,
        };
        let audio_cmd = EditorCommand::SetClipAudio(SetClipAudio {
            clip_id,
            old_audio: initial_audio,
            new_audio: target_audio,
        });
        history.execute(audio_cmd, &mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips[0].audio_params, target_audio);

        // 5. Undo SetClipAudio
        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips[0].audio_params, initial_audio);
    }

    #[test]
    fn test_add_and_remove_transition_undo_redo() {
        use nodavideo_timeline::{Transition, TransitionAlignment, TransitionId, TransitionType};

        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let track_id = sequence.tracks[1].id;

        let clip1_id = ClipId::new();
        let clip2_id = ClipId::new();
        let clip1 = Clip::new(clip1_id, AssetId::new(), track_id, "Clip 1", 0, 0, 48).unwrap();
        let clip2 = Clip::new(clip2_id, AssetId::new(), track_id, "Clip 2", 48, 0, 48).unwrap();

        history
            .execute(
                EditorCommand::AddClip(AddClip {
                    track_id,
                    clip: clip1,
                }),
                &mut sequence,
            )
            .unwrap();
        history
            .execute(
                EditorCommand::AddClip(AddClip {
                    track_id,
                    clip: clip2,
                }),
                &mut sequence,
            )
            .unwrap();

        let trans_id = TransitionId::new();
        let trans = Transition::new(
            trans_id,
            TransitionType::Dissolve,
            TransitionAlignment::Centered,
            clip1_id,
            clip2_id,
            track_id,
            48,
            24,
        )
        .unwrap();

        // 1. AddTransition
        let add_trans_cmd = EditorCommand::AddTransition(AddTransition {
            track_id,
            transition: trans.clone(),
        });
        history.execute(add_trans_cmd, &mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].transitions.len(), 1);
        assert_eq!(sequence.get_transition(trans_id), Some(&trans));

        // 2. Undo AddTransition
        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].transitions.len(), 0);

        // 3. Redo AddTransition
        history.redo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].transitions.len(), 1);

        // 4. RemoveTransition
        let remove_trans_cmd = EditorCommand::RemoveTransition(RemoveTransition {
            track_id,
            transition: trans,
        });
        history.execute(remove_trans_cmd, &mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].transitions.len(), 0);

        // 5. Undo RemoveTransition
        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].transitions.len(), 1);
    }
}
