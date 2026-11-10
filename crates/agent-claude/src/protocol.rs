//! Claude Code CLI streaming JSON event framing.

use serde::{Deserialize, Serialize};

/// Line-delimited event emitted by `claude` CLI streaming output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeStreamEvent {
    #[serde(rename = "thought")]
    Thought { content: String },

    #[serde(rename = "text")]
    Text { content: String },

    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },

    #[serde(rename = "approval_required")]
    ApprovalRequired {
        id: String,
        tool: String,
        description: Option<String>,
        #[serde(default)]
        input: serde_json::Value,
    },

    #[serde(rename = "done")]
    Done {
        #[serde(default)]
        summary: Option<String>,
    },

    #[serde(rename = "error")]
    Error { message: String },
}
