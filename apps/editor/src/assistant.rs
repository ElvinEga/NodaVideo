//! AI assistant window: provider cycling, streamed conversation, and approval cards.

use crate::asset::IconName;
use crate::assistant_history::{
    AssistantSession, AssistantSessionMeta, load_session, load_session_metas, now_ms, save_session,
    session_day_label, session_title,
};
use crate::edit_action::EditAction;
use crate::editor::Editor;
use crate::event_bus::AppEvent;
use crate::theme::{
    ACCENT, ACCENT_BACKGROUND, BACKGROUND, BORDER, ERROR, LINK, MUTED, PANEL, SUCCESS, SURFACE,
    SURFACE_HOVER, TEXT, WARNING,
};
use crate::ui_components::TextInput;
use crate::ui_components::cyberpunk_button::{
    button_cyberpunk_primary, button_cyberpunk_secondary,
};
use anyhow::Result;
use futures::StreamExt;
use futures::channel::mpsc;
use gpui::prelude::*;
use gpui::{
    AsyncApp, Context, CursorStyle, Entity, FontWeight, ScrollHandle, Stateful, WeakEntity, Window,
    div, px, rgb, svg,
};
use nodavideo_agent_acp::AcpAgentProvider;
use nodavideo_agent_claude::ClaudeAgentProvider;
use nodavideo_agent_codex::CodexAgentProvider;
use nodavideo_agent_opencode::OpenCodeAgentProvider;
use nodavideo_agent_runtime::{
    AgentConfig, AgentEvent, AgentModel, AgentRuntime, MockAgentProvider, PermissionMode, SessionId,
};
use nodavideo_hyperframes_bridge::{CompositionAsset, load_compositions};
use nodavideo_mcp_server::{ExplorerChange, McpError, McpExecutor};
use nodavideo_skill_registry::{SkillPackage, SkillRegistry};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use timeline::{Asset, TimelineEditingState};
use ulid::Ulid;

pub(crate) struct AssistantState {
    runtime: AgentRuntime,
    provider_index: usize,
    session: Option<SessionId>,
    /// Incremented when the provider changes so a stale event pump stops applying events.
    session_generation: u64,
    entries: Vec<AssistantEntry>,
    pending_approval: Option<AssistantApproval>,
    input: Entity<TextInput>,
    scroll: ScrollHandle,
    busy: bool,
    provider_menu_open: bool,
    /// Per-provider readiness probe: None until `is_available` answers. 与 entries 无关，可跨会话缓存。
    provider_readiness: Vec<Option<bool>>,
    /// Per-provider model catalog, queried from the provider itself; cached across sessions.
    model_catalogs: Vec<ModelCatalog>,
    /// Per-provider chosen model id; None means the provider's default model.
    model_choices: Vec<Option<String>>,
    /// Per-provider chosen reasoning effort; None means the active model's default.
    effort_choices: Vec<Option<String>>,
    model_menu_open: bool,
    effort_menu_open: bool,
    /// How much freedom the agent has to act without approval; applied at session start.
    permission_mode: PermissionMode,
    permission_menu_open: bool,
    /// Production skills discovered from `skills/`; the enabled set is the source of truth
    /// for the active skill, so the composer button never stores a duplicate copy.
    skills: SkillRegistry,
    skill_menu_open: bool,
    /// Whether the sessions rail is visible; the tab bar toggle flips it.
    sessions_sidebar_open: bool,
    /// Tool call ids whose card body is expanded; cards start collapsed.
    expanded_tools: HashSet<String>,
    /// Entry indices of thought rows expanded to their full text; rows start at one line.
    expanded_thoughts: HashSet<usize>,
    /// Per-group expand/collapse overrides keyed by start index; true means expanded.
    /// Absent means the default: a live run stays open, a finished run collapses.
    group_overrides: HashMap<usize, bool>,
    /// Request ids of approval cards expanded to full detail; cards start truncated.
    expanded_approvals: HashSet<String>,
    /// When the running turn started; None between turns.
    turn_started_at: Option<Instant>,
    /// How long the most recent finished turn took; shown in its speaker header.
    last_turn_elapsed: Option<Duration>,
    /// Saved sessions of the open project, newest first; refreshed after every save.
    history: Vec<AssistantSessionMeta>,
    /// A saved transcript replayed read-only; None shows the live thread.
    viewing: Option<AssistantSession>,
    /// Id of the live transcript once it has a message; None before the first send.
    transcript_id: Option<Ulid>,
    transcript_created_at_ms: u64,
}

pub(crate) fn register_assistant_providers(runtime: &AgentRuntime, cx: &mut Context<Editor>) {
    let runtime = runtime.clone();
    gpui_tokio::Tokio::spawn(cx, async move {
        runtime
            .register_provider(Arc::new(MockAgentProvider::new()))
            .await;
        runtime
            .register_provider(Arc::new(AcpAgentProvider::antigravity()))
            .await;
        runtime
            .register_provider(Arc::new(AcpAgentProvider::gemini()))
            .await;
        runtime
            .register_provider(Arc::new(CodexAgentProvider::default()))
            .await;
        runtime
            .register_provider(Arc::new(ClaudeAgentProvider::default()))
            .await;
        runtime
            .register_provider(Arc::new(OpenCodeAgentProvider::default()))
            .await;
    })
    .detach();
}

// Public APIs only
impl AssistantState {
    pub(crate) fn new(input: Entity<TextInput>) -> Self {
        Self {
            runtime: AgentRuntime::new(),
            provider_index: 0,
            session: None,
            session_generation: 0,
            entries: Vec::new(),
            pending_approval: None,
            input,
            scroll: ScrollHandle::new(),
            busy: false,
            provider_menu_open: false,
            provider_readiness: vec![None; ASSISTANT_PROVIDERS.len()],
            model_catalogs: vec![ModelCatalog::NotProbed; ASSISTANT_PROVIDERS.len()],
            model_choices: vec![None; ASSISTANT_PROVIDERS.len()],
            effort_choices: vec![None; ASSISTANT_PROVIDERS.len()],
            model_menu_open: false,
            effort_menu_open: false,
            permission_mode: PermissionMode::default(),
            permission_menu_open: false,
            skills: SkillRegistry::new(),
            skill_menu_open: false,
            sessions_sidebar_open: true,
            expanded_tools: HashSet::new(),
            expanded_thoughts: HashSet::new(),
            group_overrides: HashMap::new(),
            expanded_approvals: HashSet::new(),
            turn_started_at: None,
            last_turn_elapsed: None,
            history: Vec::new(),
            viewing: None,
            transcript_id: None,
            transcript_created_at_ms: 0,
        }
    }

    pub(crate) fn runtime(&self) -> &AgentRuntime {
        &self.runtime
    }
}

// Public APIs only
impl Editor {
    pub(super) fn assistant_window_view(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let assistant = &self.assistant;
        let provider = assistant.provider();
        let gateway_revision = self.timeline.as_ref().map(|timeline| timeline.revision);
        let viewing = assistant.viewing.as_ref();
        let entries = match viewing {
            Some(session) => &session.entries[..],
            None => &assistant.entries[..],
        };
        let thread_provider = match viewing {
            Some(session) => assistant_provider_for_id(&session.provider_id),
            None => provider,
        };

        let tab_label = match viewing {
            Some(session) => session.title.clone(),
            None => match assistant.session {
                Some(_) => provider.display_name.to_string(),
                None => "New chat".to_string(),
            },
        };
        let tab_bar = div()
            .h(px(40.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_2()
            .px(px(12.0))
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .h(px(26.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(4.0))
                    .bg(rgb(SURFACE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .child(assistant_dot(provider.hue, 7.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(TEXT))
                            .child(tab_label),
                    ),
            )
            .child(assistant_new_chat_button_view("assistant-tab-new", cx))
            .child(
                div()
                    .h(px(22.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SURFACE))
                    .child(
                        svg()
                            .path(IconName::Plug.path())
                            .size(px(11.0))
                            .text_color(rgb(MUTED)),
                    )
                    .child(
                        div()
                            .font_family("monospace")
                            .text_size(px(10.0))
                            .text_color(rgb(MUTED))
                            .child(match gateway_revision {
                                Some(revision) => format!("MCP · rev {revision}"),
                                None => "MCP · no timeline".to_string(),
                            }),
                    ),
            )
            .child(div().flex_1())
            .when(assistant.busy, |this| {
                this.child(
                    div()
                        .font_family("monospace")
                        .text_size(px(11.0))
                        .text_color(rgb(ACCENT))
                        .child("WORKING…"),
                )
            })
            .child(
                div()
                    .id("assistant-sidebar-toggle")
                    .h(px(24.0))
                    .w(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0))
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                    .on_click(cx.listener(|editor, _, _, cx| {
                        editor.assistant.sessions_sidebar_open =
                            !editor.assistant.sessions_sidebar_open;
                        cx.notify();
                    }))
                    .child(
                        svg()
                            .path(IconName::PanelLeft.path())
                            .size(px(13.0))
                            .text_color(rgb(if assistant.sessions_sidebar_open {
                                TEXT
                            } else {
                                MUTED
                            })),
                    ),
            );

        let last_turn_start = entries
            .iter()
            .rposition(|entry| matches!(entry, AssistantEntry::User(_)))
            .map(|index| index + 1)
            .unwrap_or(0);
        let last_turn_header_index = entries[last_turn_start..]
            .iter()
            .position(assistant_entry_is_prose)
            .map(|offset| last_turn_start + offset);
        let turn_label = if viewing.is_some() {
            None
        } else if assistant.busy {
            Some("working…".to_string())
        } else {
            assistant.last_turn_elapsed.map(format_turn_elapsed)
        };

        // Consecutive tool/thought entries collapse into one "Ran N tools" group; every
        // other entry renders on its own. (start, end, is_group)
        let mut items: Vec<(usize, usize, bool)> = Vec::new();
        let mut cursor = 0;
        while cursor < entries.len() {
            if assistant_entry_is_activity(&entries[cursor]) {
                let start = cursor;
                while cursor < entries.len() && assistant_entry_is_activity(&entries[cursor]) {
                    cursor += 1;
                }
                items.push((start, cursor, true));
            } else {
                items.push((cursor, cursor + 1, false));
                cursor += 1;
            }
        }

        let messages = div()
            .id("assistant-messages")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&assistant.scroll)
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .when(entries.is_empty(), |this| {
                this.child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(16.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT))
                                .child("What are you making?"),
                        )
                        .child(
                            div()
                                .max_w(px(420.0))
                                .text_size(px(12.0))
                                .line_height(px(18.0))
                                .text_color(rgb(MUTED))
                                .child(
                                    "Ask the assistant to inspect the timeline, propose an edit, or \
                                     explain a clip. Edits that need approval appear here as cards.",
                                ),
                        ),
                )
            })
            .children(items.into_iter().map(|(start, end, is_group)| {
                let first = &entries[start];
                let show_header = assistant_entry_is_prose(first)
                    && (start == 0 || !assistant_entry_is_prose(&entries[start - 1]));
                let header_label = if show_header
                    && last_turn_header_index.is_some_and(|index| index >= start && index < end)
                {
                    turn_label.as_deref()
                } else {
                    None
                };
                let content = if is_group {
                    let is_live = viewing.is_none() && assistant.busy && end == entries.len();
                    let collapsed = assistant_group_collapsed(
                        &entries[start..end],
                        start,
                        is_live,
                        &assistant.group_overrides,
                    );
                    assistant_activity_group_view(assistant, &entries[start..end], start, collapsed, cx)
                } else {
                    assistant_entry_view(first)
                };
                if show_header {
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(assistant_speaker_header_view(thread_provider, header_label))
                        .child(content)
                        .into_any_element()
                } else {
                    content
                }
            }));

        let approval = match viewing {
            Some(_) => None,
            None => assistant.pending_approval.as_ref().map(|approval| {
                let expanded = assistant.expanded_approvals.contains(&approval.request_id);
                assistant_approval_card_view(approval, expanded, window, cx)
            }),
        };

