//! Model Context Protocol (MCP) tool exposure for NodaVideo editor commands.
//!
//! Exposes typed tools that external AI agent CLIs can discover and invoke.
//!
//! Phase 1: Tool schemas are fully defined. The executor is a stub that
//! validates tool names and approval requirements. Full mutation support
//! will be added in Phase 2 when the MCP server is rewired to OpenCut's
//! `EditingState` mutation API.

pub mod error;
pub mod executor;
pub mod schema;

pub use error::McpError;
pub use executor::McpExecutor;
pub use schema::McpToolDefinition;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tools_schema_definition() {
        let tools = McpToolDefinition::all_tools();
        assert!(tools.len() >= 10);
        assert!(tools.iter().any(|t| t.name == "get_timeline"));
        assert!(
            tools
                .iter()
                .any(|t| t.name == "split_clip" && t.requires_approval)
        );
        assert!(
            tools
                .iter()
                .any(|t| t.name == "delete_clip" && t.requires_approval)
        );
        assert!(
            tools
                .iter()
                .any(|t| t.name == "export_video" && t.requires_approval)
        );
    }

    #[test]
    fn test_executor_stub_returns_placeholder() {
        let res = McpExecutor::execute(
            "get_timeline",
            &serde_json::json!({}),
            false,
        )
        .unwrap();

        assert_eq!(res["status"], "not_yet_implemented");
        assert_eq!(res["tool"], "get_timeline");
    }

    #[test]
    fn test_executor_stub_enforces_approval() {
        let err = McpExecutor::execute(
            "split_clip",
            &serde_json::json!({"clip_id": "test", "split_frame": 10}),
            false,
        );
        assert!(matches!(err, Err(McpError::ApprovalRequired { .. })));
    }

    #[test]
    fn test_executor_stub_unknown_tool() {
        let err = McpExecutor::execute(
            "nonexistent_tool",
            &serde_json::json!({}),
            true,
        );
        assert!(matches!(err, Err(McpError::ToolNotFound(_))));
    }
}
