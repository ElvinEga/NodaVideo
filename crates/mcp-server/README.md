# nodavideo-mcp-server

`nodavideo-mcp-server` provides the Model Context Protocol (MCP) typed editor tool surface for NodaVideo.

## Core Responsibilities

- **Standardized Tool Schemas (`McpToolDefinition`)**:
  - Inspection & Discovery: `get_project`, `get_timeline`, `get_selected_clips`, `search_media`.
  - Clip & Track Operations: `add_clip`, `move_clip`, `trim_clip`, `split_clip`, `delete_clip`.
  - Rendering & Export: `export_video`.
- **`McpExecutor`**:
  - Translates JSON arguments into typed domain parameters.
  - Resolves tracks and clips against the active `Sequence` and `ProjectDocument`.
  - Emits and dispatches canonical `EditorCommand` mutations directly into `CommandHistory`.
  - Enforces mandatory user approval checks before high-impact mutations (`split_clip`, `delete_clip`, `export_video`) are applied.

## Architectural Guarantees

- **Single Mutation Path**: All operations route through `EditorCommand` in `crates/command-system`.
- **Identical Semantics**: MCP tools and native UI buttons invoke the exact same command dispatch paths with unified undo/redo support.