        let input_empty = assistant.input.read(cx).text().trim().is_empty();
        let composer = div()
            .flex_shrink_0()
            .mx_3()
            .mt_2()
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(BACKGROUND))
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(10.0))
                    .pt(px(10.0))
                    .child(assistant.input.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .px(px(8.0))
                    .py(px(8.0))
                    .child(assistant_skill_button_view(assistant, cx))
                    .child(assistant_model_button_view(assistant, cx))
                    .child(assistant_model_picker_view(assistant, cx))
                    .when(!assistant.effort_options().is_empty(), |this| {
                        this.child(assistant_effort_picker_view(assistant, cx))
                    })
                    .when(assistant.provider().supports_permissions, |this| {
                        this.child(assistant_permission_picker_view(assistant, cx))
                    })
                    .child(div().flex_1())
                    .child(if assistant.busy {
                        button_cyberpunk_secondary("assistant-stop", "Stop", true, window, cx)
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.emit_event(cx, AppEvent::AssistantStopped);
                            }))
                    } else {
                        button_cyberpunk_primary(
                            "assistant-send",
                            "Send",
                            !input_empty && viewing.is_none(),
                            window,
                            cx,
                        )
                        .on_click(cx.listener(|editor, _, _, cx| {
                            let message =
                                editor.assistant.input.read(cx).text().trim().to_string();
                            if message.is_empty() {
                                return;
                            }
                            editor
                                .assistant
                                .input
                                .update(cx, |input, cx| input.clear(cx));
                            editor.emit_event(cx, AppEvent::AssistantMessageSubmitted { message });
                        }))
                    }),
            );

        let project_name = self
            .project_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Project".to_string());
        let state_label = if viewing.is_some() {
            "Viewing saved session · New chat to continue"
        } else if assistant.busy {
            "Working…"
        } else {
            "Ready"
        };
        let context_line = div()
            .flex_shrink_0()
            .px(px(12.0))
            .py(px(6.0))
            .text_size(px(10.0))
            .text_color(rgb(MUTED))
            .child(format!("{project_name} · {state_label}"));

        let (approval_label, approval_color) = match assistant.pending_approval {
            Some(_) => ("1 approval pending", WARNING),
            None => ("No pending approvals", MUTED),
        };
        let gateway_label = match gateway_revision {
            Some(revision) => format!("MCP gateway · rev {revision}"),
            None => "MCP gateway · no timeline open".to_string(),
        };
        let model_status = match viewing {
            Some(session) => {
                let model_label = match &session.model {
                    Some(model) => model.clone(),
                    None => "Default model".to_string(),
                };
                match &session.effort {
                    Some(effort) => format!("{model_label} · {}", effort_display_name(effort)),
                    None => model_label,
                }
            }
            None => {
                if assistant.effort_options().is_empty() {
                    assistant.model_label()
                } else {
                    format!("{} · {}", assistant.model_label(), assistant.effort_label())
                }
            }
        };
        let status_bar = div()
            .h(px(28.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .border_t_1()
            .border_color(rgb(BORDER))
            .text_size(px(10.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .text_color(rgb(MUTED))
                    .child(
                        svg()
                            .path(IconName::Plug.path())
                            .size(px(11.0))
                            .text_color(rgb(MUTED)),
                    )
                    .child(gateway_label),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_color(rgb(MUTED))
                            .child(format!("{} · {}", thread_provider.transport, model_status)),
                    )
                    .child(div().text_color(rgb(approval_color)).child(approval_label)),
            );

        div()
            .id("editor-assistant-window")
            .relative()
            .size_full()
            .flex()
            .overflow_hidden()
            .bg(rgb(PANEL))
            .text_color(rgb(TEXT))
            .when(assistant.sessions_sidebar_open, |this| {
                this.child(assistant_sessions_sidebar_view(assistant, cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(tab_bar)
                    .child(messages)
                    .when_some(approval, |this, approval| this.child(approval))
                    .child(composer)
                    .child(context_line)
                    .child(status_bar),
            )
            .when(assistant.provider_menu_open, |this| {
                this.child(assistant_provider_menu_view(assistant, cx))
            })
            .when(assistant.skill_menu_open, |this| {
                this.child(assistant_skill_menu_view(assistant, cx))
            })
            .when(assistant.model_menu_open, |this| {
                this.child(assistant_model_menu_view(assistant, cx))
            })
            .when(assistant.effort_menu_open, |this| {
                this.child(assistant_effort_menu_view(assistant, cx))
            })
            .when(assistant.permission_menu_open, |this| {
                this.child(assistant_permission_menu_view(assistant, cx))
            })
            .into_any_element()
    }

    pub(super) fn send_assistant_message(&mut self, message: String, cx: &mut Context<Self>) {
        if self.assistant.busy || self.assistant.viewing.is_some() {
            return;
        }
        self.assistant
            .push_entry(AssistantEntry::User(message.clone()));
        if self.assistant.transcript_id.is_none() {
            self.assistant.transcript_id = Some(Ulid::generate());
            self.assistant.transcript_created_at_ms = now_ms();
        }
        self.assistant.busy = true;
        self.assistant.turn_started_at = Some(Instant::now());
        self.assistant.last_turn_elapsed = None;
        self.assistant.scroll.scroll_to_bottom();
        cx.notify();

        let runtime = self.assistant.runtime.clone();
        let provider_id = self.assistant.provider_id().to_string();
        let existing_session = self.assistant.session;
        let generation = self.assistant.session_generation;
        let working_dir = self.project_root.clone();
        let model = self.assistant.active_model().map(|model| model.id.clone());
        let effort = self.assistant.active_effort().map(str::to_string);
        let permission_mode = self.assistant.permission_mode;

        // Providers deliver only the message string, so the active skill's instructions ride
        // along on the first message of a session. A reused session already carries them; a
        // skill change resets the session, so this never disagrees with the composer button.
        let instructions = self.assistant.skills.build_combined_instructions();
        let outgoing_message = if existing_session.is_none() && !instructions.is_empty() {
            format!("{}\n\n{}", instructions.trim_end(), message)
        } else {
            message
        };

        cx.spawn(async move |editor, cx| {
            let session = match existing_session {
                Some(session) => session,
                None => {
                    let config = AgentConfig {
                        provider_id,
                        model,
                        effort,
                        permission_mode,
                        working_dir: Some(working_dir),
                        ..AgentConfig::default()
                    };
                    let started = gpui_tokio::Tokio::spawn(cx, {
                        let runtime = runtime.clone();
                        async move { runtime.start_session(config).await }
                    })
                    .await;
                    let session = match started {
                        Ok(Ok(session)) => session,
                        Ok(Err(error)) => {
                            assistant_failed(
                                &editor,
                                cx,
                                format!("Could not start the agent session: {error}"),
                            );
                            return;
                        }
                        Err(error) => {
                            assistant_failed(
                                &editor,
                                cx,
                                format!("Starting the agent session failed: {error}"),
                            );
                            return;
                        }
                    };
                    let subscribed = gpui_tokio::Tokio::spawn(cx, {
                        let runtime = runtime.clone();
                        async move { runtime.subscribe(session).await }
                    })
                    .await;
                    let receiver = match subscribed {
                        Ok(Ok(receiver)) => receiver,
                        Ok(Err(error)) => {
                            assistant_failed(
                                &editor,
                                cx,
                                format!("Could not subscribe to the agent session: {error}"),
                            );
                            return;
                        }
                        Err(error) => {
                            assistant_failed(
                                &editor,
                                cx,
                                format!("Subscribing to the agent session failed: {error}"),
                            );
                            return;
                        }
                    };
                    let stored = editor.update(cx, |editor, cx| {
                        if editor.assistant.session_generation != generation {
                            return false;
                        }
                        editor.assistant.session = Some(session);
                        cx.notify();
                        true
                    });
                    if stored.ok() != Some(true) {
                        // The provider changed while the session was starting.
                        gpui_tokio::Tokio::spawn(cx, {
                            let runtime = runtime.clone();
                            async move {
                                let _ = runtime.cancel(session).await;
                            }
                        })
                        .detach();
                        return;
                    }
                    start_assistant_event_pump(receiver, generation, editor.clone(), cx);
                    session
                }
            };

            let sent = gpui_tokio::Tokio::spawn(cx, {
                let runtime = runtime.clone();
                async move { runtime.send_message(session, outgoing_message).await }
            })
            .await;
            match sent {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    assistant_failed(&editor, cx, format!("Could not send the message: {error}"));
                }
                Err(error) => {
                    assistant_failed(&editor, cx, format!("Sending the message failed: {error}"));
                }
            }
        })
        .detach();
    }

    pub(super) fn submit_assistant_approval(&mut self, approved: bool, cx: &mut Context<Self>) {
        let Some(approval) = self.assistant.pending_approval.take() else {
            return;
        };
        let Some(session) = self.assistant.session else {
            self.assistant.push_entry(AssistantEntry::Error(
                "The approval has no active agent session.".to_string(),
            ));
            cx.notify();
            return;
        };
        self.assistant.push_entry(AssistantEntry::Info(if approved {
            format!("Approved: {}", approval.title)
        } else {
            format!("Rejected: {}", approval.title)
        }));
        self.assistant.scroll.scroll_to_bottom();
        cx.notify();

        let runtime = self.assistant.runtime.clone();
        let request_id = approval.request_id;
        gpui_tokio::Tokio::spawn(cx, async move {
            if let Err(error) = runtime.submit_approval(session, request_id, approved).await {
                log::error!("Could not submit the approval decision: {error:?}");
            }
        })
        .detach();
    }

    pub(super) fn select_assistant_provider(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= ASSISTANT_PROVIDERS.len() {
            return;
        }
        if index == self.assistant.provider_index {
            self.assistant.provider_menu_open = false;
            cx.notify();
            return;
        }
        let previous_session = self.assistant.select_provider(index);
        let runtime = self.assistant.runtime.clone();
        if let Some(session) = previous_session {
            gpui_tokio::Tokio::spawn(cx, async move {
                if let Err(error) = runtime.cancel(session).await {
                    log::debug!("Could not cancel the previous agent session: {error:?}");
                }
            })
            .detach();
        }
        self.assistant.push_entry(AssistantEntry::Info(format!(
            "Provider: {}",
            self.assistant.provider_display_name()
        )));
        self.assistant.scroll.scroll_to_bottom();
        self.persist_assistant_transcript(cx);
        self.probe_assistant_models(cx);
        cx.notify();
    }

    /// Choosing a model resets the live session so the next message starts the agent
    /// with the new model; an effort the new model does not offer is dropped.
    pub(super) fn select_assistant_model(&mut self, model: Option<String>, cx: &mut Context<Self>) {
        self.assistant.model_menu_open = false;
        let index = self.assistant.provider_index;
        if self.assistant.model_choices[index] == model {
            cx.notify();
            return;
        }
        self.assistant.model_choices[index] = model.clone();
        let supported = match self.assistant.active_model() {
            Some(active) => active.efforts.clone(),
            None => Vec::new(),
        };
        if let Some(effort) = &self.assistant.effort_choices[index]
            && !supported.contains(effort)
        {
            self.assistant.effort_choices[index] = None;
        }
        let previous_session = self.assistant.reset_session();
        let label = match model {
            Some(_) => self.assistant.model_label(),
            None => format!("{} (default)", self.assistant.model_label()),
        };
        self.assistant
            .push_entry(AssistantEntry::Info(format!("Model: {label}")));
        self.assistant.scroll.scroll_to_bottom();
        self.persist_assistant_transcript(cx);
        cx.notify();
        if let Some(session) = previous_session {
            let runtime = self.assistant.runtime.clone();
            gpui_tokio::Tokio::spawn(cx, async move {
                if let Err(error) = runtime.cancel(session).await {
                    log::debug!("Could not cancel the previous agent session: {error:?}");
                }
            })
            .detach();
        }
    }

    /// Choosing an effort resets the live session so the next turn runs with it.
    pub(super) fn select_assistant_effort(
        &mut self,
        effort: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.assistant.effort_menu_open = false;
        let index = self.assistant.provider_index;
        if self.assistant.effort_choices[index] == effort {
            cx.notify();
            return;
        }
        self.assistant.effort_choices[index] = effort.clone();
        let previous_session = self.assistant.reset_session();
        let label = match effort {
            Some(effort) => effort_display_name(&effort),
            None => format!("{} (default)", self.assistant.effort_label()),
        };
        self.assistant
            .push_entry(AssistantEntry::Info(format!("Reasoning effort: {label}")));
        self.assistant.scroll.scroll_to_bottom();
        self.persist_assistant_transcript(cx);
        cx.notify();
        if let Some(session) = previous_session {
            let runtime = self.assistant.runtime.clone();
            gpui_tokio::Tokio::spawn(cx, async move {
                if let Err(error) = runtime.cancel(session).await {
                    log::debug!("Could not cancel the previous agent session: {error:?}");
                }
            })
            .detach();
        }
    }

    /// Choosing a permission mode resets the live session so the next message starts the
    /// agent with the new permissions; providers fix them at session creation.
    pub(super) fn select_assistant_permission_mode(
        &mut self,
        mode: PermissionMode,
        cx: &mut Context<Self>,
    ) {
        self.assistant.permission_menu_open = false;
        if self.assistant.permission_mode == mode {
            cx.notify();
            return;
        }
        self.assistant.permission_mode = mode;
        let previous_session = self.assistant.reset_session();
        self.assistant.push_entry(AssistantEntry::Info(format!(
            "Permissions: {}",
            permission_mode_label(mode)
        )));
        self.assistant.scroll.scroll_to_bottom();
        self.persist_assistant_transcript(cx);
        cx.notify();
        if let Some(session) = previous_session {
            let runtime = self.assistant.runtime.clone();
            gpui_tokio::Tokio::spawn(cx, async move {
                if let Err(error) = runtime.cancel(session).await {
                    log::debug!("Could not cancel the previous agent session: {error:?}");
                }
            })
            .detach();
        }
    }

    /// Retries a failed model catalog probe for the active provider.
    pub(super) fn refresh_assistant_models(&mut self, cx: &mut Context<Self>) {
        let index = self.assistant.provider_index;
        self.assistant.model_catalogs[index] = ModelCatalog::NotProbed;
        self.probe_assistant_models(cx);
    }

    /// Discovers the production skills shipped in `skills/` off the UI thread, using the
    /// same compile-time resource path the motion templates use. A missing or empty dir
    /// leaves the registry empty and the composer shows no skills.
    pub(super) fn load_assistant_skills(&mut self, cx: &mut Context<Self>) {
        let skills_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../skills");
        cx.spawn(async move |editor, cx| {
            let discovered = gpui_tokio::Tokio::spawn(cx, async move {
                let mut registry = SkillRegistry::new();
                if let Err(error) = registry.discover_from_dir(&skills_dir) {
                    log::debug!("Could not discover assistant skills: {error:?}");
                }
                registry
            })
            .await;
            let Ok(registry) = discovered else {
                return;
            };
            let _ = editor.update(cx, |editor, cx| {
                editor.assistant.skills = registry;
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn stop_assistant(&mut self, cx: &mut Context<Self>) {
        if !self.assistant.busy {
            return;
        }
        self.assistant.finish_turn_timing();
        let previous_session = self.assistant.reset_session();
        self.assistant
            .push_entry(AssistantEntry::Info("Stopped.".to_string()));
        self.assistant.scroll.scroll_to_bottom();
        self.persist_assistant_transcript(cx);
        cx.notify();
        if let Some(session) = previous_session {
            let runtime = self.assistant.runtime.clone();
            gpui_tokio::Tokio::spawn(cx, async move {
                if let Err(error) = runtime.cancel(session).await {
                    log::debug!("Could not cancel the assistant session: {error:?}");
                }
            })
            .detach();
        }
    }

    pub(super) fn new_assistant_chat(&mut self, cx: &mut Context<Self>) {
        self.assistant.viewing = None;
        self.persist_assistant_transcript(cx);
        let previous_session = self.assistant.reset_session();
        self.assistant.entries.clear();
        self.assistant.expanded_tools.clear();
        self.assistant.expanded_thoughts.clear();
        self.assistant.group_overrides.clear();
        self.assistant.expanded_approvals.clear();
        self.assistant.last_turn_elapsed = None;
        self.assistant.transcript_id = None;
        self.assistant.provider_menu_open = false;
        self.assistant.skill_menu_open = false;
        self.assistant.model_menu_open = false;
        self.assistant.effort_menu_open = false;
        cx.notify();
        if let Some(session) = previous_session {
            let runtime = self.assistant.runtime.clone();
            gpui_tokio::Tokio::spawn(cx, async move {
                if let Err(error) = runtime.cancel(session).await {
                    log::debug!("Could not cancel the assistant session: {error:?}");
                }
            })
            .detach();
        }
    }

    pub(super) fn select_assistant_skill(&mut self, id: String, cx: &mut Context<Self>) {
        let previous_session = self.assistant.select_skill(&id);
        self.persist_assistant_transcript(cx);
        cx.notify();
        if let Some(session) = previous_session {
            let runtime = self.assistant.runtime.clone();
            gpui_tokio::Tokio::spawn(cx, async move {
                if let Err(error) = runtime.cancel(session).await {
                    log::debug!("Could not cancel the previous agent session: {error:?}");
                }
            })
            .detach();
        }
    }

    /// Writes the live transcript to `.opencut/assistant/` off the UI thread, then refreshes
    /// the sidebar list from disk so saved rows always reflect what is actually stored.
    pub(super) fn persist_assistant_transcript(&mut self, cx: &mut Context<Self>) {
        if self.assistant.viewing.is_some() {
            return;
        }
        let Some(id) = self.assistant.transcript_id else {
            return;
        };
        if self.assistant.entries.is_empty() {
            return;
        }
        let session = AssistantSession {
            id,
            provider_id: self.assistant.provider_id().to_string(),
            model: self.assistant.model_choices[self.assistant.provider_index].clone(),
            effort: self.assistant.effort_choices[self.assistant.provider_index].clone(),
            created_at_ms: self.assistant.transcript_created_at_ms,
            updated_at_ms: now_ms(),
            title: session_title(&self.assistant.entries),
            entries: self.assistant.entries.clone(),
        };
        let project_root = self.project_root.clone();
        cx.spawn(async move |editor, cx| {
            let saved: Result<Vec<AssistantSessionMeta>> = cx
                .background_executor()
                .spawn(async move {
                    save_session(&project_root, &session)?;
                    Ok(load_session_metas(&project_root))
                })
                .await;
            match saved {
                Ok(history) => {
                    if let Err(error) = editor.update(cx, |editor, cx| {
                        editor.assistant.history = history;
                        cx.notify();
                    }) {
                        log::debug!("Editor gone before assistant history refresh: {error:?}");
                    }
                }
                Err(error) => log::warn!("Could not persist the assistant session: {error:?}"),
            }
        })
        .detach();
    }

    /// Reads this project's saved session list off the UI thread at startup.
    pub(super) fn load_assistant_history(&mut self, cx: &mut Context<Self>) {
        let project_root = self.project_root.clone();
        cx.spawn(async move |editor, cx| {
            let history = cx
                .background_executor()
                .spawn(async move { load_session_metas(&project_root) })
                .await;
            if let Err(error) = editor.update(cx, |editor, cx| {
                editor.assistant.history = history;
                cx.notify();
            }) {
                log::debug!("Editor gone before assistant history load: {error:?}");
            }
        })
        .detach();
    }

    /// Opens a saved transcript read-only. The live session, if any, keeps running untouched.
    pub(super) fn select_assistant_session(&mut self, id: Ulid, cx: &mut Context<Self>) {
        let project_root = self.project_root.clone();
        cx.spawn(async move |editor, cx| {
            let loaded = cx
                .background_executor()
                .spawn(async move { load_session(&project_root, id) })
                .await;
            match loaded {
                Ok(session) => {
                    if let Err(error) = editor.update(cx, |editor, cx| {
                        editor.assistant.viewing = Some(session);
                        editor.assistant.expanded_thoughts.clear();
                        editor.assistant.group_overrides.clear();
                        editor.assistant.expanded_approvals.clear();
                        editor.assistant.scroll.scroll_to_bottom();
                        cx.notify();
                    }) {
                        log::debug!("Editor gone before assistant session open: {error:?}");
                    }
                }
                Err(error) => log::warn!("Could not open the assistant session: {error:?}"),
            }
        })
        .detach();
    }

    pub(super) fn resume_live_assistant(&mut self, cx: &mut Context<Self>) {
        self.assistant.viewing = None;
        self.assistant.expanded_thoughts.clear();
        self.assistant.group_overrides.clear();
        self.assistant.expanded_approvals.clear();
        self.assistant.scroll.scroll_to_bottom();
        cx.notify();
    }
}

// Private APIs only
impl Editor {
    /// Probes each provider's real readiness via `is_available`, off the UI thread.
    /// Results cache across sessions; an unanswered probe stays `None` and retries next open.
    fn probe_assistant_providers(&mut self, cx: &mut Context<Self>) {
        for (index, provider) in ASSISTANT_PROVIDERS.iter().enumerate() {
            if self.assistant.provider_readiness[index].is_some() {
                continue;
            }
            let runtime = self.assistant.runtime.clone();
            let provider_id = provider.id;
            cx.spawn(async move |editor, cx| {
                let probed = gpui_tokio::Tokio::spawn(cx, async move {
                    match runtime.get_provider(provider_id).await {
                        Some(provider) => provider.is_available().await,
                        None => false,
                    }
                })
                .await;
                if let Ok(available) = probed {
                    let _ = editor.update(cx, |editor, cx| {
                        editor.assistant.provider_readiness[index] = Some(available);
                        cx.notify();
                    });
                }
            })
            .detach();
        }
    }

    /// Queries the active provider's real model catalog off the UI thread. Results cache
    /// across sessions; a failed probe stays failed until the user retries from the menu.
    fn probe_assistant_models(&mut self, cx: &mut Context<Self>) {
        let index = self.assistant.provider_index;
        match &self.assistant.model_catalogs[index] {
            ModelCatalog::Loading | ModelCatalog::Loaded(_) => return,
            ModelCatalog::NotProbed | ModelCatalog::Failed(_) => {}
        }
        self.assistant.model_catalogs[index] = ModelCatalog::Loading;
        cx.notify();
        let runtime = self.assistant.runtime.clone();
        let provider_id = self.assistant.provider_id().to_string();
        cx.spawn(async move |editor, cx| {
            let listed = gpui_tokio::Tokio::spawn(cx, async move {
                match runtime.get_provider(&provider_id).await {
                    Some(provider) => provider.list_models().await,
                    None => Err(
                        nodavideo_agent_runtime::AgentRuntimeError::ProviderUnavailable(
                            provider_id.clone(),
                            "the provider is not registered".into(),
                        ),
                    ),
                }
            })
            .await;
            let catalog = match listed {
                Ok(Ok(models)) => ModelCatalog::Loaded(models),
                Ok(Err(error)) => ModelCatalog::Failed(error.to_string()),
                Err(error) => ModelCatalog::Failed(error.to_string()),
            };
            let _ = editor.update(cx, |editor, cx| {
                editor.assistant.model_catalogs[index] = catalog;
                if index == editor.assistant.provider_index {
                    let supported = match editor.assistant.active_model() {
                        Some(model) => model.efforts.clone(),
                        None => Vec::new(),
                    };
                    if let Some(effort) = &editor.assistant.effort_choices[index]
                        && !supported.contains(effort)
                    {
                        editor.assistant.effort_choices[index] = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Runs an in-process provider's tool call off the UI thread, then applies the result.
    /// Approval-required tools were already gated by the provider's ApprovalRequested card,
    /// so execution here is approved; the executor still validates before committing.
    fn begin_assistant_tool_call(
        &mut self,
        call_id: String,
        tool_name: String,
        arguments: Value,
        cx: &mut Context<Self>,
    ) {
        self.assistant.push_entry(AssistantEntry::Tool {
            call_id: call_id.clone(),
            tool_name: tool_name.clone(),
            arguments: arguments.to_string(),
            status: ToolStatus::Running,
            result: None,
        });
        self.assistant.scroll.scroll_to_bottom();
        cx.notify();

        let seed = self.timeline.as_ref().map(|timeline| {
            let mut media_library: Vec<Asset> = self
                .explorer
                .media_metadata
                .iter()
                .filter(|(path, _)| self.explorer.imported_paths.contains(*path))
                .map(|(_, asset)| asset.clone())
                .collect();
            media_library.sort_by(|left, right| left.path().cmp(right.path()));
            (
                timeline.editing_state.clone(),
                timeline.interaction.selected_clip_ids.clone(),
                timeline.revision,
                media_library,
            )
        });
        let project_root = self.project_root.clone();
        let generation = self.assistant.session_generation;

        cx.spawn(async move |editor, cx| {
            let background_tool_name = tool_name.clone();
            let outcome = cx
                .background_executor()
                .spawn(async move {
                    run_assistant_tool_background(
                        background_tool_name,
                        &arguments,
                        seed,
                        &project_root,
                    )
                })
                .await;
            let _ = editor.update(cx, |editor, cx| {
                if editor.assistant.session_generation != generation {
                    return;
                }
                editor.finish_assistant_tool_call(call_id, tool_name, outcome, cx);
            });
        })
        .detach();
    }

    fn finish_assistant_tool_call(
        &mut self,
        call_id: String,
        tool_name: String,
        mut outcome: AssistantToolOutcome,
        cx: &mut Context<Self>,
    ) {
        if let Some(timeline) = outcome.mutated_timeline {
            self.emit_event(cx, AppEvent::Edit(EditAction::ApplyAgentEdit { timeline }));
        }
        if let Some(compositions) = outcome.reloaded_compositions {
            self.explorer.compositions = compositions;
            self.schedule_composition_thumbnails(cx);
        }
        if let Some(change) = outcome.explorer_change.take() {
            // 媒体库写请求由编辑器权威应用；失败时把工具结果改写为真实错误，避免上报虚假成功。
            let applied = match change {
                ExplorerChange::Import { paths } => {
                    self.import_external_paths(&paths, cx);
                    Ok(())
                }
                ExplorerChange::Remove { asset_id } => self.remove_media_asset(asset_id, cx),
                ExplorerChange::Rename { asset_id, name } => {
                    self.rename_media_asset(asset_id, name)
                }
            };
            if let Err(error) = applied {
                outcome.result = Err(McpError::ExecutionFailed(format!("{error:#}")));
            }
        }

        let (result, is_error) = match outcome.result {
            Ok(result) => (result, false),
            Err(error) => (json!({ "error": error.to_string() }), true),
        };
        self.assistant
            .complete_tool_call(&call_id, &tool_name, result.to_string(), is_error);
        self.assistant.scroll.scroll_to_bottom();
        cx.notify();

        let Some(session) = self.assistant.session else {
            return;
        };
        let runtime = self.assistant.runtime.clone();
        gpui_tokio::Tokio::spawn(cx, async move {
            if let Err(error) = runtime
                .submit_tool_result(session, call_id, result, is_error)
                .await
            {
                log::error!("Could not deliver the tool result to the agent: {error:?}");
            }
        })
        .detach();
    }
}

// Private APIs only
impl AssistantState {
    fn provider(&self) -> &'static AssistantProvider {
        &ASSISTANT_PROVIDERS[self.provider_index]
    }

    fn provider_id(&self) -> &'static str {
        self.provider().id
    }

    fn provider_display_name(&self) -> &'static str {
        self.provider().display_name
    }

    fn provider_executes_in_process(&self) -> bool {
        self.provider().executes_in_process
    }

    fn catalog(&self) -> &ModelCatalog {
        &self.model_catalogs[self.provider_index]
    }

    fn catalog_models(&self) -> &[AgentModel] {
        match self.catalog() {
            ModelCatalog::Loaded(models) => models,
            ModelCatalog::NotProbed | ModelCatalog::Loading | ModelCatalog::Failed(_) => &[],
        }
    }

    /// The model that will actually run: the explicit choice, else the catalog default.
    fn active_model(&self) -> Option<&AgentModel> {
        let models = self.catalog_models();
        if let Some(choice) = &self.model_choices[self.provider_index]
            && let Some(model) = models.iter().find(|model| &model.id == choice)
        {
            return Some(model);
        }
        match models.iter().find(|model| model.is_default) {
            Some(model) => Some(model),
            None => models.first(),
        }
    }

    fn model_label(&self) -> String {
        match self.active_model() {
            Some(model) => model.display_name.clone(),
            None => match self.catalog() {
                ModelCatalog::Loading => "Loading models…".to_string(),
                ModelCatalog::Failed(_) => "Model list unavailable".to_string(),
                ModelCatalog::Loaded(_) => "No models reported".to_string(),
                ModelCatalog::NotProbed => "Default model".to_string(),
            },
        }
    }

    /// Model label for a provider menu row, which may not be the active provider.
    fn provider_model_label(&self, index: usize) -> String {
        if let Some(choice) = &self.model_choices[index] {
            if let ModelCatalog::Loaded(models) = &self.model_catalogs[index]
                && let Some(model) = models.iter().find(|model| &model.id == choice)
            {
                return model.display_name.clone();
            }
            return choice.clone();
        }
        "Default model".to_string()
    }

    fn effort_options(&self) -> &[String] {
        match self.active_model() {
            Some(model) => &model.efforts,
            None => &[],
        }
    }

    /// The effort that will actually run; a choice the active model does not offer
    /// falls back to the model default instead of reaching the backend.
    fn active_effort(&self) -> Option<&str> {
        let options = self.effort_options();
        if let Some(choice) = self.effort_choices[self.provider_index].as_deref()
            && options.iter().any(|effort| effort == choice)
        {
            return Some(choice);
        }
        self.active_model()
            .and_then(|model| model.default_effort.as_deref())
    }

    fn effort_label(&self) -> String {
        match self.active_effort() {
            Some(effort) => effort_display_name(effort),
            None => "Default".to_string(),
        }
    }

    fn select_provider(&mut self, index: usize) -> Option<SessionId> {
        self.provider_index = index;
        self.provider_menu_open = false;
        self.model_menu_open = false;
        self.effort_menu_open = false;
        self.reset_session()
    }

    /// Single-active skill toggle: choosing the active skill clears it, choosing another
    /// replaces it. The session resets so the next message carries the new instructions —
    /// the button never claims a skill the running agent hasn't been given.
    fn select_skill(&mut self, id: &str) -> Option<SessionId> {
        self.skill_menu_open = false;
        let was_active = self.skills.is_enabled(id);
        for enabled_id in self.skills.enabled_skills().to_vec() {
            self.skills.set_enabled(&enabled_id, false);
        }
        if !was_active {
            self.skills.set_enabled(id, true);
        }
        self.reset_session()
    }

    fn active_skill(&self) -> Option<&SkillPackage> {
        let id = self.skills.enabled_skills().first()?;
        self.skills.get(id)
    }

    /// Drops the live session so a stale event pump stops applying events. Returns the old
    /// session for the caller to cancel off-thread. Shared by provider switch, skill change
    /// and Stop.
    fn reset_session(&mut self) -> Option<SessionId> {
        self.session_generation += 1;
        self.pending_approval = None;
        self.busy = false;
        self.turn_started_at = None;
        self.session.take()
    }

    /// Stamps the running turn's duration once it ends, so its speaker header can
    /// report how long the agent actually worked.
    fn finish_turn_timing(&mut self) {
        if let Some(started_at) = self.turn_started_at.take() {
            self.last_turn_elapsed = Some(started_at.elapsed());
        }
    }

    fn push_entry(&mut self, entry: AssistantEntry) {
        const MAX_ENTRIES: usize = 400;
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.remove(0);
            // 前截断使所有下标减一；同步移动按下标键控的展开集合，避免展开行错位。
            self.expanded_thoughts = shift_expansion_indices(&self.expanded_thoughts);
            self.group_overrides = shift_group_overrides(&self.group_overrides);
        }
        self.entries.push(entry);
    }

    /// Fills in the result on the matching tool card. If the request was never shown
    /// (e.g. a result arrived after a provider switch), append a completed card so the
    /// outcome is still visible rather than dropped.
    fn complete_tool_call(
        &mut self,
        call_id: &str,
        tool_name: &str,
        result: String,
        is_error: bool,
    ) {
        let status = if is_error {
            ToolStatus::Failed
        } else {
            ToolStatus::Done
        };
        for entry in self.entries.iter_mut().rev() {
            let AssistantEntry::Tool {
                call_id: id,
                status: slot,
                result: slot_result,
                ..
            } = entry
            else {
                continue;
            };
            if id != call_id {
                continue;
            }
            *slot = status;
            *slot_result = Some(result);
            return;
        }
        self.push_entry(AssistantEntry::Tool {
            call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            arguments: String::new(),
            status,
            result: Some(result),
        });
    }

    fn apply_agent_event(&mut self, event: AgentEvent, cx: &mut Context<Editor>) {
        match event {
            AgentEvent::MessageChunk(chunk) => {
                let appended = match self.entries.last_mut() {
                    Some(AssistantEntry::Message(text)) => {
                        text.push_str(&chunk);
                        true
                    }
                    Some(AssistantEntry::Thought(_))
                    | Some(AssistantEntry::User(_))
                    | Some(AssistantEntry::Tool { .. })
                    | Some(AssistantEntry::Info(_))
                    | Some(AssistantEntry::Error(_))
                    | None => false,
                };
                if !appended {
                    self.push_entry(AssistantEntry::Message(chunk));
                }
            }
            AgentEvent::ThoughtChunk(chunk) => {
                let appended = match self.entries.last_mut() {
                    Some(AssistantEntry::Thought(text)) => {
                        text.push_str(&chunk);
                        true
                    }
                    Some(AssistantEntry::Message(_))
                    | Some(AssistantEntry::User(_))
                    | Some(AssistantEntry::Tool { .. })
                    | Some(AssistantEntry::Info(_))
                    | Some(AssistantEntry::Error(_))
                    | None => false,
                };
                if !appended {
                    self.push_entry(AssistantEntry::Thought(chunk));
                }
            }
            AgentEvent::ToolCallRequested {
                call_id,
                tool_name,
                arguments,
            } => {
                self.push_entry(AssistantEntry::Tool {
                    call_id,
                    tool_name,
                    arguments: arguments.to_string(),
                    status: ToolStatus::Running,
                    result: None,
                });
            }
            AgentEvent::ToolCallResult {
                call_id,
                tool_name,
                result,
                is_error,
            } => {
                // 进程内执行的提供方由编辑器回填真实结果，忽略其自带的占位结果。
                if !self.provider_executes_in_process() {
                    self.complete_tool_call(&call_id, &tool_name, result.to_string(), is_error);
                }
            }
            AgentEvent::ApprovalRequested {
                request_id,
                title,
                description,
                tool_name,
                command_summary,
                parameters: _,
            } => {
                // In-process providers have no permission backend of their own, so the
                // editor applies the selected mode here instead of showing a card.
                let auto_decision = if self.provider_executes_in_process() {
                    match self.permission_mode {
                        PermissionMode::Supervised => None,
                        PermissionMode::ReadOnly => Some(false),
                        PermissionMode::FullAccess => Some(true),
                        PermissionMode::AutoAcceptEdits => {
                            Some(in_process_tool_is_edit(&tool_name))
                        }
                    }
                } else {
                    None
                };
                match auto_decision {
                    Some(approved) => {
                        self.push_entry(AssistantEntry::Info(format!(
                            "{}: {title} ({tool_name})",
                            if approved {
                                "Auto-approved"
                            } else {
                                "Auto-rejected"
                            }
                        )));
                        if let Some(session) = self.session {
                            let runtime = self.runtime.clone();
                            gpui_tokio::Tokio::spawn(cx, async move {
                                if let Err(error) =
                                    runtime.submit_approval(session, request_id, approved).await
                                {
                                    log::error!(
                                        "Could not submit the automatic approval decision: {error:?}"
                                    );
                                }
                            })
                            .detach();
                        }
                    }
                    None => {
                        self.push_entry(AssistantEntry::Info(format!(
                            "Approval requested for {tool_name}"
                        )));
                        self.pending_approval = Some(AssistantApproval {
                            request_id,
                            title,
                            description,
                            tool_name,
                            command_summary,
                        });
                    }
                }
            }
            AgentEvent::ApprovalDecision {
                request_id,
                approved,
            } => {
                if self
                    .pending_approval
                    .as_ref()
                    .is_some_and(|approval| approval.request_id == request_id)
                {
                    self.pending_approval = None;
                }
                self.push_entry(AssistantEntry::Info(if approved {
                    "The agent received the approval.".to_string()
                } else {
                    "The agent received the rejection.".to_string()
                }));
            }
            AgentEvent::SessionCompleted { summary } => {
                self.busy = false;
                self.finish_turn_timing();
                if let Some(summary) = summary {
                    self.push_entry(AssistantEntry::Info(summary));
                }
            }
            AgentEvent::SessionError { code, message } => {
                self.busy = false;
                self.finish_turn_timing();
                self.push_entry(AssistantEntry::Error(format!("{code}: {message}")));
            }
        }
        self.scroll.scroll_to_bottom();
        cx.notify();
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) enum AssistantEntry {
    User(String),
    Message(String),
    Thought(String),
    /// One MCP tool call, from request through result. `call_id` ties the streamed
    /// result back to this card so it updates in place instead of appending a second block.
    Tool {
        call_id: String,
        tool_name: String,
        arguments: String,
        status: ToolStatus,
        result: Option<String>,
    },
    Info(String),
    Error(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum ToolStatus {
    Running,
    Done,
    Failed,
}

struct AssistantApproval {
    request_id: String,
    title: String,
    description: String,
    tool_name: String,
    command_summary: String,
}

/// Result of an off-thread MCP tool call, applied back on the UI thread.
struct AssistantToolOutcome {
    result: Result<Value, McpError>,
    /// Present only when a timeline-mutating tool succeeded; applied as one undoable edit.
    mutated_timeline: Option<TimelineEditingState>,
    /// Present only when a composition was created or edited on disk; refreshes the Media Pool.
    reloaded_compositions: Option<Vec<CompositionAsset>>,
    /// Present only when an explorer write tool validated a media-library change; the editor
    /// applies it through its own import pipeline and library persistence.
    explorer_change: Option<ExplorerChange>,
}

/// Lifecycle of one provider's model catalog probe.
#[derive(Clone)]
enum ModelCatalog {
    NotProbed,
    Loading,
    Loaded(Vec<AgentModel>),
    Failed(String),
}

#[derive(Clone, Copy)]
struct AssistantProvider {
    id: &'static str,
    display_name: &'static str,
    /// How the adapter talks to the agent: a CLI, an app-server, ACP, or in-process.
    transport: &'static str,
    /// Brand dot hue; identifies the agent without tinting the chrome.
    hue: u32,
    /// True when the editor executes the provider's tool calls in-process (offline Mock).
    /// Self-executing providers (ACP/Codex/Claude) run their own tools and only stream results.
    executes_in_process: bool,
    /// True when the provider has a real permission backend the mode picker can drive.
    supports_permissions: bool,
}

const ASSISTANT_PROVIDERS: [AssistantProvider; 6] = [
    AssistantProvider {
        id: "mock",
        display_name: "Mock Assistant (Offline)",
        transport: "Offline",
        hue: 0x8a90a2,
        executes_in_process: true,
        supports_permissions: true,
    },
    AssistantProvider {
        id: "acp-antigravity",
        display_name: "Antigravity (ACP)",
        transport: "ACP",
        hue: 0xa5b4fc,
        executes_in_process: false,
        supports_permissions: false,
    },
    AssistantProvider {
        id: "acp-gemini",
        display_name: "Gemini CLI (ACP)",
        transport: "ACP",
        hue: 0x4285f4,
        executes_in_process: false,
        supports_permissions: false,
    },
    AssistantProvider {
        id: "codex-app-server",
        display_name: "Codex (App-Server)",
        transport: "App-Server",
        hue: 0x10a37f,
        executes_in_process: false,
        supports_permissions: true,
    },
    AssistantProvider {
        id: "claude-cli",
        display_name: "Claude Code (CLI)",
        transport: "CLI",
        hue: 0xd97757,
        executes_in_process: false,
        supports_permissions: false,
    },
    AssistantProvider {
        id: "opencode",
        display_name: "OpenCode",
        transport: "HTTP",
        hue: 0x22d3ee,
        executes_in_process: false,
        supports_permissions: true,
    },
];

fn start_assistant_event_pump(
    receiver: tokio::sync::broadcast::Receiver<AgentEvent>,
    generation: u64,
    editor: WeakEntity<Editor>,
    cx: &mut AsyncApp,
) {
    let (sender, mut events) = mpsc::unbounded::<AgentEvent>();
    gpui_tokio::Tokio::spawn(cx, async move {
        let mut receiver = receiver;
        while let Ok(event) = receiver.recv().await {
            if sender.unbounded_send(event).is_err() {
                break;
            }
        }
    })
    .detach();

    cx.spawn(async move |cx| {
        while let Some(event) = events.next().await {
            let turn_finished = matches!(
                event,
                AgentEvent::SessionCompleted { .. } | AgentEvent::SessionError { .. }
            );
            let applied = editor.update(cx, |editor, cx| {
                if editor.assistant.session_generation != generation {
                    return false;
                }
                match event {
                    AgentEvent::ToolCallRequested {
                        call_id,
                        tool_name,
                        arguments,
                    } if editor.assistant.provider_executes_in_process() => {
                        editor.begin_assistant_tool_call(call_id, tool_name, arguments, cx);
                    }
                    event => editor.assistant.apply_agent_event(event, cx),
                }
                if turn_finished {
                    editor.persist_assistant_transcript(cx);
                }
                true
            });
            match applied {
                Ok(true) => {}
                Ok(false) | Err(_) => break,
            }
        }
    })
    .detach();
}

fn assistant_failed(editor: &WeakEntity<Editor>, cx: &mut AsyncApp, message: String) {
    let _ = editor.update(cx, |editor, cx| {
        editor.assistant.busy = false;
        editor.assistant.push_entry(AssistantEntry::Error(message));
        cx.notify();
    });
}

/// Executes one MCP tool against a timeline snapshot on the background executor, so disk and
/// rasterization work (create/edit/render composition) never blocks the UI thread. The caller
/// applies the returned mutation and refreshed compositions back on the UI thread.
fn run_assistant_tool_background(
    tool_name: String,
    arguments: &Value,
    seed: Option<(TimelineEditingState, HashSet<Ulid>, u64, Vec<Asset>)>,
    project_root: &Path,
) -> AssistantToolOutcome {
    let Some((editing_state, selected_clips, revision, media_library)) = seed else {
        return AssistantToolOutcome {
            result: Err(McpError::Unsupported {
                tool: tool_name,
                reason: "no timeline is open".to_string(),
            }),
            mutated_timeline: None,
            reloaded_compositions: None,
            explorer_change: None,
        };
    };

    let mut executor = McpExecutor::new(editing_state)
        .with_project_root(project_root.to_path_buf())
        .with_selection(selected_clips)
        .with_revision(revision)
        .with_media_library(media_library);
    let executed = executor.execute(&tool_name, arguments, true);
    let explorer_change = executor.take_explorer_change();
    let analysis_request = executor.take_analysis_request();

    // 分析工具的真实音频解码在后台线程完成，结果并入网关临时响应；失败则改写为真实错误。
    let result = match (executed, analysis_request) {
        (Ok(summary), Some(request)) => {
            crate::media_analysis::analyze_media_asset(&request.asset, summary)
        }
        (other, _) => other,
    };

    // 仅当时间线真正提交（revision 前进）才回写；合成磁盘工具不改时间线，避免空撤销与 revision 漂移。
    let mutated_timeline = if result.is_ok() && executor.revision() > revision {
        Some(executor.into_timeline())
    } else {
        None
    };

    let reloaded_compositions = if result.is_ok()
        && matches!(
            tool_name.as_str(),
            "create_composition" | "edit_composition"
        ) {
        match load_compositions(project_root) {
            Ok(compositions) => Some(compositions),
            Err(error) => {
                log::error!("Could not reload compositions after an AI edit: {error:?}");
                None
            }
        }
    } else {
        None
    };

    AssistantToolOutcome {
        result,
        mutated_timeline,
        reloaded_compositions,
        explorer_change,
    }
}

/// Renders a single non-activity transcript entry. Tool and thought runs are grouped by
/// [`assistant_activity_group_view`] instead, so they never reach this function.
fn assistant_entry_view(entry: &AssistantEntry) -> gpui::AnyElement {
    match entry {
        AssistantEntry::User(message) => div()
            .flex()
            .justify_end()
            .child(
                div()
                    .min_w_0()
                    .max_w(px(340.0))
                    .rounded(px(8.0))
                    .rounded_br(px(2.0))
                    .bg(rgb(ACCENT_BACKGROUND))
                    .px(px(10.0))
                    .py(px(8.0))
                    .text_size(px(11.0))
                    .line_height(px(20.0))
                    .text_color(rgb(TEXT))
                    .child(message.clone()),
            )
            .into_any_element(),
        AssistantEntry::Message(message) => div()
            .text_size(px(11.0))
            .line_height(px(20.0))
            .text_color(rgb(TEXT))
            .child(message.clone())
            .into_any_element(),
        AssistantEntry::Info(info) => div()
            .text_size(px(11.0))
            .line_height(px(16.0))
            .text_color(rgb(MUTED))
            .child(info.clone())
            .into_any_element(),
        AssistantEntry::Error(error) => div()
            .rounded(px(5.0))
            .border_1()
            .border_color(rgb(ERROR))
            .bg(rgb(SURFACE))
            .p(px(10.0))
            .flex()
            .items_center()
            .gap_2()
            .child(
                svg()
                    .path(IconName::Warning.path())
                    .size(px(13.0))
                    .text_color(rgb(ERROR)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(11.0))
                    .line_height(px(16.0))
                    .text_color(rgb(ERROR))
                    .child(error.clone()),
            )
            .into_any_element(),
        AssistantEntry::Thought(_) | AssistantEntry::Tool { .. } => div().into_any_element(),
    }
}

/// Assistant-generated content that belongs to one speaker turn. User and Info entries
/// break the turn, so the next assistant block gets a fresh speaker header.
fn assistant_entry_is_prose(entry: &AssistantEntry) -> bool {
    matches!(
        entry,
        AssistantEntry::Message(_)
            | AssistantEntry::Thought(_)
            | AssistantEntry::Tool { .. }
            | AssistantEntry::Error(_)
    )
}

fn assistant_speaker_header_view(
    provider: &AssistantProvider,
    turn_label: Option<&str>,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(assistant_dot(provider.hue, 6.0))
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(MUTED))
                .child(provider.display_name),
        )
        .when_some(turn_label, |this, label| {
            this.child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(MUTED))
                    .child(format!("· {label}")),
            )
        })
}

/// Tool and thought entries are grouped into a collapsible activity block; everything
/// else renders as a standalone transcript entry.
fn assistant_entry_is_activity(entry: &AssistantEntry) -> bool {
    matches!(
        entry,
        AssistantEntry::Tool { .. } | AssistantEntry::Thought(_)
    )
}

/// A group's collapsed state: an explicit user override wins; otherwise a live run stays
/// open and a finished run auto-collapses so completed turns stop taking up space.
fn assistant_group_collapsed(
    entries: &[AssistantEntry],
    start: usize,
    is_live: bool,
    overrides: &HashMap<usize, bool>,
) -> bool {
    if let Some(expanded) = overrides.get(&start) {
        return !expanded;
    }
    let running = entries.iter().any(|entry| {
        matches!(
            entry,
            AssistantEntry::Tool {
                status: ToolStatus::Running,
                ..
            }
        )
    });
    !(running || is_live)
}

/// A run of consecutive tool/thought entries under one "Ran N tools" / "Thinking" header.
/// The header collapses the whole run; each row inside expands independently.
fn assistant_activity_group_view(
    assistant: &AssistantState,
    entries: &[AssistantEntry],
    start: usize,
    collapsed: bool,
    cx: &mut Context<Editor>,
) -> gpui::AnyElement {
    let tool_count = entries
        .iter()
        .filter(|entry| matches!(entry, AssistantEntry::Tool { .. }))
        .count();
    let label = match tool_count {
        0 => "Thinking".to_string(),
        1 => "Ran 1 tool".to_string(),
        count => format!("Ran {count} tools"),
    };
    let toggle_index = start;
    // The override stores the expanded flag, so the next expanded state equals the current collapsed state.
    let next_expanded = collapsed;
    div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .id(("assistant-group", start))
                .flex()
                .items_center()
                .gap_1()
                .px(px(4.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .cursor(CursorStyle::PointingHand)
                .hover(|style| style.bg(rgb(SURFACE)))
                .on_click(cx.listener(move |editor, _, _, cx| {
                    editor
                        .assistant
                        .group_overrides
                        .insert(toggle_index, next_expanded);
                    cx.notify();
                }))
                .child(
                    svg()
                        .path(if collapsed {
                            IconName::ChevronRight.path()
                        } else {
                            IconName::ChevronDown.path()
                        })
                        .size(px(12.0))
                        .text_color(rgb(MUTED)),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(MUTED))
                        .child(label),
                ),
        )
        .when(!collapsed, |this| {
            this.child(div().flex().flex_col().gap(px(1.0)).pl(px(18.0)).children(
                entries.iter().enumerate().map(|(offset, entry)| {
                    let index = start + offset;
                    match entry {
                        AssistantEntry::Thought(text) => {
                            let expanded = assistant.expanded_thoughts.contains(&index);
                            assistant_thought_row_view(text, index, expanded, cx)
                        }
                        AssistantEntry::Tool {
                            call_id,
                            tool_name,
                            arguments,
                            status,
                            result,
                        } => {
                            let expanded = assistant.expanded_tools.contains(call_id);
                            assistant_tool_row_view(
                                call_id,
                                tool_name,
                                arguments,
                                *status,
                                result.as_deref(),
                                expanded,
                                cx,
                            )
                        }
                        AssistantEntry::User(_)
                        | AssistantEntry::Message(_)
                        | AssistantEntry::Info(_)
                        | AssistantEntry::Error(_) => div().into_any_element(),
                    }
                }),
            ))
        })
        .into_any_element()
}

/// One tool row: status icon, human label, a summary chip and a chevron. Clicking expands
/// the real arguments and result in place — never a fabricated preview.
fn assistant_tool_row_view(
    call_id: &str,
    tool_name: &str,
    arguments: &str,
    status: ToolStatus,
    result: Option<&str>,
    expanded: bool,
    cx: &mut Context<Editor>,
) -> gpui::AnyElement {
    let (icon, icon_color) = assistant_tool_status_icon(status);
    let (label, summary) = assistant_tool_label_and_summary(tool_name, arguments);
    let result_color = if status == ToolStatus::Failed {
        ERROR
    } else {
        MUTED
    };
    let toggle_id = call_id.to_string();
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .id(call_id.to_string())
                .flex()
                .items_center()
                .gap_2()
                .px(px(4.0))
                .py(px(3.0))
                .rounded(px(4.0))
                .cursor(CursorStyle::PointingHand)
                .hover(|style| style.bg(rgb(SURFACE)))
                .on_click(cx.listener(move |editor, _, _, cx| {
                    if editor.assistant.expanded_tools.contains(&toggle_id) {
                        editor.assistant.expanded_tools.remove(&toggle_id);
                    } else {
                        editor.assistant.expanded_tools.insert(toggle_id.clone());
                    }
                    cx.notify();
                }))
                .child(
                    svg()
                        .path(icon.path())
                        .size(px(13.0))
                        .text_color(rgb(icon_color)),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_size(px(11.0))
                                .text_color(rgb(TEXT))
                                .child(label),
                        )
                        .when(!summary.is_empty(), |this| {
                            this.child(
                                div()
                                    .min_w_0()
                                    .rounded(px(4.0))
                                    .bg(rgb(SURFACE))
                                    .px(px(6.0))
                                    .py(px(1.0))
                                    .font_family("monospace")
                                    .text_size(px(10.0))
                                    .text_color(rgb(MUTED))
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .child(summary),
                            )
                        }),
                )
                .child(
                    svg()
                        .path(if expanded {
                            IconName::ChevronDown.path()
                        } else {
                            IconName::ChevronRight.path()
                        })
                        .size(px(11.0))
                        .text_color(rgb(MUTED)),
                ),
        )
        .when(expanded, |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .pl(px(21.0))
                    .py(px(6.0))
                    .when(!arguments.is_empty(), |this| {
                        this.child(
                            div()
                                .font_family("monospace")
                                .text_size(px(9.0))
                                .line_height(px(15.0))
                                .text_color(rgb(MUTED))
                                .child(arguments.to_string()),
                        )
                    })
                    .when_some(result, |this, result| {
                        this.child(
                            div()
                                .text_size(px(10.0))
                                .line_height(px(16.0))
                                .text_color(rgb(result_color))
                                .child(result.to_string()),
                        )
                    }),
            )
        })
        .into_any_element()
}

/// One thought row: a single truncated line when collapsed, the full text when expanded.
fn assistant_thought_row_view(
    text: &str,
    index: usize,
    expanded: bool,
    cx: &mut Context<Editor>,
) -> gpui::AnyElement {
    let body = if expanded {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(MUTED))
                    .child("Thought"),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .line_height(px(16.0))
                    .text_color(rgb(MUTED))
                    .child(text.to_string()),
            )
            .into_any_element()
    } else {
        let first_line = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim();
        div()
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(10.0))
            .text_color(rgb(MUTED))
            .child(div().flex_shrink_0().child("Thought ·"))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(first_line.to_string()),
            )
            .into_any_element()
    };
    div()
        .id(("assistant-thought", index))
        .px(px(4.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE)))
        .on_click(cx.listener(move |editor, _, _, cx| {
            if editor.assistant.expanded_thoughts.contains(&index) {
                editor.assistant.expanded_thoughts.remove(&index);
            } else {
                editor.assistant.expanded_thoughts.insert(index);
            }
            cx.notify();
        }))
        .child(body)
        .into_any_element()
}

