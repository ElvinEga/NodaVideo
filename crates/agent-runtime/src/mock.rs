//! In-memory mock agent provider for automated tests and offline preview.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::error::AgentRuntimeError;
use crate::event::AgentEvent;
use crate::id::SessionId;
use crate::provider::{AgentConfig, AgentModel, AgentProvider};

struct SessionState {
    _config: AgentConfig,
    tx: broadcast::Sender<AgentEvent>,
    pending_approvals: HashMap<String, tokio::sync::oneshot::Sender<bool>>,
    /// Editor-executed tool results awaited by the scripted composition loop, keyed by call id.
    pending_tool_results: HashMap<String, tokio::sync::oneshot::Sender<(serde_json::Value, bool)>>,
}

/// Simulated agent provider capable of scripted responses and approval workflows without external CLIs.
#[derive(Clone)]
pub struct MockAgentProvider {
    sessions: Arc<Mutex<HashMap<SessionId, SessionState>>>,
}

impl std::fmt::Debug for MockAgentProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockAgentProvider").finish_non_exhaustive()
    }
}

impl Default for MockAgentProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAgentProvider {
    /// Create a new mock provider instance.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl AgentProvider for MockAgentProvider {
    fn provider_id(&self) -> &str {
        "mock"
    }

    fn display_name(&self) -> &str {
        "Mock Assistant (Offline / Test)"
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn list_models(&self) -> Result<Vec<AgentModel>, AgentRuntimeError> {
        Ok(vec![AgentModel {
            id: "demo-loop".into(),
            display_name: "Demo loop".into(),
            description: Some("Scripted offline responses; no external model.".into()),
            efforts: Vec::new(),
            default_effort: None,
            is_default: true,
        }])
    }

    async fn start_session(&self, config: AgentConfig) -> Result<SessionId, AgentRuntimeError> {
        let session_id = SessionId::new();
        let (tx, _) = broadcast::channel(64);

        let state = SessionState {
            _config: config,
            tx,
            pending_approvals: HashMap::new(),
            pending_tool_results: HashMap::new(),
        };

        let mut lock = self.sessions.lock().unwrap();
        lock.insert(session_id, state);

        Ok(session_id)
    }

    async fn send_message(
        &self,
        session: SessionId,
        message: String,
    ) -> Result<(), AgentRuntimeError> {
        let tx = {
            let lock = self.sessions.lock().unwrap();
            let state = lock
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            state.tx.clone()
        };

        let msg_lower = message.to_lowercase();
        let sessions_clone = self.sessions.clone();

        tokio::spawn(async move {
            // 1. Emit thought
            let _ = tx.send(AgentEvent::ThoughtChunk(
                "Analyzing timeline state and planning editing operation...".into(),
            ));

            // Small delay to simulate processing
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;

            if msg_lower.contains("split") {
                // Emit text message
                let _ = tx.send(AgentEvent::MessageChunk(
                    "I will split the active clip at the current playhead position.".into(),
                ));

                // Request user approval before splitting
                let request_id = format!("req_{}", uuid::Uuid::new_v4());
                let (appr_tx, appr_rx) = tokio::sync::oneshot::channel();

                {
                    let mut lock = sessions_clone.lock().unwrap();
                    if let Some(state) = lock.get_mut(&session) {
                        state.pending_approvals.insert(request_id.clone(), appr_tx);
                    }
                }

                let _ = tx.send(AgentEvent::ApprovalRequested {
                    request_id: request_id.clone(),
                    title: "Split Clip".into(),
                    description: "Split the video clip on track V1 at playhead frame".into(),
                    tool_name: "split_clip".into(),
                    command_summary: "SplitClip { track: V1, frame: playhead }".into(),
                    parameters: serde_json::json!({
                        "track_id": "V1",
                        "frame": 24
                    }),
                });

                // Wait for user approval decision
                if let Ok(approved) = appr_rx.await {
                    let _ = tx.send(AgentEvent::ApprovalDecision {
                        request_id,
                        approved,
                    });

                    if approved {
                        let call_id = format!("call_{}", uuid::Uuid::new_v4());
                        let _ = tx.send(AgentEvent::ToolCallRequested {
                            call_id: call_id.clone(),
                            tool_name: "split_clip".into(),
                            arguments: serde_json::json!({
                                "track_id": "V1",
                                "frame": 24
                            }),
                        });

                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                        let _ = tx.send(AgentEvent::ToolCallResult {
                            call_id,
                            tool_name: "split_clip".into(),
                            result: serde_json::json!({ "status": "success", "new_clip_created": true }),
                            is_error: false,
                        });

                        let _ = tx.send(AgentEvent::SessionCompleted {
                            summary: Some("Successfully split clip at playhead.".into()),
                        });
                    } else {
                        let _ = tx.send(AgentEvent::MessageChunk(
                            "Operation was rejected by the user.".into(),
                        ));
                        let _ = tx.send(AgentEvent::SessionCompleted {
                            summary: Some("Split operation cancelled by user.".into()),
                        });
                    }
                }
            } else if msg_lower.contains("export") {
                let _ = tx.send(AgentEvent::MessageChunk(
                    "I will export the sequence to an MP4 video file.".into(),
                ));

                let request_id = format!("req_{}", uuid::Uuid::new_v4());
                let (appr_tx, appr_rx) = tokio::sync::oneshot::channel();

                {
                    let mut lock = sessions_clone.lock().unwrap();
                    if let Some(state) = lock.get_mut(&session) {
                        state.pending_approvals.insert(request_id.clone(), appr_tx);
                    }
                }

                let _ = tx.send(AgentEvent::ApprovalRequested {
                    request_id: request_id.clone(),
                    title: "Export Sequence".into(),
                    description: "Export the full composition to 1080p24 MP4".into(),
                    tool_name: "export_video".into(),
                    command_summary: "ExportVideo { preset: 1080p24, codec: H.264 }".into(),
                    parameters: serde_json::json!({
                        "output_name": "exported_timeline.mp4"
                    }),
                });

                if let Ok(approved) = appr_rx.await {
                    let _ = tx.send(AgentEvent::ApprovalDecision {
                        request_id,
                        approved,
                    });

                    if approved {
                        let _ = tx.send(AgentEvent::SessionCompleted {
                            summary: Some("Export initiated successfully.".into()),
                        });
                    } else {
                        let _ = tx.send(AgentEvent::MessageChunk("Export was rejected.".into()));
                        let _ = tx.send(AgentEvent::SessionCompleted {
                            summary: Some("Export rejected by user.".into()),
                        });
                    }
                }
            } else if msg_lower.contains("motion")
                || msg_lower.contains("composition")
                || msg_lower.contains("hyperframe")
                || msg_lower.contains("lower third")
                || msg_lower.contains("title")
            {
                let _ = tx.send(AgentEvent::MessageChunk(
                    "I'll create an editable motion graphics lower third, render it natively, \
                     and place it on the timeline."
                        .into(),
                ));

                // 1. Create the composition from a built-in template (stays editable on disk).
                let create_arguments = serde_json::json!({
                    "template_id": "lower-third-modern",
                    "name": "AI Lower Third",
                });
                let (create_result, create_error) = request_tool(
                    &tx,
                    &sessions_clone,
                    session,
                    "create_composition",
                    create_arguments,
                )
                .await;
                if create_error {
                    let _ = tx.send(AgentEvent::MessageChunk(format!(
                        "Could not create the composition: {create_result}"
                    )));
                    let _ = tx.send(AgentEvent::SessionCompleted {
                        summary: Some("Motion graphics creation failed.".into()),
                    });
                    return;
                }
                let Some(composition_id_str) = create_result
                    .get("composition")
                    .and_then(|composition| composition.get("id"))
                    .and_then(serde_json::Value::as_str)
                else {
                    let _ = tx.send(AgentEvent::SessionError {
                        code: "BAD_RESULT".into(),
                        message: "create_composition returned no composition id".into(),
                    });
                    return;
                };
                let composition_id = composition_id_str.to_string();

                // 2. Render it to a native transparent RGBA asset (resvg, off the UI thread).
                let render_arguments =
                    serde_json::json!({ "composition_id": composition_id.as_str() });
                let (_, render_error) = request_tool(
                    &tx,
                    &sessions_clone,
                    session,
                    "render_composition",
                    render_arguments,
                )
                .await;
                if render_error {
                    let _ = tx.send(AgentEvent::MessageChunk(
                        "Could not render the composition; it remains in the Media Pool.".into(),
                    ));
                    let _ = tx.send(AgentEvent::SessionCompleted {
                        summary: Some("Motion graphics rendering failed.".into()),
                    });
                    return;
                }

                // 3. Place it on the timeline. This mutates the edit, so it needs approval.
                let request_id = format!("req_{}", uuid::Uuid::new_v4());
                let (appr_tx, appr_rx) = tokio::sync::oneshot::channel();
                {
                    let mut lock = sessions_clone.lock().unwrap();
                    if let Some(state) = lock.get_mut(&session) {
                        state.pending_approvals.insert(request_id.clone(), appr_tx);
                    }
                }
                let _ = tx.send(AgentEvent::ApprovalRequested {
                    request_id: request_id.clone(),
                    title: "Place Motion Graphic".into(),
                    description: "Add the rendered lower third to a new upper video track. \
                                  It stays editable and is not flattened to video."
                        .into(),
                    tool_name: "add_composition_to_timeline".into(),
                    command_summary: "AddComposition { template: lower-third-modern }".into(),
                    parameters: serde_json::json!({
                        "composition_id": composition_id.as_str(),
                        "start_frame": 0
                    }),
                });

                let Ok(approved) = appr_rx.await else {
                    return;
                };
                let _ = tx.send(AgentEvent::ApprovalDecision {
                    request_id,
                    approved,
                });
                if !approved {
                    let _ = tx.send(AgentEvent::MessageChunk(
                        "Placement was rejected; the composition stays in the Media Pool.".into(),
                    ));
                    let _ = tx.send(AgentEvent::SessionCompleted {
                        summary: Some("Motion graphics placement cancelled.".into()),
                    });
                    return;
                }

                let place_arguments = serde_json::json!({
                    "composition_id": composition_id.as_str(),
                    "start_frame": 0
                });
                let (place_result, place_error) = request_tool(
                    &tx,
                    &sessions_clone,
                    session,
                    "add_composition_to_timeline",
                    place_arguments,
                )
                .await;
                if place_error {
                    let _ = tx.send(AgentEvent::MessageChunk(format!(
                        "Could not place the composition: {place_result}"
                    )));
                    let _ = tx.send(AgentEvent::SessionCompleted {
                        summary: Some("Motion graphics placement failed.".into()),
                    });
                    return;
                }
                let _ = tx.send(AgentEvent::MessageChunk(
                    "Done — the lower third is on the timeline and remains editable.".into(),
                ));
                let _ = tx.send(AgentEvent::SessionCompleted {
                    summary: Some(
                        "Created, rendered and placed an editable motion graphic.".into(),
                    ),
                });
            } else {
                // Default friendly response
                let _ = tx.send(AgentEvent::MessageChunk(format!(
                    "I received your instruction: \"{message}\". How would you like me to edit the timeline?"
                )));
                let _ = tx.send(AgentEvent::SessionCompleted { summary: None });
            }
        });

        Ok(())
    }

    async fn submit_approval(
        &self,
        session: SessionId,
        request_id: String,
        approved: bool,
    ) -> Result<(), AgentRuntimeError> {
        let sender = {
            let mut lock = self.sessions.lock().unwrap();
            let state = lock
                .get_mut(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            state.pending_approvals.remove(&request_id)
        };

        if let Some(tx) = sender {
            let _ = tx.send(approved);
            Ok(())
        } else {
            Err(AgentRuntimeError::Other(format!(
                "No pending approval found for request '{request_id}'"
            )))
        }
    }

    async fn submit_tool_result(
        &self,
        session: SessionId,
        call_id: String,
        result: serde_json::Value,
        is_error: bool,
    ) -> Result<(), AgentRuntimeError> {
        let sender = {
            let mut lock = self.sessions.lock().unwrap();
            let state = lock
                .get_mut(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            state.pending_tool_results.remove(&call_id)
        };

        match sender {
            Some(tx) => {
                let _ = tx.send((result, is_error));
                Ok(())
            }
            None => Err(AgentRuntimeError::Other(format!(
                "No pending tool result found for call '{call_id}'"
            ))),
        }
    }

    async fn cancel(&self, session: SessionId) -> Result<(), AgentRuntimeError> {
        let tx = {
            let lock = self.sessions.lock().unwrap();
            let state = lock
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            state.tx.clone()
        };

        let _ = tx.send(AgentEvent::SessionError {
            code: "CANCELLED".into(),
            message: "Session was cancelled by user".into(),
        });

        Ok(())
    }

    fn subscribe(
        &self,
        session: SessionId,
    ) -> Result<broadcast::Receiver<AgentEvent>, AgentRuntimeError> {
        let lock = self.sessions.lock().unwrap();
        let state = lock
            .get(&session)
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;
        Ok(state.tx.subscribe())
    }
}

/// Emits a tool call for the editor to execute in-process and awaits its real result.
/// The editor delivers the result through `submit_tool_result`, keyed by the call id.
/// Registering the reply channel before emitting avoids a race with a fast editor.
async fn request_tool(
    tx: &broadcast::Sender<AgentEvent>,
    sessions: &Arc<Mutex<HashMap<SessionId, SessionState>>>,
    session: SessionId,
    tool_name: &str,
    arguments: serde_json::Value,
) -> (serde_json::Value, bool) {
    let call_id = format!("call_{}", uuid::Uuid::new_v4());
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    {
        let mut lock = sessions.lock().unwrap();
        match lock.get_mut(&session) {
            Some(state) => {
                state
                    .pending_tool_results
                    .insert(call_id.clone(), result_tx);
            }
            None => {
                return (
                    serde_json::json!({ "error": "session no longer exists" }),
                    true,
                );
            }
        }
    }
    let _ = tx.send(AgentEvent::ToolCallRequested {
        call_id,
        tool_name: tool_name.to_string(),
        arguments,
    });
    match result_rx.await {
        Ok((result, is_error)) => (result, is_error),
        Err(_) => (
            serde_json::json!({ "error": "tool result channel closed" }),
            true,
        ),
    }
}
