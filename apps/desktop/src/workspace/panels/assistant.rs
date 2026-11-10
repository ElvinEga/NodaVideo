//! AI Assistant dockable panel for agent conversation, skill library, and interactive approvals.

use std::rc::Rc;
use std::sync::{Arc, RwLock};

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme, Disableable, Icon, Sizable, Theme};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use nodavideo_agent_runtime::{AgentConfig, AgentEvent, AgentRuntime, SessionId};
use nodavideo_skill_registry::SkillRegistry;

use crate::workspace::menus::ToggleAssistant;

/// Current status of a tool approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

/// Formatted message inside the assistant conversation history.
#[derive(Debug, Clone)]
pub enum ChatMessage {
    User(String),
    Assistant(String),
    Thought(String),
    ToolCall {
        name: String,
        summary: String,
    },
    ApprovalPrompt {
        request_id: String,
        title: String,
        description: String,
        command_summary: String,
        tool_name: String,
        parameters: serde_json::Value,
        status: ApprovalStatus,
    },
    System(String),
}

/// Callback signature for executing an MCP tool against editor state on approval.
pub type ToolExecutionCallback = Rc<
    dyn Fn(&str, &serde_json::Value, &mut Window, &mut App) -> Result<serde_json::Value, String>,
>;

/// AI Assistant panel view component.
pub struct AssistantPanel {
    pub prompt_input: Entity<InputState>,
    pub runtime: Arc<AgentRuntime>,
    pub skill_registry: Arc<RwLock<SkillRegistry>>,
    pub current_provider_id: String,
    pub current_session_id: Option<SessionId>,
    pub messages: Vec<ChatMessage>,
    pub is_processing: bool,
    pub status_text: String,
    pub on_tool_call: Option<ToolExecutionCallback>,
    event_task: Option<Task<()>>,
}

impl std::fmt::Debug for AssistantPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssistantPanel")
            .field("current_provider_id", &self.current_provider_id)
            .field("messages_count", &self.messages.len())
            .field("is_processing", &self.is_processing)
            .finish_non_exhaustive()
    }
}

