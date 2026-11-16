//! Native Qoder daemon sessions with host-owned permissions and editor MCP tools.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use nodavideo_agent_runtime::{
    AgentConfig, AgentEvent, AgentModel, AgentProvider, AgentRuntimeError, PermissionMode,
    SessionId,
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::host::{MCP_SERVER, SessionHost};
use crate::protocol::{
    AUTH_PAYLOAD_QODERCLI, ControlResponse, DAEMON_PROTOCOL_VERSION, ENV_SDK_AUTH_PAYLOAD_FILE,
    ENV_SDK_DAEMON, ENV_SDK_ENTRYPOINT, ENV_SDK_VERSION, InitializeInfo, MULTI_SESSION_CAPABILITY,
    QoderAccount, QoderSession, SDK_ENTRYPOINT, SDK_VERSION, control_request, parse_account,
    parse_control_response, parse_initialize, parse_models, parse_sessions, parse_version,
    response_payload, response_request_id, session_control_request, user_message,
};

const RPC_TIMEOUT: Duration = Duration::from_secs(60);
const INTERRUPT_TIMEOUT: Duration = Duration::from_secs(5);

/// Correlates daemon control requests with their responses; shared with the reader task.
#[derive(Clone)]
struct DaemonConnection {
    stdin_tx: mpsc::Sender<String>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>,
    next_id: Arc<AtomicU64>,
}

/// One live NodaVideo session backed by a Qoder daemon. The auth dir guard lives
/// here so the payload file survives for the whole session and is removed on drop.
struct ActiveQoderSession {
    native_id: String,
    tx: broadcast::Sender<AgentEvent>,
    child: Option<Child>,
    connection: Option<DaemonConnection>,
    _auth_dir: Option<TempDir>,
    host: Arc<SessionHost>,
}

impl Drop for ActiveQoderSession {
    fn drop(&mut self) {
        self.host.stop();
    }
}

/// AgentProvider adapter for the Qoder CLI SDK daemon.
pub struct QoderAgentProvider {
    id: String,
    display_name: String,
    binary: String,
    sessions: Arc<Mutex<HashMap<SessionId, ActiveQoderSession>>>,
}

impl std::fmt::Debug for QoderAgentProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QoderAgentProvider")
            .field("id", &self.id)
            .field("display_name", &self.display_name)
            .field("binary", &self.binary)
            .finish_non_exhaustive()
    }
}

impl Default for QoderAgentProvider {
    fn default() -> Self {
        Self::new("qoder", "Qoder (CLI)", resolve_binary())
    }
}

