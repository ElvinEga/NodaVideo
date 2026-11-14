//! Codex app-server AgentProvider adapter implementation.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use nodavideo_agent_runtime::{
    AgentConfig, AgentEvent, AgentModel, AgentProvider, AgentRuntimeError, SessionId,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::protocol::{CodexRpcNotification, CodexRpcRequest, CodexRpcResultResponse};

const RPC_TIMEOUT: Duration = Duration::from_secs(60);
const INTERRUPT_TIMEOUT: Duration = Duration::from_secs(5);

/// Which server request an approval answers; each family has its own response shape.
enum ApprovalKind {
    CommandExecution,
    FileChange,
    Permissions(Value),
    LegacyExec,
    LegacyPatch,
}

/// Correlates client requests with replies; shared between the session and its reader task.
#[derive(Clone)]
struct RpcChannel {
    stdin_tx: mpsc::Sender<String>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    next_id: Arc<AtomicU64>,
}

struct SharedSession {
    approvals: Mutex<HashMap<String, ApprovalKind>>,
    current_turn: Mutex<Option<String>>,
}

struct ActiveCodexSession {
    thread_id: String,
    /// Resolved model id: the explicit choice, or the catalog default picked at start.
    model: Option<String>,
    effort: Option<String>,
    tx: broadcast::Sender<AgentEvent>,
    child: Option<Child>,
    rpc: Option<RpcChannel>,
    shared: Arc<SharedSession>,
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

// Public APIs only
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

    async fn list_models(&self) -> Result<Vec<AgentModel>, AgentRuntimeError> {
        if self.binary.is_empty() {
            return Ok(Vec::new());
        }
        if !Self::probe_binary_exists(&self.binary) {
            return Err(AgentRuntimeError::ProviderUnavailable(
                self.id.clone(),
                format!("'{}' was not found on PATH", self.binary),
            ));
        }
        let (mut child, rpc, stdout) = self.launch(&AgentConfig::default())?;
        let (tx, _) = broadcast::channel(16);
        spawn_reader(
            stdout,
            tx,
            rpc.clone(),
            Arc::new(SharedSession {
                approvals: Mutex::new(HashMap::new()),
                current_turn: Mutex::new(None),
            }),
        );
        let listed = async {
            initialize(&rpc).await?;
            let list = rpc.request("model/list", json!({})).await?;
            Ok::<Vec<AgentModel>, AgentRuntimeError>(parse_models(&list))
        }
        .await;
        let _ = child.kill().await;
        listed
    }

    async fn start_session(&self, config: AgentConfig) -> Result<SessionId, AgentRuntimeError> {
        let session_id = SessionId::new();
        let (tx, _) = broadcast::channel(128);
        let shared = Arc::new(SharedSession {
            approvals: Mutex::new(HashMap::new()),
            current_turn: Mutex::new(None),
        });

        if self.binary.is_empty() || !Self::probe_binary_exists(&self.binary) {
            let mut map = self.sessions.lock().unwrap();
            map.insert(
                session_id,
                ActiveCodexSession {
                    thread_id: String::new(),
                    model: None,
                    effort: None,
                    tx,
                    child: None,
                    rpc: None,
                    shared,
                },
            );
            return Ok(session_id);
        }

        let (mut child, rpc, stdout) = self.launch(&config)?;
        spawn_reader(stdout, tx.clone(), rpc.clone(), shared.clone());

        let started = async {
            initialize(&rpc).await?;
            // The server's own configured default model can be unavailable on the account,
            // so an unspecified choice resolves to the catalog's default instead.
            let model = match config.model.clone() {
                Some(model) => Some(model),
                None => Some(default_model_id(&rpc).await?),
            };
            let mut thread_params = json!({
                "sandbox": "workspace-write",
                "approvalPolicy": "on-request",
            });
            if let Some(dir) = &config.working_dir {
                thread_params["cwd"] = json!(dir.to_string_lossy());
            }
            if let Some(model) = &model {
                thread_params["model"] = json!(model);
            }
            let thread = rpc.request("thread/start", thread_params).await?;
            let thread_id = thread
                .get("thread")
                .and_then(|thread| thread.get("id"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AgentRuntimeError::ProtocolError("thread/start returned no thread id".into())
                })?
                .to_string();
            Ok::<(String, Option<String>), AgentRuntimeError>((thread_id, model))
        }
        .await;

        let (thread_id, model) = match started {
            Ok(started) => started,
            Err(error) => {
                let _ = child.kill().await;
                return Err(error);
            }
        };

        let mut map = self.sessions.lock().unwrap();
        map.insert(
            session_id,
            ActiveCodexSession {
                thread_id,
                model,
                effort: config.effort.clone(),
                tx,
                child: Some(child),
                rpc: Some(rpc),
                shared,
            },
        );
        Ok(session_id)
    }

