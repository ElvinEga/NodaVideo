//! Model Context Protocol (MCP) tool exposure for NodaVideo editor commands.
//!
//! Exposes typed tools that external AI agent CLIs can discover and invoke.
//! All mutations are routed through [`nodavideo_command_system::EditorCommand`]
//! to ensure unified undo/redo history and user approval enforcement.

pub mod error;
pub mod executor;
pub mod schema;

pub use error::McpError;
pub use executor::McpExecutor;
pub use schema::McpToolDefinition;

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_command_system::CommandHistory;
    use nodavideo_project::{AssetId, ProjectDocument};
    use nodavideo_timeline::Sequence;

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
    }

    #[test]
    fn test_mcp_query_tools() {
        let project = ProjectDocument::default();
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(50);

        let res = McpExecutor::execute(
            "get_timeline",
            &serde_json::json!({}),
            &project,
            &mut sequence,
            &mut history,
            true,
        )
        .unwrap();

        assert_eq!(res["name"], "Main Sequence");
        assert_eq!(res["tracks"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn test_mcp_add_and_split_clip_through_history() {
        let project = ProjectDocument::default();
        let mut sequence = Sequence::default_1080p24();
        let mut history = CommandHistory::new(50);

        let v1_id = sequence.tracks[1].id;
        let asset_id = AssetId::new();

        // 1. Add clip
        let add_res = McpExecutor::execute(
            "add_clip",
            &serde_json::json!({
                "track_id": v1_id.to_string(),
                "asset_id": asset_id.to_string(),
                "name": "Clip A",
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

        let clip_id = add_res["clip_id"].as_str().unwrap().to_string();
        assert_eq!(sequence.tracks[1].clips.len(), 1);
        assert_eq!(sequence.duration_frames(), 48);
        assert!(history.can_undo());

        // 2. Attempt split without approval -> should fail
        let split_err = McpExecutor::execute(
            "split_clip",
            &serde_json::json!({
                "clip_id": clip_id,
                "split_frame": 24
            }),
            &project,
            &mut sequence,
            &mut history,
            false,
        );
        assert!(matches!(split_err, Err(McpError::ApprovalRequired { .. })));

        // 3. Split with approval -> should succeed
        let split_res = McpExecutor::execute(
            "split_clip",
            &serde_json::json!({
                "clip_id": clip_id,
                "split_frame": 24
            }),
            &project,
            &mut sequence,
            &mut history,
            true,
        )
        .unwrap();

        assert_eq!(split_res["status"], "success");
        assert_eq!(sequence.tracks[1].clips.len(), 2);

        // 4. Test Undo through history
        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips.len(), 1);

        history.undo(&mut sequence).unwrap();
        assert_eq!(sequence.tracks[1].clips.len(), 0);
    }
}
