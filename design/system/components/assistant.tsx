import type { Child, Markup } from "@design/jsx/jsx-runtime";
import { NodaButton, NodaGlyph } from "./noda-shell.tsx";
import { Checkbox, Switch } from "./panel.tsx";
import {
  ALPHA_STYLE,
  COMPOSITION_FIXTURES,
  CompositionThumb,
  MOTION,
  MotionMark,
} from "./motion-media.tsx";
import type { Composition } from "./motion-media.tsx";

// The assistant is a full-length window of its own: a sessions sidebar, a wide thread,
// a bottom composer whose model / attach / autonomy controls sit BELOW the field, an
// optional terminal dock and a status bar. A cramped dock cannot hold tool output, so
// the dock is gone. Identity stays the editor's indigo; motion results keep their amber.
// Everything here is a static mock-up: states are drawn side by side, never scripted.

export type ProviderId = "claude" | "codex" | "gemini" | "antigravity" | "mock";
export type ProviderStatus = "ready" | "signin" | "offline";

export type Provider = {
  id: ProviderId;
  name: string;
  transport: string;
  model: string;
  status: ProviderStatus;
  hue: string;
};

/// The real providers the editor registers, with their transport and an illustrative model.
export const PROVIDERS: Provider[] = [
  { id: "claude", name: "Claude Code", transport: "CLI", model: "Sonnet 4.5", status: "ready", hue: "#d97757" },
  { id: "codex", name: "Codex", transport: "App-Server", model: "GPT-5.4", status: "ready", hue: "#10a37f" },
  { id: "gemini", name: "Gemini CLI", transport: "ACP", model: "2.5 Pro", status: "ready", hue: "#4285f4" },
  { id: "antigravity", name: "Antigravity", transport: "ACP", model: "Gemini 3 Pro", status: "signin", hue: "#a5b4fc" },
  { id: "mock", name: "Mock Assistant", transport: "Offline", model: "Demo loop", status: "offline", hue: "#8a90a2" },
];

export type Skill = { id: string; name: string; desc: string; icon: Parameters<typeof NodaGlyph>[0]["name"] };

/// The production skills in skills/, each a provider-agnostic capability.
export const SKILLS: Skill[] = [
  { id: "smart-cuts", name: "Smart cuts", desc: "Remove silence, ums and dead air", icon: "scissors" },
  { id: "auto-captions", name: "Auto captions", desc: "Transcribe speech to timed subtitles", icon: "type" },
  { id: "social-video", name: "Social video", desc: "Reframe to 9:16 with hooks and captions", icon: "globe" },
  { id: "animated-titles", name: "Animated titles", desc: "HyperFrames lower thirds and titles", icon: "sparkle" },
  { id: "audio-cleanup", name: "Audio cleanup", desc: "Reduce noise, normalize loudness", icon: "mic" },
  { id: "color-grading", name: "Color grading", desc: "Match shots and apply a look", icon: "eye" },
];

export type ToolKind = "read" | "write";
export type Tool = { name: string; kind: ToolKind; approval?: boolean };
export type ToolGroup = { name: string; icon: Parameters<typeof NodaGlyph>[0]["name"]; tools: Tool[] };

/// The MCP catalog the gateway exposes, grouped as the schema orders it. `approval` marks
/// the tools the editor gates behind an explicit user decision.
export const TOOL_GROUPS: ToolGroup[] = [
  {
    name: "Project & media",
    icon: "folder",
    tools: [
      { name: "get_project", kind: "read" },
      { name: "get_timeline", kind: "read" },
      { name: "get_selected_clips", kind: "read" },
      { name: "get_media_items", kind: "read" },
      { name: "search_media", kind: "read" },
      { name: "analyze_media", kind: "read" },
      { name: "import_media", kind: "write" },
      { name: "remove_media", kind: "write" },
      { name: "rename_media", kind: "write" },
    ],
  },
  {
    name: "Timeline",
    icon: "layers",
    tools: [
      { name: "add_clip", kind: "write" },
      { name: "move_clip", kind: "write" },
      { name: "trim_clip", kind: "write" },
      { name: "split_clip", kind: "write", approval: true },
      { name: "delete_clip", kind: "write", approval: true },
      { name: "set_clip_transform", kind: "write" },
      { name: "set_clip_audio", kind: "write" },
    ],
  },
  {
    name: "Captions & text",
    icon: "type",
    tools: [
      { name: "add_text_overlay", kind: "write" },
      { name: "generate_captions", kind: "write" },
    ],
  },
  {
    name: "Changesets",
    icon: "shield",
    tools: [
      { name: "preview_changeset", kind: "read" },
      { name: "propose_changeset", kind: "write", approval: true },
    ],
  },
  {
    name: "HyperFrames",
    icon: "sparkle",
    tools: [
      { name: "list_composition_templates", kind: "read" },
      { name: "list_compositions", kind: "read" },
      { name: "create_composition", kind: "write" },
      { name: "inspect_composition", kind: "read" },
      { name: "edit_composition", kind: "write" },
      { name: "render_composition", kind: "write" },
      { name: "validate_composition", kind: "read" },
      { name: "add_composition_to_timeline", kind: "write", approval: true },
    ],
  },
  {
    name: "Export",
    icon: "download",
    tools: [{ name: "export_video", kind: "read", approval: true }],
  },
];

export const TOOL_COUNT = TOOL_GROUPS.reduce((n, g) => n + g.tools.length, 0);

export type AutonomyMode = "assist" | "collaborate" | "autonomous";
export type ToolStatus = "running" | "done" | "failed" | "approval" | "queued";
export type WindowState = "empty" | "chat" | "approval" | "streaming" | "error";

const speakerCard = COMPOSITION_FIXTURES[0];

/// A small brand dot for a provider. Muted chrome; the hue only identifies the agent.
export function ProviderDot({ provider, size = 7 }: { provider: Provider; size?: number }): Markup {
  return (
    <span
      class="inline-block shrink-0 rounded-full"
      style={`width:${size}px;height:${size}px;background:${provider.hue}`}
    />
  );
}

