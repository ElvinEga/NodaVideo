//! Tool execution dispatcher for MCP tool calls.
//!
//! Phase 1 stub: The original executor depended on NodaVideo's
//! `EditorCommand`/`CommandHistory`/`Sequence` types which have been
//! replaced by OpenCut's `EditingState`/`EditAction` model.
//! The full executor will be reimplemented in Phase 2 once the
//! AI panel is wired into the editor.

use crate::error::McpError;
use crate::schema::McpToolDefinition;

/// Dispatches typed MCP tool invocations against editor state.
///
/// Phase 1: returns a placeholder indicating the tool is recognized
/// but not yet implemented against the new editor core.
#[derive(Debug, Clone, Copy, Default)]
pub struct McpExecutor;

impl McpExecutor {
    /// Execute an MCP tool call.
    ///
    /// Phase 1: validates tool name and approval requirement,
    /// returns a placeholder response. Full implementation in Phase 2.
    pub fn execute(
        tool_name: &str,
        args: &serde_json::Value,
        approved: bool,
    ) -> Result<serde_json::Value, McpError> {
        let tools = McpToolDefinition::all_tools();
        let def = tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| McpError::ToolNotFound(tool_name.to_string()))?;

        if def.requires_approval && !approved {
            return Err(McpError::ApprovalRequired {
                tool: tool_name.to_string(),
                reason: format!("Operation '{tool_name}' requires explicit user confirmation"),
            });
        }

        // Phase 1: return a placeholder response
        Ok(serde_json::json!({
            "status": "not_yet_implemented",
            "tool": tool_name,
            "args": args,
            "message": "MCP executor will be reimplemented in Phase 2 against OpenCut's EditingState"
        }))
    }
}