/// Maps a raw tool name to a human label and pulls a short summary chip from its arguments.
fn assistant_tool_label_and_summary(tool_name: &str, arguments: &str) -> (String, String) {
    let lower = tool_name.to_lowercase();
    let label = if lower.contains("edit")
        || lower.contains("write")
        || lower.contains("patch")
        || lower.contains("replace")
        || lower.contains("apply")
    {
        "Edit file"
    } else if lower.contains("delete") || lower.contains("remove") {
        "Delete"
    } else if lower.contains("read") || lower.contains("view") {
        "Read"
    } else if lower.contains("shell")
        || lower.contains("bash")
        || lower.contains("terminal")
        || lower.contains("exec")
        || lower.contains("command")
    {
        "Terminal command ran"
    } else if lower.contains("web")
        || lower.contains("fetch")
        || lower.contains("browser")
        || lower.contains("http")
        || lower.contains("script")
    {
        "Run webscript"
    } else {
        tool_name
    };
    (label.to_string(), assistant_tool_summary(arguments))
}

/// A filename basename or command/query/url drawn from the tool's real arguments.
fn assistant_tool_summary(arguments: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(arguments) else {
        return String::new();
    };
    let Some(object) = value.as_object() else {
        return String::new();
    };
    for key in ["path", "file_path", "filename", "file"] {
        if let Some(path) = object.get(key).and_then(Value::as_str) {
            let basename = Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path);
            return basename.to_string();
        }
    }
    for key in ["command", "cmd", "query", "url"] {
        if let Some(text) = object.get(key).and_then(Value::as_str) {
            return text.to_string();
        }
    }
    String::new()
}

