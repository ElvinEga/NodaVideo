use crate::actions::EDITOR_KEY_CONTEXT;
use crate::asset::IconName;
use crate::dialog_missing_media::missing_media_dialog_view;
use crate::editor::{Editor, WorkspaceTab};
use crate::event_bus::{AppEvent, EventBus};
use crate::keyboard_shortcuts::keyboard_shortcuts_view;
use crate::layout::{
    MIN_MEDIA_PANEL_WIDTH, MIN_PREVIEW_WIDTH, MIN_PROPERTY_PANEL_WIDTH, MIN_TIMELINE_HEIGHT,
    MIN_UPPER_WORKSPACE_HEIGHT, TOPBAR_HEIGHT,
};
use crate::preview::PreviewTarget;
use crate::preview_events::PreviewEvent;
use crate::project_picker;
use crate::property_panel::property_panel_view;
use crate::settings::settings_modal_view;
use crate::theme::{
    ACCENT, ACCENT_HOVER, BACKGROUND, BORDER, MUTED, PANEL, SUCCESS, SURFACE, SURFACE_HOVER, TEXT,
};
use crate::ui_components::cyberpunk_button::{
    button_cyberpunk_primary, button_cyberpunk_secondary,
};
use crate::ui_components::{
    HorizontalSplit, HorizontalSplitConstraints, VerticalSplit, VerticalSplitConstraints,
};
use gpui::prelude::*;
use gpui::{
    App, CursorStyle, FontWeight, MouseButton, MouseDownEvent, Window, canvas, div, img, px, rgb,
    svg,
};

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_video_transform_inputs(cx);

        let viewport = window.viewport_size();
        let editor_width =
            (f32::from(viewport.width) - crate::gpui_inspector::docked_width(window)).max(0.0);
        let editor_viewport = gpui::size(px(editor_width), viewport.height);
        let workspace_height = (f32::from(viewport.height) - TOPBAR_HEIGHT).max(0.0); // 顶栏以下、上半区与时间线共用的高度。
        let workspace_heights = self
            .workspace_split_state
            .read(cx)
            .heights(workspace_height, WORKSPACE_SPLIT_CONSTRAINTS);

        if self.preview.fullscreen {
            return div()
                .id("editor-fullscreen-preview")
                .key_context(EDITOR_KEY_CONTEXT)
                .on_mouse_move(cx.listener(Self::move_preview_text))
                .on_mouse_move(cx.listener(Self::move_clip_trim))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::release_clip_trim))
                .on_mouse_up_out(MouseButton::Left, cx.listener(Self::release_clip_trim))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::release_preview_text))
                .on_mouse_up_out(MouseButton::Left, cx.listener(Self::release_preview_text))
                .track_focus(&self.focus_handle)
                .on_action(cx.listener(Self::action_toggle_playback))
                .on_action(cx.listener(Self::action_step_backward_frame))
                .on_action(cx.listener(Self::action_step_forward_frame))
                .on_action(cx.listener(Self::action_toggle_fullscreen))
                .on_action(cx.listener(Self::action_exit_fullscreen))
                .on_action(cx.listener(Self::action_toggle_inspector))
                .on_action(cx.listener(Self::action_new_project))
                .on_action(cx.listener(Self::action_save_project))
                .on_action(cx.listener(Self::action_close_window))
                .on_action(cx.listener(Self::action_minimize_window))
                .on_action(cx.listener(Self::action_zoom_window))
                .on_action(cx.listener(Self::action_bring_all_to_front))
                .on_action(cx.listener(Self::action_show_keyboard_shortcuts))
                .on_action(cx.listener(Self::action_show_about))
                .on_action(cx.listener(Self::action_mark_source_in))
                .on_action(cx.listener(Self::action_mark_source_out))
                .on_action(cx.listener(Self::action_insert_source_to_timeline))
                .size_full()
                .overflow_hidden()
                .bg(rgb(0x000000))
                .text_color(rgb(TEXT))
                .child(self.preview_player(editor_width, f32::from(viewport.height), cx));
        }
        let placement_cursor = self
            .timeline
            .as_ref()
            .and_then(|timeline| timeline.interaction.clip_move_drag.as_ref())
            .filter(|drag| drag.changed)
            .map(|drag| {
                if drag.invalid_reason.is_some() {
                    CursorStyle::OperationNotAllowed
                } else {
                    CursorStyle::ClosedHand
                }
            });
        let context_menu = self.context_menu_overlay(editor_viewport, cx);
        let rename_dialog = self
            .explorer
            .rename_dialog
            .as_ref()
            .map(|_| self.rename_dialog(cx));
        let new_timeline_dialog = self
            .explorer
            .new_timeline_dialog
            .as_ref()
            .map(|_| self.new_timeline_dialog(cx));
        let template_gallery = self
            .explorer
            .template_gallery
            .as_ref()
            .map(|_| self.template_gallery_dialog(cx));
        let missing_media_dialog = self
            .missing_media_dialog
            .as_ref()
            .map(|dialog| missing_media_dialog_view(dialog, self.event_bus.clone()));
        let settings_modal = if self.settings_open
            && let Some(timeline) = self.timeline.as_ref()
        {
            Some(settings_modal_view(&timeline.editing_state, cx))
        } else {
            None
        };

        div()
            .id("editor-root")
            .key_context(EDITOR_KEY_CONTEXT)
            .on_mouse_move(cx.listener(Self::move_preview_text))
            .on_mouse_move(cx.listener(Self::move_clip_trim))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::release_clip_trim))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::release_clip_trim))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::release_preview_text))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::release_preview_text))
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::action_toggle_playback))
            .on_action(cx.listener(Self::action_step_backward_frame))
            .on_action(cx.listener(Self::action_step_forward_frame))
            .on_action(cx.listener(Self::action_delete_selected))
            .on_action(cx.listener(Self::action_split_clip))
            .on_action(cx.listener(Self::action_undo))
            .on_action(cx.listener(Self::action_redo))
            .on_action(cx.listener(Self::action_duplicate_selected))
            .on_action(cx.listener(Self::action_copy_selected_clips))
            .on_action(cx.listener(Self::action_cut_selected_clips))
            .on_action(cx.listener(Self::action_paste_clips))
            .on_action(cx.listener(Self::action_select_all_unlocked_clips))
            .on_action(cx.listener(Self::action_select_clips_right_of_pointer))
            .on_action(cx.listener(Self::action_select_clips_left_of_pointer))
            .on_action(cx.listener(Self::action_activate_selection_tool))
            .on_action(cx.listener(Self::action_activate_blade_tool))
            .on_action(cx.listener(Self::action_toggle_fullscreen))
            .on_action(cx.listener(Self::action_exit_fullscreen))
            .on_action(cx.listener(Self::action_toggle_inspector))
            .on_action(cx.listener(Self::action_reveal_in_finder))
            .on_action(cx.listener(Self::action_open_in_default_app))
            .on_action(cx.listener(Self::action_switch_to_edit_workspace))
            .on_action(cx.listener(Self::action_switch_to_color_workspace))
            .on_action(cx.listener(Self::action_switch_to_audio_workspace))
            .on_action(cx.listener(Self::action_switch_to_aistudio_workspace))
            .on_action(cx.listener(Self::action_toggle_media_browser))
            .on_action(cx.listener(Self::action_go_to_start))
            .on_action(cx.listener(Self::action_go_to_end))
            .on_action(cx.listener(Self::action_export_video))
            .on_action(cx.listener(Self::action_open_project_folder))
            .on_action(cx.listener(Self::action_open_project_picker))
            .on_action(cx.listener(Self::action_new_timeline))
            .on_action(cx.listener(Self::action_open_settings))
            .on_action(cx.listener(Self::action_import_media))
            .on_action(cx.listener(Self::action_new_project))
            .on_action(cx.listener(Self::action_save_project))
            .on_action(cx.listener(Self::action_close_window))
            .on_action(cx.listener(Self::action_minimize_window))
            .on_action(cx.listener(Self::action_zoom_window))
            .on_action(cx.listener(Self::action_bring_all_to_front))
            .on_action(cx.listener(Self::action_show_keyboard_shortcuts))
            .on_action(cx.listener(Self::action_show_about))
            .on_action(cx.listener(Self::action_mark_source_in))
            .on_action(cx.listener(Self::action_mark_source_out))
            .on_action(cx.listener(Self::action_insert_source_to_timeline))
            .capture_any_mouse_down(cx.listener(|editor, event: &MouseDownEvent, window, cx| {
                if event.button == MouseButton::Left {
                    editor.focus_handle.focus(window, cx);
                }
            }))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .child(self.topbar(window, cx))
            .child(VerticalSplit::new(
                "editor-workspace",
                self.workspace_split_state.clone(),
                self.event_bus.clone(),
                workspace_height,
                WORKSPACE_SPLIT_CONSTRAINTS,
                self.upper_workspace(editor_width, workspace_heights.top, window, cx),
                self.timeline_view(editor_width, cx),
            ))
            .when_some(context_menu, |this, menu| this.child(menu))
            .when_some(rename_dialog, |this, dialog| this.child(dialog))
            .when_some(new_timeline_dialog, |this, dialog| this.child(dialog))
            .when_some(template_gallery, |this, dialog| this.child(dialog))
            .when(self.explorer.trash_timeline_dialog_open, |this| {
                this.child(self.trash_timeline_dialog_view(cx))
            })
            .when_some(missing_media_dialog, |this, dialog| this.child(dialog))
            .when_some(settings_modal, |this, modal| this.child(modal))
            .when(self.shortcuts_open, |this| {
                this.child(keyboard_shortcuts_view(cx))
            })
            .when(self.global_settings_input.is_some(), |this| {
                this.child(self.global_settings_dialog(cx))
            })
            .when_some(placement_cursor, |this, cursor| {
                this.child(
                    canvas(
                        |_, _, _| {},
                        move |_, (), window, _| {
                            window.set_window_cursor_style(cursor);
                        },
                    )
                    .absolute()
                    .inset_0()
                    .size_full(),
                )
            })
    }
}