    async fn send_message(
        &self,
        session: SessionId,
        message: String,
    ) -> Result<(), AgentRuntimeError> {
        let (rpc, tx, thread_id, model, effort, shared) = {
            let map = self.sessions.lock().unwrap();
            let state = map
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            (
                state.rpc.clone(),
                state.tx.clone(),
                state.thread_id.clone(),
                state.model.clone(),
                state.effort.clone(),
                state.shared.clone(),
            )
        };

        let Some(rpc) = rpc else {
            // Simulated response flow
            tokio::spawn(async move {
                let _ = tx.send(AgentEvent::ThoughtChunk(
                    "Codex app-server analyzing sequence structure...".into(),
                ));
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                let _ = tx.send(AgentEvent::MessageChunk(format!(
                    "Codex app-server response to: '{message}'"
                )));
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

                let _ = tx.send(AgentEvent::SessionCompleted {
                    summary: Some("Codex task finished successfully.".into()),
                });
            });
            return Ok(());
        };

        let mut params = json!({
            "threadId": thread_id,
            "input": [{ "type": "text", "text": message }],
        });
        if let Some(model) = model {
            params["model"] = json!(model);
        }
        if let Some(effort) = effort {
            params["effort"] = json!(effort);
        }
        let turn = rpc.request("turn/start", params).await?;
        if let Some(turn_id) = turn
            .get("turn")
            .and_then(|turn| turn.get("id"))
            .and_then(Value::as_str)
        {
            *shared.current_turn.lock().unwrap() = Some(turn_id.to_string());
        }
        Ok(())
    }

    async fn submit_approval(
        &self,
        session: SessionId,
        request_id: String,
        approved: bool,
    ) -> Result<(), AgentRuntimeError> {
        let (rpc, tx, shared) = {
            let map = self.sessions.lock().unwrap();
            let state = map
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            (state.rpc.clone(), state.tx.clone(), state.shared.clone())
        };

        let Some(rpc) = rpc else {
            let _ = tx.send(AgentEvent::ApprovalDecision {
                request_id,
                approved,
            });
            return Ok(());
        };

        let kind = shared
            .approvals
            .lock()
            .unwrap()
            .remove(&request_id)
            .ok_or_else(|| {
                AgentRuntimeError::Other(format!(
                    "No pending approval found for request '{request_id}'"
                ))
            })?;
        let result = match &kind {
            ApprovalKind::CommandExecution | ApprovalKind::FileChange => {
                json!({ "decision": if approved { "accept" } else { "decline" } })
            }
            ApprovalKind::Permissions(profile) => {
                if approved {
                    json!({ "permissions": profile, "scope": "turn" })
                } else {
                    json!({
                        "permissions": { "fileSystem": Value::Null, "network": Value::Null },
                        "scope": "turn",
                    })
                }
            }
            ApprovalKind::LegacyExec | ApprovalKind::LegacyPatch => {
                json!({ "decision": if approved { "approved" } else { "abort" } })
            }
        };
        let id: u64 = request_id.parse().map_err(|_| {
            AgentRuntimeError::ProtocolError(format!("Invalid approval request id '{request_id}'"))
        })?;
        let response = CodexRpcResultResponse::new(id, result);
        if let Err(error) = rpc.send_value(serde_json::to_value(&response)?).await {
            shared.approvals.lock().unwrap().insert(request_id, kind);
            return Err(error);
        }
        let _ = tx.send(AgentEvent::ApprovalDecision {
            request_id,
            approved,
        });
        Ok(())
    }

