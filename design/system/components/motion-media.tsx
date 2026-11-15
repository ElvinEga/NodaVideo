import type { Markup } from "@design/jsx/jsx-runtime";
import { FONT_SANS } from "../theme.ts";
import { NodaButton, NodaGlyph, NodaPanelHeader } from "./noda-shell.tsx";
import { Scene } from "./scene.tsx";
import {
  ClipSubtitle,
  Divider,
  PanelCard,
  PanelHeader,
  Pair,
  SectionLabel,
  Switch,
} from "./panel.tsx";

// Motion graphics identity: a warm amber, distinct from purple text, blue video and teal
// audio. A composition is a video-layer clip (it composites with alpha over lower tracks),
// so its track stays "V" while the clip body reads as motion graphics.
export const MOTION = {
  fill: "#8a5a1e",
  border: "#a86e2c",
  badge: "#f0c987",
  well: "#1f160c",
} as const;

/// The transparency backdrop: a dark checkerboard, the convention for "alpha here".
export const ALPHA_STYLE =
  "background-image:repeating-conic-gradient(#2a2d34 0% 25%,#232529 0% 50%);background-size:16px 16px;";

const footage = "data:image/svg+xml," + encodeURIComponent(Scene({ width: 640 }).html);

export type MotionKind =
  | "lower-third"
  | "kinetic"
  | "minimal"
  | "caption"
  | "stinger"
  | "quote"
  | "badge"
  | "wipe"
  | "iris";

/// A configured composition kept in the project. Fixtures mirror the real workspace
/// templates in templates/; previews are illustrative stills, not rendered frames.
export type Composition = {
  name: string;
  template: string;
  category: string;
  duration: string;
  kind: MotionKind;
  accent: string; // hex without '#'
  text: string;
  sub?: string;
};

export const COMPOSITION_FIXTURES: Composition[] = [
  {
    name: "Speaker card",
    template: "Lower Third (Modern)",
    category: "Lower Thirds",
    duration: "00:04.0",
    kind: "lower-third",
    accent: "6366f1",
    text: "Amara Okafor",
    sub: "Lead Director & Editor",
  },
  {
    name: "Intro title",
    template: "Kinetic Headline",
    category: "Titles",
    duration: "00:02.0",
    kind: "kinetic",
    accent: "8b5cf6",
    text: "CREATE",
    sub: "FASTER",
  },
  {
    name: "Chapter card",
    template: "Title (Minimal Fade)",
    category: "Titles",
    duration: "00:03.0",
    kind: "minimal",
    accent: "cbd5e1",
    text: "CHAPTER 01",
    sub: "The Awakening",
  },
  {
    name: "Hype caption",
    template: "Social Caption Card",
    category: "Captions",
    duration: "00:01.5",
    kind: "caption",
    accent: "facc15",
    text: "THIS CHANGES EVERYTHING",
  },
  {
    name: "Pull quote",
    template: "Quote Callout",
    category: "Captions",
    duration: "00:03.0",
    kind: "quote",
    accent: "a5b4fc",
    text: "Design is intelligence made visible.",
    sub: "Alina Wheeler",
  },
  {
    name: "Brand stinger",
    template: "Branded Stinger",
    category: "Transitions",
    duration: "00:01.0",
    kind: "stinger",
    accent: "0f172a",
    text: "NODACON",
  },
];

/// The built-in template library, grouped as the real templates/ directory is.
export const TEMPLATES: Composition[] = [
  { name: "Title (Minimal Fade)", template: "title-minimal", category: "Titles", duration: "3.0s", kind: "minimal", accent: "cbd5e1", text: "CHAPTER 01", sub: "The Awakening" },
  { name: "Kinetic Headline", template: "kinetic-headline", category: "Titles", duration: "2.0s", kind: "kinetic", accent: "8b5cf6", text: "CREATE", sub: "FASTER" },
  { name: "Lower Third (Modern)", template: "lower-third-modern", category: "Lower Thirds", duration: "4.0s", kind: "lower-third", accent: "6366f1", text: "Alex Rivera", sub: "Lead Director & Editor" },
  { name: "Broadcast Badge", template: "broadcast-badge", category: "Lower Thirds", duration: "3.0s", kind: "badge", accent: "ef4444", text: "LIVE", sub: "Night market" },
  { name: "Social Caption Card", template: "social-caption-card", category: "Captions", duration: "1.5s", kind: "caption", accent: "facc15", text: "THIS CHANGES EVERYTHING" },
  { name: "Quote Callout", template: "quote-callout", category: "Captions", duration: "3.0s", kind: "quote", accent: "a5b4fc", text: "Design is intelligence made visible.", sub: "Alina Wheeler" },
  { name: "Branded Wipe", template: "branded-wipe", category: "Transitions", duration: "1.0s", kind: "wipe", accent: "6366f1", text: "NODACON" },
  { name: "Geometric Iris", template: "geometric-iris", category: "Transitions", duration: "1.0s", kind: "iris", accent: "6366f1", text: "" },
];

