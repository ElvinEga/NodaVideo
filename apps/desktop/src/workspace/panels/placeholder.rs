//! Placeholder workspace views for Color, Audio, and AI Studio modes.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::*;

use crate::workspace::navigation::WorkspaceTab;

/// Render a clean placeholder view for non-Edit workspaces.
pub fn render_placeholder_workspace(tab: WorkspaceTab, cx: &App) -> impl IntoElement {
    let theme = cx.theme();

    let (icon, title, subtitle, milestone) = match tab {
        WorkspaceTab::Color => (
            IconName::SlidersHorizontal,
            "Color Grading & Scopes",
            "Color wheels, 3D LUT previews, curves, and waveform vectorscopes.",
            "Planned for Milestone M3 & M5",
        ),
        WorkspaceTab::Audio => (
            IconName::Volume2,
            "Multitrack Audio Mixer",
            "Multi-channel track faders, peak/RMS meters, parametric EQ, and mastering.",
            "Planned for Milestone M2 & M3",
        ),
        WorkspaceTab::AiStudio => (
            IconName::Sparkles,
            "AI Studio & Skills Canvas",
            "Visual workflow canvas, installed SKILL.md packages, and multi-agent coordination.",
            "Planned for Milestone M4 & M5",
        ),
        WorkspaceTab::Edit => unreachable!(),
    };

    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .size_full()
        .bg(theme.background)
        .p_8()
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_4()
                .max_w(px(520.0))
                .p_8()
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .bg(theme.secondary)
                .child(
                    div()
                        .p_4()
                        .rounded_full()
                        .bg(theme.muted)
                        .child(Icon::new(icon)),
                )
                .child(
                    div()
                        .text_lg()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .text_center()
                        .child(subtitle),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.mono_font_family.clone())
                        .p_1p5()
                        .px_3()
                        .rounded_sm()
                        .bg(theme.muted)
                        .text_color(theme.muted_foreground)
                        .child(milestone),
                ),
        )
}