fn format_turn_elapsed(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    if seconds == 0 {
        "worked for <1s".to_string()
    } else if seconds < 60 {
        format!("worked for {seconds}s")
    } else {
        format!("worked for {}m {}s", seconds / 60, seconds % 60)
    }
}

fn assistant_tool_status_icon(status: ToolStatus) -> (IconName, u32) {
    match status {
        ToolStatus::Running => (IconName::Spinner, WARNING),
        ToolStatus::Done => (IconName::CheckCircle, SUCCESS),
        ToolStatus::Failed => (IconName::Warning, ERROR),
    }
}

/// Rekeys an index-keyed expansion set after the transcript drops its first entry:
/// every index moves down one, and an index of 0 referred to the removed entry.
fn shift_expansion_indices(indices: &HashSet<usize>) -> HashSet<usize> {
    indices
        .iter()
        .filter_map(|index| index.checked_sub(1))
        .collect()
}

/// Same front-truncation rekey for the group override map, preserving each override's value.
fn shift_group_overrides(overrides: &HashMap<usize, bool>) -> HashMap<usize, bool> {
    overrides
        .iter()
        .filter_map(|(index, expanded)| {
            index.checked_sub(1).map(|new_index| (new_index, *expanded))
        })
        .collect()
}

fn assistant_approval_card_view(
    approval: &AssistantApproval,
    expanded: bool,
    window: &mut Window,
    cx: &mut Context<Editor>,
) -> gpui::AnyElement {
    let toggle_id = approval.request_id.clone();
    div()
        .flex_shrink_0()
        .mx_3()
        .mb_3()
        .flex()
        .rounded(px(5.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(BACKGROUND))
        .overflow_hidden()
        .child(div().w(px(2.0)).flex_shrink_0().bg(rgb(ACCENT)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .p(px(10.0))
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .id(approval.request_id.clone())
                        .flex()
                        .items_center()
                        .gap_2()
                        .cursor(CursorStyle::PointingHand)
                        .on_click(cx.listener(move |editor, _, _, cx| {
                            if editor.assistant.expanded_approvals.contains(&toggle_id) {
                                editor.assistant.expanded_approvals.remove(&toggle_id);
                            } else {
                                editor
                                    .assistant
                                    .expanded_approvals
                                    .insert(toggle_id.clone());
                            }
                            cx.notify();
                        }))
                        .child(
                            svg()
                                .path(IconName::Shield.path())
                                .size(px(13.0))
                                .text_color(rgb(ACCENT)),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(TEXT))
                                .text_ellipsis()
                                .child(approval.title.clone()),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .font_family("monospace")
                                .text_size(px(9.0))
                                .text_color(rgb(MUTED))
                                .child(approval.tool_name.clone()),
                        )
                        .child(
                            svg()
                                .path(if expanded {
                                    IconName::ChevronDown.path()
                                } else {
                                    IconName::ChevronRight.path()
                                })
                                .size(px(11.0))
                                .text_color(rgb(MUTED)),
                        ),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .line_height(px(16.0))
                        .text_color(rgb(MUTED))
                        .when(!expanded, |this| this.whitespace_nowrap().text_ellipsis())
                        .child(approval.description.clone()),
                )
                .when(expanded && !approval.command_summary.is_empty(), |this| {
                    this.child(
                        div()
                            .rounded(px(4.0))
                            .bg(rgb(SURFACE))
                            .px(px(8.0))
                            .py(px(6.0))
                            .font_family("monospace")
                            .text_size(px(10.0))
                            .line_height(px(16.0))
                            .text_color(rgb(LINK))
                            .child(approval.command_summary.clone()),
                    )
                })
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button_cyberpunk_primary(
                                "assistant-approve",
                                "Approve",
                                true,
                                window,
                                cx,
                            )
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.emit_event(
                                    cx,
                                    AppEvent::AssistantApprovalDecided { approved: true },
                                );
                            })),
                        )
                        .child(
                            button_cyberpunk_secondary(
                                "assistant-reject",
                                "Reject",
                                true,
                                window,
                                cx,
                            )
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.emit_event(
                                    cx,
                                    AppEvent::AssistantApprovalDecided { approved: false },
                                );
                            })),
                        ),
                ),
        )
        .into_any_element()
}

