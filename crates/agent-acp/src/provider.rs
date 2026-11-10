//! ACP AgentProvider implementation managing stdio JSON-RPC sessions.

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

use crate::protocol::JsonRpcRequest;

struct ActiveAcpSession {
    _config: AgentConfig,
    tx: broadcast::Sender<AgentEvent>,
    child: Option<Child>,
    stdin_tx: Option<tokio::sync::mpsc::Sender<String>>,
}

/// AgentProvider adapter communicating with external CLI agents over stdio Agent Communication Protocol.
pub struct AcpAgentProvider {
    id: String,
    display_name: String,
    binary: String,
    args: Vec<String>,
    sessions: Arc<Mutex<HashMap<SessionId, ActiveAcpSession>>>,
}

impl std::fmt::Debug for AcpAgentProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcpAgentProvider")
            .field("id", &self.id)
            .field("display_name", &self.display_name)
            .field("binary", &self.binary)
            .finish_non_exhaustive()
    }
}

impl AcpAgentProvider {
    /// Create a new ACP provider for a specified CLI command (e.g. `agy` or `gemini`).
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

    /// Construct provider configured for Antigravity (`agy acp`).
    pub fn antigravity() -> Self {
        Self::new(
            "acp-antigravity",
            "Antigravity (ACP)",
            "agy",
            vec!["acp".into()],
        )
    }

    /// Construct provider configured for Gemini CLI (`gemini --acp`).
    pub fn gemini() -> Self {
        Self::new(
            "acp-gemini",
            "Gemini CLI (ACP)",
            "gemini",
            vec!["--acp".into()],
        )
    }