function StatusPill({ status }: { status: ToolStatus }): Markup {
  const map: Record<ToolStatus, { label: string; text: string; dot: string }> = {
    running: { label: "Running", text: "text-warning", dot: "bg-warning" },
    done: { label: "Done", text: "text-success", dot: "bg-success" },
    failed: { label: "Failed", text: "text-error-text", dot: "bg-error" },
    approval: { label: "Approval", text: "text-accent-soft", dot: "bg-accent" },
    queued: { label: "Queued", text: "text-muted", dot: "bg-dim" },
  };
  const s = map[status];
  return (
    <span class={`inline-flex shrink-0 items-center gap-1 rounded-[3px] bg-surface px-1.5 py-0.5 text-[9px] font-medium ${s.text}`}>
      <span class={`size-1.5 rounded-full ${s.dot}`} />
      {s.label}
    </span>
  );
}

function KindBadge({ kind }: { kind: ToolKind }): Markup {
  return kind === "read"
    ? <span class="rounded-[3px] bg-hover px-1.5 py-0.5 text-[9px] text-muted">Read</span>
    : <span class="rounded-[3px] bg-accent-background px-1.5 py-0.5 text-[9px] text-accent-soft">Write</span>;
}

function KeyCap({ children }: { children: Child }): Markup {
  return (
    <kbd class="rounded-[3px] border border-border bg-surface px-1 font-mono text-[9px] text-subtle">
      {children}
    </kbd>
  );
}

/// A file attached to a message: kind icon, name, size and a remove control.
export function AttachmentChip(
  { name, kind, size }: { name: string; kind: "video" | "audio" | "image" | "file"; size: string },
): Markup {
  const glyph = kind === "video" ? "video" : kind === "audio" ? "mic" : kind === "image" ? "image" : "file";
  return (
    <span class="inline-flex h-6 max-w-[220px] items-center gap-1.5 rounded-[4px] border border-border bg-hover px-1.5 text-[10px]">
      <span class="shrink-0 text-accent-soft"><NodaGlyph name={glyph} size={11} /></span>
      <span class="truncate text-light">{name}</span>
      <span class="shrink-0 font-mono text-[9px] text-dim">{size}</span>
      <span class="shrink-0 text-dim"><NodaGlyph name="close" size={10} /></span>
    </span>
  );
}

/// The user's turn: a right-aligned indigo bubble. Attachments sit above the text.
export function UserMessage(
  { text, attachments, truncated = false }: {
    text: string;
    attachments?: { name: string; kind: "video" | "audio" | "image" | "file"; size: string }[];
    truncated?: boolean;
  },
): Markup {
  return (
    <div class="flex justify-end">
      <div class="max-w-[78%] rounded-[10px] rounded-br-[3px] bg-accent-background px-3 py-2.5">
        {attachments && (
          <div class="mb-2 flex flex-wrap justify-end gap-1">
            {attachments.map((a) => <AttachmentChip name={a.name} kind={a.kind} size={a.size} />)}
          </div>
        )}
        <p class="text-[12px] leading-5 text-text">{text}</p>
        {truncated && <p class="mt-1 text-[11px] text-accent-soft">Show more</p>}
      </div>
    </div>
  );
}

/// The assistant's turn: provider, runtime and elapsed time on one quiet header line,
/// then prose and cards, then a row of turn actions. No avatar tile — the thread is wide.
export function AssistantMessage(
  { provider = "claude", runtime = "NodaVideo", duration = "56s", working = false, time = "09:41", children }: {
    provider?: ProviderId;
    runtime?: string;
    duration?: string;
    working?: boolean;
    time?: string;
    children: Child;
  },
): Markup {
  const p = PROVIDERS.find((x) => x.id === provider) ?? PROVIDERS[0];
  return (
    <div class="min-w-0">
      <div class="mb-1.5 flex items-center gap-1.5">
        <ProviderDot provider={p} size={7} />
        <span class="text-[11px] font-medium text-soft">{p.name}</span>
        <span class="text-[11px] text-dim">·</span>
        <span class="text-[11px] text-muted">
          {runtime} {working ? "working for" : "worked for"} {duration}
        </span>
      </div>
      <div class="space-y-2 text-[12px] leading-5 text-light">{children}</div>
      <div class="mt-2 flex items-center gap-2 text-dim">
        <NodaGlyph name="copy" size={12} />
        <NodaGlyph name="refresh" size={12} />
        <NodaGlyph name="external" size={12} />
        <NodaGlyph name="more" size={12} />
        <span class="font-mono text-[9px]">{time}</span>
      </div>
    </div>
  );
}

/// A collapsed run of tool work, the way a long agent turn stays legible: one quiet line
/// ("Ran 2 commands · Read ref1.png") that expands to the individual calls.
export function ToolRunRow(
  { label, icon = "terminal", expanded = false, children }: {
    label: string;
    icon?: Parameters<typeof NodaGlyph>[0]["name"];
    expanded?: boolean;
    children?: Child;
  },
): Markup {
  return (
    <div class="overflow-hidden rounded-[6px] border border-border/70 bg-surface">
      <div class="flex h-8 items-center gap-2 px-2.5 text-[11px] text-muted">
        <span class="grid size-5 shrink-0 place-items-center rounded-[4px] bg-hover text-subtle">
          <NodaGlyph name={icon} size={11} />
        </span>
        <span class="truncate">{label}</span>
        <span class="ml-auto text-dim"><NodaGlyph name={expanded ? "chevron" : "collapse"} size={11} /></span>
      </div>
      {expanded && (
        <div class="space-y-1 border-t border-border/60 px-3 py-2 text-[11px] leading-5 text-muted">
          {children}
        </div>
      )}
    </div>
  );
}

/// Reasoning, collapsible. Collapsed it is one quiet line; expanded it shows the thought.
export function ThoughtBlock(
  { text, expanded = false, duration = "2.1s" }: { text: string; expanded?: boolean; duration?: string },
): Markup {
  return (
    <div class="rounded-[6px] border border-border/70 bg-surface">
      <div class="flex h-7 items-center gap-1.5 px-2 text-[10px] text-muted">
        <NodaGlyph name={expanded ? "chevron" : "collapse"} size={11} />
        <span class="font-medium">Thinking</span>
        <span class="font-mono text-[9px] text-dim">{duration}</span>
        {!expanded && <span class="ml-auto truncate text-[9px] text-dim">planning the trim…</span>}
      </div>
      {expanded && (
        <p class="border-t border-border/60 px-2.5 py-2 text-[11px] leading-4 text-muted italic">
          {text}
        </p>
      )}
    </div>
  );
}

