//! Codex app-server protocol adapter for NodaVideo.

pub mod protocol;
pub mod provider;

pub use protocol::{CodexRpcNotification, CodexRpcRequest, CodexRpcResponse};
pub use provider::CodexAgentProvider;

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_agent_runtime::{AgentConfig, AgentEvent, AgentProvider};

    #[tokio::test]
    async fn test_codex_provider_creation_and_config() {
        let provider = CodexAgentProvider::default();
        assert_eq!(provider.provider_id(), "codex-app-server");
        assert_eq!(provider.display_name(), "Codex (App-Server)");
    }

    #[tokio::test]
    async fn test_codex_simulated_session_flow() {
        let provider = CodexAgentProvider::simulated();
        let config = AgentConfig {
            provider_id: "codex-simulated".into(),
            ..Default::default()
        };

        let session = provider.start_session(config).await.unwrap();
        let mut rx = provider.subscribe(session).unwrap();

        provider
            .send_message(session, "Generate smart cuts for timeline".into())
            .await
            .unwrap();

        // 1. Thought
        let ev1 = rx.recv().await.unwrap();
        assert!(matches!(ev1, AgentEvent::ThoughtChunk(_)));

        // 2. Message
        let ev2 = rx.recv().await.unwrap();
        match ev2 {
            AgentEvent::MessageChunk(msg) => {
                assert!(msg.contains("Generate smart cuts"));
            }
            other => panic!("Expected MessageChunk, got {other:?}"),
        }

        // 3. Completed
        let ev3 = rx.recv().await.unwrap();
        assert!(matches!(ev3, AgentEvent::SessionCompleted { .. }));
    }
}
