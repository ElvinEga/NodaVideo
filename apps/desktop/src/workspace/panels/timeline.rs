//! Timeline panel displaying multitrack composition, ruler, clips, playhead, and editing controls.

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Disableable, Icon, Sizable, Theme};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use nodavideo_command_system::{
    AddClip, BatchCommand, CommandHistory, DeleteClip, EditorCommand, MoveClip, RemoveTransition,
    SplitClip, ToggleLock, ToggleMute, ToggleSolo, TrimClip,
};
use nodavideo_media_engine::WaveformData;
use nodavideo_project::{AssetId, MediaAsset, MediaType};
use nodavideo_timeline::{
    Clip, ClipId, Sequence, TimelineSelection, Track, TrackId, TrackType, Transition, TransitionId,
    TransitionType, snap_clip_move, snap_frame,
};

use crate::theme::metrics;

/// Active timeline editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineTool {
    #[default]
    Select,
    Split,
}

/// Callback when external OS files (e.g. .mp4) are dropped onto the timeline.
pub type ExternalDropCallback = Rc<
    dyn Fn(Vec<PathBuf>, Option<TrackId>, Option<i64>, &mut Window, &mut Context<TimelinePanel>),
>;

/// Timeline panel view component.
pub struct TimelinePanel {
    pub sequence: Sequence,
    pub history: CommandHistory,
    pub selection: TimelineSelection,
    pub selected_transition: Option<TransitionId>,
    pub current_tool: TimelineTool,
    pub snapping_enabled: bool,
    pub ripple_enabled: bool,
    pub zoom_level: usize,
    pub snap_threshold_frames: i64,
    pub waveforms: HashMap<AssetId, WaveformData>,
    pub is_scrubbing: bool,
    pub on_external_drop: Option<ExternalDropCallback>,
}

impl std::fmt::Debug for TimelinePanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimelinePanel")
            .field("sequence", &self.sequence.name)
            .field("current_tool", &self.current_tool)
            .field("is_scrubbing", &self.is_scrubbing)
            .finish_non_exhaustive()
    }
}

impl Default for TimelinePanel {
    fn default() -> Self {
        Self {
            sequence: Sequence::default_1080p24(),
            history: CommandHistory::new(100),
            selection: TimelineSelection::new(),
            selected_transition: None,
            current_tool: TimelineTool::Select,
            snapping_enabled: true,
            ripple_enabled: false,
            zoom_level: 100,
            snap_threshold_frames: 6,
            waveforms: HashMap::new(),
            is_scrubbing: false,
            on_external_drop: None,
        }
    }
}

