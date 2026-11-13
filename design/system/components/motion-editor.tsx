import type { Child, Markup } from "@design/jsx/jsx-runtime";
import { COLOR } from "../theme.ts";
import { NodaButton, NodaGlyph } from "./noda-shell.tsx";
import { Icon } from "./icon.tsx";
import { Window } from "./window.tsx";
import { Field, Pair, SectionLabel, Slider } from "./panel.tsx";
import { ALPHA_STYLE, COMPOSITION_FIXTURES, CompositionThumb, MotionMark } from "./motion-media.tsx";

const AMBER = "#f0c987";
const HERO = COMPOSITION_FIXTURES[0]; // Speaker card · lower third

function AreaHeader({ title, children }: { title: string; children?: Child }): Markup {
  return (
    <div class="flex h-9 shrink-0 items-center justify-between border-b border-border px-3">
      <span class="text-[11px] font-medium text-text">{title}</span>
      {children}
    </div>
  );
}

/// The elements/layers tree: the composition root and its animated elements. The selected
/// element drives the canvas handles and the properties panel.
export function LayersTree(): Markup {
  const rows = [
    { name: "Accent bar", color: "#6366f1", depth: 1, selected: false },
    { name: "Name", color: "#f1f3f9", depth: 1, selected: true },
    { name: "Role", color: "#94a3b8", depth: 1, selected: false },
  ];
  return (
    <div class="flex h-full min-h-0 flex-col border-r border-border bg-surface text-text">
      <AreaHeader title="Layers">
        <NodaButton label="Add element" icon="plus" iconOnly />
      </AreaHeader>
      <div class="min-h-0 flex-1 overflow-y-auto p-2 text-[11px]">
        <div class="flex items-center gap-1.5 rounded-[3px] px-1.5 py-1 text-subtle">
          <NodaGlyph name="chevron" size={11} />
          <span style={`color:${AMBER}`}><MotionMark size={12} /></span>
          <span class="truncate">Speaker card</span>
        </div>
        {rows.map((row) => (
          <div
            class={`mt-0.5 flex items-center gap-1.5 rounded-[3px] py-1 pr-1.5 pl-6 ${
              row.selected ? "bg-accent-background text-text" : "text-subtle"
            }`}
            style={row.selected ? "box-shadow:inset 2px 0 0 var(--color-accent)" : undefined}
          >
            <span class="size-2.5 shrink-0 rounded-[2px]" style={`background:${row.color}`} />
            <span class="min-w-0 flex-1 truncate">{row.name}</span>
            <Icon name="eye" width={11} height={11} color={COLOR.muted} />
            <Icon name="lock" width={11} height={11} color={COLOR.dim} />
          </div>
        ))}
      </div>
      <div class="shrink-0 border-t border-border px-3 py-2 text-[10px] leading-4 text-muted">
        3 elements · edited through the HyperFrames SDK
      </div>
    </div>
  );
}

/// The composition canvas at a local time, over a checkerboard for alpha. The selected
/// element shows a bounding box with resize handles and a name tag.
export function EditorCanvas(): Markup {
  const handles = [
    [0, 0], [50, 0], [100, 0],
    [0, 50], [100, 50],
    [0, 100], [50, 100], [100, 100],
  ];
  return (
    <div class="flex h-full min-h-0 flex-col bg-page text-text">
      <div class="flex h-9 shrink-0 items-center justify-between border-b border-border px-3 text-[10px] text-muted">
        <span>Canvas · local time</span>
        <span class="font-mono">Fit · 50%</span>
      </div>
      <div class="flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-monitor p-4">
        <div class="relative overflow-hidden rounded-[2px]" style={`width:600px;height:338px;${ALPHA_STYLE}`}>
          <CompositionThumb comp={HERO} />
          <div class="absolute" style="left:40px;top:252px;width:190px;height:30px">
            <div class="absolute inset-0 border border-accent" />
            <span class="absolute -top-5 left-0 rounded-[2px] bg-accent px-1.5 py-0.5 text-[9px] font-medium text-on-accent">Name</span>
            {handles.map(([x, y]) => (
              <span
                class="absolute size-2 border border-accent bg-panel"
                style={`left:calc(${x}% - 4px);top:calc(${y}% - 4px)`}
              />
            ))}
          </div>
        </div>
      </div>
      <div class="grid h-9 shrink-0 grid-cols-[1fr_auto_1fr] items-center border-t border-border px-3">
        <span class="font-mono text-[11px] text-accent-soft">00:00:01:12</span>
        <div class="flex items-center gap-1">
          <NodaButton label="Previous frame" icon="previous" iconOnly />
          <NodaButton label="Play" icon="play" iconOnly />
          <NodaButton label="Next frame" icon="next" iconOnly />
        </div>
        <span class="text-right font-mono text-[10px] text-muted">00:00:04:00</span>
      </div>
    </div>
  );
}

