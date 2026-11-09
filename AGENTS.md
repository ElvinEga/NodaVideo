# AGENTS.md

Instructions for AI coding agents working in the NodaVideo repository. Read this in full before making any change. If a decision here conflicts with your default instincts, this file wins.

## 1. What this project is

**NodaVideo** — *Your timeline. Your creativity. Your AI.*

An AI-powered desktop video editor: a native Rust + GPUI application, cross-platform (macOS, Windows, Linux), with a professional multi-track timeline and pluggable AI agents that can create, edit and manage video projects.

Product direction, in priority order:

1. It is a **professional video editor first**, an AI assistant second. The AI workspace is a dockable panel; hiding it must leave a fully usable editor.
2. The user retains **complete control** over the timeline. Agents propose and operate; the user reviews, approves, undoes.
3. Provider independence. Claude Code, Codex, Antigravity, Gemini CLI, ACP-compatible agents and custom local commands are all interchangeable. No vendor lock-in.
4. Extensible AI skills system for video-production capabilities.

Core identity: native Rust + GPUI, professional NLE, multi-provider agent support, AI-assisted editing with full user control.

**Repo status: M0 desktop shell, M1 media management & M2 timeline engine completed.** As of 2026-11-09, the foundational Cargo workspace, media engine, timeline engine, and command system crates have been implemented:
- `apps/desktop` (`nodavideo-desktop` binary): native GPUI desktop application featuring the complete professional NLE shell (native menus with shortcuts, top toolbar, workspace mode tabs for Edit/Color/Audio/AI Studio, resizable dockable panel layout for Media Browser, Preview Monitor, Inspector, Timeline, and AI Assistant, session persistence, and dark NLE theme), a fully functioning Media Browser connected to real media imports, search, filters, drag-and-drop, and an interactive multitrack Timeline connected to real clips, ruler scrubbing, razor split, trimming, snapping, and undo/redo history.
- `crates/project` (`nodavideo-project` library): versioned JSON project document (`schema_version = 1`), atomic project persistence, missing media asset detection, strongly typed IDs (`AssetId`, `ProjectId`), frame-accurate rational timing (`TimeRational`, `FrameRate`), and timeline sequence attachment.
- `crates/media-engine` (`nodavideo-media-engine` library): FFmpeg/FFprobe stream metadata probing, thumbnail generation, normalized audio waveform extraction, and SQLite-backed persistent media cache (`MediaCacheDb`).
- `crates/timeline` (`nodavideo-timeline` library): pure deterministic multitrack sequence model, tracks (video/audio, mute, solo, lock, order), timeline clips (frame-accurate source in/out and placement), snapping engine (clip edges, playhead, sequence start), trimming, splitting, ripple deletion, and selection state.
- `crates/command-system` (`nodavideo-command-system` library): canonical `EditorCommand` enum, deterministic mutation application, transactional error rollback, and unified undo/redo history stack (`CommandHistory`).

Pinned stack: `gpui-kit 0.9.0` (`gpui-pre 0.3.11`), Rust MSRV `1.92.0`, edition `2024`. Dependencies are centralized in workspace `Cargo.toml` with `Cargo.lock` committed.
Repository scaffolding includes: `.gitignore`, `rustfmt.toml`, `README.md`, `LICENSE` (Apache-2.0), and GitHub Actions CI (`.github/workflows/ci.yml`).
Current verification status: verified locally on macOS (`aarch64-apple-darwin`). Windows and Linux builds are configured in CI for automated cross-platform verification.
Domain logic for preview/export (M3) and agent adapters (M4) will be created in their respective milestones. The `skills/` folder holds two vendored GPUI Kit reference skills (`gpui-kit`, `gpui-kit-design-guides`) used as development documentation for GPUI and UI work. `docs/GPUI_KIT_GUIDELINES.md` is the practical guide for building GPUI views in this repo. When new crates land, update this file to reflect reality.

## 2. The architectural contract (non-negotiable)

These are the decisions that define NodaVideo. Any change to them requires an explicit decision by the user, recorded here.

1. **Two separate engines.**
   - The **editor** (Rust + GPUI) owns all project data: media bin and asset management, multi-track timeline, preview monitor and playback, inspector, transitions/captions/keyframes/effects, export, undo/redo, autosave and project recovery.
   - The **agent runtime** launches the AI CLIs the user already installed and authenticated. It is pluggable and never owns editor state.
   - The editor exposes a **controlled command interface**; agents are clients of it, just like a human operating the UI. Never make an AI CLI the editor.

