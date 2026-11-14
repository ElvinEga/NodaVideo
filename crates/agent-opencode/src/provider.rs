//! OpenCode AgentProvider adapter backed by a local `opencode serve` HTTP server.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use nodavideo_agent_runtime::{
    AgentConfig, AgentEvent, AgentModel, AgentProvider, AgentRuntimeError, SessionId,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::broadcast;

const SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const RPC_TIMEOUT: Duration = Duration::from_secs(60);
const INTERRUPT_TIMEOUT: Duration = Duration::from_secs(5);

/// A running `opencode serve` process shared by every session on this provider.
struct ServerHandle {
    base_url: String,
    password: String,
    client: reqwest::Client,
    child: Mutex<Child>,
}

struct ActiveOpenCodeSession {
    opencode_session_id: String,
    server: Arc<ServerHandle>,
    tx: broadcast::Sender<AgentEvent>,
    pump: tokio::task::JoinHandle<()>,
}

/// AgentProvider adapter communicating with a local OpenCode server.
pub struct OpenCodeAgentProvider {
    id: String,
    display_name: String,
    binary: String,
    sessions: Arc<Mutex<HashMap<SessionId, ActiveOpenCodeSession>>>,
    server: Arc<tokio::sync::Mutex<Option<Arc<ServerHandle>>>>,
}

impl std::fmt::Debug for OpenCodeAgentProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenCodeAgentProvider")
            .field("id", &self.id)
            .field("display_name", &self.display_name)
            .field("binary", &self.binary)
            .finish_non_exhaustive()
    }
}

impl Default for OpenCodeAgentProvider {
    fn default() -> Self {
        Self::new("opencode", "OpenCode", "opencode")
    }
}

// Public APIs only
impl OpenCodeAgentProvider {
    /// Create a new OpenCode provider instance.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        binary: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            binary: binary.into(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            server: Arc::new(tokio::sync::Mutex::new(None)),
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
impl AgentProvider for OpenCodeAgentProvider {
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
        if !Self::probe_binary_exists(&self.binary) {
            return Err(AgentRuntimeError::ProviderUnavailable(
                self.id.clone(),
                format!("'{}' was not found on PATH", self.binary),
            ));
        }
        let server = self.ensure_server().await?;
        let models = server.get("/api/model").await?;
        let default = server.get("/api/model/default").await?;
        Ok(parse_models(&models, &default))
    }

    async fn start_session(&self, config: AgentConfig) -> Result<SessionId, AgentRuntimeError> {
        if !Self::probe_binary_exists(&self.binary) {
            return Err(AgentRuntimeError::ProviderUnavailable(
                self.id.clone(),
                format!("'{}' was not found on PATH", self.binary),
            ));
        }
        let server = self.ensure_server().await?;
        let model =
            resolve_model_ref(&server, config.model.as_deref(), config.effort.as_deref()).await?;
        let mut params = json!({
            "title": "NodaVideo",
            "model": model,
            "permissions": [
                { "action": "shell", "resource": "*", "effect": "ask" },
                { "action": "edit", "resource": "*", "effect": "ask" },
            ],
        });
        if let Some(dir) = &config.working_dir {
            params["location"] = json!({ "directory": dir.to_string_lossy() });
        }
        let created = server.post("/api/session", params).await?;
        let opencode_session_id = created
            .get("data")
            .and_then(|data| data.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AgentRuntimeError::ProtocolError("OpenCode session create returned no id".into())
            })?
            .to_string();

