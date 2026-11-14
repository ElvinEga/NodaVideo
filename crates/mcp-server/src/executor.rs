//! Tool execution dispatcher for MCP tool calls.
//!
//! The executor owns an editing state and applies tool invocations to it.
//! Mutations are applied to a copy and committed only after the whole editing
//! state validates, so a failed call never leaves a partial edit behind.

use crate::composition;
use crate::error::McpError;
use crate::schema::McpToolDefinition;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::PathBuf;
use timeline::{
    Asset, AudioClip, AudioClipProperties, Clip, CompositionClip, MediaKind, TextBackground,
    TextClip, TextClipProperties, TimelineEditingState, TimelineFrameIndex, Track, TrackKind,
    VideoClip, VideoClipProperties,
};
use ulid::Ulid;

/// Dispatches typed MCP tool invocations against an editing state.
#[derive(Debug, Clone, Default)]
pub struct McpExecutor {
    timeline: TimelineEditingState,
    selected_clips: HashSet<Ulid>,
    project_root: Option<PathBuf>,
    revision: u64, // 种子状态的项目提交计数；commit 成功后自增，与编辑器保持一致。
    media_library: Vec<Asset>, // 资源管理器媒体库快照；只读观察用，非时间线 assets。
}

// Public APIs only
impl McpExecutor {
    pub fn new(timeline: TimelineEditingState) -> Self {
        Self {
            timeline,
            selected_clips: HashSet::new(),
            project_root: None,
            revision: 0,
            media_library: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_selection(mut self, selected_clips: HashSet<Ulid>) -> Self {
        self.selected_clips = selected_clips;
        self
    }

    /// Set the project directory used by composition tools for on-disk persistence.
    #[must_use]
    pub fn with_project_root(mut self, project_root: PathBuf) -> Self {
        self.project_root = Some(project_root);
        self
    }

    /// Seed the project revision the caller believes is current, for stale-state checks.
    #[must_use]
    pub fn with_revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    /// Seed the explorer media library snapshot so read tools can observe imported media.
    #[must_use]
    pub fn with_media_library(mut self, media_library: Vec<Asset>) -> Self {
        self.media_library = media_library;
        self
    }

    pub fn timeline(&self) -> &TimelineEditingState {
        &self.timeline
    }

    pub fn into_timeline(self) -> TimelineEditingState {
        self.timeline
    }

    /// The current project revision; advances only when a timeline edit commits.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Execute one MCP tool call.
    ///
    /// Tools marked as requiring approval are rejected unless `approved` is true.
    /// The caller owns the approval prompt; this check is not a UI policy.
    pub fn execute(
        &mut self,
        tool_name: &str,
        args: &Value,
        approved: bool,
    ) -> Result<Value, McpError> {
        let tools = McpToolDefinition::all_tools();
        let def = tools
            .iter()
            .find(|definition| definition.name == tool_name)
            .ok_or_else(|| McpError::ToolNotFound(tool_name.to_string()))?;

        if def.requires_approval && !approved {
            return Err(McpError::ApprovalRequired {
                tool: tool_name.to_string(),
                reason: format!("Operation '{tool_name}' requires explicit user confirmation"),
            });
        }

        if def.is_mutation {
            match args.get("expected_revision") {
                None | Some(Value::Null) => {}
                Some(value) => {
                    let Some(expected) = value.as_u64() else {
                        return Err(invalid(
                            tool_name,
                            "expected_revision must be a non-negative integer",
                        ));
                    };
                    if expected != self.revision {
                        return Err(McpError::StaleRevision {
                            expected,
                            actual: self.revision,
                        });
                    }
                }
            }
        }

        let mut result = match tool_name {
            "get_project" => Ok(self.project_summary()),
            "get_timeline" => Ok(self.timeline_summary()),
            "get_selected_clips" => Ok(self.selected_clips_summary()),
            "get_media_items" => Ok(self.media_library_summary()),
            "search_media" => self.search_media(args),
            "add_clip" => self.add_clip(args),
            "move_clip" => self.move_clip(args),
            "trim_clip" => self.trim_clip(args),
            "split_clip" => self.split_clip(args),
            "delete_clip" => self.delete_clip(args),
            "add_text_overlay" => self.add_text_overlay(args),
            "generate_captions" => self.generate_captions(args),
            "set_clip_transform" => self.set_clip_transform(args),
            "set_clip_audio" => self.set_clip_audio(args),
            "preview_changeset" => self.preview_changeset(args),
            "propose_changeset" => self.propose_changeset(args),
            "add_composition_to_timeline" => self.add_composition_to_timeline(args),
            "list_composition_templates" => Ok(composition::list_templates()),
            "list_compositions"
            | "create_composition"
            | "inspect_composition"
            | "edit_composition"
            | "render_composition"
            | "validate_composition" => self.execute_composition_tool(tool_name, args),
            "export_video" => Err(McpError::Unsupported {
                tool: tool_name.to_string(),
                reason:
                    "export runs in the editor process; call it from the editor's export action"
                        .to_string(),
            }),
            other => Err(McpError::ToolNotFound(other.to_string())),
        }?;
        if let Some(object) = result.as_object_mut() {
            object.insert("project_revision".into(), json!(self.revision));
        }
        Ok(result)
    }
}

// Private APIs only
impl McpExecutor {
    fn project_summary(&self) -> Value {
        json!({
            "timeline": self.timeline_summary(),
            "assets": self
                .timeline
                .assets
                .iter()
                .map(asset_json)
                .collect::<Vec<_>>(),
        })
    }

    fn media_library_summary(&self) -> Value {
        json!({
            "count": self.media_library.len(),
            "assets": self
                .media_library
                .iter()
                .map(asset_json)
                .collect::<Vec<_>>(),
        })
    }

    fn timeline_summary(&self) -> Value {
        let frame_rate = self.timeline.settings.frame_rate;
        let duration = self.timeline.content_duration();
        json!({
            "settings": {
                "frame_rate": {
                    "numerator": frame_rate.numerator,
                    "denominator": frame_rate.denominator,
                },
                "width": self.timeline.settings.width,
                "height": self.timeline.settings.height,
                "audio_sample_rate": self.timeline.settings.audio_sample_rate,
            },
            "duration_frames": i64::from(duration),
            "duration_seconds": frame_rate.seconds(duration),
            "tracks": self
                .timeline
                .tracks
                .iter()
                .map(|track| {
                    json!({
                        "id": track.id.to_string(),
                        "name": track.name,
                        "kind": track_kind_name(track.kind),
                        "locked": track.locked,
                        "muted": track.muted,
                        "visible": track.visible,
                        "clip_count": self
                            .timeline
                            .clips_on_track(track.id)
                            .count(),
                    })
                })
                .collect::<Vec<_>>(),
            "clips": self
                .timeline
                .clips
                .iter()
                .map(|clip| clip_json(&self.timeline, clip))
                .collect::<Vec<_>>(),
        })
    }

    fn selected_clips_summary(&self) -> Value {
        json!({
            "clips": self
                .timeline
                .clips
                .iter()
                .filter(|clip| self.selected_clips.contains(&clip.id()))
                .map(|clip| clip_json(&self.timeline, clip))
                .collect::<Vec<_>>(),
        })
    }

    fn search_media(&self, args: &Value) -> Result<Value, McpError> {
        let query = optional_str(args, "query").map(str::to_lowercase);
        let kind = match optional_str(args, "media_type") {
            Some("video") => Some(MediaKind::Video),
            Some("audio") => Some(MediaKind::Audio),
            Some("image") => Some(MediaKind::Image),
            Some(other) => {
                return Err(invalid(
                    "search_media",
                    format!("unsupported media_type {other:?}"),
                ));
            }
            None => None,
        };
        let assets = self
            .timeline
            .assets
            .iter()
            .filter(|asset| match kind {
                Some(kind) => asset.kind() == kind,
                None => true,
            })
            .filter(|asset| match &query {
                Some(query) => asset.name().to_lowercase().contains(query),
                None => true,
            })
            .map(asset_json)
            .collect::<Vec<_>>();
        Ok(json!({ "assets": assets }))
    }

    fn add_clip(&mut self, args: &Value) -> Result<Value, McpError> {
        let track_id = required_ulid("add_clip", args, "track_id")?;
        let asset_id = required_ulid("add_clip", args, "asset_id")?;
        let start = required_frame("add_clip", args, "start_frame")?;
        let source_in = required_frame("add_clip", args, "source_in")?;
        let source_out = required_frame("add_clip", args, "source_out")?;
        if source_out <= source_in {
            return Err(invalid(
                "add_clip",
                "source_out must be greater than source_in",
            ));
        }
        let asset = self
            .timeline
            .asset(asset_id)
            .cloned()
            .ok_or_else(|| invalid("add_clip", format!("asset {asset_id} is not imported")))?;
        let clip = match asset.kind() {
            MediaKind::Audio => Clip::Audio(AudioClip::new(
                Ulid::generate(),
                track_id,
                asset_id,
                start,
                source_in,
                source_out,
                AudioClipProperties::default(),
            )),
            MediaKind::Video | MediaKind::Image => Clip::Video(VideoClip::new(
                Ulid::generate(),
                track_id,
                asset_id,
                start,
                source_in,
                source_out,
                VideoClipProperties::default(),
                AudioClipProperties::default(),
            )),
        };
        let clip_id = clip.id();
        let mut new_timeline = self.timeline.clone();
        new_timeline.clips.push(clip);
        self.commit("add_clip", new_timeline)?;
        Ok(json!({ "status": "added", "clip_id": clip_id.to_string() }))
    }

    fn move_clip(&mut self, args: &Value) -> Result<Value, McpError> {
        let clip_id = required_ulid("move_clip", args, "clip_id")?;
        let start = required_frame("move_clip", args, "new_start_frame")?;
        let new_track_id = optional_ulid("move_clip", args, "new_track_id")?;
        let mut new_timeline = self.timeline.clone();
        let clip = new_timeline
            .clip_mut(clip_id)
            .ok_or_else(|| invalid("move_clip", format!("clip {clip_id} does not exist")))?;
        clip.set_timeline_start(start);
        if let Some(track_id) = new_track_id {
            clip.set_track_id(track_id);
        }
        self.commit("move_clip", new_timeline)?;
        Ok(json!({ "status": "moved", "clip_id": clip_id.to_string() }))
    }

    fn trim_clip(&mut self, args: &Value) -> Result<Value, McpError> {
        let clip_id = required_ulid("trim_clip", args, "clip_id")?;
        let start = required_frame("trim_clip", args, "new_start_frame")?;
        let source_in = required_frame("trim_clip", args, "new_source_in")?;
        let source_out = required_frame("trim_clip", args, "new_source_out")?;
        if source_out <= source_in {
            return Err(invalid(
                "trim_clip",
                "new_source_out must be greater than new_source_in",
            ));
        }
        let frame_rate = self.timeline.settings.frame_rate;
        let mut new_timeline = self.timeline.clone();
        let clip = new_timeline
            .clip_mut(clip_id)
            .ok_or_else(|| invalid("trim_clip", format!("clip {clip_id} does not exist")))?;
        match clip {
            Clip::Video(media) => {
                media.timeline_start = start;
                media.source_in = source_in;
                media.source_out = source_out;
            }
            Clip::Audio(media) => {
                media.timeline_start = start;
                media.source_in = source_in;
                media.source_out = source_out;
            }
            Clip::Text(text) => {
                text.timeline_start = start;
                text.duration = frame_rate.duration(source_out - source_in);
            }
            Clip::Composition(composition) => {
                composition.timeline_start = start;
                composition.duration = frame_rate.duration(source_out - source_in);
            }
        }
        self.commit("trim_clip", new_timeline)?;
        Ok(json!({ "status": "trimmed", "clip_id": clip_id.to_string() }))
    }

    fn split_clip(&mut self, args: &Value) -> Result<Value, McpError> {
        let clip_id = required_ulid("split_clip", args, "clip_id")?;
        let split_frame = required_frame("split_clip", args, "split_frame")?;
        let frame_rate = self.timeline.settings.frame_rate;
        let clip = self
            .timeline
            .clip(clip_id)
            .cloned()
            .ok_or_else(|| invalid("split_clip", format!("clip {clip_id} does not exist")))?;
        let local = split_frame - clip.timeline_start();
        let length = clip.frame_length(frame_rate);
        if local < TimelineFrameIndex::ONE_FRAME || local > length - TimelineFrameIndex::ONE_FRAME {
            return Err(invalid(
                "split_clip",
                format!(
                    "split_frame {} must fall strictly inside clip {clip_id} \
                     (start {}, end {})",
                    i64::from(split_frame),
                    i64::from(clip.timeline_start()),
                    i64::from(clip.timeline_start() + length)
                ),
            ));
        }
        let mut left = clip.clone();
        let mut right = clip.copy(Ulid::generate());
        right.set_timeline_start(split_frame);
        match (&mut left, &mut right) {
            (Clip::Video(left), Clip::Video(right)) => {
                let source_split = left.source_in + local;
                left.source_out = source_split;
                right.source_in = source_split;
            }
            (Clip::Audio(left), Clip::Audio(right)) => {
                let source_split = left.source_in + local;
                left.source_out = source_split;
                right.source_in = source_split;
            }
            (Clip::Text(left), Clip::Text(right)) => {
                left.duration = frame_rate.duration(local);
                right.duration = right.duration.saturating_sub(left.duration);
            }
            (Clip::Composition(left), Clip::Composition(right)) => {
                left.duration = frame_rate.duration(local);
                right.duration = right.duration.saturating_sub(left.duration);
            }
            (other_left, other_right) => {
                return Err(McpError::ExecutionFailed(format!(
                    "clip {clip_id} changed kind while splitting: {other_left:?} became {other_right:?}"
                )));
            }
        }
        let right_id = right.id();
        let mut new_timeline = self.timeline.clone();
        let index = new_timeline
            .clip_index(clip_id)
            .ok_or_else(|| invalid("split_clip", format!("clip {clip_id} does not exist")))?;
        new_timeline.clips[index] = left;
        new_timeline.clips.push(right);
        self.commit("split_clip", new_timeline)?;
        Ok(json!({
            "status": "split",
            "left_clip_id": clip_id.to_string(),
            "right_clip_id": right_id.to_string(),
        }))
    }

    fn delete_clip(&mut self, args: &Value) -> Result<Value, McpError> {
        let clip_id = required_ulid("delete_clip", args, "clip_id")?;
        if self.timeline.clip(clip_id).is_none() {
            return Err(invalid(
                "delete_clip",
                format!("clip {clip_id} does not exist"),
            ));
        }
        let mut new_timeline = self.timeline.clone();
        new_timeline.clips.retain(|clip| clip.id() != clip_id);
        self.commit("delete_clip", new_timeline)?;
        Ok(json!({ "status": "deleted", "clip_id": clip_id.to_string() }))
    }

    fn add_text_overlay(&mut self, args: &Value) -> Result<Value, McpError> {
        let track_id = required_ulid("add_text_overlay", args, "track_id")?;
        let text = required_str("add_text_overlay", args, "text")?;
        let start = required_frame("add_text_overlay", args, "start_frame")?;
        let duration_frames =
            optional_i64("add_text_overlay", args, "duration_frames")?.unwrap_or(72);
        if duration_frames <= 0 {
            return Err(invalid(
                "add_text_overlay",
                "duration_frames must be positive",
            ));
        }
        let properties =
            text_properties("add_text_overlay", args, text, 48.0, text_alignment_y(args))?;
        let clip = Clip::Text(TextClip::new(
            Ulid::generate(),
            track_id,
            start,
            self.timeline
                .settings
                .frame_rate
                .duration(duration_frames.into()),
            properties,
        ));
        let clip_id = clip.id();
        let mut new_timeline = self.timeline.clone();
        new_timeline.clips.push(clip);
        self.commit("add_text_overlay", new_timeline)?;
        Ok(json!({ "status": "added", "clip_id": clip_id.to_string() }))
    }

    fn add_composition_to_timeline(&mut self, args: &Value) -> Result<Value, McpError> {
        let composition_id = required_ulid("add_composition_to_timeline", args, "composition_id")?;
        let start = required_frame("add_composition_to_timeline", args, "start_frame")?;
        let requested_track_id = optional_ulid("add_composition_to_timeline", args, "track_id")?;
        let duration_frames = optional_i64("add_composition_to_timeline", args, "duration_frames")?;
        if let Some(frames) = duration_frames {
            if frames <= 0 {
                return Err(invalid(
                    "add_composition_to_timeline",
                    "duration_frames must be positive",
                ));
            }
        }
        let Some(project_root) = &self.project_root else {
            return Err(McpError::Unsupported {
                tool: "add_composition_to_timeline".to_string(),
                reason: "placing a composition requires an open project directory".to_string(),
            });
        };
        let asset = composition::find_composition(
            "add_composition_to_timeline",
            project_root,
            composition_id,
        )?;
        let frame_rate = self.timeline.settings.frame_rate;
        // 默认沿用合成自身时长（按墙钟换算）；显式 duration_frames 以时间线帧率计。
        let duration = match duration_frames {
            Some(frames) => frame_rate.duration(frames.into()),
            None => asset.fps.duration(asset.duration_frames.into()),
        };
        let mut new_timeline = self.timeline.clone();
        let track_id = if let Some(requested_id) = requested_track_id {
            let track = new_timeline.track(requested_id).ok_or_else(|| {
                invalid(
                    "add_composition_to_timeline",
                    format!("track {requested_id} does not exist"),
                )
            })?;
            if track.kind != TrackKind::Video {
                return Err(invalid(
                    "add_composition_to_timeline",
                    format!(
                        "track {requested_id} is not a video track; compositions require a video track"
                    ),
                ));
            }
            requested_id
        } else {
            let number = new_timeline
                .tracks
                .iter()
                .filter(|track| track.kind == TrackKind::Video)
                .count()
                + 1;
            let new_track_id = Ulid::generate();
            // 合成是覆盖层，新轨道置于最上方（设计页 24：Video 2 在 Video 1 之上）。
            new_timeline.tracks.insert(
                0,
                Track {
                    id: new_track_id,
                    name: format!("Video {number}"),
                    kind: TrackKind::Video,
                    locked: false,
                    muted: false,
                    visible: true,
                },
            );
            new_track_id
        };
        let clip = Clip::Composition(CompositionClip::new(
            Ulid::generate(),
            track_id,
            composition_id,
            start,
            duration,
        ));
        let clip_id = clip.id();
        new_timeline.clips.push(clip);
        self.commit("add_composition_to_timeline", new_timeline)?;
        Ok(json!({
            "status": "added",
            "clip_id": clip_id.to_string(),
            "track_id": track_id.to_string(),
        }))
    }

    fn generate_captions(&mut self, args: &Value) -> Result<Value, McpError> {
        let track_id = required_ulid("generate_captions", args, "track_id")?;
        let Some(captions) = args.get("captions").and_then(Value::as_array) else {
            return Err(invalid("generate_captions", "captions must be an array"));
        };
        if captions.is_empty() {
            return Err(invalid("generate_captions", "captions must not be empty"));
        }
        let frame_rate = self.timeline.settings.frame_rate;
        let font_size = optional_i64("generate_captions", args, "font_size")?.unwrap_or(36) as f64;
        let mut new_timeline = self.timeline.clone();
        let mut clip_ids = Vec::with_capacity(captions.len());
        for caption in captions {
            let text = required_str("generate_captions", caption, "text")?;
            let start = required_frame("generate_captions", caption, "start_frame")?;
            let end = required_frame("generate_captions", caption, "end_frame")?;
            if end <= start {
                return Err(invalid(
                    "generate_captions",
                    "end_frame must be greater than start_frame",
                ));
            }
            let properties = TextClipProperties {
                text: text.to_string(),
                font_size,
                color: match optional_str(args, "color") {
                    Some(color) => parse_argb_color("generate_captions", color)?,
                    None => 0xffff_ffff,
                },
                position: gpui::point(0.5, 0.85),
                ..TextClipProperties::default()
            };
            let clip = Clip::Text(TextClip::new(
                Ulid::generate(),
                track_id,
                start,
                frame_rate.duration(end - start),
                properties,
            ));
            clip_ids.push(clip.id().to_string());
            new_timeline.clips.push(clip);
        }
        self.commit("generate_captions", new_timeline)?;
        Ok(json!({ "status": "added", "clip_ids": clip_ids }))
    }

    fn set_clip_transform(&mut self, args: &Value) -> Result<Value, McpError> {
        let clip_id = required_ulid("set_clip_transform", args, "clip_id")?;
        let position_x = optional_f64("set_clip_transform", args, "position_x")?;
        let position_y = optional_f64("set_clip_transform", args, "position_y")?;
        let scale = optional_f64("set_clip_transform", args, "scale")?;
        if position_x.is_none() && position_y.is_none() && scale.is_none() {
            return Err(invalid(
                "set_clip_transform",
                "provide at least one of position_x, position_y, scale",
            ));
        }
        let mut new_timeline = self.timeline.clone();
        let clip = new_timeline.clip_mut(clip_id).ok_or_else(|| {
            invalid(
                "set_clip_transform",
                format!("clip {clip_id} does not exist"),
            )
        })?;
        match clip {
            Clip::Video(media) => {
                if let Some(value) = position_x {
                    media.video_properties.position_x = value;
                }
                if let Some(value) = position_y {
                    media.video_properties.position_y = value;
                }
                if let Some(value) = scale {
                    media.video_properties.scale = value;
                }
            }
            Clip::Text(text) => {
                if let Some(value) = position_x {
                    text.properties.position.x = value;
                }
                if let Some(value) = position_y {
                    text.properties.position.y = value;
                }
                if scale.is_some() {
                    return Err(invalid(
                        "set_clip_transform",
                        "scale applies to video clips; text uses font_size",
                    ));
                }
            }
            Clip::Audio(_) => {
                return Err(invalid(
                    "set_clip_transform",
                    format!("clip {clip_id} is an audio clip"),
                ));
            }
            Clip::Composition(_) => {
                return Err(invalid(
                    "set_clip_transform",
                    "composition clips are positioned through edit_composition parameters",
                ));
            }
        }
        self.commit("set_clip_transform", new_timeline)?;
        Ok(json!({ "status": "updated", "clip_id": clip_id.to_string() }))
    }

    fn set_clip_audio(&mut self, args: &Value) -> Result<Value, McpError> {
        let clip_id = required_ulid("set_clip_audio", args, "clip_id")?;
        let gain_db = optional_f64("set_clip_audio", args, "gain_db")?;
        let muted = optional_bool("set_clip_audio", args, "muted")?;
        if gain_db.is_none() && muted.is_none() {
            return Err(invalid("set_clip_audio", "provide gain_db and/or muted"));
        }
        let mut new_timeline = self.timeline.clone();
        let clip = new_timeline
            .clip_mut(clip_id)
            .ok_or_else(|| invalid("set_clip_audio", format!("clip {clip_id} does not exist")))?;
        match clip {
            Clip::Video(media) => {
                if let Some(value) = gain_db {
                    media.audio_properties.gain_db = value;
                }
                if let Some(value) = muted {
                    media.audio_properties.muted = value;
                }
            }
            Clip::Audio(media) => {
                if let Some(value) = gain_db {
                    media.audio_properties.gain_db = value;
                }
                if let Some(value) = muted {
                    media.audio_properties.muted = value;
                }
            }
            Clip::Text(_) => {
                return Err(invalid(
                    "set_clip_audio",
                    format!("clip {clip_id} is a text clip"),
                ));
            }
            Clip::Composition(_) => {
                return Err(invalid(
                    "set_clip_audio",
                    format!("clip {clip_id} is a composition clip"),
                ));
            }
        }
        self.commit("set_clip_audio", new_timeline)?;
        Ok(json!({ "status": "updated", "clip_id": clip_id.to_string() }))
    }

    fn preview_changeset(&self, args: &Value) -> Result<Value, McpError> {
        let title = optional_str(args, "title").unwrap_or("Proposed changes");
        let commands = changeset_commands("preview_changeset", args)?;
        let mut preview = Self::new(self.timeline.clone());
        preview.selected_clips.clone_from(&self.selected_clips);
        for command in commands {
            preview.execute(&command.tool, &command.arguments, true)?;
        }
        let new_timeline = preview.into_timeline();
        new_timeline
            .validate()
            .map_err(|error| McpError::ExecutionFailed(format!("{error}")))?;
        Ok(json!({
            "status": "preview",
            "title": title,
            "diff": diff_json(&self.timeline, &new_timeline),
            "result": timeline_summary_json(&new_timeline),
        }))
    }

    fn propose_changeset(&mut self, args: &Value) -> Result<Value, McpError> {
        let title = required_str("propose_changeset", args, "title")?.to_string();
        let commands = changeset_commands("propose_changeset", args)?;
        let mut preview = Self::new(self.timeline.clone());
        preview.selected_clips.clone_from(&self.selected_clips);
        for command in commands {
            preview.execute(&command.tool, &command.arguments, true)?;
        }
        let new_timeline = preview.into_timeline();
        let diff = diff_json(&self.timeline, &new_timeline);
        self.commit("propose_changeset", new_timeline)?;
        Ok(json!({ "status": "applied", "title": title, "diff": diff }))
    }

    fn commit(&mut self, tool: &str, new_timeline: TimelineEditingState) -> Result<(), McpError> {
        new_timeline
            .validate()
            .map_err(|error| McpError::ExecutionFailed(format!("{tool} rejected: {error}")))?;
        self.timeline = new_timeline;
        self.selected_clips
            .retain(|clip_id| self.timeline.clip(*clip_id).is_some());
        self.revision += 1;
        Ok(())
    }

    fn execute_composition_tool(&self, tool_name: &str, args: &Value) -> Result<Value, McpError> {
        let Some(project_root) = &self.project_root else {
            return Err(McpError::Unsupported {
                tool: tool_name.to_string(),
                reason: "composition tools require an open project directory".to_string(),
            });
        };
        match tool_name {
            "list_compositions" => composition::list_compositions(project_root),
            "create_composition" => composition::create_composition(project_root, args),
            "inspect_composition" => composition::inspect_composition(project_root, args),
            "edit_composition" => composition::edit_composition(project_root, args),
            "render_composition" => composition::render_composition(project_root, args),
            "validate_composition" => composition::validate_composition(project_root, args),
            other => Err(McpError::ToolNotFound(other.to_string())),
        }
    }
}

struct ChangesetCommand {
    tool: String,
    arguments: Value,
}

fn changeset_commands(tool: &str, args: &Value) -> Result<Vec<ChangesetCommand>, McpError> {
    let Some(commands) = args.get("commands").and_then(Value::as_array) else {
        return Err(invalid(tool, "commands must be an array"));
    };
    if commands.is_empty() {
        return Err(invalid(tool, "commands must not be empty"));
    }
    commands
        .iter()
        .map(|command| {
            let tool_name = ["tool", "name", "method"]
                .iter()
                .find_map(|key| command.get(*key).and_then(Value::as_str))
                .ok_or_else(|| {
                    invalid(
                        tool,
                        "each command needs a tool name under 'tool', 'name' or 'method'",
                    )
                })?;
            let arguments = ["arguments", "args", "params"]
                .iter()
                .find_map(|key| command.get(*key))
                .cloned()
                .unwrap_or_else(|| json!({}));
            Ok(ChangesetCommand {
                tool: tool_name.to_string(),
                arguments,
            })
        })
        .collect()
}

fn diff_json(before: &TimelineEditingState, after: &TimelineEditingState) -> Value {
    let before_ids = before.clips.iter().map(Clip::id).collect::<HashSet<_>>();
    let after_ids = after.clips.iter().map(Clip::id).collect::<HashSet<_>>();
    let added = after_ids
        .difference(&before_ids)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let removed = before_ids
        .difference(&after_ids)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let modified = after
        .clips
        .iter()
        .filter(|clip| {
            before_ids.contains(&clip.id()) && before.clip(clip.id()) != Some(*clip) // 逐字段比较，识别属性修改。
        })
        .map(|clip| clip.id().to_string())
        .collect::<Vec<_>>();
    json!({
        "added_clip_ids": added,
        "removed_clip_ids": removed,
        "modified_clip_ids": modified,
        "clip_count_before": before.clips.len(),
        "clip_count_after": after.clips.len(),
    })
}

fn timeline_summary_json(timeline: &TimelineEditingState) -> Value {
    let frame_rate = timeline.settings.frame_rate;
    json!({
        "duration_frames": i64::from(timeline.content_duration()),
        "duration_seconds": frame_rate.seconds(timeline.content_duration()),
        "track_count": timeline.tracks.len(),
        "asset_count": timeline.assets.len(),
        "clip_count": timeline.clips.len(),
    })
}

fn asset_json(asset: &Asset) -> Value {
    json!({
        "id": asset.id().to_string(),
        "name": asset.name(),
        "kind": media_kind_name(asset.kind()),
        "path": asset.path().to_string_lossy(),
        "width": asset.width(),
        "height": asset.height(),
        "duration_seconds": asset.duration(),
        "has_audio": asset.has_audio(),
    })
}

fn clip_json(timeline: &TimelineEditingState, clip: &Clip) -> Value {
    let frame_rate = timeline.settings.frame_rate;
    let asset_name = clip
        .asset_id()
        .and_then(|asset_id| timeline.asset(asset_id))
        .map(|asset| asset.name().clone());
    let asset_id = clip.asset_id().map(|id| id.to_string());
    json!({
        "id": clip.id().to_string(),
        "kind": clip_kind_name(clip),
        "track_id": clip.track_id().to_string(),
        "asset_id": asset_id,
        "name": asset_name,
        "start_frame": i64::from(clip.timeline_start()),
        "end_frame": i64::from(clip.timeline_end(frame_rate)),
        "source_in": clip.source_in().map(i64::from),
        "source_out": clip.source_out().map(i64::from),
    })
}

fn clip_kind_name(clip: &Clip) -> &'static str {
    match clip {
        Clip::Video(_) => "video",
        Clip::Audio(_) => "audio",
        Clip::Text(_) => "text",
        Clip::Composition(_) => "composition",
    }
}

fn media_kind_name(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Video => "video",
        MediaKind::Image => "image",
        MediaKind::Audio => "audio",
    }
}