2. **One command system, one source of truth.** Every state mutation — human or AI — goes through `EditorCommand` in `crates/command-system`. No side doors: no direct project-file writes, no direct UI widget manipulation, no provider-specific shortcuts that bypass it. This is what gives us unified undo/redo, a visible history of agent changes, preview/approval before destructive operations, replay and reverse.

3. **Permissions live in the editor, not in prompts.** Approval checks for destructive or high-impact operations (delete, overwrite, export, external publishing) are enforced in the command layer and surfaced in the UI. A prompt instruction is never a security boundary.

4. **ACP is conversation; MCP is tools.** ACP (JSON-RPC, typically stdio) carries sessions, messages, approvals and streamed agent events. MCP exposes the typed editor tool surface. Keep the two responsibilities separate.

5. **Provider-agnostic core.** Everything outside `crates/agent-*` talks only to the `AgentProvider` trait. Protocol details stay inside the adapter. Adding or switching a provider must never require changes to the editor core.

6. **Authentication belongs to the CLI.** Launch installed CLIs with their existing login. The editor must never collect, store, proxy or reproduce provider subscription credentials.

7. **Heavy work off the UI thread.** Decoding, rendering, transcription, export and agent I/O never run on the GPUI thread. The UI thread renders state and dispatches commands.

## 3. Target workspace layout

```
NodaVideo/
├── AGENTS.md                    # this file
├── docs/
│   └── GPUI_KIT_GUIDELINES.md   # GPUI Kit / GPUI UI implementation guideline
├── Cargo.toml                   # workspace manifest (pins GPUI versions)
├── apps/
│   └── desktop/
│       └── src/
│           ├── app/             # application shell, menus, shortcuts, themes
│           ├── workspace/       # dockable panels, resizable layout, tabs
│           ├── media/           # media bin, import, thumbnails, metadata
│           ├── preview/         # monitor, playback, scrubbing
│           ├── timeline/        # tracks, clips, trimming, snapping, selection
│           ├── inspector/       # clip properties, transforms, effects, audio
│           ├── effects/
│           ├── export/
│           └── assistant/       # AI Studio dockable panel, skills picker
├── crates/
│   ├── editor-core/             # application state, orchestration
│   ├── project/                 # versioned project document + migrations
│   ├── timeline/                # pure timeline model and editing operations
│   ├── media-engine/            # FFmpeg decode/encode, probing, waveforms
│   ├── gpu-renderer/            # wgpu effects and compositing
│   ├── audio-engine/            # audio processing and playback sync
│   ├── command-system/          # EditorCommand, dispatch, undo/redo, approvals
│   ├── agent-runtime/           # AgentProvider trait, session management, events
│   ├── agent-acp/               # shared ACP adapter (Antigravity, Gemini CLI, others)
│   ├── agent-codex/             # native codex app-server adapter
│   ├── agent-claude/            # Claude Code CLI stream-JSON adapter
│   ├── skill-registry/          # SKILL.md discovery, install, per-project enable
│   ├── mcp-server/              # MCP tool exposure of editor commands
│   └── hyperframes-bridge/      # motion graphics composition bridge
├── sidecars/
│   └── hyperframes/
├── skills/                      # SKILL.md packages (dev refs + production skills)
│   ├── gpui-kit/                # vendored GPUI Kit / GPUI mechanics + Coding Guides
│   ├── gpui-kit-design-guides/  # vendored normative UI Design Guides
│   └── (future)                 # auto-captions, smart-cuts, motion-graphics,
│                                # social-video, audio-cleanup
└── templates/                   # transitions, titles, captions
```

## 4. Crate responsibilities

| Crate | Owns | Must not |
| --- | --- | --- |
| `timeline` | Tracks, clips, timing, trim/split/ripple/snap logic | Know about FFmpeg, GPUI, agents, wall-clock time or randomness |
| `command-system` | `EditorCommand`, dispatch, undo/redo stack, approval gates, history | Contain provider-specific logic |
| `project` | Versioned JSON document, schema migrations, autosave/recovery | Be the media index |
| `media-engine` | FFmpeg decoding/encoding, probing, thumbnails, waveforms | Define timeline semantics |
| `gpu-renderer` / `audio-engine` | wgpu compositing/effects, audio processing and sync | Mutate project state |
| `editor-core` | Application state, orchestration between crates | Implement protocol adapters |
| `agent-runtime` | `AgentProvider` trait, sessions, event streams, provider selection | Know editor internals |
| `agent-*` adapters | One provider protocol each, JSON-RPC/stream parsing, version pinning | Call editor state directly |
| `mcp-server` | Typed tool surface over editor commands | Add its own permission model |
| `skill-registry` | `SKILL.md` discovery, install, enable, composition | Choose a provider |
| `hyperframes-bridge` | Editable compositions (titles, lower thirds, charts, animated captions, branded transitions, AI-generated HTML scenes) | Own timing or become a second timeline |
| `apps/desktop` | GPUI views, panels, input, platform integration | Contain domain logic |

