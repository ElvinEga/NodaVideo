//! Qoder CLI AgentProvider adapter: drives the SDK daemon over NDJSON stdio.
//!
//! Discovery (availability, version, model catalog, account, sessions) is
//! implemented here. Session messaging, streaming, permissions and MCP wiring
//! land in subsequent slices; the provider is not registered with the editor
//! until those are in place.

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
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::protocol::{
    AUTH_PAYLOAD_QODERCLI, ControlResponse, DAEMON_PROTOCOL_VERSION, ENV_SDK_AUTH_PAYLOAD_FILE,
    ENV_SDK_DAEMON, ENV_SDK_ENTRYPOINT, ENV_SDK_VERSION, InitializeInfo, MULTI_SESSION_CAPABILITY,
    QoderAccount, QoderSession, SDK_ENTRYPOINT, SDK_VERSION, control_request, parse_account,
    parse_control_response, parse_initialize, parse_models, parse_sessions, parse_version,
    response_request_id,
};

const RPC_TIMEOUT: Duration = Duration::from_secs(60);

/// Correlates daemon control requests with their responses; shared with the reader task.
#[derive(Clone)]
struct DaemonConnection {
    stdin_tx: mpsc::Sender<String>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>,
    next_id: Arc<AtomicU64>,
}

/// AgentProvider adapter for the Qoder CLI SDK daemon.
pub struct QoderAgentProvider {
    id: String,
    display_name: String,
    binary: String,
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
        }
    }

    /// Construct an offline provider with no binary, for tests and development.
    pub fn simulated() -> Self {
        Self {
            id: "qoder-simulated".into(),
            display_name: "Qoder (Offline Simulation)".into(),
            binary: String::new(),
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
        let mut process = connect(&self.binary, &AgentConfig::default()).await?;
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
        let mut process = connect(&self.binary, &AgentConfig::default()).await?;
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
        let mut process = connect(&self.binary, &AgentConfig::default()).await?;
        let listed = process
            .connection
            .request("daemon.get_models", json!({ "fetchStrategy": "live" }))
            .await;
        let _ = process.child.kill().await;
        Ok(parse_models(&listed?))
    }

    async fn start_session(&self, _config: AgentConfig) -> Result<SessionId, AgentRuntimeError> {
        Err(AgentRuntimeError::Other(
            "Qoder session transport is not implemented in this build yet".into(),
        ))
    }

    async fn send_message(
        &self,
        _session: SessionId,
        _message: String,
    ) -> Result<(), AgentRuntimeError> {
        Err(AgentRuntimeError::Other(
            "Qoder session transport is not implemented in this build yet".into(),
        ))
    }

    async fn submit_approval(
        &self,
        _session: SessionId,
        _request_id: String,
        _approved: bool,
    ) -> Result<(), AgentRuntimeError> {
        Err(AgentRuntimeError::Other(
            "Qoder approvals are not implemented in this build yet".into(),
        ))
    }

    async fn cancel(&self, _session: SessionId) -> Result<(), AgentRuntimeError> {
        Err(AgentRuntimeError::Other(
            "Qoder session transport is not implemented in this build yet".into(),
        ))
    }

    fn subscribe(
        &self,
        session: SessionId,
    ) -> Result<broadcast::Receiver<AgentEvent>, AgentRuntimeError> {
        Err(AgentRuntimeError::SessionNotFound(session))
    }
}

// Private APIs only
impl DaemonConnection {
    async fn request(&self, subtype: &str, params: Value) -> Result<Value, AgentRuntimeError> {
        let id = format!("nodavideo-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let (reply_tx, reply_rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id.clone(), reply_tx);
        let mut line = serde_json::to_string(&control_request(&id, subtype, params))?;
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
                    "Qoder dropped the reply to '{subtype}'"
                )));
            }
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                return Err(AgentRuntimeError::ProtocolError(format!(
                    "Qoder did not answer '{subtype}' within {}s",
                    RPC_TIMEOUT.as_secs()
                )));
            }
        };
        match parse_control_response(&reply) {
            Some(ControlResponse::Success { response }) => Ok(response),
            Some(ControlResponse::Error { code, message }) => Err(AgentRuntimeError::Other(
                format!("Qoder '{subtype}' failed ({code}): {message}"),
            )),
            None => Err(AgentRuntimeError::ProtocolError(
                "Qoder sent an unrecognized control_response".into(),
            )),
        }
    }
}

/// A running daemon plus the auth payload dir it reads; dropping it cleans up.
struct DaemonProcess {
    child: Child,
    connection: DaemonConnection,
    _auth_dir: TempDir,
}

/// Launch the daemon, wire its stdio, and complete the handshake.
async fn connect(binary: &str, config: &AgentConfig) -> Result<DaemonProcess, AgentRuntimeError> {
    let auth_dir = write_auth_payload()?;
    let payload_path = auth_dir.path().join("payload.json");
    let (mut child, connection, stdout) = launch(binary, config, &payload_path)?;
    spawn_reader(stdout, connection.pending.clone());
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
    command
        .args([
            "--sdk",
            "--print",
            "--output-format",
            "stream-json",
            "--input-format",
            "stream-json",
            "--permission-mode",
            permission_mode_flag(config.permission_mode),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env(ENV_SDK_DAEMON, "1")
        .env(ENV_SDK_VERSION, SDK_VERSION)
        .env(ENV_SDK_ENTRYPOINT, SDK_ENTRYPOINT)
        .env(ENV_SDK_AUTH_PAYLOAD_FILE, auth_payload_path);
    if let Some(dir) = &config.working_dir {
        command.current_dir(dir).arg("--cwd").arg(dir);
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
            while let Ok(Some(line)) = lines.next_line().await {
                log::debug!("qoder stderr: {line}");
            }
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

fn spawn_reader(stdout: ChildStdout, pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>) {
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
            let Some(request_id) = response_request_id(&value) else {
                continue;
            };
            let reply = pending.lock().unwrap().remove(request_id);
            if let Some(reply) = reply {
                let _ = reply.send(value);
            }
        }
    });
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
mod tests {
    use nodavideo_agent_runtime::AgentProvider;

    use crate::provider::QoderAgentProvider;

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
}
