//! Model Context Protocol (MCP) tool exposure for editor commands.
//!
//! Exposes typed tools that external AI agent CLIs can discover and invoke.
//! Tool calls are executed against a [`timeline::TimelineEditingState`]; the
//! editor owns the live state and decides when an approved change is committed.

pub mod analysis;
pub mod composition;
pub mod error;
pub mod executor;
pub mod explorer;
pub mod schema;

pub use analysis::MediaAnalysisRequest;
pub use error::McpError;
pub use executor::McpExecutor;
pub use explorer::ExplorerChange;
pub use schema::McpToolDefinition;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;
    use timeline::{
        Asset, AudioClipProperties, Clip, MediaAsset, TimelineEditingState, TimelineFrameIndex,
        Track, TrackKind, VideoClip, VideoClipProperties,
    };
    use ulid::Ulid;

    fn media_asset(id: Ulid, kind: &str) -> MediaAsset {
        MediaAsset {
            id,
            path: format!("{kind}.mp4").into(),
            name: format!("{kind} asset"),
            width: 1920,
            height: 1080,
            duration: 10.0,
            framerate: 30.0,
            frame_rate_numerator: 30,
            frame_rate_denominator: 1,
            codec: "h264".into(),
            has_audio: true,
        }
    }

    fn executor_with_clip() -> (McpExecutor, Ulid, Ulid) {
        let track_id = Ulid::generate();
        let asset_id = Ulid::generate();
        let clip_id = Ulid::generate();
        let timeline = TimelineEditingState {
            assets: vec![Asset::Video(media_asset(asset_id, "video"))],
            tracks: vec![Track {
                id: track_id,
                name: "Video 1".into(),
                kind: TrackKind::Video,
                locked: false,
                muted: false,
                visible: true,
            }],
            clips: vec![Clip::Video(VideoClip::new(
                clip_id,
                track_id,
                asset_id,
                TimelineFrameIndex::ZERO,
                TimelineFrameIndex::ZERO,
                TimelineFrameIndex::from(90),
                VideoClipProperties::default(),
                AudioClipProperties::default(),
            ))],
            ..TimelineEditingState::default()
        };
        (McpExecutor::new(timeline), track_id, asset_id)
    }

    #[test]
    fn test_all_tools_schema_definition() {
        let tools = McpToolDefinition::all_tools();
        assert!(tools.len() >= 10);
        assert!(tools.iter().any(|t| t.name == "get_timeline"));
        assert!(
            tools
                .iter()
                .any(|t| t.name == "split_clip" && t.requires_approval)
        );
        assert!(
            tools
                .iter()
                .any(|t| t.name == "delete_clip" && t.requires_approval)
        );
        assert!(
            tools
                .iter()
                .any(|t| t.name == "export_video" && t.requires_approval)
        );
        for tool in &tools {
            serde_json::to_string(&tool.input_schema).unwrap();
        }
    }

    #[test]
    fn test_executor_reports_timeline() {
        let (mut executor, _, _) = executor_with_clip();
        let res = executor.execute("get_timeline", &json!({}), false).unwrap();
        assert_eq!(res["clips"].as_array().unwrap().len(), 1);
        assert_eq!(res["tracks"][0]["kind"], "video");
        assert_eq!(res["duration_frames"], 90);
    }

    #[test]
    fn test_executor_enforces_approval() {
        let (mut executor, _, _) = executor_with_clip();
        let clip_id = executor.timeline().clips.first().map(Clip::id).unwrap();
        let err = executor.execute(
            "split_clip",
            &json!({ "clip_id": clip_id.to_string(), "split_frame": 10 }),
            false,
        );
        assert!(matches!(err, Err(McpError::ApprovalRequired { .. })));
        assert_eq!(executor.timeline().clips.len(), 1);
    }

    #[test]
    fn test_executor_unknown_tool() {
        let (mut executor, _, _) = executor_with_clip();
        let err = executor.execute("nonexistent_tool", &json!({}), true);
        assert!(matches!(err, Err(McpError::ToolNotFound(_))));
    }

    #[test]
    fn test_executor_add_move_split_delete() {
        let (mut executor, track_id, asset_id) = executor_with_clip();
        let added = executor
            .execute(
                "add_clip",
                &json!({
                    "track_id": track_id.to_string(),
                    "asset_id": asset_id.to_string(),
                    "start_frame": 100,
                    "source_in": 0,
                    "source_out": 30,
                }),
                true,
            )
            .unwrap();
        let added_id = added["clip_id"].as_str().unwrap().to_string();

        executor
            .execute(
                "move_clip",
                &json!({ "clip_id": added_id, "new_start_frame": 200 }),
                true,
            )
            .unwrap();
        assert_eq!(
            i64::from(
                executor
                    .timeline()
                    .clip(Ulid::from_string(&added_id).unwrap())
                    .unwrap()
                    .timeline_start()
            ),
            200
        );

        let split = executor
            .execute(
                "split_clip",
                &json!({ "clip_id": added_id, "split_frame": 210 }),
                true,
            )
            .unwrap();
        let right_id = split["right_clip_id"].as_str().unwrap().to_string();
        assert_eq!(executor.timeline().clips.len(), 3);
        assert!(
            executor
                .timeline()
                .clip(Ulid::from_string(&right_id).unwrap())
                .is_some()
        );

        executor
            .execute("delete_clip", &json!({ "clip_id": right_id }), true)
            .unwrap();
        assert_eq!(executor.timeline().clips.len(), 2);
    }

    #[test]
    fn test_executor_rejects_invalid_edit_atomically() {
        let (mut executor, _, _) = executor_with_clip();
        let err = executor.execute(
            "add_clip",
            &json!({
                "track_id": Ulid::generate().to_string(),
                "asset_id": Ulid::generate().to_string(),
                "start_frame": 0,
                "source_in": 0,
                "source_out": 10,
            }),
            true,
        );
        assert!(matches!(err, Err(McpError::InvalidArguments { .. })));
        assert_eq!(executor.timeline().clips.len(), 1);
    }

    #[test]
    fn test_executor_selection_snapshot() {
        let (executor, _, _) = executor_with_clip();
        let clip_id = executor.timeline().clips[0].id();
        let mut executor = executor.with_selection(HashSet::from([clip_id]));
        let res = executor
            .execute("get_selected_clips", &json!({}), false)
            .unwrap();
        assert_eq!(res["clips"].as_array().unwrap().len(), 1);
    }
}
