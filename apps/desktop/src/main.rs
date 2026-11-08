#![recursion_limit = "512"]

use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::*;

#[derive(Debug)]
pub struct WelcomeView;

impl WelcomeView {
    pub const fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for WelcomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .bg(theme.background)
            .text_color(theme.foreground)
            .p_8()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .max_w(px(560.0))
                    .p_8()
                    .rounded_xl()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.secondary)
                    .shadow_lg()
                    .child(
                        div()
                            .text_3xl()
                            .font_bold()
                            .text_color(theme.foreground)
                            .child("NodaVideo"),
                    )
                    .child(
                        div()
                            .text_base()
                            .font_medium()
                            .text_color(theme.muted_foreground)
                            .child("Your timeline. Your creativity. Your AI."),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .p_2()
                            .rounded_md()
                            .bg(theme.muted)
                            .child("Milestone M0 · Desktop Foundation"),
                    )
                    .child(
                        div().flex().gap_3().pt_4().child(
                            Button::new("quit-btn")
                                .outline()
                                .label("Quit")
                                .on_click(|_, _, cx| {
                                    cx.quit();
                                }),
                        ),
                    ),
            )
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);

        let bounds = Bounds::centered(None, size(px(1080.0), px(720.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("NodaVideo".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        open_window(options, cx, |window, cx| {
            cx.new(|cx| WelcomeView::new(window, cx))
        })
        .expect("Failed to open window");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[core::prelude::v1::test]
    fn test_welcome_view_instantiation() {
        let view = WelcomeView;
        assert_eq!(format!("{view:?}"), "WelcomeView");
    }
}
