//! Tool execution dispatcher translating MCP tool calls into queries and EditorCommands.

use std::str::FromStr;

use nodavideo_command_system::{
    AddClip, AddComposition, AddTextOverlay, AddTransition, CommandHistory, DeleteClip,
    EditorCommand, GenerateCaptions, MoveClip, SetClipAudio, SetClipTransform, SplitClip,
    StagedChangeset, TrimClip,
};
use nodavideo_hyperframes_bridge::{Composition, CompositionRenderer, TemplateRegistry};
use nodavideo_project::{AssetId, MediaType, ProjectDocument};
use nodavideo_timeline::{
    Clip, ClipId, Sequence, TextAlignment, TextStyle, TrackId, Transition, TransitionAlignment,
    TransitionId, TransitionType,
};

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

            "preview_changeset" => {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Proposed Changeset");
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());
                let commands_arr =
                    args.get("commands")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'commands' array".into(),
                        })?;

                let mut staged = StagedChangeset::new(title, description);
                if let Some(s) = skill_id {
                    staged = staged.with_skill(s);
                }

                let mut speculative = sequence.clone();
                for cmd_json in commands_arr {
                    let cmd = parse_command_spec(cmd_json, &speculative)?;
                    cmd.apply(&mut speculative)
                        .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;
                    staged.add_command(cmd);
                }

                let diff = staged
                    .compute_diff(sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "preview",
                    "changeset_id": staged.id,
                    "title": staged.title,
                    "description": staged.description,
                    "skill_id": staged.skill_id,
                    "diff": {
                        "original_duration_frames": diff.original_duration_frames,
                        "staged_duration_frames": diff.staged_duration_frames,
                        "original_clip_count": diff.original_clip_count,
                        "staged_clip_count": diff.staged_clip_count,
                        "commands_count": diff.commands_count,
                        "tracks_affected": diff.tracks_affected
                    }
                }))
            }

            "propose_changeset" => {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Proposed Changeset");
                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let skill_id = args.get("skill_id").and_then(|v| v.as_str());
                let commands_arr =
                    args.get("commands")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'commands' array".into(),
                        })?;

                let mut staged = StagedChangeset::new(title, description);
                if let Some(s) = skill_id {
                    staged = staged.with_skill(s);
                }

                let mut speculative = sequence.clone();
                for cmd_json in commands_arr {
                    let cmd = parse_command_spec(cmd_json, &speculative)?;
                    cmd.apply(&mut speculative)
                        .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;
                    staged.add_command(cmd);
                }

                let diff = staged
                    .compute_diff(sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                // Commit the staged changeset atomically into history
                history
                    .commit_staged(staged.clone(), sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "committed",
                    "changeset_id": staged.id,
                    "title": staged.title,
                    "commands_executed": staged.commands.len(),
                    "new_duration_frames": sequence.duration_frames(),
                    "diff": {
                        "original_duration_frames": diff.original_duration_frames,
                        "staged_duration_frames": diff.staged_duration_frames,
                        "original_clip_count": diff.original_clip_count,
                        "staged_clip_count": diff.staged_clip_count,
                        "commands_count": diff.commands_count,
                        "tracks_affected": diff.tracks_affected
                    }
                }))
            }

            "list_composition_templates" => {
                let registry = TemplateRegistry::builtin();
                let templates = registry
                    .list()
                    .into_iter()
                    .map(|t| {
                        serde_json::json!({
                            "id": t.id,
                            "name": t.name,
                            "category": t.category,
                            "default_duration_frames": t.default_duration_frames,
                            "parameters": t.parameters.iter().map(|p| {
                                serde_json::json!({
                                    "name": p.name,
                                    "label": p.label,
                                    "description": p.description,
                                    "default": p.default_value
                                })
                            }).collect::<Vec<_>>()
                        })
                    })
                    .collect::<Vec<_>>();

                Ok(serde_json::json!({ "templates": templates }))
            }

            "add_composition" => {
                let template_id = args
                    .get("template_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'template_id'".into(),
                    })?;
                let track_id_str =
                    args.get("track_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'track_id'".into(),
                        })?;
                let start_frame = args
                    .get("start_frame")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'start_frame'".into(),
                    })?;

                let track_id =
                    TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid track UUID: {e}"),
                    })?;

                let registry = TemplateRegistry::builtin();
                let template =
                    registry
                        .get(template_id)
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: format!("Unknown template ID: '{template_id}'"),
                        })?;

                let comp_name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&template.name);

                let mut comp = Composition::new(template, comp_name);
                if let Some(params_map) = args.get("parameters").and_then(|v| v.as_object()) {
                    for (k, v) in params_map {
                        comp.set_parameter(k, v.clone());
                    }
                }

                let renderer = CompositionRenderer::new();
                let tmp_dir = tempfile::tempdir().map_err(|e| {
                    McpError::ExecutionFailed(format!("Failed to create temp dir: {e}"))
                })?;
                let asset_path = renderer
                    .render_media_asset(&mut comp, &registry, tmp_dir.path())
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                let clip_id = ClipId::new();
                let asset_id = AssetId::new();
                let duration = comp.duration_frames;
                let clip = Clip::new(
                    clip_id,
                    asset_id,
                    track_id,
                    comp_name,
                    start_frame,
                    0,
                    duration,
                )
                .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                let cmd = EditorCommand::AddComposition(AddComposition {
                    track_id,
                    clip,
                    composition_id: comp.id.to_string(),
                    template_id: template.id.clone(),
                });

                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "clip_id": clip_id.to_string(),
                    "composition_id": comp.id.to_string(),
                    "template_id": template.id,
                    "rendered_asset": asset_path.display().to_string(),
                    "duration_frames": duration
                }))
            }

            "set_clip_transform" => {
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
                        details: format!("Invalid clip_id: {e}"),
                    })?;

                let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Clip '{clip_id}' not found"))
                })?;
                let clip = track.get_clip(clip_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Clip '{clip_id}' not found"))
                })?;

                let old_transform = clip.transform;
                let mut new_transform = old_transform;

                if let Some(px) = args.get("position_x").and_then(|v| v.as_f64()) {
                    new_transform.position_x = px;
                }
                if let Some(py) = args.get("position_y").and_then(|v| v.as_f64()) {
                    new_transform.position_y = py;
                }
                if let Some(sx) = args.get("scale_x").and_then(|v| v.as_f64()) {
                    new_transform.scale_x = sx;
                }
                if let Some(sy) = args.get("scale_y").and_then(|v| v.as_f64()) {
                    new_transform.scale_y = sy;
                }
                if let Some(rot) = args.get("rotation").and_then(|v| v.as_f64()) {
                    new_transform.rotation_degrees = rot;
                }
                if let Some(op) = args.get("opacity").and_then(|v| v.as_f64()) {
                    new_transform.opacity = op;
                }
                new_transform = new_transform.clamped();

                let cmd = EditorCommand::SetClipTransform(SetClipTransform {
                    clip_id,
                    old_transform,
                    new_transform,
                });

                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "clip_id": clip_id.to_string(),
                    "transform": {
                        "position_x": new_transform.position_x,
                        "position_y": new_transform.position_y,
                        "scale_x": new_transform.scale_x,
                        "scale_y": new_transform.scale_y,
                        "rotation": new_transform.rotation_degrees,
                        "opacity": new_transform.opacity,
                    }
                }))
            }

            "set_clip_audio" => {
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
                        details: format!("Invalid clip_id: {e}"),
                    })?;

                let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Clip '{clip_id}' not found"))
                })?;
                let clip = track.get_clip(clip_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Clip '{clip_id}' not found"))
                })?;

                let old_audio = clip.audio_params;
                let mut new_audio = old_audio;

                if let Some(vol) = args.get("volume_db").and_then(|v| v.as_f64()) {
                    new_audio.volume_db = vol;
                }
                if let Some(pan) = args.get("pan").and_then(|v| v.as_f64()) {
                    new_audio.pan = pan;
                }
                new_audio = new_audio.clamped();

                let cmd = EditorCommand::SetClipAudio(SetClipAudio {
                    clip_id,
                    old_audio,
                    new_audio,
                });

                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "clip_id": clip_id.to_string(),
                    "audio": {
                        "volume_db": new_audio.volume_db,
                        "pan": new_audio.pan,
                    }
                }))
            }

            "add_transition" => {
                let track_id_str =
                    args.get("track_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'track_id'".into(),
                        })?;
                let from_clip_id_str = args
                    .get("from_clip_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'from_clip_id'".into(),
                    })?;
                let to_clip_id_str =
                    args.get("to_clip_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'to_clip_id'".into(),
                        })?;

                let track_id =
                    TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid track UUID: {e}"),
                    })?;
                let from_clip_id =
                    ClipId::from_str(from_clip_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid from_clip_id: {e}"),
                    })?;
                let to_clip_id =
                    ClipId::from_str(to_clip_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid to_clip_id: {e}"),
                    })?;

                let trans_type_str = args
                    .get("transition_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("crossfade");
                let transition_type = match trans_type_str {
                    "dissolve" => TransitionType::Dissolve,
                    "fade_to_black" => TransitionType::FadeToBlack,
                    "dip_to_white" => TransitionType::DipToWhite,
                    "wipe_left" => TransitionType::WipeLeft,
                    "wipe_right" => TransitionType::WipeRight,
                    _ => TransitionType::Crossfade,
                };

                let alignment_str = args
                    .get("alignment")
                    .and_then(|v| v.as_str())
                    .unwrap_or("centered");
                let alignment = match alignment_str {
                    "start_at_cut" => TransitionAlignment::StartAtCut,
                    "end_at_cut" => TransitionAlignment::EndAtCut,
                    _ => TransitionAlignment::Centered,
                };

                let duration_frames = args
                    .get("duration_frames")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(24);

                let track = sequence.get_track(track_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Track '{track_id}' not found"))
                })?;
                let from_clip = track.get_clip(from_clip_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Clip '{from_clip_id}' not found"))
                })?;
                let cut_frame = from_clip.end_frame();

                let transition_id = TransitionId::new();
                let transition = Transition::new(
                    transition_id,
                    transition_type,
                    alignment,
                    from_clip_id,
                    to_clip_id,
                    track_id,
                    cut_frame,
                    duration_frames,
                )
                .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                let cmd = EditorCommand::AddTransition(AddTransition {
                    track_id,
                    transition,
                });

                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "transition_id": transition_id.to_string(),
                    "track_id": track_id.to_string(),
                    "cut_frame": cut_frame,
                    "duration_frames": duration_frames,
                    "transition_type": trans_type_str,
                }))
            }

            "add_text_overlay" => {
                let track_id_str =
                    args.get("track_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'track_id'".into(),
                        })?;
                let text = args.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                    McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'text'".into(),
                    }
                })?;
                let start_frame = args
                    .get("start_frame")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: "Missing 'start_frame'".into(),
                    })?;
                let duration_frames = args
                    .get("duration_frames")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(72);

                let track_id =
                    TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid track UUID: {e}"),
                    })?;

                let font_family = args
                    .get("font_family")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Inter")
                    .to_string();
                let font_size = args.get("font_size").and_then(|v| v.as_u64()).unwrap_or(48) as u32;
                let color = args
                    .get("color")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#ffffff")
                    .to_string();
                let background_color = args
                    .get("background_color")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let alignment_str = args
                    .get("alignment")
                    .and_then(|v| v.as_str())
                    .unwrap_or("bottom");
                let alignment = match alignment_str {
                    "top" => TextAlignment::Top,
                    "center" => TextAlignment::Center,
                    _ => TextAlignment::Bottom,
                };

                let style = TextStyle {
                    font_family,
                    font_size,
                    color,
                    background_color,
                    stroke_color: Some("#000000".into()),
                    stroke_width: 2,
                    alignment,
                };

                let clip_id = ClipId::new();
                let clip = Clip::new_text_overlay(
                    clip_id,
                    track_id,
                    text,
                    start_frame,
                    duration_frames,
                    style,
                )
                .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                let cmd = EditorCommand::AddTextOverlay(AddTextOverlay { track_id, clip });
                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "clip_id": clip_id.to_string(),
                    "track_id": track_id.to_string(),
                    "start_frame": start_frame,
                    "duration_frames": duration_frames,
                    "text": text,
                }))
            }

            "generate_captions" => {
                let track_id_str =
                    args.get("track_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'track_id'".into(),
                        })?;
                let track_id =
                    TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                        tool: tool_name.into(),
                        details: format!("Invalid track UUID: {e}"),
                    })?;

                let captions_arr =
                    args.get("captions")
                        .and_then(|v| v.as_array())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Missing 'captions' array".into(),
                        })?;

                let font_size = args.get("font_size").and_then(|v| v.as_u64()).unwrap_or(36) as u32;
                let color = args
                    .get("color")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#ffffff")
                    .to_string();

                let mut clips = Vec::new();
                for item in captions_arr {
                    let text = item.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                        McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Caption missing 'text'".into(),
                        }
                    })?;
                    let start_frame = item
                        .get("start_frame")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: "Caption missing 'start_frame'".into(),
                        })?;
                    let end_frame =
                        item.get("end_frame")
                            .and_then(|v| v.as_i64())
                            .ok_or_else(|| McpError::InvalidArguments {
                                tool: tool_name.into(),
                                details: "Caption missing 'end_frame'".into(),
                            })?;
                    let duration_frames = end_frame - start_frame;
                    if duration_frames <= 0 {
                        return Err(McpError::InvalidArguments {
                            tool: tool_name.into(),
                            details: format!("Invalid caption timing: {start_frame}..{end_frame}"),
                        });
                    }

                    let style = TextStyle {
                        font_family: "Inter".into(),
                        font_size,
                        color: color.clone(),
                        background_color: Some("#000000b0".into()),
                        stroke_color: Some("#000000".into()),
                        stroke_width: 2,
                        alignment: TextAlignment::Bottom,
                    };
                    let clip_id = ClipId::new();
                    let clip = Clip::new_text_overlay(
                        clip_id,
                        track_id,
                        text,
                        start_frame,
                        duration_frames,
                        style,
                    )
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                    clips.push(clip);
                }

                let clip_count = clips.len();
                let cmd = EditorCommand::GenerateCaptions(GenerateCaptions { track_id, clips });

                history
                    .execute(cmd, sequence)
                    .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                Ok(serde_json::json!({
                    "status": "success",
                    "track_id": track_id.to_string(),
                    "captions_generated": clip_count,
                }))
            }

            _ => Err(McpError::ToolNotFound(tool_name.to_string())),
        }
    }
}

