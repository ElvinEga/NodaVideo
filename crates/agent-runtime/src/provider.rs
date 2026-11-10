//! Canonical provider abstraction and session configuration.

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::AgentRuntimeError;
use crate::event::AgentEvent;
use crate::id::SessionId;

/// Configuration options for initializing an agent session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Target provider identifier (e.g., "acp:antigravity", "acp:gemini", "mock").
    pub provider_id: String,

    /// Optional specific model name requested for the session.
    pub model: Option<String>,

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
            system_prompt: None,
            skills: Vec::new(),
            environment: HashMap::new(),
            working_dir: None,
        }
    }
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

    /// Cancel an in-progress session execution.
    async fn cancel(&self, session: SessionId) -> Result<(), AgentRuntimeError>;

    /// Subscribe to real-time events streamed from the session.
    fn subscribe(
        &self,
        session: SessionId,
    ) -> Result<broadcast::Receiver<AgentEvent>, AgentRuntimeError>;
}
