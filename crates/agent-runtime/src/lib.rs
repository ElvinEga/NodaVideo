//! Agent runtime orchestration, provider traits, and session event bus for **NodaVideo**.
//!
//! Exposes the canonical [`AgentProvider`] trait and [`AgentRuntime`] registry
//! allowing interchangeable pluggable AI assistants (Antigravity ACP, Gemini CLI,
//! Claude Code, Codex, and offline mock).

pub mod error;
pub mod event;
pub mod id;
pub mod mock;
pub mod provider;
pub mod registry;

pub use error::AgentRuntimeError;
pub use event::AgentEvent;
pub use id::SessionId;
pub use mock::MockAgentProvider;
pub use provider::{AgentConfig, AgentModel, AgentProvider};
pub use registry::AgentRuntime;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_runtime_registration_and_listing() {
        let runtime = AgentRuntime::new();
        let mock_provider = Arc::new(MockAgentProvider::new());
        runtime.register_provider(mock_provider).await;

        let providers = runtime.list_providers().await;
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].0, "mock");
        assert_eq!(providers[0].1, "Mock Assistant (Offline / Test)");
    }

    #[tokio::test]
    async fn test_mock_session_message_flow() {
        let runtime = AgentRuntime::new();
        let mock_provider = Arc::new(MockAgentProvider::new());
        runtime.register_provider(mock_provider).await;

        let config = AgentConfig {
            provider_id: "mock".into(),
            ..Default::default()
        };

        let session = runtime.start_session(config).await.unwrap();
        let mut rx = runtime.subscribe(session).await.unwrap();

        runtime
            .send_message(session, "Hello assistant".into())
            .await
            .unwrap();

        // 1. First event should be thought
        let ev1 = rx.recv().await.unwrap();
        match ev1 {
            AgentEvent::ThoughtChunk(thought) => {
                assert!(thought.contains("Analyzing timeline"));
            }
            other => panic!("Expected ThoughtChunk, got {other:?}"),
        }

        // 2. Second event should be message
        let ev2 = rx.recv().await.unwrap();
        match ev2 {
            AgentEvent::MessageChunk(msg) => {
                assert!(msg.contains("Hello assistant"));
            }
            other => panic!("Expected MessageChunk, got {other:?}"),
        }

        // 3. Third event should be completion
        let ev3 = rx.recv().await.unwrap();
        assert!(matches!(ev3, AgentEvent::SessionCompleted { .. }));
    }

    #[tokio::test]
    async fn test_mock_approval_workflow() {
        let runtime = AgentRuntime::new();
        let mock_provider = Arc::new(MockAgentProvider::new());
        runtime.register_provider(mock_provider).await;

        let config = AgentConfig {
            provider_id: "mock".into(),
            ..Default::default()
        };

        let session = runtime.start_session(config).await.unwrap();
        let mut rx = runtime.subscribe(session).await.unwrap();

        runtime
            .send_message(session, "Please split clip on track V1".into())
            .await
            .unwrap();

        // Drain until approval requested
        let req_id = loop {
            let ev = rx.recv().await.unwrap();
            if let AgentEvent::ApprovalRequested {
                request_id,
                tool_name,
                ..
            } = ev
            {
                assert_eq!(tool_name, "split_clip");
                break request_id;
            }
        };

        // Submit approval
        runtime
            .submit_approval(session, req_id, true)
            .await
            .unwrap();

        // Expect ApprovalDecision(true), ToolCallRequested, ToolCallResult, SessionCompleted
        let decision_ev = rx.recv().await.unwrap();
        assert!(matches!(
            decision_ev,
            AgentEvent::ApprovalDecision { approved: true, .. }
        ));

        let tool_ev = rx.recv().await.unwrap();
        assert!(
            matches!(tool_ev, AgentEvent::ToolCallRequested { tool_name, .. } if tool_name == "split_clip")
        );

        let res_ev = rx.recv().await.unwrap();
        assert!(matches!(
            res_ev,
            AgentEvent::ToolCallResult {
                is_error: false,
                ..
            }
        ));

        let done_ev = rx.recv().await.unwrap();
        assert!(matches!(done_ev, AgentEvent::SessionCompleted { .. }));
    }
}
