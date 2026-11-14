//! AI assistant dock: provider cycling, streamed conversation, and approval cards.

use crate::edit_action::EditAction;
use crate::editor::Editor;
use crate::event_bus::AppEvent;
use crate::theme::{ACCENT, BORDER, ERROR, LINK, MUTED, PANEL, SURFACE, SURFACE_HOVER, TEXT};
use crate::ui_components::TextInput;
use crate::ui_components::cyberpunk_button::{
    button_cyberpunk_primary, button_cyberpunk_secondary,
};
use futures::StreamExt;
use futures::channel::mpsc;
use gpui::prelude::*;
use gpui::{AsyncApp, Context, Entity, FontWeight, ScrollHandle, WeakEntity, Window, div, px, rgb};
use nodavideo_agent_acp::AcpAgentProvider;
use nodavideo_agent_claude::ClaudeAgentProvider;
use nodavideo_agent_codex::CodexAgentProvider;
use nodavideo_agent_runtime::{
    AgentConfig, AgentEvent, AgentRuntime, MockAgentProvider, SessionId,
};
use nodavideo_hyperframes_bridge::{CompositionAsset, load_compositions};
use nodavideo_mcp_server::{ExplorerChange, McpError, McpExecutor};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
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
        }
    }

    pub(crate) fn runtime(&self) -> &AgentRuntime {
        &self.runtime
    }
}