fn track_kind_name(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "video",
        TrackKind::Audio => "audio",
        TrackKind::Text => "text",
    }
}

fn text_properties(
    tool: &str,
    args: &Value,
    text: &str,
    default_font_size: f64,
    position_y: f64,
) -> Result<TextClipProperties, McpError> {
    let properties = TextClipProperties {
        text: text.to_string(),
        font: optional_str(args, "font_family")
            .unwrap_or("Sans")
            .to_string(),
        font_size: optional_f64(tool, args, "font_size")?.unwrap_or(default_font_size),
        color: match optional_str(args, "color") {
            Some(color) => parse_argb_color(tool, color)?,
            None => 0xffff_ffff,
        },
        background: match optional_str(args, "background_color") {
            Some(color) => Some(parse_background_color(tool, color)?),
            None => None,
        },
        position: gpui::point(0.5, position_y),
    };
    Ok(properties)
}

fn text_alignment_y(args: &Value) -> f64 {
    match optional_str(args, "alignment") {
        Some("top") => 0.15,
        Some("center") => 0.5,
        Some("bottom") | None => 0.85,
        Some(other) => {
            log::debug!("unknown text alignment {other:?}; using bottom");
            0.85
        }
    }
}

fn parse_argb_color(tool: &str, value: &str) -> Result<u32, McpError> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    let parsed = match hex.len() {
        6 => u32::from_str_radix(hex, 16)
            .ok()
            .map(|rgb| 0xff00_0000 | rgb),
        8 => u32::from_str_radix(hex, 16).ok(),
        _ => None,
    };
    match parsed {
        Some(argb) => Ok(argb),
        None => Err(invalid(
            tool,
            format!("color {value:?} must be #RRGGBB or #RRGGBBAA"),
        )),
    }
}

