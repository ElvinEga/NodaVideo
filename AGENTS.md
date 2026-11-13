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

**Repo status (corrected 2026-11-12): M0 and M1 are complete; M2 and M3 are partial; M4 and M5 have working domain and protocol layers.** The earlier "all milestones completed" claim described implemented modules rather than end-to-end working features — see the **Reality check** below and the milestone table in §8. As of 2026-11-12, the foundational Cargo workspace, media engine, timeline engine, command system, MP4 export pipeline, multi-provider AI adapters, and AI editing workflow architecture have been implemented:
- `apps/desktop` (`nodavideo-desktop` binary): native GPUI desktop application featuring the complete professional NLE shell (native menus with shortcuts, top toolbar, workspace mode tabs for Edit/Color/Audio/AI Studio, resizable dockable panel layout for Media Browser, Preview Monitor, Inspector, Timeline, and AI Assistant, session persistence, and dark NLE theme), a fully functioning Media Browser connected to real media imports, search, filters, drag-and-drop, an interactive multitrack Timeline connected to real clips, ruler scrubbing, razor split, trimming, snapping, and undo/redo history, a real-time Preview Monitor with playback loop, frame-accurate transport controls, full MP4 export workflow, a dedicated AI Assistant dockable panel with provider cycling (Antigravity ACP / Gemini ACP / Codex App-Server / Claude Code CLI / Mock Offline), active skill badge indicator, streaming chat message history, real-time thought/tool-call inspection, interactive approval cards for staged sequence edits, and status bar AI readiness indicator.
- `crates/project` (`nodavideo-project` library): versioned JSON project document (`schema_version = 1`), atomic project persistence, missing media asset detection, strongly typed IDs (`AssetId`, `ProjectId`), frame-accurate rational timing (`TimeRational`, `FrameRate`), and timeline sequence attachment.
- `crates/media-engine` (`nodavideo-media-engine` library): FFmpeg/FFprobe stream metadata probing, thumbnail generation, normalized audio waveform extraction, real-time timeline frame extraction with disk caching (`render_sequence_frame`), broadcast-standard multitrack MP4 export filtergraph assembly (`export_sequence`), and SQLite-backed persistent media cache (`MediaCacheDb`); central FFmpeg/FFprobe executable resolution with env overrides (`binaries`), and pure frame-exact conversion helpers (`time_math`) used only at the FFmpeg CLI boundary.
- `crates/timeline` (`nodavideo-timeline` library): pure deterministic multitrack sequence model, tracks (video/audio, mute, solo, lock, order), timeline clips (frame-accurate source in/out and placement), text overlays and auto-captions (`TextOverlay`, `TextStyle`, `TextAlignment`, `CaptionSegment`), edit cut transitions (crossfade, dissolve, fade-to-black, dip-to-white, wipe), snapping engine (clip edges, playhead, sequence start), trimming, splitting, ripple deletion, and selection state.
- `crates/command-system` (`nodavideo-command-system` library): canonical `EditorCommand` enum with atomic `BatchCommand` and `StagedChangeset` support, text overlays (`AddTextOverlay`), subtitle caption batches (`GenerateCaptions`), clip transforms, audio parameters, transition placement, deterministic speculative preview diff projection, transactional error rollback, and unified undo/redo history stack (`CommandHistory`).
- `crates/agent-runtime` (`nodavideo-agent-runtime` library): pluggable `AgentProvider` trait, `AgentConfig`, strongly typed `AgentEvent` streams, `MockAgentProvider` with approval prompts and tool calls for offline CI, and thread-safe session router (`AgentRuntime`).
- `crates/mcp-server` (`nodavideo-mcp-server` library): Model Context Protocol editor tool schemas (`get_project`, `get_timeline`, `get_selected_clips`, `search_media`, `add_clip`, `move_clip`, `trim_clip`, `split_clip`, `delete_clip`, `add_transition`, `add_text_overlay`, `generate_captions`, `list_composition_templates`, `add_composition`, `set_clip_transform`, `set_clip_audio`, `export_video`, `preview_changeset`, `propose_changeset`), argument validation, approval gating, and direct command execution through `CommandHistory`.
- `crates/agent-acp` (`nodavideo-agent-acp` library): Agent Client Protocol (ACP) JSON-RPC 2.0 stdio provider adapter for Antigravity (`agy acp`) and Gemini CLI (`gemini --acp`) with simulated fallback for tests.
- `crates/agent-codex` (`nodavideo-agent-codex` library): native `codex app-server` JSON-RPC 2.0 stdio provider adapter with simulated fallback for tests.
- `crates/agent-claude` (`nodavideo-agent-claude` library): Anthropic Claude Code CLI (`claude --output-format stream-json`) streaming adapter with simulated fallback for tests.
- `crates/skill-registry` (`nodavideo-skill-registry` library): `SKILL.md` parser, recursive directory scanner, and dynamic prompt instruction composer for production video skills (`smart-cuts`, `auto-captions`, `social-video`, `animated-titles`).
- `crates/hyperframes-bridge` (`nodavideo-hyperframes-bridge` library): broadcast motion graphics engine, template registry (`lower-third-modern`, `title-minimal`, `social-caption-card`, `branded-transition`), parameter validation, HTML/SVG vector composition rendering, and MCP tool bridge.
- `crates/audio-engine` (`nodavideo-audio-engine` library): multitrack audio processing and playback sync engine, interleaved float PCM buffers (`AudioBuffer`), stereo panning, volume gain conversion (`db_to_linear`), track solo/mute gating, peak/RMS level metering in dBFS (`AudioMeter`), and master sample clock synchronization (`AudioClock`).
- `crates/gpu-renderer` (`nodavideo-gpu-renderer` library): wgpu video compositing and effects pipeline, WGSL clip 2D transform and edit cut transition shaders, and deterministic pure software fallback compositor (`SoftwareCompositor`).
- `crates/editor-core` (`nodavideo-editor-core` library): central application state container (`EditorSession`), timeline selection state, transport playback navigation, cut jumping, staged changeset lifecycle, and subsystem orchestration.
- `templates/`: standard production motion graphics and title template bundles (`titles`, `lower-thirds`, `social`, `transitions`) with recursive directory loading.
- `sidecars/hyperframes`: vector motion graphics and HTML scene sidecar runtime (`@nodavideo/hyperframes-sidecar`) with headless CLI renderer and browser preview testbed.

