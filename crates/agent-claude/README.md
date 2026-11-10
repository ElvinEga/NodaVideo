# nodavideo-agent-claude

`nodavideo-agent-claude` implements the Anthropic Claude Code CLI (`claude --output-format stream-json`) adapter.

## Core Capabilities

- **Streaming Line-Delimited JSON Protocol**: Parses streamed JSON lines emitted by the Claude Code CLI subprocess.
- **Event Mapping**: Translates thought blocks, text tokens, tool uses, and error messages into unified `AgentEvent` variants.
- **Interactive Approval Interception**: Converts Claude approval requests into native NodaVideo approval cards and delivers approval responses over stdio.
- **Offline Simulation Mode**: Enables deterministic automated verification without requiring external CLI credentials or network access.