### Canonical interfaces

The rest of the application depends on these shapes. Do not redesign them casually.

```rust
#[async_trait]
pub trait AgentProvider: Send + Sync {
    async fn start_session(&self, config: AgentConfig) -> Result<SessionId>;
    async fn send_message(&self, session: SessionId, message: String) -> Result<()>;
    async fn cancel(&self, session: SessionId) -> Result<()>;
    fn subscribe(&self, session: SessionId) -> AgentEventStream;
}

pub enum EditorCommand {
    AddClip(AddClip),
    MoveClip(MoveClip),
    TrimClip(TrimClip),
    SplitClip(SplitClip),
    DeleteClip(DeleteClip),
    AddTransition(AddTransition),
    ApplyEffect(ApplyEffect),
    AddComposition(AddComposition),
    // ...extended as milestones land; every variant is undoable and replayable
}
```

Command application must be **deterministic**: given `(project_state, command)` it always produces the same new state. No wall-clock reads, no random IDs, no I/O inside command application — pass IDs, timestamps and resolved assets in from the caller. This is what makes replay and undo trustworthy.

## 5. Locked technical decisions

- **Time**: store internally as **integer frames or rational time**, never floating-point seconds. Convert only at UI, file-format and FFmpeg boundaries. A frame rate lives on the sequence, not on a clip.
- **Media**: FFmpeg for all decoding and encoding. Do not implement codecs. wgpu for GPU effects and compositing. Rust audio engine for audio processing.
- **Project format**: a **versioned JSON project document** is the source of truth (`schema_version` required; every schema change ships a migration). **SQLite** holds the media index, thumbnails, waveforms and application metadata — it is a cache/index, never project state.
- **HyperFrames**: retained for motion graphics and AI-generated HTML scenes only. It produces editable animated compositions; the Rust timeline owns their timing and placement. Never let it compete as a second timeline.
- **GPUI**: pin exact compatible versions (GPUI + GPUI Kit) in `Cargo.lock`. GPUI is still evolving — do not bump casually, and smoke-test macOS, Windows and Linux early in every milestone.
- **Agent protocols**: pin and test against actual installed CLI versions; these interfaces evolve. Prefer ACP where supported, dedicated adapters elsewhere.

| Agent | Integration |
| --- | --- |
| Antigravity | ACP adapter: `agy acp` |
| Gemini CLI | ACP adapter: `gemini --acp` |
| Codex | Native `codex app-server` adapter (conversations, approvals, streamed events) |
| Claude Code | CLI stream-JSON adapter, or ACP if the installed version supports it |
| Other / custom | ACP first; custom local command adapter where necessary |

- **Skills**: standard `SKILL.md` format, installable from folders or Git repos, containing instructions, examples, scripts and references. Skills can be installed, enabled per project, selected for a task and combined into a workflow.
  Keep the distinction sharp: **provider** = which CLI/model works; **skill** = how a task is performed; **editor tools** = what the agent is allowed to do. Selecting "Auto captions" supplies skill instructions plus relevant editor tools; it must not lock the user into Claude or Codex.

## 6. Editor command / tool surface

Expose these as typed tools through **both** the native Rust API and the MCP server, with identical semantics and identical permission checks:

```
get_project · get_timeline · get_selected_clips · search_media
add_clip · move_clip · trim_clip · split_clip · delete_clip
add_audio_track · add_transition · add_text_overlay · generate_captions
apply_effect · set_clip_transform · render_preview · export_video
```

Typical agent flow for *"Create a 30-second promo from the imported clips with an intro, subtitles, music and an outro"*: inspect project → inspect media → propose a sequence → add/arrange clips → generate captions and motion graphics → render preview → **ask for approval** before finalizing or exporting. Every step is a command in the shared history.

## 7. UI direction

Dark, dense professional NLE workspace, in the spirit of Premiere Pro / DaVinci Resolve, with resizable dockable panels: media bin, large preview monitor with playback controls and scrubbing, inspector (position, scale, rotation, opacity, effects, audio), multi-track timeline (video/audio/image/text), and a dockable AI Assistant panel. Workspace tabs: Edit / Color / Audio / AI Studio.

Core interactions: drag media to tracks, trim clip edges, reposition, drag between tracks to change layers, split at playhead, ripple delete, snapping, zoom and horizontal scroll, multi-select/group/align, inspector editing, undo/redo for every action, and AI-proposed changes shown as a preview before applying.

The skills picker is a library of video-production capabilities (Auto captions, Smart cuts, Motion graphics, Audio cleanup, Social video, Color grading), not a list of generic prompts.