    async fn cancel(&self, session: SessionId) -> Result<(), AgentRuntimeError> {
        let (rpc, tx, thread_id, shared, child) = {
            let mut map = self.sessions.lock().unwrap();
            let state = map
                .get_mut(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            (
                state.rpc.clone(),
                state.tx.clone(),
                state.thread_id.clone(),
                state.shared.clone(),
                state.child.take(),
            )
        };

        if let Some(rpc) = rpc {
            let turn_id = shared.current_turn.lock().unwrap().clone();
            if let Some(turn_id) = turn_id {
                let interrupted = tokio::time::timeout(
                    INTERRUPT_TIMEOUT,
                    rpc.request(
                        "turn/interrupt",
                        json!({ "threadId": thread_id, "turnId": turn_id }),
                    ),
                )
                .await;
                match interrupted {
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => {
                        log::debug!("Could not interrupt the Codex turn: {error:?}");
                    }
                    Err(_) => {
                        log::debug!("Codex did not acknowledge the turn interrupt in time");
                    }
                }
            }
        }
        if let Some(mut child) = child {
            let _ = child.kill().await;
        }
        let _ = tx.send(AgentEvent::SessionCompleted {
            summary: Some("Session cancelled by user".into()),
        });
        Ok(())
    }

    fn subscribe(
        &self,
        session: SessionId,
    ) -> Result<broadcast::Receiver<AgentEvent>, AgentRuntimeError> {
        let map = self.sessions.lock().unwrap();
        let state = map
            .get(&session)
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;
        Ok(state.tx.subscribe())
    }
}

// Private APIs only
impl CodexAgentProvider {
    fn launch(
        &self,
        config: &AgentConfig,
    ) -> Result<(Child, RpcChannel, ChildStdout), AgentRuntimeError> {
        let mut cmd = Command::new(&self.binary);
        cmd.args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }
        for (key, value) in &config.environment {
            cmd.env(key, value);
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
        // Drain stderr so a chatty server can never fill the pipe and stall.
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    log::debug!("codex stderr: {line}");
                }
            });
        }

        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(32);
        tokio::spawn(async move {
            let mut writer = stdin;
            while let Some(msg) = stdin_rx.recv().await {
                if writer.write_all(msg.as_bytes()).await.is_err() || writer.flush().await.is_err()
                {
                    break;
                }
            }
        });
        let rpc = RpcChannel {
            stdin_tx,
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        };
        Ok((child, rpc, stdout))
    }
}

// Private APIs only
impl RpcChannel {
    async fn request(&self, method: &str, params: Value) -> Result<Value, AgentRuntimeError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (reply_tx, reply_rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, reply_tx);
        let req = CodexRpcRequest::new(id, method, Some(params));
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');
        if self.stdin_tx.send(line).await.is_err() {
            self.pending.lock().unwrap().remove(&id);
            return Err(AgentRuntimeError::ProtocolError(
                "Codex stdin channel closed".into(),
            ));
        }
        let reply = match tokio::time::timeout(RPC_TIMEOUT, reply_rx).await {
            Ok(Ok(reply)) => reply,
            Ok(Err(_)) => {
                return Err(AgentRuntimeError::ProtocolError(format!(
                    "Codex dropped the reply to '{method}'"
                )));
            }
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                return Err(AgentRuntimeError::ProtocolError(format!(
                    "Codex did not answer '{method}' within {}s",
                    RPC_TIMEOUT.as_secs()
                )));
            }
        };
        if let Some(error) = reply.get("error") {
            return Err(AgentRuntimeError::JsonRpcError {
                code: error.get("code").and_then(Value::as_i64).unwrap_or(0),
                message: error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
                    .to_string(),
                data: error.get("data").cloned(),
            });
        }
        Ok(reply.get("result").cloned().unwrap_or(Value::Null))
    }

    async fn send_value(&self, value: Value) -> Result<(), AgentRuntimeError> {
        let mut line = serde_json::to_string(&value)?;
        line.push('\n');
        self.stdin_tx
            .send(line)
            .await
            .map_err(|_| AgentRuntimeError::ProtocolError("Codex stdin channel closed".into()))
    }
}