impl AssistantPanel {
    pub fn new(
        runtime: Arc<AgentRuntime>,
        skill_registry: Arc<RwLock<SkillRegistry>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let prompt_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Ask AI assistant or describe an edit…")
        });

        Self {
            prompt_input,
            runtime,
            skill_registry,
            current_provider_id: "acp-antigravity".into(),
            current_session_id: None,
            messages: vec![ChatMessage::System(
                "NodaVideo AI Studio ready. Select an agent provider or type an editing instruction below.".into(),
            )],
            is_processing: false,
            status_text: "Antigravity Ready".into(),
            on_tool_call: None,
            event_task: None,
        }
    }

    /// Switch active agent provider.
    pub fn cycle_provider(&mut self, cx: &mut Context<Self>) {
        let next_provider = match self.current_provider_id.as_str() {
            "acp-antigravity" => "acp-gemini",
            "acp-gemini" => "codex-app-server",
            "codex-app-server" => "claude-cli",
            "claude-cli" => "mock",
            _ => "acp-antigravity",
        };
        self.current_provider_id = next_provider.to_string();
        self.current_session_id = None;
        self.status_text = format!("Provider: {}", self.provider_display_name());
        self.messages.push(ChatMessage::System(format!(
            "Switched provider to {}",
            self.provider_display_name()
        )));
        cx.notify();
    }

    pub fn provider_display_name(&self) -> &str {
        match self.current_provider_id.as_str() {
            "acp-antigravity" => "Antigravity (ACP)",
            "acp-gemini" => "Gemini CLI (ACP)",
            "codex-app-server" => "Codex (App-Server)",
            "claude-cli" => "Claude Code (CLI)",
            "mock" => "Mock Assistant (Offline)",
            other => other,
        }
    }

    /// Send the user prompt into the active session.
    pub fn send_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.prompt_input.read(cx).value().to_string();
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        // Clear input field
        self.prompt_input
            .update(cx, |input, cx| input.set_value("", window, cx));

        self.messages.push(ChatMessage::User(trimmed.clone()));
        self.is_processing = true;
        self.status_text = "Agent thinking...".into();
        cx.notify();

        let runtime = self.runtime.clone();
        let provider_id = self.current_provider_id.clone();
        let session_opt = self.current_session_id;

        self.event_task = Some(cx.spawn(async move |this, cx| {
            let session_id = if let Some(sid) = session_opt {
                sid
            } else {
                let config = AgentConfig {
                    provider_id: provider_id.clone(),
                    ..Default::default()
                };
                match runtime.start_session(config).await {
                    Ok(id) => {
                        this.update(cx, |this, _| {
                            this.current_session_id = Some(id);
                        })
                        .ok();
                        id
                    }
                    Err(err) => {
                        this.update(cx, |this, cx| {
                            this.messages.push(ChatMessage::System(format!(
                                "Failed to start session with {provider_id}: {err}"
                            )));
                            this.is_processing = false;
                            this.status_text = "Error".into();
                            cx.notify();
                        })
                        .ok();
                        return;
                    }
                }
            };

            let Ok(mut rx) = runtime.subscribe(session_id).await else {
                return;
            };

            if let Err(err) = runtime.send_message(session_id, trimmed).await {
                this.update(cx, |this, cx| {
                    this.messages
                        .push(ChatMessage::System(format!("Send failed: {err}")));
                    this.is_processing = false;
                    this.status_text = "Error".into();
                    cx.notify();
                })
                .ok();
                return;
            }

            // Stream incoming events from session
            while let Ok(event) = rx.recv().await {
                let should_stop = this
                    .update(cx, |this, cx| {
                        match event {
                            AgentEvent::MessageChunk(chunk) => {
                                // Append or push
                                if let Some(ChatMessage::Assistant(existing)) =
                                    this.messages.last_mut()
                                {
                                    existing.push_str(&chunk);
                                } else {
                                    this.messages.push(ChatMessage::Assistant(chunk));
                                }
                            }
                            AgentEvent::ThoughtChunk(thought) => {
                                this.messages.push(ChatMessage::Thought(thought));
                            }
                            AgentEvent::ToolCallRequested {
                                tool_name,
                                arguments,
                                ..
                            } => {
                                let summary = format!("{tool_name}({arguments})");
                                this.messages.push(ChatMessage::ToolCall {
                                    name: tool_name,
                                    summary,
                                });
                            }
                            AgentEvent::ApprovalRequested {
                                request_id,
                                title,
                                description,
                                tool_name,
                                command_summary,
                                parameters,
                            } => {
                                this.status_text = "Approval required".into();
                                this.messages.push(ChatMessage::ApprovalPrompt {
                                    request_id,
                                    title,
                                    description,
                                    command_summary,
                                    tool_name,
                                    parameters,
                                    status: ApprovalStatus::Pending,
                                });
                            }
                            AgentEvent::ApprovalDecision {
                                request_id,
                                approved,
                            } => {
                                for msg in &mut this.messages {
                                    if let ChatMessage::ApprovalPrompt {
                                        request_id: r_id,
                                        status,
                                        ..
                                    } = msg
                                    {
                                        if r_id == &request_id {
                                            *status = if approved {
                                                ApprovalStatus::Approved
                                            } else {
                                                ApprovalStatus::Rejected
                                            };
                                        }
                                    }
                                }
                            }
                            AgentEvent::SessionCompleted { summary } => {
                                this.is_processing = false;
                                this.status_text = "Ready".into();
                                if let Some(sum) = summary {
                                    this.messages.push(ChatMessage::Assistant(sum));
                                }
                                cx.notify();
                                return true;
                            }
                            AgentEvent::SessionError { code, message } => {
                                this.is_processing = false;
                                this.status_text = format!("Error: {code}");
                                this.messages.push(ChatMessage::System(format!(
                                    "Error ({code}): {message}"
                                )));
                                cx.notify();
                                return true;
                            }
                            AgentEvent::ToolCallResult { .. } => {}
                        }
                        cx.notify();
                        false
                    })
                    .unwrap_or(true);

                if should_stop {
                    break;
                }
            }
        }));
    }

    /// Handle user approval decision on a tool prompt.
    pub fn handle_approval_decision(
        &mut self,
        request_id: String,
        approved: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session_id) = self.current_session_id else {
            return;
        };

        let mut tool_to_execute = None;

        for msg in &mut self.messages {
            if let ChatMessage::ApprovalPrompt {
                request_id: r_id,
                tool_name,
                parameters,
                status,
                ..
            } = msg
            {
                if r_id == &request_id && *status == ApprovalStatus::Pending {
                    *status = if approved {
                        ApprovalStatus::Approved
                    } else {
                        ApprovalStatus::Rejected
                    };
                    if approved {
                        tool_to_execute = Some((tool_name.clone(), parameters.clone()));
                    }
                }
            }
        }

        // Execute tool through callback on timeline state if approved
        if let Some((tool_name, params)) = tool_to_execute {
            if let Some(ref cb) = self.on_tool_call {
                match cb(&tool_name, &params, window, cx) {
                    Ok(res) => {
                        self.messages.push(ChatMessage::System(format!(
                            "Executed tool '{tool_name}': {res}"
                        )));
                    }
                    Err(err) => {
                        self.messages
                            .push(ChatMessage::System(format!("Tool execution error: {err}")));
                    }
                }
            }
        }

        let runtime = self.runtime.clone();
        cx.spawn(async move |_, _| {
            let _ = runtime
                .submit_approval(session_id, request_id, approved)
                .await;
        })
        .detach();

        cx.notify();
    }
}