        let session_id = SessionId::new();
        let (tx, _) = broadcast::channel(128);
        let pump = spawn_event_pump(server.clone(), opencode_session_id.clone(), tx.clone());
        let mut map = self.sessions.lock().unwrap();
        map.insert(
            session_id,
            ActiveOpenCodeSession {
                opencode_session_id,
                server,
                tx,
                pump,
            },
        );
        Ok(session_id)
    }

    async fn send_message(
        &self,
        session: SessionId,
        message: String,
    ) -> Result<(), AgentRuntimeError> {
        let (server, opencode_session_id) = {
            let map = self.sessions.lock().unwrap();
            let state = map
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            (state.server.clone(), state.opencode_session_id.clone())
        };
        server
            .post(
                &format!("/api/session/{opencode_session_id}/prompt"),
                json!({ "text": message }),
            )
            .await?;
        Ok(())
    }

    async fn submit_approval(
        &self,
        session: SessionId,
        request_id: String,
        approved: bool,
    ) -> Result<(), AgentRuntimeError> {
        let (server, opencode_session_id, tx) = {
            let map = self.sessions.lock().unwrap();
            let state = map
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            (
                state.server.clone(),
                state.opencode_session_id.clone(),
                state.tx.clone(),
            )
        };
        let decision = if approved {
            "once"
        } else {
            "reject"
        };
        server
            .post(
                &format!("/api/session/{opencode_session_id}/permission/{request_id}/reply"),
                json!({ "decision": decision }),
            )
            .await?;
        let _ = tx.send(AgentEvent::ApprovalDecision {
            request_id,
            approved,
        });
        Ok(())
    }

    async fn cancel(&self, session: SessionId) -> Result<(), AgentRuntimeError> {
        let state = {
            let mut map = self.sessions.lock().unwrap();
            map.remove(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?
        };
        state.pump.abort();
        let interrupted = tokio::time::timeout(
            INTERRUPT_TIMEOUT,
            state.server.post(
                &format!("/api/session/{}/interrupt", state.opencode_session_id),
                json!({}),
            ),
        )
        .await;
        match interrupted {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                log::debug!("Could not interrupt the OpenCode session: {error:?}");
            }
            Err(_) => {
                log::debug!("OpenCode did not acknowledge the interrupt in time");
            }
        }
        let _ = state.tx.send(AgentEvent::SessionCompleted {
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
impl OpenCodeAgentProvider {
    async fn ensure_server(&self) -> Result<Arc<ServerHandle>, AgentRuntimeError> {
        let mut guard = self.server.lock().await;
        if let Some(handle) = guard.as_ref() {
            let alive = matches!(handle.child.lock().unwrap().try_wait(), Ok(None));
            if alive {
                return Ok(handle.clone());
            }
        }
        let new_handle = Arc::new(launch_server(&self.binary).await?);
        *guard = Some(new_handle.clone());
        Ok(new_handle)
    }
}

// Private APIs only
impl ServerHandle {
    async fn get(&self, path: &str) -> Result<Value, AgentRuntimeError> {
        self.request(reqwest::Method::GET, path, None).await
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, AgentRuntimeError> {
        self.request(reqwest::Method::POST, path, Some(body)).await
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, AgentRuntimeError> {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.base_url))
            .basic_auth("opencode", Some(&self.password))
            .timeout(RPC_TIMEOUT);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.map_err(|error| {
            AgentRuntimeError::ProtocolError(format!(
                "OpenCode server request to '{path}' failed: {error}"
            ))
        })?;
        let status = response.status();
        let text = response.text().await.map_err(|error| {
            AgentRuntimeError::ProtocolError(format!(
                "Could not read the OpenCode response for '{path}': {error}"
            ))
        })?;
        if !status.is_success() {
            return Err(AgentRuntimeError::ProtocolError(format!(
                "OpenCode returned {status} for '{path}': {text}"
            )));
        }
        if text.is_empty() {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_str(&text)?)
    }

    async fn event_response(&self) -> Result<reqwest::Response, AgentRuntimeError> {
        let response = self
            .client
            .get(format!("{}/api/event", self.base_url))
            .basic_auth("opencode", Some(&self.password))
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .send()
            .await
            .map_err(|error| {
                AgentRuntimeError::ProtocolError(format!(
                    "Could not open the OpenCode event stream: {error}"
                ))
            })?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AgentRuntimeError::ProtocolError(format!(
                "OpenCode returned {status} for '/api/event': {text}"
            )));
        }
        Ok(response)
    }
}

async fn launch_server(binary: &str) -> Result<ServerHandle, AgentRuntimeError> {
    let password = uuid::Uuid::new_v4().to_string();
    let mut cmd = Command::new(binary);
    cmd.args(["serve", "--hostname", "127.0.0.1", "--port", "0"])
        .env("OPENCODE_PASSWORD", &password)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd
        .spawn()
        .map_err(|source| AgentRuntimeError::ProcessFailed {
            program: binary.to_string(),
            source,
        })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        AgentRuntimeError::ProtocolError("Failed to capture OpenCode stdout".into())
    })?;
    // Drain stderr so a chatty server can never fill the pipe and stall.
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::debug!("opencode stderr: {line}");
            }
        });
    }
    let base_url = tokio::time::timeout(SERVER_START_TIMEOUT, read_server_url(stdout))
        .await
        .map_err(|_| {
            AgentRuntimeError::ProtocolError(format!(
                "OpenCode server did not start within {}s",
                SERVER_START_TIMEOUT.as_secs()
            ))
        })??;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| {
            AgentRuntimeError::ProtocolError(format!(
                "Could not build the OpenCode HTTP client: {error}"
            ))
        })?;
    Ok(ServerHandle {
        base_url,
        password,
        client,
        child: Mutex::new(child),
    })
}

