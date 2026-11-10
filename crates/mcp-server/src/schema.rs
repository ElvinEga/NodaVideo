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
        vec![
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
                name: "add_clip".into(),
                description: "Add a media asset to a specified timeline track at a given frame position.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "Target track UUID" },
                        "asset_id": { "type": "string", "description": "Media asset UUID to place" },
                        "name": { "type": "string", "description": "Display name of the new clip" },
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
                name: "add_composition".into(),
                description: "Instantiate a motion graphics composition template and place it on a timeline track.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "template_id": { "type": "string", "description": "ID of template (e.g. lower-third-modern, title-minimal, social-caption-card)" },
                        "track_id": { "type": "string", "description": "UUID of video track (recommended: V2 or upper track)" },
                        "start_frame": { "type": "integer", "minimum": 0, "description": "Timeline frame placement position" },
                        "name": { "type": "string", "description": "Optional human-readable title" },
                        "parameters": { "type": "object", "description": "Customizable template parameters (title, subtitle, colors, etc.)" }
                    },
                    "required": ["template_id", "track_id", "start_frame"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "set_clip_transform".into(),
                description: "Update the spatial and visual transform attributes (position, scale, rotation, opacity) of a timeline clip.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "UUID of the clip to update" },
                        "position_x": { "type": "number", "description": "Horizontal position offset in pixels (0.0 = centered)" },
                        "position_y": { "type": "number", "description": "Vertical position offset in pixels (0.0 = centered)" },
                        "scale_x": { "type": "number", "minimum": 0.01, "description": "Horizontal scale multiplier (1.0 = 100%)" },
                        "scale_y": { "type": "number", "minimum": 0.01, "description": "Vertical scale multiplier (1.0 = 100%)" },
                        "rotation": { "type": "number", "description": "Rotation in degrees clockwise" },
                        "opacity": { "type": "number", "minimum": 0.0, "maximum": 1.0, "description": "Visual opacity (0.0 to 1.0)" }
                    },
                    "required": ["clip_id"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "set_clip_audio".into(),
                description: "Update the audio parameters (volume dB gain, stereo pan) of a timeline clip.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "clip_id": { "type": "string", "description": "UUID of the clip to update" },
                        "volume_db": { "type": "number", "minimum": -100.0, "maximum": 24.0, "description": "Audio gain in decibels (0.0 = unity gain, negative = attenuate, positive = boost)" },
                        "pan": { "type": "number", "minimum": -1.0, "maximum": 1.0, "description": "Stereo pan position (-1.0 = full left, 0.0 = center, +1.0 = full right)" }
                    },
                    "required": ["clip_id"],
                    "additionalProperties": false
                }),
                is_mutation: true,
                requires_approval: false,
            },
            Self {
                name: "add_transition".into(),
                description: "Add a visual or audio transition (crossfade, dissolve, fade to black, wipe) across an edit cut between two adjacent clips on a track.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "track_id": { "type": "string", "description": "UUID of track containing the edit cut" },
                        "from_clip_id": { "type": "string", "description": "UUID of the outgoing clip" },
                        "to_clip_id": { "type": "string", "description": "UUID of the incoming clip" },
                        "transition_type": {
                            "type": "string",
                            "enum": ["crossfade", "dissolve", "fade_to_black", "dip_to_white", "wipe_left", "wipe_right"],
                            "description": "Type of transition effect"
                        },
                        "alignment": {
                            "type": "string",
                            "enum": ["centered", "start_at_cut", "end_at_cut"],
                            "description": "Alignment of the transition duration relative to cut frame (default: centered)"
                        },
                        "duration_frames": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Duration of the transition in frames (default: 24)"
                        }
                    },
                    "required": ["track_id", "from_clip_id", "to_clip_id"],
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
        ]
    }
}
