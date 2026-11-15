# NodaVideo Core Replacement Plan — OpenCut Port

**Strategy:** Replace NodaVideo's `crates/` and `apps/` with OpenCut's `rust/libs/` and `rust/apps/`, then re-add NodaVideo-specific features (AI agents, skills, MCP) on top.

**Status:** IMPLEMENTED — Phases 1 and 2 are complete and the editor runs on the
OpenCut core. Phase 3 AI-panel integration is in progress (assistant dock panel
shipped; approval gating of MCP tools still pending).

**Verified 2026-11-13:** `cargo check --all-targets`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `cargo test
--workspace` all pass. The editor binary launches and stays running. Tests that
need local media expect `.testdata/super-long.mp4` (repo root, gitignored);
generate a stand-in with FFmpeg if it is absent.

---

## 0. Executive Summary

NodaVideo's domain layer (timeline model, command system, agent adapters) is architecturally sound but its playback engine is fundamentally broken — it spawns one FFmpeg process per frame (~37.5 ms/frame), has no audio output, no GPU compositing, and preview/export don't agree. OpenCut solves all of these with `ffmpeg-next` FFI decoding, VideoToolbox on macOS, Metal GPU conversion, `cpal` audio output, and a shared decoder path for preview + export.

This plan replaces NodaVideo's non-working editor core with OpenCut's working code while preserving NodaVideo's AI-specific crates (agent-runtime, agent-acp, agent-codex, agent-claude, mcp-server, skill-registry, hyperframes-bridge) as a layer on top. The result is a working video editor with AI capabilities.

**Key trade-off:** NodaVideo's `EditorCommand` system (undo/redo, approval gating, replay) is discarded. OpenCut has its own `EditAction`/`EditingState` mutation model. The MCP server will need to be rewritten to target OpenCut's mutation API instead of `EditorCommand`. This is the biggest piece of integration work.

---

## 1. What Gets Replaced vs What Stays

### 1.1 REPLACED (NodaVideo → OpenCut)

| NodaVideo path | OpenCut replacement | Reason |
|---|---|---|
| `crates/media-engine/` | `libs/media-backend/` + `libs/engine/` (probe, decode, GPU) | OpenCut uses FFI + VideoToolbox; NodaVideo spawns CLI processes |
| `crates/timeline/` | `libs/timeline/` | OpenCut's model is working and tested; NodaVideo's has no working playback |
| `crates/audio-engine/` | `libs/player-ui/` (audio_output, audio_player) | OpenCut has real `cpal` playback; NodaVideo has mixing primitives only |
| `crates/gpu-renderer/` | `libs/engine/` (gpu.rs, cpu.rs) | OpenCut has Metal compute shaders; NodaVideo has wgpu stubs |
| `crates/editor-core/` | `apps/editor/` (editor.rs, editing.rs) | OpenCut's EditorState is wired to real UI; NodaVideo's EditorSession is disconnected |
| `crates/command-system/` | `apps/editor/` (edit_action.rs, editing.rs) | OpenCut has its own EditAction/EditingState mutation model |
| `apps/desktop/` | `apps/editor/` + `apps/player/` | OpenCut's editor is working with full timeline UI, export, transcription |

### 1.2 STAYS (NodaVideo-specific, no OpenCut equivalent)

| NodaVideo path | Purpose | Re-integration plan |
|---|---|---|
| `crates/agent-runtime/` | `AgentProvider` trait, `AgentRuntime`, `MockAgentProvider` | Add as workspace dep to `apps/editor`; wire into a dockable panel |
| `crates/agent-acp/` | ACP JSON-RPC 2.0 adapter (Antigravity, Gemini CLI) | Same |
| `crates/agent-codex/` | Codex app-server JSON-RPC adapter | Same |
| `crates/agent-claude/` | Claude Code CLI streaming adapter | Same |
| `crates/mcp-server/` | MCP editor tool schemas + executor | **Rewrite required:** adapt tool implementations to OpenCut's `EditingState` mutation API instead of `EditorCommand` |
| `crates/skill-registry/` | SKILL.md parser, discovery, prompt composer | Add as workspace dep; load skills from `skills/` directory |
| `crates/hyperframes-bridge/` | Motion graphics templates, HTML/SVG composition | **Adapt:** render via OpenCut's text clip system or as a video surface overlay |
| `skills/` (6 production skills) | smart-cuts, auto-captions, social-video, animated-titles, audio-cleanup, color-grading | Keep as-is; skill-registry loads them |
| `templates/` (8 JSON templates) | Motion graphics template bundles | Keep; hyperframes-bridge consumes them |
| `sidecars/hyperframes/` | Node.js sidecar renderer | Keep; hyperframes-bridge invokes it |
| `docs/GPUI_KIT_GUIDELINES.md` | GPUI development guide | **Update:** OpenCut uses vendored Zed GPUI, not gpui-kit |
| `skills/gpui-kit/`, `skills/gpui-kit-design-guides/` | GPUI Kit reference skills | **Remove or archive:** no longer relevant (OpenCut uses Zed GPUI) |
| `AGENTS.md` | Agent instructions | **Rewrite** to reflect new architecture |
| `README.md` | Project readme | **Rewrite** |
| `.gitignore` | Git ignore patterns | **Merge** with OpenCut's patterns |
| `rustfmt.toml` | Rust formatting config | **Replace** with OpenCut's config |

