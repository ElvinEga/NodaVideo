# NodaVideo

*Your timeline. Your creativity. Your AI.*

An AI-powered desktop video editor built with native Rust + GPUI. Cross-platform
(macOS, Windows, Linux), featuring a professional multi-track timeline, real-time
GPU-accelerated playback, and pluggable AI agents.

## Architecture

NodaVideo combines two codebases:

- **Editor core** — from [OpenCut](https://github.com/opencut/opencut): working
  video engine with `ffmpeg-next` FFI decoding, VideoToolbox on macOS, Metal GPU
  compositing, `cpal` audio output, and a shared decoder path for preview + export.
- **AI layer** — NodaVideo's original agent system: pluggable `AgentProvider`
  trait, ACP/Codex/Claude/OpenCode/Qoder adapters, MCP tool executor, skill registry, and
  HyperFrames motion graphics bridge.

## Product Direction

1. **Professional video editor first**, AI assistant second. Hiding the AI
   workspace leaves a fully featured desktop NLE.
2. **Complete user control.** Agents propose actions; the user reviews, approves,
   or undoes them.
3. **Provider independence.** Antigravity, Codex, Gemini CLI, Claude Code,
   OpenCode, Qoder CLI, and ACP-compatible agents share one provider interface.
4. **Extensible AI skills** system for video-production capabilities.

## Requirements

- Latest stable Rust (edition 2024, MSRV 1.92.0)
- Vendored FFmpeg 8.1.2 development libraries in `vendor/ffmpeg-8.1.2/`
- Vendored Zed GPUI at `vendor/zed/` (git clone or submodule)
- On macOS: Xcode command line tools and `pkg-config` (`brew install pkg-config`)
- On Windows: Visual Studio 2022 C++ Build Tools and Windows SDK

## Build and run

Run commands from the repository root. Each command selects a platform explicitly:

```sh
cargo editor-mac                 # Windows: cargo editor-win
cargo player-mac <file>          # Windows: cargo player-win <file>
cargo build-editor-mac
cargo test-mac                   # Windows: cargo test-win
```

For other Cargo operations, pass the platform configuration:

```sh
cargo check --config .cargo/macos.toml -p editor   # Windows: .cargo/windows.toml
cargo clippy --config .cargo/macos.toml --all-features
```

## Qoder CLI provider

Install Qoder CLI and authenticate with `qodercli login`, then select **Qoder CLI SDK**
in the Assistant provider menu. The provider uses native Rust and Qoder's SDK daemon
protocol; it does not use ACP or a Node sidecar. `NODAVIDEO_QODER_BIN` selects a custom
CLI executable. The menu reports the installed version and sign-in status without
reading or storing authentication tokens.

Models come from the live CLI catalog. Qoder receives the editor's canonical MCP tool
schemas, with host-owned approvals and real gateway results. Saved NodaVideo Qoder
conversations resume using their native session IDs; older transcripts without IDs
remain read-only. Supervised mode is the default. Full access is an explicit opt-in,
not a sandbox. See [Qoder provider details and limitations](docs/QODER_PROVIDER.md).

## Workspace structure

```
libs/           — Editor core (from OpenCut)
  media-backend/  FFmpeg FFI video/audio decoding, VideoToolbox
  timeline/       Timeline model (Clip, Track, FrameRate, serialization)
  engine/         Playback engine (TimelineBackend, TimelineDecoder, GPU/CPU, export)
  transcribe/     Audio transcription (Whisper API)
  player-ui/      Player UI components, cpal audio output
  waveform/       Audio waveform extraction
  platform-fs/    Platform-specific file system operations

apps/           — Applications
  editor/         Multi-track NLE editor
  player/         Standalone debugging player
  cli/            Headless CLI for probing, validating, exporting

crates/         — AI layer (NodaVideo-specific)
  agent-runtime/  AgentProvider trait, AgentRuntime, MockAgentProvider
  agent-acp/      ACP JSON-RPC adapter (Antigravity, Gemini CLI)
  agent-codex/    Codex app-server JSON-RPC adapter
  agent-claude/   Claude Code CLI streaming adapter
  agent-opencode/ OpenCode HTTP/SSE adapter
  agent-qoder/    Native Qoder SDK daemon, host approvals and virtual MCP
  mcp-server/     MCP editor tool schemas and executor
  skill-registry/ SKILL.md parser, discovery, prompt composer
  hyperframes-bridge/ Motion graphics templates, HTML/SVG composition

skills/         — Production AI skills (smart-cuts, auto-captions, etc.)
templates/      — Motion graphics template bundles
sidecars/       — HyperFrames sidecar renderer
vendor/         — Vendored Zed GPUI + FFmpeg libraries
```

## Documentation

- [Migration plan](docs/MIGRATION_PLAN.md): Detailed plan for the OpenCut core replacement
- [Design spec](design/spec.md): User stories and acceptance criteria (from OpenCut)

## License

Apache-2.0. See [LICENSE](LICENSE) for details.
