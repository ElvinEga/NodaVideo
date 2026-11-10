//! Claude Code CLI protocol adapter for NodaVideo.

pub mod protocol;
pub mod provider;

pub use protocol::ClaudeStreamEvent;
pub use provider::ClaudeAgentProvider;

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_agent_runtime::{AgentConfig, AgentEvent, AgentProvider};

    #[tokio::test]
    async fn test_claude_provider_creation_and_config() {
        let provider = ClaudeAgentProvider::default();
        assert_eq!(provider.provider_id(), "claude-cli");
        assert_eq!(provider.display_name(), "Claude Code (CLI)");
    }

    #[tokio::test]
    async fn test_claude_simulated_session_flow() {
        let provider = ClaudeAgentProvider::simulated();
        let config = AgentConfig {
            provider_id: "claude-simulated".into(),
            ..Default::default()
        };

        let session = provider.start_session(config).await.unwrap();
        let mut rx = provider.subscribe(session).unwrap();

        provider
            .send_message(session, "Auto caption dialogue on track A1".into())
            .await
            .unwrap();

        // 1. Thought
        let ev1 = rx.recv().await.unwrap();
        assert!(matches!(ev1, AgentEvent::ThoughtChunk(_)));

        // 2. Message
        let ev2 = rx.recv().await.unwrap();
        match ev2 {
            AgentEvent::MessageChunk(msg) => {
                assert!(msg.contains("Auto caption dialogue"));
            }
            other => panic!("Expected MessageChunk, got {other:?}"),
        }

        // 3. Completed
        let ev3 = rx.recv().await.unwrap();
        assert!(matches!(ev3, AgentEvent::SessionCompleted { .. }));
    }
}