/// The properties and animation controls for the selected element: transform, typography
/// and animation timing. Visual edits write back to the HTML source through the SDK.
export function EditorProperties(): Markup {
  return (
    <div class="flex h-full min-h-0 flex-col border-l border-border bg-surface text-text">
      <AreaHeader title="Properties" />
      <div class="min-h-0 flex-1 overflow-y-auto px-3 py-3">
        <div class="mb-3 flex items-center gap-2">
          <span class="size-2.5 rounded-[2px] bg-text" />
          <span class="text-[12px] font-medium">Name</span>
          <span class="ml-auto text-[10px] text-muted">Text</span>
        </div>
        <SectionLabel>TRANSFORM</SectionLabel>
        <div class="mt-2 space-y-2">
          <div class="flex gap-2">
            <Field prefix="X" value="140" width={110} />
            <Field prefix="Y" value="885" width={110} />
          </div>
          <div class="flex items-center justify-between">
            <span class="text-[11px] text-muted">Opacity</span>
            <Slider fill={104} />
          </div>
          <Pair name="Scale" value="100 %" />
          <Pair name="Rotation" value="0.0°" />
        </div>
        <div class="my-3 h-px bg-divider" />
        <SectionLabel>TYPOGRAPHY</SectionLabel>
        <div class="mt-2 space-y-2">
          <Pair name="Font" value="Sans" />
          <Pair name="Size" value="64 px" />
          <div class="flex items-center justify-between">
            <span class="text-[11px] text-muted">Color</span>
            <span class="flex items-center gap-1.5 font-mono text-[12px]">
              <span class="size-3.5 rounded-[3px] border border-border bg-white" />#FFFFFF
            </span>
          </div>
        </div>
        <div class="my-3 h-px bg-divider" />
        <SectionLabel>ANIMATION</SectionLabel>
        <div class="mt-2 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-[11px] text-muted">In</span>
            <Field value="0.0" unit="s" width={110} />
          </div>
          <div class="flex items-center justify-between">
            <span class="text-[11px] text-muted">Out</span>
            <Field value="4.0" unit="s" width={110} />
          </div>
          <div class="flex h-8 items-center justify-between rounded-md border border-border bg-surface px-2 text-[12px]">
            <span class="text-muted">Easing</span>
            <span class="flex items-center gap-1.5">Ease Out<NodaGlyph name="chevron" size={11} /></span>
          </div>
        </div>
      </div>
    </div>
  );
}