fn parse_command_spec(
    spec: &serde_json::Value,
    sequence: &Sequence,
) -> Result<EditorCommand, McpError> {
    let tool = spec
        .get("tool")
        .or_else(|| spec.get("name"))
        .or_else(|| spec.get("type"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidArguments {
            tool: "propose_changeset".into(),
            details: "Command spec missing 'tool'".into(),
        })?;

    match tool {
        "add_clip" => {
            let track_id_str = spec
                .get("track_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_clip".into(),
                    details: "Missing 'track_id'".into(),
                })?;
            let asset_id_str = spec
                .get("asset_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_clip".into(),
                    details: "Missing 'asset_id'".into(),
                })?;
            let start_frame = spec
                .get("start_frame")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_clip".into(),
                    details: "Missing 'start_frame'".into(),
                })?;
            let source_in = spec
                .get("source_in")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_clip".into(),
                    details: "Missing 'source_in'".into(),
                })?;
            let source_out = spec
                .get("source_out")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_clip".into(),
                    details: "Missing 'source_out'".into(),
                })?;
            let clip_name = spec.get("name").and_then(|v| v.as_str()).unwrap_or("Clip");

            let track_id =
                TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "add_clip".into(),
                    details: format!("Invalid track UUID: {e}"),
                })?;
            let asset_id =
                AssetId::from_str(asset_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "add_clip".into(),
                    details: format!("Invalid asset UUID: {e}"),
                })?;

            let clip_id = if let Some(id_str) = spec.get("clip_id").and_then(|v| v.as_str()) {
                ClipId::from_str(id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "add_clip".into(),
                    details: format!("Invalid clip UUID: {e}"),
                })?
            } else {
                ClipId::new()
            };

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

            Ok(EditorCommand::AddClip(AddClip { track_id, clip }))
        }

        "move_clip" => {
            let clip_id_str = spec
                .get("clip_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "move_clip".into(),
                    details: "Missing 'clip_id'".into(),
                })?;
            let to_track_str = spec
                .get("to_track_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "move_clip".into(),
                    details: "Missing 'to_track_id'".into(),
                })?;
            let new_start_frame = spec
                .get("new_start_frame")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "move_clip".into(),
                    details: "Missing 'new_start_frame'".into(),
                })?;

            let clip_id =
                ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "move_clip".into(),
                    details: format!("Invalid clip UUID: {e}"),
                })?;
            let to_track =
                TrackId::from_str(to_track_str).map_err(|e| McpError::InvalidArguments {
                    tool: "move_clip".into(),
                    details: format!("Invalid track UUID: {e}"),
                })?;

            let (from_track, old_start_frame) = {
                let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Clip '{clip_id}' not found on any track"))
                })?;
                let clip = sequence.get_clip(clip_id).unwrap();
                (track.id, clip.start_frame)
            };

            Ok(EditorCommand::MoveClip(MoveClip {
                clip_id,
                from_track,
                to_track,
                old_start_frame,
                new_start_frame,
            }))
        }

        "trim_clip" => {
            let clip_id_str = spec
                .get("clip_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "trim_clip".into(),
                    details: "Missing 'clip_id'".into(),
                })?;
            let new_start_frame = spec
                .get("new_start_frame")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "trim_clip".into(),
                    details: "Missing 'new_start_frame'".into(),
                })?;
            let new_source_in = spec
                .get("new_source_in")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "trim_clip".into(),
                    details: "Missing 'new_source_in'".into(),
                })?;
            let new_source_out = spec
                .get("new_source_out")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "trim_clip".into(),
                    details: "Missing 'new_source_out'".into(),
                })?;

            let clip_id =
                ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "trim_clip".into(),
                    details: format!("Invalid clip UUID: {e}"),
                })?;

            let (track_id, old_start_frame, old_source_in, old_source_out) = {
                let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Clip '{clip_id}' not found on any track"))
                })?;
                let clip = sequence.get_clip(clip_id).unwrap();
                (track.id, clip.start_frame, clip.source_in, clip.source_out)
            };

            Ok(EditorCommand::TrimClip(TrimClip {
                clip_id,
                track_id,
                old_start_frame,
                new_start_frame,
                old_source_in,
                new_source_in,
                old_source_out,
                new_source_out,
            }))
        }

        "split_clip" => {
            let clip_id_str = spec
                .get("clip_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "split_clip".into(),
                    details: "Missing 'clip_id'".into(),
                })?;
            let split_frame = spec
                .get("split_frame")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "split_clip".into(),
                    details: "Missing 'split_frame'".into(),
                })?;

            let clip_id =
                ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "split_clip".into(),
                    details: format!("Invalid clip UUID: {e}"),
                })?;

            let (track_id, original_source_out) = {
                let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Clip '{clip_id}' not found on any track"))
                })?;
                let clip = sequence.get_clip(clip_id).unwrap();
                (track.id, clip.source_out)
            };

            let new_clip_id = if let Some(id_str) = spec.get("new_clip_id").and_then(|v| v.as_str())
            {
                ClipId::from_str(id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "split_clip".into(),
                    details: format!("Invalid new_clip_id UUID: {e}"),
                })?
            } else {
                ClipId::new()
            };

            Ok(EditorCommand::SplitClip(SplitClip {
                original_clip_id: clip_id,
                track_id,
                split_frame,
                new_clip_id,
                original_source_out,
            }))
        }

        "delete_clip" => {
            let clip_id_str = spec
                .get("clip_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "delete_clip".into(),
                    details: "Missing 'clip_id'".into(),
                })?;

            let clip_id =
                ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "delete_clip".into(),
                    details: format!("Invalid clip UUID: {e}"),
                })?;

            let (track_id, clip) = {
                let track = sequence.find_track_for_clip(clip_id).ok_or_else(|| {
                    McpError::ExecutionFailed(format!("Clip '{clip_id}' not found on any track"))
                })?;
                let clip = sequence.get_clip(clip_id).unwrap().clone();
                (track.id, clip)
            };

            Ok(EditorCommand::DeleteClip(DeleteClip { track_id, clip }))
        }

        "add_composition" => {
            let template_id = spec
                .get("template_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_composition".into(),
                    details: "Missing 'template_id'".into(),
                })?;
            let track_id_str = spec
                .get("track_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_composition".into(),
                    details: "Missing 'track_id'".into(),
                })?;
            let start_frame = spec
                .get("start_frame")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_composition".into(),
                    details: "Missing 'start_frame'".into(),
                })?;

            let track_id =
                TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "add_composition".into(),
                    details: format!("Invalid track UUID: {e}"),
                })?;

            let registry = TemplateRegistry::builtin();
            let template = registry
                .get(template_id)
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_composition".into(),
                    details: format!("Unknown template ID: '{template_id}'"),
                })?;

            let comp_name = spec
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&template.name);

            let mut comp = Composition::new(template, comp_name);
            if let Some(params_map) = spec.get("parameters").and_then(|v| v.as_object()) {
                for (k, v) in params_map {
                    comp.set_parameter(k, v.clone());
                }
            }

            let clip_id = ClipId::new();
            let asset_id = AssetId::new();
            let duration = comp.duration_frames;
            let clip = Clip::new(
                clip_id,
                asset_id,
                track_id,
                comp_name,
                start_frame,
                0,
                duration,
            )
            .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

            Ok(EditorCommand::AddComposition(AddComposition {
                track_id,
                clip,
                composition_id: comp.id.to_string(),
                template_id: template.id.clone(),
            }))
        }

        "set_clip_transform" => {
            let clip_id_str = spec
                .get("clip_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "set_clip_transform".into(),
                    details: "Missing 'clip_id'".into(),
                })?;
            let clip_id =
                ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "set_clip_transform".into(),
                    details: format!("Invalid clip_id: {e}"),
                })?;
            let track = sequence
                .find_track_for_clip(clip_id)
                .ok_or_else(|| McpError::ExecutionFailed(format!("Clip '{clip_id}' not found")))?;
            let clip = track
                .get_clip(clip_id)
                .ok_or_else(|| McpError::ExecutionFailed(format!("Clip '{clip_id}' not found")))?;

            let old_transform = clip.transform;
            let mut new_transform = old_transform;

            if let Some(px) = spec.get("position_x").and_then(|v| v.as_f64()) {
                new_transform.position_x = px;
            }
            if let Some(py) = spec.get("position_y").and_then(|v| v.as_f64()) {
                new_transform.position_y = py;
            }
            if let Some(sx) = spec.get("scale_x").and_then(|v| v.as_f64()) {
                new_transform.scale_x = sx;
            }
            if let Some(sy) = spec.get("scale_y").and_then(|v| v.as_f64()) {
                new_transform.scale_y = sy;
            }
            if let Some(rot) = spec.get("rotation").and_then(|v| v.as_f64()) {
                new_transform.rotation_degrees = rot;
            }
            if let Some(op) = spec.get("opacity").and_then(|v| v.as_f64()) {
                new_transform.opacity = op;
            }
            new_transform = new_transform.clamped();

            Ok(EditorCommand::SetClipTransform(SetClipTransform {
                clip_id,
                old_transform,
                new_transform,
            }))
        }

        "set_clip_audio" => {
            let clip_id_str = spec
                .get("clip_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "set_clip_audio".into(),
                    details: "Missing 'clip_id'".into(),
                })?;
            let clip_id =
                ClipId::from_str(clip_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "set_clip_audio".into(),
                    details: format!("Invalid clip_id: {e}"),
                })?;
            let track = sequence
                .find_track_for_clip(clip_id)
                .ok_or_else(|| McpError::ExecutionFailed(format!("Clip '{clip_id}' not found")))?;
            let clip = track
                .get_clip(clip_id)
                .ok_or_else(|| McpError::ExecutionFailed(format!("Clip '{clip_id}' not found")))?;

            let old_audio = clip.audio_params;
            let mut new_audio = old_audio;
            if let Some(vol) = spec.get("volume_db").and_then(|v| v.as_f64()) {
                new_audio.volume_db = vol;
            }
            if let Some(pan) = spec.get("pan").and_then(|v| v.as_f64()) {
                new_audio.pan = pan;
            }
            new_audio = new_audio.clamped();

            Ok(EditorCommand::SetClipAudio(SetClipAudio {
                clip_id,
                old_audio,
                new_audio,
            }))
        }

        "add_transition" => {
            let track_id_str = spec
                .get("track_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_transition".into(),
                    details: "Missing 'track_id'".into(),
                })?;
            let from_clip_id_str = spec
                .get("from_clip_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_transition".into(),
                    details: "Missing 'from_clip_id'".into(),
                })?;
            let to_clip_id_str =
                spec.get("to_clip_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: "add_transition".into(),
                        details: "Missing 'to_clip_id'".into(),
                    })?;

            let track_id =
                TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "add_transition".into(),
                    details: format!("Invalid track UUID: {e}"),
                })?;
            let from_clip_id =
                ClipId::from_str(from_clip_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "add_transition".into(),
                    details: format!("Invalid from_clip_id: {e}"),
                })?;
            let to_clip_id =
                ClipId::from_str(to_clip_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "add_transition".into(),
                    details: format!("Invalid to_clip_id: {e}"),
                })?;

            let trans_type_str = spec
                .get("transition_type")
                .and_then(|v| v.as_str())
                .unwrap_or("crossfade");
            let transition_type = match trans_type_str {
                "dissolve" => TransitionType::Dissolve,
                "fade_to_black" => TransitionType::FadeToBlack,
                "dip_to_white" => TransitionType::DipToWhite,
                "wipe_left" => TransitionType::WipeLeft,
                "wipe_right" => TransitionType::WipeRight,
                _ => TransitionType::Crossfade,
            };

            let alignment_str = spec
                .get("alignment")
                .and_then(|v| v.as_str())
                .unwrap_or("centered");
            let alignment = match alignment_str {
                "start_at_cut" => TransitionAlignment::StartAtCut,
                "end_at_cut" => TransitionAlignment::EndAtCut,
                _ => TransitionAlignment::Centered,
            };

            let duration_frames = spec
                .get("duration_frames")
                .and_then(|v| v.as_i64())
                .unwrap_or(24);

            let track = sequence.get_track(track_id).ok_or_else(|| {
                McpError::ExecutionFailed(format!("Track '{track_id}' not found"))
            })?;
            let from_clip = track.get_clip(from_clip_id).ok_or_else(|| {
                McpError::ExecutionFailed(format!("Clip '{from_clip_id}' not found"))
            })?;
            let cut_frame = from_clip.end_frame();

            let transition = Transition::new(
                TransitionId::new(),
                transition_type,
                alignment,
                from_clip_id,
                to_clip_id,
                track_id,
                cut_frame,
                duration_frames,
            )
            .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

            Ok(EditorCommand::AddTransition(AddTransition {
                track_id,
                transition,
            }))
        }

        "add_text_overlay" => {
            let track_id_str = spec
                .get("track_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_text_overlay".into(),
                    details: "Missing 'track_id'".into(),
                })?;
            let text = spec.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                McpError::InvalidArguments {
                    tool: "add_text_overlay".into(),
                    details: "Missing 'text'".into(),
                }
            })?;
            let start_frame = spec
                .get("start_frame")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "add_text_overlay".into(),
                    details: "Missing 'start_frame'".into(),
                })?;
            let duration_frames = spec
                .get("duration_frames")
                .and_then(|v| v.as_i64())
                .unwrap_or(72);

            let track_id =
                TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "add_text_overlay".into(),
                    details: format!("Invalid track UUID: {e}"),
                })?;

            let font_family = spec
                .get("font_family")
                .and_then(|v| v.as_str())
                .unwrap_or("Inter")
                .to_string();
            let font_size = spec.get("font_size").and_then(|v| v.as_u64()).unwrap_or(48) as u32;
            let color = spec
                .get("color")
                .and_then(|v| v.as_str())
                .unwrap_or("#ffffff")
                .to_string();
            let background_color = spec
                .get("background_color")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let alignment_str = spec
                .get("alignment")
                .and_then(|v| v.as_str())
                .unwrap_or("bottom");
            let alignment = match alignment_str {
                "top" => TextAlignment::Top,
                "center" => TextAlignment::Center,
                _ => TextAlignment::Bottom,
            };

            let style = TextStyle {
                font_family,
                font_size,
                color,
                background_color,
                stroke_color: Some("#000000".into()),
                stroke_width: 2,
                alignment,
            };

            let clip_id = ClipId::new();
            let clip = Clip::new_text_overlay(
                clip_id,
                track_id,
                text,
                start_frame,
                duration_frames,
                style,
            )
            .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

            Ok(EditorCommand::AddTextOverlay(AddTextOverlay {
                track_id,
                clip,
            }))
        }

        "generate_captions" => {
            let track_id_str = spec
                .get("track_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::InvalidArguments {
                    tool: "generate_captions".into(),
                    details: "Missing 'track_id'".into(),
                })?;
            let track_id =
                TrackId::from_str(track_id_str).map_err(|e| McpError::InvalidArguments {
                    tool: "generate_captions".into(),
                    details: format!("Invalid track UUID: {e}"),
                })?;

            let captions_arr =
                spec.get("captions")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: "generate_captions".into(),
                        details: "Missing 'captions' array".into(),
                    })?;

            let font_size = spec.get("font_size").and_then(|v| v.as_u64()).unwrap_or(36) as u32;
            let color = spec
                .get("color")
                .and_then(|v| v.as_str())
                .unwrap_or("#ffffff")
                .to_string();

            let mut clips = Vec::new();
            for item in captions_arr {
                let text = item.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                    McpError::InvalidArguments {
                        tool: "generate_captions".into(),
                        details: "Caption missing 'text'".into(),
                    }
                })?;
                let start_frame = item
                    .get("start_frame")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::InvalidArguments {
                        tool: "generate_captions".into(),
                        details: "Caption missing 'start_frame'".into(),
                    })?;
                let end_frame =
                    item.get("end_frame")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| McpError::InvalidArguments {
                            tool: "generate_captions".into(),
                            details: "Caption missing 'end_frame'".into(),
                        })?;
                let duration_frames = end_frame - start_frame;
                if duration_frames <= 0 {
                    return Err(McpError::InvalidArguments {
                        tool: "generate_captions".into(),
                        details: format!("Invalid caption timing: {start_frame}..{end_frame}"),
                    });
                }

                let style = TextStyle {
                    font_family: "Inter".into(),
                    font_size,
                    color: color.clone(),
                    background_color: Some("#000000b0".into()),
                    stroke_color: Some("#000000".into()),
                    stroke_width: 2,
                    alignment: TextAlignment::Bottom,
                };
                let clip_id = ClipId::new();
                let clip = Clip::new_text_overlay(
                    clip_id,
                    track_id,
                    text,
                    start_frame,
                    duration_frames,
                    style,
                )
                .map_err(|e| McpError::ExecutionFailed(e.to_string()))?;

                clips.push(clip);
            }

            Ok(EditorCommand::GenerateCaptions(GenerateCaptions {
                track_id,
                clips,
            }))
        }

        other => Err(McpError::ToolNotFound(format!(
            "Unsupported changeset command tool: {other}"
        ))),
    }
}
