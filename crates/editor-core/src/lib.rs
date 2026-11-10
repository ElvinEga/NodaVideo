//! Central orchestration and application state for NodaVideo.
//!
//! Owns the `EditorSession`, coordinating:
//! - Versioned project documents (`nodavideo-project`)
//! - Deterministic multitrack sequence editing (`nodavideo-timeline`)
//! - Unified undo/redo command history (`nodavideo-command-system`)
//! - Transport playback and frame-accurate seeking
//! - Selection state tracking
//! - Staged changeset preview and approval
//! - Subsystem dispatch for media, audio, GPU compositing, and export

pub mod error;
pub mod export;
pub mod playback;
pub mod selection;
pub mod session;

pub use error::EditorError;
pub use export::{ExportCoordinator, ExportStatus};
pub use playback::{PlaybackController, PlayheadState, format_timecode};
pub use selection::SelectionState;
pub use session::EditorSession;

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_command_system::{AddClip, EditorCommand, StagedChangeset};
    use nodavideo_project::{AssetId, FrameRate, MediaType, TimeRational};
    use nodavideo_timeline::{Clip, ClipId};
    use std::path::PathBuf;

    #[test]
    fn test_editor_session_initialization() {
        let session = EditorSession::new_empty("My New Project");
        assert_eq!(session.project.name, "My New Project");
        assert_eq!(session.sequence.tracks.len(), 4);
        assert_eq!(session.duration_frames(), 0);
        assert_eq!(session.playback.current_frame(), 0);
        assert_eq!(session.playback.current_timecode(), "00:00:00:00");
        assert!(!session.can_undo());
        assert!(!session.can_redo());
        assert!(!session.selection.has_selection());
        assert!(!session.has_staged_changeset());
    }

    #[test]
    fn test_command_execution_undo_redo_and_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let project_path = temp_dir.path().join("test_project.nodavideo");

        let mut session = EditorSession::new_empty("Persistence Test");
        let v1_id = session.sequence.tracks[1].id;

        // Import a dummy asset
        let asset = session.import_file(
            PathBuf::from("/path/to/media.mp4"),
            MediaType::Video,
            1024 * 1024,
            Some(TimeRational::from_frames(120, FrameRate::FPS_24)),
            Some((1920, 1080)),
        );
        let asset_id = asset.id;

        // Add clip to V1 track
        let clip_id = ClipId::new();
        let clip = Clip::new(clip_id, asset_id, v1_id, "Clip 1", 0, 0, 72).unwrap();
        session
            .execute_command(EditorCommand::AddClip(AddClip {
                track_id: v1_id,
                clip,
            }))
            .unwrap();

        assert_eq!(session.duration_frames(), 72);
        assert!(session.can_undo());

        // Save project to disk
        session.save_as(&project_path).unwrap();
        assert!(project_path.exists());

        // Reopen project from disk
        let reopened = EditorSession::load_from_file(&project_path).unwrap();
        assert_eq!(reopened.project.name, "Persistence Test");
        assert_eq!(reopened.project.assets.len(), 1);
        assert_eq!(reopened.project.assets[0].id, asset_id);
        assert_eq!(reopened.sequence.duration_frames(), 72);
        assert_eq!(reopened.sequence.tracks[1].clips.len(), 1);
        assert_eq!(reopened.sequence.tracks[1].clips[0].id, clip_id);

        // Undo in original session
        session.undo().unwrap();
        assert_eq!(session.duration_frames(), 0);
        assert!(session.can_redo());

        // Redo in original session
        session.redo().unwrap();
        assert_eq!(session.duration_frames(), 72);
    }

    #[test]
    fn test_playback_navigation_and_cut_jumping() {
        let mut session = EditorSession::new_empty("Playback Test");
        let v1_id = session.sequence.tracks[1].id;
        let asset_id = AssetId::new();

        // Place Clip 1: frames 0..48
        let clip1 = Clip::new(ClipId::new(), asset_id, v1_id, "Clip 1", 0, 0, 48).unwrap();
        // Place Clip 2: frames 48..96
        let clip2 = Clip::new(ClipId::new(), asset_id, v1_id, "Clip 2", 48, 0, 48).unwrap();

        session
            .execute_command(EditorCommand::AddClip(AddClip {
                track_id: v1_id,
                clip: clip1,
            }))
            .unwrap();
        session
            .execute_command(EditorCommand::AddClip(AddClip {
                track_id: v1_id,
                clip: clip2,
            }))
            .unwrap();

        assert_eq!(session.duration_frames(), 96);

        // Frame seeking
        session.seek_frame(20);
        assert_eq!(session.playback.current_frame(), 20);

        // Cut jumping
        session.jump_to_next_cut();
        assert_eq!(session.playback.current_frame(), 48);

        session.jump_to_next_cut();
        assert_eq!(session.playback.current_frame(), 96);

        session.jump_to_previous_cut();
        assert_eq!(session.playback.current_frame(), 48);

        session.jump_to_previous_cut();
        assert_eq!(session.playback.current_frame(), 0);

        // Playhead stepping
        session.step_forward(10);
        assert_eq!(session.playback.current_frame(), 10);
        session.step_backward(5);
        assert_eq!(session.playback.current_frame(), 5);

        // Real-time playback advancing
        session.play();
        assert!(session.playback.is_playing());
        // 1.0 second at 24fps -> 24 frames advanced
        session.advance_playback(1.0);
        assert_eq!(session.playback.current_frame(), 29);
    }

    #[test]
    fn test_staged_changeset_workflow() {
        let mut session = EditorSession::new_empty("AI Workflow Test");
        let v1_id = session.sequence.tracks[1].id;
        let asset_id = AssetId::new();

        let clip1 = Clip::new(ClipId::new(), asset_id, v1_id, "AI Clip 1", 0, 0, 50).unwrap();
        let clip2 = Clip::new(ClipId::new(), asset_id, v1_id, "AI Clip 2", 50, 0, 50).unwrap();

        let mut staged = StagedChangeset::new("Smart Cuts", "Generate rough cut from 2 clips")
            .with_skill("smart-cuts");
        staged.add_command(EditorCommand::AddClip(AddClip {
            track_id: v1_id,
            clip: clip1,
        }));
        staged.add_command(EditorCommand::AddClip(AddClip {
            track_id: v1_id,
            clip: clip2,
        }));

        // Stage proposal
        let diff = session.stage_changeset(staged).unwrap();
        assert_eq!(diff.original_clip_count, 0);
        assert_eq!(diff.staged_clip_count, 2);
        assert_eq!(diff.staged_duration_frames, 100);
        assert_eq!(session.duration_frames(), 0); // Active timeline unchanged until committed

        // Commit staged proposal
        session.commit_staged_changeset().unwrap();
        assert!(!session.has_staged_changeset());
        assert_eq!(session.duration_frames(), 100);
        assert!(session.can_undo());

        // Single undo reverts entire batch
        session.undo().unwrap();
        assert_eq!(session.duration_frames(), 0);
    }

    #[test]
    fn test_selection_state_tracking() {
        let mut session = EditorSession::new_empty("Selection Test");
        let v1_id = session.sequence.tracks[1].id;
        let asset_id = AssetId::new();
        let clip_id = ClipId::new();
        let clip = Clip::new(clip_id, asset_id, v1_id, "Clip", 0, 0, 24).unwrap();

        session
            .execute_command(EditorCommand::AddClip(AddClip {
                track_id: v1_id,
                clip,
            }))
            .unwrap();

        assert!(!session.selection.has_selection());

        session.select_clip(clip_id);
        assert!(session.selection.has_selection());
        assert!(session.selection.is_clip_selected(&clip_id));
        let selected = session.get_selected_clips();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, clip_id);

        session.clear_selection();
        assert!(!session.selection.has_selection());
    }
}
