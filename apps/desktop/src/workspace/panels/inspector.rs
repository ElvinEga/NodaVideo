//! Inspector panel for clip attributes, transforms, effects, and audio settings.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::*;

/// Inspector view component.
#[derive(Debug, Default)]
pub struct InspectorPanel {
    pub has_selection: bool,
}

impl InspectorPanel {
    pub fn new() -> Self {
        Self {
            has_selection: false,
        }
    }
}

impl Render for InspectorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .border_l_1()
            .border_color(theme.border)
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
                            .child(Icon::new(IconName::SlidersHorizontal))
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("Inspector"),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("Properties"),
                    ),
            )
            .child(
                // Contextual Body
                if self.has_selection {
                    self.render_properties(cx).into_any_element()
                } else {
                    self.render_empty_selection(cx).into_any_element()
                },
            )
    }
}

impl InspectorPanel {
    fn render_empty_selection(&self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .p_4()
            .gap_4()
            .child(
                // Empty selection note
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .p_4()
                    .gap_2()
                    .rounded_md()
                    .bg(theme.muted)
                    .border_1()
                    .border_color(theme.border)
                    .child(Icon::new(IconName::SlidersHorizontal))
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(theme.foreground)
                            .child("No Clip Selected"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .text_center()
                            .child("Select a clip on the timeline or in the media bin to inspect its transform, audio, and effects."),
                    ),
            )
            .child(
                // Preview of structured sections
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(self.render_section_preview(
                        "Video Transform",
                        &[("Position X / Y", "0.0 / 0.0 px"), ("Scale", "100.0%"), ("Rotation", "0.0°"), ("Opacity", "100%")],
                        cx,
                    ))
                    .child(self.render_section_preview(
                        "Audio Parameters",
                        &[("Volume", "0.0 dB"), ("Pan", "Center (0)"), ("Channels", "Stereo (L/R)")],
                        cx,
                    ))
                    .child(self.render_section_preview(
                        "Color & Effects",
                        &[("Color Space", "Rec.709"), ("LUT", "None"), ("Effects Stack", "0 applied")],
                        cx,
                    )),
            )
    }

    fn render_section_preview(
        &self,
        title: &'static str,
        fields: &[(&'static str, &'static str)],
        cx: &App,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let mono_font = theme.mono_font_family.clone();

        div()
            .flex()
            .flex_col()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary)
            .p_3()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(theme.muted_foreground)
                    .child(title.to_uppercase()),
            )
            .children(fields.iter().map(|(label, val)| {
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .py_0p5()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(*label),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font.clone())
                            .text_color(theme.foreground)
                            .child(*val),
                    )
            }))
    }

    fn render_properties(&self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .p_4()
            .text_xs()
            .text_color(theme.foreground)
            .child("Clip selected properties")
    }
}