/// A four-point spark: the motion-graphics mark, used in badges and the panel header.
export function MotionMark({ size = 14, class: className }: { size?: number; class?: string }): Markup {
  return (
    <svg
      aria-hidden="true"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      class={className}
      fill="currentColor"
    >
      <path d="M12 2l2.2 7.8L22 12l-7.8 2.2L12 22l-2.2-7.8L2 12l7.8-2.2z" />
    </svg>
  );
}

/// Draws a composition's visual in its native 1920×1080 space, scaled to fill the parent.
/// The layer itself is transparent; the parent supplies a checkerboard (alpha) or footage.
/// `progress` (0–1) animates the lower third in, so a clip filmstrip reads as motion.
export function CompositionThumb(
  { comp, progress = 1, class: className }: { comp: Composition; progress?: number; class?: string },
): Markup {
  const accent = `#${comp.accent}`;
  const p = progress;
  const sans = `font-family:${FONT_SANS}`;
  let body: Markup;
  switch (comp.kind) {
    case "lower-third":
      body = (
        <g>
          <rect x="100" y={970 - 150 * p} width="14" height={150 * p} rx="7" fill={accent} />
          <text x={140 + (1 - p) * 50} y="885" font-size="64" font-weight="700" fill="#ffffff" opacity={p} style={sans}>{comp.text}</text>
          <text x={140 + (1 - p) * 50} y="948" font-size="36" fill="#94a3b8" opacity={p} style={sans}>{comp.sub}</text>
        </g>
      );
      break;
    case "kinetic":
      body = (
        <g style={sans}>
          <text x="960" y="500" text-anchor="middle" font-size="190" font-weight="900" letter-spacing="-8" fill="#ffffff">{comp.text}</text>
          <text x="960" y="705" text-anchor="middle" font-size="190" font-weight="900" letter-spacing="-8" fill={accent}>{comp.sub}</text>
        </g>
      );
      break;
    case "minimal":
      body = (
        <g style={sans}>
          <text x="960" y="520" text-anchor="middle" font-size="100" font-weight="800" letter-spacing="14" fill="#ffffff">{comp.text}</text>
          <text x="960" y="615" text-anchor="middle" font-size="42" letter-spacing="3" fill="#cbd5e1">{comp.sub}</text>
        </g>
      );
      break;
    case "caption":
      body = (
        <g transform="rotate(-1.5 960 845)">
          <rect x="430" y="770" width="1060" height="150" rx="18" fill={accent} />
          <text x="960" y="872" text-anchor="middle" font-size="64" font-weight="900" letter-spacing="2" fill="#000000" style={sans}>{comp.text}</text>
        </g>
      );
      break;
    case "stinger":
      body = (
        <g>
          <rect width="1920" height="1080" fill={accent} />
          <text x="960" y="585" text-anchor="middle" font-size="150" font-weight="900" letter-spacing="30" fill="#ffffff" style={sans}>{comp.text}</text>
        </g>
      );
      break;
    case "quote":
      body = (
        <g style={sans}>
          <text x="960" y="450" text-anchor="middle" font-size="220" font-weight="800" fill={accent} opacity="0.85">“</text>
          <text x="960" y="580" text-anchor="middle" font-size="52" font-weight="600" fill="#ffffff">{comp.text}</text>
          <text x="960" y="660" text-anchor="middle" font-size="34" fill="#94a3b8">{comp.sub}</text>
        </g>
      );
      break;
    case "badge":
      body = (
        <g style={sans}>
          <rect x="1480" y="80" width="360" height="92" rx="46" fill={accent} />
          <circle cx="1545" cy="126" r="15" fill="#ffffff" opacity="0.9" />
          <text x="1690" y="143" text-anchor="middle" font-size="48" font-weight="800" fill="#ffffff">{comp.text}</text>
          <text x="1840" y="225" text-anchor="end" font-size="38" fill="#ffffff">{comp.sub}</text>
        </g>
      );
      break;
    case "wipe":
      body = (
        <g>
          <path d="M0 0L1240 0L940 1080L0 1080Z" fill={accent} opacity="0.96" />
          <text x="470" y="585" text-anchor="middle" font-size="120" font-weight="900" letter-spacing="16" fill="#ffffff" style={sans}>{comp.text}</text>
        </g>
      );
      break;
    case "iris":
      body = (
        <g fill="none" stroke={accent}>
          <circle cx="960" cy="540" r="520" stroke-width="26" opacity="0.35" />
          <circle cx="960" cy="540" r="400" stroke-width="26" opacity="0.55" />
          <circle cx="960" cy="540" r="280" stroke-width="26" opacity="0.75" />
          <circle cx="960" cy="540" r="160" stroke-width="26" />
          <circle cx="960" cy="540" r="70" fill={accent} stroke="none" />
        </g>
      );
      break;
  }
  return (
    <svg
      aria-hidden="true"
      class={`size-full ${className ?? ""}`}
      viewBox="0 0 1920 1080"
      preserveAspectRatio="xMidYMid meet"
    >
      {body}
    </svg>
  );
}

