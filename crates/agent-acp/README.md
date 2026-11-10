# nodavideo-agent-acp

`nodavideo-agent-acp` implements the Agent Client Protocol (ACP) adapter over stdio JSON-RPC 2.0.

## Supported Providers

- **Antigravity CLI**: `agy acp`
- **Gemini CLI**: `gemini --acp`
- **Simulated / Offline Mode**: For automated CI testing and environments where the external CLI tools are not installed.

## Core Responsibilities

- **JSON-RPC 2.0 framing**: Structured request/response and streaming notification parsing over process stdio.
- **Process Management**: Spawns, monitors, and gracefully terminates external CLI child processes without UI thread blocking.
- **Protocol Translation**: Translates ACP streaming chunks and approval requests into unified `AgentEvent` items.
