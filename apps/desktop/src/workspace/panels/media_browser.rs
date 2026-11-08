//! Media Browser panel for asset management and import.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// Media category filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaCategory {
    #[default]
    All,
    Video,
    Audio,
    Graphics,
}

/// Media Browser panel view state and elements.
#[derive(Debug)]
pub struct MediaBrowserPanel {
    pub search_input: Entity<InputState>,
    pub selected_category: MediaCategory,
}

impl MediaBrowserPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search media bin…"));

        Self {
            search_input,
            selected_category: MediaCategory::All,
        }
    }

    pub fn set_category(&mut self, category: MediaCategory, cx: &mut Context<Self>) {
        self.selected_category = category;
        cx.notify();
    }
}

impl Render for MediaBrowserPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .border_r_1()
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
                            .child(Icon::new(IconName::Folder))
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("Media Browser"),
                            ),
                    )
                    .child(
                        Button::new("import-btn")
                            .ghost()
                            .xsmall()
                            .label("Import")
                            .icon(IconName::Plus)
                            .on_click(|_, _, _| {
                                // M1 import pipeline placeholder
                            }),
                    ),
            )
            .child(
                // Search Field & Filter Controls
                div()
                    .flex()
                    .flex_col()
                    .p_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(Input::new(&self.search_input))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(self.render_filter_button("All", MediaCategory::All, cx))
                            .child(self.render_filter_button("Video", MediaCategory::Video, cx))
                            .child(self.render_filter_button("Audio", MediaCategory::Audio, cx))
                            .child(self.render_filter_button(
                                "Graphics",
                                MediaCategory::Graphics,
                                cx,
                            )),
                    ),
            )
            .child(
                // Content area: Sample media item representation + Empty state explanation
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_hidden()
                    .p_3()
                    .gap_3()
                    .child(
                        // Sample representative item
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .p_2()
                            .rounded_md()
                            .bg(theme.muted)
                            .border_1()
                            .border_color(theme.border)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w_12()
                                    .h_9()
                                    .rounded_sm()
                                    .bg(theme.secondary)
                                    .child(Icon::new(IconName::Film)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_medium()
                                            .text_color(theme.foreground)
                                            .child("A001_C001_RAW.mp4"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.muted_foreground)
                                                    .child("00:01:24:12"),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.muted_foreground)
                                                    .child("3840×2160 24fps"),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        // Empty State Explanation
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .flex_1()
                            .p_4()
                            .gap_2()
                            .border_1()
                            .border_dashed()
                            .border_color(theme.border)
                            .rounded_md()
                            .child(
                                div()
                                    .p_2()
                                    .rounded_full()
                                    .bg(theme.muted)
                                    .child(Icon::new(IconName::Folder)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_medium()
                                    .text_color(theme.foreground)
                                    .child("Bin Empty"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .text_center()
                                    .child("Drag video, audio, or graphics files here to import."),
                            ),
                    ),
            )
    }
}

impl MediaBrowserPanel {
    fn render_filter_button(
        &self,
        label: &'static str,
        category: MediaCategory,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let is_selected = self.selected_category == category;

        Button::new(label)
            .xsmall()
            .label(label)
            .when(is_selected, |btn| btn.secondary())
            .when(!is_selected, |btn| btn.ghost())
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_category(category, cx);
            }))
    }
}