// Public APIs only
impl Editor {
    pub(super) fn assistant_panel_view(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let assistant = &self.assistant;
        let mut header = div()
            .h(px(64.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px(px(16.0))
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("AI Assistant"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(MUTED))
                            .text_ellipsis()
                            .child(assistant.provider_display_name()),
                    ),
            );
        if assistant.busy {
            header = header.child(
                div()
                    .flex_shrink_0()
                    .font_family("monospace")
                    .text_size(px(11.0))
                    .text_color(rgb(ACCENT))
                    .child("WORKING…"),
            );
        }
        header = header.child(
            button_cyberpunk_secondary("assistant-cycle-provider", "Switch", true, window, cx)
                .on_click(cx.listener(|editor, _, _, cx| {
                    editor.emit_event(cx, AppEvent::AssistantProviderCycled);
                })),
        );

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
            .when(assistant.entries.is_empty(), |this| {
                this.child(
                    div()
                        .text_size(px(12.0))
                        .line_height(px(18.0))
                        .text_color(rgb(MUTED))
                        .child(
                            "Ask the assistant to inspect the timeline, propose an edit, or \
                             explain a clip. Edits that need approval appear here as cards.",
                        ),
                )
            })
            .children(assistant.entries.iter().map(assistant_entry_view));

        let approval = assistant
            .pending_approval
            .as_ref()
            .map(|approval| assistant_approval_card_view(approval, window, cx));

        let input_row = div()
            .flex_shrink_0()
            .border_t_1()
            .border_color(rgb(BORDER))
            .p_2()
            .flex()
            .items_center()
            .gap_2()
            .child(div().flex_1().min_w_0().child(assistant.input.clone()))
            .child(
                button_cyberpunk_primary("assistant-send", "Send", true, window, cx).on_click(
                    cx.listener(|editor, _, _, cx| {
                        let message = editor.assistant.input.read(cx).text().trim().to_string();
                        if message.is_empty() {
                            return;
                        }
                        editor
                            .assistant
                            .input
                            .update(cx, |input, cx| input.clear(cx));
                        editor.emit_event(cx, AppEvent::AssistantMessageSubmitted { message });
                    }),
                ),
            );

        div()
            .id("editor-assistant-panel")
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(PANEL))
            .text_color(rgb(TEXT))
            .child(header)
            .child(messages)
            .when_some(approval, |this, approval| this.child(approval))
            .child(input_row)
            .into_any_element()
    }

    pub(super) fn send_assistant_message(&mut self, message: String, cx: &mut Context<Self>) {
        if self.assistant.busy {
            return;
        }
        self.assistant
            .push_entry(AssistantEntry::User(message.clone()));
        self.assistant.busy = true;
        self.assistant.scroll.scroll_to_bottom();
        cx.notify();

        let runtime = self.assistant.runtime.clone();
        let provider_id = self.assistant.provider_id().to_string();
        let existing_session = self.assistant.session;
        let generation = self.assistant.session_generation;
        let working_dir = self.project_root.clone();

        cx.spawn(async move |editor, cx| {
            let session = match existing_session {
                Some(session) => session,
                None => {
                    let config = AgentConfig {
                        provider_id,
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
                async move { runtime.send_message(session, message).await }
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

    pub(super) fn cycle_assistant_provider(&mut self, cx: &mut Context<Self>) {
        let previous_session = self.assistant.cycle_provider();
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
        cx.notify();
    }
}

// Private APIs only
impl Editor {
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
        self.assistant.push_entry(AssistantEntry::ToolCall {
            tool_name: tool_name.clone(),
            arguments: arguments.to_string(),
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
        self.assistant.push_entry(AssistantEntry::ToolResult {
            tool_name,
            result: result.to_string(),
            is_error,
        });
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

    fn cycle_provider(&mut self) -> Option<SessionId> {
        self.provider_index = (self.provider_index + 1) % ASSISTANT_PROVIDERS.len();
        self.session_generation += 1;
        self.pending_approval = None;
        self.busy = false;
        self.session.take()
    }

    fn push_entry(&mut self, entry: AssistantEntry) {
        const MAX_ENTRIES: usize = 400;
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push(entry);
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
                    | Some(AssistantEntry::ToolCall { .. })
                    | Some(AssistantEntry::ToolResult { .. })
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
                    | Some(AssistantEntry::ToolCall { .. })
                    | Some(AssistantEntry::ToolResult { .. })
                    | Some(AssistantEntry::Info(_))
                    | Some(AssistantEntry::Error(_))
                    | None => false,
                };
                if !appended {
                    self.push_entry(AssistantEntry::Thought(chunk));
                }
            }
            AgentEvent::ToolCallRequested {
                tool_name,
                arguments,
                ..
            } => {
                self.push_entry(AssistantEntry::ToolCall {
                    tool_name,
                    arguments: arguments.to_string(),
                });
            }
            AgentEvent::ToolCallResult {
                tool_name,
                result,
                is_error,
                ..
            } => {
                // 进程内执行的提供方由编辑器回填真实结果，忽略其自带的占位结果。
                if !self.provider_executes_in_process() {
                    self.push_entry(AssistantEntry::ToolResult {
                        tool_name,
                        result: result.to_string(),
                        is_error,
                    });
                }
            }
            AgentEvent::ApprovalRequested {
                request_id,
                title,
                description,
                tool_name,
                command_summary,
                parameters,
            } => {
                self.push_entry(AssistantEntry::Info(format!(
                    "Approval requested for {tool_name}: {parameters}"
                )));
                self.pending_approval = Some(AssistantApproval {
                    request_id,
                    title,
                    description,
                    tool_name,
                    command_summary,
                });
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
                if let Some(summary) = summary {
                    self.push_entry(AssistantEntry::Info(summary));
                }
            }
            AgentEvent::SessionError { code, message } => {
                self.busy = false;
                self.push_entry(AssistantEntry::Error(format!("{code}: {message}")));
            }
        }
        self.scroll.scroll_to_bottom();
        cx.notify();
    }
}

enum AssistantEntry {
    User(String),
    Message(String),
    Thought(String),
    ToolCall {
        tool_name: String,
        arguments: String,
    },
    ToolResult {
        tool_name: String,
        result: String,
        is_error: bool,
    },
    Info(String),
    Error(String),
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

#[derive(Clone, Copy)]
struct AssistantProvider {
    id: &'static str,
    display_name: &'static str,
    /// True when the editor executes the provider's tool calls in-process (offline Mock).
    /// Self-executing providers (ACP/Codex/Claude) run their own tools and only stream results.
    executes_in_process: bool,
}

const ASSISTANT_PROVIDERS: [AssistantProvider; 5] = [
    AssistantProvider {
        id: "mock",
        display_name: "Mock Assistant (Offline)",
        executes_in_process: true,
    },
    AssistantProvider {
        id: "acp-antigravity",
        display_name: "Antigravity (ACP)",
        executes_in_process: false,
    },
    AssistantProvider {
        id: "acp-gemini",
        display_name: "Gemini CLI (ACP)",
        executes_in_process: false,
    },
    AssistantProvider {
        id: "codex-app-server",
        display_name: "Codex (App-Server)",
        executes_in_process: false,
    },
    AssistantProvider {
        id: "claude-cli",
        display_name: "Claude Code (CLI)",
        executes_in_process: false,
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

fn assistant_entry_view(entry: &AssistantEntry) -> gpui::AnyElement {
    match entry {
        AssistantEntry::User(message) => div()
            .flex()
            .justify_end()
            .child(
                div()
                    .max_w(px(340.0))
                    .rounded(px(8.0))
                    .bg(rgb(SURFACE_HOVER))
                    .p_2()
                    .text_size(px(12.0))
                    .line_height(px(18.0))
                    .child(message.clone()),
            )
            .into_any_element(),
        AssistantEntry::Message(message) => div()
            .flex()
            .flex_col()
            .gap_1()
            .child(assistant_entry_label("ASSISTANT", ACCENT))
            .child(
                div()
                    .text_size(px(12.0))
                    .line_height(px(18.0))
                    .child(message.clone()),
            )
            .into_any_element(),
        AssistantEntry::Thought(thought) => div()
            .flex()
            .flex_col()
            .gap_1()
            .child(assistant_entry_label("THOUGHT", MUTED))
            .child(
                div()
                    .text_size(px(12.0))
                    .line_height(px(17.0))
                    .text_color(rgb(MUTED))
                    .child(thought.clone()),
            )
            .into_any_element(),
        AssistantEntry::ToolCall {
            tool_name,
            arguments,
        } => assistant_monospace_block_view("TOOL CALL", tool_name, arguments, LINK)
            .into_any_element(),
        AssistantEntry::ToolResult {
            tool_name,
            result,
            is_error,
        } => {
            let color = if *is_error {
                ERROR
            } else {
                MUTED
            };
            assistant_monospace_block_view("TOOL RESULT", tool_name, result, color)
                .into_any_element()
        }
        AssistantEntry::Info(info) => div()
            .text_size(px(11.0))
            .line_height(px(16.0))
            .text_color(rgb(MUTED))
            .child(info.clone())
            .into_any_element(),
        AssistantEntry::Error(error) => div()
            .rounded(px(6.0))
            .bg(rgb(SURFACE))
            .p_2()
            .text_size(px(12.0))
            .line_height(px(18.0))
            .text_color(rgb(ERROR))
            .child(error.clone())
            .into_any_element(),
    }
}

fn assistant_approval_card_view(
    approval: &AssistantApproval,
    window: &mut Window,
    cx: &mut Context<Editor>,
) -> gpui::AnyElement {
    div()
        .flex_shrink_0()
        .m_3()
        .p_3()
        .rounded(px(8.0))
        .border_1()
        .border_color(rgb(ACCENT))
        .bg(rgb(SURFACE))
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(ACCENT))
                .child(approval.title.clone()),
        )
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(18.0))
                .child(approval.description.clone()),
        )
        .child(
            div()
                .font_family("monospace")
                .text_size(px(11.0))
                .text_color(rgb(MUTED))
                .child(format!(
                    "{} · {}",
                    approval.tool_name, approval.command_summary
                )),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    button_cyberpunk_primary("assistant-approve", "Approve", true, window, cx)
                        .on_click(cx.listener(|editor, _, _, cx| {
                            editor.emit_event(
                                cx,
                                AppEvent::AssistantApprovalDecided { approved: true },
                            );
                        })),
                )
                .child(
                    button_cyberpunk_secondary("assistant-reject", "Reject", true, window, cx)
                        .on_click(cx.listener(|editor, _, _, cx| {
                            editor.emit_event(
                                cx,
                                AppEvent::AssistantApprovalDecided { approved: false },
                            );
                        })),
                ),
        )
        .into_any_element()
}

fn assistant_monospace_block_view(
    label: &'static str,
    tool_name: &str,
    body: &str,
    label_color: u32,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(assistant_entry_label(label, label_color))
                .child(
                    div()
                        .font_family("monospace")
                        .text_size(px(11.0))
                        .text_color(rgb(LINK))
                        .child(tool_name.to_string()),
                ),
        )
        .child(
            div()
                .rounded(px(6.0))
                .bg(rgb(SURFACE))
                .p_2()
                .font_family("monospace")
                .text_size(px(11.0))
                .line_height(px(16.0))
                .text_color(rgb(TEXT))
                .child(body.to_string()),
        )
}

fn assistant_entry_label(label: &'static str, color: u32) -> gpui::Div {
    div()
        .font_family("monospace")
        .text_size(px(10.0))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(color))
        .child(label)
}
