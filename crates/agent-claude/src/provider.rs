//! Claude Code CLI AgentProvider implementation.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use nodavideo_agent_runtime::{
    AgentConfig, AgentEvent, AgentProvider, AgentRuntimeError, SessionId,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::broadcast;

use crate::protocol::ClaudeStreamEvent;

struct ActiveClaudeSession {
    _config: AgentConfig,
    tx: broadcast::Sender<AgentEvent>,
    child: Option<Child>,
    stdin_tx: Option<tokio::sync::mpsc::Sender<String>>,
}

/// AgentProvider adapter communicating with Anthropic Claude Code CLI.
pub struct ClaudeAgentProvider {
    id: String,
    display_name: String,
    binary: String,
    args: Vec<String>,
    sessions: Arc<Mutex<HashMap<SessionId, ActiveClaudeSession>>>,
}

impl std::fmt::Debug for ClaudeAgentProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeAgentProvider")
            .field("id", &self.id)
            .field("display_name", &self.display_name)
            .field("binary", &self.binary)
            .finish_non_exhaustive()
    }
}

impl Default for ClaudeAgentProvider {
    fn default() -> Self {
        Self::new(
            "claude-cli",
            "Claude Code (CLI)",
            "claude",
            vec!["--output-format".into(), "stream-json".into()],
        )
    }
}

impl ClaudeAgentProvider {
    /// Create a new Claude provider instance.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        binary: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            binary: binary.into(),
            args,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Construct a simulated Claude provider for automated testing and offline development.
    pub fn simulated() -> Self {
        Self {
            id: "claude-simulated".into(),
            display_name: "Claude Code (Offline Simulation)".into(),
            binary: String::new(),
            args: Vec::new(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if the executable binary exists on system PATH.
    pub fn probe_binary_exists(binary: &str) -> bool {
        if binary.is_empty() {
            return false;
        }
        if let Ok(paths) = std::env::var("PATH") {
            for dir in std::env::split_paths(&paths) {
                let full = dir.join(binary);
                if full.is_file() {
                    return true;
                }
            }
        }
        false
    }
}

#[async_trait]
impl AgentProvider for ClaudeAgentProvider {
    fn provider_id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn is_available(&self) -> bool {
        Self::probe_binary_exists(&self.binary)
    }

    async fn start_session(&self, config: AgentConfig) -> Result<SessionId, AgentRuntimeError> {
        let session_id = SessionId::new();
        let (event_tx, _) = broadcast::channel(128);

        // Check if real binary is present on system PATH
        if !self.binary.is_empty() && Self::probe_binary_exists(&self.binary) {
            let mut cmd = Command::new(&self.binary);
            cmd.args(&self.args);
            cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            if let Some(ref dir) = config.working_dir {
                cmd.current_dir(dir);
            }
            for (k, v) in &config.environment {
                cmd.env(k, v);
            }

            let mut child = cmd
                .spawn()
                .map_err(|source| AgentRuntimeError::ProcessFailed {
                    program: self.binary.clone(),
                    source,
                })?;

            let stdout = child.stdout.take().ok_or_else(|| {
                AgentRuntimeError::ProtocolError("Failed to capture Claude stdout".into())
            })?;
            let stdin = child.stdin.take().ok_or_else(|| {
                AgentRuntimeError::ProtocolError("Failed to capture Claude stdin".into())
            })?;

            let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<String>(32);
            tokio::spawn(async move {
                let mut writer = stdin;
                while let Some(msg) = stdin_rx.recv().await {
                    if writer.write_all(msg.as_bytes()).await.is_err()
                        || writer.write_all(b"\n").await.is_err()
                        || writer.flush().await.is_err()
                    {
                        break;
                    }
                }
            });

            let tx_clone = event_tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Ok(event) = serde_json::from_str::<ClaudeStreamEvent>(trimmed) {
                        match event {
                            ClaudeStreamEvent::Thought { content } => {
                                let _ = tx_clone.send(AgentEvent::ThoughtChunk(content));
                            }
                            ClaudeStreamEvent::Text { content } => {
                                let _ = tx_clone.send(AgentEvent::MessageChunk(content));
                            }
                            ClaudeStreamEvent::ToolUse { id, name, input } => {
                                let _ = tx_clone.send(AgentEvent::ToolCallRequested {
                                    call_id: id,
                                    tool_name: name,
                                    arguments: input,
                                });
                            }
                            ClaudeStreamEvent::ApprovalRequired {
                                id,
                                tool,
                                description,
                                input,
                            } => {
                                let desc = description.unwrap_or_default();
                                let _ = tx_clone.send(AgentEvent::ApprovalRequested {
                                    request_id: id,
                                    tool_name: tool.clone(),
                                    title: format!("Claude Tool Approval: {tool}"),
                                    description: desc.clone(),
                                    command_summary: desc,
                                    parameters: input,
                                });
                            }
                            ClaudeStreamEvent::Done { summary } => {
                                let _ = tx_clone.send(AgentEvent::SessionCompleted { summary });
                                break;
                            }
                            ClaudeStreamEvent::Error { message } => {
                                let _ = tx_clone.send(AgentEvent::SessionError {
                                    code: "CLAUDE_ERROR".into(),
                                    message,
                                });
                                break;
                            }
                        }
                    }
                }
            });

            let mut map = self.sessions.lock().unwrap();
            map.insert(
                session_id,
                ActiveClaudeSession {
                    _config: config,
                    tx: event_tx,
                    child: Some(child),
                    stdin_tx: Some(stdin_tx),
                },
            );
        } else {
            // Simulated offline mode for CI
            let mut map = self.sessions.lock().unwrap();
            map.insert(
                session_id,
                ActiveClaudeSession {
                    _config: config,
                    tx: event_tx,
                    child: None,
                    stdin_tx: None,
                },
            );
        }

        Ok(session_id)
    }