impl TimelinePanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pixels per frame based on current zoom level.
    pub fn pixels_per_frame(&self) -> f32 {
        5.0 * (self.zoom_level as f32 / 100.0)
    }

    /// Set current editing tool.
    pub fn set_tool(&mut self, tool: TimelineTool, cx: &mut Context<Self>) {
        self.current_tool = tool;
        cx.notify();
    }

    /// Toggle magnetic snapping on/off.
    pub fn toggle_snapping(&mut self, cx: &mut Context<Self>) {
        self.snapping_enabled = !self.snapping_enabled;
        cx.notify();
    }

    /// Toggle ripple editing mode on/off.
    pub fn toggle_ripple(&mut self, cx: &mut Context<Self>) {
        self.ripple_enabled = !self.ripple_enabled;
        cx.notify();
    }

    /// Zoom in on the timeline.
    pub fn zoom_in(&mut self, cx: &mut Context<Self>) {
        if self.zoom_level < 400 {
            self.zoom_level = (self.zoom_level + 25).min(400);
            cx.notify();
        }
    }

    /// Zoom out on the timeline.
    pub fn zoom_out(&mut self, cx: &mut Context<Self>) {
        if self.zoom_level > 25 {
            self.zoom_level = (self.zoom_level - 25).max(25);
            cx.notify();
        }
    }

    /// Current playhead position in frames.
    pub fn playhead(&self) -> i64 {
        self.selection.playhead_frame
    }

    /// Default timeline duration when sequence has no clips (30 seconds).
    pub fn default_empty_duration_frames(&self) -> i64 {
        let fps = self.sequence.fps.rounded_int().max(1);
        30 * fps
    }

    /// Effective timeline duration for navigation and playback, falling back to 30s when empty.
    pub fn effective_duration_frames(&self) -> i64 {
        let actual = self.sequence.duration_frames();
        if actual > 0 {
            actual
        } else {
            self.default_empty_duration_frames()
        }
    }

    /// Move playhead to specified frame.
    pub fn set_playhead(&mut self, frame: i64, cx: &mut Context<Self>) {
        let target = if self.snapping_enabled {
            snap_frame(
                frame.max(0),
                &self.sequence,
                None,
                None,
                self.snap_threshold_frames,
            )
            .snapped_frame
        } else {
            frame.max(0)
        };
        self.selection.set_playhead(target);
        cx.notify();
    }

    /// Advance playhead by a number of frames.
    pub fn step_forward(&mut self, frames: i64, cx: &mut Context<Self>) {
        let new_frame = self.selection.playhead_frame + frames;
        self.selection.set_playhead(new_frame);
        cx.notify();
    }

    /// Move playhead backwards by a number of frames.
    pub fn step_backward(&mut self, frames: i64, cx: &mut Context<Self>) {
        let new_frame = (self.selection.playhead_frame - frames).max(0);
        self.selection.set_playhead(new_frame);
        cx.notify();
    }

    /// Register or update cached audio waveform data for an asset.
    pub fn set_waveform(&mut self, asset_id: AssetId, waveform: WaveformData) {
        self.waveforms.insert(asset_id, waveform);
    }

    /// Extract or compute waveform peak samples for rendering inside an audio clip.
    pub fn get_clip_waveform_peaks(&self, clip: &Clip, num_bars: usize) -> Vec<f32> {
        if num_bars == 0 {
            return Vec::new();
        }

        if let Some(wf) = self.waveforms.get(&clip.asset_id) {
            if !wf.peaks.is_empty() {
                let fps = self.sequence.fps.to_f64().max(1.0);
                let start_sec = (clip.source_in as f64 / fps).max(0.0);
                let end_sec = (clip.source_out as f64 / fps).max(start_sec + 0.01);
                let pps = wf.points_per_second as f64;
                let start_idx = (start_sec * pps) as usize;
                let end_idx = ((end_sec * pps) as usize).min(wf.peaks.len());

                if start_idx < end_idx {
                    let slice = &wf.peaks[start_idx..end_idx];
                    let chunk_size = (slice.len() as f64 / num_bars as f64).max(1.0);
                    let mut sampled = Vec::with_capacity(num_bars);

                    for b in 0..num_bars {
                        let c_start = (b as f64 * chunk_size) as usize;
                        let c_end = (((b + 1) as f64 * chunk_size) as usize).min(slice.len());
                        if c_start < c_end {
                            let mut max_val: f32 = 0.0;
                            for &p in &slice[c_start..c_end] {
                                if p > max_val {
                                    max_val = p;
                                }
                            }
                            sampled.push(max_val.clamp(0.05, 1.0));
                        } else {
                            sampled.push(0.05);
                        }
                    }
                    return sampled;
                }
            }
        }

        // Fallback: generate naturalistic audio envelope waveforms based on clip parameters
        let seed = (clip.start_frame as f32).mul_add(0.13, clip.source_in as f32 * 0.07);
        (0..num_bars)
            .map(|i| {
                let t = (i as f32).mul_add(0.22, seed);
                let harmonic1 = (t * 0.9).sin().abs() * 0.55;
                let harmonic2 = (t * 2.1).cos().abs() * 0.35;
                (harmonic1 + harmonic2 + 0.1).clamp(0.05, 1.0)
            })
            .collect()
    }

    /// Undo last command.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if let Ok(Some(_)) = self.history.undo(&mut self.sequence) {
            cx.notify();
        }
    }

    /// Redo last undone command.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if let Ok(Some(_)) = self.history.redo(&mut self.sequence) {
            cx.notify();
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Set active sequence and reset selection and history.
    pub fn set_sequence(&mut self, sequence: Sequence, cx: &mut Context<Self>) {
        self.sequence = sequence;
        self.selection.clear_selection();
        self.selected_transition = None;
        self.selection.set_playhead(0);
        self.history.clear();
        cx.notify();
    }

    /// Split clip(s) at current playhead position.
    pub fn split_at_playhead(&mut self, cx: &mut Context<Self>) {
        let playhead = self.selection.playhead_frame;

        // 1. If no clips are selected, split all unlocked clips intersecting playhead; otherwise split selected clips
        let target_clips: Vec<(TrackId, ClipId, i64)> = if self.selection.selected_clips.is_empty()
        {
            self.sequence
                .tracks
                .iter()
                .filter(|t| !t.locked)
                .flat_map(|t| {
                    t.clips.iter().filter_map(|c| {
                        if c.contains_timeline_frame(playhead)
                            && playhead > c.start_frame
                            && playhead < c.end_frame()
                        {
                            Some((t.id, c.id, c.source_out))
                        } else {
                            None
                        }
                    })
                })
                .collect()
        } else {
            self.selection
                .selected_clips
                .iter()
                .filter_map(|&clip_id| {
                    let track = self.sequence.find_track_for_clip(clip_id)?;
                    if track.locked {
                        return None;
                    }
                    let clip = track.get_clip(clip_id)?;
                    if clip.contains_timeline_frame(playhead)
                        && playhead > clip.start_frame
                        && playhead < clip.end_frame()
                    {
                        Some((track.id, clip.id, clip.source_out))
                    } else {
                        None
                    }
                })
                .collect()
        };

        for (track_id, clip_id, source_out) in target_clips {
            let new_clip_id = ClipId::new();
            let cmd = EditorCommand::SplitClip(SplitClip {
                original_clip_id: clip_id,
                track_id,
                split_frame: playhead,
                new_clip_id,
                original_source_out: source_out,
            });
            if self.history.execute(cmd, &mut self.sequence).is_ok() {
                self.selection.select_single(new_clip_id);
            }
        }
        cx.notify();
    }

    /// Delete currently selected clip(s) or transition.
    pub fn delete_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(trans_id) = self.selected_transition {
            for track in &self.sequence.tracks {
                if let Some(trans) = track.transitions.iter().find(|t| t.id == trans_id) {
                    let cmd = EditorCommand::RemoveTransition(RemoveTransition {
                        track_id: trans.track_id,
                        transition: trans.clone(),
                    });
                    let _ = self.history.execute(cmd, &mut self.sequence);
                    break;
                }
            }
            self.selected_transition = None;
            cx.notify();
            return;
        }

        let to_delete: Vec<(TrackId, Clip)> = self
            .selection
            .selected_clips
            .iter()
            .filter_map(|&id| {
                let track = self.sequence.find_track_for_clip(id)?;
                if track.locked {
                    return None;
                }
                let clip = track.get_clip(id)?.clone();
                Some((track.id, clip))
            })
            .collect();

        for (track_id, clip) in to_delete {
            let cmd = EditorCommand::DeleteClip(DeleteClip { track_id, clip });
            let _ = self.history.execute(cmd, &mut self.sequence);
        }

        self.selection.clear_selection();
        cx.notify();
    }

    /// Return the currently primary-selected clip on the timeline, if any.
    pub fn get_selected_clip(&self) -> Option<Clip> {
        let clip_id = self.selection.primary_selected()?;
        self.sequence.get_clip(clip_id).cloned()
    }

    /// Return the currently selected transition on the timeline, if any.
    pub fn get_selected_transition(&self) -> Option<Transition> {
        let trans_id = self.selected_transition?;
        for track in &self.sequence.tracks {
            if let Some(t) = track.transitions.iter().find(|t| t.id == trans_id) {
                return Some(t.clone());
            }
        }
        None
    }

    /// Trim selected clip in-point by delta frames.
    pub fn trim_selected_in(&mut self, delta_frames: i64, cx: &mut Context<Self>) {
        let Some(clip_id) = self.selection.primary_selected() else {
            return;
        };
        let Some(track) = self.sequence.find_track_for_clip(clip_id) else {
            return;
        };
        if track.locked {
            return;
        }
        let Some(clip) = track.get_clip(clip_id) else {
            return;
        };

        let new_source_in = clip.source_in + delta_frames;
        if new_source_in >= 0 && new_source_in < clip.source_out {
            let new_start_frame = clip.start_frame + delta_frames;
            if new_start_frame >= 0 {
                let cmd = EditorCommand::TrimClip(TrimClip {
                    clip_id,
                    track_id: track.id,
                    old_start_frame: clip.start_frame,
                    new_start_frame,
                    old_source_in: clip.source_in,
                    new_source_in,
                    old_source_out: clip.source_out,
                    new_source_out: clip.source_out,
                });
                let _ = self.history.execute(cmd, &mut self.sequence);
                cx.notify();
            }
        }
    }

    /// Trim selected clip out-point by delta frames.
    pub fn trim_selected_out(&mut self, delta_frames: i64, cx: &mut Context<Self>) {
        let Some(clip_id) = self.selection.primary_selected() else {
            return;
        };
        let Some(track) = self.sequence.find_track_for_clip(clip_id) else {
            return;
        };
        if track.locked {
            return;
        }
        let Some(clip) = track.get_clip(clip_id) else {
            return;
        };

        let new_source_out = clip.source_out + delta_frames;
        if new_source_out > clip.source_in {
            let cmd = EditorCommand::TrimClip(TrimClip {
                clip_id,
                track_id: track.id,
                old_start_frame: clip.start_frame,
                new_start_frame: clip.start_frame,
                old_source_in: clip.source_in,
                new_source_in: clip.source_in,
                old_source_out: clip.source_out,
                new_source_out,
            });
            let _ = self.history.execute(cmd, &mut self.sequence);
            cx.notify();
        }
    }

    /// Move selected clip by delta frames.
    pub fn nudge_selected(&mut self, delta_frames: i64, cx: &mut Context<Self>) {
        let Some(clip_id) = self.selection.primary_selected() else {
            return;
        };
        let Some(track) = self.sequence.find_track_for_clip(clip_id) else {
            return;
        };
        if track.locked {
            return;
        }
        let Some(clip) = track.get_clip(clip_id) else {
            return;
        };

        let candidate_start = (clip.start_frame + delta_frames).max(0);
        let final_start = if self.snapping_enabled {
            snap_clip_move(
                candidate_start,
                clip.duration_frames(),
                &self.sequence,
                Some(self.selection.playhead_frame),
                clip.id,
                self.snap_threshold_frames,
            )
            .snapped_frame
        } else {
            candidate_start
        };

        let cmd = EditorCommand::MoveClip(MoveClip {
            clip_id,
            from_track: track.id,
            to_track: track.id,
            old_start_frame: clip.start_frame,
            new_start_frame: final_start,
        });
        let _ = self.history.execute(cmd, &mut self.sequence);
        cx.notify();
    }

    /// Toggle mute on track.
    pub fn toggle_track_mute(&mut self, track_id: TrackId, cx: &mut Context<Self>) {
        let cmd = EditorCommand::ToggleMute(ToggleMute { track_id });
        let _ = self.history.execute(cmd, &mut self.sequence);
        cx.notify();
    }

    /// Toggle solo on track.
    pub fn toggle_track_solo(&mut self, track_id: TrackId, cx: &mut Context<Self>) {
        let cmd = EditorCommand::ToggleSolo(ToggleSolo { track_id });
        let _ = self.history.execute(cmd, &mut self.sequence);
        cx.notify();
    }

    /// Toggle lock on track.
    pub fn toggle_track_lock(&mut self, track_id: TrackId, cx: &mut Context<Self>) {
        let cmd = EditorCommand::ToggleLock(ToggleLock { track_id });
        let _ = self.history.execute(cmd, &mut self.sequence);
        cx.notify();
    }

    /// Apply asset drop onto timeline, creating linked video + audio clips if applicable.
    pub fn drop_asset_clip(
        &mut self,
        track_id: TrackId,
        asset: &MediaAsset,
        target_frame: Option<i64>,
    ) -> bool {
        let Some(track) = self.sequence.get_track(track_id) else {
            return false;
        };
        if track.locked {
            return false;
        }

        // Calculate clip duration in frames
        let duration_frames = asset.duration.map_or(
            120, // Default 5 seconds at 24fps for images
            |d| d.to_frames(self.sequence.fps).max(1),
        );

        // Place clip at target_frame or current playhead frame
        let candidate_start = target_frame.unwrap_or(self.selection.playhead_frame).max(0);
        let start_frame = if self.snapping_enabled {
            snap_clip_move(
                candidate_start,
                duration_frames,
                &self.sequence,
                Some(self.selection.playhead_frame),
                ClipId::new(),
                self.snap_threshold_frames,
            )
            .snapped_frame
        } else {
            candidate_start
        };

        // Check if this is a video asset with audio being placed on a video track
        let has_audio = asset.media_type == MediaType::Video
            && (asset.audio_codec.is_some() || asset.audio_channels.is_some());

        if track.track_type == TrackType::Video && has_audio {
            // Find the primary unlocked audio track (e.g. A1)
            let audio_track = self
                .sequence
                .tracks
                .iter()
                .find(|t| t.track_type == TrackType::Audio && !t.locked)
                .cloned();

            let video_clip_id = ClipId::new();
            let video_clip = match Clip::new(
                video_clip_id,
                asset.id,
                track_id,
                asset.original_filename.clone(),
                start_frame,
                0,
                duration_frames,
            ) {
                Ok(c) => c,
                Err(_) => return false,
            };

            let mut batch_commands = vec![EditorCommand::AddClip(AddClip {
                track_id,
                clip: video_clip,
            })];

            if let Some(a_track) = audio_track {
                let a_id = ClipId::new();
                if let Ok(a_clip) = Clip::new(
                    a_id,
                    asset.id,
                    a_track.id,
                    format!("{} [Audio]", asset.original_filename),
                    start_frame,
                    0,
                    duration_frames,
                ) {
                    batch_commands.push(EditorCommand::AddClip(AddClip {
                        track_id: a_track.id,
                        clip: a_clip,
                    }));
                }
            }

            let batch = EditorCommand::Batch(BatchCommand {
                name: format!("Add Linked Clip: {}", asset.original_filename),
                commands: batch_commands,
            });

            if self.history.execute(batch, &mut self.sequence).is_ok() {
                self.selection.select_single(video_clip_id);
                self.selection.set_playhead(start_frame + duration_frames);
                return true;
            }
            return false;
        }

        // Standard single clip placement
        match (track.track_type, asset.media_type) {
            (TrackType::Video, MediaType::Video | MediaType::Image)
            | (TrackType::Audio, MediaType::Audio | MediaType::Video) => {}
            _ => return false, // Incompatible track type
        }

        let clip_id = ClipId::new();
        let clip = match Clip::new(
            clip_id,
            asset.id,
            track_id,
            asset.original_filename.clone(),
            start_frame,
            0,
            duration_frames,
        ) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let cmd = EditorCommand::AddClip(AddClip { track_id, clip });
        if self.history.execute(cmd, &mut self.sequence).is_ok() {
            self.selection.select_single(clip_id);
            self.selection.set_playhead(start_frame + duration_frames);
            return true;
        }
        false
    }

    /// Handle dropping a media asset onto a track with target frame and audio-linking support.
    pub fn handle_asset_drop_with_frame(
        &mut self,
        track_id: TrackId,
        asset: &MediaAsset,
        target_frame: Option<i64>,
        cx: &mut Context<Self>,
    ) {
        if self.drop_asset_clip(track_id, asset, target_frame) {
            cx.notify();
        }
    }

    /// Handle dropping a media asset from Media Browser onto a track at current playhead.
    pub fn handle_asset_drop(
        &mut self,
        track_id: TrackId,
        asset: &MediaAsset,
        cx: &mut Context<Self>,
    ) {
        self.handle_asset_drop_with_frame(track_id, asset, None, cx);
    }
}