/// The header's model/provider chooser: brand dot, name, model, chevron. Opening it probes
/// real readiness. One AgentProvider abstraction backs every entry, with the same tools.
/// The composer's model picker: provider dot, name, chevron. Opens the same provider
/// dropdown the old dock selector used, with real readiness per row.
fn assistant_model_button_view(
    assistant: &AssistantState,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    let provider = assistant.provider();
    div()
        .id("assistant-model-button")
        .h(px(28.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .px(px(8.0))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(|editor, _, _, cx| {
            let opening = !editor.assistant.provider_menu_open;
            editor.assistant.provider_menu_open = opening;
            editor.assistant.model_menu_open = false;
            editor.assistant.effort_menu_open = false;
            editor.assistant.skill_menu_open = false;
            editor.assistant.permission_menu_open = false;
            if opening {
                editor.probe_assistant_providers(cx);
            }
            cx.notify();
        }))
        .child(assistant_dot(provider.hue, 7.0))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT))
                .child(provider.display_name),
        )
        .child(
            svg()
                .path(IconName::ArrowDown.path())
                .size(px(11.0))
                .text_color(rgb(MUTED)),
        )
}

/// The composer's model picker: the model that will actually run, chevron. Opening it
/// queries the active provider's real catalog; the effort picker appears once a model
/// with effort variants is active.
fn assistant_model_picker_view(
    assistant: &AssistantState,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    let label = assistant.model_label();
    div()
        .id("assistant-model-picker")
        .h(px(28.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .px(px(8.0))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(|editor, _, _, cx| {
            let opening = !editor.assistant.model_menu_open;
            editor.assistant.model_menu_open = opening;
            editor.assistant.provider_menu_open = false;
            editor.assistant.effort_menu_open = false;
            editor.assistant.skill_menu_open = false;
            editor.assistant.permission_menu_open = false;
            if opening {
                editor.probe_assistant_models(cx);
            }
            cx.notify();
        }))
        .child(div().text_size(px(11.0)).text_color(rgb(TEXT)).child(label))
        .child(
            svg()
                .path(IconName::ArrowDown.path())
                .size(px(11.0))
                .text_color(rgb(MUTED)),
        )
}

/// The composer's reasoning-effort picker, shown only when the active model offers
/// effort variants. The label is the effort that will actually run.
fn assistant_effort_picker_view(
    assistant: &AssistantState,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    let label = assistant.effort_label();
    div()
        .id("assistant-effort-picker")
        .h(px(28.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .px(px(8.0))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(|editor, _, _, cx| {
            let opening = !editor.assistant.effort_menu_open;
            editor.assistant.effort_menu_open = opening;
            editor.assistant.provider_menu_open = false;
            editor.assistant.model_menu_open = false;
            editor.assistant.skill_menu_open = false;
            editor.assistant.permission_menu_open = false;
            cx.notify();
        }))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT))
                .child(format!("Effort: {label}")),
        )
        .child(
            svg()
                .path(IconName::ArrowDown.path())
                .size(px(11.0))
                .text_color(rgb(MUTED)),
        )
}

