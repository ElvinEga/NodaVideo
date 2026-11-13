use crate::asset::IconName;
use crate::clip_placement::DEFAULT_IMAGE_DURATION_SECONDS;
use crate::context_menu::ContextMenu;
use crate::edit_action::EditAction;
use crate::editor::Editor;
use crate::event_bus::{AppEvent, AssetDragMoveEvent};
use crate::explorer_drag::AssetBeingDragged;
use crate::explorer_file_entry::{FileTreeEntry, FileTreeEntryKind};
use crate::frame_preview::{HoverPreview, hover_preview_at};
use crate::layout::{
    RULER_HEIGHT, TIMELINE_HEADER_HEIGHT, TIMELINE_PADDING, TRACK_HEADER_WIDTH, TRACK_HEIGHT,
};
use crate::preview::PreviewTarget;
use crate::theme::{ACCENT, BORDER, ERROR, MUTED, SURFACE, SURFACE_HOVER, TEXT};
use crate::timeline::TimelineFrameIndex;
use crate::timeline_document;
use crate::timeline_interactions::{MarqueeSelection, TimelineTool};
use crate::title::TextTitle;
use crate::track::TrackKind;
use gpui::prelude::*;
use gpui::{
    Anchor, AnchoredPositionMode, Bounds, ClickEvent, CursorStyle, DragMoveEvent, ElementId, Empty,
    ExternalPaths, FontWeight, MouseButton, MouseDownEvent, MouseUpEvent, ObjectFit, PathBuilder,
    Window, anchored, canvas, deferred, div, img, point, px, relative, rgb, rgba, svg,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use ulid::Ulid;

// 捏合增量倍率；越大缩放越灵敏。It's fun to be PI.
const TIMELINE_PINCH_SENSITIVITY: f32 = std::f32::consts::PI;

struct RulerDrag;

const MAX_RULER_TICKS: usize = 240;
const MIN_RULER_LABEL_SPACING: f32 = 72.0;
const MIN_FRAME_TICK_SPACING: f32 = 4.0;
const FRAME_TICK_OVERSCAN: f32 = 120.0;

const SEEK_POPUP_WIDTH: f32 = 80.0;
const SEEK_THUMB_WIDTH: f32 = 72.0;
const SEEK_THUMB_MAX_HEIGHT: f32 = 128.0;
const SEEK_THUMB_MIN_HEIGHT: f32 = 40.0;
const TICK_STEPS: [f64; 12] = [
    1.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0,
];

fn timeline_marquee(selection: &MarqueeSelection) -> gpui::AnyElement {
    let left = selection.start_x.min(selection.current_x);
    let top = selection.start_y.min(selection.current_y);
    let width = (selection.start_x - selection.current_x).abs();
    let height = (selection.start_y - selection.current_y).abs();

    div()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(width))
        .h(px(height))
        .border_1()
        .border_color(rgb(ACCENT))
        .bg(gpui::rgba(0xf0b75e24))
        .into_any_element()
}

