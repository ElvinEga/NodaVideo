//! Canonical provider abstraction and session configuration.

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::AgentRuntimeError;
use crate::event::AgentEvent;
use crate::id::SessionId;

/// How much freedom the agent has to run commands and change files without asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PermissionMode {
    /// Inspect only; commands and file changes are denied.
    ReadOnly,
    /// Ask before commands and file changes.
    #[default]
    Supervised,
    /// Auto-approve edits; ask before other actions.
    AutoAcceptEdits,
    /// Allow commands and edits without prompts.
    FullAccess,
}

/// Configuration options for initializing an agent session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Target provider identifier (e.g., "acp:antigravity", "acp:gemini", "mock").
    pub provider_id: String,

    /// Optional specific model name requested for the session.
    pub model: Option<String>,

    /// Optional reasoning-effort variant requested for the session (e.g. "low", "high", "max").
    pub effort: Option<String>,

    /// How much freedom the session's agent has to act without user approval.
    pub permission_mode: PermissionMode,

    /// Optional custom system instructions or context.
    pub system_prompt: Option<String>,

    /// List of enabled skill names for this session.
    pub skills: Vec<String>,

    /// Environment variables to supply to the agent process.
    pub environment: HashMap<String, String>,

    /// Working directory for agent file operations.
    pub working_dir: Option<PathBuf>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider_id: "mock".into(),
            model: None,
            effort: None,
            permission_mode: PermissionMode::default(),
            system_prompt: None,
            skills: Vec::new(),
            environment: HashMap::new(),
            working_dir: None,
        }
    }
}

/// One model a provider can run, as reported by the provider itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentModel {
    /// Identifier passed back as `AgentConfig::model` (e.g. "gpt-6-luna", "claude-sonnet-4-6").
    pub id: String,

    /// Human-friendly label for the picker.
    pub display_name: String,

    /// Optional one-line description from the provider catalog.
    pub description: Option<String>,

    /// Reasoning-effort variants this model supports, in provider order; empty when the
    /// model has no effort dimension.
    pub efforts: Vec<String>,

    /// The effort used when `AgentConfig::effort` is None.
    pub default_effort: Option<String>,

    /// True when the provider marks this model as its default.
    pub is_default: bool,
}

/// Pluggable interface that every agent provider adapter must implement.
///
/// Implementations must be provider-agnostic outside this crate and never mutate
/// editor state directly. All tool calls and timeline mutations must route through
/// the canonical command layer.
#[async_trait]
pub trait AgentProvider: Send + Sync {
    /// Short machine identifier (e.g. "acp-antigravity", "acp-gemini", "codex", "claude", "mock").
    fn provider_id(&self) -> &str;

    /// Human-friendly display label (e.g. "Antigravity (ACP)", "Gemini CLI", "Claude Code").
    fn display_name(&self) -> &str;

    /// Probe whether the underlying CLI tool or service is installed and available locally.
    async fn is_available(&self) -> bool;

    /// List the models this provider can actually run, queried from the provider itself.
    /// The default is an empty catalog for providers that expose no model listing.
    async fn list_models(&self) -> Result<Vec<AgentModel>, AgentRuntimeError> {
        Ok(Vec::new())
    }

    /// Start a new conversation or task session with the provider.
    async fn start_session(&self, config: AgentConfig) -> Result<SessionId, AgentRuntimeError>;

    /// Send a user message or instruction into an existing session.
    async fn send_message(
        &self,
        session: SessionId,
        message: String,
    ) -> Result<(), AgentRuntimeError>;

    /// Submit a user approval decision for a pending high-impact tool execution.
    async fn submit_approval(
        &self,
        session: SessionId,
        request_id: String,
        approved: bool,
    ) -> Result<(), AgentRuntimeError>;

    /// Feed the result of an editor-executed tool call back into the session.
    ///
    /// Providers that delegate tool execution to the editor (the offline mock)
    /// use this to continue their turn. Providers that run their own MCP client
    /// execute tools themselves and ignore it; the default implementation is a
    /// no-op so they need not opt in.
    async fn submit_tool_result(
        &self,
        _session: SessionId,
        _call_id: String,
        _result: serde_json::Value,
        _is_error: bool,
    ) -> Result<(), AgentRuntimeError> {
        Ok(())
    }

    /// Cancel an in-progress session execution.
    async fn cancel(&self, session: SessionId) -> Result<(), AgentRuntimeError>;

    /// Subscribe to real-time events streamed from the session.
    fn subscribe(
        &self,
        session: SessionId,
    ) -> Result<broadcast::Receiver<AgentEvent>, AgentRuntimeError>;
}
