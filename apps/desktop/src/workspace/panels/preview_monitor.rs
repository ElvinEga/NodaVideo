//! Preview Monitor panel for playback, scrubbing, and frame inspection.

use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// Callback signature for transport button triggers.
type TransportCallback = Rc<dyn Fn(&mut Window, &mut App)>;

/// Preview Monitor view component.
pub struct PreviewMonitorPanel {
    pub is_playing: bool,
    pub is_looping: bool,
    pub timecode: SharedString,
    pub duration: SharedString,
    pub sequence_name: SharedString,
    pub sequence_info: SharedString,
    pub zoom_level: SharedString,
    pub current_frame_path: Option<PathBuf>,
    pub audio_meter_left: f32,
    pub audio_meter_right: f32,
    pub on_step_backward: Option<TransportCallback>,
    pub on_step_forward: Option<TransportCallback>,
    pub on_rewind_start: Option<TransportCallback>,
    pub on_forward_end: Option<TransportCallback>,
}

impl std::fmt::Debug for PreviewMonitorPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewMonitorPanel")
            .field("is_playing", &self.is_playing)
            .field("is_looping", &self.is_looping)
            .field("timecode", &self.timecode)
            .field("duration", &self.duration)
            .field("current_frame_path", &self.current_frame_path)
            .finish_non_exhaustive()
    }
}

impl Default for PreviewMonitorPanel {
    fn default() -> Self {
        Self {
            is_playing: false,
            is_looping: false,
            timecode: "00:00:00:00".into(),
            duration: "00:00:00:00".into(),
            sequence_name: "Main Sequence".into(),
            sequence_info: "1920×1080 24fps".into(),
            zoom_level: "Fit (100%)".into(),
            current_frame_path: None,
            audio_meter_left: -60.0,
            audio_meter_right: -60.0,
            on_step_backward: None,
            on_step_forward: None,
            on_rewind_start: None,
            on_forward_end: None,
        }
    }
}

impl PreviewMonitorPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update audio level meters (values in dBFS, range -60.0 to 0.0).
    pub fn update_audio_meters(&mut self, left_db: f32, right_db: f32, cx: &mut Context<Self>) {
        self.audio_meter_left = left_db.clamp(-60.0, 0.0);
        self.audio_meter_right = right_db.clamp(-60.0, 0.0);
        cx.notify();
    }

    /// Update preview frame and timecode display.
    pub fn update_frame(
        &mut self,
        frame_path: Option<PathBuf>,
        timecode: impl Into<SharedString>,
        duration: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.current_frame_path = frame_path;
        self.timecode = timecode.into();
        self.duration = duration.into();
        cx.notify();
    }

    /// Toggle playback state.
    pub fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        self.is_playing = !self.is_playing;
        cx.notify();
    }

    /// Set explicit playback state.
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.is_playing != playing {
            self.is_playing = playing;
            cx.notify();
        }
    }

    /// Toggle loop state.
    pub fn toggle_loop(&mut self, cx: &mut Context<Self>) {
        self.is_looping = !self.is_looping;
        cx.notify();
    }
}