/// A composition card in the Media Pool: a rendered still over a checkerboard, the spark
/// badge, name, template and duration. Grid and list share one shape.
function MotionCard(
  { comp, selected, list }: { comp: Composition; selected: boolean; list: boolean },
): Markup {
  return (
    <div
      class={`min-w-0 overflow-hidden rounded-[3px] border ${
        selected ? "border-accent bg-accent-background" : "border-border bg-panel"
      } ${list ? "mb-1 flex items-center gap-2 p-1" : ""}`}
    >
      <div
        class={`relative flex shrink-0 items-center justify-center overflow-hidden ${
          list ? "h-9 w-14" : "aspect-video w-full"
        }`}
        style={ALPHA_STYLE}
      >
        <CompositionThumb comp={comp} />
        {!list && (
          <span
            class="absolute top-1 left-1 grid size-4 place-items-center rounded-[2px]"
            style={`background:${MOTION.fill};color:${MOTION.badge}`}
          >
            <MotionMark size={10} />
          </span>
        )}
        {!list && (
          <span class="absolute right-1 bottom-1 rounded-[2px] bg-monitor/90 px-1 py-0.5 font-mono text-[9px]">
            {comp.duration}
          </span>
        )}
      </div>
      <div class={`min-w-0 flex-1 ${list ? "" : "p-2"}`}>
        <span class="block truncate text-[10px]">{comp.name}</span>
        <div class="mt-1 flex items-center justify-between gap-1 text-[9px] text-muted">
          <span class="truncate">{comp.template}</span>
          <span class="shrink-0" style={`color:${MOTION.badge}`}>
            <MotionMark size={9} />
          </span>
        </div>
      </div>
      {list && <span class="pr-2 font-mono text-[10px] text-subtle">{comp.duration}</span>}
    </div>
  );
}