// Public APIs only
impl Editor {
    /// The height of the timeline panel at the bottom of the window, as laid out by `render`.
    pub(super) fn timeline_height(&self, window: &Window, cx: &App) -> f32 {
        let workspace_height = (f32::from(window.viewport_size().height) - TOPBAR_HEIGHT).max(0.0);
        self.workspace_split_state
            .read(cx)
            .heights(workspace_height, WORKSPACE_SPLIT_CONSTRAINTS)
            .bottom
    }
}

impl Editor {
    fn upper_workspace(
        &self,
        width: f32,
        preview_height: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match self.workspace {
            WorkspaceTab::Edit => self.edit_workspace(width, preview_height, window, cx),
            WorkspaceTab::Color => self.color_workspace(width, preview_height, window, cx),
            WorkspaceTab::Audio => self.audio_workspace(width, preview_height, cx),
            WorkspaceTab::AiStudio => self.assistant_window_view(window, cx),
        }
    }

    fn edit_workspace(
        &self,
        width: f32,
        preview_height: f32,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if self.show_media_browser && self.show_inspector {
            let constraints = HorizontalSplitConstraints {
                min_left: MIN_MEDIA_PANEL_WIDTH,
                min_center: MIN_PREVIEW_WIDTH,
                min_right: MIN_PROPERTY_PANEL_WIDTH,
            };
            let widths = self.upper_split_state.read(cx).widths(width, constraints);
            HorizontalSplit::new(
                "editor-upper-workspace",
                self.upper_split_state.clone(),
                self.event_bus.clone(),
                width,
                constraints,
                self.explorer_panel(cx),
                self.monitor_panel(widths.center, preview_height, cx),
                property_panel_view(&self.property_panel),
            )
            .into_any_element()
        } else if self.show_media_browser {
            let left_width = 320.0f32;
            let center_width = (width - left_width).max(MIN_PREVIEW_WIDTH);
            div()
                .size_full()
                .flex()
                .child(
                    div()
                        .w(px(left_width))
                        .h_full()
                        .child(self.explorer_panel(cx)),
                )
                .child(self.monitor_panel(center_width, preview_height, cx))
                .into_any_element()
        } else if self.show_inspector {
            let right_width = 300.0f32;
            let center_width = (width - right_width).max(MIN_PREVIEW_WIDTH);
            div()
                .size_full()
                .flex()
                .child(self.monitor_panel(center_width, preview_height, cx))
                .child(
                    div()
                        .w(px(right_width))
                        .h_full()
                        .child(property_panel_view(&self.property_panel)),
                )
                .into_any_element()
        } else {
            div()
                .size_full()
                .flex()
                .child(self.monitor_panel(width, preview_height, cx))
                .into_any_element()
        }
    }

