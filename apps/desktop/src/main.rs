#![recursion_limit = "512"]

pub mod app;
pub mod theme;
pub mod workspace;

fn main() {
    app::run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[core::prelude::v1::test]
    fn test_workspace_tab_indexing() {
        use workspace::navigation::WorkspaceTab;
        assert_eq!(WorkspaceTab::Edit.index(), 0);
        assert_eq!(WorkspaceTab::Color.index(), 1);
        assert_eq!(WorkspaceTab::Audio.index(), 2);
        assert_eq!(WorkspaceTab::AiStudio.index(), 3);
        assert_eq!(WorkspaceTab::from_index(0), WorkspaceTab::Edit);
        assert_eq!(WorkspaceTab::from_index(1), WorkspaceTab::Color);
        assert_eq!(WorkspaceTab::from_index(2), WorkspaceTab::Audio);
        assert_eq!(WorkspaceTab::from_index(3), WorkspaceTab::AiStudio);
        assert_eq!(WorkspaceTab::from_index(99), WorkspaceTab::Edit);
    }

    #[core::prelude::v1::test]
    fn test_workspace_layout_state_toggles() {
        use workspace::layout::WorkspaceLayoutState;
        let mut state = WorkspaceLayoutState::default();
        assert!(state.show_media_browser);
        assert!(state.show_inspector);
        assert!(state.show_timeline);
        assert!(state.show_assistant);

        state.toggle_media_browser();
        assert!(!state.show_media_browser);

        state.toggle_assistant();
        assert!(!state.show_assistant);

        state.reset_layout();
        assert!(state.show_media_browser);
        assert!(state.show_assistant);
    }

    #[core::prelude::v1::test]
    fn test_timeline_panel_state_and_commands() {
        use nodavideo_command_system::{AddClip, EditorCommand};
        use nodavideo_project::AssetId;
        use nodavideo_timeline::{Clip, ClipId};
        use workspace::panels::timeline::{TimelinePanel, TimelineTool};

        let mut panel = TimelinePanel::new();
        assert_eq!(panel.sequence.tracks.len(), 4);
        assert_eq!(panel.zoom_level, 100);
        assert_eq!(panel.current_tool, TimelineTool::Select);
        assert!(panel.snapping_enabled);
        assert!(!panel.ripple_enabled);
        assert_eq!(panel.pixels_per_frame(), 5.0);

        // Zoom calculations
        panel.zoom_level = 200;
        assert_eq!(panel.pixels_per_frame(), 10.0);
        panel.zoom_level = 50;
        assert_eq!(panel.pixels_per_frame(), 2.5);

        // Add clip command via history
        let v1_id = panel.sequence.tracks[1].id;
        let clip_id = ClipId::new();
        let clip = Clip::new(clip_id, AssetId::new(), v1_id, "Test", 0, 0, 72).unwrap();

        let add_cmd = EditorCommand::AddClip(AddClip {
            track_id: v1_id,
            clip,
        });
        panel.history.execute(add_cmd, &mut panel.sequence).unwrap();

        assert_eq!(panel.sequence.tracks[1].clips.len(), 1);
        assert_eq!(panel.sequence.duration_frames(), 72);
        assert!(panel.can_undo());
        assert!(!panel.can_redo());

        // Undo
        panel.history.undo(&mut panel.sequence).unwrap();
        assert_eq!(panel.sequence.tracks[1].clips.len(), 0);
        assert_eq!(panel.sequence.duration_frames(), 0);
        assert!(!panel.can_undo());
        assert!(panel.can_redo());

        // Redo
        panel.history.redo(&mut panel.sequence).unwrap();
        assert_eq!(panel.sequence.tracks[1].clips.len(), 1);
        assert_eq!(panel.sequence.duration_frames(), 72);
    }
}