impl Editor {
    pub(super) fn timeline_view(
        &self,
        editor_width: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(timeline) = self.timeline.as_ref() else {
            return self.empty_timeline_view(cx);
        };
        let frames_per_second = timeline
            .editing_state
            .settings
            .frame_rate
            .frames_per_second();

        div()
            .id("editor-timeline")
            .relative()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(0x0a0a0c))
            .on_mouse_move(cx.listener(Self::update_clip_move))
            .on_mouse_move(cx.listener(Self::update_marquee_selection))
            .on_scroll(cx.listener(Self::finish_timeline_scroll))
            .on_pinch(cx.listener(pinch_timeline))
            .on_mouse_up(MouseButton::Left, cx.listener(finish_clip_move))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(Self::finish_marquee_selection),
            )
            .on_mouse_up_out(MouseButton::Left, cx.listener(finish_clip_move))
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(Self::finish_marquee_selection),
            )
            .child(self.timeline_toolbar(frames_per_second, cx))
            .child(self.timeline_tracks_container(editor_width, cx))
            .when_some(
                timeline.interaction.marquee_selection.as_ref(),
                |this, selection| this.child(timeline_marquee(selection)),
            )
            .into_any_element()
    }

    fn timeline_tracks_container(
        &self,
        editor_width: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let timeline = self
            .timeline
            .as_ref()
            .expect("timeline view requires timeline state");
        let mut displayed_end = timeline.editing_state.content_duration();
        if let Some(drag) = &timeline.interaction.clip_move_drag {
            let starts = drag
                .placements
                .iter()
                .map(|(clip_id, _, start)| (*clip_id, *start))
                .collect::<HashMap<_, _>>();
            for clip in &timeline.editing_state.clips {
                let Some(start) = starts.get(&clip.id()) else {
                    continue;
                };
                displayed_end = displayed_end
                    .max(*start + clip.frame_length(timeline.editing_state.settings.frame_rate));
            }
        }
        let viewport_width = (editor_width - TRACK_HEADER_WIDTH).max(0.0);
        let content_end = TIMELINE_PADDING
            + timeline.editing_state.seconds(displayed_end) as f32 * timeline.pixels_per_second;
        // 末尾预留半个可视区域，使内容结尾或拖动片段的结尾能滚动到标尺中点。
        let timeline_width = (content_end + viewport_width * 0.5).max(viewport_width);
        let max_scroll = ((timeline_width - viewport_width) * 100.0).round() / 100.0;
        // 滚轮偏移在 GPUI 绘制前才会限界；标尺构建时先限界，避免读到临时越界值而抖动。
        let scroll_offset_x = f32::from(timeline.h_scroll.offset().x).clamp(-max_scroll, 0.0);
        let duration =
            ((timeline_width - TIMELINE_PADDING).max(0.0) / timeline.pixels_per_second) as f64;
        let track_headers = timeline
            .editing_state
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| self.track_header(index, track, cx))
            .collect::<Vec<_>>();
        let track_rows = timeline
            .editing_state
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| self.track_row(index, track, timeline_width, cx))
            .collect::<Vec<_>>();
        // 悬停预览：拖动、框选与素材拖放期间不弹出，避免与正在进行的编辑重叠。
        let hover_popup = timeline.interaction.pointer_guide.and_then(|position| {
            if timeline.interaction.clip_move_drag.is_some()
                || timeline.interaction.marquee_selection.is_some()
                || cx.has_active_drag()
            {
                return None;
            }
            let seconds = timeline.editing_state.seconds(position);
            let guide_x =
                TIMELINE_PADDING + seconds as f32 * timeline.pixels_per_second + scroll_offset_x;
            let left = (guide_x - SEEK_POPUP_WIDTH / 2.0)
                .clamp(0.0, (viewport_width - SEEK_POPUP_WIDTH).max(0.0));
            let caret_x = (guide_x - left).clamp(10.0, SEEK_POPUP_WIDTH - 10.0);
            Some((
                hover_preview_at(timeline, position),
                format_time_precise(seconds),
                left,
                caret_x,
            ))
        });
        let tracks = div()
            .id("timeline-tracks-vertical-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .restrict_scroll_to_axis()
            .track_scroll(&timeline.v_scroll)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|editor, event: &MouseDownEvent, window, cx| {
                    editor.begin_marquee_selection(event, window, cx);
                }),
            )
            .child(
                div()
                    .h(px(RULER_HEIGHT
                        + timeline.editing_state.tracks.len() as f32
                            * TRACK_HEIGHT))
                    .min_h_full()
                    .w_full()
                    .flex()
                    .child(
                        div()
                            .w(px(TRACK_HEADER_WIDTH))
                            .h_full()
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .border_r_1()
                            .border_color(rgb(BORDER))
                            .child(
                                div()
                                    .h(px(RULER_HEIGHT))
                                    .flex_shrink_0()
                                    .border_b_1()
                                    .border_color(rgb(BORDER)),
                            )
                            .children(track_headers),
                    )
                    .child(
                        div()
                            .id("editor-timeline-scroll")
                            .min_w_0()
                            .flex_1()
                            .h_full()
                            .overflow_x_scroll()
                            .restrict_scroll_to_axis()
                            .track_scroll(&timeline.h_scroll)
                            .cursor(match timeline.interaction.active_tool {
                                TimelineTool::Blade => CursorStyle::Crosshair,
                                TimelineTool::Selection => CursorStyle::Arrow,
                            })
                            .on_mouse_move(cx.listener(Self::update_pointer_guide))
                            .on_hover(cx.listener(Self::update_pointer_guide_hover))
                            .child(
                                div()
                                    .relative()
                                    .w(px(timeline_width))
                                    .min_h_full()
                                    .on_drag_move::<FileTreeEntry>(cx.listener(
                                        move |editor,
                                              event: &DragMoveEvent<FileTreeEntry>,
                                              _,
                                              cx| {
                                            let event = AssetDragMoveEvent {
                                                event: event.event.clone(),
                                                bounds: event.bounds,
                                            };
                                            editor.emit_event(cx, AppEvent::DragMove(event));
                                        },
                                    ))
                                    .on_drop(cx.listener(
                                        move |editor, _drag: &FileTreeEntry, _, cx| {
                                            editor.emit_event(cx, AppEvent::DragDrop);
                                        },
                                    ))
                                    .on_drag_move::<TextTitle>(cx.listener(
                                        move |editor, event: &DragMoveEvent<TextTitle>, _, cx| {
                                            let event = AssetDragMoveEvent {
                                                event: event.event.clone(),
                                                bounds: event.bounds,
                                            };
                                            editor.emit_event(cx, AppEvent::DragMove(event));
                                        },
                                    ))
                                    .on_drop(cx.listener(
                                        move |editor, _drag: &TextTitle, _, cx| {
                                            editor.emit_event(cx, AppEvent::DragDrop);
                                        },
                                    ))
                                    .on_drop(cx.listener(|editor, paths: &ExternalPaths, _, cx| {
                                        editor.handle_timeline_tracks_external_drop(
                                            paths.paths(),
                                            cx,
                                        );
                                    }))
                                    .on_mouse_up_out(
                                        MouseButton::Left,
                                        cx.listener(|editor, _, _, cx| {
                                            if !cx.has_active_drag() {
                                                return;
                                            }
                                            editor.active_asset_drag = AssetBeingDragged::None;
                                            if let Some(timeline) = editor.timeline.as_mut() {
                                                timeline.preview_drop_asset = None;
                                                timeline.interaction.snap_guide = None;
                                            }
                                            cx.notify();
                                        }),
                                    )
                                    .child(div().h(px(RULER_HEIGHT)).flex_shrink_0())
                                    .children(track_rows)
                                    .child(self.timeline_playhead(cx))
                                    .when_some(timeline.interaction.snap_guide, |this, guide| {
                                        let guide_left = TIMELINE_PADDING
                                            + timeline.editing_state.seconds(guide) as f32
                                                * timeline.pixels_per_second;
                                        this.child(
                                            div()
                                                .absolute()
                                                .top_0()
                                                .bottom_0()
                                                .left(px(guide_left))
                                                .w(px(2.0))
                                                .bg(rgb(0x63c8ff))
                                                .child(
                                                    div()
                                                        .absolute()
                                                        .top_0()
                                                        .left(px(-3.0))
                                                        .size_2()
                                                        .rounded_full()
                                                        .bg(rgb(0x63c8ff)),
                                                ),
                                        )
                                    })
                                    .when_some(
                                        timeline.interaction.pointer_guide,
                                        |this, position| {
                                            let guide_left = TIMELINE_PADDING
                                                + timeline.editing_state.seconds(position) as f32
                                                    * timeline.pixels_per_second;
                                            this.child(
                                                div()
                                                    .absolute()
                                                    .top_0()
                                                    .bottom_0()
                                                    .left(px(guide_left))
                                                    .w(px(2.0))
                                                    .bg(rgb(ERROR))
                                                    .when(
                                                        timeline.interaction.active_tool
                                                            == TimelineTool::Blade,
                                                        |this| this.cursor(CursorStyle::Crosshair),
                                                    ),
                                            )
                                        },
                                    )
                                    .when_some(
                                        match &self.context_menu {
                                            ContextMenu::TextTrack(menu) => Some(menu.position),
                                            ContextMenu::None
                                            | ContextMenu::File(_)
                                            | ContextMenu::TimelineClip(_) => None,
                                        },
                                        |this, position| {
                                            let guide_left = TIMELINE_PADDING
                                                + timeline.editing_state.seconds(position) as f32
                                                    * timeline.pixels_per_second;
                                            this.child(
                                                div()
                                                    .absolute()
                                                    .top_0()
                                                    .bottom_0()
                                                    .left(px(guide_left))
                                                    .w_0()
                                                    .border_l_1()
                                                    .border_dashed()
                                                    .border_color(rgb(ACCENT)),
                                            )
                                        },
                                    ),
                            ),
                    ),
            )
            .into_any_element();
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(tracks)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .h(px(RULER_HEIGHT))
                    .flex()
                    .occlude()
                    .bg(rgb(0x0a0a0c))
                    .child(
                        div()
                            .w(px(TRACK_HEADER_WIDTH))
                            .h_full()
                            .flex_shrink_0()
                            .border_r_1()
                            .border_b_1()
                            .border_color(rgb(BORDER)),
                    )
                    .child(
                        div()
                            .id("timeline-ruler-pointer-area")
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .overflow_hidden()
                            .on_mouse_move(cx.listener(Self::update_pointer_guide))
                            .on_hover(cx.listener(Self::update_pointer_guide_hover))
                            .child(
                                div()
                                    .absolute()
                                    .left(px(scroll_offset_x))
                                    .top_0()
                                    .w(px(timeline_width))
                                    .h_full()
                                    .child(self.timeline_ruler(
                                        duration,
                                        -scroll_offset_x,
                                        viewport_width,
                                        cx,
                                    ))
                                    .child(self.timeline_playhead(cx)),
                            )
                            .when_some(hover_popup, |this, (preview, timecode, left, caret_x)| {
                                this.child(
                                    deferred(
                                        anchored()
                                            .position_mode(AnchoredPositionMode::Local)
                                            .position(point(px(left), px(0.0)))
                                            .anchor(Anchor::BottomLeft)
                                            .offset(point(px(0.0), px(-6.0)))
                                            .child(
                                                self.seek_preview_popup(
                                                    &preview, timecode, caret_x,
                                                ),
                                            ),
                                    )
                                    .with_priority(1),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    fn timeline_playhead_seconds(&self, cx: &Context<Self>) -> f64 {
        let timeline = self
            .timeline
            .as_ref()
            .expect("timeline view requires timeline state");
        if let PreviewTarget::Timeline { path, player, .. } = &self.preview.target
            && self.project_root.join(path) == timeline.path
        {
            return player.read(cx).backend().position().as_secs_f64();
        }
        timeline.editing_state.seconds(timeline.playhead())
    }

    fn timeline_playhead(&self, cx: &Context<Self>) -> gpui::AnyElement {
        let timeline = self
            .timeline
            .as_ref()
            .expect("timeline view requires timeline state");
        let left = TIMELINE_PADDING
            + self.timeline_playhead_seconds(cx) as f32 * timeline.pixels_per_second;

        // 同一元素既画在标尺浮层里，也画在轨道滚动内容里；后者顶部 RULER_HEIGHT 被标尺浮层遮住。
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(left))
            .child(
                div()
                    .absolute()
                    .top(px(RULER_HEIGHT)) // 光晕只在轨道区，不压标尺文字。
                    .bottom_0()
                    .left(px(-1.0))
                    .w(px(3.0))
                    .bg(rgba(0xf0b75e2e)),
            )
            .child(
                div()
                    .absolute()
                    .top(px(26.0)) // 从标记尖端开始，标记以上不画线。
                    .bottom_0()
                    .left_0()
                    .w(px(1.0))
                    .bg(rgb(ACCENT)),
            )
            .child(
                svg()
                    .path(IconName::Playhead.path())
                    .absolute()
                    .top(px(14.0)) // 位于标尺下半部，避开时间标签。
                    .left(px(-5.5))
                    .w(px(12.0))
                    .h(px(13.0))
                    .text_color(rgb(ACCENT)),
            )
            .into_any_element()
    }

    fn seek_preview_popup(
        &self,
        preview: &HoverPreview,
        timecode: String,
        caret_x: f32,
    ) -> gpui::AnyElement {
        let (thumb, thumb_height) = match preview {
            HoverPreview::Video { key, aspect } => {
                let height = seek_thumb_height(*aspect);
                let content = match self.frame_image(key) {
                    Some(image) => img(image)
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .into_any_element(),
                    None => div()
                        .size_full()
                        .bg(rgb(if self.frame_failed(key) {
                            0x000000
                        } else {
                            SURFACE_HOVER
                        }))
                        .into_any_element(),
                };
                (content, height)
            }
            HoverPreview::Image { path, aspect } => (
                img(path.clone())
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element(),
                seek_thumb_height(*aspect),
            ),
            HoverPreview::Empty => (
                div().size_full().bg(rgb(0x000000)).into_any_element(),
                SEEK_THUMB_MAX_HEIGHT,
            ),
        };
        let card_height = thumb_height + 30.0; // 4 内边距 + 缩略图 + 4 间距 + 18 时间码 + 4 内边距
        div()
            .relative()
            .w(px(SEEK_POPUP_WIDTH))
            .child(
                div()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SURFACE))
                    .p(px(4.0))
                    .shadow_lg()
                    .child(
                        div()
                            .w(px(SEEK_THUMB_WIDTH))
                            .h(px(thumb_height))
                            .rounded(px(4.0))
                            .overflow_hidden()
                            .bg(rgb(0x000000))
                            .child(thumb),
                    )
                    .child(
                        div()
                            .mt(px(4.0))
                            .h(px(18.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .font_family("monospace")
                            .text_size(px(11.0))
                            .text_color(rgb(TEXT))
                            .child(timecode),
                    ),
            )
            .child(seek_preview_caret(caret_x, card_height))
            .into_any_element()
    }

    fn timeline_ruler(
        &self,
        duration: f64,
        scroll_left: f32,
        viewport_width: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let timeline = self
            .timeline
            .as_ref()
            .expect("timeline view requires timeline state");
        let frame_rate = timeline.editing_state.settings.frame_rate;
        let frames_per_second = frame_rate.frames_per_second();
        let displayed_frames = i64::from(frame_rate.ceil(duration)).max(1);
        let pixels_per_frame = timeline.pixels_per_second / frames_per_second as f32;
        let frame_step = frame_tick_step(pixels_per_frame);
        let visible_start = ((scroll_left - FRAME_TICK_OVERSCAN - TIMELINE_PADDING).max(0.0)
            / pixels_per_frame.max(f32::EPSILON))
        .floor() as i64;
        let visible_end = ((scroll_left + viewport_width + FRAME_TICK_OVERSCAN - TIMELINE_PADDING)
            .max(0.0)
            / pixels_per_frame.max(f32::EPSILON))
        .ceil() as i64;
        let first_frame = visible_start
            .div_euclid(frame_step)
            .saturating_mul(frame_step)
            .max(frame_step);
        let last_frame = visible_end.min(displayed_frames);
        let nominal_fps = frames_per_second.round().max(1.0) as i64;
        let frame_ticks = (first_frame..=last_frame)
            .step_by(frame_step as usize)
            .map(|frame| {
                let emphasized = frame % nominal_fps == 0;
                let medium = !emphasized && frame % 5 == 0;
                let height = if emphasized {
                    12.0
                } else if medium {
                    8.0
                } else {
                    5.0
                };
                div()
                    .absolute()
                    .left(px(TIMELINE_PADDING
                        + frame_rate.seconds(TimelineFrameIndex::from(frame)) as f32
                            * timeline.pixels_per_second))
                    .bottom_0()
                    .h(px(height))
                    .border_l_1()
                    .border_color(rgb(if emphasized {
                        0x5a5a62
                    } else {
                        0x3a3a40
                    }))
            });
        let tick_step = ruler_tick_step(duration, timeline.pixels_per_second);
        let tick_count = (duration / tick_step).ceil() as usize + 1;
        let ruler_ticks = (0..tick_count).map(|index| {
            let time = index as f64 * tick_step;
            div()
                .absolute()
                .left(px(
                    TIMELINE_PADDING + time as f32 * timeline.pixels_per_second
                ))
                .top_0()
                .h_full()
                .border_l_1()
                .border_color(rgb(0x333338))
                .pl_1()
                .font_family("monospace")
                .text_xs()
                .text_color(rgb(MUTED))
                .child(format_time_precise(time))
        });
        div()
            .id("timeline-ruler")
            .on_pinch(cx.listener(pinch_timeline))
            .on_click(cx.listener(|editor, event: &gpui::ClickEvent, window, cx| {
                editor.ui_callback_seek_ruler(event.position().x.into(), window, cx);
            }))
            .on_drag(RulerDrag, |_, _, _, cx| cx.new(|_| Empty))
            .on_drag_move(
                cx.listener(|editor, event: &DragMoveEvent<RulerDrag>, window, cx| {
                    editor.ui_callback_seek_ruler(event.event.position.x.into(), window, cx);
                }),
            )
            .relative()
            .w_full()
            .h(px(RULER_HEIGHT))
            .border_b_1()
            .border_color(rgb(BORDER))
            .children(frame_ticks)
            .children(ruler_ticks)
            .into_any_element()
    }

    fn timeline_toolbar(&self, frames_per_second: f64, cx: &mut Context<Self>) -> gpui::AnyElement {
        let timeline = self
            .timeline
            .as_ref()
            .expect("timeline view requires timeline state");
        let selection_active = timeline.interaction.active_tool == TimelineTool::Selection;
        let blade_active = timeline.interaction.active_tool == TimelineTool::Blade;
        let snap_active = timeline.snapping_enabled;
        let magnet_active = timeline.track_magnet_enabled;

        let selection_icon_color = if selection_active {
            rgb(0x818cf8)
        } else {
            rgb(MUTED)
        };
        let blade_icon_color = if blade_active {
            rgb(0x818cf8)
        } else {
            rgb(MUTED)
        };
        let snap_icon_color = if snap_active {
            rgb(0x818cf8)
        } else {
            rgb(MUTED)
        };
        let magnet_icon_color = if magnet_active {
            rgb(0x818cf8)
        } else {
            rgb(MUTED)
        };

        let timeline_title = timeline
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Sequence 01".to_string());

        let playhead_frame = timeline.playhead();
        let timecode_text = format_smpte_timecode(playhead_frame, frames_per_second);

        let min_log = 0.1_f32.ln();
        let max_log = 1000.0_f32.ln();
        let current_log = timeline.pixels_per_second.clamp(0.1, 1000.0).ln();
        let zoom_fraction = ((current_log - min_log) / (max_log - min_log)).clamp(0.0, 1.0);
        let zoom_track_bounds = Rc::new(Cell::new(Bounds::default()));

        div()
            .id("timeline-toolbar")
            .h(px(TIMELINE_HEADER_HEIGHT))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .border_b_1()
            .border_color(rgb(BORDER))
            // Left Group: Identity, Tools, Add Track
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    // Sequence Name
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT))
                            .child(timeline_title),
                    )
                    // Frame Rate
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(rgb(MUTED))
                            .child(format!("{frames_per_second:.0} fps")),
                    )
                    // Divider
                    .child(div().h(px(14.0)).border_r_1().border_color(rgb(BORDER)))
                    // Primary Edit Tools (Icon-only with tooltips and active state)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            // Select Tool
                            .child(
                                div()
                                    .id("timeline-selection-tool")
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .when(selection_active, |this| this.bg(rgb(0x312e81)))
                                    .when(!selection_active, |this| {
                                        this.hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    })
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view(
                                            "Selection Tool",
                                            Some("V"),
                                            cx,
                                        )
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        let Some(timeline) = editor.timeline.as_mut() else {
                                            return;
                                        };
                                        timeline.activate_timeline_tool(TimelineTool::Selection);
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Pointer.path())
                                            .size(px(13.0))
                                            .text_color(selection_icon_color)
                                            .when(!selection_active, |this| {
                                                this.hover(|s| s.text_color(rgb(TEXT)))
                                            }),
                                    ),
                            )
                            // Blade Tool
                            .child(
                                div()
                                    .id("timeline-blade-tool")
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .when(blade_active, |this| this.bg(rgb(0x312e81)))
                                    .when(!blade_active, |this| {
                                        this.hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    })
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view("Blade Tool", Some("B"), cx)
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        let Some(timeline) = editor.timeline.as_mut() else {
                                            return;
                                        };
                                        timeline.activate_timeline_tool(TimelineTool::Blade);
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Scissors.path())
                                            .size(px(13.0))
                                            .text_color(blade_icon_color)
                                            .when(!blade_active, |this| {
                                                this.hover(|s| s.text_color(rgb(TEXT)))
                                            }),
                                    ),
                            )
                            // Snap Toggle
                            .child(
                                div()
                                    .id("toggle-timeline-snapping")
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .when(snap_active, |this| this.bg(rgb(0x312e81)))
                                    .when(!snap_active, |this| {
                                        this.hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    })
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view("Snapping", Some("S"), cx)
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        editor.toggle_snapping();
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Snap.path())
                                            .size(px(13.0))
                                            .text_color(snap_icon_color)
                                            .when(!snap_active, |this| {
                                                this.hover(|s| s.text_color(rgb(TEXT)))
                                            }),
                                    ),
                            )
                            // Magnet Toggle
                            .child(
                                div()
                                    .id("toggle-track-magnet")
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .when(magnet_active, |this| this.bg(rgb(0x312e81)))
                                    .when(!magnet_active, |this| {
                                        this.hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    })
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view(
                                            "Track Magnet",
                                            Some("Ripple"),
                                            cx,
                                        )
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        editor.toggle_track_magnet();
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Magnet.path())
                                            .size(px(13.0))
                                            .text_color(magnet_icon_color)
                                            .when(!magnet_active, |this| {
                                                this.hover(|s| s.text_color(rgb(TEXT)))
                                            }),
                                    ),
                            ),
                    )
                    // Divider
                    .child(div().h(px(14.0)).border_r_1().border_color(rgb(BORDER)))
                    // Add Track Tools (Icon buttons with tooltips)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            // Add Video Track
                            .child(
                                div()
                                    .id("add-video-track")
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view("Add Video Track", None, cx)
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        if let Err(error) = editor.add_track(TrackKind::Video) {
                                            log::error!("{error:?}");
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Video.path())
                                            .size(px(13.0))
                                            .text_color(rgb(MUTED))
                                            .hover(|s| s.text_color(rgb(TEXT))),
                                    ),
                            )
                            // Add Audio Track
                            .child(
                                div()
                                    .id("add-audio-track")
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view("Add Audio Track", None, cx)
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        if let Err(error) = editor.add_track(TrackKind::Audio) {
                                            log::error!("{error:?}");
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Mic.path())
                                            .size(px(13.0))
                                            .text_color(rgb(MUTED))
                                            .hover(|s| s.text_color(rgb(TEXT))),
                                    ),
                            )
                            // Add Text Track
                            .child(
                                div()
                                    .id("add-text-track")
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view("Add Text Track", None, cx)
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        if let Err(error) = editor.add_track(TrackKind::Text) {
                                            log::error!("{error:?}");
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Text.path())
                                            .size(px(13.0))
                                            .text_color(rgb(MUTED))
                                            .hover(|s| s.text_color(rgb(TEXT))),
                                    ),
                            ),
                    ),
            )
            // Right Group: Playhead Timecode & Sleek Zoom Slider
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    // Timecode in accent-soft monospace
                    .child(
                        div()
                            .font_family("monospace")
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0x818cf8))
                            .child(timecode_text),
                    )
                    // Divider
                    .child(div().h(px(14.0)).border_r_1().border_color(rgb(BORDER)))
                    // Sleek Zoom Bar
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            // Zoom Out
                            .child(
                                div()
                                    .id("zoom-out")
                                    .w(px(20.0))
                                    .h(px(20.0))
                                    .rounded(px(3.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .text_color(rgb(MUTED))
                                    .hover(|s| s.text_color(rgb(TEXT)))
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view("Zoom Out", Some("−"), cx)
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        if editor.timeline.is_none() {
                                            return;
                                        }
                                        let anchor = editor.timeline_playhead_seconds(cx);
                                        let timeline = editor.timeline.as_mut().unwrap();
                                        timeline.zoom(0.8, anchor);
                                        if let Err(error) = timeline.save() {
                                            log::error!("{error:?}");
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Minus.path())
                                            .size(px(11.0))
                                            .text_color(rgb(MUTED))
                                            .hover(|s| s.text_color(rgb(TEXT))),
                                    ),
                            )
                            // Sleek Zoom Progress Rail
                            .child({
                                let bounds_cell = zoom_track_bounds.clone();
                                div()
                                    .id("timeline-zoom-slider")
                                    .w(px(64.0))
                                    .h(px(16.0))
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view(
                                            "Timeline Zoom",
                                            Some("Cmd+Scroll"),
                                            cx,
                                        )
                                    })
                                    .on_click(cx.listener(
                                        move |editor, event: &ClickEvent, _, cx| {
                                            let bounds = bounds_cell.get();
                                            let width = f32::from(bounds.size.width);
                                            if width <= 0.0 || editor.timeline.is_none() {
                                                return;
                                            }
                                            let offset_x = (f32::from(event.position().x)
                                                - f32::from(bounds.origin.x))
                                            .clamp(0.0, width);
                                            let ratio = offset_x / width;
                                            let target_log = min_log + ratio * (max_log - min_log);
                                            let target_pps = target_log.exp().clamp(0.1, 1000.0);
                                            let anchor = editor.timeline_playhead_seconds(cx);
                                            let timeline = editor.timeline.as_mut().unwrap();
                                            let factor = target_pps / timeline.pixels_per_second;
                                            timeline.zoom(factor, anchor);
                                            if let Err(error) = timeline.save() {
                                                log::error!("{error:?}");
                                            }
                                            cx.notify();
                                        },
                                    ))
                                    .child(
                                        div()
                                            .relative()
                                            .w_full()
                                            .h(px(2.0))
                                            .rounded_full()
                                            .bg(rgb(0x27272a))
                                            .child(
                                                div()
                                                    .h_full()
                                                    .w(relative(zoom_fraction))
                                                    .rounded_full()
                                                    .bg(rgb(0x818cf8)),
                                            )
                                            .child(
                                                div()
                                                    .absolute()
                                                    .top(px(-3.5))
                                                    .left(relative(zoom_fraction))
                                                    .w_0()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(
                                                        div()
                                                            .size(px(9.0))
                                                            .rounded_full()
                                                            .bg(rgb(0x94a3b8))
                                                            .border_1()
                                                            .border_color(rgb(0x18181b)),
                                                    ),
                                            )
                                            .child(
                                                canvas(
                                                    move |bounds, _, _| {
                                                        zoom_track_bounds.set(bounds)
                                                    },
                                                    |_, _, _, _| {},
                                                )
                                                .absolute()
                                                .inset_0(),
                                            ),
                                    )
                            })
                            // Zoom In
                            .child(
                                div()
                                    .id("zoom-in")
                                    .w(px(20.0))
                                    .h(px(20.0))
                                    .rounded(px(3.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .text_color(rgb(MUTED))
                                    .hover(|s| s.text_color(rgb(TEXT)))
                                    .tooltip(|_, cx| {
                                        crate::tooltip::tooltip_view("Zoom In", Some("+"), cx)
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        if editor.timeline.is_none() {
                                            return;
                                        }
                                        let anchor = editor.timeline_playhead_seconds(cx);
                                        let timeline = editor.timeline.as_mut().unwrap();
                                        timeline.zoom(1.25, anchor);
                                        if let Err(error) = timeline.save() {
                                            log::error!("{error:?}");
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Plus.path())
                                            .size(px(11.0))
                                            .text_color(rgb(MUTED))
                                            .hover(|s| s.text_color(rgb(TEXT))),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn frame_tick_step(pixels_per_frame: f32) -> i64 {
    (MIN_FRAME_TICK_SPACING / pixels_per_frame.max(f32::EPSILON))
        .ceil()
        .max(1.0) as i64
}

fn ruler_tick_step(duration: f64, pixels_per_second: f32) -> f64 {
    let spacing_step = MIN_RULER_LABEL_SPACING as f64 / pixels_per_second.max(f32::EPSILON) as f64;
    let count_step = duration.max(0.0) / MAX_RULER_TICKS as f64;
    let minimum_step = spacing_step.max(count_step);

    TICK_STEPS
        .iter()
        .copied()
        .find(|step| *step >= minimum_step)
        .unwrap_or_else(|| {
            let largest_step = *TICK_STEPS
                .last()
                .expect("timeline tick steps are not empty");
            (minimum_step / largest_step).ceil() * largest_step
        })
}

fn format_smpte_timecode(frame_index: TimelineFrameIndex, fps: f64) -> String {
    let total_frames: i64 = i64::from(frame_index).max(0);
    let fps_int = fps.round().max(1.0) as i64;
    let frames = total_frames % fps_int;
    let total_seconds = total_frames / fps_int;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}:{frames:02}")
}

fn format_time_precise(seconds: f64) -> String {
    let minutes = (seconds / 60.0).floor() as u64;
    let seconds = seconds % 60.0;
    format!("{minutes:02}:{seconds:04.1}")
}

/// 缩略图高度随源宽高比变化：竖屏 9:16 取上限 128，横屏 16:9 约 40。
fn seek_thumb_height(aspect: f32) -> f32 {
    (SEEK_THUMB_WIDTH / aspect).clamp(SEEK_THUMB_MIN_HEIGHT, SEEK_THUMB_MAX_HEIGHT)
}

fn seek_preview_caret(caret_x: f32, card_height: f32) -> gpui::AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let left = bounds.origin;
            let tip = bounds.origin + point(px(5.0), px(8.0));
            let right = bounds.origin + point(px(10.0), px(0.0));
            let mut fill = PathBuilder::fill();
            fill.move_to(left);
            fill.line_to(tip);
            fill.line_to(right);
            fill.close();
            window.paint_path(
                fill.build().expect("valid seek preview caret"),
                rgb(SURFACE),
            );
            let mut outline = PathBuilder::stroke(px(1.0));
            outline.move_to(left);
            outline.line_to(tip);
            outline.line_to(right);
            window.paint_path(
                outline.build().expect("valid seek preview caret outline"),
                rgb(BORDER),
            );
        },
    )
    .absolute()
    .top(px(card_height))
    .left(px(caret_x - 5.0))
    .w(px(10.0))
    .h(px(8.0))
    .into_any_element()
}

fn finish_clip_move(
    editor: &mut Editor,
    _: &MouseUpEvent,
    _: &mut Window,
    cx: &mut Context<Editor>,
) {
    let Some(timeline) = editor.timeline.as_mut() else {
        return;
    };
    let Some(drag) = timeline.interaction.clip_move_drag.take() else {
        return;
    };
    timeline.interaction.snap_guide = None;
    if !drag.changed {
        // 没有拖动的点击：只选中被点击的 clip。
        editor.select_only_clip(Some(drag.anchor_clip_id));
    } else if drag.invalid_reason.is_none() {
        editor.emit_event(
            cx,
            AppEvent::Edit(EditAction::MoveClips {
                placements: drag.placements,
            }),
        );
    }
    cx.notify();
}

fn pinch_timeline(
    editor: &mut Editor,
    event: &gpui::PinchEvent,
    _: &mut Window,
    cx: &mut Context<Editor>,
) {
    if editor.timeline.is_none() {
        return;
    }
    let anchor = editor.timeline_playhead_seconds(cx);
    let timeline = editor.timeline.as_mut().unwrap();
    let previous_zoom = timeline.pixels_per_second;
    timeline.zoom(
        (event.delta * TIMELINE_PINCH_SENSITIVITY)
            .exp()
            .clamp(0.5, 2.0),
        anchor,
    );
    if matches!(
        event.phase,
        gpui::TouchPhase::Ended | gpui::TouchPhase::Cancelled
    ) && let Err(error) = timeline.save()
    {
        log::error!("Could not save timeline zoom: {error:?}");
    }
    if timeline.pixels_per_second != previous_zoom {
        cx.notify();
    }
    cx.stop_propagation();
}

impl Editor {
    pub(super) fn empty_timeline_view(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let existing_timelines =
            timeline_document::project_timeline_files(&self.project_root).unwrap_or_default();

        div()
            .id("editor-timeline-empty")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p_6()
            .bg(rgb(0x131417))
            .on_drop(cx.listener(|editor, drag: &FileTreeEntry, _, cx| {
                editor.handle_empty_timeline_asset_drop(drag, cx);
            }))
            .on_drop(cx.listener(|editor, paths: &ExternalPaths, _, cx| {
                editor.handle_empty_timeline_external_drop(paths.paths(), cx);
            }))
            .child(
                div()
                    .w_full()
                    .max_w(px(520.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .p_8()
                    .rounded_xl()
                    .border_2()
                    .border_dashed()
                    .border_color(rgb(0x2e313a))
                    .bg(rgb(0x18191e))
                    .hover(|s| s.border_color(rgb(0x6366f1)).bg(rgb(0x1c1e26)))
                    .child(
                        div()
                            .size(px(46.0))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgb(0x22242e))
                            .border_1()
                            .border_color(rgb(0x363945))
                            .child(
                                svg()
                                    .path(IconName::Playhead.path())
                                    .size(px(20.0))
                                    .text_color(rgb(0x6366f1)),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(15.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xe4e4e7))
                            .child("No Active Timeline"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0x8a8f9d))
                            .child(
                                "Drag media from your computer or the Media Pool to create a timeline instantly",
                            ),
                    )
                    .child(
                        div()
                            .id("btn-create-new-timeline")
                            .cursor(CursorStyle::PointingHand)
                            .flex()
                            .items_center()
                            .gap_2()
                            .mt_2()
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(0x6366f1))
                            .hover(|s| s.bg(rgb(0x4f46e5)))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0xffffff))
                            .child(
                                svg()
                                    .path(IconName::Plus.path())
                                    .size(px(13.0))
                                    .text_color(rgb(0xffffff)),
                            )
                            .child("Create New Timeline")
                            .on_click(cx.listener(|editor, _, _, cx| {
                                if let Err(error) =
                                    editor.create_and_activate_new_timeline(None, cx)
                                {
                                    log::error!("Failed to create timeline: {error:?}");
                                }
                            })),
                    ),
            )
            .when(!existing_timelines.is_empty(), |this| {
                this.child(
                    div()
                        .mt_5()
                        .w_full()
                        .max_w(px(520.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(0x6b7280))
                                .child("OR OPEN AN EXISTING TIMELINE"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .justify_center()
                                .gap_2()
                                .children(existing_timelines.into_iter().take(8).map(|path| {
                                    let display_name = path
                                        .file_name()
                                        .and_then(|name| name.to_str())
                                        .and_then(|name| {
                                            name.strip_suffix(".timeline")
                                                .or_else(|| name.strip_suffix(".timeline.json"))
                                        })
                                        .unwrap_or("Timeline")
                                        .to_string();
                                    let path_clone = path.clone();
                                    div()
                                        .id(ElementId::Name(
                                            format!("chip-open-{}", path.display()).into(),
                                        ))
                                        .cursor(CursorStyle::PointingHand)
                                        .flex()
                                        .items_center()
                                        .gap_1p5()
                                        .px_2p5()
                                        .py_1()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(rgb(0x2e313a))
                                        .bg(rgb(0x18191e))
                                        .hover(|s| {
                                            s.bg(rgb(0x252833)).border_color(rgb(0x6366f1))
                                        })
                                        .text_size(px(11.0))
                                        .text_color(rgb(0xd1d5db))
                                        .child(
                                            svg()
                                                .path(IconName::Playhead.path())
                                                .size(px(11.0))
                                                .text_color(rgb(0x9ca3af)),
                                        )
                                        .child(display_name)
                                        .on_click(cx.listener(move |editor, _, _, cx| {
                                            editor.emit_event(
                                                cx,
                                                AppEvent::OpenTimeline {
                                                    path: path_clone.clone(),
                                                },
                                            );
                                        }))
                                })),
                        ),
                )
            })
            .into_any_element()
    }

    pub(crate) fn handle_empty_timeline_asset_drop(
        &mut self,
        drag: &FileTreeEntry,
        cx: &mut Context<Self>,
    ) {
        match drag.kind {
            FileTreeEntryKind::Timeline => {
                let relative = drag
                    .absolute_path
                    .strip_prefix(&self.project_root)
                    .unwrap_or(&drag.absolute_path)
                    .to_path_buf();
                self.emit_event(cx, AppEvent::OpenTimeline { path: relative });
            }
            FileTreeEntryKind::Video | FileTreeEntryKind::Audio | FileTreeEntryKind::Image => {
                if let Err(error) =
                    self.auto_create_timeline_and_insert_asset(&drag.absolute_path, cx)
                {
                    log::error!("Failed to create timeline from asset: {error:?}");
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_empty_timeline_external_drop(
        &mut self,
        paths: &[PathBuf],
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() {
            return;
        }
        self.import_external_paths(paths, cx);

        // Check if a .timeline file was dropped
        if let Some(timeline_path) = paths
            .iter()
            .find(|p| timeline_document::is_timeline_path(p))
            && let Ok(relative) = timeline_path.strip_prefix(&self.project_root)
        {
            self.emit_event(
                cx,
                AppEvent::OpenTimeline {
                    path: relative.to_path_buf(),
                },
            );
            return;
        }

        // Find first media file to create timeline
        let first_media = paths.iter().find(|path| {
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            matches!(
                ext.as_str(),
                "mp4"
                    | "mov"
                    | "m4v"
                    | "mkv"
                    | "avi"
                    | "webm"
                    | "mp3"
                    | "wav"
                    | "aac"
                    | "flac"
                    | "ogg"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "webp"
            )
        });

        if let Some(media_path) = first_media
            && let Err(error) = self.auto_create_timeline_and_insert_asset(media_path, cx)
        {
            log::error!("Failed to auto-create timeline from external media: {error:?}");
        }
    }

    pub(crate) fn handle_timeline_tracks_external_drop(
        &mut self,
        paths: &[PathBuf],
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() {
            return;
        }
        self.import_external_paths(paths, cx);

        let Some(first_path) = paths.first() else {
            return;
        };
        let canonical = first_path
            .canonicalize()
            .unwrap_or_else(|_| first_path.to_path_buf());

        let asset = if let Some(cached) = self.explorer.media_metadata.get(&canonical) {
            cached.clone()
        } else {
            match engine::media_asset::probe_media_asset(&canonical) {
                Ok(mut probed) => {
                    probed.set_id(Ulid::generate());
                    self.explorer
                        .media_metadata
                        .insert(canonical.clone(), probed.clone());
                    probed
                }
                Err(error) => {
                    log::error!("Failed to probe dropped media: {error:?}");
                    return;
                }
            }
        };

        let Some(timeline) = self.timeline.as_ref() else {
            return;
        };

        let is_audio = asset.kind() == timeline::MediaKind::Audio;
        let expected_kind = if is_audio {
            TrackKind::Audio
        } else {
            TrackKind::Video
        };

        let playhead = timeline.playhead();
        let duration = timeline
            .editing_state
            .nearest_time(asset.duration().unwrap_or(DEFAULT_IMAGE_DURATION_SECONDS))
            .max(TimelineFrameIndex::ONE_FRAME);

        let mut target_track_id = timeline
            .editing_state
            .tracks
            .iter()
            .filter(|t| t.kind == expected_kind && !t.locked)
            .find(|t| {
                crate::clip_placement::validate_clip_placement(
                    &timeline.editing_state,
                    t.id,
                    asset.kind(),
                    duration,
                    playhead,
                    &std::collections::HashSet::new(),
                )
                .is_ok()
            })
            .map(|t| t.id);

        let start_pos = if target_track_id.is_some() {
            playhead
        } else {
            let end_time = timeline.editing_state.content_duration();
            target_track_id = timeline
                .editing_state
                .tracks
                .iter()
                .find(|t| t.kind == expected_kind && !t.locked)
                .map(|t| t.id);
            end_time
        };

        let placement = match target_track_id {
            Some(track_id) => Some((Some(track_id), start_pos)),
            None => {
                if timeline
                    .editing_state
                    .tracks
                    .iter()
                    .any(|t| t.kind == expected_kind)
                {
                    log::error!(
                        "Failed to place dropped external media: no unlocked {expected_kind:?} track is available"
                    );
                    None
                } else {
                    Some((None, playhead)) // 没有同类轨道：新建轨道并连同片段一起提交。
                }
            }
        };
        if let Some((track_id, start)) = placement
            && let Err(error) =
                self.place_explorer_asset(canonical, track_id, start, asset, None, cx)
        {
            log::error!("Failed to place dropped external media: {error:?}");
        }
    }
}
