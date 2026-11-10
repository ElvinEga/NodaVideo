# nodavideo-agent-codex

`nodavideo-agent-codex` implements the native OpenAI Codex `codex app-server` JSON-RPC 2.0 adapter.

## Core Capabilities

- **JSON-RPC 2.0 Stdio Integration**: Connects directly to `codex app-server` via subprocess stdio.
- **Bi-directional Streaming**: Streams token chunks, thoughts, and message completions into `AgentEvent` streams.
- **Approval Interception**: Maps Codex tool execution approval requests into typed editor approval prompts and forwards user approval decisions back over JSON-RPC.
- **Offline Simulation Mode**: Provides high-fidelity offline execution for automated CI test suites without requiring live API keys or CLI installations.
