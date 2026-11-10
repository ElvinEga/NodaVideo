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
        ]
    }
}