// Public APIs only
impl QoderAgentProvider {
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
        }
    }

    /// Construct an offline provider with no binary, for tests and development.
    pub fn simulated() -> Self {
        Self {
            id: "qoder-simulated".into(),
            display_name: "Qoder (Offline Simulation)".into(),
            binary: String::new(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check whether `binary` exists: an absolute path directly, otherwise on PATH.
    pub fn probe_binary_exists(binary: &str) -> bool {
        if binary.is_empty() {
            return false;
        }
        let path = PathBuf::from(binary);
        if path.is_absolute() {
            return path.is_file();
        }
        if let Ok(paths) = std::env::var("PATH") {
            for dir in std::env::split_paths(&paths) {
                if dir.join(binary).is_file() {
                    return true;
                }
            }
        }
        false
    }

    /// The installed CLI version from `qodercli --version`, or None when unavailable.
    pub async fn cli_version(&self) -> Option<String> {
        if !Self::probe_binary_exists(&self.binary) {
            return None;
        }
        let output = Command::new(&self.binary)
            .arg("--version")
            .output()
            .await
            .ok()?;
        parse_version(&String::from_utf8_lossy(&output.stdout))
    }

    /// The signed-in account, or None when no local login is present.
    pub async fn account_info(&self) -> Result<Option<QoderAccount>, AgentRuntimeError> {
        if self.binary.is_empty() {
            return Ok(None);
        }
        if !Self::probe_binary_exists(&self.binary) {
            return Err(AgentRuntimeError::ProviderUnavailable(
                self.id.clone(),
                format!("'{}' was not found on PATH", self.binary),
            ));
        }
        let mut process = connect(&self.binary, &AgentConfig::default(), None).await?;
        let info = process
            .connection
            .request("daemon.account_info", json!({}))
            .await;
        let _ = process.child.kill().await;
        Ok(parse_account(&info?))
    }

    /// Persisted Qoder conversations reported by the daemon, newest first.
    pub async fn list_sessions(&self) -> Result<Vec<QoderSession>, AgentRuntimeError> {
        if self.binary.is_empty() {
            return Ok(Vec::new());
        }
        if !Self::probe_binary_exists(&self.binary) {
            return Err(AgentRuntimeError::ProviderUnavailable(
                self.id.clone(),
                format!("'{}' was not found on PATH", self.binary),
            ));
        }
        let mut process = connect(&self.binary, &AgentConfig::default(), None).await?;
        let listed = process
            .connection
            .request("daemon.list_sessions", json!({}))
            .await;
        let _ = process.child.kill().await;
        Ok(parse_sessions(&listed?))
    }
}

#[async_trait]
impl AgentProvider for QoderAgentProvider {
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
        let mut process = connect(&self.binary, &AgentConfig::default(), None).await?;
        let listed = process
            .connection
            .request("daemon.get_models", json!({ "fetchStrategy": "live" }))
            .await;
        let _ = process.child.kill().await;
        Ok(parse_models(&listed?))
    }

    async fn status_detail(&self) -> Result<Option<String>, AgentRuntimeError> {
        let version = self
            .cli_version()
            .await
            .unwrap_or_else(|| "Unknown version".into());
        let account = self.account_info().await?;
        let auth = if account.is_some() {
            "Signed in"
        } else {
            "Sign in with qodercli login"
        };
        Ok(Some(format!("{version} · {auth}")))
    }

    async fn start_session(&self, config: AgentConfig) -> Result<SessionId, AgentRuntimeError> {
        let session_id = SessionId::new();
        let (tx, _) = broadcast::channel(128);

        let host = Arc::new(SessionHost::new(&config, tx.clone()));
        if self.id != "qoder-simulated" && !Self::probe_binary_exists(&self.binary) {
            return Err(AgentRuntimeError::ProviderUnavailable(
                self.id.clone(),
                "Install qodercli and run `qodercli login` before starting Qoder".into(),
            ));
        }
        if self.id == "qoder-simulated" {
            self.sessions.lock().unwrap().insert(
                session_id,
                ActiveQoderSession {
                    native_id: String::new(),
                    tx,
                    child: None,
                    connection: None,
                    _auth_dir: None,
                    host,
                },
            );
            return Ok(session_id);
        }

        let mut process = connect(&self.binary, &config, Some(host.clone())).await?;
        let connection = process.connection.clone();
        let opened = async {
            // Authentication must populate the worker's model provider before opening it.
            let authentication = connection.request("daemon.authenticate", json!({})).await?;
            if authentication.get("authenticated").and_then(Value::as_bool) != Some(true) {
                return Err(AgentRuntimeError::Other(
                    "Qoder is signed out; run `qodercli login` and try again".into(),
                ));
            }
            // The daemon never inherits a default session directory.
            let cwd = match &config.working_dir {
                Some(dir) => dir.clone(),
                None => std::env::current_dir().map_err(|source| AgentRuntimeError::Io {
                    path: PathBuf::from("."),
                    source,
                })?,
            };
            let params = json!({
                "worker_args": worker_args(&config),
                "cwd": cwd.to_string_lossy(),
            });
            let frame = connection
                .request_frame("daemon.open_session", params)
                .await?;
            let native_id = frame
                .get("session_id")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AgentRuntimeError::ProtocolError(
                        "Qoder open_session returned no session id".into(),
                    )
                })?
                .to_string();
            let model = resolve_session_model(&connection, config.model.as_deref()).await?;
            connection
                .request_session(&native_id, "set_model", json!({ "model": model }))
                .await?;
            let overrides = config
                .editor_tools
                .iter()
                .map(|tool| (tool.name.clone(), json!({"alwaysLoad":true})))
                .collect::<serde_json::Map<String, Value>>();
            let mut params = json!({
                "supportsCatalogReadyInitialize":true,
                "supportsAvailableModelsUpdate":true,
                "supportsCommandsChanged":true,
                "initializeTimeoutMs":60000,
            });
            if !config.editor_tools.is_empty() {
                params["sdkMcpServers"] = json!([MCP_SERVER]);
                params["sdkMcpToolOverrides"] = json!({MCP_SERVER:overrides});
            }
            if let Some(prompt) = &config.system_prompt {
                params["appendSystemPrompt"] = json!(prompt);
            }
            connection
                .request_session(&native_id, "initialize", params)
                .await?;
            Ok::<String, AgentRuntimeError>(native_id)
        }
        .await;

        let native_id = match opened {
            Ok(native_id) => native_id,
            Err(error) => {
                let _ = process.child.kill().await;
                return Err(error);
            }
        };

        self.sessions.lock().unwrap().insert(
            session_id,
            ActiveQoderSession {
                native_id,
                tx,
                child: Some(process.child),
                connection: Some(process.connection),
                _auth_dir: Some(process._auth_dir),
                host,
            },
        );
        Ok(session_id)
    }

    async fn send_message(
        &self,
        session: SessionId,
        message: String,
    ) -> Result<(), AgentRuntimeError> {
        let (connection, tx, native_id) = {
            let map = self.sessions.lock().unwrap();
            let state = map
                .get(&session)
                .ok_or(AgentRuntimeError::SessionNotFound(session))?;
            if *state.host.stopped.borrow() {
                return Err(AgentRuntimeError::Other(
                    "Qoder disconnected; resume the conversation to continue".into(),
                ));
            }
            (
                state.connection.clone(),
                state.tx.clone(),
                state.native_id.clone(),
            )
        };

        let Some(connection) = connection else {
            tokio::spawn(async move {
                let _ = tx.send(AgentEvent::ThoughtChunk(
                    "Qoder analyzing the request...".into(),
                ));
                tokio::time::sleep(Duration::from_millis(10)).await;
                let _ = tx.send(AgentEvent::MessageChunk(format!(
                    "Qoder offline response to: '{message}'"
                )));
                tokio::time::sleep(Duration::from_millis(5)).await;
                let _ = tx.send(AgentEvent::SessionCompleted {
                    summary: Some("Qoder offline task finished.".into()),
                });
            });
            return Ok(());
        };

        connection
            .send_frame(user_message(&native_id, &message))
            .await
    }

    fn native_session_id(&self, session: SessionId) -> Option<String> {
        self.sessions
            .lock()
            .unwrap()
            .get(&session)
            .filter(|state| !state.native_id.is_empty())
            .map(|state| state.native_id.clone())
    }

    async fn submit_approval(
        &self,
        session: SessionId,
        request_id: String,
        approved: bool,
    ) -> Result<(), AgentRuntimeError> {
        let sessions = self.sessions.lock().unwrap();
        let state = sessions
            .get(&session)
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;
        state.host.approve(&request_id, approved)
    }

    async fn submit_tool_result(
        &self,
        session: SessionId,
        call_id: String,
        result: Value,
        is_error: bool,
    ) -> Result<(), AgentRuntimeError> {
        let sessions = self.sessions.lock().unwrap();
        let state = sessions
            .get(&session)
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;
        state.host.tool_result(&call_id, result, is_error)
    }

    async fn cancel(&self, session: SessionId) -> Result<(), AgentRuntimeError> {
        let mut state = self
            .sessions
            .lock()
            .unwrap()
            .remove(&session)
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;
        state.host.stop();
        let connection = state.connection.take();
        let child = state.child.take();
        let tx = state.tx.clone();
        let native_id = state.native_id.clone();

        if let Some(connection) = connection {
            if !native_id.is_empty() {
                let interrupted = tokio::time::timeout(
                    INTERRUPT_TIMEOUT,
                    connection.request_session(&native_id, "interrupt", json!({})),
                )
                .await;
                match interrupted {
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => {
                        log::debug!("Could not interrupt the Qoder turn: {error:?}");
                    }
                    Err(_) => {
                        log::debug!("Qoder did not acknowledge the interrupt in time");
                    }
                }
                let _ = tokio::time::timeout(
                    INTERRUPT_TIMEOUT,
                    connection.request_session(&native_id, "daemon.close_session", json!({})),
                )
                .await;
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
impl DaemonConnection {
    fn next_request_id(&self) -> String {
        format!("nodavideo-{}", self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Send a pre-built envelope and await its `control_response`, returning the
    /// full frame on success so callers can read envelope-level fields.
    async fn call(&self, envelope: Value) -> Result<Value, AgentRuntimeError> {
        let id = envelope
            .get("request_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AgentRuntimeError::ProtocolError("Qoder request envelope has no request_id".into())
            })?
            .to_string();
        let method = envelope
            .get("request")
            .and_then(|request| request.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("request")
            .to_string();
        let (reply_tx, reply_rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id.clone(), reply_tx);
        let mut line = serde_json::to_string(&envelope)?;
        line.push('\n');
        if self.stdin_tx.send(line).await.is_err() {
            self.pending.lock().unwrap().remove(&id);
            return Err(AgentRuntimeError::ProtocolError(
                "Qoder stdin channel closed".into(),
            ));
        }
        let reply = match tokio::time::timeout(RPC_TIMEOUT, reply_rx).await {
            Ok(Ok(reply)) => reply,
            Ok(Err(_)) => {
                return Err(AgentRuntimeError::ProtocolError(format!(
                    "Qoder dropped the reply to '{method}'"
                )));
            }
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                return Err(AgentRuntimeError::ProtocolError(format!(
                    "Qoder did not answer '{method}' within {}s",
                    RPC_TIMEOUT.as_secs()
                )));
            }
        };
        match parse_control_response(&reply) {
            Some(ControlResponse::Success { .. }) => Ok(reply),
            Some(ControlResponse::Error { code, message }) => Err(AgentRuntimeError::Other(
                format!("Qoder '{method}' failed ({code}): {message}"),
            )),
            None => Err(AgentRuntimeError::ProtocolError(
                "Qoder sent an unrecognized control_response".into(),
            )),
        }
    }

    async fn request(&self, subtype: &str, params: Value) -> Result<Value, AgentRuntimeError> {
        let frame = self
            .call(control_request(&self.next_request_id(), subtype, params))
            .await?;
        Ok(response_payload(&frame))
    }

    async fn request_frame(
        &self,
        subtype: &str,
        params: Value,
    ) -> Result<Value, AgentRuntimeError> {
        self.call(control_request(&self.next_request_id(), subtype, params))
            .await
    }

    async fn request_session(
        &self,
        session_id: &str,
        subtype: &str,
        params: Value,
    ) -> Result<Value, AgentRuntimeError> {
        let frame = self
            .call(session_control_request(
                &self.next_request_id(),
                subtype,
                session_id,
                params,
            ))
            .await?;
        Ok(response_payload(&frame))
    }

    /// Write a session frame that expects no reply (a user message).
    async fn send_frame(&self, frame: Value) -> Result<(), AgentRuntimeError> {
        let mut line = serde_json::to_string(&frame)?;
        line.push('\n');
        if self.stdin_tx.send(line).await.is_err() {
            return Err(AgentRuntimeError::ProtocolError(
                "Qoder stdin channel closed".into(),
            ));
        }
        Ok(())
    }
}

/// A running daemon plus the auth payload dir it reads; dropping it cleans up.
struct DaemonProcess {
    child: Child,
    connection: DaemonConnection,
    _auth_dir: TempDir,
}

/// Launch the daemon, wire its stdio, and complete the handshake. `tx` is Some for
/// a session (frames stream to it) and None for one-shot discovery calls.
async fn connect(
    binary: &str,
    config: &AgentConfig,
    host: Option<Arc<SessionHost>>,
) -> Result<DaemonProcess, AgentRuntimeError> {
    let auth_dir = write_auth_payload()?;
    let payload_path = auth_dir.path().join("payload.json");
    let (mut child, connection, stdout) = launch(binary, config, &payload_path)?;
    spawn_reader(stdout, connection.clone(), host);
    match initialize(&connection).await {
        Ok(_info) => Ok(DaemonProcess {
            child,
            connection,
            _auth_dir: auth_dir,
        }),
        Err(error) => {
            let _ = child.kill().await;
            Err(error)
        }
    }
}

/// Write the `{type:"qodercli"}` auth payload to a private temp file. It carries
/// no token: the daemon reuses the local login read-only. The dir is removed on drop.
fn write_auth_payload() -> Result<TempDir, AgentRuntimeError> {
    let dir = tempfile::Builder::new()
        .prefix("qoder-sdk-auth-")
        .tempdir()
        .map_err(|source| AgentRuntimeError::Io {
            path: std::env::temp_dir(),
            source,
        })?;
    let path = dir.path().join("payload.json");
    std::fs::write(&path, AUTH_PAYLOAD_QODERCLI).map_err(|source| AgentRuntimeError::Io {
        path: path.clone(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(
            |source| AgentRuntimeError::Io {
                path: path.clone(),
                source,
            },
        )?;
    }
    Ok(dir)
}

fn launch(
    binary: &str,
    config: &AgentConfig,
    auth_payload_path: &Path,
) -> Result<(Child, DaemonConnection, ChildStdout), AgentRuntimeError> {
    let mut command = Command::new(binary);
    // The daemon speaks NDJSON from `--sdk` plus the env markers; per-session flags
    // (print/stream-json/permission-mode) travel in open_session's worker_args.
    command
        .arg("--sdk")
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env(ENV_SDK_DAEMON, "1")
        .env(ENV_SDK_VERSION, SDK_VERSION)
        .env(ENV_SDK_ENTRYPOINT, SDK_ENTRYPOINT)
        .env(ENV_SDK_AUTH_PAYLOAD_FILE, auth_payload_path);
    if let Some(dir) = &config.working_dir {
        command.current_dir(dir);
    }
    for (key, value) in &config.environment {
        command.env(key, value);
    }
    let mut child = command
        .spawn()
        .map_err(|source| AgentRuntimeError::ProcessFailed {
            program: binary.to_string(),
            source,
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AgentRuntimeError::ProtocolError("Failed to capture Qoder stdout".into()))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AgentRuntimeError::ProtocolError("Failed to capture Qoder stdin".into()))?;
    // Drain stderr so daemon diagnostics can never fill the pipe and stall.
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(_)) = lines.next_line().await {}
        });
    }
    let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(32);
    tokio::spawn(async move {
        let mut writer = stdin;
        while let Some(message) = stdin_rx.recv().await {
            if writer.write_all(message.as_bytes()).await.is_err() || writer.flush().await.is_err()
            {
                break;
            }
        }
    });
    let connection = DaemonConnection {
        stdin_tx,
        pending: Arc::new(Mutex::new(HashMap::new())),
        next_id: Arc::new(AtomicU64::new(1)),
    };
    Ok((child, connection, stdout))
}

async fn initialize(connection: &DaemonConnection) -> Result<InitializeInfo, AgentRuntimeError> {
    // Auth is conveyed by the QODER_SDK_AUTH_PAYLOAD_FILE env var, not inline.
    let response = connection
        .request(
            "daemon.initialize",
            json!({
                "daemon_protocol_version": DAEMON_PROTOCOL_VERSION,
                "sdk_version": SDK_VERSION,
                "capabilities": [MULTI_SESSION_CAPABILITY],
            }),
        )
        .await?;
    let info = parse_initialize(&response).ok_or_else(|| {
        AgentRuntimeError::ProtocolError("Qoder initialize returned no daemon version".into())
    })?;
    if !info
        .capabilities
        .iter()
        .any(|capability| capability == MULTI_SESSION_CAPABILITY)
    {
        return Err(AgentRuntimeError::ProtocolError(
            "Qoder daemon is missing the multi_session_v2 capability".into(),
        ));
    }
    log::debug!("Qoder daemon {} ready", info.daemon_version);
    Ok(info)
}

fn spawn_reader(stdout: ChildStdout, connection: DaemonConnection, host: Option<Arc<SessionHost>>) {
    let (requests, mut incoming) = mpsc::channel::<Value>(32);
    if let Some(worker_host) = host.clone() {
        let worker_connection = connection.clone();
        tokio::spawn(async move {
            let mut stopped = worker_host.stopped.subscribe();
            while !*stopped.borrow_and_update() {
                tokio::select! {
                    biased;
                    _ = stopped.changed() => break,
                    frame = incoming.recv() => {
                        let Some(frame) = frame else { break };
                        let response = tokio::select! {
                            biased;
                            _ = stopped.changed() => break,
                            response = worker_host.handle(&frame) => response,
                        };
                        if worker_connection.send_frame(response).await.is_err() { break; }
                    }
                }
            }
        });
    }
    tokio::spawn(async move {
        let mut tool_names: HashMap<String, String> = HashMap::new();
        let mut reader = BufReader::new(stdout);
        let mut failure = None;
        loop {
            let value = match read_frame(&mut reader, 64 * 1024 * 1024).await {
                Ok(Some(value)) => value,
                Ok(None) => break,
                Err(error) => {
                    failure = Some(error.to_string());
                    break;
                }
            };
            match value.get("type").and_then(Value::as_str) {
                Some("control_response") => {
                    if let Some(request_id) = response_request_id(&value) {
                        let reply = connection.pending.lock().unwrap().remove(request_id);
                        if let Some(reply) = reply {
                            let _ = reply.send(value);
                        }
                    }
                }
                Some("control_request") => {
                    if requests.try_send(value.clone()).is_err() {
                        let response = json!({"type":"control_response","session_id":value["session_id"],
                            "response":{"subtype":"error","request_id":value["request_id"],"error":"NodaVideo host is busy or unavailable"}});
                        if connection.send_frame(response).await.is_err() {
                            break;
                        }
                    }
                }
                Some("control_cancel_request" | "control_cancel") => {
                    if let Some(host) = &host
                        && let Some(id) = value.get("request_id").and_then(Value::as_str)
                    {
                        host.cancel_request(id);
                    }
                }
                _ => {
                    if let Some(host) = &host {
                        map_session_frame(&value, &host.events, &mut tool_names);
                    }
                }
            }
        }
        connection.pending.lock().unwrap().clear();
        if let Some(host) = host {
            let was_stopped = *host.stopped.borrow();
            host.stop();
            if !was_stopped {
                let (code, message) = match failure {
                    Some(message) => ("QODER_PROTOCOL", message),
                    None => (
                        "QODER_EXITED",
                        "The Qoder process disconnected; start or resume the conversation again"
                            .into(),
                    ),
                };
                let _ = host.events.send(AgentEvent::SessionError {
                    code: code.into(),
                    message,
                });
            }
        }
    });
}

async fn read_frame(
    reader: &mut (impl AsyncBufRead + Unpin),
    limit: usize,
) -> std::io::Result<Option<Value>> {
    let mut bytes = Vec::new();
    loop {
        let chunk = reader.fill_buf().await?;
        if chunk.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Qoder returned an incomplete frame",
            ));
        }
        let newline = chunk.iter().position(|byte| *byte == b'\n');
        let count = match newline {
            Some(index) => index + 1,
            None => chunk.len(),
        };
        if bytes.len() + count > limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Qoder frame exceeds the 64 MiB protocol limit",
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
        reader.consume(count);
        if newline.is_none() {
            continue;
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            bytes.clear();
            continue;
        }
        return match serde_json::from_slice(&bytes) {
            Ok(value) => Ok(Some(value)),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Qoder returned invalid JSON",
            )),
        };
    }
}