### 1.3 NEW (from OpenCut, not in NodaVideo)

| OpenCut path | Purpose |
|---|---|
| `libs/transcribe/` | Audio transcription (OpenAI Whisper API) |
| `libs/waveform/` | Audio waveform extraction via FFmpeg |
| `libs/platform-fs/` | Platform-specific file system operations |
| `libs/player-ui/` | Player UI components (video player, timeline player, audio output) |
| `apps/cli/` | Headless CLI for probing, validating, exporting timelines |
| `apps/player/` | Standalone debugging player |
| `vendor/zed/` | Vendored Zed GPUI crate (git submodule) |
| `vendor/ffmpeg-8.1.2/` | Vendored FFmpeg development libraries |
| `.cargo/` | Platform-specific cargo config (FFmpeg paths, runners) |
| `design/spec.md` | OpenCut design spec |
| `.agents/skills/` | OpenCut development skills (rust-style, opencut-conventions, architecture, general-rules, ui-design) |

---

## 2. Dependency Reconciliation

### 2.1 GPUI: Zed GPUI replaces gpui-kit

**Current state:** NodaVideo depends on `gpui-kit = "0.9.0"` / `gpui-pre = "0.3.11"` from crates.io. OpenCut depends on a vendored copy of Zed's GPUI at `vendor/zed/crates/gpui` (git submodule pointing to `https://github.com/zed-industries/zed.git`).

**Decision:** Switch to OpenCut's vendored Zed GPUI. This is non-negotiable — OpenCut's entire UI, surface rendering, and export pipeline depend on Zed GPUI APIs. gpui-kit is a different, incompatible fork.

**Impact:**
- The vendored `gpui-kit` reference skills (`skills/gpui-kit/`, `skills/gpui-kit-design-guides/`) and `docs/GPUI_KIT_GUIDELINES.md` become obsolete. They should be archived (not deleted) since they contain useful GPUI concepts, but their API specifics won't apply.
- The `[profile.dev.package]` entries for `gpui-pre`, `gpui-component`, `gpui-kit`, `gpui-kit-assets`, `gpui-pre-platform` must be removed; replace with OpenCut's `[profile.dev.package."*"] opt-level = 1`.
- The `[patch."https://github.com/zed-industries/zed"]` section must be included in the merged Cargo.toml.

### 2.2 FFmpeg: ffmpeg-next FFI replaces CLI spawning

**Current state:** NodaVideo resolves `ffmpeg`/`ffprobe` binaries at runtime and spawns CLI processes. OpenCut uses `ffmpeg-next = "8.1"` (Rust FFI bindings) linked against vendored FFmpeg 8.1.2 libraries.

**Decision:** Switch to `ffmpeg-next` FFI. This is the core architectural change that makes the editor actually work.

**Impact:**
- The vendored FFmpeg libraries at `vendor/ffmpeg-8.1.2/` must be present. OpenCut's README says "Existing FFmpeg development libraries in `rust/vendor/ffmpeg-8.1.2/`; do not build FFmpeg locally."
- The `.cargo/macos.toml` and `.cargo/windows.toml` config files must be copied — they set `FFMPEG_DIR` and `PKG_CONFIG_PATH`.
- The `scripts/cargo-cli.sh` runner (sets `DYLD_LIBRARY_PATH` at runtime) must be copied.
- NodaVideo's `crates/media-engine/src/binaries.rs` (CLI binary resolution) is deleted — no longer needed.
- All `#[cfg(test)]` code that spawns FFmpeg CLI must be removed or rewritten.

