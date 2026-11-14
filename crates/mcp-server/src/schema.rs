//! MCP tool definitions and parameter JSON schemas.

use serde::{Deserialize, Serialize};

/// Definition of an editor tool exposed to AI agents via MCP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolDefinition {
    /// Unique name of the tool (e.g. "get_timeline", "split_clip").
    pub name: String,

    /// Descriptive summary explaining what the tool does and when to call it.
    pub description: String,

    /// JSON Schema describing required and optional parameters.
    pub input_schema: serde_json::Value,

    /// Whether this tool mutates editor project state.
    pub is_mutation: bool,

    /// Whether this tool requires explicit user approval before execution.
    pub requires_approval: bool,
}

impl McpToolDefinition {
    /// Return the list of canonical editor tools supported by NodaVideo.
    pub fn all_tools() -> Vec<Self> {
        let mut tools = vec![
            // Query Tools
            Self {
                name: "get_project".into(),
                description: "Get metadata of the active NodaVideo project, including assets and sequence summary.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "get_timeline".into(),
                description: "Inspect the current multitrack timeline sequence, including all video and audio tracks, clips, and timing.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "get_selected_clips".into(),
                description: "Get currently selected clips on the timeline.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "get_media_items".into(),
                description: "List the imported media library (explorer media bin): id, name, kind, path, dimensions and duration. Independent of what is placed on the timeline.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "search_media".into(),
                description: "Search imported media assets in the project media bin by name or media type.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query filter" },
                        "media_type": { "type": "string", "enum": ["video", "audio", "image"], "description": "Optional media type filter" }
                    },
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            // Mutation Tools
            Self {
                name: "import_media".into(),
                description: "Reference original media files into the project media library without copying them. Probing runs asynchronously in the editor, so this returns import_queued; read the resulting assets (with ids, dimensions and duration) via get_media_items once they appear.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "description": "Absolute paths of existing media files to reference into the library",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["paths"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "remove_media".into(),
                description: "Remove an asset from the project media library. The source file on disk is kept. Rejected when timeline clips still reference the asset.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "asset_id": { "type": "string", "description": "ULID of the library asset to remove (from get_media_items)" }
                    },
                    "required": ["asset_id"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "rename_media".into(),
                description: "Rename an asset's display name in the project media library. The source file on disk is not renamed or moved.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "asset_id": { "type": "string", "description": "ULID of the library asset to rename (from get_media_items)" },
                        "name": { "type": "string", "description": "New display name (non-empty)" }
                    },
                    "required": ["asset_id", "name"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "add_clip".into(),
                description: "Add a media asset to a specified timeline track at a given frame position.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Target track UUID" },
                        "asset_id": { "type": "string", "description": "Media asset UUID to place" },
                        "start_frame": { "type": "integer", "minimum": 0, "description": "Timeline start frame" },
                        "source_in": { "type": "integer", "minimum": 0, "description": "Source asset in-point in frames" },
                        "source_out": { "type": "integer", "minimum": 1, "description": "Source asset out-point in frames" }
                    },
                    "required": ["track_id", "asset_id", "start_frame", "source_in", "source_out"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "move_clip".into(),
                description: "Move an existing timeline clip to a new start frame or track.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "Target clip UUID" },
                        "new_track_id": { "type": "string", "description": "Destination track UUID (or null to keep same track)" },
                        "new_start_frame": { "type": "integer", "minimum": 0, "description": "New timeline start frame position" }
                    },
                    "required": ["clip_id", "new_start_frame"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "trim_clip".into(),
                description: "Trim the start or end frame of an existing timeline clip.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "Target clip UUID" },
                        "new_start_frame": { "type": "integer", "minimum": 0, "description": "New timeline start frame" },
                        "new_source_in": { "type": "integer", "minimum": 0, "description": "New source in-point frame" },
                        "new_source_out": { "type": "integer", "minimum": 1, "description": "New source out-point frame" }
                    },
                    "required": ["clip_id", "new_start_frame", "new_source_in", "new_source_out"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "split_clip".into(),
                description: "Split an existing clip into two adjacent clips at a specified timeline frame.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "UUID of clip to split" },
                        "split_frame": { "type": "integer", "minimum": 0, "description": "Timeline frame position where split occurs" }
                    },
                    "required": ["clip_id", "split_frame"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: true,
            },
            Self {
                name: "delete_clip".into(),
                description: "Delete a clip from the timeline sequence.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "UUID of clip to delete" }
                    },
                    "required": ["clip_id"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: true,
            },
            Self {
                name: "export_video".into(),
                description: "Export the current sequence composition to an MP4 video file.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "output_path": { "type": "string", "description": "Destination file path (.mp4)" }
                    },
                    "required": ["output_path"],
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: true,
            },
            Self {
                name: "preview_changeset".into(),
                description: "Compute a speculative diff for a proposed multi-step editing changeset without committing or modifying timeline state.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Short title of proposed edit" },
                        "description": { "type": "string", "description": "Detailed explanation of changes" },
                        "skill_id": { "type": "string", "description": "Originating skill package ID" },
                        "commands": {
                            "type": "array",
                            "description": "List of command objects specifying tool name and arguments",
                            "items": { "type": "object" }
                        }
                    },
                    "required": ["title", "commands"],
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "propose_changeset".into(),
                description: "Propose a multi-command editing plan (smart cuts, captions, social video) requiring explicit user approval before atomic commit.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Short title of proposed edit" },
                        "description": { "type": "string", "description": "Detailed explanation of changes" },
                        "skill_id": { "type": "string", "description": "Originating skill package ID" },
                        "commands": {
                            "type": "array",
                            "description": "List of command objects specifying tool name and arguments",
                            "items": { "type": "object" }
                        }
                    },
                    "required": ["title", "description", "commands"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: true,
            },
            Self {
                name: "list_composition_templates".into(),
                description: "List available HyperFrames motion graphics templates (lower thirds, titles, captions, transitions) with their parameter schemas.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "list_compositions".into(),
                description: "List the motion graphics compositions saved in the current project, with their template and parameter values.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "create_composition".into(),
                description: "Create a reusable motion graphics composition from a template and save it to the project. Returns its id for editing, rendering and placement. The composition stays editable; it is not flattened to video.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "template_id": { "type": "string", "description": "Template id from list_composition_templates" },
                        "name": { "type": "string", "description": "Display name (defaults to the template name)" }
                    },
                    "required": ["template_id"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "inspect_composition".into(),
                description: "Inspect a project composition: its template, current parameter values, dimensions and duration.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "composition_id": { "type": "string", "description": "ULID of the composition" }
                    },
                    "required": ["composition_id"],
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "edit_composition".into(),
                description: "Update a composition's template parameters (text, colors, numbers). Values are validated against the template before saving, so the source stays editable and consistent.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "composition_id": { "type": "string", "description": "ULID of the composition" },
                        "parameters": {
                            "type": "object",
                            "description": "Parameter name/value pairs to merge (e.g. {\"title\": \"Jane Doe\", \"accent_color\": \"#10b981\"})",
                            "additionalProperties": true
                        }
                    },
                    "required": ["composition_id", "parameters"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "render_composition".into(),
                description: "Render a composition to a transparent PNG cache asset and return its path. Rendering is a cache step; the editable composition source is preserved.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "composition_id": { "type": "string", "description": "ULID of the composition" }
                    },
                    "required": ["composition_id"],
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "validate_composition".into(),
                description: "Validate a composition's parameters against its template schema without rendering or modifying it.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "composition_id": { "type": "string", "description": "ULID of the composition" }
                    },
                    "required": ["composition_id"],
                    "additionalProperties": false
                }),
                is_mutation: false,
                requires_approval: false,
            },
            Self {
                name: "add_composition_to_timeline".into(),
                description: "Place an editable motion graphics composition on the timeline as a clip. Requires user approval. Omit track_id to create a dedicated upper video track; the composition stays editable and is not flattened to video.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "composition_id": { "type": "string", "description": "ULID of the project composition to place" },
                        "start_frame": { "type": "integer", "minimum": 0, "description": "Timeline position in frames where the composition starts" },
                        "track_id": { "type": "string", "description": "Optional ULID of an existing video track; omit to create a dedicated upper video track" },
                        "duration_frames": { "type": "integer", "minimum": 1, "description": "Optional duration in timeline frames; defaults to the composition's native length" }
                    },
                    "required": ["composition_id", "start_frame"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: true,
            },
            Self {
                name: "set_clip_transform".into(),
                description: "Update the transform of a video clip or the position of a text clip.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "ULID of the clip to update" },
                        "position_x": { "type": "number", "description": "Horizontal offset in timeline pixels (0.0 = centered)" },
                        "position_y": { "type": "number", "description": "Vertical offset in timeline pixels (0.0 = centered)" },
                        "scale": { "type": "number", "minimum": 0.01, "description": "Uniform scale multiplier for video clips (1.0 = 100%)" }
                    },
                    "required": ["clip_id"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "set_clip_audio".into(),
                description: "Update the audio gain and mute state of a video or audio clip.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "ULID of the clip to update" },
                        "gain_db": { "type": "number", "minimum": -100.0, "maximum": 24.0, "description": "Audio gain in decibels (0.0 = unity gain)" },
                        "muted": { "type": "boolean", "description": "Whether the clip audio is muted" }
                    },
                    "required": ["clip_id"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "add_text_overlay".into(),
                description: "Add a stylized text overlay, title, or lower-third caption to a video track.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "UUID of track to place text overlay on (recommended: upper video track)" },
                        "text": { "type": "string", "description": "Text content to display" },
                        "start_frame": { "type": "integer", "minimum": 0, "description": "Timeline position in frames where text starts" },
                        "duration_frames": { "type": "integer", "minimum": 1, "description": "Duration in frames (default: 72)" },
                        "font_family": { "type": "string", "description": "Font family name (default: Inter)" },
                        "font_size": { "type": "integer", "minimum": 8, "description": "Font size in canvas pixels (default: 48)" },
                        "color": { "type": "string", "description": "Hex text color (default: #ffffff)" },
                        "background_color": { "type": "string", "description": "Optional background box hex color (e.g. #00000088)" },
                        "alignment": {
                            "type": "string",
                            "enum": ["top", "center", "bottom"],
                            "description": "Screen alignment (default: bottom)"
                        }
                    },
                    "required": ["track_id", "text", "start_frame"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "generate_captions".into(),
                description: "Batch generate timed subtitle caption clips on a track from transcript segments.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "UUID of track for subtitles (e.g. V2)" },
                        "captions": {
                            "type": "array",
                            "description": "Array of timed subtitle captions",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "text": { "type": "string" },
                                    "start_frame": { "type": "integer", "minimum": 0 },
                                    "end_frame": { "type": "integer", "minimum": 1 }
                                },
                                "required": ["text", "start_frame", "end_frame"]
                            }
                        },
                        "font_size": { "type": "integer", "description": "Font size in pixels (default: 36)" },
                        "color": { "type": "string", "description": "Hex text color (default: #ffffff)" }
                    },
                    "required": ["track_id", "captions"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
        ];
        for tool in &mut tools {
            if tool.is_mutation {
                if let Some(properties) = tool
                    .input_schema
                    .get_mut("properties")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    properties.insert(
                        "expected_revision".into(),
                        serde_json::json!({
                            "type": "integer",
                            "minimum": 0,
                            "description": "Optional project_revision from a previous result. The call is rejected as stale if the project has changed since, so the agent never overwrites newer user edits."
                        }),
                    );
                }
            }
        }
        tools
    }
}