/// The composition's internal animation timeline: one row per element, with keyframe
/// diamonds and a local playhead. This is the composition's own time, not the native
/// timeline — the native clip owns where the whole composition sits in the sequence.
export function CompositionTimeline(): Markup {
  const rows = [
    { name: "Accent bar", color: "#6366f1", left: 0, width: 100, keys: [0, 12] },
    { name: "Name", color: "#f1f3f9", left: 8, width: 92, keys: [8, 20, 100] },
    { name: "Role", color: "#94a3b8", left: 16, width: 84, keys: [16, 28, 100] },
  ];
  const playhead = 37.5;
  return (
    <div class="flex h-full min-h-0 flex-col border-t border-border bg-page text-text">
      <div class="flex h-9 shrink-0 items-center gap-3 border-b border-border px-3">
        <span class="text-[11px] font-medium">Composition timeline</span>
        <span class="font-mono text-[10px] text-accent-soft">00:00:01:12 / 00:00:04:00</span>
        <NodaButton label="Play" icon="play" iconOnly />
        <NodaButton label="Add keyframe" icon="plus" />
        <span class="ml-auto text-[10px] text-muted">Local time · 24 fps</span>
      </div>
      <div class="grid h-6 shrink-0 grid-cols-[180px_1fr] border-b border-border">
        <div class="flex items-center border-r border-border px-3 text-[9px] text-muted">ELEMENTS</div>
        <div class="relative flex items-center justify-between px-3 font-mono text-[9px] text-muted">
          {["0s", "1s", "2s", "3s", "4s"].map((t) => <span>{t}</span>)}
        </div>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto">
        {rows.map((row) => (
          <div class="grid h-9 grid-cols-[180px_1fr] border-b border-border/60">
            <div class="flex items-center gap-2 border-r border-border px-3">
              <span class="size-2.5 rounded-[2px]" style={`background:${row.color}`} />
              <span class="truncate text-[10px] text-subtle">{row.name}</span>
            </div>
            <div class="relative mx-3">
              <div
                class="absolute inset-y-2 rounded-[3px] bg-accent/18"
                style={`left:${row.left}%;width:${row.width}%`}
              />
              {row.keys.map((k) => (
                <span
                  class="absolute top-1/2 size-2 -translate-x-1/2 -translate-y-1/2 rotate-45 border border-accent-soft bg-accent"
                  style={`left:${k}%`}
                />
              ))}
              <div class="absolute inset-y-0 w-px bg-accent" style={`left:${playhead}%`} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

const CODE_LINES: { n: number; parts: [string, string][] }[] = [
  { n: 1, parts: [["<div ", "tag"], ["id", "attr"], ["=\"root\" ", "str"], ["data-duration", "attr"], ["=\"4\"", "str"], [">", "tag"]] },
  { n: 2, parts: [["  <div ", "tag"], ["class", "attr"], ["=\"bar\" ", "str"], ["/>", "tag"]] },
  { n: 3, parts: [["  <div ", "tag"], ["class", "attr"], ["=\"name\"", "str"], [">", "tag"], ["Amara Okafor", "txt"], ["</div>", "tag"]] },
  { n: 4, parts: [["  <div ", "tag"], ["class", "attr"], ["=\"role\"", "str"], [">", "tag"], ["Lead Director & Editor", "txt"], ["</div>", "tag"]] },
  { n: 5, parts: [["</div>", "tag"]] },
  { n: 6, parts: [["", "txt"]] },
  { n: 7, parts: [["// animations.js — GSAP, seekable", "cmt"]] },
  { n: 8, parts: [["const ", "kw"], ["tl = gsap.timeline({ paused: ", "txt"], ["true", "kw"], [" });", "txt"]] },
  { n: 9, parts: [["tl.from(", "txt"], [".bar", "str"], [", { scaleY: ", "txt"], ["0", "num"], [", duration: ", "txt"], ["0.4", "num"], [" });", "txt"]] },
  { n: 10, parts: [["tl.from(", "txt"], [".name", "str"], [", { x: ", "txt"], ["-50", "num"], [", opacity: ", "txt"], ["0", "num"], [" }, ", "txt"], ["0.2", "num"], [");", "txt"]] },
  { n: 11, parts: [["tl.from(", "txt"], [".role", "str"], [", { opacity: ", "txt"], ["0", "num"], [" }, ", "txt"], ["0.4", "num"], [");", "txt"]] },
  { n: 12, parts: [["hyperframes.register(tl);", "txt"]] },
];

const CODE_COLOR: Record<string, string> = {
  tag: "text-accent-soft",
  attr: "text-[#c3a9e8]",
  str: "text-[#f0c987]",
  txt: "text-light",
  kw: "text-[#7fd0ae]",
  num: "text-[#f0c987]",
  cmt: "text-dim",
};

/// The code-editing level: HTML, CSS and JS (GSAP) tabs over the composition source, with
/// validation. The source stays the source of truth; the visual editor writes through the SDK.
export function EditorCodePane(): Markup {
  return (
    <div class="flex h-full min-h-0 flex-col bg-page text-text">
      <div class="flex h-9 shrink-0 items-center gap-1 border-b border-border px-2">
        {["HTML", "CSS", "JS · GSAP"].map((tab, i) => (
          <span
            class={`rounded-[3px] px-2.5 py-1 text-[11px] ${
              i === 0 ? "bg-hover text-text" : "text-muted"
            }`}
          >
            {tab}
          </span>
        ))}
        <span class="ml-auto">
          <NodaButton label="Validate" icon="check" />
        </span>
      </div>
      <div class="min-h-0 flex-1 overflow-auto bg-[#0d0e11] py-3 font-mono text-[11px] leading-5">
        {CODE_LINES.map((line) => (
          <div class="flex px-3">
            <span class="w-7 shrink-0 select-none text-right text-dim">{line.n}</span>
            <span class="pl-3 whitespace-pre">
              {line.parts.map(([text, kind]) => <span class={CODE_COLOR[kind]}>{text}</span>)}
            </span>
          </div>
        ))}
      </div>
      <div class="flex h-8 shrink-0 items-center gap-2 border-t border-border px-3 text-[10px]">
        <span class="size-1.5 rounded-full bg-success" />
        <span class="text-subtle">Valid · 3 elements · 1920 × 1080 · 4.0 s</span>
        <span class="ml-auto text-muted">index.html</span>
      </div>
    </div>
  );
}

/// The dedicated Motion Graphics Editor window. `mode` swaps the center area between the
/// visual canvas and the code editor; layers, properties and the composition timeline stay.
export function MotionEditorWindow({ mode = "visual" }: { mode?: "visual" | "code" }): Markup {
  return (
    <Window width={1180} height={720}>
      <div class="flex h-12 items-center gap-3 border-b border-border bg-surface px-3">
        <NodaButton label="Back to edit" icon="collapse" />
        <span class="mx-1 h-4 border-r border-border" />
        <span style={`color:${AMBER}`}><MotionMark size={14} /></span>
        <span class="text-[13px] font-semibold">Speaker card</span>
        <span class="text-[10px] text-muted">Lower Third (Modern)</span>
        <div class="ml-auto flex items-center gap-3 text-[10px] text-muted">
          <span class="font-mono">1920 × 1080</span>
          <span class="font-mono">24 fps</span>
          <span class="mx-1 h-4 border-r border-border" />
          <NodaButton label="Save composition" primary />
          <NodaButton label="Export" />
        </div>
      </div>
      <div class="grid h-[464px] grid-cols-[220px_minmax(0,1fr)_260px]">
        <LayersTree />
        {mode === "visual" ? <EditorCanvas /> : <EditorCodePane />}
        <EditorProperties />
      </div>
      <div class="h-[180px]">
        <CompositionTimeline />
      </div>
    </Window>
  );
}