    fn monitor_panel(&self, width: f32, height: f32, cx: &mut Context<Self>) -> gpui::AnyElement {
        let (title, detail) = match &self.preview.target {
            PreviewTarget::Timeline { .. } => {
                let name = self
                    .timeline
                    .as_ref()
                    .and_then(|t| t.path.file_stem())
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Sequence 01".to_string());
                ("Timeline", name)
            }
            PreviewTarget::VideoFile { path, .. }
            | PreviewTarget::AudioFile { path, .. }
            | PreviewTarget::ImageFile(path) => {
                let name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Source".to_string());
                ("Source", name)
            }
            PreviewTarget::None => ("Preview", "No Media".to_string()),
            PreviewTarget::Error { .. } => ("Preview", "Error".to_string()),
            PreviewTarget::TextTitle(title_id) => {
                let name = self
                    .explorer
                    .titles
                    .iter()
                    .find(|title| title.id == *title_id)
                    .map(|title| title.name.clone())
                    .unwrap_or_else(|| "Title".to_string());
                ("Source", name)
            }
        };

        let (current_tc, total_tc) = self.preview_timecode_and_duration(cx);
        let playing = self.is_preview_playing(cx);
        let is_empty = matches!(self.preview.target, PreviewTarget::None);
        let title_id = match self.preview.target {
            PreviewTarget::TextTitle(title_id) => Some(title_id),
            PreviewTarget::None
            | PreviewTarget::Error { .. }
            | PreviewTarget::Timeline { .. }
            | PreviewTarget::VideoFile { .. }
            | PreviewTarget::AudioFile { .. }
            | PreviewTarget::ImageFile(_) => None,
        };
        let title_font_label = match title_id {
            Some(title_id) => {
                match self
                    .explorer
                    .titles
                    .iter()
                    .find(|title| title.id == title_id)
                {
                    Some(title) => format!("Text · {}", title.properties.font),
                    None => "Text".to_string(),
                }
            }
            None => String::new(),
        };

        let header_height = 36.0f32;
        let control_height = 40.0f32;
        let footer_height = if title_id.is_some() {
            40.0f32
        } else {
            0.0f32
        };
        let screen_height = (height - header_height - control_height - footer_height).max(60.0);

        div()
            .w(px(width))
            .h(px(height))
            .flex()
            .flex_col()
            .bg(rgb(PANEL))
            // Monitor Header
            .child(
                div()
                    .h(px(header_height))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT))
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(MUTED))
                                    .child(detail),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(rgb(MUTED))
                                    .child("Fit"),
                            )
                            .child(
                                div()
                                    .id("monitor-fullscreen-toggle")
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        editor.preview.fullscreen = !editor.preview.fullscreen;
                                        cx.notify();
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Expand.path())
                                            .size(px(13.0))
                                            .text_color(rgb(MUTED)),
                                    ),
                            ),
                    ),
            )
            // Screen Viewport
            .child(
                div()
                    .w(px(width))
                    .h(px(screen_height))
                    .bg(rgb(0x090a0d))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.preview_player(width, screen_height, cx)),
            )
            // Monitor Control Bar
            .child(
                div()
                    .h(px(control_height))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL))
                    // Left: Monospace Timecode
                    .child(
                        div().flex_1().flex().items_center().justify_start().child(
                            div()
                                .font_family("monospace")
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0x818cf8))
                                .child(current_tc),
                        ),
                    )
                    // Center: Transport controls (Previous, Play/Pause, Next)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            // Back / Previous video or start
                            .child(
                                div()
                                    .id("monitor-prev-frame")
                                    .w(px(28.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        editor.navigate_previous_video_or_start(cx);
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Previous.path())
                                            .size(px(13.0))
                                            .text_color(rgb(if is_empty {
                                                MUTED
                                            } else {
                                                TEXT
                                            })),
                                    ),
                            )
                            // Play / Pause Toggle
                            .child(
                                div()
                                    .id("monitor-play-toggle")
                                    .h(px(26.0))
                                    .px_2()
                                    .rounded(px(4.0))
                                    .bg(rgb(SURFACE))
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)).border_color(rgb(ACCENT)))
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        editor.emit_event(
                                            cx,
                                            AppEvent::Preview(PreviewEvent::TogglePlayback),
                                        );
                                    }))
                                    .child(
                                        svg()
                                            .path(if playing {
                                                IconName::Pause.path()
                                            } else {
                                                IconName::Play.path()
                                            })
                                            .size(px(13.0))
                                            .text_color(rgb(TEXT)),
                                    ),
                            )
                            // Next video
                            .child(
                                div()
                                    .id("monitor-next-frame")
                                    .w(px(28.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        editor.navigate_next_video(cx);
                                    }))
                                    .child(
                                        svg()
                                            .path(IconName::Next.path())
                                            .size(px(13.0))
                                            .text_color(rgb(if is_empty {
                                                MUTED
                                            } else {
                                                TEXT
                                            })),
                                    ),
                            ),
                    )
                    // Right: Duration / FPS
                    .child(
                        div().flex_1().flex().items_center().justify_end().child(
                            div()
                                .font_family("monospace")
                                .text_size(px(10.5))
                                .text_color(rgb(MUTED))
                                .child(total_tc),
                        ),
                    ),
            )
            // Title footer: font label + Add to Timeline (only for a pool title preview)
            .when_some(title_id, move |this, title_id| {
                this.child(
                    div()
                        .h(px(footer_height))
                        .px_3()
                        .flex()
                        .items_center()
                        .justify_between()
                        .border_t_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(PANEL))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(rgb(MUTED))
                                .child(title_font_label),
                        )
                        .child(
                            div()
                                .id("monitor-add-title-to-timeline")
                                .h(px(26.0))
                                .px_3()
                                .rounded(px(4.0))
                                .bg(rgb(ACCENT))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(ACCENT_HOVER)))
                                .on_click(cx.listener(move |editor, _, _, cx| {
                                    editor
                                        .emit_event(cx, AppEvent::AddTitleToTimeline { title_id });
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0xffffff))
                                        .child("Add to Timeline"),
                                ),
                        ),
                )
            })
            .into_any_element()
    }

    fn color_workspace(
        &self,
        width: f32,
        preview_height: f32,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let left_width = 280.0f32;
        let right_width = 320.0f32;
        let center_width = (width - left_width - right_width).max(MIN_PREVIEW_WIDTH);

        div()
            .size_full()
            .flex()
            .bg(rgb(BACKGROUND))
            // Left: Scopes & Color Analyzers
            .child(
                div()
                    .w(px(left_width))
                    .h_full()
                    .border_r_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(36.0))
                            .px_3()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(TEXT))
                                    .child("SCOPES & HISTOGRAM"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .px_1p5()
                                    .rounded(px(3.0))
                                    .bg(rgb(SURFACE))
                                    .text_color(rgb(MUTED))
                                    .child("RGB Parade"),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_3()
                            // Scope Box 1: Waveform / Parade
                            .child(
                                div()
                                    .flex_1()
                                    .rounded(px(6.0))
                                    .bg(rgb(0x0c0d10))
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .p_2()
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(9.0))
                                            .text_color(rgb(MUTED))
                                            .child("100 IRE ──"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_around()
                                            .h(px(60.0))
                                            .child(
                                                div()
                                                    .w(px(24.0))
                                                    .h_full()
                                                    .bg(rgb(0xef4444))
                                                    .opacity(0.35)
                                                    .rounded_sm(),
                                            )
                                            .child(
                                                div()
                                                    .w(px(24.0))
                                                    .h_full()
                                                    .bg(rgb(SUCCESS))
                                                    .opacity(0.35)
                                                    .rounded_sm(),
                                            )
                                            .child(
                                                div()
                                                    .w(px(24.0))
                                                    .h_full()
                                                    .bg(rgb(0x3b82f6))
                                                    .opacity(0.35)
                                                    .rounded_sm(),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(9.0))
                                            .text_color(rgb(MUTED))
                                            .child("  0 IRE ──"),
                                    ),
                            )
                            // Scope Box 2: Vectorscope
                            .child(
                                div()
                                    .h(px(140.0))
                                    .rounded(px(6.0))
                                    .bg(rgb(0x0c0d10))
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .size(px(90.0))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(rgb(0x2e313a))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                div()
                                                    .size(px(40.0))
                                                    .rounded_full()
                                                    .border_1()
                                                    .border_color(rgb(0x3f4350)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .text_color(rgb(MUTED))
                                            .child("Vectorscope (Rec.709)"),
                                    ),
                            ),
                    ),
            )
            // Center: Large Preview Monitor
            .child(self.monitor_panel(center_width, preview_height, cx))
            // Right: Color Grading Wheels & Controls
            .child(
                div()
                    .w(px(right_width))
                    .h_full()
                    .border_l_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(36.0))
                            .px_3()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(TEXT))
                                    .child("COLOR WHEELS & LOOKS"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .px_1p5()
                                    .rounded(px(3.0))
                                    .bg(rgb(0x23243a))
                                    .text_color(rgb(ACCENT))
                                    .child("Primary"),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_3()
                            // Color Wheels (Lift / Gamma / Gain)
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .gap_2()
                                    .child(color_wheel_widget("Lift (Shadows)"))
                                    .child(color_wheel_widget("Gamma (Mid)"))
                                    .child(color_wheel_widget("Gain (Highlight)")),
                            )
                            // Grading Sliders
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .p_2()
                                    .rounded(px(6.0))
                                    .bg(rgb(SURFACE))
                                    .child(color_slider_row("Exposure", "0.0 EV"))
                                    .child(color_slider_row("Contrast", "+15"))
                                    .child(color_slider_row("Highlights", "-5"))
                                    .child(color_slider_row("Shadows", "+10"))
                                    .child(color_slider_row("Temperature", "5600 K"))
                                    .child(color_slider_row("Tint", "0"))
                                    .child(color_slider_row("Saturation", "100%")),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn audio_workspace(
        &self,
        width: f32,
        preview_height: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mixer_width = (width * 0.58).clamp(420.0, 720.0);
        let preview_width = (width - mixer_width).max(MIN_PREVIEW_WIDTH);

        div()
            .size_full()
            .flex()
            .bg(rgb(BACKGROUND))
            // Left: Multitrack Audio Mixer
            .child(
                div()
                    .w(px(mixer_width))
                    .h_full()
                    .border_r_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(36.0))
                            .px_3()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(TEXT))
                                    .child("MULTITRACK AUDIO MIXER"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(rgb(MUTED))
                                    .child("48 kHz · 24-bit · Stereo Out"),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .p_3()
                            .flex()
                            .gap_3()
                            .overflow_hidden()
                            .child(audio_channel_strip("A1: Video Audio", -2.5, false, false))
                            .child(audio_channel_strip("A2: Voiceover", 0.0, false, false))
                            .child(audio_channel_strip("A3: Music BGM", -6.0, false, false))
                            .child(audio_channel_strip("A4: Sound FX", -4.0, false, false))
                            .child(audio_master_strip("MASTER OUT", -0.5)),
                    ),
            )
            // Right: Audio Preview Player & EQ
            .child(
                div()
                    .w(px(preview_width))
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(self.monitor_panel(preview_width, preview_height * 0.65, cx))
                    .child(
                        div()
                            .flex_1()
                            .border_t_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(PANEL))
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(TEXT))
                                    .child("MASTER PARAMETRIC EQ & DYNAMICS"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .rounded(px(6.0))
                                    .bg(rgb(0x0c0d10))
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .p_2()
                                    .flex()
                                    .items_center()
                                    .justify_around()
                                    .child(eq_band_node("Low (80Hz)", "+2dB"))
                                    .child(eq_band_node("Low-Mid (250Hz)", "-1dB"))
                                    .child(eq_band_node("High-Mid (2.5kHz)", "+3dB"))
                                    .child(eq_band_node("High (12kHz)", "+1dB")),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn topbar(&self, window: &mut Window, cx: &mut Context<Self>) -> gpui::AnyElement {
        let project_name = self
            .project_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled project".to_string());

        let export_enabled = self.timeline.is_some();
        let can_undo = self.can_undo();
        let can_redo = self.can_redo();

        div()
            .id("editor-topbar")
            .h(px(TOPBAR_HEIGHT))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .border_b_1()
            .border_color(rgb(BORDER))
            .bg(rgb(BACKGROUND))
            // Left: NodaMark, NodaVideo, Divider, Project Name, Unsaved Dot
            .child(
                div()
                    .flex()
                    .min_w_0()
                    .items_center()
                    .gap_2p5()
                    .child(
                        img(IconName::NodaMark.path())
                            .size(px(24.0))
                            .flex_shrink_0(),
                    )
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT))
                            .child("NodaVideo"),
                    )
                    .child(
                        div()
                            .mx_1()
                            .h(px(16.0))
                            .border_r_1()
                            .border_color(rgb(BORDER)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .text_ellipsis()
                            .child(project_name),
                    )
                    .when(export_enabled, |this| {
                        this.child(div().size(px(6.0)).rounded_full().bg(rgb(0x818cf8)))
                    }),
            )
            // Center: Workspaces navigation tabs
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_stretch()
                    .gap_1()
                    .child(self.workspace_nav_tab("workspace-edit", "Edit", WorkspaceTab::Edit, cx))
                    .child(self.workspace_nav_tab(
                        "workspace-color",
                        "Color",
                        WorkspaceTab::Color,
                        cx,
                    ))
                    .child(self.workspace_nav_tab(
                        "workspace-audio",
                        "Audio",
                        WorkspaceTab::Audio,
                        cx,
                    ))
                    .child(self.workspace_nav_tab(
                        "workspace-ai",
                        "AI Studio",
                        WorkspaceTab::AiStudio,
                        cx,
                    )),
            )
            // Right: Undo/Redo, Dividers, Panel Toggles, Projects, Export
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_1p5()
                    // Undo
                    .child(
                        div()
                            .id("topbar-undo")
                            .h(px(28.0))
                            .w(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .when(can_undo, |this| {
                                this.bg(rgb(SURFACE))
                                    .text_color(rgb(TEXT))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    .on_click(cx.listener(|editor, _, window, cx| {
                                        editor.action_undo(&crate::actions::Undo, window, cx);
                                    }))
                            })
                            .when(!can_undo, |this| {
                                this.bg(rgb(SURFACE))
                                    .text_color(rgb(0x4a4d57))
                                    .cursor_not_allowed()
                            })
                            .text_size(px(13.0))
                            .child("↶"),
                    )
                    // Redo
                    .child(
                        div()
                            .id("topbar-redo")
                            .h(px(28.0))
                            .w(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .when(can_redo, |this| {
                                this.bg(rgb(SURFACE))
                                    .text_color(rgb(TEXT))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    .on_click(cx.listener(|editor, _, window, cx| {
                                        editor.action_redo(&crate::actions::Redo, window, cx);
                                    }))
                            })
                            .when(!can_redo, |this| {
                                this.bg(rgb(SURFACE))
                                    .text_color(rgb(0x4a4d57))
                                    .cursor_not_allowed()
                            })
                            .text_size(px(13.0))
                            .child("↷"),
                    )
                    // Divider
                    .child(
                        div()
                            .mx_1()
                            .h(px(16.0))
                            .border_r_1()
                            .border_color(rgb(BORDER)),
                    )
                    // Media Panel Toggle
                    .child(
                        div()
                            .id("toggle-media-panel-btn")
                            .h(px(28.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(3.0))
                            .border_1()
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .when(self.show_media_browser, |this| {
                                this.border_color(rgb(ACCENT))
                                    .bg(rgb(0x23243a))
                                    .text_color(rgb(TEXT))
                            })
                            .when(!self.show_media_browser, |this| {
                                this.border_color(rgb(BORDER))
                                    .bg(rgb(SURFACE))
                                    .text_color(rgb(MUTED))
                                    .hover(|s| s.text_color(rgb(TEXT)))
                            })
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.show_media_browser = !editor.show_media_browser;
                                cx.notify();
                            }))
                            .child("Media"),
                    )
                    // Inspector Toggle
                    .child(
                        div()
                            .id("toggle-inspector-btn")
                            .h(px(28.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(3.0))
                            .border_1()
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .when(self.show_inspector, |this| {
                                this.border_color(rgb(ACCENT))
                                    .bg(rgb(0x23243a))
                                    .text_color(rgb(TEXT))
                            })
                            .when(!self.show_inspector, |this| {
                                this.border_color(rgb(BORDER))
                                    .bg(rgb(SURFACE))
                                    .text_color(rgb(MUTED))
                                    .hover(|s| s.text_color(rgb(TEXT)))
                            })
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.show_inspector = !editor.show_inspector;
                                cx.notify();
                            }))
                            .child("Inspector"),
                    )
                    // AI Assistant: opens the full-window AI Studio workspace
                    .child(
                        div()
                            .id("open-assistant-btn")
                            .h(px(28.0))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(3.0))
                            .border_1()
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .when(self.workspace == WorkspaceTab::AiStudio, |this| {
                                this.border_color(rgb(ACCENT))
                                    .bg(rgb(0x23243a))
                                    .text_color(rgb(TEXT))
                            })
                            .when(self.workspace != WorkspaceTab::AiStudio, |this| {
                                this.border_color(rgb(BORDER))
                                    .bg(rgb(SURFACE))
                                    .text_color(rgb(MUTED))
                                    .hover(|s| s.text_color(rgb(TEXT)))
                            })
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.workspace = WorkspaceTab::AiStudio;
                                cx.notify();
                            }))
                            .child("AI"),
                    )
                    // Divider
                    .child(
                        div()
                            .mx_1()
                            .h(px(16.0))
                            .border_r_1()
                            .border_color(rgb(BORDER)),
                    )
                    // Projects
                    .child(
                        button_cyberpunk_secondary("topbar-projects", "Projects", true, window, cx)
                            .h(px(28.0))
                            .px_2p5()
                            .on_click(cx.listener(|_, _, _, cx| {
                                let event_bus = cx.new(|_| EventBus {});
                                if let Err(error) = project_picker::open(cx, event_bus, None) {
                                    log::error!("Could not open project picker: {error:?}");
                                }
                            })),
                    )
                    // Export Video Primary Button
                    .child(
                        button_cyberpunk_primary(
                            "topbar-export",
                            "Export Video",
                            export_enabled,
                            window,
                            cx,
                        )
                        .h(px(28.0))
                        .px_3()
                        .when(export_enabled, |button| {
                            button.on_click(cx.listener(|editor, _, _, cx| {
                                let Some(timeline) = editor.timeline.as_ref() else {
                                    return;
                                };
                                let timeline_path = timeline.path.clone();
                                editor.emit_event(cx, AppEvent::OpenExportWindow { timeline_path });
                            }))
                        }),
                    ),
            )
            .into_any_element()
    }

    fn workspace_nav_tab(
        &self,
        id: &'static str,
        label: &'static str,
        tab: WorkspaceTab,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let selected = self.workspace == tab;
        div()
            .id(id)
            .h_full()
            .px_4()
            .flex()
            .items_center()
            .justify_center()
            .border_b_2()
            .cursor_pointer()
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .when(selected, |this| {
                this.border_color(rgb(ACCENT)).text_color(rgb(TEXT))
            })
            .when(!selected, |this| {
                this.border_color(rgb(0x00000000))
                    .text_color(rgb(MUTED))
                    .hover(|s| s.text_color(rgb(TEXT)))
            })
            .on_click(cx.listener(move |editor, _, _, cx| {
                editor.workspace = tab;
                cx.notify();
            }))
            .child(label)
            .into_any_element()
    }
}