/// The model dropdown: the provider default plus every model the provider actually
/// reported, with its description. A failed probe shows the real error and retries.
fn assistant_model_menu_view(assistant: &AssistantState, cx: &mut Context<Editor>) -> gpui::Div {
    let choice = assistant.model_choices[assistant.provider_index].clone();
    let default_subtitle = match assistant.catalog() {
        ModelCatalog::Loaded(models) => {
            let default_model = match models.iter().find(|model| model.is_default) {
                Some(model) => Some(model),
                None => models.first(),
            };
            match default_model {
                Some(model) => format!("Use {}", model.display_name),
                None => "The provider chooses the model".to_string(),
            }
        }
        ModelCatalog::Loading => "Loading the provider's catalog…".to_string(),
        ModelCatalog::Failed(_) => "The provider chooses the model".to_string(),
        ModelCatalog::NotProbed => "The provider chooses the model".to_string(),
    };
    let mut rows = vec![
        assistant_model_row_view(
            "assistant-model-default".to_string(),
            None,
            "Provider default".to_string(),
            default_subtitle,
            choice.is_none(),
            cx,
        )
        .into_any_element(),
    ];
    match assistant.catalog() {
        ModelCatalog::Loading => {
            rows.push(assistant_menu_note_view("Loading models…", MUTED).into_any_element());
        }
        ModelCatalog::Failed(message) => {
            rows.push(
                div()
                    .id("assistant-model-retry")
                    .rounded(px(6.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                    .on_click(cx.listener(|editor, _, _, cx| {
                        editor.emit_event(cx, AppEvent::AssistantModelsRefresh);
                    }))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(ERROR))
                            .child(message.clone()),
                    )
                    .child(
                        div()
                            .mt(px(2.0))
                            .text_size(px(10.0))
                            .text_color(rgb(MUTED))
                            .child("Click to retry"),
                    )
                    .into_any_element(),
            );
        }
        ModelCatalog::Loaded(models) => {
            if models.is_empty() {
                rows.push(
                    assistant_menu_note_view("This provider reported no models.", MUTED)
                        .into_any_element(),
                );
            }
            for model in models {
                let subtitle = match &model.description {
                    Some(description) => description.clone(),
                    None => {
                        if model.efforts.is_empty() {
                            model.id.clone()
                        } else {
                            format!("Efforts: {}", model.efforts.join(" · "))
                        }
                    }
                };
                rows.push(
                    assistant_model_row_view(
                        format!("assistant-model-{}", model.id),
                        Some(model.id.clone()),
                        model.display_name.clone(),
                        subtitle,
                        choice.as_deref() == Some(model.id.as_str()),
                        cx,
                    )
                    .into_any_element(),
                );
            }
        }
        ModelCatalog::NotProbed => {}
    }

    let menu = div()
        .absolute()
        .bottom(px(96.0))
        .left(px(if assistant.sessions_sidebar_open {
            252.0
        } else {
            12.0
        }))
        .right(px(12.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .p(px(3.0))
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(12.0))
                .pt(px(8.0))
                .pb(px(4.0))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(MUTED))
                .child("MODEL"),
        )
        .child(
            div()
                .id("assistant-model-menu-rows")
                .max_h(px(320.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .children(rows),
        );

    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .id("assistant-model-backdrop")
                .absolute()
                .inset_0()
                .on_click(cx.listener(|editor, _, _, cx| {
                    editor.assistant.model_menu_open = false;
                    cx.notify();
                })),
        )
        .child(menu)
}