async fn read_server_url(stdout: ChildStdout) -> Result<String, AgentRuntimeError> {
    let mut lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if let Some(url) = trimmed.strip_prefix("server listening on ") {
            let url = url.trim().to_string();
            tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    log::debug!("opencode stdout: {line}");
                }
            });
            return Ok(url);
        }
        log::debug!("opencode stdout: {trimmed}");
    }
    Err(AgentRuntimeError::ProtocolError(
        "OpenCode server exited before reporting its address".into(),
    ))
}

async fn resolve_model_ref(
    server: &ServerHandle,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Value, AgentRuntimeError> {
    let mut reference = if let Some(model) = model {
        let Some((provider_id, model_id)) = model.split_once('/') else {
            return Err(AgentRuntimeError::ProtocolError(format!(
                "OpenCode model '{model}' must be in 'provider/model' form"
            )));
        };
        json!({ "providerID": provider_id, "id": model_id })
    } else {
        let default = server.get("/api/model/default").await?;
        let data = default
            .get("data")
            .filter(|data| !data.is_null())
            .ok_or_else(|| AgentRuntimeError::Other("OpenCode reported no default model".into()))?;
        let provider_id = data
            .get("providerID")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AgentRuntimeError::ProtocolError("OpenCode default model has no providerID".into())
            })?;
        let model_id = data.get("id").and_then(Value::as_str).ok_or_else(|| {
            AgentRuntimeError::ProtocolError("OpenCode default model has no id".into())
        })?;
        json!({ "providerID": provider_id, "id": model_id })
    };
    if let Some(variant) = effort {
        reference["variant"] = json!(variant);
    }
    Ok(reference)
}

fn parse_models(models: &Value, default: &Value) -> Vec<AgentModel> {
    let Some(data) = models.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    let default_data = default.get("data").filter(|data| !data.is_null());
    let default_key = default_data.map(|data| {
        (
            data.get("providerID").and_then(Value::as_str).unwrap_or(""),
            data.get("id").and_then(Value::as_str).unwrap_or(""),
        )
    });
    let mut parsed: Vec<AgentModel> = data
        .iter()
        .filter(|model| model.get("enabled").and_then(Value::as_bool) != Some(false))
        .filter_map(|model| {
            let provider_id = model.get("providerID").and_then(Value::as_str)?;
            let id = model.get("id").and_then(Value::as_str)?;
            let display_name = model.get("name").and_then(Value::as_str).unwrap_or(id);
            let efforts = match model.get("variants").and_then(Value::as_array) {
                Some(variants) => variants
                    .iter()
                    .filter_map(|variant| {
                        variant
                            .get("id")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                    .collect(),
                None => Vec::new(),
            };
            Some(AgentModel {
                id: format!("{provider_id}/{id}"),
                display_name: display_name.to_string(),
                description: None,
                efforts,
                default_effort: None,
                is_default: default_key == Some((provider_id, id)),
            })
        })
        .collect();
    parsed.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
    });
    parsed
}

fn spawn_event_pump(
    server: Arc<ServerHandle>,
    opencode_session_id: String,
    tx: broadcast::Sender<AgentEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let response = match server.event_response().await {
            Ok(response) => response,
            Err(error) => {
                let _ = tx.send(AgentEvent::SessionError {
                    code: "OPENCODE_STREAM".into(),
                    message: error.to_string(),
                });
                return;
            }
        };
        let mut tool_names: HashMap<String, String> = HashMap::new();
        let mut buffer = String::new();
        let mut stream = std::pin::pin!(response.bytes_stream());
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    log::debug!("OpenCode event stream failed: {error:?}");
                    break;
                }
            };
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(index) = buffer.find('\n') {
                let line = buffer[..index].trim_end_matches('\r').to_string();
                buffer.drain(..=index);
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data.is_empty() {
                    continue;
                }
                let Ok(event) = serde_json::from_str::<Value>(data) else {
                    continue;
                };
                handle_event(&event, &opencode_session_id, &tx, &mut tool_names);
            }
        }
        if tx.receiver_count() > 0 {
            let _ = tx.send(AgentEvent::SessionError {
                code: "OPENCODE_DISCONNECTED".into(),
                message: "The OpenCode event stream ended.".into(),
            });
        }
    })
}