// Helpers for Color, Audio, and AI Studio views
fn color_wheel_widget(label: &'static str) -> gpui::AnyElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .child(
            div()
                .size(px(72.0))
                .rounded_full()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(0x0c0d10))
                .flex()
                .items_center()
                .justify_center()
                .child(div().size(px(8.0)).rounded_full().bg(rgb(ACCENT))),
        )
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(MUTED))
                .child(label),
        )
        .into_any_element()
}

fn color_slider_row(name: &'static str, val: &'static str) -> gpui::AnyElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .text_xs()
        .child(div().text_color(rgb(MUTED)).child(name))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(val),
        )
        .into_any_element()
}

fn audio_channel_strip(name: &'static str, db: f32, _muted: bool, _solo: bool) -> gpui::AnyElement {
    div()
        .w(px(88.0))
        .h_full()
        .rounded(px(6.0))
        .bg(rgb(SURFACE))
        .border_1()
        .border_color(rgb(BORDER))
        .p_2()
        .flex()
        .flex_col()
        .justify_between()
        .items_center()
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .text_ellipsis()
                .child(name),
        )
        // Pan Knob
        .child(
            div()
                .text_size(px(9.0))
                .px_1p5()
                .rounded_sm()
                .bg(rgb(PANEL))
                .text_color(rgb(MUTED))
                .child("PAN < C >"),
        )
        // Mute / Solo
        .child(
            div()
                .flex()
                .gap_1()
                .child(
                    div()
                        .px_1p5()
                        .py_0p5()
                        .rounded(px(2.0))
                        .bg(rgb(PANEL))
                        .text_size(px(9.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(MUTED))
                        .child("M"),
                )
                .child(
                    div()
                        .px_1p5()
                        .py_0p5()
                        .rounded(px(2.0))
                        .bg(rgb(PANEL))
                        .text_size(px(9.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(MUTED))
                        .child("S"),
                ),
        )
        // Vertical Meter & Fader
        .child(
            div()
                .flex_1()
                .w_full()
                .py_2()
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                // VU Meter bar
                .child(
                    div()
                        .w(px(6.0))
                        .h(px(100.0))
                        .rounded_full()
                        .bg(rgb(0x0a0b0d))
                        .flex()
                        .flex_col()
                        .justify_end()
                        .child(div().w_full().h(px(65.0)).rounded_full().bg(rgb(SUCCESS))),
                )
                // Fader track & handle
                .child(
                    div()
                        .w(px(14.0))
                        .h(px(100.0))
                        .rounded_sm()
                        .bg(rgb(0x0c0d10))
                        .relative()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .absolute()
                                .w(px(20.0))
                                .h(px(8.0))
                                .rounded_sm()
                                .bg(rgb(0x8a90a2))
                                .border_1()
                                .border_color(rgb(0x2e313a)),
                        ),
                ),
        )
        .child(
            div()
                .text_size(px(10.0))
                .font_family("monospace")
                .text_color(rgb(TEXT))
                .child(format!("{db:+.1} dB")),
        )
        .into_any_element()
}