impl Render for TimelinePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let ppf = self.pixels_per_frame();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .border_t_1()
            .border_color(theme.border)
            .child(self.render_timeline_toolbar(theme, cx))
            .child(self.render_canvas(theme, ppf, cx))
    }
}

impl TimelinePanel {
    /// Render top toolbar of the timeline panel.
    fn render_timeline_toolbar(&self, theme: &Theme, cx: &Context<Self>) -> impl IntoElement {
        let snapping_enabled = self.snapping_enabled;
        let ripple_enabled = self.ripple_enabled;
        let current_tool = self.current_tool;
        let mono_font = theme.mono_font_family.clone();
        let playhead_frame = self.selection.playhead_frame;
        let playhead_timecode = self.sequence.frame_to_timecode(playhead_frame);
        let duration_timecode = self
            .sequence
            .frame_to_timecode(self.effective_duration_frames());
        let has_selection = !self.selection.selected_clips.is_empty();

        div()
            .flex()
            .items_center()
            .justify_between()
            .h_9()
            .px_3()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.secondary)
            .child(
                // Left: Tools, Splitting, Deletion & Snapping Controls
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(Icon::new(IconName::SlidersHorizontal))
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child(format!("Timeline: {}", self.sequence.name)),
                    )
                    .child(
                        div()
                            .h_4()
                            .w(px(1.0))
                            .bg(theme.border)
                            .mx_1(),
                    )
                    // Select Tool (V)
                    .child(
                        Button::new("tool-select")
                            .ghost()
                            .xsmall()
                            .icon(IconName::MousePointer)
                            .tooltip("Select Tool (V)")
                            .when(current_tool == TimelineTool::Select, |btn| btn.outline())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_tool(TimelineTool::Select, cx);
                            })),
                    )
                    // Blade / Split Tool (B)
                    .child(
                        Button::new("tool-split")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Scissors)
                            .tooltip("Razor / Split Tool (C/B)")
                            .when(current_tool == TimelineTool::Split, |btn| btn.outline())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_tool(TimelineTool::Split, cx);
                            })),
                    )
                    // Split at Playhead button
                    .child(
                        Button::new("btn-split-playhead")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Slice)
                            .tooltip("Split at Playhead (Cmd+K)")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.split_at_playhead(cx);
                            })),
                    )
                    // Delete Selected button
                    .child(
                        Button::new("btn-delete-clip")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Trash)
                            .tooltip("Delete Selected (Backspace)")
                            .when(!has_selection, |btn| btn.disabled(true))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.delete_selected(cx);
                            })),
                    )
                    .child(
                        div()
                            .h_4()
                            .w(px(1.0))
                            .bg(theme.border)
                            .mx_1(),
                    )
                    // Snapping Magnet Toggle
                    .child(
                        Button::new("tool-snapping")
                            .ghost()
                            .xsmall()
                            .label("Snap")
                            .tooltip("Toggle Magnetic Snapping (S)")
                            .when(snapping_enabled, |btn| btn.outline())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_snapping(cx);
                            })),
                    )
                    // Ripple Edit Mode Toggle
                    .child(
                        Button::new("tool-ripple")
                            .ghost()
                            .xsmall()
                            .label("Ripple")
                            .tooltip("Toggle Ripple Edit Mode")
                            .when(ripple_enabled, |btn| btn.outline())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_ripple(cx);
                            })),
                    )
                    .child(
                        div()
                            .h_4()
                            .w(px(1.0))
                            .bg(theme.border)
                            .mx_1(),
                    )
                    // Nudge Left (<)
                    .child(
                        Button::new("btn-nudge-left")
                            .ghost()
                            .xsmall()
                            .icon(IconName::ChevronLeft)
                            .tooltip("Nudge Clip Left 1 Frame")
                            .when(!has_selection, |btn| btn.disabled(true))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.nudge_selected(-1, cx);
                            })),
                    )
                    // Nudge Right (>)
                    .child(
                        Button::new("btn-nudge-right")
                            .ghost()
                            .xsmall()
                            .icon(IconName::ChevronRight)
                            .tooltip("Nudge Clip Right 1 Frame")
                            .when(!has_selection, |btn| btn.disabled(true))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.nudge_selected(1, cx);
                            })),
                    )
                    // Trim In (-1s)
                    .child(
                        Button::new("btn-trim-in")
                            .ghost()
                            .xsmall()
                            .label("Trim In")
                            .tooltip("Trim In +1s")
                            .when(!has_selection, |btn| btn.disabled(true))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.trim_selected_in(24, cx);
                            })),
                    )
                    // Trim Out (-1s)
                    .child(
                        Button::new("btn-trim-out")
                            .ghost()
                            .xsmall()
                            .label("Trim Out")
                            .tooltip("Trim Out -1s")
                            .when(!has_selection, |btn| btn.disabled(true))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.trim_selected_out(-24, cx);
                            })),
                    ),
            )
            .child(
                // Right: Timecode Display & Zoom Controls
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        // Current Playhead Timecode / Duration Timecode
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .bg(theme.muted)
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .font_family(mono_font.clone())
                                    .text_color(theme.accent)
                                    .child(playhead_timecode),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family(mono_font.clone())
                                    .text_color(theme.muted_foreground)
                                    .child(format!("/ {duration_timecode}")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Button::new("zoom-out")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Minus)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.zoom_out(cx);
                                    })),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_family(mono_font)
                                    .text_color(theme.muted_foreground)
                                    .w_12()
                                    .text_center()
                                    .child(format!("{}%", self.zoom_level)),
                            )
                            .child(
                                Button::new("zoom-in")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Plus)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.zoom_in(cx);
                                    })),
                            ),
                    ),
            )
    }

    /// Render scrollable timeline canvas area with tracks, ruler, clips, and playhead.
    fn render_canvas(&self, theme: &Theme, ppf: f32, cx: &Context<Self>) -> impl IntoElement {
        let playhead_frame = self.selection.playhead_frame;
        let total_clips: usize = self.sequence.tracks.iter().map(|t| t.clips.len()).sum();

        div()
            .flex()
            .flex_1()
            .overflow_hidden()
            .child(
                // Left Track Headers Column
                div()
                    .flex()
                    .flex_col()
                    .w(metrics::TIMELINE_TRACK_HEADER_WIDTH)
                    .border_r_1()
                    .border_color(theme.border)
                    .bg(theme.secondary)
                    .child(
                        // Ruler corner header
                        div()
                            .h(metrics::TIMELINE_RULER_HEIGHT)
                            .border_b_1()
                            .border_color(theme.border)
                            .flex()
                            .items_center()
                            .px_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.muted_foreground)
                                    .child("TRACKS"),
                            ),
                    )
                    .children(
                        self.sequence
                            .tracks
                            .iter()
                            .map(|track| self.render_track_header(track, theme, cx)),
                    ),
            )
            .child(
                // Right Scrollable Timeline Canvas
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_hidden()
                    .bg(theme.background)
                    .child(
                        // Interactive Time Ruler Bar
                        div()
                            .flex()
                            .items_center()
                            .h(metrics::TIMELINE_RULER_HEIGHT)
                            .border_b_1()
                            .border_color(theme.border)
                            .bg(theme.secondary)
                            .relative()
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                    this.is_scrubbing = true;
                                    let x = f32::from(event.position.x);
                                    let frame = (x / ppf).max(0.0) as i64;
                                    this.set_playhead(frame, cx);
                                }),
                            )
                            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                                if this.is_scrubbing {
                                    let x = f32::from(event.position.x);
                                    let frame = (x / ppf).max(0.0) as i64;
                                    this.set_playhead(frame, cx);
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.is_scrubbing = false;
                                    cx.notify();
                                }),
                            )
                            .children(self.render_ruler_ticks(theme, ppf))
                            .child(
                                // Red playhead scrubber cap on ruler
                                div()
                                    .absolute()
                                    .top_1()
                                    .left(px((playhead_frame as f32).mul_add(ppf, -6.0).max(-6.0)))
                                    .w(px(14.0))
                                    .h(px(16.0))
                                    .rounded_xs()
                                    .bg(theme.danger)
                                    .border_1()
                                    .border_color(gpui::rgb(0xffffff).opacity(0.85))
                                    .shadow_sm()
                                    .cursor_col_resize(),
                            ),
                    )
                    .child(
                        // Track Lanes Canvas
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .relative()
                            .overflow_hidden()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                    this.is_scrubbing = true;
                                    let x = f32::from(event.position.x);
                                    let frame = (x / ppf).max(0.0) as i64;
                                    this.set_playhead(frame, cx);
                                }),
                            )
                            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                                if this.is_scrubbing {
                                    let x = f32::from(event.position.x);
                                    let frame = (x / ppf).max(0.0) as i64;
                                    this.set_playhead(frame, cx);
                                }
                            }))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.is_scrubbing = false;
                                    cx.notify();
                                }),
                            )
                            .children(
                                self.sequence
                                    .tracks
                                    .iter()
                                    .map(|track| self.render_track_lane(track, theme, ppf, cx)),
                            )
                            .child(
                                // Playhead indicator vertical line
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left(px((playhead_frame as f32 * ppf).max(0.0)))
                                    .w(px(2.0))
                                    .bg(theme.danger),
                            )
                            .when(total_clips == 0, |this| {
                                let default_v1 = self
                                    .sequence
                                    .tracks
                                    .iter()
                                    .find(|t| t.track_type == TrackType::Video)
                                    .map(|t| t.id);

                                this.child(
                                    // Empty Timeline State Overlay
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .flex_1()
                                        .p_4()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                                this.is_scrubbing = true;
                                                let x = f32::from(event.position.x);
                                                let frame = (x / ppf).max(0.0) as i64;
                                                this.set_playhead(frame, cx);
                                            }),
                                        )
                                        .when_some(default_v1, |this, v1_id| {
                                            this.on_drop(cx.listener(move |this, asset: &MediaAsset, window, cx| {
                                                let mouse_x: f32 = window.mouse_position().x.into();
                                                let timeline_x = (mouse_x - 130.0).max(0.0);
                                                let target_frame = (timeline_x / ppf) as i64;
                                                this.handle_asset_drop_with_frame(v1_id, asset, Some(target_frame), cx);
                                            }))
                                            .on_drop(cx.listener(move |this, paths: &ExternalPaths, window, cx| {
                                                let mouse_x: f32 = window.mouse_position().x.into();
                                                let timeline_x = (mouse_x - 130.0).max(0.0);
                                                let target_frame = (timeline_x / ppf) as i64;
                                                if let Some(cb) = this.on_external_drop.clone() {
                                                    cb(paths.0.to_vec(), Some(v1_id), Some(target_frame), window, cx);
                                                }
                                            }))
                                        })
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_medium()
                                                .text_color(theme.muted_foreground)
                                                .child("Timeline is empty. Drag video or audio from the Media Browser or Desktop to start editing."),
                                        ),
                                )
                            }),
                    ),
            )
    }
}