/// The Media Browser filtered to the Motion category. New Motion Graphic opens the template
/// gallery; the bin lists project compositions, not files. With `empty`, it invites creation.
export function MotionMediaBin(
  { comps, view = "grid", selected = 0, empty = false }: {
    comps: Composition[];
    view?: "grid" | "list";
    selected?: number;
    empty?: boolean;
  },
): Markup {
  const count = empty ? 0 : comps.length;
  const categories = ["All", "Video", "Audio", "Image", "Captions", "Text", "Motion"];
  return (
    <section
      aria-label="Media Browser"
      class="flex h-full min-h-[370px] min-w-0 flex-col bg-surface text-text"
    >
      <NodaPanelHeader title="Media Browser" detail={String(count)}>
        <NodaButton label="New Motion Graphic" primary />
      </NodaPanelHeader>
      <div class="flex items-center gap-2 px-3 pt-3">
        <div class="flex h-7 min-w-0 flex-1 items-center gap-2 rounded-[3px] border border-border bg-page px-2 text-muted">
          <NodaGlyph name="search" />
          <span class="truncate text-[11px]">Search media…</span>
        </div>
        <NodaButton label="Grid view" icon="grid" iconOnly state={view === "grid" ? "Pressed" : "Default"} />
        <NodaButton label="List view" icon="list" iconOnly state={view === "list" ? "Pressed" : "Default"} />
      </div>
      <div class="flex gap-1 overflow-x-auto px-3 py-2.5">
        {categories.map((filter) => (
          <button
            type="button"
            aria-pressed={String(filter === "Motion")}
            class={`shrink-0 rounded-[3px] px-2 py-1 text-[10px] ${
              filter === "Motion" ? "bg-hover text-text" : "text-muted"
            }`}
          >
            {filter}
          </button>
        ))}
      </div>
      {empty
        ? (
          <div class="m-3 mt-0 flex flex-1 flex-col items-center justify-center gap-3 rounded-[3px] border border-dashed border-rail px-5 py-8 text-center">
            <span class="grid size-9 place-items-center rounded-[6px]" style={`background:${MOTION.fill};color:${MOTION.badge}`}>
              <MotionMark size={18} />
            </span>
            <p class="text-[12px] font-medium">Create your first motion graphic</p>
            <p class="max-w-[220px] text-[11px] leading-5 text-muted">
              Pick a template, edit its text and colors, then drag it onto a video track.
              HTML compositions live in your project.
            </p>
            <NodaButton label="New Motion Graphic" primary />
          </div>
        )
        : (
          <div
            class={`min-h-0 flex-1 overflow-y-auto px-3 pb-3 ${
              view === "list" ? "" : "grid auto-rows-max content-start grid-cols-2 gap-2"
            }`}
          >
            {comps.map((comp, index) => (
              <MotionCard comp={comp} selected={index === selected} list={view === "list"} />
            ))}
          </div>
        )}
      <footer class="flex h-7 shrink-0 items-center justify-between border-t border-border px-3 text-[10px] text-muted">
        <span>{empty ? "No compositions yet" : `${count} compositions · 1 selected`}</span>
        <span>HTML · in project</span>
      </footer>
    </section>
  );
}

/// The source monitor showing a composition: a rendered frame over a checkerboard (alpha)
/// or over footage (composited), with Add to Timeline. The transport reads the composition
/// duration; the frame is produced by the HyperFrames renderer, not decoded from a file.
export function MotionSourceMonitor(
  { comp, over = "alpha" }: { comp: Composition; over?: "alpha" | "footage" },
): Markup {
  return (
    <section
      aria-label="Source monitor"
      class="flex h-full min-h-[370px] min-w-0 flex-col bg-panel text-text"
    >
      <NodaPanelHeader title="Source" detail={comp.name}>
        <div class="flex items-center gap-2">
          <div class="flex overflow-hidden rounded-[3px] border border-border text-[10px]">
            <span class={`px-2 py-0.5 ${over === "alpha" ? "bg-hover text-text" : "text-muted"}`}>Alpha</span>
            <span class={`px-2 py-0.5 ${over === "footage" ? "bg-hover text-text" : "text-muted"}`}>Footage</span>
          </div>
          <NodaButton label="Fullscreen · Cmd-F" icon="expand" iconOnly />
        </div>
      </NodaPanelHeader>
      <div class="flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-monitor p-5">
        <div class="relative aspect-video w-full max-w-[440px] overflow-hidden rounded-[2px]" style={over === "alpha" ? ALPHA_STYLE : undefined}>
          {over === "footage" && <img src={footage} alt="" class="absolute inset-0 size-full object-cover" />}
          <div class="absolute inset-0">
            <CompositionThumb comp={comp} />
          </div>
        </div>
      </div>
      <div class="grid h-11 shrink-0 grid-cols-[1fr_auto_1fr] items-center border-t border-border px-3">
        <span class="font-mono text-[11px] text-accent-soft">00:00:01:12</span>
        <div class="flex items-center gap-1">
          <NodaButton label="Previous frame" icon="previous" iconOnly />
          <NodaButton label="Play · Space" icon="play" iconOnly />
          <NodaButton label="Next frame" icon="next" iconOnly />
        </div>
        <span class="text-right font-mono text-[10px] text-muted">{comp.duration}</span>
      </div>
      <div class="flex h-11 shrink-0 items-center justify-between border-t border-border px-3">
        <span class="flex items-center gap-1.5 text-[10px] text-muted">
          <span style={`color:${MOTION.badge}`}><MotionMark size={11} /></span>
          {comp.template}
        </span>
        <NodaButton label="Add to Timeline" primary />
      </div>
    </section>
  );
}