impl Render for PreviewMonitorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_playing = self.is_playing;
        let is_looping = self.is_looping;
        let mono_font = theme.mono_font_family.clone();
        let frame_path = self
            .current_frame_path
            .as_ref()
            .filter(|p| p.exists())
            .cloned();

        let rewind_start = self.on_rewind_start.clone();
        let step_back = self.on_step_backward.clone();
        let step_fwd = self.on_step_forward.clone();
        let forward_end = self.on_forward_end.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(theme.background)
            .child(
                // Panel Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h_9()
                    .flex_shrink_0()
                    .px_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .bg(theme.secondary)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Film))
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child(format!("Preview: {}", self.sequence_name)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("— {}", self.sequence_info)),
                            ),
                    )
                    .child(
                        div().flex().items_center().gap_1().child(
                            Button::new("fit-zoom-btn")
                                .ghost()
                                .xsmall()
                                .label(self.zoom_level.clone()),
                        ),
                    ),
            )
            .child(
                // Central Video Viewport
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .p_2()
                    .bg(theme.background)
                    .child(
                        // Monitor Aspect Canvas Screen
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .size_full()
                            .max_w(px(880.0))
                            .max_h(px(495.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(theme.border)
                            .bg(gpui::rgb(0x0a0a0a))
                            .overflow_hidden()
                            .when_some(frame_path, |this, path| {
                                this.child(img(path).size_full().object_fit(ObjectFit::Contain))
                            })
                            .when(self.current_frame_path.is_none(), |this| {
                                this.p_4().child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .p_2p5()
                                                .rounded_full()
                                                .bg(theme.muted)
                                                .child(Icon::new(IconName::Play)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_medium()
                                                .text_color(theme.foreground)
                                                .child("Empty Timeline Playhead"),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child("Drag clips onto the timeline to begin playback preview"),
                                        ),
                                )
                            }),
                    ),
            )
            .child(
                // Transport Controls Bar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h_10()
                    .flex_shrink_0()
                    .px_4()
                    .border_t_1()
                    .border_color(theme.border)
                    .bg(theme.secondary)
                    .child(
                        // Timecode Display
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .font_family(mono_font.clone())
                                    .text_sm()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child(self.timecode.clone()),
                            )
                            .child(
                                div()
                                    .font_family(mono_font.clone())
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("/ {}", self.duration)),
                            ),
                    )
                    .child(
                        // Transport Buttons
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Button::new("rewind-start-btn")
                                    .ghost()
                                    .small()
                                    .icon(IconName::SkipBack)
                                    .tooltip("Rewind to Start (Home)")
                                    .on_click(move |_, win, app| {
                                        if let Some(ref cb) = rewind_start {
                                            cb(win, app);
                                        }
                                    }),
                            )
                            .child(
                                Button::new("step-back-btn")
                                    .ghost()
                                    .small()
                                    .icon(IconName::ChevronLeft)
                                    .tooltip("Step Backward 1 Frame (Left Arrow)")
                                    .on_click(move |_, win, app| {
                                        if let Some(ref cb) = step_back {
                                            cb(win, app);
                                        }
                                    }),
                            )
                            .child(
                                Button::new("play-btn")
                                    .primary()
                                    .small()
                                    .icon(if is_playing {
                                        IconName::Pause
                                    } else {
                                        IconName::Play
                                    })
                                    .tooltip(if is_playing {
                                        "Pause (Space)"
                                    } else {
                                        "Play (Space)"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_playback(cx);
                                    })),
                            )
                            .child(
                                Button::new("step-forward-btn")
                                    .ghost()
                                    .small()
                                    .icon(IconName::ChevronRight)
                                    .tooltip("Step Forward 1 Frame (Right Arrow)")
                                    .on_click(move |_, win, app| {
                                        if let Some(ref cb) = step_fwd {
                                            cb(win, app);
                                        }
                                    }),
                            )
                            .child(
                                Button::new("forward-end-btn")
                                    .ghost()
                                    .small()
                                    .icon(IconName::SkipForward)
                                    .tooltip("Forward to End (End)")
                                    .on_click(move |_, win, app| {
                                        if let Some(ref cb) = forward_end {
                                            cb(win, app);
                                        }
                                    }),
                            )
                            .child(
                                Button::new("loop-btn")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Repeat)
                                    .tooltip("Toggle Playback Loop (L)")
                                    .when(is_looping, |btn| btn.outline())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_loop(cx);
                                    })),
                            ),
                    )
                    .child({
                        let left_pct = ((self.audio_meter_left + 60.0) / 60.0).clamp(0.0, 1.0);
                        let right_pct = ((self.audio_meter_right + 60.0) / 60.0).clamp(0.0, 1.0);
                        let left_color = if self.audio_meter_left > -3.0 {
                            gpui::rgb(0xef4444)
                        } else if self.audio_meter_left > -12.0 {
                            gpui::rgb(0xf59e0b)
                        } else {
                            gpui::rgb(0x10b981)
                        };
                        let right_color = if self.audio_meter_right > -3.0 {
                            gpui::rgb(0xef4444)
                        } else if self.audio_meter_right > -12.0 {
                            gpui::rgb(0xf59e0b)
                        } else {
                            gpui::rgb(0x10b981)
                        };
                        let peak_db = self.audio_meter_left.max(self.audio_meter_right);

                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_semibold()
                                                    .text_color(theme.muted_foreground)
                                                    .child("L"),
                                            )
                                            .child(
                                                div()
                                                    .w_16()
                                                    .h_1p5()
                                                    .rounded_full()
                                                    .bg(theme.muted)
                                                    .overflow_hidden()
                                                    .child(
                                                        div()
                                                            .h_full()
                                                            .w(px(left_pct * 64.0))
                                                            .bg(left_color),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_semibold()
                                                    .text_color(theme.muted_foreground)
                                                    .child("R"),
                                            )
                                            .child(
                                                div()
                                                    .w_16()
                                                    .h_1p5()
                                                    .rounded_full()
                                                    .bg(theme.muted)
                                                    .overflow_hidden()
                                                    .child(
                                                        div()
                                                            .h_full()
                                                            .w(px(right_pct * 64.0))
                                                            .bg(right_color),
                                                    ),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .font_family(mono_font)
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(if peak_db <= -59.0 {
                                        "-inf dB".to_string()
                                    } else {
                                        format!("{peak_db:.1} dB")
                                    }),
                            )
                    }),
            )
    }
}