### Reality check (2026-11-12 audit)

An audit of the playback/editing path against the running code found that the
domain layer is sound but the editor is not yet reliably usable as an NLE. The
bullets above describe implemented modules, not end-to-end working features:

- **Playback is a slideshow.** One FFmpeg process and one JPEG per requested
  frame; measured ~37.5 ms/frame at 640x360, before compositing or audio. Phase A
  added a single-flight coalescing scheduler and a wall-clock clock, but did not
  replace the per-frame process model. A persistent decoder measured ~1.25 ms/frame
  on the same clip. See `docs/PLAYBACK_ENGINE_DESIGN.md`.
- **Playback is silent.** There is no audio output backend (`cpal` / `rodio` /
  ALSA / CoreAudio / SDL2) anywhere in the workspace. `crates/audio-engine`
  provides mixing primitives but is used only from `#[cfg(test)]` code in the
  desktop binary. No A/V sync exists.
- **Preview and export disagree.** Preview resolves one topmost video clip and
  holds `Option<PathBuf>`; export builds a separate FFmpeg filtergraph. Neither
  renders transitions, titles, captions, transforms or per-clip audio parameters.
- **Direct manipulation is incomplete.** Ruler and track-lane mouse handlers scrub
  the playhead; clips have selection clicks but no drag-to-move and no edge-trim
  handles. Trimming is available only through fixed-step toolbar buttons that
  dispatch `TrimClip`. `Track::add_clip` also allows overlapping clips on one
  lane, and `TrimClip::apply` does not validate invariants.
