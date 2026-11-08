//! Timeline panel displaying tracks, ruler, playhead, and editing controls.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Icon, Sizable, Theme};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::theme::metrics;

/// Active timeline editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineTool {
    #[default]
    Select,
    Split,
}

/// Timeline panel view component.
#[derive(Debug)]
pub struct TimelinePanel {
    pub current_tool: TimelineTool,
    pub snapping_enabled: bool,
    pub ripple_enabled: bool,
    pub zoom_level: usize,
}

impl Default for TimelinePanel {
    fn default() -> Self {
        Self {
            current_tool: TimelineTool::Select,
            snapping_enabled: true,
            ripple_enabled: false,
            zoom_level: 100,
        }
    }
}

impl TimelinePanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_tool(&mut self, tool: TimelineTool, cx: &mut Context<Self>) {
        self.current_tool = tool;
        cx.notify();
    }

    pub fn toggle_snapping(&mut self, cx: &mut Context<Self>) {
        self.snapping_enabled = !self.snapping_enabled;
        cx.notify();
    }

    pub fn toggle_ripple(&mut self, cx: &mut Context<Self>) {
        self.ripple_enabled = !self.ripple_enabled;
        cx.notify();
    }

    pub fn zoom_in(&mut self, cx: &mut Context<Self>) {
        if self.zoom_level < 400 {
            self.zoom_level += 25;
            cx.notify();
        }
    }

    pub fn zoom_out(&mut self, cx: &mut Context<Self>) {
        if self.zoom_level > 25 {
            self.zoom_level -= 25;
            cx.notify();
        }
    }
}

impl Render for TimelinePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let snapping_enabled = self.snapping_enabled;
        let ripple_enabled = self.ripple_enabled;
        let current_tool = self.current_tool;
        let mono_font = theme.mono_font_family.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .border_t_1()
            .border_color(theme.border)
            .child(
                // Timeline Toolbar Header
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
                        // Left: Tools & Sequence Identity
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::SlidersHorizontal))
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("Timeline: Main Sequence"),
                            )
                            .child(
                                div()
                                    .h_4()
                                    .w(px(1.0))
                                    .bg(theme.border)
                                    .mx_1(),
                            )
                            .child(
                                // Select Tool (V)
                                Button::new("tool-select")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::MousePointer)
                                    .when(current_tool == TimelineTool::Select, |btn| btn.outline())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_tool(TimelineTool::Select, cx);
                                    })),
                            )
                            .child(
                                // Blade/Split Tool (B)
                                Button::new("tool-split")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Scissors)
                                    .when(current_tool == TimelineTool::Split, |btn| btn.outline())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_tool(TimelineTool::Split, cx);
                                    })),
                            )
                            .child(
                                // Snapping Magnet Toggle
                                Button::new("tool-snapping")
                                    .ghost()
                                    .xsmall()
                                    .label("Snap")
                                    .when(snapping_enabled, |btn| btn.outline())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_snapping(cx);
                                    })),
                            )
                            .child(
                                // Ripple Edit Mode Toggle
                                Button::new("tool-ripple")
                                    .ghost()
                                    .xsmall()
                                    .label("Ripple")
                                    .when(ripple_enabled, |btn| btn.outline())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_ripple(cx);
                                    })),
                            ),
                    )
                    .child(
                        // Right: Zoom controls
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
                                    .font_family(mono_font.clone())
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
            .child(
                // Timeline Canvas Area: Track Headers + Time Ruler & Track Lanes
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
                                // Ruler corner placeholder
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
                            .child(self.render_track_header("V2", "Video 2", IconName::Film, &theme))
                            .child(self.render_track_header("V1", "Video 1", IconName::Film, &theme))
                            .child(self.render_track_header("A1", "Audio 1", IconName::Volume2, &theme))
                            .child(self.render_track_header("A2", "Audio 2", IconName::Volume2, &theme)),
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
                                // Time Ruler Bar
                                div()
                                    .flex()
                                    .items_center()
                                    .h(metrics::TIMELINE_RULER_HEIGHT)
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .bg(theme.secondary)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_16()
                                            .px_2()
                                            .children([
                                                "00:00:00:00",
                                                "00:00:05:00",
                                                "00:00:10:00",
                                                "00:00:15:00",
                                                "00:00:20:00",
                                                "00:00:25:00",
                                                "00:00:30:00",
                                            ].iter().map(|mark| {
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(div().w(px(1.0)).h_3().bg(theme.border))
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_family(mono_font.clone())
                                                            .text_color(theme.muted_foreground)
                                                            .child(*mark),
                                                    )
                                            })),
                                    ),
                            )
                            .child(
                                // Track Lanes Canvas
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .relative()
                                    .child(self.render_track_lane("lane-v2", &theme))
                                    .child(self.render_track_lane("lane-v1", &theme))
                                    .child(self.render_track_lane("lane-a1", &theme))
                                    .child(self.render_track_lane("lane-a2", &theme))
                                    .child(
                                        // Playhead indicator at frame 0
                                        div()
                                            .absolute()
                                            .top_0()
                                            .bottom_0()
                                            .left(px(24.0))
                                            .w(px(1.5))
                                            .bg(theme.danger),
                                    )
                                    .child(
                                        // Empty Timeline State Overlay
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .flex_1()
                                            .p_4()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_medium()
                                                    .text_color(theme.muted_foreground)
                                                    .child("Timeline is empty. Drag media here from the Media Browser to create clips."),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}

impl TimelinePanel {
    fn render_track_header(
        &self,
        id: &'static str,
        label: &'static str,
        icon: IconName,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(metrics::TIMELINE_TRACK_HEIGHT)
            .px_2()
            .border_b_1()
            .border_color(theme.border)
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
                            .child(id),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(label),
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
                            .text_color(theme.muted_foreground)
                            .child("M"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("S"),
                    ),
            )
    }

    fn render_track_lane(&self, _id: &'static str, theme: &Theme) -> impl IntoElement {
        div()
            .h(metrics::TIMELINE_TRACK_HEIGHT)
            .w_full()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.background)
    }
}