**UI implementation rules:** before writing or changing any GPUI view, read `docs/GPUI_KIT_GUIDELINES.md` and the normative vendored guides in `skills/gpui-kit-design-guides/references/design-guides.md` and `skills/gpui-kit/references/coding-guides.md`. Verify every API against the pinned `gpui-kit 0.9.0` crate source in the local Cargo registry — published component pages contain stale examples. Use `cx.theme()` semantic tokens, rem-based helpers and desktop-native keyboard/focus behavior; never hard-code product colors.

## 8. Implementation order and acceptance tests

Build a working editor **vertically**, then expand. Do not attempt the full CapCut feature set at once.

| Milestone | Scope | Acceptance test | Status |
| --- | --- | --- | --- |
| M0 | Native desktop foundation: GPUI + GPUI Kit, menus, dockable/resizable panels, keyboard shortcuts, themes, project management | App launches on macOS, Windows and Linux; panels resize and persist; a new project can be created and reopened | **Completed on macOS** (CI configured for Windows/Linux) |
| M1 | Media management: import video/audio/images, thumbnails, metadata, waveform generation, media bin | Import a clip; see thumbnail, duration and waveform; relaunch retains the media index | **Completed on macOS** (Real file imports, drag-and-drop, FFprobe metadata, thumbnailing, 100-pt/sec waveform cache, SQLite index, and atomic project persistence; CI configured for Windows/Linux) |
| M2 | Timeline engine: multi-track editing, drag and drop, trim, split, snapping, playhead, selection, undo/redo | Drag a clip to a track, trim both edges, split at playhead, ripple delete; undo/redo restores exact state; times are exact at frame boundaries | **Completed on macOS** (Pure deterministic `crates/timeline` domain, canonical `EditorCommand` & `CommandHistory` in `crates/command-system`, frame-accurate snapping, timeline serialization, interactive GPUI timeline panel with razor split, trimming, dragging, and undo/redo; CI configured for Windows/Linux) |
| M3 | Preview and export: playback, scrubbing, A/V sync, effects pipeline, MP4 export | **First vertical slice: import video → drag to timeline → trim → preview → export MP4**, with output playable in a third-party player | Planned |
| M4 | AI integration: ACP, Codex app-server, Claude CLI adapter, provider selector, conversation UI, skill picker, MCP editor tools | Two different providers can run the same task; every agent mutation appears in undo history; disconnecting an agent leaves the editor fully functional | Planned |
| M5 | AI editing workflows: smart cuts, captions, animated titles, social-video formats, natural-language editing | A prompt produces a reviewable sequence, a preview render, and only commits after user approval | Planned |

Integrate **Codex and Antigravity** through their supported protocols before adding the more complex AI editing workflows.

## 9. Conventions for agents working in this repo

- **Rust**: workspace-level edition, MSRV and lint config live in the root `Cargo.toml`; do not override them per crate.
- **Errors**: `thiserror` for domain errors in library crates; `anyhow` only at application boundaries. No `unwrap()` / `expect()` in library crates unless the invariant is provably impossible — and then document why inline.
- **Async**: tokio for agent I/O and long-running tasks. Keep timeline model and command application synchronous and pure so replay/undo stay deterministic.
- **IDs**: opaque strongly-typed newtypes (`ClipId`, `TrackId`, `SessionId`, `ProjectId`), never raw strings or integers in public APIs.
- **Tests**: unit tests for timeline invariants (trim, split, ripple, snapping, frame-exact arithmetic); project schema migration tests; adapter contract tests using recorded JSON-RPC fixtures. **CI must not require live CLI logins or network access.** Add an acceptance test with every milestone.
- **Cross-platform**: no platform-specific APIs in `crates/*`; macOS/Windows/Linux specifics belong in `apps/desktop` behind traits or `cfg`.
- **Dependencies**: adding a dependency to a core crate requires a short justification recorded in the PR/commit and, if it changes the architecture, here.
- **Commands**: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. These are the intended CI contract; set them up in M0 and update this section with the real commands once CI exists.
- **Docs**: when you change the architecture, crate boundaries, command surface or milestone status, update this file and the affected crate README in the same change. An AGENTS.md that drifts from the code is worse than none.

## 10. When in doubt

Propose a design before adding a new engine, protocol, timeline or command path. Specifically: **do not** introduce a second timeline, a second mutation path, provider-specific logic in `editor-core`, credential handling in the editor, or float-second time in the model. Ask the user rather than guessing on those.

Suggested next artifact: a detailed implementation-ready technical specification covering crate responsibilities, project JSON schemas, agent protocol interfaces and milestone acceptance tests.
