# NodaVideo

*Your timeline. Your creativity. Your AI.*

NodaVideo is a native desktop video editor for macOS and Windows, written in
Rust with [GPUI](https://github.com/zed-industries/zed). It pairs a professional
multi-track timeline with an AI assistant that can work through the agent CLI
you already use — Antigravity, Gemini CLI, Codex, Claude Code, OpenCode, or
Qoder — without locking you into a single provider.

The editor is the product. The AI workspace is a mode you can enter and leave;
everything you can do by hand is available with the assistant closed.

## Highlights

- **Native performance.** No Electron or web view. FFmpeg is linked directly
  through `ffmpeg-next`, decoding and playback run off the UI thread, and
  macOS uses VideoToolbox for decode and Metal for GPU frame conversion.
- **Frame-accurate timeline.** Multi-track video, audio, and text editing on
  integer frames with rational frame rates — never floating-point seconds.
- **Real preview, real export.** One shared decoder path drives both the
  preview and the export encoder, so what you see is what is rendered.
- **Provider-independent AI assistant.** A single `AgentProvider` interface
  fronts six agent CLIs plus an offline mock. Model and effort choices come
  from each provider's live catalog.
- **Approval-gated tool use.** Agents propose timeline edits through the
  editor's MCP gateway. Every mutation requires your approval, commits through
  one shared editing path, and is undoable.
- **Integrated terminal.** A real project-scoped PTY shell lives beside the
  assistant. Toggle it with Ctrl + \`.
- **Extensible skills.** Production skills such as smart cuts, auto captions,
  social formatting, and animated titles live as plain `SKILL.md` packages.

## Requirements

| Requirement | Notes |
| --- | --- |
| Rust 1.92 or newer | Edition 2024. Install with [rustup](https://rustup.rs). |
| Zed GPUI source | Must be present at `vendor/zed` (not tracked in git). |
| FFmpeg 8.1.2 development libraries | Must be present at `vendor/ffmpeg-8.1.2` (not tracked in git). |
| macOS | Xcode command line tools and `pkg-config` (`brew install pkg-config`). |
| Windows | Visual Studio 2022 C++ Build Tools and the Windows SDK. |

macOS is the primary, continuously tested platform. A Windows build
configuration (aliases plus the wrapper scripts under `scripts/`) is included;
other platforms are not verified.

## Setup

NodaVideo links FFmpeg and GPUI from vendored sources. Neither is committed, so
clone or place them before building:

```sh
# Zed's GPUI, at the revision this workspace was built against.
git clone https://github.com/zed-industries/zed.git vendor/zed
git -C vendor/zed checkout 48741fd17d0dec28be6ad664b0a06dc7202086be
```

Place the FFmpeg 8.1.2 development libraries so that pkg-config files exist at
`vendor/ffmpeg-8.1.2/lib/pkgconfig` (headers under `vendor/ffmpeg-8.1.2/include`).
The build links these libraries directly; it never builds FFmpeg itself.

The repository's Cargo aliases supply the required environment
(`FFMPEG_DIR`, `PKG_CONFIG_PATH`) through the files in `.cargo/`. Plain
`cargo build` and `cargo test` do not load them — use the aliases below, or
pass `--config` explicitly.

## Build and run

Run all commands from the repository root.

```sh
cargo editor-mac          # build and launch the editor (Windows: cargo editor-win)
cargo build-editor-mac    # build only (Windows: cargo build-editor-win)
cargo test-mac            # editor and library tests (Windows: cargo test-win)
```

For any other Cargo operation, select the platform configuration explicitly:

```sh
cargo check --config .cargo/macos.toml -p editor --all-targets
cargo clippy --config .cargo/macos.toml -p editor --all-targets
```

To produce a standalone release binary:

```sh
cargo build --release --config .cargo/macos.toml -p editor
# binary: target/release/editor
```

On first launch the editor opens a project picker. A project is a folder; the
editor stores its timelines, media library, and settings inside it. The last
opened project is restored on the next launch.

## Using the editor

- **Media.** Import files with `Cmd + I` (they are referenced in place, not
  copied), or drag them from the file explorer into the media pool or onto the
  timeline.
- **Timeline.** Drag clips between tracks, trim edges, split at the playhead,
  and edit text clips from the Inspector. Undo and redo cover every edit.
- **Export.** `Cmd + E` renders the active timeline to MP4 with H.264 video and
  AAC audio, using the timeline's own dimensions, frame rate, and sample rate.
- **AI Studio.** `Cmd + Shift + A` (or `Cmd + Alt + A`) opens the assistant
  workspace. Pick a provider, start a chat, and review any proposed timeline
  change in an approval card before it applies.
- **Terminal.** Ctrl + \` toggles a real shell in the project root, with tabs,
  selection, copy and paste, and scrollback.

Frequently used shortcuts:

| Shortcut | Action |
| --- | --- |
| `Space` | Play / pause |
| `Cmd + I` | Import media |
| `Cmd + E` | Export the active timeline |
| `Cmd + S` | Save |
| `Cmd + O` | Open another project folder |
| `Cmd + ,` | Settings |
| `Cmd + Shift + A` | AI Studio workspace |
| Ctrl + \` | Toggle the terminal |

The full list is available from **Help → Keyboard Shortcuts**.

## AI assistant

Providers are interchangeable and optional. NodaVideo launches whichever local
CLI you select; authentication stays inside that CLI, and NodaVideo never
reads, stores, or logs its tokens.

| Provider | Connection | Prerequisite |
| --- | --- | --- |
| Mock | Built in, fully offline | None (used for development) |
| Antigravity | ACP over stdio (`agy acp`) | Antigravity CLI installed and signed in |
| Gemini CLI | ACP over stdio (`gemini --acp`) | Gemini CLI installed and configured |
| Codex | App-server JSON-RPC (`codex`) | Codex CLI installed and signed in |
| Claude Code | Streaming CLI (`claude`) | Claude Code CLI installed and signed in |
| OpenCode | Local HTTP/SSE server (`opencode`) | OpenCode CLI installed |
| Qoder | Qoder CLI SDK daemon (native Rust) | Qoder CLI installed and logged in |

The Qoder provider additionally supports `NODAVIDEO_QODER_BIN` to select a
custom CLI executable. See [docs/QODER_PROVIDER.md](docs/QODER_PROVIDER.md) for
its protocol, approval model, and limitations.

How the assistant stays under your control:

- Every timeline mutation arrives as a typed editor tool call, is validated
  against the current project revision, and waits for an explicit decision.
- Approved edit calls commit through the same editing gateway as manual edits,
  so they are atomic and undoable. Rejected calls change nothing.
- Native CLI tool activity is displayed for transparency only; it is never
  executed a second time by the editor.
- Permission modes default to supervised. Full access is a deliberate opt-in,
  not a sandbox replacement.
- Attachments are sent to providers as file references, never uploaded or
  copied into the project by the editor.

Conversations are saved as transcripts in the project. A transcript restores
as read-only history unless the provider supplied a real session ID that can be
resumed.

## Headless CLI

The same engine powers a headless command line tool for scripts and CI:

```sh
cargo cli --help
cargo cli probe /path/to/media.mp4 --json
cargo cli validate timelines/episode.json --json
cargo cli export timelines/episode.json -o episode.mp4
cargo cli timeline remove-gaps timelines/episode.json --output compacted.json
cargo cli transcribe recording.mp4 --format srt -o subtitles.srt
```

See [apps/cli/README.md](apps/cli/README.md) for the timeline format contract,
export behavior, and every command.

## Transcription

Transcription uses the MiniMax speech-to-text API. In the editor, add your key
under **Settings** (`Cmd + ,`). In the CLI, set `MINIMAX_API_KEY` in the
environment. Audio is decoded locally and uploaded; output can be plain text,
JSON, SRT, or VTT, and SRT files can be imported into the editor.

## Repository layout

```
apps/
  editor/           Multi-track editor and AI Studio
  player/           Standalone debugging player
  cli/              Headless probe / validate / export / transcribe
libs/
  media-backend/    FFmpeg FFI decoding (VideoToolbox on macOS)
  timeline/         Timeline model, frame types, project persistence
  engine/           Playback, decoder caches, GPU conversion, export encoder
  transcribe/       Speech-to-text client
  player-ui/        Player components and cpal audio output
  waveform/         Waveform extraction
  platform-fs/      Platform file system helpers
crates/
  agent-runtime/    AgentProvider trait, runtime, sessions, events, mock
  agent-acp/        ACP adapter (Antigravity, Gemini CLI)
  agent-codex/      Codex app-server adapter
  agent-claude/     Claude Code adapter
  agent-opencode/   OpenCode server adapter
  agent-qoder/      Native Qoder SDK daemon adapter
  mcp-server/       Editor tool schemas, executor, transactional gateway
  skill-registry/   SKILL.md discovery and prompt composition
  hyperframes-bridge/  Motion graphics templates and composition rendering
skills/             Production AI skills
templates/          Motion graphics template bundles
sidecars/           HyperFrames renderer sidecar
design/             Static design system and pages (build output is generated)
vendor/             Zed GPUI and FFmpeg (obtained separately)
```

## Development

```sh
cargo test-mac                                   # all editor and library tests
cargo check --config .cargo/macos.toml --all-targets
cargo clippy --config .cargo/macos.toml --all-features
cargo fmt --check
```

Conventions for contributors and coding agents are documented in
[AGENTS.md](AGENTS.md) and `.agents/skills/`. In short: integer frames for all
timeline positions, heavy work off the UI thread, FFmpeg only through the
vendored libraries, and UI changes start in `design/`.

## Documentation

- [Qoder provider details](docs/QODER_PROVIDER.md)
- [CLI reference](apps/cli/README.md)
- [Agent and contributor guide](AGENTS.md)
- [Design spec](design/spec.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