/// A single MCP tool call: the tool name, its status, the arguments and the result.
/// This is the assistant reaching into the native editor — the signature of the window.
export function ToolCallCard(
  { tool, status, args, result, icon = "plug", children }: {
    tool: string;
    status: ToolStatus;
    args?: string;
    result?: string;
    icon?: Parameters<typeof NodaGlyph>[0]["name"];
    children?: Child;
  },
): Markup {
  return (
    <div class="overflow-hidden rounded-[6px] border border-border bg-surface">
      <div class="flex h-8 items-center gap-2 border-b border-border/60 px-2.5">
        <span class="text-muted"><NodaGlyph name={icon} size={12} /></span>
        <span class="truncate font-mono text-[11px] text-light">{tool}</span>
        <span class="ml-auto"><StatusPill status={status} /></span>
      </div>
      <div class="space-y-1.5 px-2.5 py-2">
        {args && <p class="font-mono text-[10px] leading-4 text-subtle">{args}</p>}
        {result && <p class="text-[11px] leading-4 text-muted">{result}</p>}
        {children}
      </div>
    </div>
  );
}

/// A before → after diff line, mono timecodes, used inside approval and tool cards.
export function DiffLine({ before, after }: { before: string; after: string }): Markup {
  return (
    <div class="flex items-center gap-2 rounded-[4px] bg-hover px-2 py-1.5 font-mono text-[10px]">
      <span class="text-muted line-through decoration-error/60">{before}</span>
      <span class="text-dim"><NodaGlyph name="arrowRight" size={11} /></span>
      <span class="text-accent-soft">{after}</span>
    </div>
  );
}

/// The approval gate: a concrete change the user must accept before the timeline moves.
/// `diff` is the change body; `tool` names the MCP call; `always` offers a standing rule.
export function ApprovalCard(
  { title, detail, tool, diff, always = true, decided }: {
    title: string;
    detail: string;
    tool: string;
    diff: Child;
    always?: boolean;
    decided?: "approved" | "rejected";
  },
): Markup {
  return (
    <div class="rounded-[6px] border border-border border-l-2 border-l-accent bg-surface p-2.5">
      <div class="flex items-center gap-2">
        <span class="text-accent-soft"><NodaGlyph name="shield" size={13} /></span>
        <p class="text-[12px] font-medium text-text">{title}</p>
        <span class="ml-auto font-mono text-[9px] text-dim">{tool}</span>
      </div>
      <p class="mt-1 text-[11px] leading-4 text-muted">{detail}</p>
      <div class="mt-2">{diff}</div>
      {decided
        ? (
          <div class={`mt-2.5 flex items-center gap-1.5 text-[11px] font-medium ${decided === "approved" ? "text-success" : "text-error-text"}`}>
            <NodaGlyph name={decided === "approved" ? "checkCircle" : "xCircle"} size={12} />
            {decided === "approved" ? "Approved · applied to the timeline" : "Rejected · no change made"}
          </div>
        )
        : (
          <>
            <div class="mt-2.5 flex gap-2">
              <NodaButton label="Approve" primary />
              <NodaButton label="Reject" />
            </div>
            {always && (
              <label class="mt-2 flex items-center gap-1.5 text-[10px] text-muted">
                <Checkbox checked={false} />
                Always allow <span class="font-mono">{tool}</span>
              </label>
            )}
          </>
        )}
    </div>
  );
}

/// A HyperFrames result in the thread: the rendered composition over alpha, with the
/// amber motion identity, Add to Timeline and Open in Editor. The AI made a real asset.
export function CompositionResultCard({ comp = speakerCard }: { comp?: Composition }): Markup {
  return (
    <div class="overflow-hidden rounded-[6px] border bg-surface" style={`border-color:${MOTION.border}`}>
      <div class="flex h-8 items-center gap-2 px-2.5" style={`background:${MOTION.well}`}>
        <span style={`color:${MOTION.badge}`}><MotionMark size={12} /></span>
        <span class="truncate text-[11px] font-medium text-text">{comp.name}</span>
        <span class="ml-auto shrink-0 font-mono text-[9px] text-muted">{comp.duration}</span>
      </div>
      <div class="relative aspect-video w-full" style={ALPHA_STYLE}>
        <CompositionThumb comp={comp} />
        <span class="absolute top-1 left-1 rounded-[2px] bg-monitor/80 px-1 py-0.5 font-mono text-[8px] text-subtle">
          {comp.template}
        </span>
      </div>
      <div class="flex items-center gap-2 p-2.5">
        <NodaButton label="Add to Timeline" primary />
        <NodaButton label="Open in Editor" />
      </div>
    </div>
  );
}

/// A failed turn: what went wrong and how to recover, in the interface's voice.
export function ErrorCard(
  { title, detail }: { title: string; detail: string },
): Markup {
  return (
    <div class="rounded-[6px] border border-error/40 bg-error-background p-2.5">
      <div class="flex items-center gap-2">
        <span class="text-error-text"><NodaGlyph name="warning" size={13} /></span>
        <p class="text-[12px] font-medium text-error-text">{title}</p>
      </div>
      <p class="mt-1 text-[11px] leading-4 text-subtle">{detail}</p>
      <div class="mt-2 flex gap-2">
        <NodaButton label="Retry" />
        <NodaButton label="Dismiss" />
      </div>
    </div>
  );
}