/// The template gallery opened by New Motion Graphic: the built-in library grouped by
/// category, each a rendered still. Choosing one creates a composition with default
/// parameters; nothing is imported and no file is copied.
export function TemplateGallery({ selected = 2 }: { selected?: number }): Markup {
  const groups = ["Titles", "Lower Thirds", "Captions", "Transitions"];
  let cardIndex = -1;
  return (
    <div class="w-[720px] overflow-hidden rounded-[10px] border border-border bg-panel font-sans text-text shadow-menu">
      <div class="flex h-12 items-center justify-between border-b border-border px-4">
        <div class="flex items-center gap-2">
          <span style={`color:${MOTION.badge}`}><MotionMark size={16} /></span>
          <span class="text-[13px] font-semibold">New Motion Graphic</span>
        </div>
        <NodaButton label="Close" icon="close" iconOnly />
      </div>
      <div class="max-h-[440px] overflow-y-auto px-4 py-3">
        {groups.map((group) => (
          <div class="mb-4">
            <p class="mb-2 text-[10px] font-semibold tracking-[0.12em] text-muted uppercase">{group}</p>
            <div class="grid grid-cols-4 gap-2">
              {TEMPLATES.filter((t) => t.category === group).map((t) => {
                cardIndex += 1;
                const isSelected = cardIndex === selected;
                return (
                  <div
                    class={`overflow-hidden rounded-[4px] border ${
                      isSelected ? "border-accent bg-accent-background" : "border-border bg-surface"
                    }`}
                  >
                    <div class="relative aspect-video w-full" style={ALPHA_STYLE}>
                      <CompositionThumb comp={t} />
                      {isSelected && (
                        <span class="absolute top-1 right-1 grid size-4 place-items-center rounded-full bg-accent text-on-accent">
                          <NodaGlyph name="check" size={10} />
                        </span>
                      )}
                    </div>
                    <div class="p-1.5">
                      <p class="truncate text-[10px]">{t.name}</p>
                      <p class="mt-0.5 font-mono text-[9px] text-muted">{t.duration}</p>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        ))}
      </div>
      <div class="flex h-12 items-center justify-between border-t border-border px-4">
        <span class="text-[10px] text-muted">Creates a composition with default parameters · editable in the Inspector</span>
        <div class="flex gap-2">
          <NodaButton label="Cancel" />
          <NodaButton label="Create" primary />
        </div>
      </div>
    </div>
  );
}

function ParamText({ label, value }: { label: string; value: string }): Markup {
  return (
    <div>
      <p class="mb-1 text-[11px] text-soft">{label}</p>
      <div class="flex h-8 items-center rounded-md border border-border bg-surface px-2 text-[13px]">{value}</div>
    </div>
  );
}

function ParamColor({ label, hex }: { label: string; hex: string }): Markup {
  return (
    <div>
      <p class="mb-1 text-[11px] text-soft">{label}</p>
      <div class="flex h-8 items-center gap-2 rounded-md border border-border bg-surface px-2">
        <span class="size-4 rounded-[4px] border border-border" style={`background:${hex}`} />
        <span class="font-mono text-[13px] text-muted">#</span>
        <span class="font-mono text-[13px]">{hex.replace("#", "").toUpperCase()}</span>
      </div>
    </div>
  );
}

/// The Inspector for a selected composition clip: source facts, the template's editable
/// parameters, compositing, and the door into the Motion Graphics Editor. Placing a
/// composition copies its parameters into the clip, like a text title.
export function CompositionPanel(): Markup {
  return (
    <PanelCard height={700}>
      <PanelHeader
        icon={{ type: "file", label: "MG", labelColor: MOTION.badge, border: MOTION.border }}
        title="Speaker card"
        subtitle={<ClipSubtitle clipType="Motion graphic" track="Video 2" kind="V" />}
        bottomDivider
      />
      <div class="h-[644px] overflow-y-auto px-3 py-3">
        <SectionLabel>SOURCE</SectionLabel>
        <div class="mt-1">
          <Pair name="Template" value="Lower Third" />
          <Pair name="Size" value="1920 × 1080" />
          <Pair name="Frame rate" value="24 fps" />
          <Pair name="Duration" value="4.00 s · 96 f" />
        </div>
        <div class="mt-1 flex h-8 items-center rounded-md bg-well px-2 font-mono text-[11px] text-subtle">
          <span class="truncate">compositions/speaker-card/</span>
        </div>
        <Divider class="my-3" />
        <SectionLabel action="Reset">PARAMETERS</SectionLabel>
        <div class="mt-2 space-y-2.5">
          <ParamText label="Speaker Name" value="Amara Okafor" />
          <ParamText label="Title / Role" value="Lead Director & Editor" />
          <ParamColor label="Accent Color" hex="#6366f1" />
        </div>
        <Divider class="my-3" />
        <SectionLabel>COMPOSITING</SectionLabel>
        <div class="mt-2 flex items-center justify-between">
          <span class="text-[12px] text-soft">Transparent background</span>
          <Switch on />
        </div>
        <p class="mt-1.5 text-[10px] leading-4 text-muted">
          Renders with alpha over the tracks below. The native GPU renderer composites it
          with footage.
        </p>
        <Divider class="my-3" />
        <div class="space-y-2">
          <NodaButton label="Open Motion Graphics Editor" primary />
          <NodaButton label="Relink composition" />
        </div>
        <p class="mt-3 text-[10px] leading-4 text-dim">
          Edits here change this clip only; the Media Pool composition keeps its own
          parameters.
        </p>
      </div>
    </PanelCard>
  );
}

/// A compact filmstrip of the composition animating in: each cell is a moment of the lower
/// third (bar grows, text slides and fades in), so the clip body reads as motion, not a still.
function MotionStrip({ width }: { width: number }): Markup {
  const cell = 40;
  const count = Math.ceil(width / cell);
  const moments = [0.4, 0.7, 1];
  return (
    <div class="absolute -top-px -left-px flex" style={`width:${width}px;height:64px`}>
      {Array.from({ length: count }, (_, index) => {
        const p = moments[index % moments.length];
        return (
          <div class="relative shrink-0 overflow-hidden" style={`width:${cell}px;height:64px;background:${MOTION.well}`}>
            <svg aria-hidden="true" width={cell} height="64" viewBox="0 0 40 64">
              <rect x="6" y={54 - 34 * p} width="4" height={34 * p} rx="2" fill="#6366f1" />
              <rect x="14" y={30 - 4 * (1 - p)} width="22" height="5" rx="2" fill="#f1f3f9" opacity={p} />
              <rect x="14" y="40" width="16" height="4" rx="2" fill="#8a90a2" opacity={p} />
            </svg>
            {index > 0 && <div class="absolute top-0 left-0 h-full w-px bg-black/40" />}
          </div>
        );
      })}
    </div>
  );
}

/// A composition clip on a video track: the amber body, an animating filmstrip, the spark
/// badge and the name on a top scrim. Selecting it opens the composition Inspector.
export function MotionClip(
  { width, name, selected }: { width: number; name: string; selected?: boolean },
): Markup {
  const nameTop = 20 - 12 * 0.8605 - 1;
  return (
    <div
      class={`relative h-16 overflow-hidden rounded-md border ${
        selected ? "border-accent" : ""
      }`}
      style={`width:${width}px;background:${MOTION.fill};${selected ? "" : `border-color:${MOTION.border}`}`}
    >
      <MotionStrip width={width} />
      <div class="absolute inset-x-0 top-0 h-7" style="background:linear-gradient(to bottom,rgba(20,12,4,.82),rgba(20,12,4,0))" />
      <span class="absolute top-[7px] left-[7px]" style={`color:${MOTION.badge}`}>
        <MotionMark size={12} />
      </span>
      <span
        class="absolute left-[24px] text-xs leading-none font-semibold whitespace-nowrap text-text"
        style={`top:${nameTop}px`}
      >
        {name}
      </span>
    </div>
  );
}