    /// Construct a simulated ACP provider for automated testing and offline development.
    pub fn simulated(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
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
impl AgentProvider for AcpAgentProvider {
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
        let (tx, _) = broadcast::channel(128);

        // Attempt to spawn the child process if binary is available
        let (child, stdin_tx) = if Self::probe_binary_exists(&self.binary) {
            let mut cmd = Command::new(&self.binary);
            cmd.args(&self.args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            if let Some(ref dir) = config.working_dir {
                cmd.current_dir(dir);
            }
            for (k, v) in &config.environment {
                cmd.env(k, v);
            }

            match cmd.spawn() {
                Ok(mut child) => {
                    let stdin = child.stdin.take();
                    let stdout = child.stdout.take();
                    let (st_tx, mut st_rx) = tokio::sync::mpsc::channel::<String>(32);

                    // Background stdin writer task
                    if let Some(mut stdin_pipe) = stdin {
                        tokio::spawn(async move {
                            while let Some(line) = st_rx.recv().await {
                                if stdin_pipe.write_all(line.as_bytes()).await.is_err()
                                    || stdin_pipe.flush().await.is_err()
                                {
                                    break;
                                }
                            }
                        });
                    }

                    // Background stdout reader task
                    let tx_out = tx.clone();
                    if let Some(stdout_pipe) = stdout {
                        tokio::spawn(async move {
                            let mut reader = BufReader::new(stdout_pipe).lines();
                            while let Ok(Some(line)) = reader.next_line().await {
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                                    // Parse ACP message or notification
                                    if let Some(method) = val.get("method").and_then(|m| m.as_str())
                                    {
                                        let params = val.get("params").cloned().unwrap_or_default();
                                        match method {
                                            "agent/message" => {
                                                if let Some(text) =
                                                    params.get("text").and_then(|t| t.as_str())
                                                {
                                                    let _ = tx_out.send(AgentEvent::MessageChunk(
                                                        text.into(),
                                                    ));
                                                }
                                            }
                                            "agent/thought" => {
                                                if let Some(thought) =
                                                    params.get("thought").and_then(|t| t.as_str())
                                                {
                                                    let _ = tx_out.send(AgentEvent::ThoughtChunk(
                                                        thought.into(),
                                                    ));
                                                }
                                            }
                                            "agent/tool_call" => {
                                                let call_id = params
                                                    .get("call_id")
                                                    .and_then(|c| c.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let tool_name = params
                                                    .get("tool_name")
                                                    .and_then(|t| t.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let args = params
                                                    .get("arguments")
                                                    .cloned()
                                                    .unwrap_or(serde_json::json!({}));
                                                let _ =
                                                    tx_out.send(AgentEvent::ToolCallRequested {
                                                        call_id,
                                                        tool_name,
                                                        arguments: args,
                                                    });
                                            }
                                            "agent/approval_request" => {
                                                let request_id = params
                                                    .get("request_id")
                                                    .and_then(|r| r.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let title = params
                                                    .get("title")
                                                    .and_then(|t| t.as_str())
                                                    .unwrap_or("Approval Required")
                                                    .to_string();
                                                let description = params
                                                    .get("description")
                                                    .and_then(|d| d.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let tool_name = params
                                                    .get("tool_name")
                                                    .and_then(|t| t.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let command_summary = params
                                                    .get("command_summary")
                                                    .and_then(|c| c.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let parameters = params
                                                    .get("parameters")
                                                    .cloned()
                                                    .unwrap_or(serde_json::json!({}));
                                                let _ =
                                                    tx_out.send(AgentEvent::ApprovalRequested {
                                                        request_id,
                                                        title,
                                                        description,
                                                        tool_name,
                                                        command_summary,
                                                        parameters,
                                                    });
                                            }
                                            "agent/complete" => {
                                                let summary = params
                                                    .get("summary")
                                                    .and_then(|s| s.as_str())
                                                    .map(|s| s.to_string());
                                                let _ = tx_out
                                                    .send(AgentEvent::SessionCompleted { summary });
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        });
                    }

                    (Some(child), Some(st_tx))
                }
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        };

        let session = ActiveAcpSession {
            _config: config,
            tx,
            child,
            stdin_tx,
        };

        let mut lock = self.sessions.lock().unwrap();
        lock.insert(session_id, session);

        Ok(session_id)
    }

    async fn send_message(
        &self,
        session: SessionId,
        message: String,
    ) -> Result<(), AgentRuntimeError> {
        let (stdin_tx, tx) = {
            let lock = self.sessions.lock().unwrap();
            let state = lock
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            (state.stdin_tx.clone(), state.tx.clone())
        };

        if let Some(stdin) = stdin_tx {
            let req = JsonRpcRequest::new(
                uuid::Uuid::new_v4().to_string(),
                "session/send_message",
                Some(serde_json::json!({ "message": message })),
            );
            let mut line = serde_json::to_string(&req)?;
            line.push('\n');
            let _ = stdin.send(line).await;
        } else {
            // Simulated fallback response when testing without child binary
            let tx_clone = tx.clone();
            let display_name = self.display_name.clone();
            tokio::spawn(async move {
                let _ = tx_clone.send(AgentEvent::ThoughtChunk(format!(
                    "[{display_name}] Parsing user editing intent: \"{message}\"..."
                )));
                tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                let _ = tx_clone.send(AgentEvent::MessageChunk(format!(
                    "[{display_name}] Ready to inspect sequence and execute video editing tools for: \"{message}\"."
                )));
                let _ = tx_clone.send(AgentEvent::SessionCompleted { summary: None });
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
        let stdin_tx = {
            let lock = self.sessions.lock().unwrap();
            let state = lock
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            state.stdin_tx.clone()
        };

        if let Some(stdin) = stdin_tx {
            let req = JsonRpcRequest::new(
                uuid::Uuid::new_v4().to_string(),
                "session/submit_approval",
                Some(serde_json::json!({
                    "request_id": request_id,
                    "approved": approved
                })),
            );
            let mut line = serde_json::to_string(&req)?;
            line.push('\n');
            let _ = stdin.send(line).await;
        }

        Ok(())
    }

    async fn cancel(&self, session: SessionId) -> Result<(), AgentRuntimeError> {
        let child = {
            let mut lock = self.sessions.lock().unwrap();
            let state = lock
                .get_mut(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            state.child.take()
        };

        if let Some(mut c) = child {
            let _ = c.kill().await;
        }

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