/// The provider chooser button that lives in the composer toolbar, BELOW the field:
/// brand dot, provider, model, reasoning effort and a chevron. One tap opens ModelMenu.
export function ModelButton(
  { provider = "claude", effort = "high" }: { provider?: ProviderId; effort?: string },
): Markup {
  const p = PROVIDERS.find((x) => x.id === provider) ?? PROVIDERS[0];
  return (
    <button
      type="button"
      class="inline-flex h-7 min-w-0 shrink items-center gap-1.5 rounded-[5px] bg-hover px-2 text-[11px] text-light hover:bg-rail/40"
    >
      <ProviderDot provider={p} size={7} />
      <span class="truncate font-medium">{p.name}</span>
      <span class="truncate text-muted">{p.model}</span>
      <span class="shrink-0 rounded-[3px] bg-surface px-1 font-mono text-[9px] text-subtle">{effort}</span>
      <span class="shrink-0 text-muted"><NodaGlyph name="chevron" size={11} /></span>
    </button>
  );
}

function ProviderStatusNote({ status }: { status: ProviderStatus }): Markup {
  if (status === "ready") {
    return <span class="flex items-center gap-1 text-[9px] text-success"><span class="size-1.5 rounded-full bg-success" />Ready</span>;
  }
  if (status === "signin") {
    return <span class="flex items-center gap-1 text-[9px] text-warning"><span class="size-1.5 rounded-full bg-warning" />Sign in</span>;
  }
  return <span class="flex items-center gap-1 text-[9px] text-muted"><span class="size-1.5 rounded-full bg-dim" />Offline</span>;
}

/// The provider dropdown: every registered agent, its transport, model and readiness.
/// One AgentProvider abstraction; the same NodaVideo and HyperFrames tools for all.
export function ModelMenu({ selected = "claude" }: { selected?: ProviderId }): Markup {
  return (
    <div class="w-[300px] rounded-lg border border-border bg-menu p-[3px] font-sans shadow-menu">
      <p class="px-3 pt-2 pb-1 text-[10px] font-semibold tracking-[0.1em] text-muted uppercase">Provider</p>
      {PROVIDERS.map((p) => (
        <div
          role="menuitemradio"
          aria-checked={String(p.id === selected)}
          class={`flex items-center gap-2.5 rounded-md px-3 py-2 ${p.id === selected ? "bg-menu-hover" : ""}`}
        >
          <ProviderDot provider={p} size={8} />
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-1.5">
              <span class="truncate text-[12px] text-text">{p.name}</span>
              <span class="rounded-[3px] bg-surface px-1 font-mono text-[8px] text-muted">{p.transport}</span>
            </div>
            <p class="mt-0.5 truncate text-[10px] text-muted">{p.model}</p>
          </div>
          <ProviderStatusNote status={p.status} />
          {p.id === selected && <span class="text-accent-soft"><NodaGlyph name="check" size={13} /></span>}
        </div>
      ))}
      <div class="mt-1 border-t border-border pt-1">
        <div role="menuitem" class="flex items-center gap-2 rounded-md px-3 py-2 text-[11px] text-muted">
          <NodaGlyph name="sliders" size={13} />
          Manage providers…
        </div>
      </div>
    </div>
  );
}

const AUTONOMY: { id: AutonomyMode; name: string; icon: Parameters<typeof NodaGlyph>[0]["name"]; note: string }[] = [
  { id: "assist", name: "Assist", icon: "hand", note: "Proposes changes; you apply each one." },
  { id: "collaborate", name: "Collaborate", icon: "shield", note: "Applies safe edits; asks before cuts and deletes." },
  { id: "autonomous", name: "Autonomous", icon: "zap", note: "Runs a whole plan; still asks before destructive edits." },
];

/// The autonomy button in the composer toolbar, BELOW the field. Autonomous tints amber
/// to signal that fewer gates remain — destructive edits still ask in every mode.
export function AutonomyButton({ mode = "collaborate" }: { mode?: AutonomyMode }): Markup {
  const m = AUTONOMY.find((x) => x.id === mode) ?? AUTONOMY[1];
  const tint = mode === "autonomous" ? "text-warning" : "text-light";
  return (
    <button
      type="button"
      class={`inline-flex h-7 shrink-0 items-center gap-1.5 rounded-[5px] bg-hover px-2 text-[11px] whitespace-nowrap hover:bg-rail/40 ${tint}`}
    >
      <NodaGlyph name={m.icon} size={12} />
      {m.name}
      <span class="text-muted"><NodaGlyph name="chevron" size={11} /></span>
    </button>
  );
}

/// The autonomy dropdown: the three modes with what each one gates.
export function AutonomyMenu({ selected = "collaborate" }: { selected?: AutonomyMode }): Markup {
  return (
    <div class="w-[300px] rounded-lg border border-border bg-menu p-[3px] font-sans shadow-menu">
      <p class="px-3 pt-2 pb-1 text-[10px] font-semibold tracking-[0.1em] text-muted uppercase">Autonomy</p>
      {AUTONOMY.map((m) => (
        <div
          role="menuitemradio"
          aria-checked={String(m.id === selected)}
          class={`flex items-start gap-2.5 rounded-md px-3 py-2 ${m.id === selected ? "bg-menu-hover" : ""}`}
        >
          <span class={`mt-0.5 ${m.id === "autonomous" ? "text-warning" : m.id === selected ? "text-accent-soft" : "text-muted"}`}>
            <NodaGlyph name={m.icon} size={14} />
          </span>
          <div class="min-w-0 flex-1">
            <p class="text-[12px] text-text">{m.name}</p>
            <p class="mt-0.5 text-[10px] leading-4 text-muted">{m.note}</p>
          </div>
          {m.id === selected && <span class="mt-0.5 text-accent-soft"><NodaGlyph name="check" size={13} /></span>}
        </div>
      ))}
      <p class="border-t border-border px-3 py-2 text-[10px] leading-4 text-muted">
        Destructive edits always ask, in every mode.
      </p>
    </div>
  );
}

