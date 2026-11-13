use crate::theme::{BACKGROUND, BORDER, MUTED, SURFACE, TEXT};
use gpui::prelude::*;
use gpui::{App, Context, IntoElement, Render, Window, div, px, rgb};

pub(crate) struct SimpleTooltip {
    pub(crate) title: String,
    pub(crate) shortcut: Option<String>,
}

impl Render for SimpleTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .px_2()
            .py_1()
            .rounded(px(4.0))
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(SURFACE))
            .shadow_md()
            .text_size(px(11.0))
            .text_color(rgb(TEXT))
            .child(self.title.clone())
            .when_some(self.shortcut.clone(), |this, shortcut| {
                this.child(
                    div()
                        .px(px(4.0))
                        .py(px(0.5))
                        .rounded(px(3.0))
                        .bg(rgb(BACKGROUND))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .text_size(px(9.5))
                        .font_family("monospace")
                        .text_color(rgb(MUTED))
                        .child(shortcut),
                )
            })
    }
}

pub(crate) fn tooltip_view(
    title: impl Into<String>,
    shortcut: Option<&str>,
    cx: &mut App,
) -> gpui::AnyView {
    let title = title.into();
    let shortcut = shortcut.map(|s| s.to_string());
    cx.new(|_| SimpleTooltip { title, shortcut }).into()
}
