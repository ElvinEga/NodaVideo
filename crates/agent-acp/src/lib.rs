//! Agent Communication Protocol (ACP) adapter for NodaVideo.
//!
//! Provides a standardized JSON-RPC 2.0 communication layer over stdio
//! for ACP-compatible agent CLIs including Antigravity (`agy acp`) and
//! Gemini CLI (`gemini --acp`).

pub mod protocol;
pub mod provider;

pub use protocol::{JsonRpcErrorPayload, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
pub use provider::AcpAgentProvider;

#[cfg(test)]
mod tests {
    use super::*;
    use nodavideo_agent_runtime::{AgentConfig, AgentEvent, AgentProvider};

    #[tokio::test]
    async fn test_acp_provider_creation_and_config() {
        let provider = AcpAgentProvider::antigravity();
        assert_eq!(provider.provider_id(), "acp-antigravity");
        assert_eq!(provider.display_name(), "Antigravity (ACP)");

        let gemini = AcpAgentProvider::gemini();
        assert_eq!(gemini.provider_id(), "acp-gemini");
        assert_eq!(gemini.display_name(), "Gemini CLI (ACP)");
    }

    #[tokio::test]
    async fn test_acp_simulated_session_flow() {
        let provider = AcpAgentProvider::simulated("acp-sim", "Simulated ACP");
        let config = AgentConfig {
            provider_id: "acp-sim".into(),
            ..Default::default()
        };

        let session = provider.start_session(config).await.unwrap();
        let mut rx = provider.subscribe(session).unwrap();

        provider
            .send_message(session, "Create promo video".into())
            .await
            .unwrap();

        // 1. Thought
        let ev1 = rx.recv().await.unwrap();
        assert!(matches!(ev1, AgentEvent::ThoughtChunk(_)));

        // 2. Message
        let ev2 = rx.recv().await.unwrap();
        match ev2 {
            AgentEvent::MessageChunk(msg) => {
                assert!(msg.contains("Simulated ACP"));
                assert!(msg.contains("Create promo video"));
            }
            other => panic!("Expected MessageChunk, got {other:?}"),
        }

        // 3. Completed
        let ev3 = rx.recv().await.unwrap();
        assert!(matches!(ev3, AgentEvent::SessionCompleted { .. }));
    }
}