fn parse_background_color(tool: &str, value: &str) -> Result<TextBackground, McpError> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    let parsed = match hex.len() {
        6 | 8 => u32::from_str_radix(hex, 16).ok(),
        _ => None,
    };
    let Some(rgba) = parsed else {
        return Err(invalid(
            tool,
            format!("background_color {value:?} must be #RRGGBB or #RRGGBBAA"),
        ));
    };
    let red = (rgba >> 16) as u8;
    let green = (rgba >> 8) as u8;
    let blue = rgba as u8;
    let alpha = match hex.len() {
        8 => (rgba >> 24) as u8,
        6 => 0xff,
        _ => unreachable!("length was checked above"),
    };
    Ok(TextBackground {
        red,
        green,
        blue,
        alpha,
    })
}

pub(crate) fn invalid(tool: &str, details: impl Into<String>) -> McpError {
    McpError::InvalidArguments {
        tool: tool.to_string(),
        details: details.into(),
    }
}

pub(crate) fn required_ulid(tool: &str, args: &Value, field: &str) -> Result<Ulid, McpError> {
    let Some(value) = args.get(field).and_then(Value::as_str) else {
        return Err(invalid(tool, format!("{field} must be a string")));
    };
    Ulid::from_string(value)
        .map_err(|error| invalid(tool, format!("{field} is not a valid ULID: {error}")))
}