/// Translate one Qoder session frame into NodaVideo events. Text and thinking
/// stream from `stream_event` deltas; `assistant` frames contribute only tool_use
/// blocks (their text would duplicate the deltas). `tool_names` correlates a
/// tool_result back to the tool_use that requested it.
fn map_session_frame(
    value: &Value,
    tx: &broadcast::Sender<AgentEvent>,
    tool_names: &mut HashMap<String, String>,
) {
    let Some(kind) = value.get("type").and_then(Value::as_str) else {
        return;
    };
    match kind {
        "stream_event" => {
            let Some(event) = value.get("event") else {
                return;
            };
            if event.get("type").and_then(Value::as_str) != Some("content_block_delta") {
                return;
            }
            let Some(delta) = event.get("delta") else {
                return;
            };
            match delta.get("type").and_then(Value::as_str) {
                Some("text_delta") => {
                    if let Some(text) = delta.get("text").and_then(Value::as_str) {
                        let _ = tx.send(AgentEvent::MessageChunk(text.to_string()));
                    }
                }
                Some("thinking_delta") => {
                    if let Some(thinking) = delta.get("thinking").and_then(Value::as_str) {
                        let _ = tx.send(AgentEvent::ThoughtChunk(thinking.to_string()));
                    }
                }
                _ => {}
            }
        }
        "assistant" => {
            let Some(content) = value
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(Value::as_array)
            else {
                return;
            };
            for block in content {
                if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                    continue;
                }
                let call_id = block
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let tool_name = block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("tool")
                    .to_string();
                tool_names.insert(call_id.clone(), tool_name.clone());
                if tool_name.starts_with("mcp__nodavideo__") {
                    continue;
                }
                let _ = tx.send(AgentEvent::ToolCallRequested {
                    call_id,
                    tool_name,
                    arguments: block.get("input").cloned().unwrap_or_else(|| json!({})),
                });
            }
        }
        "user" => {
            let Some(content) = value
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(Value::as_array)
            else {
                return;
            };
            for block in content {
                if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                    continue;
                }
                let call_id = block
                    .get("tool_use_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let tool_name = tool_names
                    .remove(&call_id)
                    .unwrap_or_else(|| "tool".to_string());
                if tool_name.starts_with("mcp__nodavideo__") {
                    continue;
                }
                let _ = tx.send(AgentEvent::ToolCallResult {
                    call_id,
                    tool_name,
                    result: block.get("content").cloned().unwrap_or(Value::Null),
                    is_error: block
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                });
            }
        }
        "result" => {
            let subtype = value.get("subtype").and_then(Value::as_str).unwrap_or("");
            if subtype == "success" {
                let _ = tx.send(AgentEvent::SessionCompleted { summary: None });
                return;
            }
            let errors_text = value.get("errors").and_then(Value::as_array).map(|errors| {
                errors
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<&str>>()
                    .join("; ")
            });
            let message = match errors_text {
                Some(text) if !text.is_empty() => text,
                _ => value
                    .get("error_message")
                    .and_then(Value::as_str)
                    .unwrap_or("Qoder turn failed")
                    .to_string(),
            };
            let _ = tx.send(AgentEvent::SessionError {
                code: format!("QODER_{subtype}"),
                message,
            });
        }
        "status_event" => {
            if value.get("status").and_then(Value::as_str) != Some("daemon.session_updated") {
                return;
            }
            let Some(data) = value.get("data") else {
                return;
            };
            if data.get("state").and_then(Value::as_str) != Some("failed") {
                return;
            }
            let message = match data.get("error") {
                Some(error) => error.to_string(),
                None => "Qoder session failed".to_string(),
            };
            let _ = tx.send(AgentEvent::SessionError {
                code: "QODER_SESSION_FAILED".into(),
                message,
            });
        }
        _ => {}
    }
}

