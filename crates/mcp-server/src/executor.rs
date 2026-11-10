//! Tool execution dispatcher translating MCP tool calls into queries and EditorCommands.

use std::str::FromStr;

use nodavideo_command_system::{
    AddClip, CommandHistory, DeleteClip, EditorCommand, MoveClip, SplitClip, TrimClip,
};
use nodavideo_project::{AssetId, MediaType, ProjectDocument};
use nodavideo_timeline::{Clip, ClipId, Sequence, TrackId};

use crate::error::McpError;
use crate::schema::McpToolDefinition;

/// Dispatches typed MCP tool invocations against editor project and timeline state.
#[derive(Debug, Clone, Copy, Default)]
pub struct McpExecutor;

impl McpExecutor {
    /// Execute an MCP tool call against the given project and sequence state.
    ///
    /// If the tool mutates state, it constructs and executes an `EditorCommand`
    /// through `CommandHistory`, ensuring deterministic application and undo/redo support.
    pub fn execute(
        tool_name: &str,
        args: &serde_json::Value,
        project: &ProjectDocument,
        sequence: &mut Sequence,
        history: &mut CommandHistory,
        approved: bool,
    ) -> Result<serde_json::Value, McpError> {
        let tools = McpToolDefinition::all_tools();
        let def = tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| McpError::ToolNotFound(tool_name.to_string()))?;

        if def.requires_approval && !approved {
            return Err(McpError::ApprovalRequired {
                tool: tool_name.to_string(),
                reason: format!("Operation '{tool_name}' requires explicit user confirmation"),
            });
        }

