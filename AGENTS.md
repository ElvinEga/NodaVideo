# AGENTS.md

Instructions for AI coding agents working in the NodaVideo repository. Read this in full before making any change. If a decision here conflicts with your default instincts, this file wins.

## 1. What this project is

**NodaVideo** — *Your timeline. Your creativity. Your AI.*

An AI-powered desktop video editor: a native Rust + GPUI application, cross-platform
(macOS, Windows, Linux), with a professional multi-track timeline and pluggable AI
agents that can create, edit and manage video projects.

Product direction, in priority order:

1. It is a **professional video editor first**, an AI assistant second. The AI
   workspace is a dockable panel; hiding it must leave a fully usable editor.
2. The user retains **complete control** over the timeline. Agents propose and
   operate; the user reviews, approves, undoes.
3. Provider independence. Claude Code, Codex, Antigravity, Gemini CLI,
   ACP-compatible agents and custom local commands are all interchangeable.
4. Extensible AI skills system for video-production capabilities.

Core identity: native Rust + GPUI, professional NLE, multi-provider agent support,
AI-assisted editing with full user control.

## 2. Architecture

NodaVideo is a hybrid of two codebases:

### Editor core (from OpenCut)

The working video engine, located in `libs/` and `apps/`:

- **`libs/media-backend/`** — FFmpeg FFI (`ffmpeg-next 8.1`) video/audio decoding.
  VideoToolbox on macOS, FFmpeg software decode on other platforms.
- **`libs/timeline/`** — Timeline model: `Clip` enum (Video/Audio/Text), `Track`,
  `TimelineEditingState`, `TimelineSerialization`, `FrameRate`, `TimelineFrameIndex`.
  Uses `Ulid` IDs. Folder-based project persistence (`.timeline` JSON files).
- **`libs/engine/`** — Playback engine: `TimelineBackend` (wall-clock playback),
  `TimelineDecoder` (per-track/asset `VideoReader` caching, ~1.25ms/frame),
  `SurfaceConverter` (Metal GPU YUV→BGRA on macOS, CPU on others),
  `ExportEncoder` (headless GPUI render → FFmpeg encode).
- **`libs/player-ui/`** — Player UI components, `cpal` audio output.
- **`libs/transcribe/`** — Audio transcription via Whisper API.
- **`libs/waveform/`** — Audio waveform extraction.
- **`libs/platform-fs/`** — Platform-specific file system operations.
- **`apps/editor/`** — Multi-track NLE editor (file explorer, preview, timeline,
  property panels, export, transcription, context menus).
- **`apps/player/`** — Standalone debugging player.
- **`apps/cli/`** — Headless CLI for probing, validating, exporting timelines.

### AI layer (from NodaVideo)

Located in `crates/`:

- **`crates/agent-runtime/`** — `AgentProvider` trait, `AgentRuntime` session
  router, `AgentEvent` streams, `MockAgentProvider` for offline testing.
- **`crates/agent-acp/`** — ACP JSON-RPC 2.0 stdio adapter (Antigravity, Gemini CLI).
- **`crates/agent-codex/`** — Codex app-server JSON-RPC 2.0 adapter.
- **`crates/agent-claude/`** — Claude Code CLI streaming adapter.
- **`crates/agent-opencode/`** — OpenCode HTTP/SSE adapter.
- **`crates/agent-qoder/`** — Native Rust adapter for the Qoder CLI SDK daemon
  (NDJSON protocol v2, not ACP; no Node sidecar). Borrows the local CLI login
  without reading tokens. Supplies host-owned once-only approvals and the editor's
  canonical tools through virtual SDK MCP. See `docs/QODER_PROVIDER.md`.
- **`crates/mcp-server/`** — MCP editor tool schemas and executor.
  Rewired directly to OpenCut's `TimelineEditingState` mutation API (`add_clip`, `move_clip`,
  `trim_clip`, `split_clip`, `delete_clip`, `add_text_overlay`, `generate_captions`, etc.)
  with atomic transaction rollback and approval gating.
- **`apps/editor/`** — Multi-track NLE editor with integrated dockable AI Assistant panel
  (right dock tab next to Inspector with `cmd-shift-a` shortcut), supporting multi-provider
  selection (Antigravity ACP, Gemini ACP, Codex, Claude Code, OpenCode, Qoder, Mock), streamed conversation,
  real-time thought inspection, and interactive approval cards.
- **`crates/skill-registry/`** — `SKILL.md` parser, recursive directory scanner,
  dynamic prompt instruction composer.
- **`crates/hyperframes-bridge/`** — Motion graphics template registry, HTML/SVG
  composition rendering, MCP tool bridge.

### Key architectural principles