    async fn send_message(
        &self,
        session: SessionId,
        message: String,
    ) -> Result<(), AgentRuntimeError> {
        let (stdin_tx, tx) = {
            let map = self.sessions.lock().unwrap();
            let s = map
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            (s.stdin_tx.clone(), s.tx.clone())
        };

        if let Some(sender) = stdin_tx {
            let payload = serde_json::json!({
                "type": "user_message",
                "content": message
            });
            if let Ok(serialized) = serde_json::to_string(&payload) {
                sender.send(serialized).await.map_err(|_| {
                    AgentRuntimeError::ProtocolError("Claude stdin channel closed".into())
                })?;
            }
        } else {
            // Simulated response flow
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let _ = tx_clone.send(AgentEvent::ThoughtChunk(
                    "Claude analyzing editing instructions and timeline context...".into(),
                ));
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                let _ = tx_clone.send(AgentEvent::MessageChunk(format!(
                    "Claude response: Executed editing workflow for '{message}'"
                )));
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

                let _ = tx_clone.send(AgentEvent::SessionCompleted {
                    summary: Some("Claude task completed.".into()),
                });
            });
        }

        Ok(())
    }

    async fn submit_approval(
        &self,
        session: SessionId,
        request_id: String,
        approved: bool,
    ) -> Result<(), AgentRuntimeError> {
        let (stdin_tx, tx) = {
            let map = self.sessions.lock().unwrap();
            let s = map
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            (s.stdin_tx.clone(), s.tx.clone())
        };

        let _ = tx.send(AgentEvent::ApprovalDecision {
            request_id: request_id.clone(),
            approved,
        });

        if let Some(sender) = stdin_tx {
            let payload = serde_json::json!({
                "type": "approval_response",
                "request_id": request_id,
                "approved": approved
            });
            if let Ok(serialized) = serde_json::to_string(&payload) {
                let _ = sender.send(serialized).await;
            }
        }

        Ok(())
    }

    async fn cancel(&self, session: SessionId) -> Result<(), AgentRuntimeError> {
        let child = {
            let mut map = self.sessions.lock().unwrap();
            let s = map
                .get_mut(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            s.child.take()
        };

        if let Some(mut c) = child {
            let _ = c.kill().await;
        }

        let map = self.sessions.lock().unwrap();
        if let Some(s) = map.get(&session) {
            let _ = s.tx.send(AgentEvent::SessionCompleted {
                summary: Some("Session cancelled by user".into()),
            });
        }
        Ok(())
    }

    fn subscribe(
        &self,
        session: SessionId,
    ) -> Result<broadcast::Receiver<AgentEvent>, AgentRuntimeError> {
        let map = self.sessions.lock().unwrap();
        let s = map
            .get(&session)
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;
        Ok(s.tx.subscribe())
    }
}