fn optional_ulid(tool: &str, args: &Value, field: &str) -> Result<Option<Ulid>, McpError> {
    match args.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(_) => required_ulid(tool, args, field).map(Some),
    }
}

fn required_frame(tool: &str, args: &Value, field: &str) -> Result<TimelineFrameIndex, McpError> {
    let Some(value) = args.get(field).and_then(Value::as_i64) else {
        return Err(invalid(tool, format!("{field} must be an integer")));
    };
    if value < 0 {
        return Err(invalid(tool, format!("{field} must not be negative")));
    }
    Ok(TimelineFrameIndex::from(value))
}

pub(crate) fn required_str<'a>(
    tool: &str,
    args: &'a Value,
    field: &str,
) -> Result<&'a str, McpError> {
    match args.get(field).and_then(Value::as_str) {
        Some(value) => Ok(value),
        None => Err(invalid(tool, format!("{field} must be a string"))),
    }
}

pub(crate) fn optional_str<'a>(args: &'a Value, field: &str) -> Option<&'a str> {
    args.get(field).and_then(Value::as_str)
}

fn optional_i64(tool: &str, args: &Value, field: &str) -> Result<Option<i64>, McpError> {
    match args.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => match value.as_i64() {
            Some(number) => Ok(Some(number)),
            None => Err(invalid(tool, format!("{field} must be an integer"))),
        },
    }
}

fn optional_f64(tool: &str, args: &Value, field: &str) -> Result<Option<f64>, McpError> {
    match args.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => match value.as_f64() {
            Some(number) => Ok(Some(number)),
            None => Err(invalid(tool, format!("{field} must be a number"))),
        },
    }
}

fn optional_bool(tool: &str, args: &Value, field: &str) -> Result<Option<bool>, McpError> {
    match args.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => match value.as_bool() {
            Some(flag) => Ok(Some(flag)),
            None => Err(invalid(tool, format!("{field} must be a boolean"))),
        },
    }
}
