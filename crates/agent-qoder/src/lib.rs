//! Qoder CLI SDK daemon adapter for NodaVideo.
//!
//! Speaks the Qoder daemon's NDJSON control protocol directly from Rust (no ACP,
//! no Node sidecar). Authentication reuses the local qodercli login read-only.

pub mod protocol;
pub mod provider;

pub use protocol::{QoderAccount, QoderSession};
pub use provider::QoderAgentProvider;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nodavideo_agent_runtime::{AgentProvider, AgentRuntime};

    use crate::provider::QoderAgentProvider;

    #[test]
    fn default_provider_identity() {
        let provider = QoderAgentProvider::default();
        assert_eq!(provider.provider_id(), "qoder");
        assert_eq!(provider.display_name(), "Qoder (CLI)");
    }

    #[tokio::test]
    async fn registers_in_runtime() {
        let runtime = AgentRuntime::new();
        runtime
            .register_provider(Arc::new(QoderAgentProvider::simulated()))
            .await;
        let providers = runtime.list_providers().await;
        assert!(providers.iter().any(|(id, _)| id == "qoder-simulated"));
    }
}
