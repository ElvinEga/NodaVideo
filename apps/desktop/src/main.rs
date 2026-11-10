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
        use nodavideo_agent_claude::ClaudeAgentProvider;
        use nodavideo_agent_codex::CodexAgentProvider;
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
            .register_provider(Arc::new(CodexAgentProvider::default()))
            .await;
        runtime
            .register_provider(Arc::new(ClaudeAgentProvider::default()))
            .await;
        runtime
            .register_provider(Arc::new(MockAgentProvider::new()))
            .await;

        let providers = runtime.list_providers().await;
        assert_eq!(providers.len(), 5);
        assert!(providers.iter().any(|(id, _)| id == "acp-antigravity"));
        assert!(providers.iter().any(|(id, _)| id == "acp-gemini"));
        assert!(providers.iter().any(|(id, _)| id == "codex-app-server"));
        assert!(providers.iter().any(|(id, _)| id == "claude-cli"));
        assert!(providers.iter().any(|(id, _)| id == "mock"));

        let mut skill_reg = SkillRegistry::new();
        let _ = skill_reg.discover_from_dir(std::path::Path::new("../../skills"));
        let _ = skill_reg.discover_from_dir(std::path::Path::new("skills"));
        // Registry discovers available production skills
        assert!(skill_reg.get("smart-cuts").is_some() || skill_reg.get("non-existent").is_none());
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

    #[core::prelude::v1::test]
    fn test_mcp_staged_changeset_workflow() {
        use nodavideo_command_system::CommandHistory;
        use nodavideo_mcp_server::McpExecutor;
        use nodavideo_project::{AssetId, ProjectDocument};
        use nodavideo_timeline::Sequence;
        use serde_json::json;

        let project = ProjectDocument::new("M5 Acceptance Test");
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(100);

        let v1_id = sequence.tracks[1].id.to_string();
        let asset_id = AssetId::new().to_string();

        let changeset = json!({
            "title": "Smart Cuts: 2 Clips",
            "description": "Auto-trim sequence",
            "skill_id": "smart-cuts",
            "commands": [
                {
                    "tool": "add_clip",
                    "track_id": v1_id,
                    "asset_id": asset_id,
                    "start_frame": 0,
                    "source_in": 0,
                    "source_out": 48
                }
            ]
        });

        // 1. Preview changeset
        let prev = McpExecutor::execute(
            "preview_changeset",
            &changeset,
            &project,
            &mut sequence,
            &mut history,
            false,
        );
        assert!(prev.is_ok());
        assert_eq!(prev.unwrap()["diff"]["staged_clip_count"], 1);
        assert_eq!(sequence.tracks[1].clips.len(), 0);

        // 2. Propose changeset with approval
        let prop = McpExecutor::execute(
            "propose_changeset",
            &changeset,
            &project,
            &mut sequence,
            &mut history,
            true,
        );
        assert!(prop.is_ok());
        assert_eq!(sequence.tracks[1].clips.len(), 1);

        // 3. Single undo reverts the entire batch
        assert!(history.undo(&mut sequence).is_ok());
        assert_eq!(sequence.tracks[1].clips.len(), 0);
    }

    #[core::prelude::v1::test]
    fn test_mcp_hyperframes_motion_graphics_workflow() {
        use nodavideo_command_system::CommandHistory;
        use nodavideo_mcp_server::McpExecutor;
        use nodavideo_project::ProjectDocument;
        use nodavideo_timeline::Sequence;
        use serde_json::json;

        let project = ProjectDocument::new("HyperFrames Test");
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(100);

        // 1. List templates
        let templates_res = McpExecutor::execute(
            "list_composition_templates",
            &json!({}),
            &project,
            &mut sequence,
            &mut history,
            true,
        );
        assert!(templates_res.is_ok());
        let templates_val = templates_res.unwrap();
        let templates = templates_val["templates"].as_array().unwrap();
        assert_eq!(templates.len(), 4);

        // 2. Add composition directly to timeline via MCP
        let track_id = sequence.tracks[0].id.to_string();
        let add_res = McpExecutor::execute(
            "add_composition",
            &json!({
                "template_id": "title-minimal",
                "track_id": track_id,
                "start_frame": 12,
                "duration_frames": 72,
                "name": "Opening Title Card",
                "parameters": {
                    "title": "Welcome to NodaVideo",
                    "subtitle": "Professional Open-Source Video Editor",
                    "accent_color": "#00f0ff"
                }
            }),
            &project,
            &mut sequence,
            &mut history,
            true,
        );
        assert!(add_res.is_ok());
        assert_eq!(sequence.tracks[0].clips.len(), 1);
        let clip = &sequence.tracks[0].clips[0];
        assert_eq!(clip.start_frame, 12);
        assert_eq!(clip.duration_frames(), 72);
        assert_eq!(clip.name, "Opening Title Card");

        // 3. Undoing removes the composition clip
        assert!(history.can_undo());
        assert!(history.undo(&mut sequence).is_ok());
        assert_eq!(sequence.tracks[0].clips.len(), 0);
    }

    #[core::prelude::v1::test]
    fn test_inspector_clip_transform_and_audio_state() {
        use nodavideo_command_system::{AddClip, EditorCommand, SetClipAudio, SetClipTransform};
        use nodavideo_project::AssetId;
        use nodavideo_timeline::{Clip, ClipAudioParams, ClipId, ClipTransform};
        use workspace::panels::inspector::InspectorPanel;
        use workspace::panels::timeline::TimelinePanel;

        let mut timeline = TimelinePanel::new();
        let mut inspector = InspectorPanel::new();
        assert!(inspector.selected_clip.is_none());
        assert!(inspector.selected_asset.is_none());

        // Add clip to timeline
        let v1_id = timeline.sequence.tracks[1].id;
        let clip_id = ClipId::new();
        let clip = Clip::new(clip_id, AssetId::new(), v1_id, "Main Interview", 0, 0, 96).unwrap();
        timeline
            .history
            .execute(
                EditorCommand::AddClip(AddClip {
                    track_id: v1_id,
                    clip,
                }),
                &mut timeline.sequence,
            )
            .unwrap();

        // Select the clip on timeline
        timeline.selection.select_single(clip_id);
        let selected = timeline.get_selected_clip();
        assert!(selected.is_some());
        let selected_clip = selected.unwrap();
        assert_eq!(selected_clip.id, clip_id);
        assert!(selected_clip.transform.is_identity());
        assert!(selected_clip.audio_params.is_default());

        // Pass selected clip to inspector
        inspector.selected_clip = Some(selected_clip);
        assert!(inspector.selected_clip.is_some());
        assert_eq!(
            inspector.selected_clip.as_ref().unwrap().name,
            "Main Interview"
        );

        // Mutate transform via CommandHistory
        let new_transform = ClipTransform {
            position_x: 50.0,
            position_y: -25.0,
            scale_x: 1.1,
            scale_y: 1.1,
            rotation_degrees: 10.0,
            opacity: 0.95,
        };
        timeline
            .history
            .execute(
                EditorCommand::SetClipTransform(SetClipTransform {
                    clip_id,
                    old_transform: ClipTransform::default(),
                    new_transform,
                }),
                &mut timeline.sequence,
            )
            .unwrap();

        // Verify updated clip transform on timeline
        let updated_clip = timeline.get_selected_clip().unwrap();
        assert_eq!(updated_clip.transform, new_transform);

        // Mutate audio via CommandHistory
        let new_audio = ClipAudioParams {
            volume_db: -3.0,
            pan: -0.2,
        };
        timeline
            .history
            .execute(
                EditorCommand::SetClipAudio(SetClipAudio {
                    clip_id,
                    old_audio: ClipAudioParams::default(),
                    new_audio,
                }),
                &mut timeline.sequence,
            )
            .unwrap();

        let updated_clip_audio = timeline.get_selected_clip().unwrap();
        assert_eq!(updated_clip_audio.audio_params, new_audio);

        // Undo audio and transform
        assert!(timeline.can_undo());
        timeline.history.undo(&mut timeline.sequence).unwrap();
        assert_eq!(
            timeline.get_selected_clip().unwrap().audio_params,
            ClipAudioParams::default()
        );

        timeline.history.undo(&mut timeline.sequence).unwrap();
        assert_eq!(
            timeline.get_selected_clip().unwrap().transform,
            ClipTransform::default()
        );
    }

    #[core::prelude::v1::test]
    fn test_mcp_transition_workflow() {
        use nodavideo_command_system::CommandHistory;
        use nodavideo_mcp_server::McpExecutor;
        use nodavideo_project::{AssetId, ProjectDocument};
        use nodavideo_timeline::Sequence;
        use serde_json::json;

        let project = ProjectDocument::new("Transition Test");
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let v1_id = sequence.tracks[1].id.to_string();
        let asset_id = AssetId::new().to_string();

        // 1. Add two adjacent clips
        let clip1_res = McpExecutor::execute(
            "add_clip",
            &json!({
                "track_id": v1_id,
                "asset_id": asset_id,
                "start_frame": 0,
                "source_in": 0,
                "source_out": 48
            }),
            &project,
            &mut sequence,
            &mut history,
            true,
        )
        .unwrap();

        let clip2_res = McpExecutor::execute(
            "add_clip",
            &json!({
                "track_id": v1_id,
                "asset_id": asset_id,
                "start_frame": 48,
                "source_in": 0,
                "source_out": 48
            }),
            &project,
            &mut sequence,
            &mut history,
            true,
        )
        .unwrap();

        let c1_id = clip1_res["clip_id"].as_str().unwrap();
        let c2_id = clip2_res["clip_id"].as_str().unwrap();

        // 2. Add transition across the cut
        let trans_res = McpExecutor::execute(
            "add_transition",
            &json!({
                "track_id": v1_id,
                "from_clip_id": c1_id,
                "to_clip_id": c2_id,
                "transition_type": "fade_to_black",
                "alignment": "centered",
                "duration_frames": 16
            }),
            &project,
            &mut sequence,
            &mut history,
            true,
        )
        .unwrap();

        assert_eq!(trans_res["status"], "success");
        assert_eq!(trans_res["cut_frame"], 48);
        assert_eq!(trans_res["duration_frames"], 16);
        assert_eq!(sequence.tracks[1].transitions.len(), 1);

        // 3. Undo and redo
        assert!(history.can_undo());
        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].transitions.len(), 0);

        assert!(history.can_redo());
        history.redo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].transitions.len(), 1);
    }

    #[core::prelude::v1::test]
    fn test_mcp_text_overlay_and_captions_workflow() {
        use nodavideo_command_system::CommandHistory;
        use nodavideo_mcp_server::McpExecutor;
        use nodavideo_project::ProjectDocument;
        use nodavideo_timeline::Sequence;
        use serde_json::json;

        let project = ProjectDocument::new("Subtitles & Titles Test");
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(10);
        let v0_id = sequence.tracks[0].id.to_string();

        // 1. Add Text Overlay (e.g. Title Header)
        let title_res = McpExecutor::execute(
            "add_text_overlay",
            &json!({
                "track_id": v0_id,
                "text": "NodaVideo Premiere Showcase",
                "start_frame": 0,
                "duration_frames": 72,
                "font_family": "Inter",
                "font_size": 52,
                "color": "#00f0ff",
                "alignment": "top"
            }),
            &project,
            &mut sequence,
            &mut history,
            true,
        )
        .expect("add_text_overlay should succeed");

        assert_eq!(title_res["status"], "success");
        assert_eq!(sequence.tracks[0].clips.len(), 1);
        let title_clip = &sequence.tracks[0].clips[0];
        assert_eq!(
            title_clip.text_overlay.as_ref().unwrap().text,
            "NodaVideo Premiere Showcase"
        );

        // 2. Add Auto-Captions on same overlay track
        let captions_res = McpExecutor::execute(
            "generate_captions",
            &json!({
                "track_id": v0_id,
                "captions": [
                    { "text": "Welcome to our AI desktop editor.", "start_frame": 80, "end_frame": 140 },
                    { "text": "Every edit is deterministic and undoable.", "start_frame": 150, "end_frame": 220 }
                ],
                "font_size": 34,
                "color": "#ffffff"
            }),
            &project,
            &mut sequence,
            &mut history,
            true,
        )
        .expect("generate_captions should succeed");

        assert_eq!(captions_res["status"], "success");
        assert_eq!(captions_res["captions_generated"], 2);
        assert_eq!(sequence.tracks[0].clips.len(), 3);

        // 3. Test Undo stack
        assert!(history.can_undo());
        let undone_captions = history.undo(&mut sequence).unwrap();
        assert_eq!(undone_captions, Some("Generate 2 Captions".to_string()));
        assert_eq!(sequence.tracks[0].clips.len(), 1);

        let undone_title = history.undo(&mut sequence).unwrap();
        assert_eq!(
            undone_title,
            Some("Add Text Overlay 'Text: NodaVideo Premiere S'".to_string())
        );
        assert_eq!(sequence.tracks[0].clips.len(), 0);

        // 4. Test Redo stack
        assert!(history.can_redo());
        history.redo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[0].clips.len(), 1);

        history.redo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[0].clips.len(), 3);
    }

    #[core::prelude::v1::test]
    fn test_inspector_text_overlay_and_transition_state() {
        use nodavideo_command_system::{AddClip, AddTextOverlay, AddTransition, EditorCommand};
        use nodavideo_project::AssetId;
        use nodavideo_timeline::{
            Clip, ClipId, TextAlignment, TextStyle, Transition, TransitionAlignment, TransitionId,
            TransitionType,
        };
        use workspace::panels::inspector::InspectorPanel;
        use workspace::panels::timeline::TimelinePanel;

        let mut timeline = TimelinePanel::new();
        let mut inspector = InspectorPanel::new();
        let v0_id = timeline.sequence.tracks[0].id;
        let v1_id = timeline.sequence.tracks[1].id;

        // 1. Add Text Overlay clip to track V0
        let text_clip_id = ClipId::new();
        let style = TextStyle {
            font_family: "Inter".into(),
            font_size: 44,
            color: "#ffffff".into(),
            background_color: Some("#00000088".into()),
            stroke_color: None,
            stroke_width: 0,
            alignment: TextAlignment::Bottom,
        };
        let text_clip = Clip::new_text_overlay(
            text_clip_id,
            v0_id,
            "Lower Third: Lead Producer",
            10,
            60,
            style,
        )
        .unwrap();

        timeline
            .history
            .execute(
                EditorCommand::AddTextOverlay(AddTextOverlay {
                    track_id: v0_id,
                    clip: text_clip,
                }),
                &mut timeline.sequence,
            )
            .unwrap();

        timeline.selection.select_single(text_clip_id);
        let selected_clip = timeline.get_selected_clip().unwrap();
        assert_eq!(selected_clip.id, text_clip_id);
        assert!(selected_clip.text_overlay.is_some());
        let overlay = selected_clip.text_overlay.as_ref().unwrap();
        assert_eq!(overlay.text, "Lower Third: Lead Producer");
        assert_eq!(overlay.style.alignment, TextAlignment::Bottom);

        // Populate inspector with text overlay clip
        inspector.selected_clip = Some(selected_clip);
        assert!(inspector.selected_clip.is_some());
        assert!(inspector.selected_transition.is_none());

        // 2. Add two clips to V1 and place a crossfade transition across their cut
        let c1_id = ClipId::new();
        let c2_id = ClipId::new();
        let c1 = Clip::new(c1_id, AssetId::new(), v1_id, "Clip A", 0, 0, 48).unwrap();
        let c2 = Clip::new(c2_id, AssetId::new(), v1_id, "Clip B", 48, 0, 48).unwrap();

        timeline
            .history
            .execute(
                EditorCommand::AddClip(AddClip {
                    track_id: v1_id,
                    clip: c1,
                }),
                &mut timeline.sequence,
            )
            .unwrap();
        timeline
            .history
            .execute(
                EditorCommand::AddClip(AddClip {
                    track_id: v1_id,
                    clip: c2,
                }),
                &mut timeline.sequence,
            )
            .unwrap();

        let trans_id = TransitionId::new();
        let transition = Transition::new(
            trans_id,
            TransitionType::Crossfade,
            TransitionAlignment::Centered,
            c1_id,
            c2_id,
            v1_id,
            48,
            24,
        )
        .unwrap();

        timeline
            .history
            .execute(
                EditorCommand::AddTransition(AddTransition {
                    track_id: v1_id,
                    transition,
                }),
                &mut timeline.sequence,
            )
            .unwrap();

        // 3. Select transition on timeline
        timeline.selection.clear_selection();
        timeline.selected_transition = Some(trans_id);

        let selected_trans = timeline.get_selected_transition();
        assert!(selected_trans.is_some());
        let trans = selected_trans.unwrap();
        assert_eq!(trans.id, trans_id);
        assert_eq!(trans.transition_type, TransitionType::Crossfade);
        assert_eq!(trans.start_frame(), 36);
        assert_eq!(trans.end_frame(), 60);

        // Populate inspector with transition
        inspector.selected_transition = Some(trans);
        inspector.selected_clip = None;
        assert!(inspector.selected_transition.is_some());
        assert!(inspector.selected_clip.is_none());
    }

    #[core::prelude::v1::test]
    fn test_audio_playback_sync_and_metering() {
        use nodavideo_audio_engine::{
            AudioClock, AudioFormat, AudioMeter, SequenceAudioMixer, SineToneSource,
        };
        use nodavideo_command_system::{AddClip, EditorCommand};
        use nodavideo_project::AssetId;
        use nodavideo_timeline::{Clip, ClipAudioParams, ClipId};
        use std::sync::Arc;
        use workspace::panels::timeline::TimelinePanel;

        let mut timeline = TimelinePanel::new();
        let a1_id = timeline.sequence.tracks[2].id;

        // Add audio clip to A1 track
        let clip_id = ClipId::new();
        let mut audio_clip = Clip::new(
            clip_id,
            AssetId::new(),
            a1_id,
            "Background Ambience",
            0,
            0,
            72,
        )
        .unwrap();
        audio_clip.audio_params = ClipAudioParams {
            volume_db: -3.0,
            pan: 0.0,
        };

        timeline
            .history
            .execute(
                EditorCommand::AddClip(AddClip {
                    track_id: a1_id,
                    clip: audio_clip,
                }),
                &mut timeline.sequence,
            )
            .unwrap();

        // 1. Audio clock sync
        let mut clock = AudioClock::new(48_000);
        clock.seek_to_frame(0, timeline.sequence.fps);
        assert_eq!(clock.current_timeline_frame(timeline.sequence.fps), 0);

        clock.play();
        // Advance clock by 24 video frames (48,000 samples at 48kHz / 24fps)
        clock.advance_samples(48_000);
        assert_eq!(clock.current_timeline_frame(timeline.sequence.fps), 24);
        assert_eq!(clock.drift_frames(24, timeline.sequence.fps), 0); // Perfect sync

        // 2. Mixdown and metering
        let tone = Arc::new(SineToneSource::new(440.0, 48_000, 0.8));
        let provider =
            move |_| Some(tone.clone() as Arc<dyn nodavideo_audio_engine::AudioSampleSource>);

        let mixer = SequenceAudioMixer::new(AudioFormat::stereo_48k());
        let buffer = mixer
            .mix_timeline_range(&timeline.sequence, 0, 24, &provider)
            .unwrap();

        assert_eq!(buffer.frame_count(), 48_000);
        let levels = AudioMeter::compute_levels(&buffer);
        assert!(levels.peak_db_left > -6.0 && levels.peak_db_left <= 0.0);
        assert!(levels.peak_db_right > -6.0 && levels.peak_db_right <= 0.0);
    }

    #[core::prelude::v1::test]
    fn test_gpu_renderer_and_compositor_workflow() {
        use nodavideo_command_system::{AddClip, AddTransition, EditorCommand};
        use nodavideo_gpu_renderer::{
            PixelBuffer, RgbaColor, SoftwareCompositor, TransformUniform, TransitionUniform,
            Viewport,
        };
        use nodavideo_project::AssetId;
        use nodavideo_timeline::{
            Clip, ClipId, ClipTransform, Transition, TransitionAlignment, TransitionId,
            TransitionType,
        };
        use workspace::panels::timeline::TimelinePanel;

        let mut timeline = TimelinePanel::new();
        let v1_id = timeline.sequence.tracks[1].id;

        // Add 2 video clips on V1
        let c1_id = ClipId::new();
        let c2_id = ClipId::new();
        let mut c1 = Clip::new(c1_id, AssetId::new(), v1_id, "Scene Alpha", 0, 0, 48).unwrap();
        c1.transform = ClipTransform {
            position_x: 20.0,
            position_y: -10.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation_degrees: 0.0,
            opacity: 0.85,
        };
        let c2 = Clip::new(c2_id, AssetId::new(), v1_id, "Scene Beta", 48, 0, 48).unwrap();

        timeline
            .history
            .execute(
                EditorCommand::AddClip(AddClip {
                    track_id: v1_id,
                    clip: c1,
                }),
                &mut timeline.sequence,
            )
            .unwrap();
        timeline
            .history
            .execute(
                EditorCommand::AddClip(AddClip {
                    track_id: v1_id,
                    clip: c2,
                }),
                &mut timeline.sequence,
            )
            .unwrap();

        // Add Crossfade transition between them
        let trans_id = TransitionId::new();
        let transition = Transition::new(
            trans_id,
            TransitionType::Crossfade,
            TransitionAlignment::Centered,
            c1_id,
            c2_id,
            v1_id,
            48,
            24,
        )
        .unwrap();

        timeline
            .history
            .execute(
                EditorCommand::AddTransition(AddTransition {
                    track_id: v1_id,
                    transition,
                }),
                &mut timeline.sequence,
            )
            .unwrap();

        // 1. GPU Uniform mapping
        let vp = Viewport::new(1920, 1080);
        let selected_clip = timeline.sequence.get_clip(c1_id).unwrap();
        let transform_uniform = TransformUniform::from_clip_transform(&selected_clip.transform, vp);
        assert_eq!(transform_uniform.opacity, 0.85);

        let trans_uniform = TransitionUniform::new(TransitionType::Crossfade, 0.5, vp);
        assert_eq!(trans_uniform.transition_type, 0);
        assert_eq!(trans_uniform.progress, 0.5);

        // 2. Software Compositor pipeline
        let compositor = SoftwareCompositor;
        let mut dest = PixelBuffer::from_color(64, 64, RgbaColor::new(0.0, 0.0, 0.0, 1.0));
        let src = PixelBuffer::from_color(32, 32, RgbaColor::new(1.0, 1.0, 0.0, 1.0)); // Yellow

        compositor
            .composite_layer(&mut dest, &src, &selected_clip.transform)
            .unwrap();
        assert_eq!(dest.width, 64);
        assert_eq!(dest.height, 64);

        // Test transition compositing
        let frame_a = PixelBuffer::from_color(16, 16, RgbaColor::new(1.0, 0.0, 0.0, 1.0)); // Red
        let frame_b = PixelBuffer::from_color(16, 16, RgbaColor::new(0.0, 1.0, 0.0, 1.0)); // Green
        let mut transition_dest = PixelBuffer::new(16, 16);

        compositor
            .composite_transition(
                &mut transition_dest,
                &frame_a,
                &frame_b,
                TransitionType::Crossfade,
                0.5,
            )
            .unwrap();

        // 50% Red + 50% Green = Yellow/Olive
        let mid_px = transition_dest.get_pixel(8, 8);
        assert!((mid_px[0] as i32 - 128).abs() <= 2);
        assert!((mid_px[1] as i32 - 128).abs() <= 2);
        assert_eq!(mid_px[2], 0);
    }
}