- **Transport state is duplicated** across `TimelinePanel`, `PreviewMonitorPanel`,
  `WorkspaceLayoutState` and `EditorSession` / `PlaybackController`.

**Phase A (2026-11-12) stabilized the existing path**: central FFmpeg/FFprobe
resolution (GUI `PATH` fix), frame-accurate sequence/asset fps retiming in preview
and export, overlay clips no longer blanking preview, single-flight preview
rendering, wall-clock playback with no overshoot, exact frame stepping, and clip
clicks no longer scrubbing the playhead. Verified: `cargo fmt --all -- --check`,
`cargo test --workspace` (123 passed, 0 failed),
`cargo clippy --workspace --all-targets -- -D warnings` and
`cargo build --workspace` all clean on macOS aarch64 with rustc 1.99.1.

**Phase B is a design proposal only** (`docs/PLAYBACK_ENGINE_DESIGN.md`, status
PROPOSED). §10 requires sign-off on decisions D1–D7 before any new engine is
implemented. No Phase B code exists.

Pinned stack: `gpui-kit 0.9.0` (`gpui-pre 0.3.11`), Rust MSRV `1.92.0`, edition `2024`. Dependencies are centralized in workspace `Cargo.toml` with `Cargo.lock` committed.
Repository scaffolding includes: `.gitignore`, `rustfmt.toml`, `README.md`, `LICENSE` (Apache-2.0), and GitHub Actions CI (`.github/workflows/ci.yml`).
Current verification status: verified locally on macOS (`aarch64-apple-darwin`). Windows and Linux builds are configured in CI for automated cross-platform verification.
Domain logic for agent adapters (M4) and AI editing workflows (M5) will be created in their respective milestones. The `skills/` folder holds two vendored GPUI Kit reference skills (`gpui-kit`, `gpui-kit-design-guides`) used as development documentation for GPUI and UI work. `docs/GPUI_KIT_GUIDELINES.md` is the practical guide for building GPUI views in this repo. When new crates land, update this file to reflect reality.

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
│   ├── GPUI_KIT_GUIDELINES.md    # GPUI Kit / GPUI UI implementation guideline
│   └── PLAYBACK_ENGINE_DESIGN.md # Phase B real playback engine (PROPOSED)
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
list_composition_templates · add_composition
apply_effect · set_clip_transform · set_clip_audio · render_preview · export_video
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
| M2 | Timeline engine: multi-track editing, drag and drop, trim, split, snapping, playhead, selection, undo/redo | Drag a clip to a track, trim both edges, split at playhead, ripple delete; undo/redo restores exact state; times are exact at frame boundaries | **Partial on macOS — acceptance test NOT met.** Pure deterministic `crates/timeline` domain, canonical `EditorCommand` & `CommandHistory`, frame-accurate snapping, serialization and undo/redo are complete. The GPUI timeline has ruler/lane scrubbing, razor split, drop-to-add, fixed-step trim buttons and clip selection, but **no drag-to-move or edge-trim handles**; overlapping clips are allowed on one lane; `TrimClip::apply` lacks invariant validation. Clip clicks no longer scrub the playhead (Phase A A6). |
| M3 | Preview and export: playback, scrubbing, A/V sync, effects pipeline, MP4 export | **First vertical slice: import video → drag to timeline → trim → preview → export MP4**, with output playable in a third-party player | **Partial on macOS — acceptance test NOT met.** Dynamic FFmpeg multitrack video/audio filtergraph MP4 export (with sequence/asset fps retiming after Phase A), `Cmd+E`/`Ctrl+E`, off-thread execution and progress reporting work. **Playback is not real-time**: one FFmpeg process + one JPEG per frame, no audio output backend, no A/V sync, no GPU compositing. Preview shows one topmost clip and does not match export for transitions, titles, captions or transforms. Phase A added wall-clock playback, coalesced rendering and no-overshoot. See `docs/PLAYBACK_ENGINE_DESIGN.md`. |
| M4 | AI integration: ACP, Codex app-server, Claude CLI adapter, provider selector, conversation UI, skill picker, MCP editor tools | Two different providers can run the same task; every agent mutation appears in undo history; disconnecting an agent leaves the editor fully functional | **Completed on macOS** (Pluggable `AgentProvider`, ACP adapter for Antigravity & Gemini CLI, offline Mock provider, MCP tool surface over `EditorCommand` & `CommandHistory`, interactive approvals, AI Studio dockable panel; CI configured for Windows/Linux) |
| M5 | AI editing workflows: smart cuts, captions, animated titles, social-video formats, natural-language editing | A prompt produces a reviewable sequence, a preview render, and only commits after user approval | **Completed on macOS** (Atomic `BatchCommand` & `StagedChangeset` engine, speculative sequence diff projection, production skills: smart-cuts, auto-captions, social-video, animated-titles; Codex app-server & Claude Code CLI adapters; CI configured for Windows/Linux) |