/// The reasoning-effort dropdown for the active model: the model default plus every
/// effort variant the model actually supports.
fn assistant_effort_menu_view(assistant: &AssistantState, cx: &mut Context<Editor>) -> gpui::Div {
    let choice = assistant.effort_choices[assistant.provider_index].clone();
    let default_subtitle = match assistant
        .active_model()
        .and_then(|model| model.default_effort.as_deref())
    {
        Some(effort) => format!("Use {}", effort_display_name(effort)),
        None => "The model chooses the effort".to_string(),
    };
    let mut rows = vec![
        assistant_effort_row_view(
            "assistant-effort-default".to_string(),
            None,
            "Model default".to_string(),
            default_subtitle,
            choice.is_none(),
            cx,
        )
        .into_any_element(),
    ];
    for effort in assistant.effort_options() {
        rows.push(
            assistant_effort_row_view(
                format!("assistant-effort-{effort}"),
                Some(effort.clone()),
                effort_display_name(effort),
                String::new(),
                choice.as_deref() == Some(effort.as_str()),
                cx,
            )
            .into_any_element(),
        );
    }

    let menu = div()
        .absolute()
        .bottom(px(96.0))
        .left(px(if assistant.sessions_sidebar_open {
            252.0
        } else {
            12.0
        }))
        .right(px(12.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .p(px(3.0))
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(12.0))
                .pt(px(8.0))
                .pb(px(4.0))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(MUTED))
                .child("REASONING EFFORT"),
        )
        .child(
            div()
                .id("assistant-effort-menu-rows")
                .max_h(px(320.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .children(rows),
        );

    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .id("assistant-effort-backdrop")
                .absolute()
                .inset_0()
                .on_click(cx.listener(|editor, _, _, cx| {
                    editor.assistant.effort_menu_open = false;
                    cx.notify();
                })),
        )
        .child(menu)
}

/// The permission chip: shows the active mode with its icon and opens the mode menu.
/// Only mounted for providers with a real permission backend.
fn assistant_permission_picker_view(
    assistant: &AssistantState,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    let mode = assistant.permission_mode;
    div()
        .id("assistant-permission-picker")
        .h(px(28.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .px(px(8.0))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(|editor, _, _, cx| {
            let opening = !editor.assistant.permission_menu_open;
            editor.assistant.permission_menu_open = opening;
            editor.assistant.provider_menu_open = false;
            editor.assistant.model_menu_open = false;
            editor.assistant.effort_menu_open = false;
            editor.assistant.skill_menu_open = false;
            cx.notify();
        }))
        .child(
            svg()
                .path(permission_mode_icon(mode).path())
                .size(px(13.0))
                .text_color(rgb(permission_mode_hue(mode))),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(TEXT))
                .child(permission_mode_label(mode)),
        )
        .child(
            svg()
                .path(IconName::ArrowDown.path())
                .size(px(11.0))
                .text_color(rgb(MUTED)),
        )
}

/// The permission dropdown: a click-away backdrop plus the four real modes. Choosing a
/// row emits one AssistantPermissionModeSelected and restarts the agent session.
fn assistant_permission_menu_view(
    assistant: &AssistantState,
    cx: &mut Context<Editor>,
) -> gpui::Div {
    let modes = [
        PermissionMode::ReadOnly,
        PermissionMode::Supervised,
        PermissionMode::AutoAcceptEdits,
        PermissionMode::FullAccess,
    ];
    let menu = div()
        .absolute()
        .bottom(px(96.0))
        .left(px(if assistant.sessions_sidebar_open {
            252.0
        } else {
            12.0
        }))
        .right(px(12.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .p(px(3.0))
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(12.0))
                .pt(px(8.0))
                .pb(px(4.0))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(MUTED))
                .child("PERMISSIONS"),
        )
        .child(
            div()
                .id("assistant-permission-menu-rows")
                .max_h(px(320.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .children(modes.into_iter().map(|mode| {
                    assistant_permission_row_view(mode, mode == assistant.permission_mode, cx)
                })),
        )
        .child(assistant_menu_note_view(
            "Changing mode starts a fresh agent session.",
            MUTED,
        ));

    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .id("assistant-permission-backdrop")
                .absolute()
                .inset_0()
                .on_click(cx.listener(|editor, _, _, cx| {
                    editor.assistant.permission_menu_open = false;
                    cx.notify();
                })),
        )
        .child(menu)
}

fn assistant_permission_row_view(
    mode: PermissionMode,
    selected: bool,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    div()
        .id(format!(
            "assistant-permission-{}",
            permission_mode_label(mode)
        ))
        .flex()
        .items_center()
        .gap(px(10.0))
        .rounded(px(6.0))
        .px(px(12.0))
        .py(px(8.0))
        .when(selected, |this| this.bg(rgb(SURFACE_HOVER)))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(move |editor, _, _, cx| {
            editor.emit_event(cx, AppEvent::AssistantPermissionModeSelected { mode });
        }))
        .child(
            svg()
                .path(permission_mode_icon(mode).path())
                .size(px(13.0))
                .flex_shrink_0()
                .text_color(rgb(permission_mode_hue(mode))),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT))
                        .child(permission_mode_label(mode)),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(10.0))
                        .text_color(rgb(MUTED))
                        .child(permission_mode_description(mode)),
                ),
        )
        .when(selected, |this| {
            this.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(13.0))
                    .text_color(rgb(LINK))
                    .child("✓"),
            )
        })
}

fn permission_mode_label(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::ReadOnly => "Read-only",
        PermissionMode::Supervised => "Supervised",
        PermissionMode::AutoAcceptEdits => "Auto-accept edits",
        PermissionMode::FullAccess => "Full access",
    }
}

fn permission_mode_description(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::ReadOnly => "Inspect without changing files.",
        PermissionMode::Supervised => "Ask before commands and file changes.",
        PermissionMode::AutoAcceptEdits => "Auto-approve edits, ask before other actions.",
        PermissionMode::FullAccess => "Allow commands and edits without prompts.",
    }
}

fn permission_mode_icon(mode: PermissionMode) -> IconName {
    match mode {
        PermissionMode::ReadOnly => IconName::Eye,
        PermissionMode::Supervised => IconName::Lock,
        PermissionMode::AutoAcceptEdits => IconName::Edit,
        PermissionMode::FullAccess => IconName::Shield,
    }
}

fn permission_mode_hue(mode: PermissionMode) -> u32 {
    match mode {
        PermissionMode::ReadOnly => MUTED,
        PermissionMode::Supervised => MUTED,
        PermissionMode::AutoAcceptEdits => LINK,
        PermissionMode::FullAccess => WARNING,
    }
}

/// The approval-gated MCP tools that mutate the edit. Auto-accept edits approves these
/// without a card; export and other non-edit actions still ask.
fn in_process_tool_is_edit(tool_name: &str) -> bool {
    match tool_name {
        "split_clip" | "delete_clip" | "propose_changeset" | "add_composition_to_timeline" => true,
        "export_video" => false,
        _ => false,
    }
}

fn assistant_model_row_view(
    id: String,
    model: Option<String>,
    title: String,
    subtitle: String,
    selected: bool,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(10.0))
        .rounded(px(6.0))
        .px(px(12.0))
        .py(px(8.0))
        .when(selected, |this| this.bg(rgb(SURFACE_HOVER)))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(move |editor, _, _, cx| {
            editor.emit_event(
                cx,
                AppEvent::AssistantModelSelected {
                    model: model.clone(),
                },
            );
        }))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(TEXT))
                        .text_ellipsis()
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(10.0))
                        .text_color(rgb(MUTED))
                        .text_ellipsis()
                        .child(subtitle),
                ),
        )
        .when(selected, |this| {
            this.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(13.0))
                    .text_color(rgb(LINK))
                    .child("✓"),
            )
        })
}

fn assistant_effort_row_view(
    id: String,
    effort: Option<String>,
    title: String,
    subtitle: String,
    selected: bool,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(10.0))
        .rounded(px(6.0))
        .px(px(12.0))
        .py(px(8.0))
        .when(selected, |this| this.bg(rgb(SURFACE_HOVER)))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(move |editor, _, _, cx| {
            editor.emit_event(
                cx,
                AppEvent::AssistantEffortSelected {
                    effort: effort.clone(),
                },
            );
        }))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(TEXT))
                        .text_ellipsis()
                        .child(title),
                )
                .when(!subtitle.is_empty(), |this| {
                    this.child(
                        div()
                            .mt(px(2.0))
                            .text_size(px(10.0))
                            .text_color(rgb(MUTED))
                            .text_ellipsis()
                            .child(subtitle),
                    )
                }),
        )
        .when(selected, |this| {
            this.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(13.0))
                    .text_color(rgb(LINK))
                    .child("✓"),
            )
        })
}