### 2.3 Metal replaces wgpu (macOS)

**Current state:** NodaVideo has `crates/gpu-renderer/` with wgpu stubs. OpenCut uses `metal = "0.33"` with a `convert.metal` compute shader for YUV→BGRA conversion.

**Decision:** Use OpenCut's Metal pipeline on macOS, CPU fallback on other platforms. wgpu is removed.

### 2.4 cpal replaces (nothing)

**Current state:** NodaVideo has no audio output at all. OpenCut uses `cpal = "0.17.1"` for real-time audio playback.

**Decision:** Adopt OpenCut's `cpal`-based audio output from `libs/player-ui/`.

### 2.5 Dependency version reconciliation

Merge OpenCut's workspace dependencies into the combined Cargo.toml. Key additions:
- `ffmpeg-next = "8.1"`
- `gpui = { path = "vendor/zed/crates/gpui" }`
- `gpui_platform`, `gpui_tokio` (Zed GPUI companion crates)
- `cpal = "0.17.1"`
- `metal = "0.33"`, `core-video = "0.5.2"`, `core-foundation = "0.10"`, `block2 = "0.6"`
- `objc2-app-kit`, `objc2-foundation` (macOS)
- `ulid = { version = "3", features = ["serde"] }` (replaces NodaVideo's `uuid`)
- `resvg = "=0.45.1"` (SVG rendering for text clips)
- `notify = "=8.2.1"` (file watcher for project directory)
- `schemars = { version = "=1.2.2", features = ["derive"] }` (JSON schema generation)
- `smallvec = "1"`, `unicode-segmentation = "1"`, `unicode-script = "0.5.8"`
- `reqwest = { version = "=0.12.24", ... }` (for transcription API)

**Removed from NodaVideo's deps:**
- `gpui-kit`, `gpui-pre`, `gpui-component`, `gpui-kit-assets`, `gpui-pre-platform`
- `wgpu` (replaced by Metal)
- `rusqlite` (OpenCut uses file-based projects, no SQLite cache)
- `uuid` (replaced by `ulid`)
- `chrono` (OpenCut doesn't use it)
- `async-trait` (not needed; OpenCut doesn't use it)

**Kept from NodaVideo's deps:**
- `tokio` (upgrade to `rt-multi-thread` features)
- `anyhow`, `serde`, `serde_json`, `image`, `tempfile`, `futures`
- NodaVideo's AI crate dependencies (`thiserror`, etc.)

### 2.6 Workspace lint reconciliation

NodaVideo has extensive `[workspace.lints]` with `unsafe_code = "forbid"`. OpenCut uses `unsafe` extensively (VideoToolbox FFI, Metal, CoreVideo). **Decision:** Change `unsafe_code` from `"forbid"` to `"allow"` (or `"warn"` with `#[allow(unsafe_code)]` on specific modules). This is required for OpenCut's macOS code to compile.

---

## 3. Workspace Cargo.toml Merge

The merged root `Cargo.toml` will live at the repository root (not in a `rust/` subdirectory — OpenCut's workspace is at `rust/Cargo.toml`, but NodaVideo's is at the root). We move everything up one level.

### 3.1 Merged workspace members

```toml
[workspace]
resolver = "3"
members = [
    # Editor core (from OpenCut)
    "libs/media-backend",
    "libs/timeline",
    "libs/engine",
    "libs/transcribe",
    "libs/player-ui",
    "libs/waveform",
    "libs/platform-fs",
    # Applications (from OpenCut)
    "apps/editor",
    "apps/player",
    "apps/cli",
    # AI layer (from NodaVideo, preserved)
    "crates/agent-runtime",
    "crates/agent-acp",
    "crates/agent-codex",
    "crates/agent-claude",
    "crates/mcp-server",
    "crates/skill-registry",
    "crates/hyperframes-bridge",
]
exclude = ["vendor"]
```

### 3.2 Workspace package metadata

```toml
[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.92.0"
license = "Apache-2.0"
authors = ["NodaVideo Contributors"]
publish = false
```

### 3.3 Patch section (required for vendored Zed GPUI)

```toml
[patch."https://github.com/zed-industries/zed"]
gpui = { path = "vendor/zed/crates/gpui" }
gpui_macros = { path = "vendor/zed/crates/gpui_macros" }
gpui_platform = { path = "vendor/zed/crates/gpui_platform" }
```

### 3.4 Profiles

```toml
[profile.dev.package."*"]
opt-level = 1

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true
```

---

## 4. Module-by-Module Mapping

### 4.1 Core engine mapping

| NodaVideo module | → | OpenCut module | Migration notes |
|---|---|---|---|
| `media-engine::probe` | → | `media-backend::media_info` + `engine::probe` | OpenCut uses `ffmpeg-next` FFI, not `ffprobe` CLI |
| `media-engine::thumbnail` | → | `engine::image` + `media-backend::video` | Decoded frame → image, no CLI |
| `media-engine::waveform` | → | `waveform::lib` | OpenCut uses `ffmpeg-next` audio extraction |
| `media-engine::preview` | → | `engine::timeline_decoder` + `engine::timeline_backend` | Persistent `VideoReader` per track/asset; ~1.25 ms/frame |
| `media-engine::export` | → | `engine::export` + `engine::export_encoder` | Headless GPUI window → FFmpeg encode; same decoder path as preview |
| `media-engine::cache` | → | (removed) | OpenCut uses in-memory caching; no SQLite |
| `media-engine::binaries` | → | (removed) | No CLI binary resolution needed |
| `media-engine::time_math` | → | `timeline::time` (`FrameRate` methods) | OpenCut's `FrameRate` has `seconds()`, `duration()`, `audio_samples()`, `rescale_nearest()` |
| `audio-engine::*` | → | `player-ui::audio_output` + `player-ui::audio_player` | Real `cpal` playback |
| `gpu-renderer::*` | → | `engine::gpu` (macOS) + `engine::cpu` (fallback) | Metal compute shader `convert.metal` |
| `editor-core::session` | → | `apps/editor::editor` (`Editor` struct) | OpenCut's Editor owns TimelineBackend, TimelineDecoder, project state |
| `editor-core::playback` | → | `engine::timeline_backend` (`TimelineBackend`, `PlaybackClock`) | Wall-clock anchored playback |
| `editor-core::export` | → | `engine::export` (`export_timeline()`) | Headless GPUI render |
| `editor-core::selection` | → | `apps/editor::editing` (`EditingState`) | Selection is part of editing state |
| `command-system::*` | → | `apps/editor::edit_action` + `apps/editor::editing` | `EditAction` enum replaces `EditorCommand`; `EditingState` handles mutations |
| `timeline::*` | → | `libs/timeline::*` | Full replacement: `Clip` enum, `Track`, `TimelineEditingState`, `TimelineSerialization` |
| `project::*` | → | `apps/editor::project` + `libs/timeline::serialization` | OpenCut uses folder-based projects (timeline is a `.timeline` file), not NodaVideo's JSON document + SQLite |

### 4.2 Timeline model differences

| Concept | NodaVideo | OpenCut | Migration |
|---|---|---|---|
| Frame indexing | `Frame(i64)` newtype in `timeline::sequence` | `TimelineFrameIndex(i64)` newtype in `timeline::time` | Direct replacement |
| Frame rate | `FrameRate { num, den }` in `project::time` | `FrameRate { numerator, denominator }` in `timeline::time` | Rename fields; same semantics |
| Clip ID | `ClipId(Uuid)` | `Ulid` (embedded in `Clip` enum variants) | Use `Ulid` throughout |
| Track ID | `TrackId(Uuid)` | `Ulid` (embedded in `Track`) | Use `Ulid` throughout |
| Asset ID | `AssetId(Uuid)` | `Ulid` (embedded in `Asset`) | Use `Ulid` throughout |
| Clip types | Separate structs: `Clip`, `TextOverlay`, `Transition` | `Clip` enum: `Video`, `Audio`, `Text` | Map NodaVideo's overlays to `Clip::Text`; transitions are OpenCut's crossfade between clips |
| Persistence | JSON document (`schema_version = 1`) + SQLite cache | Folder-based project (`.timeline` JSON file, media by relative path) | Use OpenCut's folder-based approach |
| Transitions | `EditCutTransition` with variants (crossfade, dissolve, fade-to-black, dip-to-white, wipe) | Not explicitly modeled (clips are placed on tracks; crossfade is implicit) | **Deferred:** add transition support as a future feature |
| Snapping | `timeline::snap` module | Not present in OpenCut | **Deferred:** re-add snapping if needed |

### 4.3 UI mapping

| NodaVideo panel | → | OpenCut equivalent | Notes |
|---|---|---|---|
| `workspace/panels/media_browser` | → | `apps/editor::explorer` + `explorer_view` + `explorer_drag` | File explorer with drag-and-drop |
| `workspace/panels/preview_monitor` | → | `apps/editor::preview` + `preview_events` + `preview_image` + `preview_text` | Full preview with timeline playback |
| `workspace/panels/timeline` | → | `apps/editor::timeline` + `timeline_clip` + `timeline_interactions` + `timeline_trim` + `timeline_ui` + `timeline_document` | Full timeline editing |
| `workspace/panels/inspector` | → | `apps/editor::property_panel` + 12 property panel variants | Rich property editing |
| `workspace/panels/assistant` | → | (new — to be built) | AI assistant panel using NodaVideo's agent crates |
| `workspace/toolbar` | → | `apps/editor::layout` + `apps/editor::actions` | Toolbar and action dispatch |
| `workspace/menus` | → | `apps/editor::layout` + `apps/editor::main` | Menu setup |
| `workspace/navigation` | → | `apps/editor::layout` | Navigation tabs |
| `workspace/layout` | → | `apps/editor::layout` | Panel layout |
| `theme/mod` | → | `apps/editor::theme` | Theme |

### 4.4 OpenCut features not in NodaVideo (new capabilities)

| Feature | OpenCut module | Value |
|---|---|---|
| Transcription | `libs/transcribe` + `apps/editor::transcription_window` | AI-powered subtitle generation via Whisper API |
| SRT subtitle support | `apps/editor::srt` | Import/edit SRT files |
| Project picker | `apps/editor::project_picker` | Browse and open folder-based projects |
| Missing media dialog | `apps/editor::dialog_missing_media` | Handle missing media gracefully |
| Global settings dialog | `apps/editor::global_settings_dialog` | Editor preferences |
| Context menus | `apps/editor::context_menu` + `timeline_clip_menu` + `timeline_track_menu` | Right-click menus |
| GPUI inspector | `apps/editor::gpui_inspector` | Debug UI tree |
| Standalone player | `apps/player` | Debug video/audio/timeline playback |
| Headless CLI | `apps/cli` | Probe, validate, export from command line |
| UI components | `apps/editor::ui-components` | Reusable: cyberpunk_button, horizontal/vertical_split, text_area, text_input |

---

## 5. NodaVideo Features: Deferred vs Immediately Re-added

### 5.1 Immediate (Phase 1 — working editor)

These are done by the replacement itself — no additional work beyond copying OpenCut code:

- ✅ Real-time video playback with persistent decoder (~1.25 ms/frame)
- ✅ Audio playback via `cpal`
- ✅ GPU-accelerated YUV→BGRA conversion (Metal on macOS)
- ✅ Multi-track timeline editing (drag, trim, split, blade)
- ✅ Preview monitor with real compositing
- ✅ MP4 export (headless GPUI render → FFmpeg encode)
- ✅ Audio waveform display
- ✅ Text clips with SVG rendering
- ✅ Folder-based project persistence
- ✅ File explorer with drag-and-drop
- ✅ Property panels for all clip types
- ✅ Context menus
- ✅ Transcription (Whisper API)
- ✅ SRT subtitle import

### 5.2 Phase 2 — AI layer integration

After the editor is working, re-add NodaVideo's AI crates:

| Task | Crates involved | Effort | Notes |
|---|---|---|---|
| Add agent crates to workspace | agent-runtime, agent-acp, agent-codex, agent-claude | Low | Add to Cargo.toml, fix any dependency conflicts |
| Build AI assistant dockable panel | apps/editor (new module) | Medium | New GPUI panel; chat message history, thought/tool-call inspection |
| Wire agent provider cycling | apps/editor (new module) | Medium | Provider selection, streaming events |
| Interactive approval cards | apps/editor (new module) | Medium | For staged edits proposed by agents |
| Rewrite MCP server | mcp-server | **High** | All tool implementations must target OpenCut's `EditingState` instead of `EditorCommand`. Tools: `get_project`, `get_timeline`, `add_clip`, `move_clip`, `trim_clip`, `split_clip`, `delete_clip`, `add_text_overlay`, `export_video`, `preview_changeset`, `propose_changeset` |
| Wire skill-registry | skill-registry | Low | Load skills from `skills/` directory |
| Skill badge indicator | apps/editor | Low | UI element showing active skills |

### 5.3 Phase 3 — Advanced features

| Task | Effort | Notes |
|---|---|---|
| HyperFrames bridge integration | Medium | Render motion graphics as video surface overlays or text clips with SVG templates |
| Production skills activation | Low | `skills/smart-cuts`, `auto-captions`, `social-video`, `animated-titles`, `audio-cleanup`, `color-grading` — loaded by skill-registry |
| Template system | Medium | Adapt `templates/` JSON to work with OpenCut's clip system |
| Undo/redo history | Medium | OpenCut has `EditAction`; add a history stack similar to NodaVideo's `CommandHistory` |
| Approval gating for agent edits | Medium | Intercept agent mutations with preview/approve flow |
| Snapping engine | Low | Re-add from NodaVideo's `timeline::snap` if needed |
| Transitions (crossfade, dissolve, etc.) | Medium | OpenCut doesn't model transitions explicitly; add as a feature |
| Cross-platform (Windows, Linux) | High | OpenCut is macOS-first; Windows has partial support; Linux needs CPU fallback (no Metal) |

---

## 6. Directory Structure After Migration

```
NodaVideo/
├── .cargo/
│   ├── config.toml          # from OpenCut (cargo aliases)
│   ├── macos.toml           # from OpenCut (FFmpeg paths for macOS)
│   ├── windows.toml         # from OpenCut (FFmpeg paths for Windows)
│   ├── cli.toml             # from OpenCut (CLI tool config)
│   └── ffmpeg-windows.toml  # from OpenCut
├── .gitignore               # merged
├── .gitmodules              # from OpenCut (zed submodule)
├── AGENTS.md                # rewritten
├── README.md                # rewritten
├── Cargo.toml               # merged workspace root
├── Cargo.lock               # regenerated
├── rustfmt.toml             # from OpenCut
├── LICENSE                  # kept
│
├── apps/
│   ├── editor/              # from OpenCut (working editor)
│   ├── player/              # from OpenCut (debugging player)
│   └── cli/                 # from OpenCut (headless CLI)
│
├── crates/                  # NodaVideo AI layer (preserved)
│   ├── agent-runtime/
│   ├── agent-acp/
│   ├── agent-codex/
│   ├── agent-claude/
│   ├── mcp-server/          # requires rewrite (Phase 2)
│   ├── skill-registry/
│   └── hyperframes-bridge/  # requires adaptation (Phase 3)
│
├── libs/                    # from OpenCut (working engine)
│   ├── media-backend/
│   ├── timeline/
│   ├── engine/
│   ├── transcribe/
│   ├── player-ui/
│   ├── waveform/
│   └── platform-fs/
│
├── vendor/
│   ├── zed/                 # git submodule (GPUI source)
│   └── ffmpeg-8.1.2/        # vendored FFmpeg libraries
│
├── scripts/
│   ├── cargo-cli.sh         # from OpenCut (runtime library path setup)
│   └── cargo-ffmpeg-windows.cmd  # from OpenCut
│
├── design/
│   └── spec.md              # from OpenCut (design spec)
│
├── skills/                  # NodaVideo production skills (preserved)
│   ├── smart-cuts/
│   ├── auto-captions/
│   ├── social-video/
│   ├── animated-titles/
│   ├── audio-cleanup/
│   └── color-grading/
│
├── templates/               # NodaVideo template bundles (preserved)
│   ├── titles/
│   ├── lower-thirds/
│   ├── social/
│   └── transitions/
│
├── sidecars/
│   └── hyperframes/         # NodaVideo sidecar (preserved)
│
├── docs/
│   ├── MIGRATION_PLAN.md    # this document
│   └── GPUI_KIT_GUIDELINES.md  # archived (mark as deprecated)
│
└── .workbuddy-ai/           # agent memory (not tracked in git)
```

**Removed directories:**
- `crates/media-engine/` — replaced by `libs/media-backend/` + `libs/engine/`
- `crates/timeline/` — replaced by `libs/timeline/`
- `crates/audio-engine/` — replaced by `libs/player-ui/`
- `crates/gpu-renderer/` — replaced by `libs/engine/` (gpu.rs, cpu.rs)
- `crates/editor-core/` — replaced by `apps/editor/`
- `crates/command-system/` — replaced by `apps/editor/` (edit_action.rs, editing.rs)
- `crates/project/` — replaced by `apps/editor/` (project.rs) + `libs/timeline/` (serialization.rs)
- `skills/gpui-kit/` — archived (no longer relevant)
- `skills/gpui-kit-design-guides/` — archived (no longer relevant)
- `docs/PLAYBACK_ENGINE_DESIGN.md` — removed (problem solved by OpenCut)

---

## 7. Execution Plan (Step-by-Step)

### Step 1: Create migration branch
```bash
git checkout -b migrate/opencut-core
```

### Step 2: Copy OpenCut's directory structure into NodaVideo root

Move the following from `opencut/rust/` to the repo root:
- `libs/` → `libs/`
- `apps/` → `apps/` (replaces existing `apps/desktop/`)
- `.cargo/` → `.cargo/`
- `rustfmt.toml` → `rustfmt.toml` (overwrite)
- `.gitmodules` → `.gitmodules` (add zed submodule)

Move from `opencut/` to repo root:
- `design/` → `design/`
- `scripts/` → `scripts/` (only the shell/cmd scripts, not the .ts file)

### Step 3: Remove replaced NodaVideo directories
```bash
rm -rf crates/media-engine crates/timeline crates/audio-engine
rm -rf crates/gpu-renderer crates/editor-core crates/command-system
rm -rf crates/project
rm -rf apps/desktop
rm -rf docs/PLAYBACK_ENGINE_DESIGN.md
```

### Step 4: Set up vendor directories

Initialize the Zed GPUI submodule:
```bash
git submodule add https://github.com/zed-industries/zed.git vendor/zed
```

Copy vendored FFmpeg libraries to `vendor/ffmpeg-8.1.2/` (must be obtained separately — OpenCut's README says "Existing FFmpeg development libraries in `rust/vendor/ffmpeg-8.1.2/`").

### Step 5: Write merged Cargo.toml

Combine workspace members, dependencies, patches, and profiles as described in §3. Key entries:
- `members` = OpenCut libs + apps + NodaVideo AI crates
- `exclude = ["vendor"]`
- `[patch."https://github.com/zed-industries/zed"]` section
- Merged `[workspace.dependencies]` (OpenCut + NodaVideo AI deps)
- `unsafe_code` changed from `"forbid"` to `"allow"` (or remove the lint)

### Step 6: Update AI crate Cargo.toml files

Each preserved NodaVideo AI crate needs its dependencies updated:
- Remove dependencies on deleted crates (`nodavideo-project`, `nodavideo-timeline`, `nodavideo-command-system`, `nodavideo-media-engine`, etc.)
- Add dependencies on OpenCut crates where needed (`timeline`, `engine`)
- Remove `gpui-kit` dependency
- Update `uuid` → `ulid` if IDs are used
- The crates that don't depend on the editor core at all (`agent-runtime`, `agent-acp`, `agent-codex`, `agent-claude`, `skill-registry`) should compile with minimal changes — they're protocol/adapter crates
- `mcp-server` and `hyperframes-bridge` need the most changes (they reference NodaVideo's timeline/command types)

### Step 7: Delete Cargo.lock and regenerate
```bash
rm Cargo.lock
cargo generate-lockfile --config .cargo/macos.toml
```

### Step 8: Build verification

```bash
# Check compilation (macOS)
cargo check --config .cargo/macos.toml --all-targets

# Run tests
cargo test-mac

# Build editor
cargo build-editor-mac

# Run editor
cargo editor-mac
```

### Step 9: Update documentation
- Rewrite `AGENTS.md` to reflect the new architecture
- Rewrite `README.md` with build instructions
- Merge `.gitignore` (add OpenCut's patterns)
- Archive `docs/GPUI_KIT_GUIDELINES.md` (mark as deprecated)
- Update `.workbuddy-ai/memory/MEMORY.md` with the new architecture

### Step 10: Commit
Create a single atomic commit (or a small series) on the migration branch. Suggested commit message:
```
Replace editor core with OpenCut engine

Adopt OpenCut's working video engine (ffmpeg-next FFI, VideoToolbox,
Metal GPU, cpal audio) as NodaVideo's editor core. Remove broken
media-engine, timeline, audio-engine, gpu-renderer, editor-core,
command-system, and project crates. Preserve AI layer (agent-runtime,
agent-acp, agent-codex, agent-claude, mcp-server, skill-registry,
hyperframes-bridge) for Phase 2 integration.
```

### Step 11: Merge migration branch to main
After verification, merge:
```bash
git checkout main
git merge migrate/opencut-core
```

---

## 8. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Vendored Zed GPUI submodule is huge (>1GB) | High | Slow clone, large repo | Use `--depth 1` for submodule; consider shallow clone |
| FFmpeg 8.1.2 vendored libraries not available | Medium | Build fails | Must obtain separately; document the source |
| OpenCut's code uses APIs not in the Zed submodule version | Low | Build fails | Pin to the exact Zed commit OpenCut was developed against |
| AI crates don't compile after dep changes | Medium | Build fails | Fix incrementally; these crates are mostly protocol adapters with few external deps |
| `unsafe_code = "forbid"` conflicts with OpenCut | **Certain** | Build fails | Change to `"allow"` — required for VideoToolbox/Metal FFI |
| OpenCut macOS-only features don't work on Linux | High | No GPU decode/compositing | CPU fallback exists (`engine::cpu`); VideoToolbox is macOS-only by design |
| MCP server rewrite is complex | High | Delays Phase 2 | Phase 2 is deferred; editor works without it |
| OpenCut's folder-based projects differ from NodaVideo's JSON+SQLite model | Medium | Existing project files won't load | Accept: NodaVideo had no real users; OpenCut's model is better |
| `[patch]` section conflicts with crates.io versions | Low | Build fails | The patch only applies to the `zed-industries/zed` URL; should be clean |
| Workspace member path changes break `cargo` aliases | Low | Build fails | Cargo aliases use `-p editor` etc., which are package names, not paths |

---

## 9. What Is Lost

Being transparent about what gets discarded:

1. **`EditorCommand` system** — NodaVideo's canonical command enum with `BatchCommand`, `StagedChangeset`, `CommandHistory`, approval gating, and speculative preview diff projection. OpenCut has a simpler `EditAction`/`EditingState` model. The MCP server and approval-gated agent edits must be rebuilt against OpenCut's model.

2. **NodaVideo's transition system** — `EditCutTransition` with crossfade, dissolve, fade-to-black, dip-to-white, wipe variants. OpenCut doesn't model transitions explicitly. Must be re-added as a future feature.

3. **NodaVideo's snapping engine** — `timeline::snap` with clip-edge, playhead, and sequence-start snapping. OpenCut doesn't have this. Can be re-added.

4. **SQLite media cache** — `MediaCacheDb` with persistent thumbnail/waveform caching. OpenCut uses in-memory caching. Acceptable trade-off.

5. **Versioned JSON project document** — `schema_version = 1` with atomic persistence. OpenCut uses folder-based projects with `.timeline` files. OpenCut's approach is simpler and more user-friendly.

6. **gpui-kit integration** — The gpui-kit component library and its design system are no longer used. The vendored GPUI Kit reference skills become obsolete.

7. **Cross-platform parity (initially)** — OpenCut is macOS-first with partial Windows support. Linux support requires the CPU fallback path only (no VideoToolbox, no Metal).

---

## 10. Acceptance Criteria

The migration is complete when:

1. ✅ `cargo check --config .cargo/macos.toml --all-targets` passes with zero errors
2. ✅ `cargo test-mac` passes (at least the engine, timeline, and media-backend test suites)
3. ✅ `cargo editor-mac` launches the editor
4. ✅ A video file can be imported via the file explorer
5. ✅ A clip can be dragged onto the timeline
6. ✅ Playback produces real-time video (not one-frame-per-process) and audio
7. ✅ Timeline trimming, splitting, and clip moving work via direct manipulation
8. ✅ Export to MP4 produces a valid output file
9. ✅ The editor can be closed and reopened, loading the same project
10. ✅ NodaVideo AI crates (`agent-runtime`, `agent-acp`, `agent-codex`, `agent-claude`, `skill-registry`) compile as workspace members (even if not yet wired into the UI)
