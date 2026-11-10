//! Codex app-server AgentProvider adapter implementation.

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

use crate::protocol::{CodexRpcNotification, CodexRpcRequest, CodexRpcResponse};

struct ActiveCodexSession {
    _config: AgentConfig,
    tx: broadcast::Sender<AgentEvent>,
    child: Option<Child>,
    stdin_tx: Option<tokio::sync::mpsc::Sender<String>>,
}

/// AgentProvider adapter communicating with OpenAI Codex app-server.
pub struct CodexAgentProvider {
    id: String,
    display_name: String,
    binary: String,
    args: Vec<String>,
    sessions: Arc<Mutex<HashMap<SessionId, ActiveCodexSession>>>,
}

impl std::fmt::Debug for CodexAgentProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexAgentProvider")
            .field("id", &self.id)
            .field("display_name", &self.display_name)
            .field("binary", &self.binary)
            .finish_non_exhaustive()
    }
}

impl Default for CodexAgentProvider {
    fn default() -> Self {
        Self::new(
            "codex-app-server",
            "Codex (App-Server)",
            "codex",
            vec!["app-server".into()],
        )
    }
}

impl CodexAgentProvider {
    /// Create a new Codex provider instance.
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

    /// Construct a simulated Codex provider for automated testing and offline development.
    pub fn simulated() -> Self {
        Self {
            id: "codex-simulated".into(),
            display_name: "Codex (Offline Simulation)".into(),
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
impl AgentProvider for CodexAgentProvider {
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
                AgentRuntimeError::ProtocolError("Failed to capture Codex stdout".into())
            })?;
            let stdin = child.stdin.take().ok_or_else(|| {
                AgentRuntimeError::ProtocolError("Failed to capture Codex stdin".into())
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

                    if let Ok(notif) = serde_json::from_str::<CodexRpcNotification>(trimmed) {
                        match notif.method.as_str() {
                            "codex/thought" => {
                                if let Some(params) = notif.params {
                                    if let Some(text) = params.get("text").and_then(|v| v.as_str())
                                    {
                                        let _ = tx_clone
                                            .send(AgentEvent::ThoughtChunk(text.to_string()));
                                    }
                                }
                            }
                            "codex/message" | "codex/token" => {
                                if let Some(params) = notif.params {
                                    if let Some(chunk) =
                                        params.get("content").and_then(|v| v.as_str())
                                    {
                                        let _ = tx_clone
                                            .send(AgentEvent::MessageChunk(chunk.to_string()));
                                    }
                                }
                            }
                            "codex/approvalRequested" => {
                                if let Some(params) = notif.params {
                                    let req_id = params
                                        .get("request_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("req-1")
                                        .to_string();
                                    let tool = params
                                        .get("tool_name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("edit")
                                        .to_string();
                                    let title = params
                                        .get("title")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Approval Required")
                                        .to_string();
                                    let desc = params
                                        .get("description")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let cmd_summary = params
                                        .get("command_summary")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let p = params
                                        .get("parameters")
                                        .cloned()
                                        .unwrap_or(serde_json::Value::Null);

                                    let _ = tx_clone.send(AgentEvent::ApprovalRequested {
                                        request_id: req_id,
                                        tool_name: tool,
                                        title,
                                        description: desc,
                                        command_summary: cmd_summary,
                                        parameters: p,
                                    });
                                }
                            }
                            "codex/done" => {
                                let _ =
                                    tx_clone.send(AgentEvent::SessionCompleted { summary: None });
                                break;
                            }
                            _ => {}
                        }
                    } else if let Ok(resp) = serde_json::from_str::<CodexRpcResponse>(trimmed) {
                        if let Some(err) = resp.error {
                            let _ = tx_clone.send(AgentEvent::SessionError {
                                code: "CODEX_ERROR".into(),
                                message: err.to_string(),
                            });
                            break;
                        }
                    }
                }
            });

            let mut map = self.sessions.lock().unwrap();
            map.insert(
                session_id,
                ActiveCodexSession {
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
                ActiveCodexSession {
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
            let req = CodexRpcRequest::new(
                1,
                "codex/sendMessage",
                Some(serde_json::json!({ "message": message })),
            );
            let serialized = serde_json::to_string(&req).map_err(|e| {
                AgentRuntimeError::ProtocolError(format!("Failed to serialize message: {e}"))
            })?;
            sender.send(serialized).await.map_err(|_| {
                AgentRuntimeError::ProtocolError("Codex stdin channel closed".into())
            })?;
        } else {
            // Simulated response flow
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let _ = tx_clone.send(AgentEvent::ThoughtChunk(
                    "Codex app-server analyzing sequence structure...".into(),
                ));
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                let _ = tx_clone.send(AgentEvent::MessageChunk(format!(
                    "Codex app-server response to: '{message}'"
                )));
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

                let _ = tx_clone.send(AgentEvent::SessionCompleted {
                    summary: Some("Codex task finished successfully.".into()),
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
            let req = CodexRpcRequest::new(
                2,
                "codex/approvalDecision",
                Some(serde_json::json!({
                    "request_id": request_id,
                    "approved": approved
                })),
            );
            if let Ok(serialized) = serde_json::to_string(&req) {
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