impl Render for AssistantPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let mono_font = theme.mono_font_family.clone();
        let provider_name = self.provider_display_name().to_string();
        let is_processing = self.is_processing;
        let cx_ref: &Context<Self> = cx;

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
                                Button::new("provider-toggle-btn")
                                    .ghost()
                                    .xsmall()
                                    .label(provider_name)
                                    .tooltip("Switch Agent Provider (Antigravity / Gemini / Codex / Claude / Mock)")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cycle_provider(cx);
                                    })),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(theme.muted)
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(format!(
                                        "{} Skills",
                                        self.skill_registry
                                            .read()
                                            .map_or(0, |r| r.list().len())
                                    )),
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
                // Message History Stream
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_hidden()
                    .p_3()
                    .gap_3()
                    .children(self.messages.iter().enumerate().map(|(idx, msg)| {
                        self.render_message_item(idx, msg, &theme, mono_font.clone(), cx_ref)
                    })),
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
                            .child(div().text_xs().text_color(theme.muted_foreground).child(
                                if is_processing {
                                    "Agent is thinking…"
                                } else {
                                    "Press Return or click Send"
                                },
                            ))
                            .child(
                                Button::new("send-prompt-btn")
                                    .primary()
                                    .xsmall()
                                    .label("Send")
                                    .icon(IconName::Send)
                                    .when(is_processing, |btn| btn.disabled(true))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.send_prompt(window, cx);
                                    })),
                            ),
                    ),
            )
    }
}