- **Two separate engines:** The editor owns all project state; the agent runtime
  launches installed/authenticated CLIs. Never make a CLI the editor.
- **OpenCut's mutation model:** `EditAction` → `EditingState`. Mock and Qoder
  editor-tool calls use the shared `McpExecutor` and `ApplyAgentEdit` gateway.
  Native CLI tool activity is display-only; never execute it a second time.
  Reject stale project/session/revision snapshots and revoked tool authorizations
  before committing, and send success only after the editor commits.
- **GPUI:** Uses vendored Zed GPUI at `vendor/zed/crates/gpui`, NOT gpui-kit.
- **FFmpeg:** Uses `ffmpeg-next` FFI bindings linked against vendored FFmpeg 8.1.2
  libraries, NOT CLI process spawning.
- **GPU:** Metal compute shaders on macOS (`libs/engine/src/convert.metal`),
  CPU fallback on other platforms.
- **Audio:** `cpal 0.17` for real-time audio output.
- **Time:** Integer frames / rational time (`TimelineFrameIndex`, `FrameRate`),
  never float seconds.
- **Heavy work off the UI thread:** Decode, render, transcription, export, agent
  I/O all run asynchronously.
- **`unsafe` is allowed** — required for VideoToolbox/Metal/CoreVideo FFI.

## 3. Build and test

```sh
cargo editor-mac                 # Run editor (macOS)
cargo test-mac                   # Run tests (macOS)
cargo check --config .cargo/macos.toml --all-targets
cargo clippy --config .cargo/macos.toml --all-features
```

Use `.cargo/windows.toml` on Windows. Plain `cargo test` loads no platform
configuration; use `test-mac` or `test-win` for the editor and its native media
dependencies.

## 4. Vendored dependencies

- **`vendor/zed/`** — Zed GPUI source (git repo, not a submodule in the current
  setup). Required for all GPUI compilation.
- **`vendor/ffmpeg-8.1.2/`** — FFmpeg development libraries. Must be obtained
  separately. The `.cargo/macos.toml` config sets `FFMPEG_DIR` and
  `PKG_CONFIG_PATH` to point here.

## 5. Skills system

Production skills live in `skills/` (each has a `SKILL.md`):
- `smart-cuts/` — AI-powered cut detection
- `auto-captions/` — Automatic subtitle generation
- `social-video/` — Social media video formatting
- `animated-titles/` — Animated title generation
- `audio-cleanup/` — Audio noise reduction
- `color-grading/` — Color grading workflows

The skill-registry crate loads these at runtime. Skills are provider-agnostic
instructions, not code.

## 6. Migration status

The OpenCut core replacement was performed on 2026-11-13. See
`docs/MIGRATION_PLAN.md` for the full plan.

**Phase 1 (complete):** Editor core replaced. OpenCut libs, apps, .cargo, scripts
copied in. NodaVideo AI crates preserved. Cargo.toml merged. MCP server stubbed.

**Phase 2 (complete):** MCP server executor rewritten against OpenCut's
`TimelineEditingState` mutation API with transactional rollback. The AI crates
are wired into the editor as a right-dock AI Assistant tab (`cmd-alt-a`),
supporting provider cycling (Mock, Antigravity/Gemini ACP, Codex, Claude),
streamed messages and thoughts, tool-call display, and approval cards. The
panel displays agent tool activity; tool execution still belongs to the MCP
server.

**Phase 3 (in progress):** Mock and Qoder can execute the shared editor and
HyperFrames tools with approval gating. Qoder supports live models, streamed
turns, Stop, and native resume from saved NodaVideo transcripts. Other native
providers currently report tool activity but are not wired to this in-process
editor gateway. Transitions, snapping refinements, and other migration work remain.

## 7. Agent guidelines

1. Never write absolute paths in files; always use relative paths to the repo root.
2. The OpenCut design spec (`design/spec.md`) defines acceptance criteria for the
   editor. Tell the user if it needs updating after making changes.
3. The `.agents/skills/` directory contains OpenCut development conventions
   (rust-style, opencut-conventions, architecture, general-rules, ui-design).
   Follow these when working on the editor core.
4. When working on AI crates, ensure they don't introduce dependencies on
   deleted NodaVideo crates (media-engine, timeline, command-system, project,
   editor-core, audio-engine, gpu-renderer).
5. `unsafe_code` is allowed in the workspace — required for macOS FFI.
6. Qoder auth stays inside the CLI. Never collect or log its credentials, enable
   bypass permissions by default, or invent model/session catalogs. Offline tests
   must not require a CLI account or network access. Native resume needs an actual
   provider session ID; a saved transcript alone is not a resumed conversation.