impl TimelinePanel {
    /// Render track header containing track label, mute, solo, and lock controls.
    fn render_track_header(&self, track: &Track, theme: &Theme, cx: &Context<Self>) -> AnyElement {
        let track_id = track.id;
        let is_muted = track.muted;
        let is_soloed = track.soloed;
        let is_locked = track.locked;

        let icon = match track.track_type {
            TrackType::Video => IconName::Film,
            TrackType::Audio => IconName::Volume2,
        };

        div()
            .id(ElementId::Name(
                format!("track-header-{}", track.name).into(),
            ))
            .flex()
            .items_center()
            .justify_between()
            .h(metrics::TIMELINE_TRACK_HEIGHT)
            .px_2()
            .border_b_1()
            .border_color(theme.border)
            .bg(if is_locked {
                theme.muted
            } else {
                theme.secondary
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(Icon::new(icon))
                    .child(
                        div()
                            .text_xs()
                            .font_bold()
                            .text_color(theme.foreground)
                            .child(track.name.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    // Mute Button (M)
                    .child(
                        Button::new(format!("mute-{}", track.id))
                            .xsmall()
                            .ghost()
                            .label("M")
                            .when(is_muted, |btn| btn.danger())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_track_mute(track_id, cx);
                            })),
                    )
                    // Solo Button (S)
                    .child(
                        Button::new(format!("solo-{}", track.id))
                            .xsmall()
                            .ghost()
                            .label("S")
                            .when(is_soloed, |btn| btn.outline())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_track_solo(track_id, cx);
                            })),
                    )
                    // Lock Button (L)
                    .child(
                        Button::new(format!("lock-{}", track.id))
                            .xsmall()
                            .ghost()
                            .label("L")
                            .when(is_locked, |btn| btn.danger())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_track_lock(track_id, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    /// Render ruler ticks based on zoom and FPS.
    fn render_ruler_ticks(&self, theme: &Theme, ppf: f32) -> Vec<AnyElement> {
        let fps = self.sequence.fps.rounded_int().max(1);
        let mono_font = theme.mono_font_family.clone();

        // Major interval in seconds based on zoom
        let interval_seconds = if self.zoom_level >= 200 {
            1
        } else if self.zoom_level >= 100 {
            5
        } else if self.zoom_level >= 50 {
            10
        } else {
            30
        };

        let interval_frames = interval_seconds * fps;
        let mut ticks = Vec::new();

        for i in 0..20 {
            let frame = i * interval_frames;
            let left_px = frame as f32 * ppf;
            let timecode = self.sequence.frame_to_timecode(frame);

            ticks.push(
                div()
                    .absolute()
                    .left(px(left_px))
                    .top_0()
                    .bottom_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_1()
                    .child(div().w(px(1.0)).h_3().bg(theme.border))
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font.clone())
                            .text_color(theme.muted_foreground)
                            .child(timecode),
                    )
                    .into_any_element(),
            );
        }

        ticks
    }

    /// Render a single track lane with its placed clips.
    fn render_track_lane(
        &self,
        track: &Track,
        theme: &Theme,
        ppf: f32,
        cx: &Context<Self>,
    ) -> AnyElement {
        let track_id = track.id;
        let is_locked = track.locked;

        div()
            .id(ElementId::Name(format!("lane-{}", track.name).into()))
            .h(metrics::TIMELINE_TRACK_HEIGHT)
            .w_full()
            .relative()
            .border_b_1()
            .border_color(theme.border)
            .bg(if is_locked {
                theme.muted
            } else {
                theme.background
            })
            // Dropping an imported asset from Media Browser onto this track
            .on_drop(cx.listener(move |this, asset: &MediaAsset, window, cx| {
                let mouse_x: f32 = window.mouse_position().x.into();
                let timeline_x = (mouse_x - 130.0).max(0.0);
                let target_frame = (timeline_x / ppf) as i64;
                this.handle_asset_drop_with_frame(track_id, asset, Some(target_frame), cx);
            }))
            // Dropping an external file from Finder/OS onto this track
            .on_drop(cx.listener(move |this, paths: &ExternalPaths, window, cx| {
                let mouse_x: f32 = window.mouse_position().x.into();
                let timeline_x = (mouse_x - 130.0).max(0.0);
                let target_frame = (timeline_x / ppf) as i64;
                if let Some(cb) = this.on_external_drop.clone() {
                    cb(paths.0.to_vec(), Some(track_id), Some(target_frame), window, cx);
                }
            }))
            .children(
                track
                    .clips
                    .iter()
                    .map(|clip| self.render_clip_element(clip, track, theme, ppf, cx)),
            )
            .children(
                track
                    .transitions
                    .iter()
                    .map(|trans| self.render_transition_element(trans, track, theme, ppf, cx)),
            )
            .into_any_element()
    }

    /// Render individual clip element placed on a track lane.
    fn render_clip_element(
        &self,
        clip: &Clip,
        track: &Track,
        theme: &Theme,
        ppf: f32,
        cx: &Context<Self>,
    ) -> AnyElement {
        let clip_id = clip.id;
        let track_id = track.id;
        let is_selected = self.selection.is_selected(clip_id);
        let left_px = (clip.start_frame as f32 * ppf).max(0.0);
        let width_px = (clip.duration_frames() as f32 * ppf).max(18.0);
        let mono_font = theme.mono_font_family.clone();
        let clip_name = clip.name.clone();
        let clip_duration = clip.duration_frames();
        let clip_source_out = clip.source_out;
        let duration_text = self.sequence.frame_to_timecode(clip_duration);
        let current_tool = self.current_tool;
        let playhead_frame = self.selection.playhead_frame;
        let is_audio = track.track_type == TrackType::Audio;
        let num_bars = ((width_px - 8.0) / 3.0).clamp(4.0, 120.0) as usize;

        // Styling based on track type and overlay content
        let (bg_color, border_color, badge_pill): (Rgba, Hsla, Option<AnyElement>) =
            if clip.text_overlay.is_some() {
                let is_sub = clip_name.to_lowercase().contains("subtitle")
                    || clip_name.to_lowercase().contains("caption");
                if is_sub {
                    (
                        gpui::rgb(0x451a03),        // Dark amber
                        gpui::rgb(0xf59e0b).into(), // Amber
                        Some(
                            div()
                                .px_1()
                                .py_0p5()
                                .rounded_sm()
                                .bg(gpui::rgb(0xb45309))
                                .text_xs()
                                .font_bold()
                                .text_color(gpui::rgb(0xfef3c7))
                                .child("SUB")
                                .into_any_element(),
                        ),
                    )
                } else {
                    (
                        gpui::rgb(0x2e1065),        // Dark violet
                        gpui::rgb(0xa855f7).into(), // Purple
                        Some(
                            div()
                                .px_1()
                                .py_0p5()
                                .rounded_sm()
                                .bg(gpui::rgb(0x7c3aed))
                                .text_xs()
                                .font_bold()
                                .text_color(gpui::rgb(0xf3e8ff))
                                .child("TXT")
                                .into_any_element(),
                        ),
                    )
                }
            } else {
                match track.track_type {
                    TrackType::Video => (
                        gpui::rgb(0x1e293b),        // Dark slate/blue
                        gpui::rgb(0x38bdf8).into(), // Light sky blue
                        None,
                    ),
                    TrackType::Audio => (
                        gpui::rgb(0x134e4a),        // Dark teal/emerald
                        gpui::rgb(0x34d399).into(), // Emerald
                        None,
                    ),
                }
            };

        div()
            .id(ElementId::Name(format!("clip-{clip_id}").into()))
            .absolute()
            .top(px(4.0))
            .bottom(px(4.0))
            .left(px(left_px))
            .w(px(width_px))
            .rounded_sm()
            .bg(bg_color)
            .border_1()
            .border_color(if is_selected {
                theme.accent
            } else {
                border_color
            })
            .cursor_pointer()
            .overflow_hidden()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_transition = None;
                match current_tool {
                    TimelineTool::Select => {
                        this.selection.select_single(clip_id);
                        cx.notify();
                    }
                    TimelineTool::Split => {
                        // Razor split at playhead or clip
                        let new_clip_id = ClipId::new();
                        let cmd = EditorCommand::SplitClip(SplitClip {
                            original_clip_id: clip_id,
                            track_id,
                            split_frame: playhead_frame,
                            new_clip_id,
                            original_source_out: clip_source_out,
                        });
                        let _ = this.history.execute(cmd, &mut this.sequence);
                        this.selection.select_single(new_clip_id);
                        cx.notify();
                    }
                }
            }))
            .when(is_audio, |this| {
                let peaks = self.get_clip_waveform_peaks(clip, num_bars);
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_start()
                        .gap(px(1.0))
                        .px_1()
                        .opacity(0.40)
                        .children(peaks.into_iter().map(|p| {
                            let bar_h = (p * 24.0).clamp(2.0, 26.0);
                            div()
                                .w(px(2.0))
                                .h(px(bar_h))
                                .rounded_xs()
                                .bg(gpui::rgb(0x34d399))
                        })),
                )
            })
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .size_full()
                    .px_1p5()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .overflow_hidden()
                            .children(badge_pill)
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .truncate()
                                    .child(clip_name),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font)
                            .text_color(theme.muted_foreground)
                            .child(duration_text),
                    ),
            )
            .into_any_element()
    }

    /// Render individual transition element spanning cut points on a track lane.
    fn render_transition_element(
        &self,
        transition: &Transition,
        _track: &Track,
        theme: &Theme,
        ppf: f32,
        cx: &Context<Self>,
    ) -> AnyElement {
        let transition_id = transition.id;
        let is_selected = self.selected_transition == Some(transition_id);
        let start_frame = transition.start_frame();
        let left_px = (start_frame as f32 * ppf).max(0.0);
        let width_px = (transition.duration_frames as f32 * ppf).max(18.0);
        let mono_font = theme.mono_font_family.clone();
        let duration_text = format!("{}f", transition.duration_frames);
        let trans_label = match transition.transition_type {
            TransitionType::Crossfade => "X-Fade",
            TransitionType::Dissolve => "Dissolve",
            TransitionType::FadeToBlack => "Fade Black",
            TransitionType::DipToWhite => "Dip White",
            TransitionType::WipeLeft => "Wipe L",
            TransitionType::WipeRight => "Wipe R",
        };

        div()
            .id(ElementId::Name(
                format!("transition-{transition_id}").into(),
            ))
            .absolute()
            .top(px(8.0))
            .bottom(px(8.0))
            .left(px(left_px))
            .w(px(width_px))
            .rounded_sm()
            .bg(gpui::rgb(0x1e1b4b).opacity(0.85))
            .border_1()
            .border_color(if is_selected {
                theme.accent
            } else {
                gpui::rgb(0x818cf8).into()
            })
            .cursor_pointer()
            .overflow_hidden()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selection.clear_selection();
                this.selected_transition = Some(transition_id);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .size_full()
                    .px_1()
                    .child(
                        div()
                            .text_xs()
                            .font_bold()
                            .text_color(gpui::rgb(0xe0e7ff))
                            .truncate()
                            .child(trans_label),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font)
                            .text_color(gpui::rgb(0xa5b4fc))
                            .child(duration_text),
                    ),
            )
            .into_any_element()
    }
}