/// The skill picker: production capabilities the assistant can apply to this project.
export function SkillMenu({ selected = "smart-cuts" }: { selected?: string }): Markup {
  return (
    <div class="w-[300px] rounded-lg border border-border bg-menu p-[3px] font-sans shadow-menu">
      <p class="px-3 pt-2 pb-1 text-[10px] font-semibold tracking-[0.1em] text-muted uppercase">Skills</p>
      {SKILLS.map((s) => (
        <div
          role="menuitemcheckbox"
          aria-checked={String(s.id === selected)}
          class={`flex items-start gap-2.5 rounded-md px-3 py-2 ${s.id === selected ? "bg-menu-hover" : ""}`}
        >
          <span class={`mt-0.5 ${s.id === selected ? "text-accent-soft" : "text-muted"}`}>
            <NodaGlyph name={s.icon} size={14} />
          </span>
          <div class="min-w-0 flex-1">
            <p class="text-[12px] text-text">{s.name}</p>
            <p class="mt-0.5 text-[10px] leading-4 text-muted">{s.desc}</p>
          </div>
          {s.id === selected && <span class="mt-0.5 text-accent-soft"><NodaGlyph name="check" size={13} /></span>}
        </div>
      ))}
      <div class="mt-1 border-t border-border pt-1">
        <div role="menuitem" class="flex items-center gap-2 rounded-md px-3 py-2 text-[11px] text-muted">
          <NodaGlyph name="folder" size={13} />
          Browse skill library…
        </div>
      </div>
    </div>
  );
}

/// A skill card for the gallery: icon, name, what it does, and the tools it drives.
export function SkillCard({ skill, active = false }: { skill: Skill; active?: boolean }): Markup {
  return (
    <div class={`w-[220px] rounded-[6px] border p-3 ${active ? "border-accent bg-accent-background" : "border-border bg-panel"}`}>
      <div class="flex items-center gap-2">
        <span class={`grid size-7 place-items-center rounded-[6px] ${active ? "bg-accent text-on-accent" : "bg-surface text-accent-soft"}`}>
          <NodaGlyph name={skill.icon} size={15} />
        </span>
        <p class="text-[12px] font-medium text-text">{skill.name}</p>
        {active && <span class="ml-auto text-accent-soft"><NodaGlyph name="check" size={13} /></span>}
      </div>
      <p class="mt-2 text-[10px] leading-4 text-muted">{skill.desc}</p>
      <p class="mt-2 font-mono text-[9px] text-dim">skills/{skill.id}/SKILL.md</p>
    </div>
  );
}

/// The composer button that opens the skill picker; shows the active skill when set.
export function SkillButton({ skill }: { skill?: Skill }): Markup {
  return (
    <button
      type="button"
      class={`inline-flex h-7 shrink-0 items-center gap-1.5 rounded-[5px] px-2 text-[11px] whitespace-nowrap ${
        skill ? "bg-accent-background text-accent-soft" : "bg-hover text-light hover:bg-rail/40"
      }`}
    >
      <NodaGlyph name="sparkle" size={13} />
      {skill ? skill.name : "Skills"}
      <NodaGlyph name="chevron" size={11} />
    </button>
  );
}

/// The composer: attachment chips and any warning ABOVE the field, the growing message
/// field, then the toolbar BELOW it — attach, skill, model, autonomy, and send/stop at
/// the right. This is the reference layout: controls under the text, never above it.
export function Composer(
  { state = "empty", skill, attachments, warning, mode = "collaborate", provider = "claude" }: {
    state?: "empty" | "typing" | "streaming" | "disabled";
    skill?: Skill;
    attachments?: { name: string; kind: "video" | "audio" | "image" | "file"; size: string }[];
    warning?: string;
    mode?: AutonomyMode;
    provider?: ProviderId;
  },
): Markup {
  const streaming = state === "streaming";
  const disabled = state === "disabled";
  const text = state === "typing"
    ? "Trim the opening clip to 8 seconds and add a lower third for the speaker."
    : state === "streaming"
    ? "Cut the silence from the interview, then caption it."
    : "";
  return (
    <div class={`rounded-[10px] border bg-surface ${disabled ? "border-border opacity-60" : "border-border"}`}>
      {warning && (
        <p class="px-3 pt-2.5 text-[11px] text-warning">{warning}</p>
      )}
      {attachments && (
        <div class="flex flex-wrap gap-1 px-2.5 pt-2.5">
          {attachments.map((a) => <AttachmentChip name={a.name} kind={a.kind} size={a.size} />)}
        </div>
      )}
      <div class={`min-h-[44px] px-3 py-2.5 text-[12px] leading-5 ${text ? "text-text" : "text-muted"}`}>
        {text || "Ask about your edit…"}
        {state === "typing" && <span class="ml-px inline-block h-3.5 w-px translate-y-0.5 bg-accent-soft" />}
      </div>
      <div class="flex items-center gap-1 px-2 pb-2">
        <NodaButton label="Attach file" icon="plus" iconOnly state={disabled ? "Disabled" : "Default"} />
        <SkillButton skill={skill} />
        <ModelButton provider={provider} />
        <AutonomyButton mode={mode} />
        <span class="ml-auto" />
        {streaming
          ? <NodaButton label="Stop · Esc" icon="stop" iconOnly primary />
          : state === "typing"
          ? <NodaButton label="Send · Enter" icon="arrowUp" primary state={disabled ? "Disabled" : "Default"} />
          : <NodaButton label="Send · Enter" icon="arrowUp" iconOnly primary state={disabled || state === "empty" ? "Disabled" : "Default"} />}
      </div>
    </div>
  );
}

/// The context line under the composer: which project and sequence the agent is acting
/// on, and whether it is currently working. References, never copies.
export function ComposerContext({ working = false }: { working?: boolean }): Markup {
  return (
    <div class="flex items-center gap-3 px-1 text-[10px] text-muted">
      <span class="flex items-center gap-1.5"><NodaGlyph name="folder" size={12} />City after hours</span>
      <span class="flex items-center gap-1.5"><NodaGlyph name="layers" size={12} />Sequence 01</span>
      <span class="ml-auto flex items-center gap-1.5">
        {working
          ? <><span class="size-1.5 rounded-full bg-warning" />Working…</>
          : <><span class="size-1.5 rounded-full bg-success" />Ready</>}
      </span>
    </div>
  );
}

