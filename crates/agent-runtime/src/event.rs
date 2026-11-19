//! Event types streamed from agent providers during task execution.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug, Default, PartialEq)]
enum ToolCallState {
    Pending,
    Committed,
    #[default]
    Revoked,
}

#[derive(Debug, Clone, Default)]
pub struct ToolCallAuthorization(Arc<Mutex<ToolCallState>>);

#[derive(Debug)]
pub struct ToolCallCommit<'a>(MutexGuard<'a, ToolCallState>);

// Public APIs only
impl ToolCallCommit<'_> {
    pub fn complete(mut self) {
        *self.0 = ToolCallState::Committed;
    }
}

impl PartialEq for ToolCallAuthorization {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

// Public APIs only
impl ToolCallAuthorization {
    pub fn approved() -> Self {
        Self(Arc::new(Mutex::new(ToolCallState::Pending)))
    }

    pub fn is_valid(&self) -> bool {
        *self.0.lock().unwrap() == ToolCallState::Pending
    }

    pub fn revoke(&self) -> bool {
        let mut state = self.0.lock().unwrap();
        if *state == ToolCallState::Committed {
            return false;
        }
        *state = ToolCallState::Revoked;
        true
    }

    pub fn lock_for_commit(&self) -> Option<ToolCallCommit<'_>> {
        let state = self.0.lock().unwrap();
        if *state != ToolCallState::Pending {
            return None;
        }
        Some(ToolCallCommit(state))
    }
}

/// Discrete event yielded during an agent execution session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum AgentEvent {
    /// Streamed text content intended for the user in the assistant chat.
    MessageChunk(String),

    /// Streamed internal reasoning or thinking process.
    ThoughtChunk(String),

    /// Agent request to invoke a registered editor tool via MCP.
    ToolCallRequested {
        call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
    },

    /// An approved tool invocation that only the editor may execute.
    EditorToolCallRequested {
        call_id: String,
        tool_name: String,
        arguments: serde_json::Value,
        // A serialized event must never retain permission to execute an editor mutation.
        #[serde(skip)]
        authorization: ToolCallAuthorization,
    },

    /// Result of an executed tool call returned to the agent session.
    ToolCallResult {
        call_id: String,
        tool_name: String,
        result: serde_json::Value,
        is_error: bool,
    },

    /// Request for user approval before executing high-impact or destructive commands.
    ApprovalRequested {
        request_id: String,
        title: String,
        description: String,
        tool_name: String,
        command_summary: String,
        parameters: serde_json::Value,
    },

    /// Result of user approval decision.
    ApprovalDecision { request_id: String, approved: bool },

    /// Agent finished executing current goal or turn.
    SessionCompleted { summary: Option<String> },

    /// Error encountered during session execution.
    SessionError { code: String, message: String },
}

#[cfg(test)]
mod authorization_tests {
    use crate::event::{AgentEvent, ToolCallAuthorization};
    use serde_json::json;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn revoked_and_default_authorizations_cannot_commit() {
        let default = ToolCallAuthorization::default();
        assert!(default.lock_for_commit().is_none());
        let approved = ToolCallAuthorization::approved();
        assert!(approved.revoke());
        assert!(!approved.is_valid());
        assert!(approved.lock_for_commit().is_none());
    }

    #[test]
    fn serialized_editor_events_never_retain_authorization() {
        let original = AgentEvent::EditorToolCallRequested {
            call_id: "call".into(),
            tool_name: "edit".into(),
            arguments: json!({}),
            authorization: ToolCallAuthorization::approved(),
        };
        let encoded = serde_json::to_value(&original).unwrap();
        assert!(encoded["payload"].get("authorization").is_none());
        let replayed: AgentEvent = serde_json::from_value(encoded).unwrap();
        let AgentEvent::EditorToolCallRequested { authorization, .. } = replayed else {
            panic!("expected editor call");
        };
        assert!(authorization.lock_for_commit().is_none());
    }

    #[test]
    fn commit_wins_against_concurrent_revocation() {
        let authorization = ToolCallAuthorization::approved();
        let commit = authorization.lock_for_commit().unwrap();
        let competing = authorization.clone();
        let (started, waiting) = mpsc::channel();
        let worker = thread::spawn(move || {
            started.send(()).unwrap();
            competing.revoke()
        });
        waiting.recv().unwrap();
        assert!(!worker.is_finished());
        commit.complete();
        assert!(!worker.join().unwrap());
        assert!(!authorization.is_valid());
        assert!(authorization.lock_for_commit().is_none());
    }

    #[test]
    fn abandoned_commit_remains_revocable() {
        let authorization = ToolCallAuthorization::approved();
        drop(authorization.lock_for_commit().unwrap());
        assert!(authorization.is_valid());
        assert!(authorization.revoke());
        assert!(authorization.lock_for_commit().is_none());
    }
}