impl AssistantPanel {
    fn render_message_item(
        &self,
        idx: usize,
        msg: &ChatMessage,
        theme: &Theme,
        mono_font: SharedString,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        match msg {
            ChatMessage::User(text) => div().flex().flex_col().items_end().child(
                div()
                    .max_w(px(280.0))
                    .p_2p5()
                    .rounded_lg()
                    .bg(theme.primary)
                    .text_xs()
                    .text_color(theme.primary_foreground)
                    .child(text.clone()),
            ),

            ChatMessage::Assistant(text) => div().flex().flex_col().items_start().child(
                div()
                    .max_w(px(300.0))
                    .p_2p5()
                    .rounded_lg()
                    .bg(theme.secondary)
                    .border_1()
                    .border_color(theme.border)
                    .text_xs()
                    .text_color(theme.foreground)
                    .child(text.clone()),
            ),

            ChatMessage::Thought(thought) => div()
                .flex()
                .items_center()
                .gap_1p5()
                .p_2()
                .rounded_md()
                .bg(theme.muted)
                .text_xs()
                .italic()
                .text_color(theme.muted_foreground)
                .child(Icon::new(IconName::Sparkles).xsmall())
                .child(thought.clone()),

            ChatMessage::ToolCall { name, summary } => div()
                .flex()
                .flex_col()
                .p_2()
                .gap_1()
                .rounded_sm()
                .border_1()
                .border_color(theme.border)
                .bg(theme.secondary)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .text_xs()
                        .font_medium()
                        .text_color(theme.primary)
                        .child(Icon::new(IconName::SlidersHorizontal).xsmall())
                        .child(format!("Tool Call: {name}")),
                )
                .child(
                    div()
                        .font_family(mono_font)
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(summary.clone()),
                ),

            ChatMessage::ApprovalPrompt {
                request_id,
                title,
                description,
                command_summary,
                tool_name,
                status,
                ..
            } => {
                let req_id_approve = request_id.clone();
                let req_id_reject = request_id.clone();
                let is_pending = *status == ApprovalStatus::Pending;
                let is_changeset = tool_name == "propose_changeset";

                div()
                    .flex()
                    .flex_col()
                    .p_3()
                    .gap_2()
                    .rounded_md()
                    .border_1()
                    .border_color(if is_pending {
                        theme.primary
                    } else {
                        theme.border
                    })
                    .bg(theme.secondary)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .text_xs()
                                    .font_bold()
                                    .text_color(theme.foreground)
                                    .child(Icon::new(IconName::Sparkles).xsmall())
                                    .child(title.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_medium()
                                    .text_color(match status {
                                        ApprovalStatus::Pending => theme.primary,
                                        ApprovalStatus::Approved => gpui::rgb(0x22c55e).into(),
                                        ApprovalStatus::Rejected => theme.danger,
                                    })
                                    .child(match status {
                                        ApprovalStatus::Pending => "Approval Requested",
                                        ApprovalStatus::Approved => "Approved & Applied",
                                        ApprovalStatus::Rejected => "Rejected",
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(description.clone()),
                    )
                    .child(
                        div()
                            .font_family(mono_font)
                            .p_1p5()
                            .rounded_sm()
                            .bg(theme.muted)
                            .text_xs()
                            .text_color(theme.foreground)
                            .child(command_summary.clone()),
                    )
                    .when(is_pending, |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    Button::new(format!("appr-btn-{idx}"))
                                        .primary()
                                        .xsmall()
                                        .label(if is_changeset {
                                            "Approve & Commit Staged Edit"
                                        } else {
                                            "Approve & Apply"
                                        })
                                        .on_click(cx.listener(move |this, _, win, cx| {
                                            this.handle_approval_decision(
                                                req_id_approve.clone(),
                                                true,
                                                win,
                                                cx,
                                            );
                                        })),
                                )
                                .child(
                                    Button::new(format!("reject-btn-{idx}"))
                                        .ghost()
                                        .xsmall()
                                        .label("Reject")
                                        .on_click(cx.listener(move |this, _, win, cx| {
                                            this.handle_approval_decision(
                                                req_id_reject.clone(),
                                                false,
                                                win,
                                                cx,
                                            );
                                        })),
                                ),
                        )
                    })
            }

            ChatMessage::System(msg) => div()
                .flex()
                .items_center()
                .justify_center()
                .p_1p5()
                .text_xs()
                .italic()
                .text_color(theme.muted_foreground)
                .child(msg.clone()),
        }
    }
}
