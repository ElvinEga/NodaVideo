//! In-memory mock agent provider for automated tests and offline preview.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::error::AgentRuntimeError;
use crate::event::AgentEvent;
use crate::id::SessionId;
use crate::provider::{AgentConfig, AgentProvider};

struct SessionState {
    _config: AgentConfig,
    tx: broadcast::Sender<AgentEvent>,
    pending_approvals: HashMap<String, tokio::sync::oneshot::Sender<bool>>,
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

    async fn start_session(&self, config: AgentConfig) -> Result<SessionId, AgentRuntimeError> {
        let session_id = SessionId::new();
        let (tx, _) = broadcast::channel(64);

        let state = SessionState {
            _config: config,
            tx,
            pending_approvals: HashMap::new(),
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