/// The session tab strip across the top of the window: one tab per open conversation,
/// a new-tab button, and window-level context (gateway revision, search, sidebar).
export function SessionTabBar({ title = "New session" }: { title?: string }): Markup {
  return (
    <div class="flex h-10 shrink-0 items-center gap-1 border-b border-border bg-panel px-2">
      <span class="flex h-7 min-w-0 max-w-[280px] items-center gap-2 rounded-[6px] bg-surface px-3 text-[11px] text-text">
        <span class="shrink-0 text-accent-soft"><NodaGlyph name="assistant" size={12} /></span>
        <span class="truncate">{title}</span>
        <span class="shrink-0 text-dim"><NodaGlyph name="close" size={11} /></span>
      </span>
      <span class="grid size-7 shrink-0 place-items-center rounded-[6px] text-muted hover:bg-hover">
        <NodaGlyph name="plus" size={13} />
      </span>
      <span class="ml-auto flex items-center gap-2">
        <span class="rounded-[3px] bg-surface px-1.5 py-0.5 font-mono text-[9px] text-subtle">MCP · rev 12</span>
        <span class="grid size-7 place-items-center rounded-[6px] text-muted hover:bg-hover"><NodaGlyph name="search" size={13} /></span>
        <span class="grid size-7 place-items-center rounded-[6px] text-muted hover:bg-hover"><NodaGlyph name="inspector" size={13} /></span>
      </span>
    </div>
  );
}

