# Qoder CLI provider

## Setup

Install Qoder CLI using its official installation instructions and run `qodercli login`
outside NodaVideo. Select **Qoder CLI SDK** in the Assistant provider menu. The menu
shows installation readiness separately from the version and account-status probe.
It never displays or stores the login token. A failed status probe remains an error,
not a claim that the user is signed out.

The editor accepts `NODAVIDEO_QODER_BIN` at application initialization for a custom
executable. Without it, discovery checks `qodercli` and `qoder` on PATH, then the
versioned CLI installation directory. Library callers can pass a binary directly to
`QoderAgentProvider::new`. Restart the editor after installing the CLI or changing
its binary; readiness is cached for the current editor process.

## Transport and ownership

`crates/agent-qoder` implements the existing `AgentProvider` trait in Rust. It spawns
one `qodercli --sdk` daemon per active conversation and speaks NDJSON directly. There
is no ACP adapter, Node sidecar, or separately spawned editor MCP server.

The verified lifecycle is:

1. Write a private, temporary `{"type":"qodercli"}` auth payload. It contains no token;
   the daemon borrows the existing local CLI login read-only.
2. Initialize daemon protocol v2 with `multi_session_v2`, then authenticate.
3. Open a worker with an explicit project working directory and configured permission
   mode. Set the selected live model using the session's `set_model` request.
4. Initialize the session while servicing reverse SDK MCP requests. Await the correlated
   control response, not merely the `system/init` event. Frames are bounded at 64 MiB;
   real initialize responses can exceed 64 KiB.
5. Send user messages and translate streamed text, thinking, native tool activity,
   completion, errors, and host requests into shared runtime events.

Model IDs, names, and effort options come from `daemon.get_models`, never a compiled
catalog. Selected effort is sent with `--reasoning-effort`. The daemon's `auto` entry
is a client-side selector, not a worker model: unset/auto selection resolves to the
first concrete model returned by the daemon. Custom system instructions use session
`appendSystemPrompt`. The editor's enabled skills continue using the shared prompt
composer.

## Permissions and editor tools

The default is **Supervised**, mapped to CLI `default`, with
`--permission-prompt-tool stdio`. Native `can_use_tool` requests wait for the existing
NodaVideo approval card. Allow sends the original input and a once-only decision;
Deny, cancellation, missing subscribers, and a 120-second approval timeout fail closed.
Ordinary headless execution is not used as a substitute for interactive permissions.

| Mode | CLI mode | Editor MCP behavior |
| --- | --- | --- |
| Read only | `plan` | Denies mutations and approval-required tools |
| Supervised | `default` | Asks before mutations and approval-required tools |
| Auto-accept edits | `acceptEdits` | CLI may accept native edits; editor mutations still ask |
| Full access | `bypassPermissions` | Explicit opt-in; editor schema/revision validation still applies |

The adapter never passes `--yolo`. CLI modes are policy settings, not an OS sandbox.
Approved native shell/file tools execute in the CLI and can have external effects;
NodaVideo cannot undo those effects. Review shell commands before approving them.

The editor supplies `McpToolDefinition::all_tools()` to the provider as `AgentTool`
definitions. A virtual SDK server named `nodavideo` exposes that exact catalog,
including registered HyperFrames tools. `sdkMcpToolOverrides.alwaysLoad` makes the
catalog directly discoverable; strict MCP configuration excludes inherited servers
for these sessions.

A native permission request to enter this registered bridge is not permission to
mutate the editor. The host gates the actual `tools/call`, then emits a distinct
`EditorToolCallRequested`. Only that event reaches the shared `McpExecutor` and
`ApplyAgentEdit` path. Native tool snapshots are activity only, preventing duplicate
execution. Results and errors returned to Qoder come from the editor, not optimistic
acknowledgments.

Timeline work uses a background snapshot. Before applying it, the editor checks the
session generation, project, open timeline, revision, and revocable authorization.
Successful changes preserve undo history and are committed before the MCP reply.
Cancellation and the final commit share an authorization lock: if cancellation wins,
late timeline/library results cannot commit; if commit wins, the host returns its real
outcome instead of claiming cancellation prevented it. Filesystem operations already
running may have completed before cancellation; they are not rolled back by a later
timeline-snapshot rejection. Imports return `import_queued`, not a claim that asynchronous
media probing has finished.

## Sessions and cleanup

NodaVideo stores its transcript together with the native Qoder session ID, model,
and effort. Selecting a saved Qoder conversation with a native ID restores the local
view and passes `--resume` on the next send. Stop preserves that ID, closes the old
worker, and orders a later resume after startup and cancellation finish. Model, effort,
and permission changes restart Qoder with the new settings while retaining its native
conversation. New Chat, provider changes, and skill changes save the original transcript
and start a separate conversation; they never display old context as if a new worker knew it.

Older transcripts without a native ID, and transcripts from providers without this
integration, remain read-only. A deleted or incompatible native conversation produces
a real startup error; the adapter does not silently substitute an empty conversation.
The provider exposes the daemon's native session listing API, but the editor sidebar
lists NodaVideo's project transcripts, not every conversation created in another client.

Stop cancels pending approvals/results, interrupts the session, closes it, and kills
its owned daemon if needed. Dropping the provider kills its children and removes the
private auth directory. The adapter drains stderr without logging raw diagnostic
contents. It never signs the user out or terminates unrelated editor/CLI processes.

## Current limits

- The shared composer/provider API is text-only. Image/file attachments are not wired;
  no attachment control is presented as working. Existing media tools reference files.
- Only once-only Allow/Deny is exposed. Session/persistent allow-rule editing and rich
  SDK elicitation forms are not implemented; elicitation requests are cancelled.
- Version/account status is a probe, not continuous account monitoring. Login and
  installation remain external CLI operations.
- Qoder and Mock use this editor gateway. Other registered native adapters retain their
  existing tool-activity behavior; this change does not retrofit their MCP transports.
- The adapter was live-tested on macOS with CLI **1.1.89**, SDK wire contract **1.0.59**.
  Windows/Linux execution and other CLI versions have not been live-verified.

## Validation

Offline validation requires neither login nor network:

```sh
cargo test -p nodavideo-agent-qoder -p nodavideo-agent-runtime
cargo clippy -p nodavideo-agent-qoder -p nodavideo-agent-runtime --all-targets -- -D warnings
cargo check --config .cargo/macos.toml -p editor --all-targets
cargo test-mac --workspace
```

Offline cases cover daemon envelopes, streaming, missing CLI detection, model/session
parsing, permission allow/deny/timeout/cancel, MCP listing and notification replies,
read-only enforcement, actual result correlation, rejected late results, commit/cancel
ordering, cancellation-marker cleanup and overflow, non-replayable serialized
authorizations, and bounded large/malformed frames. Live verification exercised MCP discovery, a host-approved
synthetic call, a denied call that never executed, and native resume across daemon
processes with a remembered codeword. Synthetic probe tools are not production tools.

Native GPUI checks remain a manual acceptance step: select Qoder, inspect status and
models, send a prompt, approve/deny an editor action, undo/redo a timeline edit, edit
concurrently to check stale-revision rejection, Stop, and resume from history. No
editor instance is launched or controlled by the offline suite.