async fn initialize(rpc: &RpcChannel) -> Result<(), AgentRuntimeError> {
    rpc.request(
        "initialize",
        json!({
            "clientInfo": {
                "name": "nodavideo",
                "title": "NodaVideo",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {},
        }),
    )
    .await?;
    let notification = CodexRpcNotification {
        jsonrpc: "2.0".into(),
        method: "initialized".into(),
        params: None,
    };
    rpc.send_value(serde_json::to_value(&notification)?).await
}

async fn default_model_id(rpc: &RpcChannel) -> Result<String, AgentRuntimeError> {
    let list = rpc.request("model/list", json!({})).await?;
    let models = parse_models(&list);
    let chosen = match models.iter().find(|model| model.is_default) {
        Some(model) => model,
        None => match models.first() {
            Some(model) => model,
            None => {
                return Err(AgentRuntimeError::Other(
                    "Codex reported no usable models".into(),
                ));
            }
        },
    };
    Ok(chosen.id.clone())
}

fn parse_models(list: &Value) -> Vec<AgentModel> {
    let Some(data) = list.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    data.iter()
        .filter(|model| model.get("hidden").and_then(Value::as_bool) != Some(true))
        .filter_map(|model| {
            let id = model.get("id").and_then(Value::as_str)?;
            let display_name = model
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or(id);
            let efforts = match model
                .get("supportedReasoningEfforts")
                .and_then(Value::as_array)
            {
                Some(options) => options
                    .iter()
                    .filter_map(|option| {
                        option
                            .get("reasoningEffort")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .collect(),
                None => Vec::new(),
            };
            Some(AgentModel {
                id: id.to_string(),
                display_name: display_name.to_string(),
                description: model
                    .get("description")
                    .and_then(Value::as_str)
                    .filter(|description| !description.is_empty())
                    .map(str::to_string),
                efforts,
                default_effort: model
                    .get("defaultReasoningEffort")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                is_default: model
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn spawn_reader(
    stdout: ChildStdout,
    tx: broadcast::Sender<AgentEvent>,
    rpc: RpcChannel,
    shared: Arc<SharedSession>,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            let Some(method) = value.get("method").and_then(Value::as_str) else {
                if let Some(id) = value.get("id").and_then(Value::as_u64) {
                    let pending_reply = rpc.pending.lock().unwrap().remove(&id);
                    if let Some(reply) = pending_reply {
                        let _ = reply.send(value);
                    }
                }
                continue;
            };
            let params = value.get("params").cloned().unwrap_or(Value::Null);
            match value.get("id").and_then(Value::as_u64) {
                Some(id) => {
                    handle_server_request(method, id, &params, &tx, &rpc, &shared).await;
                }
                None => handle_notification(method, &params, &tx, &shared),
            }
        }
    });
}

fn handle_notification(
    method: &str,
    params: &Value,
    tx: &broadcast::Sender<AgentEvent>,
    shared: &SharedSession,
) {
    match method {
        "item/agentMessage/delta" => {
            if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                let _ = tx.send(AgentEvent::MessageChunk(delta.to_string()));
            }
        }
        "item/reasoning/textDelta" | "item/reasoning/summaryTextDelta" => {
            if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                let _ = tx.send(AgentEvent::ThoughtChunk(delta.to_string()));
            }
        }
        "item/started" => {
            if let Some(item) = params.get("item") {
                emit_item_started(item, tx);
            }
        }
        "item/completed" => {
            if let Some(item) = params.get("item") {
                emit_item_completed(item, tx);
            }
        }
        "turn/started" => {
            if let Some(turn_id) = params
                .get("turn")
                .and_then(|turn| turn.get("id"))
                .and_then(Value::as_str)
            {
                *shared.current_turn.lock().unwrap() = Some(turn_id.to_string());
            }
        }
        "turn/completed" => {
            *shared.current_turn.lock().unwrap() = None;
            let Some(turn) = params.get("turn") else {
                return;
            };
            let status = turn.get("status").and_then(Value::as_str).unwrap_or("");
            match status {
                "completed" => {
                    let _ = tx.send(AgentEvent::SessionCompleted { summary: None });
                }
                "interrupted" => {
                    let _ = tx.send(AgentEvent::SessionCompleted {
                        summary: Some("Turn interrupted.".into()),
                    });
                }
                "failed" => {
                    let message = turn
                        .get("error")
                        .and_then(|error| error.get("message"))
                        .and_then(Value::as_str)
                        .unwrap_or("The turn failed.")
                        .to_string();
                    let _ = tx.send(AgentEvent::SessionError {
                        code: "CODEX_TURN_FAILED".into(),
                        message,
                    });
                }
                other => {
                    let _ = tx.send(AgentEvent::SessionError {
                        code: "CODEX_TURN_STATUS".into(),
                        message: format!("Unexpected turn status '{other}'"),
                    });
                }
            }
        }
        "error" => {
            let message = params
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Unknown Codex error")
                .to_string();
            let will_retry = params
                .get("willRetry")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if will_retry {
                let _ = tx.send(AgentEvent::ThoughtChunk(format!(
                    "Codex retrying after error: {message}"
                )));
            } else {
                let _ = tx.send(AgentEvent::SessionError {
                    code: "CODEX_ERROR".into(),
                    message,
                });
            }
        }
        "warning" => {
            if let Some(message) = params.get("message").and_then(Value::as_str) {
                let _ = tx.send(AgentEvent::ThoughtChunk(format!(
                    "Codex warning: {message}"
                )));
            }
        }
        "configWarning" => {
            if let Some(summary) = params.get("summary").and_then(Value::as_str) {
                let _ = tx.send(AgentEvent::ThoughtChunk(format!("Codex config: {summary}")));
            }
        }
        _ => {}
    }
}

fn emit_item_started(item: &Value, tx: &broadcast::Sender<AgentEvent>) {
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
    let call_id = item
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    match item_type {
        "commandExecution" => {
            let _ = tx.send(AgentEvent::ToolCallRequested {
                call_id,
                tool_name: "shell".into(),
                arguments: json!({
                    "command": item.get("command").cloned().unwrap_or(Value::Null),
                    "cwd": item.get("cwd").cloned().unwrap_or(Value::Null),
                }),
            });
        }
        "mcpToolCall" => {
            let server = item.get("server").and_then(Value::as_str).unwrap_or("mcp");
            let tool = item.get("tool").and_then(Value::as_str).unwrap_or("tool");
            let _ = tx.send(AgentEvent::ToolCallRequested {
                call_id,
                tool_name: format!("{server}/{tool}"),
                arguments: item.get("arguments").cloned().unwrap_or_else(|| json!({})),
            });
        }
        "fileChange" => {
            let _ = tx.send(AgentEvent::ToolCallRequested {
                call_id,
                tool_name: "apply_patch".into(),
                arguments: json!({
                    "changes": item.get("changes").cloned().unwrap_or(Value::Null),
                }),
            });
        }
        "webSearch" => {
            let _ = tx.send(AgentEvent::ToolCallRequested {
                call_id,
                tool_name: "web_search".into(),
                arguments: json!({
                    "query": item.get("query").cloned().unwrap_or(Value::Null),
                }),
            });
        }
        _ => {}
    }
}

fn emit_item_completed(item: &Value, tx: &broadcast::Sender<AgentEvent>) {
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
    let call_id = item
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let status = item.get("status").and_then(Value::as_str).unwrap_or("");
    match item_type {
        "commandExecution" => {
            let exit_code = item.get("exitCode").and_then(Value::as_i64);
            let failed = status == "failed" || exit_code.is_some_and(|code| code != 0);
            let _ = tx.send(AgentEvent::ToolCallResult {
                call_id,
                tool_name: "shell".into(),
                result: json!({
                    "output": item.get("aggregatedOutput").cloned().unwrap_or(Value::Null),
                    "exitCode": exit_code,
                }),
                is_error: failed,
            });
        }
        "mcpToolCall" => {
            let server = item.get("server").and_then(Value::as_str).unwrap_or("mcp");
            let tool = item.get("tool").and_then(Value::as_str).unwrap_or("tool");
            let error = item.get("error").filter(|error| !error.is_null());
            let failed = error.is_some() || status == "failed";
            let result = match error {
                Some(error) => json!({ "error": error }),
                None => item.get("result").cloned().unwrap_or(Value::Null),
            };
            let _ = tx.send(AgentEvent::ToolCallResult {
                call_id,
                tool_name: format!("{server}/{tool}"),
                result,
                is_error: failed,
            });
        }
        "fileChange" => {
            let _ = tx.send(AgentEvent::ToolCallResult {
                call_id,
                tool_name: "apply_patch".into(),
                result: json!({
                    "changes": item.get("changes").cloned().unwrap_or(Value::Null),
                    "status": status,
                }),
                is_error: status == "failed",
            });
        }
        "webSearch" => {
            let _ = tx.send(AgentEvent::ToolCallResult {
                call_id,
                tool_name: "web_search".into(),
                result: json!({
                    "results": item.get("results").cloned().unwrap_or(Value::Null),
                }),
                is_error: false,
            });
        }
        _ => {}
    }
}

async fn handle_server_request(
    method: &str,
    id: u64,
    params: &Value,
    tx: &broadcast::Sender<AgentEvent>,
    rpc: &RpcChannel,
    shared: &SharedSession,
) {
    let request_id = id.to_string();
    let reason = params
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let kind = match method {
        "item/commandExecution/requestApproval" => {
            let command = params
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let _ = tx.send(AgentEvent::ApprovalRequested {
                request_id: request_id.clone(),
                title: "Run command".into(),
                description: reason.clone(),
                tool_name: "shell".into(),
                command_summary: command,
                parameters: params.clone(),
            });
            ApprovalKind::CommandExecution
        }
        "item/fileChange/requestApproval" => {
            let summary = if reason.is_empty() {
                "Review the proposed file changes.".to_string()
            } else {
                reason.clone()
            };
            let _ = tx.send(AgentEvent::ApprovalRequested {
                request_id: request_id.clone(),
                title: "Apply file changes".into(),
                description: reason.clone(),
                tool_name: "apply_patch".into(),
                command_summary: summary,
                parameters: params.clone(),
            });
            ApprovalKind::FileChange
        }
        "item/permissions/requestApproval" => {
            let profile = params
                .get("permissions")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let summary = if reason.is_empty() {
                "The agent is asking for additional sandbox permissions.".to_string()
            } else {
                reason.clone()
            };
            let _ = tx.send(AgentEvent::ApprovalRequested {
                request_id: request_id.clone(),
                title: "Grant permissions".into(),
                description: reason.clone(),
                tool_name: "permissions".into(),
                command_summary: summary,
                parameters: profile.clone(),
            });
            ApprovalKind::Permissions(profile)
        }
        "execCommandApproval" => {
            let command = match params.get("command").and_then(Value::as_array) {
                Some(parts) => parts
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" "),
                None => String::new(),
            };
            let _ = tx.send(AgentEvent::ApprovalRequested {
                request_id: request_id.clone(),
                title: "Run command".into(),
                description: reason.clone(),
                tool_name: "shell".into(),
                command_summary: command,
                parameters: params.clone(),
            });
            ApprovalKind::LegacyExec
        }
        "applyPatchApproval" => {
            let files = match params.get("fileChanges").and_then(Value::as_object) {
                Some(changes) => changes.keys().cloned().collect::<Vec<_>>().join(", "),
                None => String::new(),
            };
            let _ = tx.send(AgentEvent::ApprovalRequested {
                request_id: request_id.clone(),
                title: "Apply file changes".into(),
                description: reason.clone(),
                tool_name: "apply_patch".into(),
                command_summary: files,
                parameters: params.clone(),
            });
            ApprovalKind::LegacyPatch
        }
        other => {
            // Answer unsupported server requests with an error so the server never waits
            // forever for a reply NodaVideo will not send.
            log::debug!("Unsupported Codex server request '{other}'");
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("NodaVideo does not implement '{other}'"),
                },
            });
            if let Err(error) = rpc.send_value(response).await {
                log::debug!("Could not reply to the Codex server request: {error:?}");
            }
            return;
        }
    };
    shared.approvals.lock().unwrap().insert(request_id, kind);
}