        match tool_name {
            "get_project" => Ok(serde_json::json!({
                "id": project.id.to_string(),
                "name": project.name,
                "asset_count": project.assets.len(),
                "duration_frames": sequence.duration_frames(),
                "duration_timecode": sequence.duration_timecode(),
                "fps": sequence.fps.to_f64(),
            })),

            "get_timeline" => {
                let tracks: Vec<_> = sequence
                    .tracks
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "id": t.id.to_string(),
                            "name": t.name,
                            "type": format!("{:?}", t.track_type).to_lowercase(),
                            "clip_count": t.clips.len(),
                            "clips": t.clips.iter().map(|c| {
                                serde_json::json!({
                                    "id": c.id.to_string(),
                                    "name": c.name,
                                    "start_frame": c.start_frame,
                                    "duration_frames": c.duration_frames(),
                                    "source_in": c.source_in,
                                    "source_out": c.source_out,
                                })
                            }).collect::<Vec<_>>()
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "sequence_id": sequence.id.to_string(),
                    "name": sequence.name,
                    "fps": sequence.fps.to_f64(),
                    "width": sequence.width,
                    "height": sequence.height,
                    "duration_frames": sequence.duration_frames(),
                    "duration_timecode": sequence.duration_timecode(),
                    "tracks": tracks
                }))
            }

            "get_selected_clips" => {
                // Return clips with selected flag if any
                Ok(serde_json::json!({
                    "selected_clips": []
                }))
            }

            "search_media" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let type_filter = args.get("media_type").and_then(|v| v.as_str());

                let matching: Vec<_> = project
                    .assets
                    .iter()
                    .filter(|a| {
                        if !query.is_empty()
                            && !a
                                .file_path
                                .display()
                                .to_string()
                                .to_lowercase()
                                .contains(&query)
                        {
                            return false;
                        }
                        if let Some(tf) = type_filter {
                            matches!(
                                (tf, a.media_type),
                                ("video", MediaType::Video)
                                    | ("audio", MediaType::Audio)
                                    | ("image", MediaType::Image)
                            )
                        } else {
                            true
                        }
                    })
                    .map(|a| {
                        serde_json::json!({
                            "id": a.id.to_string(),
                            "path": a.file_path.display().to_string(),
                            "type": format!("{:?}", a.media_type).to_lowercase(),
                            "duration_seconds": a.duration.map(|d| d.to_seconds_f64()),
                        })
                    })
                    .collect();

                Ok(serde_json::json!({ "results": matching }))
            }

            "add_clip" => {
                let track_id_str =
                    args.get("track_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'track_id'".into(),
                        })?;
                let asset_id_str =
                    args.get("asset_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'asset_id'".into(),
                        })?;
                let start_frame = args
                    .get("start_frame")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'start_frame'".into(),
                    })?;
                let source_in =
                    args.get("source_in")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'source_in'".into(),
                        })?;
                let source_out =
                    args.get("source_out")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'source_out'".into(),
                        })?;
                let clip_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("Clip");

                let track_id =
                    TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid track UUID: {e}"),
                    })?;
                let asset_id =
                    AssetId::from_str(asset_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid asset UUID: {e}"),
                    })?;

                let clip_id = ClipId::new();
                let clip = Clip::new(
                    clip_id,
                    asset_id,
                    track_id,
                    clip_name,
                    start_frame,
                    source_in,
                    source_out,
                )
                .map_err(|e| McpError::ExecutionFailed(format!("Invalid clip parameters: {e}")))?;

                let cmd = EditorCommand::AddClip(AddClip { track_id, clip });
                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "clip_id": clip_id.to_string(),
                    "track_id": track_id.to_string(),
                    "start_frame": start_frame,
                    "duration_frames": source_out - source_in
                }))
            }

            "move_clip" => {
                let clip_id_str =
                    args.get("clip_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'clip_id'".into(),
                        })?;
                let new_start_frame = args
                    .get("new_start_frame")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'new_start_frame'".into(),
                    })?;
                let new_track_id = match args.get("new_track_id").and_then(|v| v.as_str()) {
                    Some(s) => {
                        Some(
                            TrackId::from_str(s).map_err(|e| McpError::InvalidArguments {
                                tool: tool_name.into(),
                                details: format!("Invalid track UUID: {e}"),
                            })?,
                        )
                    }
                    None => None,
                };

                let clip_id =
                    ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid clip UUID: {e}"),
                    })?;

                let (from_track, old_start_frame) = {
                    let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                        McpError::ExecutionFailed(format!(
                            "Clip '{clip_id}' not found on any track"
                        ))
                    })?;
                    let clip = sequence.get_clip(clip_id).unwrap();
                    (track.id, clip.start_frame)
                };
                let to_track = new_track_id.unwrap_or(from_track);

                let cmd = EditorCommand::MoveClip(MoveClip {
                    clip_id,
                    from_track,
                    to_track,
                    old_start_frame,
                    new_start_frame,
                });

                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "clip_id": clip_id.to_string(),
                    "new_start_frame": new_start_frame
                }))
            }

            "trim_clip" => {
                let clip_id_str =
                    args.get("clip_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'clip_id'".into(),
                        })?;
                let new_start_frame = args
                    .get("new_start_frame")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'new_start_frame'".into(),
                    })?;
                let new_source_in = args
                    .get("new_source_in")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'new_source_in'".into(),
                    })?;
                let new_source_out = args
                    .get("new_source_out")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'new_source_out'".into(),
                    })?;

                let clip_id =
                    ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid clip UUID: {e}"),
                    })?;

                let (track_id, old_start_frame, old_source_in, old_source_out) = {
                    let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                        McpError::ExecutionFailed(format!(
                            "Clip '{clip_id}' not found on any track"
                        ))
                    })?;
                    let clip = sequence.get_clip(clip_id).unwrap();
                    (track.id, clip.start_frame, clip.source_in, clip.source_out)
                };

                let cmd = EditorCommand::TrimClip(TrimClip {
                    clip_id,
                    track_id,
                    old_start_frame,
                    new_start_frame,
                    old_source_in,
                    new_source_in,
                    old_source_out,
                    new_source_out,
                });

                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "clip_id": clip_id.to_string(),
                    "duration_frames": new_source_out - new_source_in
                }))
            }

            "split_clip" => {
                let clip_id_str =
                    args.get("clip_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'clip_id'".into(),
                        })?;
                let split_frame = args
                    .get("split_frame")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'split_frame'".into(),
                    })?;

                let clip_id =
                    ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid clip UUID: {e}"),
                    })?;

                let (track_id, original_source_out) = {
                    let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                        McpError::ExecutionFailed(format!(
                            "Clip '{clip_id}' not found on any track"
                        ))
                    })?;
                    let clip = sequence.get_clip(clip_id).unwrap();
                    (track.id, clip.source_out)
                };

                let new_right_clip_id = ClipId::new();
                let cmd = EditorCommand::SplitClip(SplitClip {
                    original_clip_id: clip_id,
                    track_id,
                    split_frame,
                    new_clip_id: new_right_clip_id,
                    original_source_out,
                });

                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "original_clip_id": clip_id.to_string(),
                    "new_clip_id": new_right_clip_id.to_string(),
                    "split_frame": split_frame
                }))
            }

            "delete_clip" => {
                let clip_id_str =
                    args.get("clip_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'clip_id'".into(),
                        })?;

                let clip_id =
                    ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid clip UUID: {e}"),
                    })?;

                let (track_id, clip) = {
                    let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                        McpError::ExecutionFailed(format!(
                            "Clip '{clip_id}' not found on any track"
                        ))
                    })?;
                    let clip = sequence.get_clip(clip_id).unwrap().clone();
                    (track.id, clip)
                };

                let cmd = EditorCommand::DeleteClip(DeleteClip { track_id, clip });

                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "deleted_clip_id": clip_id.to_string()
                }))
            }

            "export_video" => {
                let output_path = args
                    .get("output_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'output_path'".into(),
                    })?;

                Ok(serde_json::json!({
                    "status": "scheduled",
                    "output_path": output_path,
                    "duration_frames": sequence.duration_frames(),
                    "fps": sequence.fps.to_f64()
                }))
            }

            _ => Err(McpError::ToolNotFound(tool_name.to_string())),
        }
    }
}