/// The sessions sidebar: project switcher, new chat, and conversations grouped by day
/// with the provider that ran them. Empty projects say so instead of inventing threads.
export function SessionsSidebar({ empty = false, selected = 0 }: { empty?: boolean; selected?: number }): Markup {
  const groups: { label: string; items: { title: string; provider: ProviderId; time: string }[] }[] = [
    {
      label: "Today",
      items: [
        { title: "Trim opening + lower third", provider: "claude", time: "09:41" },
        { title: "Social cut of the interview", provider: "codex", time: "08:12" },
      ],
    },
    {
      label: "Yesterday",
      items: [
        { title: "Caption the night market", provider: "gemini", time: "18:03" },
        { title: "Brand stinger for the intro", provider: "claude", time: "14:20" },
      ],
    },
  ];
  let index = -1;
  return (
    <aside class="flex h-full min-h-0 w-[260px] shrink-0 flex-col border-r border-border bg-surface text-text">
      <div class="flex h-12 shrink-0 items-center gap-2 px-3">
        <span class="grid size-6 shrink-0 place-items-center rounded-[6px] bg-accent-background text-accent-soft">
          <NodaGlyph name="sparkle" size={13} />
        </span>
        <span class="truncate text-[12px] font-medium">City after hours</span>
        <span class="text-muted"><NodaGlyph name="chevron" size={12} /></span>
        <span class="ml-auto flex items-center gap-1 text-muted">
          <NodaGlyph name="search" size={13} />
          <NodaGlyph name="more" size={13} />
        </span>
      </div>
      <div class="px-2 pb-2">
        <div class="flex h-8 items-center gap-2 rounded-[6px] bg-hover px-2.5 text-[11px] text-light">
          <NodaGlyph name="plus" size={13} />
          New chat
          <span class="ml-auto"><KeyCap>⌘N</KeyCap></span>
        </div>
      </div>
      <div class="flex items-center justify-between border-t border-border px-3 py-2">
        <span class="text-[10px] font-semibold tracking-[0.1em] text-dim uppercase">Sessions</span>
        <span class="text-muted"><NodaGlyph name="sliders" size={12} /></span>
      </div>
      {empty
        ? <p class="px-3 py-2 text-[11px] text-muted">No sessions yet</p>
        : (
          <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
            {groups.map((group) => (
              <div class="mb-3">
                <p class="px-1 pb-1 text-[9px] font-semibold tracking-[0.1em] text-dim uppercase">{group.label}</p>
                {group.items.map((item) => {
                  index += 1;
                  const active = index === selected;
                  const p = PROVIDERS.find((x) => x.id === item.provider) ?? PROVIDERS[0];
                  return (
                    <div class={`mb-0.5 flex items-center gap-2 rounded-[6px] px-2 py-1.5 ${active ? "bg-hover" : ""}`}>
                      <ProviderDot provider={p} size={6} />
                      <span class="truncate text-[11px] text-text">{item.title}</span>
                      <span class="ml-auto shrink-0 font-mono text-[9px] text-dim">{item.time}</span>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
        )}
      <div class="mt-auto flex h-8 shrink-0 items-center gap-2 border-t border-border px-3 text-[10px] text-muted">
        <NodaGlyph name="clock" size={12} />
        Stored in this project
      </div>
    </aside>
  );
}

/// The window status bar: gateway state on the left; pending approvals, actions and the
/// terminal toggle on the right. The toggle is lit while the terminal dock is open.
export function StatusBar({ terminal = false, pending = false }: { terminal?: boolean; pending?: boolean }): Markup {
  return (
    <div class="flex h-8 shrink-0 items-center gap-3 border-t border-border bg-surface px-3 text-[10px] text-muted">
      <span class="flex items-center gap-1.5">
        <span class="size-1.5 rounded-full bg-success" />
        MCP gateway · revision 12
      </span>
      <span class="ml-auto flex items-center gap-3">
        {pending && (
          <span class="flex items-center gap-1 text-accent-soft">
            <NodaGlyph name="shield" size={11} />1 approval pending
          </span>
        )}
        <span class="flex items-center gap-1"><NodaGlyph name="play" size={11} />Actions<NodaGlyph name="chevron" size={10} /></span>
        <span class={`flex items-center gap-1 ${terminal ? "text-accent-soft" : ""}`}>
          <NodaGlyph name="terminal" size={12} />Terminal
        </span>
      </span>
    </div>
  );
}

/// The full-length assistant window: sessions sidebar, session tabs, a wide thread, the
/// bottom composer, an optional terminal dock and the status bar. The editor's timeline
/// lives in the Edit workspace; approvals raised here apply to that one timeline.
export function AssistantWindow(
  { state = "chat", terminal = false }: { state?: WindowState; terminal?: boolean },
): Markup {
  const empty = state === "empty";
  return (
    <div class="flex min-w-[1080px] flex-col overflow-hidden rounded-[6px] border border-border bg-page font-sans text-text">
      <WindowMenuBar />
      <div class="flex h-[520px] min-h-0">
        <SessionsSidebar empty={empty} />
        <div class="flex min-w-0 flex-1 flex-col bg-panel">
          <SessionTabBar title={empty ? "New session" : "Social cut of the interview"} />
          {empty
            ? (
              <div class="flex min-h-0 flex-1 flex-col items-center justify-center gap-4 px-10">
                <h2 class="text-[20px] font-semibold tracking-tight text-text">What are you making?</h2>
                <div class="w-full max-w-[720px] space-y-2">
                  <Composer state="empty" />
                  <ComposerContext />
                </div>
              </div>
            )
            : (
              <>
                <div class="min-h-0 flex-1 space-y-4 overflow-y-auto px-6 py-4">
                  <WindowThread state={state} />
                </div>
                <div class="shrink-0 space-y-2 px-6 pb-3">
                  <Composer
                    state={state === "streaming" ? "streaming" : state === "approval" ? "disabled" : "empty"}
                    skill={state === "chat" ? SKILLS[0] : undefined}
                    mode={state === "streaming" ? "autonomous" : "collaborate"}
                    provider="codex"
                  />
                  <ComposerContext working={state === "streaming"} />
                </div>
              </>
            )}
        </div>
      </div>
      {terminal && <Terminal state={state === "streaming" ? "codex" : "shell"} dock />}
      <StatusBar terminal={terminal} pending={state === "approval"} />
    </div>
  );
}

function WindowThread({ state }: { state: WindowState }): Markup {
  if (state === "approval") {
    return (
      <>
        <UserMessage text="Trim the opening clip to 8 seconds." />
        <AssistantMessage provider="claude" duration="4s">
          <p>I'll shorten the opening clip. The rest of the timeline stays in place.</p>
          <ApprovalCard
            title="Review timeline change"
            detail="Trim the end of the opening video clip."
            tool="trim_clip"
            diff={<DiffLine before="00:00:18:12" after="00:00:08:00" />}
          />
        </AssistantMessage>
      </>
    );
  }
  if (state === "streaming") {
    return (
      <>
        <UserMessage text="Cut the silence from the interview, then caption it." />
        <AssistantMessage provider="codex" duration="17s" working>
          <ThoughtBlock text="Analyzing the interview audio for silence regions, then planning cuts and captions." expanded />
          <ToolRunRow label="Reading 2 files" icon="file" expanded>
            <p class="border-l border-border pl-3">The interview is 6:12 with 1:48 of silence.</p>
            <p class="border-l border-border pl-3">Read <span class="font-mono text-accent-soft">interview.mov</span></p>
            <p class="border-l border-border pl-3">Read <span class="font-mono text-accent-soft">sequence-01.timeline</span></p>
          </ToolRunRow>
          <ToolCallCard tool="analyze_media" status="running" icon="mic" args="asset: interview.mov · audio: true" />
        </AssistantMessage>
      </>
    );
  }
  if (state === "error") {
    return (
      <>
        <UserMessage text="Export the sequence to MP4." />
        <AssistantMessage provider="claude" duration="2s">
          <ErrorCard title="Export couldn't start" detail="No video tracks in the sequence. Add a clip, then export again." />
        </AssistantMessage>
      </>
    );
  }
  return (
    <>
      <UserMessage
        text="Cut the silence from the interview and caption it for social. Keep the best takes."
        attachments={[{ name: "interview.mov", kind: "video", size: "412 MB" }]}
      />
      <AssistantMessage provider="codex" duration="56s" time="08:12">
        <p>I found six silent gaps. Here's the cut plan — approve it and I'll apply the cuts atomically, then generate captions.</p>
        <ToolRunRow label="Ran 2 commands · Read interview.mov" />
        <ToolCallCard tool="analyze_media" status="done" icon="mic" args="interview.mov · peak −3.2 dBFS · rms −18.4 dBFS" result="6 silence regions · 1:48 total" />
        <ApprovalCard
          title="Review edit plan"
          detail="Six smart cuts remove 1:48 of silence. The kept takes stay in order; captions follow."
          tool="propose_changeset"
          diff={
            <div class="space-y-1 rounded-[4px] bg-hover px-2 py-1.5 font-mono text-[10px] text-subtle">
              <p>6 × split_clip + delete_clip</p>
              <p class="text-muted">06:12 → 04:24 · atomic · one undo</p>
            </div>
          }
        />
        <CompositionResultCard />
      </AssistantMessage>
    </>
  );
}

function WindowMenuBar(): Markup {
  return (
    <div class="flex h-7 shrink-0 items-center gap-5 border-b border-border bg-surface px-4 text-[10px]">
      <span class="font-semibold">NodaVideo</span>
      {["File", "Edit", "View", "Playback", "Window", "Help"].map((menu) => <span>{menu}</span>)}
    </div>
  );
}

export type TerminalState = "shell" | "codex" | "claude";

/// The integrated CLI terminal: a tab per agent, a scrollback of real agent output, and
/// an input line. Self-executing agents (Codex, Claude) run here; their timeline edits
/// still surface as NodaVideo approval cards, so the editor keeps the canonical gate.
export function Terminal(
  { state = "codex", dock = false }: { state?: TerminalState; dock?: boolean },
): Markup {
  return (
    <section
      aria-label="Terminal"
      class={`flex flex-col bg-monitor font-mono text-text ${dock ? "h-[200px] shrink-0 border-t border-border" : "h-[360px] w-[620px] overflow-hidden rounded-[6px] border border-border"}`}
    >
      <div class="flex h-8 shrink-0 items-center gap-1 border-b border-border bg-panel px-2 font-sans">
        <TerminalTab label="Codex" active={state === "codex"} hue="#10a37f" />
        <TerminalTab label="Claude Code" active={state === "claude"} hue="#d97757" />
        <TerminalTab label="zsh" active={state === "shell"} hue="#8a90a2" />
        <span class="grid size-6 place-items-center rounded-[3px] text-muted hover:bg-hover"><NodaGlyph name="plus" size={13} /></span>
        <span class="ml-auto flex items-center gap-1 text-muted">
          <span class="grid size-6 place-items-center rounded-[3px] hover:bg-hover"><NodaGlyph name="copy" size={13} /></span>
          <span class="grid size-6 place-items-center rounded-[3px] hover:bg-hover"><NodaGlyph name="trash" size={13} /></span>
          <span class="grid size-6 place-items-center rounded-[3px] hover:bg-hover"><NodaGlyph name="close" size={13} /></span>
        </span>
      </div>
      <div class="min-h-0 flex-1 space-y-1 overflow-y-auto p-3 text-[11px] leading-5">
        <TerminalBody state={state} />
      </div>
      <div class="flex h-9 shrink-0 items-center gap-2 border-t border-border px-3 text-[11px]">
        <span class="text-success">❯</span>
        <span class="text-light">{state === "shell" ? "git status" : ""}</span>
        <span class="inline-block h-3.5 w-1.5 bg-light/70" />
        <span class="ml-auto font-sans text-[9px] text-dim">{state === "shell" ? "zsh · project root" : "MCP connected · revision 12"}</span>
      </div>
    </section>
  );
}

function TerminalTab({ label, active, hue }: { label: string; active: boolean; hue: string }): Markup {
  return (
    <span class={`flex h-6 items-center gap-1.5 rounded-[3px] px-2 text-[10px] ${active ? "bg-monitor text-text" : "text-muted hover:bg-hover"}`}>
      <span class="size-1.5 rounded-full" style={`background:${hue}`} />
      {label}
    </span>
  );
}

function TerminalBody({ state }: { state: TerminalState }): Markup {
  if (state === "shell") {
    return (
      <>
        <p class="text-muted">Last login: Tue Nov 14 09:38 on ttys004</p>
        <p><span class="text-success">❯</span> <span class="text-light">cd ~/Projects/city-after-hours</span></p>
        <p><span class="text-success">❯</span> <span class="text-light">codex</span></p>
        <p class="text-dim">Launching Codex app-server…</p>
      </>
    );
  }
  if (state === "claude") {
    return (
      <>
        <p class="text-muted">╭──────────────────────────────────────────────╮</p>
        <p class="text-muted">│ <span class="text-light">Claude Code</span> · Sonnet 4.5 · NodaVideo MCP │</p>
        <p class="text-muted">╰──────────────────────────────────────────────╯</p>
        <p><span class="text-accent-soft">&gt;</span> <span class="text-light">add a lower third for the speaker at 6s</span></p>
        <p class="text-dim">● Creating composition from Lower Third (Modern)…</p>
        <p><span class="text-success">●</span> <span class="text-accent-soft">create_composition</span><span class="text-muted">(speaker-card)</span></p>
        <p class="text-muted">  ⎿ rendered 96 frames · 1920×1080 · 4.0s</p>
        <p><span class="text-warning">●</span> <span class="text-accent-soft">add_composition_to_timeline</span></p>
        <p class="text-warning">  ⎿ approval requested in NodaVideo — see the Assistant window</p>
        <p class="text-dim">  waiting for your decision…</p>
      </>
    );
  }
  return (
    <>
      <p class="text-muted">╭──────────────────────────────────────────────╮</p>
      <p class="text-muted">│ <span class="text-light">Codex</span> · GPT-5.4 · NodaVideo MCP connected │</p>
      <p class="text-muted">╰──────────────────────────────────────────────╯</p>
      <p><span class="text-accent-soft">&gt;</span> <span class="text-light">cut the silence from the interview</span></p>
      <p class="text-dim">● Thinking…</p>
      <p><span class="text-success">●</span> <span class="text-accent-soft">analyze_media</span><span class="text-muted">(interview.mov)</span></p>
      <p class="text-muted">  ⎿ 6 silence regions · 1:48 total · rms −18.4 dBFS</p>
      <p><span class="text-success">●</span> <span class="text-accent-soft">preview_changeset</span><span class="text-muted">(6 smart cuts)</span></p>
      <p class="text-muted">  ⎿ 06:12 → 04:24 · no state changed</p>
      <p><span class="text-warning">●</span> <span class="text-accent-soft">propose_changeset</span></p>
      <p class="text-warning">  ⎿ approval requested in NodaVideo — see the Assistant window</p>
    </>
  );
}

/// The tool & plugin browser: the MCP catalog grouped, each tool read/write and whether
/// it needs approval, with a per-tool enable switch. The connection state is shown once.
export function ToolBrowser({ groups = TOOL_GROUPS }: { groups?: ToolGroup[] }): Markup {
  const count = groups.reduce((n, g) => n + g.tools.length, 0);
  return (
    <div class="flex w-[320px] flex-col overflow-hidden rounded-[6px] border border-border bg-surface font-sans text-text">
      <div class="flex h-10 shrink-0 items-center justify-between border-b border-border px-3">
        <div class="flex items-center gap-2">
          <NodaGlyph name="plug" size={14} />
          <span class="text-[12px] font-medium">Tools</span>
          <span class="text-[10px] text-muted">{count}</span>
        </div>
        <span class="flex items-center gap-1 rounded-[3px] bg-success/10 px-1.5 py-0.5 text-[9px] text-success">
          <span class="size-1.5 rounded-full bg-success" />MCP connected
        </span>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto py-1">
        {groups.map((group) => (
          <div class="mb-1">
            <div class="flex items-center gap-2 px-3 py-1.5 text-[10px] font-semibold tracking-[0.06em] text-muted uppercase">
              <NodaGlyph name={group.icon} size={12} />
              {group.name}
              <span class="ml-auto font-mono text-[9px] text-dim">{group.tools.length}</span>
            </div>
            {group.tools.map((tool) => (
              <div class="flex items-center gap-2 px-3 py-1.5 hover:bg-hover">
                <span class="min-w-0 flex-1 truncate font-mono text-[10px] text-light">{tool.name}</span>
                {tool.approval && <span class="text-accent-soft" title="Needs approval"><NodaGlyph name="shield" size={11} /></span>}
                <KindBadge kind={tool.kind} />
                <Switch on />
              </div>
            ))}
          </div>
        ))}
      </div>
      <div class="flex h-8 shrink-0 items-center justify-between border-t border-border px-3 text-[9px] text-muted">
        <span>nodavideo · hyperframes</span>
        <span class="flex items-center gap-1"><NodaGlyph name="plus" size={11} />Add server</span>
      </div>
    </div>
  );
}