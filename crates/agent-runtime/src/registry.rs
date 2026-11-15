//! Provider registry and multi-provider agent session coordinator.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::error::AgentRuntimeError;
use crate::id::SessionId;
use crate::provider::{AgentConfig, AgentProvider};

/// Runtime registry holding available agent providers and managing active session dispatch.
#[derive(Clone, Default)]
pub struct AgentRuntime {
    providers: Arc<RwLock<HashMap<String, Arc<dyn AgentProvider>>>>,
    session_providers: Arc<RwLock<HashMap<SessionId, String>>>,
}

impl std::fmt::Debug for AgentRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRuntime").finish_non_exhaustive()
    }
}

impl AgentRuntime {
    /// Create a new empty agent runtime.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider adapter with the runtime.
    pub async fn register_provider(&self, provider: Arc<dyn AgentProvider>) {
        let mut lock = self.providers.write().await;
        lock.insert(provider.provider_id().to_string(), provider);
    }

    /// List all registered provider IDs and their human-friendly display names.
    pub async fn list_providers(&self) -> Vec<(String, String)> {
        let lock = self.providers.read().await;
        lock.values()
            .map(|p| (p.provider_id().to_string(), p.display_name().to_string()))
            .collect()
    }

    /// Retrieve a registered provider by its identifier.
    pub async fn get_provider(&self, provider_id: &str) -> Option<Arc<dyn AgentProvider>> {
        let lock = self.providers.read().await;
        lock.get(provider_id).cloned()
    }

    /// Start a new session using the specified provider.
    pub async fn start_session(&self, config: AgentConfig) -> Result<SessionId, AgentRuntimeError> {
        let provider = self
            .get_provider(&config.provider_id)
            .await
            .ok_or_else(|| {
                AgentRuntimeError::ProviderUnavailable(
                    config.provider_id.clone(),
                    "Provider is not registered".into(),
                )
            })?;

        if !provider.is_available().await {
            return Err(AgentRuntimeError::ProviderUnavailable(
                config.provider_id.clone(),
                "Provider CLI or service is not available on this machine".into(),
            ));
        }

        let session_id = provider.start_session(config.clone()).await?;

        let mut lock = self.session_providers.write().await;
        lock.insert(session_id, config.provider_id);

        Ok(session_id)
    }

    /// Send a message to an active session, routing to the correct provider.
    pub async fn send_message(
        &self,
        session: SessionId,
        message: String,
    ) -> Result<(), AgentRuntimeError> {
        let provider_id = {
            let lock = self.session_providers.read().await;
            lock.get(&session)
                .cloned()
                .ok_or(AgentRuntimeError::SessionNotFound(session))?
        };

        let provider = self
            .get_provider(&provider_id)
            .await
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;

        provider.send_message(session, message).await
    }

    /// Submit an approval decision for a pending high-impact action in a session.
    pub async fn submit_approval(
        &self,
        session: SessionId,
        request_id: String,
        approved: bool,
    ) -> Result<(), AgentRuntimeError> {
        let provider_id = {
            let lock = self.session_providers.read().await;
            lock.get(&session)
                .cloned()
                .ok_or(AgentRuntimeError::SessionNotFound(session))?
        };

        let provider = self
            .get_provider(&provider_id)
            .await
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;

        provider
            .submit_approval(session, request_id, approved)
            .await
    }

    /// Feed an editor-executed tool result back into the session's provider.
    pub async fn submit_tool_result(
        &self,
        session: SessionId,
        call_id: String,
        result: serde_json::Value,
        is_error: bool,
    ) -> Result<(), AgentRuntimeError> {
        let provider_id = {
            let lock = self.session_providers.read().await;
            lock.get(&session)
                .cloned()
                .ok_or(AgentRuntimeError::SessionNotFound(session))?
        };

        let provider = self
            .get_provider(&provider_id)
            .await
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;

        provider
            .submit_tool_result(session, call_id, result, is_error)
            .await
    }

    /// Cancel an active session.
    pub async fn cancel(&self, session: SessionId) -> Result<(), AgentRuntimeError> {
        let provider_id = {
            let lock = self.session_providers.read().await;
            lock.get(&session)
                .cloned()
                .ok_or(AgentRuntimeError::SessionNotFound(session))?
        };

        let provider = self
            .get_provider(&provider_id)
            .await
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;

        provider.cancel(session).await
    }

    /// Subscribe to real-time events from an active session.
    pub async fn subscribe(
        &self,
        session: SessionId,
    ) -> Result<tokio::sync::broadcast::Receiver<crate::event::AgentEvent>, AgentRuntimeError> {
        let provider_id = {
            let lock = self.session_providers.read().await;
            lock.get(&session)
                .cloned()
                .ok_or(AgentRuntimeError::SessionNotFound(session))?
        };

        let provider = self
            .get_provider(&provider_id)
            .await
            .ok_or(AgentRuntimeError::SessionNotFound(session))?;

        provider.subscribe(session)
    }
}