fn assistant_menu_note_view(text: &str, color: u32) -> gpui::Div {
    div()
        .px(px(12.0))
        .py(px(8.0))
        .text_size(px(11.0))
        .text_color(rgb(color))
        .child(text.to_string())
}

fn effort_display_name(effort: &str) -> String {
    let mut chars = effort.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The window's left rail: the live thread on top, then this project's saved sessions
/// grouped by local day. A saved row replays its transcript read-only; the live row
/// returns to the running conversation.
fn assistant_sessions_sidebar_view(
    assistant: &AssistantState,
    cx: &mut Context<Editor>,
) -> gpui::Div {
    let provider = assistant.provider();
    let viewing_id = assistant.viewing.as_ref().map(|session| session.id);
    let mut rows: Vec<gpui::AnyElement> = Vec::new();
    if !assistant.entries.is_empty() {
        rows.push(
            assistant_session_row_view(
                "assistant-live-session".to_string(),
                None,
                provider.hue,
                &session_title(&assistant.entries),
                format!(
                    "{} · {} messages",
                    provider.display_name,
                    assistant.entries.len()
                ),
                viewing_id.is_none(),
                cx,
            )
            .into_any_element(),
        );
    }
    let mut last_day: Option<String> = None;
    for meta in &assistant.history {
        if assistant.transcript_id == Some(meta.id) {
            continue; // 上面的 live 行已代表正在进行的转录，避免重复。
        }
        let day = session_day_label(meta.updated_at_ms);
        if last_day.as_deref() != Some(day.as_str()) {
            rows.push(
                div()
                    .px(px(10.0))
                    .pt(px(8.0))
                    .pb(px(2.0))
                    .text_size(px(9.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(MUTED))
                    .child(day.clone())
                    .into_any_element(),
            );
            last_day = Some(day);
        }
        let session_provider = assistant_provider_for_id(&meta.provider_id);
        rows.push(
            assistant_session_row_view(
                format!("assistant-session-{}", meta.id),
                Some(meta.id),
                session_provider.hue,
                &meta.title,
                format!(
                    "{} · {} messages",
                    session_provider.display_name, meta.message_count
                ),
                viewing_id == Some(meta.id),
                cx,
            )
            .into_any_element(),
        );
    }
    let empty = rows.is_empty();
    div()
        .w(px(240.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgb(BORDER))
        .bg(rgb(PANEL))
        .child(
            div()
                .h(px(40.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_between()
                .px(px(12.0))
                .border_b_1()
                .border_color(rgb(BORDER))
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child("Sessions"),
                )
                .child(assistant_new_chat_button_view("assistant-sidebar-new", cx)),
        )
        .child(
            div()
                .id("assistant-sessions")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .p(px(8.0))
                .flex()
                .flex_col()
                .gap_1()
                .when(empty, |this| {
                    this.child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_1()
                            .px(px(12.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(MUTED))
                                    .child("No sessions yet"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(rgb(MUTED))
                                    .child("Send a message to start one."),
                            ),
                    )
                })
                .children(rows),
        )
}

/// One sidebar row. `session_id` None is the live thread (click returns to it); Some is a
/// saved transcript (click replays it). The selected row is already on screen, so it is
/// not clickable.
fn assistant_session_row_view(
    id: String,
    session_id: Option<Ulid>,
    hue: u32,
    title: &str,
    subtitle: String,
    selected: bool,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(8.0))
        .rounded(px(6.0))
        .px(px(10.0))
        .py(px(8.0))
        .when(selected, |this| {
            this.bg(rgb(SURFACE))
                .border_1()
                .border_color(rgb(ACCENT))
        })
        .when(!selected, |this| {
            this.cursor(CursorStyle::PointingHand)
                .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                .on_click(cx.listener(move |editor, _, _, cx| match session_id {
                    Some(id) => {
                        editor.emit_event(cx, AppEvent::AssistantSessionSelected { id });
                    }
                    None => {
                        editor.emit_event(cx, AppEvent::AssistantLiveSessionResumed);
                    }
                }))
        })
        .child(assistant_dot(hue, 7.0))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT))
                        .text_ellipsis()
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(MUTED))
                        .text_ellipsis()
                        .child(subtitle),
                ),
        )
}

fn assistant_provider_for_id(id: &str) -> &'static AssistantProvider {
    match ASSISTANT_PROVIDERS.iter().find(|provider| provider.id == id) {
        Some(provider) => provider,
        None => &ASSISTANT_PROVIDERS[0],
    }
}

/// New chat: drops the live session and clears the thread. Shared by the sidebar
/// header and the tab bar so both start a fresh conversation the same way.
fn assistant_new_chat_button_view(
    id: &'static str,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(24.0))
        .w(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(|editor, _, _, cx| {
            editor.emit_event(cx, AppEvent::AssistantNewChat);
        }))
        .child(
            svg()
                .path(IconName::Plus.path())
                .size(px(12.0))
                .text_color(rgb(TEXT)),
        )
}

/// The provider dropdown: a click-away backdrop plus every registered agent with its
/// transport, model and real readiness. Choosing a row emits one AssistantProviderSelected.
fn assistant_provider_menu_view(
    assistant: &AssistantState,
    cx: &mut Context<Editor>,
) -> gpui::Div {
    let menu = div()
        .absolute()
        .bottom(px(96.0))
        .left(px(if assistant.sessions_sidebar_open {
            252.0
        } else {
            12.0
        }))
        .right(px(12.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .p(px(3.0))
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(12.0))
                .pt(px(8.0))
                .pb(px(4.0))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(MUTED))
                .child("PROVIDER"),
        )
        .children(
            ASSISTANT_PROVIDERS
                .iter()
                .enumerate()
                .map(|(index, provider)| {
                    assistant_provider_row_view(assistant, index, provider, cx)
                }),
        );

    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .id("assistant-provider-backdrop")
                .absolute()
                .inset_0()
                .on_click(cx.listener(|editor, _, _, cx| {
                    editor.assistant.provider_menu_open = false;
                    cx.notify();
                })),
        )
        .child(menu)
}

fn assistant_provider_row_view(
    assistant: &AssistantState,
    index: usize,
    provider: &AssistantProvider,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    let selected = index == assistant.provider_index;
    let readiness = assistant.provider_readiness[index];
    let unavailable = readiness == Some(false);
    let (readiness_label, readiness_color) = assistant_readiness_label(readiness);
    let name_color = if unavailable {
        MUTED
    } else {
        TEXT
    };
    div()
        .id(("assistant-provider", index))
        .flex()
        .items_center()
        .gap(px(10.0))
        .rounded(px(6.0))
        .px(px(12.0))
        .py(px(8.0))
        .when(selected, |this| this.bg(rgb(SURFACE_HOVER)))
        .when(!unavailable, |this| {
            this.cursor(CursorStyle::PointingHand)
                .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                .on_click(cx.listener(move |editor, _, _, cx| {
                    editor.emit_event(cx, AppEvent::AssistantProviderSelected { index });
                }))
        })
        .child(assistant_dot(provider.hue, 8.0))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .min_w_0()
                                .text_size(px(12.0))
                                .text_color(rgb(name_color))
                                .text_ellipsis()
                                .child(provider.display_name),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .rounded(px(3.0))
                                .bg(rgb(BACKGROUND))
                                .px(px(4.0))
                                .font_family("monospace")
                                .text_size(px(8.0))
                                .text_color(rgb(MUTED))
                                .child(provider.transport),
                        ),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(10.0))
                        .text_color(rgb(MUTED))
                        .text_ellipsis()
                        .child(assistant.provider_model_label(index)),
                ),
        )
        .child(
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(assistant_dot(readiness_color, 5.0))
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(rgb(readiness_color))
                        .child(readiness_label),
                ),
        )
        .when(selected, |this| {
            this.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(13.0))
                    .text_color(rgb(LINK))
                    .child("✓"),
            )
        })
}

fn assistant_dot(hue: u32, size: f32) -> gpui::Div {
    div()
        .flex_shrink_0()
        .w(px(size))
        .h(px(size))
        .rounded_full()
        .bg(rgb(hue))
}

fn assistant_readiness_label(readiness: Option<bool>) -> (&'static str, u32) {
    match readiness {
        Some(true) => ("Ready", SUCCESS),
        Some(false) => ("Unavailable", MUTED),
        None => ("Checking…", MUTED),
    }
}

/// The composer's skill picker: sparkle, the active skill's name (or "Skills"), chevron.
/// Tinted with the accent when a skill is active so the applied capability is visible.
fn assistant_skill_button_view(
    assistant: &AssistantState,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    let active = assistant.active_skill();
    let (label, background, border, foreground) = match active {
        Some(skill) => (skill.name.clone(), ACCENT_BACKGROUND, ACCENT, LINK),
        None => ("Skills".to_string(), SURFACE, BORDER, TEXT),
    };
    div()
        .id("assistant-skill-button")
        .h(px(28.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(8.0))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(|editor, _, _, cx| {
            let opening = !editor.assistant.skill_menu_open;
            editor.assistant.skill_menu_open = opening;
            editor.assistant.provider_menu_open = false;
            editor.assistant.model_menu_open = false;
            editor.assistant.effort_menu_open = false;
            editor.assistant.permission_menu_open = false;
            cx.notify();
        }))
        .child(
            svg()
                .path(IconName::Sparkle.path())
                .size(px(13.0))
                .text_color(rgb(foreground)),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(foreground))
                .child(label),
        )
        .child(
            svg()
                .path(IconName::ArrowDown.path())
                .size(px(11.0))
                .text_color(rgb(MUTED)),
        )
}

/// The skill dropdown: a click-away backdrop plus every discovered production skill with
/// its real description. Choosing a row emits one AssistantSkillSelected.
fn assistant_skill_menu_view(
    assistant: &AssistantState,
    cx: &mut Context<Editor>,
) -> gpui::Div {
    let mut skills = assistant.skills.list();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    let empty = skills.is_empty();
    let menu = div()
        .absolute()
        .bottom(px(96.0))
        .left(px(if assistant.sessions_sidebar_open {
            252.0
        } else {
            12.0
        }))
        .right(px(12.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .p(px(3.0))
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(12.0))
                .pt(px(8.0))
                .pb(px(4.0))
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(MUTED))
                .child("SKILLS"),
        )
        .when(empty, |this| {
            this.child(
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .text_size(px(11.0))
                    .text_color(rgb(MUTED))
                    .child("No skills found."),
            )
        })
        .children(
            skills
                .into_iter()
                .enumerate()
                .map(|(index, skill)| assistant_skill_row_view(assistant, index, skill, cx)),
        );

    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .id("assistant-skill-backdrop")
                .absolute()
                .inset_0()
                .on_click(cx.listener(|editor, _, _, cx| {
                    editor.assistant.skill_menu_open = false;
                    cx.notify();
                })),
        )
        .child(menu)
}

fn assistant_skill_row_view(
    assistant: &AssistantState,
    index: usize,
    skill: &SkillPackage,
    cx: &mut Context<Editor>,
) -> Stateful<gpui::Div> {
    let active = assistant.skills.is_enabled(&skill.id);
    let id = skill.id.clone();
    let icon_color = if active {
        LINK
    } else {
        MUTED
    };
    div()
        .id(("assistant-skill", index))
        .flex()
        .items_start()
        .gap(px(10.0))
        .rounded(px(6.0))
        .px(px(12.0))
        .py(px(8.0))
        .cursor(CursorStyle::PointingHand)
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .when(active, |this| this.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(move |editor, _, _, cx| {
            editor.emit_event(
                cx,
                AppEvent::AssistantSkillSelected { id: id.clone() },
            );
        }))
        .child(
            svg()
                .path(IconName::Sparkle.path())
                .size(px(14.0))
                .text_color(rgb(icon_color)),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(TEXT))
                        .child(skill.name.clone()),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(10.0))
                        .line_height(px(16.0))
                        .text_color(rgb(MUTED))
                        .child(skill.description.clone()),
                ),
        )
        .when(active, |this| {
            this.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(13.0))
                    .text_color(rgb(LINK))
                    .child("✓"),
            )
        })
}