fn audio_master_strip(name: &'static str, db: f32) -> gpui::AnyElement {
    div()
        .w(px(96.0))
        .h_full()
        .rounded(px(6.0))
        .bg(rgb(0x1d1e26))
        .border_1()
        .border_color(rgb(ACCENT))
        .p_2()
        .flex()
        .flex_col()
        .justify_between()
        .items_center()
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(0x818cf8))
                .child(name),
        )
        .child(
            div()
                .text_size(px(9.0))
                .px_1p5()
                .rounded_sm()
                .bg(rgb(0x23243a))
                .text_color(rgb(TEXT))
                .child("STEREO 0.0"),
        )
        .child(
            div()
                .flex_1()
                .w_full()
                .py_2()
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    div()
                        .w(px(8.0))
                        .h(px(100.0))
                        .rounded_full()
                        .bg(rgb(0x0a0b0d))
                        .flex()
                        .flex_col()
                        .justify_end()
                        .child(div().w_full().h(px(72.0)).rounded_full().bg(rgb(SUCCESS))),
                )
                .child(
                    div()
                        .w(px(16.0))
                        .h(px(100.0))
                        .rounded_sm()
                        .bg(rgb(0x0c0d10))
                        .relative()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .absolute()
                                .w(px(22.0))
                                .h(px(10.0))
                                .rounded_sm()
                                .bg(rgb(ACCENT)),
                        ),
                ),
        )
        .child(
            div()
                .text_size(px(10.0))
                .font_family("monospace")
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT))
                .child(format!("{db:+.1} dB")),
        )
        .into_any_element()
}

fn eq_band_node(label: &'static str, gain: &'static str) -> gpui::AnyElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .child(
            div()
                .size(px(20.0))
                .rounded_full()
                .bg(rgb(ACCENT))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(9.0))
                .text_color(rgb(0x131417))
                .font_weight(FontWeight::BOLD)
                .child("●"),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(MUTED))
                .child(label),
        )
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT))
                .child(gain),
        )
        .into_any_element()
}

const WORKSPACE_SPLIT_CONSTRAINTS: VerticalSplitConstraints = VerticalSplitConstraints {
    min_top: MIN_UPPER_WORKSPACE_HEIGHT,
    min_bottom: MIN_TIMELINE_HEIGHT,
};
