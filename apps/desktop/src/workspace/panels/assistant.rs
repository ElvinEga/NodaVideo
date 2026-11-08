//! AI Assistant dockable panel for agent conversation and video-editing skills.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme, Icon, Sizable, Theme};
use gpui_kit::*;

use crate::workspace::menus::ToggleAssistant;

/// AI Assistant panel view component.
#[derive(Debug)]
pub struct AssistantPanel {
    pub prompt_input: Entity<InputState>,
}

impl AssistantPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let prompt_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Ask AI assistant or describe an edit…")
        });

        Self { prompt_input }
    }
}

impl Render for AssistantPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

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
                            .child(Icon::new(IconName::Sparkles))
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("AI Assistant"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .p_0p5()
                                    .px_1p5()
                                    .rounded_sm()
                                    .bg(theme.muted)
                                    .text_color(theme.muted_foreground)
                                    .child("M4 Preview"),
                            ),
                    )
                    .child(
                        Button::new("close-assistant-btn")
                            .ghost()
                            .xsmall()
                            .icon(IconName::X)
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(ToggleAssistant), cx);
                            }),
                    ),
            )
            .child(
                // Conversation Area: Empty State & Capabilities Guide
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_hidden()
                    .p_4()
                    .gap_4()
                    .child(
                        // Overview Card
                        div()
                            .flex()
                            .flex_col()
                            .p_3()
                            .gap_2()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.secondary)
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("AI Editing Engine (Planned M4)"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("Connects external AI CLIs via ACP, Codex app-server, and Claude Code. The editor owns all project state; agents propose mutations that you review, approve, or undo."),
                            ),
                    )
                    .child(
                        // Capability shortcuts
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.muted_foreground)
                                    .child("AVAILABLE IN M4 & M5"),
                            )
                            .child(self.render_capability_card(
                                "✂️ Smart Cuts",
                                "Detect and ripple-delete silence, jump-cuts, and pauses.",
                                &theme,
                            ))
                            .child(self.render_capability_card(
                                "📝 Auto Captions",
                                "Speech-to-text transcription with synchronized subtitle clips.",
                                &theme,
                            ))
                            .child(self.render_capability_card(
                                "🎨 Color & Look Matching",
                                "Analyze shots and apply coherent color grading proposals.",
                                &theme,
                            )),
                    ),
            )
            .child(
                // Prompt Input Box
                div()
                    .flex()
                    .flex_col()
                    .p_3()
                    .gap_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .bg(theme.secondary)
                    .child(Input::new(&self.prompt_input))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("Press Return to send"),
                            )
                            .child(
                                Button::new("send-prompt-btn")
                                    .primary()
                                    .xsmall()
                                    .label("Send")
                                    .icon(IconName::Send)
                                    .on_click(|_, _, _| {}),
                            ),
                    ),
            )
    }
}

impl AssistantPanel {
    fn render_capability_card(
        &self,
        title: &'static str,
        desc: &'static str,
        theme: &Theme,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .p_2p5()
            .gap_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary)
            .child(
                div()
                    .text_xs()
                    .font_medium()
                    .text_color(theme.foreground)
                    .child(title),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(desc),
            )
    }
}