fn handle_event(
    event: &Value,
    session_id: &str,
    tx: &broadcast::Sender<AgentEvent>,
    tool_names: &mut HashMap<String, String>,
) {
    let Some(event_type) = event.get("type").and_then(Value::as_str) else {
        return;
    };
    let data = event.get("data").cloned().unwrap_or(Value::Null);
    if data.get("sessionID").and_then(Value::as_str) != Some(session_id) {
        return;
    }
    match event_type {
        "session.text.delta" => {
            if let Some(delta) = data.get("delta").and_then(Value::as_str) {
                let _ = tx.send(AgentEvent::MessageChunk(delta.to_string()));
            }
        }
        "session.reasoning.delta" => {
            if let Some(delta) = data.get("delta").and_then(Value::as_str) {
                let _ = tx.send(AgentEvent::ThoughtChunk(delta.to_string()));
            }
        }
        "session.tool.input.started" => {
            if let (Some(id), Some(name)) = (
                data.get("id").and_then(Value::as_str),
                data.get("name").and_then(Value::as_str),
            ) {
                tool_names.insert(id.to_string(), name.to_string());
            }
        }
        "session.tool.called" => {
            let Some(call_id) = data.get("id").and_then(Value::as_str) else {
                return;
            };
            let tool_name = tool_names
                .get(call_id)
                .cloned()
                .unwrap_or_else(|| tool_name_from_id(call_id));
            let _ = tx.send(AgentEvent::ToolCallRequested {
                call_id: call_id.to_string(),
                tool_name,
                arguments: data.get("input").cloned().unwrap_or_else(|| json!({})),
            });
        }
        "session.tool.success" | "session.tool.failed" => {
            let Some(call_id) = data.get("id").and_then(Value::as_str) else {
                return;
            };
            let tool_name = tool_names
                .get(call_id)
                .cloned()
                .unwrap_or_else(|| tool_name_from_id(call_id));
            let _ = tx.send(AgentEvent::ToolCallResult {
                call_id: call_id.to_string(),
                tool_name,
                result: json!({
                    "content": data.get("content").cloned().unwrap_or(Value::Null),
                    "metadata": data.get("metadata").cloned().unwrap_or(Value::Null),
                }),
                is_error: event_type == "session.tool.failed",
            });
        }
        "permission.asked" => {
            let Some(request_id) = data.get("id").and_then(Value::as_str) else {
                return;
            };
            let action = data.get("action").and_then(Value::as_str).unwrap_or("tool");
            let resources = match data.get("resources").and_then(Value::as_array) {
                Some(resources) => resources
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("\n"),
                None => String::new(),
            };
            let title = match action {
                "shell" => "Run command".to_string(),
                "edit" => "Edit files".to_string(),
                other => format!("Allow {other}"),
            };
            let _ = tx.send(AgentEvent::ApprovalRequested {
                request_id: request_id.to_string(),
                title,
                description: "OpenCode is waiting for your approval to continue.".into(),
                tool_name: action.to_string(),
                command_summary: resources,
                parameters: data.clone(),
            });
        }
        "session.execution.succeeded" | "session.execution.interrupted" => {
            let _ = tx.send(AgentEvent::SessionCompleted { summary: None });
        }
        "session.execution.failed" => {
            let error = data.get("error");
            let code = error
                .and_then(|error| error.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("OPENCODE_EXECUTION_FAILED")
                .to_string();
            let message = error
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("The OpenCode execution failed.")
                .to_string();
            let _ = tx.send(AgentEvent::SessionError { code, message });
        }
        // permission.replied is answered by submit_approval, which emits the decision itself.
        _ => {}
    }
}

fn tool_name_from_id(call_id: &str) -> String {
    let name = call_id.strip_prefix("functions.").unwrap_or(call_id);
    name.split(':').next().unwrap_or(name).to_string()
}
