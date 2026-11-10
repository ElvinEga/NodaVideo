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

    #[core::prelude::v1::test]
    fn test_preview_monitor_panel_state() {
        use workspace::panels::preview_monitor::PreviewMonitorPanel;

        let panel = PreviewMonitorPanel::new();
        assert!(!panel.is_playing);
        assert!(!panel.is_looping);
        assert_eq!(panel.timecode, "00:00:00:00");
        assert_eq!(panel.duration, "00:00:00:00");
        assert_eq!(panel.sequence_name, "Main Sequence");
        assert_eq!(panel.sequence_info, "1920×1080 24fps");
        assert!(panel.current_frame_path.is_none());
    }

    #[core::prelude::v1::test]
    fn test_export_settings_defaults() {
        use nodavideo_media_engine::ExportSettings;
        use nodavideo_project::FrameRate;

        let settings = ExportSettings::default();
        assert_eq!(settings.width, 1920);
        assert_eq!(settings.height, 1080);
        assert_eq!(settings.fps, FrameRate::FPS_24);
        assert_eq!(settings.video_bitrate_kbps, 8000);
        assert_eq!(settings.audio_bitrate_kbps, 192);
        assert_eq!(settings.video_codec, "libx264");
        assert_eq!(settings.audio_codec, "aac");
    }

    #[tokio::test]
    async fn test_agent_runtime_and_skill_discovery() {
        use nodavideo_agent_acp::AcpAgentProvider;
        use nodavideo_agent_runtime::{AgentRuntime, MockAgentProvider};
        use nodavideo_skill_registry::SkillRegistry;
        use std::sync::Arc;

        let runtime = AgentRuntime::new();
        runtime
            .register_provider(Arc::new(AcpAgentProvider::antigravity()))
            .await;
        runtime
            .register_provider(Arc::new(AcpAgentProvider::gemini()))
            .await;
        runtime
            .register_provider(Arc::new(MockAgentProvider::new()))
            .await;

        let providers = runtime.list_providers().await;
        assert_eq!(providers.len(), 3);
        assert!(providers.iter().any(|(id, _)| id == "acp-antigravity"));
        assert!(providers.iter().any(|(id, _)| id == "acp-gemini"));
        assert!(providers.iter().any(|(id, _)| id == "mock"));

        let mut skill_reg = SkillRegistry::new();
        let _ = skill_reg.discover_from_dir(std::path::Path::new("../../skills"));
        let _ = skill_reg.discover_from_dir(std::path::Path::new("skills"));
        // Registry discovers available skills when path exists
        assert!(skill_reg.get("non-existent").is_none());
    }

    #[core::prelude::v1::test]
    fn test_mcp_executor_timeline_integration() {
        use nodavideo_command_system::CommandHistory;
        use nodavideo_mcp_server::McpExecutor;
        use nodavideo_project::{AssetId, ProjectDocument};
        use nodavideo_timeline::Sequence;
        use serde_json::json;

        let project = ProjectDocument::new("Integration Test");
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(100);

        let track_id = sequence.tracks[0].id.to_string();
        let asset_id = AssetId::new().to_string();

        let add_res = McpExecutor::execute(
            "add_clip",
            &json!({
                "track_id": track_id,
                "asset_id": asset_id,
                "start_frame": 0,
                "source_in": 0,
                "source_out": 48
            }),
            &project,
            &mut sequence,
            &mut history,
            true,
        );
        assert!(add_res.is_ok());
        let clip_id_str = add_res.unwrap()["clip_id"].as_str().unwrap().to_string();

        let split_res = McpExecutor::execute(
            "split_clip",
            &json!({
                "clip_id": clip_id_str,
                "split_frame": 24
            }),
            &project,
            &mut sequence,
            &mut history,
            true,
        );
        assert!(split_res.is_ok());
        assert_eq!(sequence.tracks[0].clips.len(), 2);
        assert!(history.can_undo());

        // Test unified undo history
        assert!(history.undo(&mut sequence).is_ok());
        assert_eq!(sequence.tracks[0].clips.len(), 1);
        assert!(history.undo(&mut sequence).is_ok());
        assert_eq!(sequence.tracks[0].clips.len(), 0);
    }
}