/// The per-session worker flags. The model is chosen with a `set_model` control
/// request after open_session; `--model` here is ignored by the daemon worker.
fn worker_args(config: &AgentConfig) -> Vec<String> {
    let mut args = vec![
        "--print".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--input-format".into(),
        "stream-json".into(),
        "--tools".into(),
        "default".into(),
        "--disable-builtin-skills".into(),
        "--include-partial-messages".into(),
        "--permission-mode".into(),
        permission_mode_flag(config.permission_mode).into(),
        "--permission-prompt-tool".into(),
        "stdio".into(),
    ];
    if let Some(id) = &config.resume_session_id {
        args.extend(["--resume".into(), id.clone()]);
    }
    if !config.editor_tools.is_empty() {
        args.extend([
            "--mcp-config".into(),
            json!({"mcpServers":{
                MCP_SERVER:{"type":"sdk","name":MCP_SERVER}
            }})
            .to_string(),
            "--strict-mcp-config".into(),
        ]);
    }
    if let Some(effort) = &config.effort {
        args.extend(["--reasoning-effort".into(), effort.clone()]);
    }
    args
}

/// Resolve a concrete session model. The catalog default "auto" is a client-side
/// meta-selector the worker rejects, so an unset or "auto" choice falls back to the
/// first concrete model the daemon reports.
async fn resolve_session_model(
    connection: &DaemonConnection,
    requested: Option<&str>,
) -> Result<String, AgentRuntimeError> {
    match requested {
        Some(model) if model != "auto" => Ok(model.to_string()),
        _ => {
            let catalog = connection
                .request("daemon.get_models", json!({ "fetchStrategy": "cache" }))
                .await?;
            match parse_models(&catalog)
                .iter()
                .find(|model| model.id != "auto")
            {
                Some(model) => Ok(model.id.clone()),
                None => Err(AgentRuntimeError::Other(
                    "Qoder reported no concrete session model".into(),
                )),
            }
        }
    }
}

