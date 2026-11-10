# nodavideo-hyperframes-bridge

`nodavideo-hyperframes-bridge` provides motion graphics and HTML-based composition capabilities for NodaVideo.

## Architectural Mandate

> *"HyperFrames: retained for motion graphics and AI-generated HTML scenes only. It produces editable animated compositions; the Rust timeline owns their timing and placement. Never let it compete as a second timeline."* — `AGENTS.md`

## Responsibilities

- **Composition Templates (`TemplateDefinition`)**:
  - Strongly typed parameter schemas (`String`, `Number`, `Color`, `Boolean`).
  - Parameter validation and fallback defaults.
  - Token replacement (`{{key}}`) into HTML and SVG vector templates.
  - Built-in broadcast templates:
    - `lower-third-modern`: Broadcast lower-third overlay with speaker name, subtitle, and accent border.
    - `title-minimal`: Minimalist centered title sequence with optional subtitle and accent line.
    - `social-caption-card`: High-contrast card with pill tag for vertical/social video snippets.
    - `branded-transition`: Branded motion graphic transition overlay.
- **Composition Instance (`Composition`)**:
  - Instance metadata, parameter values, template reference, and unique `CompositionId`.
- **Composition Renderer (`CompositionRenderer`)**:
  - Headless document generation (`render_html_document`).
  - Frame asset rendering for video timeline integration (`render_media_asset`).
  - Offline self-contained rendering fallback ensuring 100% CI reproducibility without external browser dependencies.
- **Timeline & Command Integration**:
  - Integrated with `EditorCommand::AddComposition` in `crates/command-system`.
  - Exposed via MCP tools `list_composition_templates` and `add_composition` in `crates/mcp-server`.
  - Full undo/redo reversibility: undos cleanly remove the composition clip from the sequence.
