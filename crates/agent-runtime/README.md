# nodavideo-agent-runtime

`nodavideo-agent-runtime` defines the canonical AI agent abstraction layer for NodaVideo.

## Core Responsibilities

- **`AgentProvider` trait**: Asynchronous interface for pluggable AI CLI and agent engines (`start_session`, `send_message`, `submit_approval`, `cancel`, `subscribe`).
- **`AgentConfig`**: Provider selection, requested model, optional system prompts, enabled skills, and process environment variables.
- **`AgentEvent` stream**: Strongly typed real-time stream of thoughts (`ThoughtChunk`), assistant responses (`MessageChunk`), tool call requests (`ToolCallRequested`), approval cards (`ApprovalRequested`), and lifecycle notifications (`SessionCompleted`, `SessionError`).
- **`MockAgentProvider`**: High-fidelity, deterministic in-memory provider for offline testing and preview workflows with zero external network or login requirements.
- **`AgentRuntime`**: Thread-safe registry and session router managing active sessions and provider availability.

## Contract Compliance (AGENTS.md)

- **Pluggable & Provider-Agnostic**: Outside of specific adapters, the editor core and UI only ever interact with `AgentProvider`.
- **Never Owns Editor State**: The agent runtime is an external client to the editor. It emits requests and events; all state mutations route through the command layer.
