# NodaVideo 🎬

> **Your timeline. Your creativity. Your AI.**

NodaVideo is an AI-powered desktop video editor: a native Rust + GPUI application built for cross-platform performance (macOS, Windows, and Linux). It combines a professional multi-track timeline with pluggable AI agents (Claude Code, Codex, Antigravity, and ACP-compatible providers) that can create, edit, and manage video projects under full user control.

---

## Product Direction

1. **Professional video editor first**, AI assistant second. Hiding the AI workspace leaves an uncompromising, fully featured desktop NLE.
2. **Complete user control.** Agents propose actions and draft edits; the user reviews, inspects, approves, or undoes them.
3. **Provider independence.** Antigravity, Codex, Gemini CLI, Claude Code, and ACP-compatible agents operate interchangeably without vendor lock-in.
4. **Deterministic command system.** Every state mutation passes through an undoable, replayable `EditorCommand` stream.

---

## Architectural Principles

- **Two decoupled engines:**
  - The **editor** (Rust + GPUI) owns all project documents, media bin assets, timeline editing, preview playback, and rendering.
  - The **agent runtime** orchestrates external AI CLIs via structured protocols (ACP, Codex app-server, MCP tool surface) and never directly mutates editor state.
- **Single mutation gateway:** No backdoor edits or direct disk writes. All actions—human or AI—are captured by `crates/command-system`.
- **Permissions in the editor:** Approval gates for destructive operations (delete, overwrite, export) are enforced by the command system and surfaced in the UI.
- **Heavy work off the UI thread:** Media decoding, effects compositing, audio mixing, transcription, and agent I/O run asynchronously off the main GPUI thread.

---

## Prerequisites

- **Rust toolchain:** Rust 1.92.0 or later (declared MSRV). Edition 2024.
- **macOS:**
  - macOS 15+ recommended.
  - Xcode Command Line Tools (`xcode-select --install`).
- **Windows:**
  - Windows 10 or later (64-bit).
  - Visual Studio 2022 C++ Build Tools (MSVC toolchain) and Windows SDK.
  - CMake available on `PATH`.
- **Linux (Ubuntu 24.04 / Debian):**
  - Wayland or X11 session with working Vulkan drivers.
  - Required packages:
    ```bash
    sudo apt update
    sudo apt install -y gcc g++ clang libfontconfig-dev libwayland-dev \
      libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
      libssl-dev libzstd-dev libasound2-dev vulkan-validationlayers libvulkan1
    ```

---

## Building and Running

Clone the repository and build the desktop application:

```bash
# Run the desktop application
cargo run -p nodavideo-desktop

# Run test suite across workspace
cargo test --workspace

# Run Clippy checks with warnings treated as errors
cargo clippy --workspace --all-targets -- -D warnings

# Verify formatting
cargo fmt --all -- --check
```

---

## Workspace Layout

```
NodaVideo/
├── apps/
│   └── desktop/                 # Native GPUI desktop application (nodavideo-desktop)
├── crates/                      # Core domain and protocol libraries (landing in M1–M4)
├── docs/                        # Architecture & GPUI Kit guidelines
├── skills/                      # Reference and extensible production skills
├── AGENTS.md                    # Canonical instructions and contracts for AI agents
├── Cargo.toml                   # Workspace manifest with centralized dependencies & lints
├── Cargo.lock                   # Pinned dependency graph
└── rustfmt.toml                 # Workspace formatting configuration
```

---

## Milestone Roadmap

- [x] **M0 (Foundation - Bootstrap):** Root workspace, centralized dependencies, MSRV 1.92+, GPUI Kit 0.9.0 desktop entry, clean startup & shutdown.
- [ ] **M0 (Desktop Foundation Complete):** Menus, dockable/resizable panels, keyboard shortcuts, themes, project persistence.
- [ ] **M1 (Media Management):** Asset bin, FFmpeg probing, thumbnails, audio waveforms, SQLite index cache.
- [ ] **M2 (Timeline Engine):** Multi-track editing, trimming, splitting, ripple edits, snapping, pure deterministic model.
- [ ] **M3 (Preview & Export):** Playback sync, wgpu compositing, first vertical slice (import → edit → export MP4).
- [ ] **M4 (AI Integration):** ACP adapter, Codex app-server adapter, Claude CLI stream, MCP tool surface.
- [ ] **M5 (AI Editing Workflows):** Smart cuts, auto-captions, motion graphics, prompt-to-edit workflows.

---

## License

Apache-2.0. See [LICENSE](LICENSE) for details.
