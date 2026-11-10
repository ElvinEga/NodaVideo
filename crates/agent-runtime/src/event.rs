//! Event types streamed from agent providers during task execution.

use serde::{Deserialize, Serialize};

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