/// Map NodaVideo's permission mode to a Qoder CLI mode. Never `yolo`.
fn permission_mode_flag(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::ReadOnly => "plan",
        PermissionMode::Supervised => "default",
        PermissionMode::AutoAcceptEdits => "acceptEdits",
        PermissionMode::FullAccess => "bypassPermissions",
    }
}

/// Prefer a PATH entry, then the newest versioned binary in the install dir.
fn resolve_binary() -> String {
    for name in ["qodercli", "qoder"] {
        if QoderAgentProvider::probe_binary_exists(name) {
            return name.to_string();
        }
    }
    newest_installed_binary().unwrap_or_default()
}

fn newest_installed_binary() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let dir = PathBuf::from(home)
        .join(".qoder")
        .join("bin")
        .join("qodercli");
    let mut best: Option<(Vec<u64>, PathBuf)> = None;
    for entry in std::fs::read_dir(&dir).ok()?.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(version) = name.strip_prefix("qodercli-") else {
            continue;
        };
        let parts: Vec<u64> = version
            .split('.')
            .filter_map(|part| part.parse().ok())
            .collect();
        if parts.is_empty() {
            continue;
        }
        let replace = match &best {
            Some((best_parts, _)) => parts > *best_parts,
            None => true,
        };
        if replace {
            best = Some((parts, path));
        }
    }
    best.map(|(_, path)| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod host_protocol_tests {
    use crate::provider::{QoderAgentProvider, read_frame, worker_args};
    use nodavideo_agent_runtime::{AgentConfig, AgentProvider, AgentTool};
    use serde_json::json;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn initialize_frames_larger_than_64_kib_are_accepted() {
        let expected = json!({"type":"control_response","catalog":"a".repeat(72_000)});
        let input = format!("{expected}\n");
        let mut reader = BufReader::with_capacity(17, input.as_bytes());
        assert_eq!(
            read_frame(&mut reader, 64 * 1024 * 1024).await.unwrap(),
            Some(expected)
        );
        assert!(
            read_frame(&mut reader, 64 * 1024 * 1024)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn malformed_oversized_and_truncated_frames_fail_without_content() {
        for (input, limit) in [
            ("{\"secret\":\"not-json\"\n", 1024),
            ("{\"secret\":1}", 1024),
            ("{\"secret\":1}\n", 5),
        ] {
            let mut reader = BufReader::new(input.as_bytes());
            let error = read_frame(&mut reader, limit).await.unwrap_err();
            assert!(!error.to_string().contains("secret"));
        }
    }

    #[tokio::test]
    async fn missing_cli_never_falls_back_to_simulation() {
        let provider = QoderAgentProvider::new("qoder", "Qoder", "nodavideo-missing-qoder-binary");
        assert!(
            provider
                .start_session(AgentConfig::default())
                .await
                .is_err()
        );
    }

    #[test]
    fn worker_options_include_host_permissions_resume_and_real_mcp() {
        let config = AgentConfig {
            resume_session_id: Some("saved-session".into()),
            effort: Some("high".into()),
            editor_tools: vec![AgentTool {
                name: "inspect".into(),
                description: "Inspect".into(),
                input_schema: json!({"type":"object"}),
                is_mutation: false,
                requires_approval: false,
            }],
            ..AgentConfig::default()
        };
        let args = worker_args(&config);
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--permission-prompt-tool", "stdio"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--resume", "saved-session"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--reasoning-effort", "high"])
        );
        let mcp = args
            .windows(2)
            .find(|pair| pair[0] == "--mcp-config")
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&mcp[1]).unwrap();
        assert_eq!(value["mcpServers"]["nodavideo"]["type"], "sdk");
        assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
        assert!(
            !args
                .iter()
                .any(|arg| arg == "--yolo" || arg == "bypassPermissions")
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use nodavideo_agent_runtime::{AgentConfig, AgentEvent, AgentProvider, PermissionMode};
    use serde_json::json;
    use tokio::sync::broadcast;

    use crate::provider::{QoderAgentProvider, map_session_frame, worker_args};

    #[test]
    fn probe_rejects_empty_and_missing() {
        assert!(!QoderAgentProvider::probe_binary_exists(""));
        assert!(!QoderAgentProvider::probe_binary_exists(
            "nodavideo-definitely-missing-binary-xyz"
        ));
    }

    #[test]
    fn probe_accepts_absolute_path() {
        let dir = std::env::temp_dir().join(format!("qoder-probe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("qodercli-fake");
        std::fs::write(&binary, b"#!/bin/sh\n").unwrap();
        assert!(QoderAgentProvider::probe_binary_exists(
            binary.to_str().unwrap()
        ));
        let _ = std::fs::remove_file(&binary);
        let _ = std::fs::remove_dir(&dir);
    }

    #[tokio::test]
    async fn simulated_provider_is_unavailable_with_empty_catalog() {
        let provider = QoderAgentProvider::simulated();
        assert_eq!(provider.provider_id(), "qoder-simulated");
        assert!(!provider.is_available().await);
        assert!(provider.list_models().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn simulated_discovery_is_empty_offline() {
        let provider = QoderAgentProvider::simulated();
        assert!(provider.cli_version().await.is_none());
        assert!(provider.account_info().await.unwrap().is_none());
        assert!(provider.list_sessions().await.unwrap().is_empty());
    }

    #[test]
    fn worker_args_request_streaming_and_supervised_mode() {
        let args = worker_args(&AgentConfig::default());
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--input-format", "stream-json"])
        );
        assert!(args.contains(&"--include-partial-messages".to_string()));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--permission-mode", "default"])
        );
        let full = worker_args(&AgentConfig {
            permission_mode: PermissionMode::FullAccess,
            ..AgentConfig::default()
        });
        assert!(
            full.windows(2)
                .any(|pair| pair == ["--permission-mode", "bypassPermissions"])
        );
        assert!(!full.contains(&"--yolo".to_string()));
    }

    #[tokio::test]
    async fn simulated_session_streams_then_completes() {
        let provider = QoderAgentProvider::simulated();
        let session = provider
            .start_session(AgentConfig::default())
            .await
            .unwrap();
        let mut events = provider.subscribe(session).unwrap();
        provider
            .send_message(session, "hello".into())
            .await
            .unwrap();
        let mut saw_message = false;
        let mut saw_completed = false;
        for _ in 0..8 {
            match tokio::time::timeout(std::time::Duration::from_secs(2), events.recv()).await {
                Ok(Ok(AgentEvent::MessageChunk(_))) => saw_message = true,
                Ok(Ok(AgentEvent::SessionCompleted { .. })) => {
                    saw_completed = true;
                    break;
                }
                Ok(Ok(_)) => {}
                Ok(Err(_)) | Err(_) => break,
            }
        }
        assert!(saw_message);
        assert!(saw_completed);
    }

    #[tokio::test]
    async fn simulated_cancel_completes_session() {
        let provider = QoderAgentProvider::simulated();
        let session = provider
            .start_session(AgentConfig::default())
            .await
            .unwrap();
        let mut events = provider.subscribe(session).unwrap();
        provider.cancel(session).await.unwrap();
        let event = tokio::time::timeout(std::time::Duration::from_secs(2), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(event, AgentEvent::SessionCompleted { .. }));
    }

    #[tokio::test]
    async fn subscribe_unknown_session_errors() {
        let provider = QoderAgentProvider::simulated();
        let unknown = nodavideo_agent_runtime::SessionId::new();
        assert!(provider.subscribe(unknown).is_err());
        assert!(provider.cancel(unknown).await.is_err());
        assert!(provider.send_message(unknown, "hi".into()).await.is_err());
    }

    fn drain(rx: &mut broadcast::Receiver<AgentEvent>) -> Vec<AgentEvent> {
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        events
    }

    #[test]
    fn stream_deltas_map_to_message_and_thought_chunks() {
        let (tx, mut rx) = broadcast::channel(16);
        let mut names = HashMap::new();
        map_session_frame(
            &json!({"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"ok"}}}),
            &tx,
            &mut names,
        );
        map_session_frame(
            &json!({"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"hmm"}}}),
            &tx,
            &mut names,
        );
        map_session_frame(
            &json!({"type":"stream_event","event":{"type":"message_stop"}}),
            &tx,
            &mut names,
        );
        let events = drain(&mut rx);
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], AgentEvent::MessageChunk(text) if text == "ok"));
        assert!(matches!(&events[1], AgentEvent::ThoughtChunk(text) if text == "hmm"));
    }

    #[test]
    fn assistant_tool_use_and_user_tool_result_correlate() {
        let (tx, mut rx) = broadcast::channel(16);
        let mut names = HashMap::new();
        map_session_frame(
            &json!({"type":"assistant","message":{"content":[
                {"type":"text","text":"ignored"},
                {"type":"tool_use","id":"tu1","name":"Bash","input":{"command":"ls"}}
            ]}}),
            &tx,
            &mut names,
        );
        map_session_frame(
            &json!({"type":"user","message":{"content":[
                {"type":"tool_result","tool_use_id":"tu1","content":"file.txt","is_error":false}
            ]}}),
            &tx,
            &mut names,
        );
        let events = drain(&mut rx);
        assert_eq!(events.len(), 2);
        assert!(
            matches!(&events[0], AgentEvent::ToolCallRequested { call_id, tool_name, .. } if call_id == "tu1" && tool_name == "Bash")
        );
        assert!(
            matches!(&events[1], AgentEvent::ToolCallResult { call_id, tool_name, is_error, .. } if call_id == "tu1" && tool_name == "Bash" && !is_error)
        );
    }

    #[test]
    fn result_success_completes_and_error_reports() {
        let (tx, mut rx) = broadcast::channel(16);
        let mut names = HashMap::new();
        map_session_frame(
            &json!({"type":"result","subtype":"success","result":"ok"}),
            &tx,
            &mut names,
        );
        map_session_frame(
            &json!({"type":"result","subtype":"error_during_execution","errors":["boom"]}),
            &tx,
            &mut names,
        );
        let events = drain(&mut rx);
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            AgentEvent::SessionCompleted { summary: None }
        ));
        assert!(
            matches!(&events[1], AgentEvent::SessionError { message, .. } if message == "boom")
        );
    }

    #[test]
    fn session_failed_status_maps_to_error() {
        let (tx, mut rx) = broadcast::channel(16);
        let mut names = HashMap::new();
        map_session_frame(
            &json!({"type":"status_event","status":"daemon.session_updated","data":{"state":"failed","error":{"reason":"crash"}}}),
            &tx,
            &mut names,
        );
        let events = drain(&mut rx);
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0], AgentEvent::SessionError { code, .. } if code == "QODER_SESSION_FAILED")
        );
    }
}