Integrate **Codex and Antigravity** through their supported protocols before adding the more complex AI editing workflows.

## 9. Conventions for agents working in this repo

- **Rust**: workspace-level edition, MSRV and lint config live in the root `Cargo.toml`; do not override them per crate.
- **Errors**: `thiserror` for domain errors in library crates; `anyhow` only at application boundaries. No `unwrap()` / `expect()` in library crates unless the invariant is provably impossible — and then document why inline.
- **Async**: tokio for agent I/O and long-running tasks. Keep timeline model and command application synchronous and pure so replay/undo stay deterministic.
- **IDs**: opaque strongly-typed newtypes (`ClipId`, `TrackId`, `SessionId`, `ProjectId`), never raw strings or integers in public APIs.
- **Tests**: unit tests for timeline invariants (trim, split, ripple, snapping, frame-exact arithmetic); project schema migration tests; adapter contract tests using recorded JSON-RPC fixtures. **CI must not require live CLI logins, network access or audio hardware.** Add an acceptance test with every milestone.
- **Unit tests in `apps/desktop`**: `use gpui_kit::*;` re-exports GPUI's `#[gpui::test]` proc-macro under the bare name `test`. In any `#[cfg(test)] mod tests` that uses `use super::*;`, write `#[core::prelude::v1::test]` instead of `#[test]` (as `main.rs` already does). A plain `#[test]` resolves to the GPUI wrapper, recurses infinitely, and crashes rustc with SIGBUS.
- **Cross-platform**: no platform-specific APIs in `crates/*`; macOS/Windows/Linux specifics belong in `apps/desktop` behind traits or `cfg`.
- **Dependencies**: adding a dependency to a core crate requires a short justification recorded in the PR/commit and, if it changes the architecture, here.
- **Commands**: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. These are the intended CI contract; set them up in M0 and update this section with the real commands once CI exists.
- **Docs**: when you change the architecture, crate boundaries, command surface or milestone status, update this file and the affected crate README in the same change. An AGENTS.md that drifts from the code is worse than none.

## 10. When in doubt

Propose a design before adding a new engine, protocol, timeline or command path. Specifically: **do not** introduce a second timeline, a second mutation path, provider-specific logic in `editor-core`, credential handling in the editor, or float-second time in the model. Ask the user rather than guessing on those.

Phase B (a real playback engine) is such a change. `docs/PLAYBACK_ENGINE_DESIGN.md` is the proposal and is **design-only**; its decisions D1–D7 need explicit user sign-off before B1 begins. In particular, D4 asks whether transport state (playhead / playing / loop) is exempt from `EditorCommand` as ephemeral view state.

Suggested next artifact: review and sign off `docs/PLAYBACK_ENGINE_DESIGN.md`. A separate implementation-ready technical specification covering project JSON schemas and agent protocol interfaces remains outstanding.
