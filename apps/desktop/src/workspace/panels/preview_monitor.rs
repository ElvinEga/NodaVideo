//! Preview Monitor panel for playback, scrubbing, and frame inspection.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// Preview Monitor view component.
#[derive(Debug)]
pub struct PreviewMonitorPanel {
    pub is_playing: bool,
    pub is_looping: bool,
    pub timecode: SharedString,
    pub duration: SharedString,
    pub zoom_level: SharedString,
}

impl Default for PreviewMonitorPanel {
    fn default() -> Self {
        Self {
            is_playing: false,
            is_looping: false,
            timecode: "00:00:00:00".into(),
            duration: "00:00:00:00".into(),
            zoom_level: "Fit (100%)".into(),
        }
    }
}

impl PreviewMonitorPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        self.is_playing = !self.is_playing;
        cx.notify();
    }

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

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .child(
                // Panel Header
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
                                    .child("Preview Monitor"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("— Sequence 1 (1920×1080 24fps)"),
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
                    .p_4()
                    .bg(theme.background)
                    .child(
                        // Monitor Aspect Canvas Screen
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .w_full()
                            .h_full()
                            .max_w(px(880.0))
                            .max_h(px(495.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.secondary)
                            .p_6()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .p_3()
                                            .rounded_full()
                                            .bg(theme.muted)
                                            .child(Icon::new(IconName::Play)),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_medium()
                                            .text_color(theme.foreground)
                                            .child("No Sequence Loaded"),
                                    )
                                    .child(
                                        div().text_xs().text_color(theme.muted_foreground).child(
                                            "Drop clips onto the timeline to begin playback",
                                        ),
                                    ),
                            ),
                    ),
            )
            .child(
                // Transport Controls Bar
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h_10()
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
                                    .font_family(mono_font)
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
                                Button::new("step-back-btn")
                                    .ghost()
                                    .small()
                                    .icon(IconName::ChevronLeft)
                                    .on_click(|_, _, _| {}),
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
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_playback(cx);
                                    })),
                            )
                            .child(
                                Button::new("step-forward-btn")
                                    .ghost()
                                    .small()
                                    .icon(IconName::ChevronRight)
                                    .on_click(|_, _, _| {}),
                            )
                            .child(
                                Button::new("loop-btn")
                                    .ghost()
                                    .small()
                                    .icon(IconName::RotateCcw)
                                    .when(is_looping, |btn| btn.outline())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_loop(cx);
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Render: Draft 24fps"),
                    ),
            )
    }
}
