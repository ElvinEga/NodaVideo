import type { Child, Markup } from "@design/jsx/jsx-runtime";
import { Icon } from "./icon.tsx";
import { Scene } from "./scene.tsx";

export type Workspace = "Edit" | "Color" | "Audio" | "AI Studio";
export type ControlState =
  | "Default"
  | "Hover"
  | "Pressed"
  | "Focused"
  | "Disabled";
export type BinState =
  | "grid"
  | "list"
  | "empty"
  | "filtered"
  | "importing"
  | "error"
  | "missing"
  | "drop";

const scene = "data:image/svg+xml," +
  encodeURIComponent(Scene({ width: 640 }).html);
const glyphs = {
  search: "M10 17a7 7 0 1 0 0-14 7 7 0 0 0 0 14m5-2 6 6",
  grid: "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z",
  list: "M3 5h2m4 0h12M3 12h2m4 0h12M3 19h2m4 0h12",
  import: "M12 16V3m-5 5 5-5 5 5M4 15v6h16v-6",
  undo: "M8 4 3 9l5 5M3 9h11a6 6 0 0 1 0 12",
  redo: "m16 4 5 5-5 5m5-5H10a6 6 0 0 0 0 12",
  media: "M3 3h18v18H3zM9 3v18",
  inspector: "M3 3h18v18H3zM15 3v18",
  assistant: "M4 3h16v14H9l-5 4V3zm4 5h8m-8 4h5",
  play: "m8 4 12 8-12 8V4z",
  previous: "M5 4v16M19 4 7 12l12 8V4z",
  next: "M19 4v16M5 4l12 8-12 8V4z",
  chevron: "m6 9 6 6 6-6",
  video: "M3 5h13v14H3zM16 10l5-4v12l-5-4",
  image: "M3 3h18v18H3zM3 17l6-7 5 5 3-3 4 5M16 7h.01",
  close: "m6 6 12 12M6 18 18 6",
  expand: "M3 9V3h6m6 0h6v6M3 15v6h6m6 0h6v-6",
  check: "m4 12 5 5L20 6",
  sliders: "M6 3v7m0 4v7M12 3v12m0 4v2M18 3v3m0 4v11M3 10h6m0 5h6m0-9h6",
} as const;

type Glyph = keyof typeof glyphs;

export function NodaGlyph(
  { name, size = 14 }: { name: Glyph; size?: number },
): Markup {
  return (
    <svg
      aria-hidden="true"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d={glyphs[name]} />
    </svg>
  );
}

export function NodaMark(): Markup {
  return (
    <svg
      aria-hidden="true"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
    >
      <path
        d="M4 19V5h4l8 14h4V5"
        class="stroke-accent"
        stroke-width="3"
        stroke-linejoin="round"
      />
      <path
        d="M4 19h4V9m8 6V5h4"
        class="stroke-accent-soft"
        stroke-width="2"
        stroke-linejoin="round"
      />
    </svg>
  );
}

export function NodaButton(
  { label, icon, primary = false, state = "Default", iconOnly = false }: {
    label: string;
    icon?: Glyph;
    primary?: boolean;
    state?: ControlState;
    iconOnly?: boolean;
  },
): Markup {
  const disabled = state === "Disabled";
  const colors = disabled
    ? "border-border bg-surface text-dim"
    : primary
    ? state === "Pressed"
      ? "border-accent bg-clip-video text-on-accent"
      : state === "Hover"
      ? "border-accent-soft bg-accent-hover text-on-accent"
      : "border-accent bg-accent-hover text-on-accent"
    : state === "Pressed"
    ? "border-accent bg-accent-background text-accent-soft"
    : state === "Hover"
    ? "border-rail bg-hover text-text"
    : "border-border bg-surface text-light";
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      disabled={disabled}
      class={`inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-[3px] border text-[11px] font-medium ${
        iconOnly ? "w-7" : "px-2.5"
      } ${colors} ${
        state === "Focused"
          ? "outline-2 outline-offset-2 outline-accent-soft"
          : ""
      }`}
    >
      {icon && <NodaGlyph name={icon} />}
      {!iconOnly && label}
    </button>
  );
}

export function ReviewPage(
  { title, subtitle, children }: {
    title: string;
    subtitle: string;
    children: Child;
  },
): Markup {
  return (
    <main class="mx-auto max-w-[1600px] bg-sheet p-6 font-sans text-ink sm:p-10">
      <div class="mb-7 flex flex-wrap items-end justify-between gap-4 border-b border-rule pb-6">
        <div>
          <p class="mb-2 text-[11px] font-semibold tracking-[0.14em] uppercase">
            NodaVideo / Design review
          </p>
          <h1 class="text-3xl font-semibold tracking-tight">{title}</h1>
          <p class="mt-2 max-w-[820px] text-sm leading-6 text-ink/70">
            {subtitle}
          </p>
        </div>
        <span class="border border-rule px-2 py-1 font-mono text-[10px] text-ink/70">
          STATIC PROPOSAL
        </span>
      </div>
      {children}
    </main>
  );
}

export function ReviewSection(
  { title, caption, children }: {
    title: string;
    caption: string;
    children: Child;
  },
): Markup {
  return (
    <section class="mt-9 border-t border-rule pt-6">
      <h2 class="text-lg font-semibold tracking-tight">{title}</h2>
      <p class="mt-1 mb-4 max-w-[1000px] text-sm leading-6 text-ink/70">
        {caption}
      </p>
      {children}
    </section>
  );
}

export function ReviewFigure(
  { title, caption, children }: {
    title: string;
    caption: string;
    children: Child;
  },
): Markup {
  return (
    <figure class="row-span-3 grid grid-rows-subgrid gap-2">
      <h3 class="text-sm font-semibold">{title}</h3>
      <figcaption class="text-xs leading-5 text-ink/70">{caption}</figcaption>
      <div class="min-w-0 overflow-auto">{children}</div>
    </figure>
  );
}

export function NodaTopBar(
  { workspace = "Edit", empty = false, assistant = false, hidden = false }: {
    workspace?: Workspace;
    empty?: boolean;
    assistant?: boolean;
    hidden?: boolean;
  },
): Markup {
  return (
    <header class="grid h-12 grid-cols-[1fr_auto_1fr] items-center gap-5 border-b border-border bg-page px-4 text-text">
      <div class="flex min-w-0 items-center gap-2.5">
        <NodaMark />
        <span class="text-[14px] font-semibold tracking-tight">NodaVideo</span>
        <span class="mx-1 h-4 border-r border-border" />
        <span class="truncate text-xs text-subtle">
          {empty ? "Untitled project" : "City after hours"}
        </span>
        {!empty && (
          <span
            class="size-1.5 shrink-0 rounded-full bg-accent-soft"
            title="Unsaved changes"
          />
        )}
      </div>
      <nav aria-label="Workspaces" class="flex h-full items-stretch gap-1">
        {(["Edit", "Color", "Audio", "AI Studio"] as Workspace[]).map((tab) => (
          <button
            type="button"
            aria-current={workspace === tab ? "page" : undefined}
            class={`border-b-2 px-4 text-[12px] font-medium ${
              workspace === tab
                ? "border-accent text-text"
                : "border-transparent text-muted"
            }`}
          >
            {tab}
          </button>
        ))}
      </nav>
      <div class="flex items-center justify-end gap-1.5">
        <NodaButton
          label="Undo · Cmd-Z"
          icon="undo"
          iconOnly
          state={empty ? "Disabled" : "Default"}
        />
        <NodaButton
          label="Redo · Cmd-Shift-Z"
          icon="redo"
          iconOnly
          state="Disabled"
        />
        <span class="mx-1 h-4 border-r border-border" />
        <NodaButton
          label="Media Browser · Cmd-1"
          icon="media"
          iconOnly
          state={hidden ? "Default" : "Pressed"}
        />
        <NodaButton
          label="Inspector · Cmd-2"
          icon="inspector"
          iconOnly
          state={hidden || assistant ? "Default" : "Pressed"}
        />
        <NodaButton
          label="AI Assistant · Cmd-Shift-A"
          icon="assistant"
          iconOnly
          state={assistant ? "Pressed" : "Default"}
        />
      </div>
    </header>
  );
}

export function NodaPanelHeader(
  { title, detail, children }: {
    title: string;
    detail?: string;
    children?: Child;
  },
): Markup {
  return (
    <div class="flex h-10 shrink-0 items-center justify-between gap-2 border-b border-border px-3">
      <div class="flex min-w-0 items-center gap-2">
        <h3 class="truncate text-[12px] font-medium text-text">{title}</h3>
        {detail && <span class="text-[10px] text-muted">{detail}</span>}
      </div>
      {children}
    </div>
  );
}

export function MediaBin({ state = "grid" }: { state?: BinState }): Markup {
  const empty = state === "empty" || state === "drop";
  return (
    <section
      aria-label="Media Browser"
      class="flex h-full min-h-[370px] min-w-0 flex-col bg-surface text-text"
    >
      <NodaPanelHeader title="Media Browser" detail={empty ? "0" : "6"}>
        <NodaButton label="Import Media" icon="import" primary />
      </NodaPanelHeader>
      <div class="flex items-center gap-2 px-3 pt-3">
        <div class="flex h-7 min-w-0 flex-1 items-center gap-2 rounded-[3px] border border-border bg-page px-2 text-muted">
          <NodaGlyph name="search" />
          <span class="truncate text-[11px]">
            {state === "filtered" ? "interview" : "Search media…"}
          </span>
        </div>
        <NodaButton
          label="Grid view"
          icon="grid"
          iconOnly
          state={state === "list" ? "Default" : "Pressed"}
        />
        <NodaButton
          label="List view"
          icon="list"
          iconOnly
          state={state === "list" ? "Pressed" : "Default"}
        />
      </div>
      <div class="flex gap-1 px-3 py-2.5">
        {["All", "Video", "Audio", "Graphics"].map((filter) => (
          <button
            type="button"
            aria-pressed={String(filter === "All")}
            class={`rounded-[3px] px-2.5 py-1 text-[10px] ${
              filter === "All" ? "bg-hover text-text" : "text-muted"
            }`}
          >
            {filter}
          </button>
        ))}
      </div>
      {empty
        ? (
          <div
            class={`m-3 mt-0 flex flex-1 flex-col items-center justify-center gap-3 rounded-[3px] border border-dashed px-5 py-8 text-center ${
              state === "drop"
                ? "border-accent bg-accent-background"
                : "border-rail"
            }`}
          >
            <span class="text-muted">
              <NodaGlyph name="import" size={24} />
            </span>
            <p class="text-[12px] font-medium">
              {state === "drop"
                ? "Release to import media"
                : "Your media starts here"}
            </p>
            <p class="max-w-[210px] text-[11px] leading-5 text-muted">
              Drop video, audio, or images here to import, or click Import
              Media.
            </p>
            {state !== "drop" && (
              <NodaButton label="Import Media" icon="import" primary />
            )}
          </div>
        )
        : state === "filtered"
        ? (
          <div class="flex flex-1 flex-col items-center justify-center gap-2 px-4 text-center">
            <NodaGlyph name="search" size={22} />
            <p class="text-xs">No media matches “interview”</p>
            <p class="text-[11px] text-muted">
              Try another name or clear the search.
            </p>
            <NodaButton label="Clear search" />
          </div>
        )
        : (
          <div
            class={`min-h-0 flex-1 overflow-y-auto px-3 pb-3 ${
              state === "list"
                ? ""
                : "grid auto-rows-max content-start grid-cols-2 gap-2"
            }`}
          >
            {media.map((asset, index) => (
              <MediaCard
                asset={asset}
                selected={index === 0}
                list={state === "list"}
                missing={state === "missing" && index === 1}
              />
            ))}
          </div>
        )}
      {state === "importing" && (
        <div class="mx-3 mb-3 text-[10px] text-subtle">
          <div class="mb-2 flex justify-between">
            <span>Importing 3 of 8 files…</span>
            <span>38%</span>
          </div>
          <div class="h-1 bg-border">
            <div class="h-1 w-[38%] bg-accent" />
          </div>
        </div>
      )}
      {state === "error" && (
        <div class="mx-3 mb-3 rounded-[3px] border border-error/40 bg-error-background p-2.5 text-[11px] leading-5">
          <span class="text-error-text">Couldn’t import damaged.mov</span>
          <p class="text-subtle">
            No readable video stream. Choose another file.
          </p>
        </div>
      )}
      <footer class="flex h-7 shrink-0 items-center justify-between border-t border-border px-3 text-[10px] text-muted">
        <span>
          {empty
            ? "No media imported"
            : state === "filtered"
            ? "0 of 6 assets"
            : "6 assets · 1 selected"}
        </span>
        <span>Referenced, not copied</span>
      </footer>
    </section>
  );
}

const media = [
  {
    name: "street-at-dusk.mov",
    kind: "video",
    duration: "00:18:12",
    detail: "3840 × 2160",
    audio: true,
  },
  {
    name: "market-walk.mp4",
    kind: "video",
    duration: "00:24:08",
    detail: "1920 × 1080",
    audio: true,
  },
  {
    name: "station-arrival.mov",
    kind: "video",
    duration: "00:12:16",
    detail: "3840 × 2160",
    audio: false,
  },
  {
    name: "city-ambience.wav",
    kind: "audio",
    duration: "01:08:00",
    detail: "48 kHz · Stereo",
    audio: true,
  },
  {
    name: "evening-score.wav",
    kind: "audio",
    duration: "02:32:00",
    detail: "48 kHz · Stereo",
    audio: true,
  },
  {
    name: "end-title.png",
    kind: "image",
    duration: "PNG",
    detail: "1920 × 1080",
    audio: false,
  },
] as const;

function MediaCard(
  { asset, selected, list, missing }: {
    asset: typeof media[number];
    selected: boolean;
    list: boolean;
    missing: boolean;
  },
): Markup {
  return (
    <div
      class={`min-w-0 overflow-hidden rounded-[3px] border ${
        selected
          ? "border-accent bg-accent-background"
          : "border-border bg-panel"
      } ${list ? "mb-1 flex items-center gap-2 p-1" : ""}`}
    >
      <div
        class={`relative flex shrink-0 items-center justify-center overflow-hidden bg-page ${
          list ? "h-9 w-14" : "aspect-video"
        }`}
      >
        {missing
          ? (
            <span class="text-error-text">
              <Icon name="warning" width={20} height={20} />
            </span>
          )
          : asset.kind === "video"
          ? <img src={scene} alt="" class="size-full object-cover" />
          : asset.kind === "audio"
          ? (
            <div class="w-full px-2 text-audio-badge-text">
              <Waveform />
            </div>
          )
          : (
            <div class="flex flex-col items-center gap-1 text-muted">
              <NodaGlyph name="image" size={18} />
              {!list && (
                <span class="text-[9px] tracking-[0.2em]">
                  CITY AFTER HOURS
                </span>
              )}
            </div>
          )}
        {!list && (
          <span class="absolute right-1 bottom-1 rounded-[2px] bg-monitor/90 px-1 py-0.5 font-mono text-[9px]">
            {asset.duration}
          </span>
        )}
      </div>
      <div class={`min-w-0 flex-1 ${list ? "" : "p-2"}`}>
        <div class="flex min-w-0 items-center gap-1.5">
          <span class={missing ? "text-error-text" : "text-muted"}>
            <NodaGlyph
              name={asset.kind === "audio" ? "sliders" : asset.kind}
              size={11}
            />
          </span>
          <span class="truncate text-[10px]">{asset.name}</span>
        </div>
        <div class="mt-1 flex items-center justify-between text-[9px] text-muted">
          <span>{missing ? "Source missing" : asset.detail}</span>
          {asset.audio && !missing && (
            <Icon name="unmute" width={10} height={10} />
          )}
        </div>
      </div>
      {list && (
        <span class="pr-2 font-mono text-[10px] text-subtle">
          {asset.duration}
        </span>
      )}
    </div>
  );
}

export function Waveform(): Markup {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 320 30"
      preserveAspectRatio="none"
      class="h-7 w-full"
    >
      {Array.from({ length: 80 }, (_, index) => {
        const height = 3 +
          Math.abs(Math.sin(index * 1.7) * Math.cos(index * 0.21)) * 24;
        return (
          <rect
            x={index * 4}
            y={(30 - height) / 2}
            width="2"
            height={height}
            fill="currentColor"
            opacity="0.65"
          />
        );
      })}
    </svg>
  );
}

export function PreviewMonitor(
  { source = false, empty = false }: { source?: boolean; empty?: boolean },
): Markup {
  return (
    <section
      aria-label={source ? "Source monitor" : "Timeline monitor"}
      class="flex h-full min-h-[370px] min-w-0 flex-col bg-panel text-text"
    >
      <NodaPanelHeader
        title={source ? "Source" : "Timeline"}
        detail={empty
          ? undefined
          : source
          ? "street-at-dusk.mov"
          : "Sequence 01"}
      >
        <div class="flex items-center gap-2">
          <span class="text-[10px] text-muted">Fit</span>
          <NodaGlyph name="chevron" size={11} />
          <NodaButton label="Fullscreen · Cmd-F" icon="expand" iconOnly />
        </div>
      </NodaPanelHeader>
      <div class="flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-monitor p-5">
        {empty
          ? (
            <div class="flex flex-col items-center gap-3 text-center">
              <span class="text-dim">
                <NodaGlyph name="video" size={30} />
              </span>
              <p class="text-xs text-subtle">Nothing to preview yet</p>
              <p class="text-[11px] text-muted">
                Add media to your timeline to begin.
              </p>
            </div>
          )
          : (
            <img
              src={scene}
              alt="Illustrative dusk street frame"
              class="max-h-full max-w-full object-contain"
            />
          )}
      </div>
      {source && <SourceRange />}
      <div class="grid h-11 shrink-0 grid-cols-[1fr_auto_1fr] items-center border-t border-border px-3">
        <span class="font-mono text-[11px] text-accent-soft">
          {empty ? "00:00:00:00" : "00:00:06:12"}
        </span>
        <div class="flex items-center gap-1">
          <NodaButton
            label="Previous frame"
            icon="previous"
            iconOnly
            state={empty ? "Disabled" : "Default"}
          />
          <NodaButton
            label="Play · Space"
            icon="play"
            iconOnly
            state={empty ? "Disabled" : "Default"}
          />
          <NodaButton
            label="Next frame"
            icon="next"
            iconOnly
            state={empty ? "Disabled" : "Default"}
          />
        </div>
        <span class="text-right font-mono text-[10px] text-muted">
          {empty ? "24 fps" : source ? "00:00:18:12" : "00:00:42:00"}
        </span>
      </div>
      {source && (
        <div class="flex h-11 shrink-0 items-center justify-between border-t border-border px-3">
          <span class="text-[10px] text-muted">Range · 00:00:08:00</span>
          <NodaButton label="Insert to Timeline" primary />
        </div>
      )}
    </section>
  );
}

function SourceRange(): Markup {
  return (
    <div class="border-t border-border px-4 pt-3 pb-2">
      <div class="relative h-6 rounded-[2px] bg-page">
        <div class="absolute inset-y-0 left-[22%] w-[44%] border-x-2 border-accent-soft bg-accent/20">
          <span class="absolute left-0 top-0 h-full w-1 bg-accent" />
          <span class="absolute right-0 top-0 h-full w-1 bg-accent" />
        </div>
        <span class="absolute inset-y-0 left-[35%] w-px bg-text" />
      </div>
      <div class="mt-2 flex justify-between font-mono text-[10px] text-subtle">
        <span>
          <kbd class="mr-1 text-accent-soft">I</kbd> 00:00:04:00
        </span>
        <span>
          <kbd class="mr-1 text-accent-soft">O</kbd> 00:00:12:00
        </span>
      </div>
    </div>
  );
}

export function Inspector({ empty = false }: { empty?: boolean }): Markup {
  return (
    <section
      aria-label="Inspector"
      class="flex h-full min-h-[370px] flex-col overflow-y-auto bg-surface text-text"
    >
      <NodaPanelHeader title="Inspector">
        <NodaButton label="Hide Inspector" icon="close" iconOnly />
      </NodaPanelHeader>
      {empty
        ? (
          <div class="flex flex-1 flex-col items-center justify-center gap-3 px-5 text-center text-muted">
            <NodaGlyph name="sliders" size={22} />
            <p class="text-xs text-subtle">No clip selected</p>
            <p class="text-[11px] leading-5">
              Select a timeline clip to adjust its properties.
            </p>
          </div>
        )
        : (
          <>
            <div class="border-b border-border px-3 py-3">
              <p class="truncate text-xs">street-at-dusk.mov</p>
              <p class="mt-1 text-[10px] text-muted">
                Video · 3840 × 2160 · 24 fps
              </p>
            </div>
            <div class="flex gap-5 border-b border-border px-3 text-[11px]">
              <span class="border-b-2 border-accent py-2">Video</span>
              <span class="py-2 text-muted">Audio</span>
            </div>
            <div class="space-y-3 px-3 py-3">
              <div class="flex items-center justify-between">
                <h4 class="text-[11px] font-medium">Transform</h4>
                <span class="text-[10px] text-muted">Reset</span>
              </div>
              <PropertyRow label="Position" values={["X  0", "Y  0"]} />
              <PropertyRow label="Scale" values={["100.0 %"]} />
              <PropertyRow label="Rotation" values={["0.0°"]} />
              <PropertyRow label="Opacity" values={["100 %"]} />
            </div>
            <div class="space-y-3 border-t border-border px-3 py-3">
              <h4 class="text-[11px] font-medium">Audio</h4>
              <PropertyRow label="Gain" values={["0.0 dB"]} />
              <PropertyRow label="Pan" values={["Center"]} />
            </div>
          </>
        )}
    </section>
  );
}

function PropertyRow(
  { label, values }: { label: string; values: string[] },
): Markup {
  return (
    <div class="flex items-center gap-2 text-[10px]">
      <span class="w-14 shrink-0 text-muted">{label}</span>
      <div class="flex min-w-0 flex-1 gap-1.5">
        {values.map((value) => (
          <span class="flex h-6 min-w-0 flex-1 items-center justify-center rounded-[3px] border border-border bg-page font-mono text-subtle">
            {value}
          </span>
        ))}
      </div>
    </div>
  );
}

export function AssistantPanel(
  { approval = false }: { approval?: boolean },
): Markup {
  return (
    <section
      aria-label="AI Assistant"
      class="flex h-full min-h-[370px] flex-col overflow-y-auto bg-surface text-text"
    >
      <NodaPanelHeader title="AI Assistant">
        <NodaButton
          label="Hide AI Assistant · Cmd-Shift-A"
          icon="close"
          iconOnly
        />
      </NodaPanelHeader>
      <div class="flex h-10 items-center justify-between border-b border-border px-3 text-[11px]">
        <span class="text-muted">Provider</span>
        <span class="flex items-center gap-2">
          Claude Code<NodaGlyph name="chevron" size={12} />
        </span>
      </div>
      {approval
        ? (
          <div class="flex-1 space-y-4 p-3 text-[11px] leading-5">
            <div class="rounded-[3px] bg-hover p-2.5">
              Trim the opening clip to 8 seconds.
            </div>
            <div>
              <p class="mb-1 text-muted">Assistant</p>
              <p>
                I’ll shorten the opening clip. The rest of the timeline stays in
                place.
              </p>
            </div>
            <div class="border-l-2 border-accent bg-page p-3">
              <p class="font-medium">Review timeline change</p>
              <p class="mt-1 text-muted">Trim · street-at-dusk.mov</p>
              <p class="font-mono text-[10px] text-subtle">
                00:00:18:12 → 00:00:08:00
              </p>
              <div class="mt-3 flex gap-2">
                <NodaButton label="Approve" primary />
                <NodaButton label="Reject" />
              </div>
            </div>
            <p class="text-[10px] text-muted">
              Thoughts and tool activity <span class="float-right">▸</span>
            </p>
          </div>
        )
        : (
          <div class="flex flex-1 flex-col items-center justify-center gap-3 px-5 text-center">
            <span class="text-accent-soft">
              <NodaGlyph name="assistant" size={25} />
            </span>
            <p class="text-xs">What are you making?</p>
            <p class="max-w-[240px] text-[11px] leading-5 text-muted">
              Ask for help with cuts, captions, or organizing your edit.
            </p>
          </div>
        )}
      <div class="m-3 rounded-[3px] border border-border bg-page p-2.5">
        <p class="mb-4 text-[11px] text-muted">Ask about your edit…</p>
        <div class="flex justify-between text-[9px] text-muted">
          <span>Timeline changes need approval</span>
          <NodaGlyph name="import" size={13} />
        </div>
      </div>
    </section>
  );
}

export function TimelinePanel(
  { empty = false, drop }: { empty?: boolean; drop?: "valid" | "invalid" },
): Markup {
  return (
    <section
      aria-label="Timeline"
      class="border-t border-border bg-page text-text"
    >
      <div class="flex h-10 items-center gap-3 border-b border-border px-3">
        <span class="text-[11px] font-medium">Sequence 01</span>
        <span class="text-[10px] text-muted">24 fps</span>
        <span class="mx-1 h-4 border-r border-border" />
        <Icon name="pointer" />
        <Icon name="scissors" />
        <span class="rounded-[3px] bg-accent-background p-1.5 text-accent-soft">
          <Icon name="snap" />
        </span>
        <Icon name="magnet" color="currentColor" />
        <span class="ml-auto font-mono text-[11px] text-accent-soft">
          {empty ? "00:00:00:00" : "00:00:06:12"}
        </span>
        <span class="mx-2 h-4 border-r border-border" />
        <span class="text-[10px] text-muted">−</span>
        <div class="h-0.5 w-16 bg-rail">
          <div class="-mt-1 ml-6 size-2.5 rounded-full bg-subtle" />
        </div>
        <span class="text-[10px] text-muted">+</span>
      </div>
      <div class="grid grid-cols-[148px_1fr]">
        <div class="flex h-7 items-center border-r border-b border-border px-3 text-[9px] text-muted">
          TRACKS
        </div>
        <div class="flex h-7 items-center justify-between border-b border-border px-3 font-mono text-[9px] text-muted">
          {[
            "00:00",
            "00:05",
            "00:10",
            "00:15",
            "00:20",
            "00:25",
            "00:30",
            "00:35",
            "00:40",
          ].map((time) => <span key={time}>{time}</span>)}
        </div>
      </div>
      {empty
        ? (
          <div class="flex h-[152px] flex-col items-center justify-center gap-2">
            <p class="text-xs text-subtle">Make your first cut</p>
            <p class="text-[11px] text-muted">
              Drag media here to create a track and start your timeline.
            </p>
          </div>
        )
        : (
          <div class="relative">
            <TimelineLane kind="T1" name="Titles">
              <div class="absolute inset-y-1 left-[5%] w-[16%] truncate rounded-[3px] border border-image-badge-border bg-clip-title px-2 pt-2 text-[10px]">
                City after hours
              </div>
            </TimelineLane>
            <TimelineLane kind="V1" name="Video">
              <div class="absolute inset-y-1 left-0 w-[30%] rounded-[3px] border border-accent bg-clip-video px-2 pt-1.5 text-[10px]">
                street-at-dusk.mov<div class="mt-2 h-2 border-t border-accent-soft/20" />
              </div>
              <div class="absolute inset-y-1 left-[30%] w-[24%] rounded-[3px] border border-video-badge-border bg-video-badge-border/40 px-2 pt-1.5 text-[10px]">
                market-walk.mp4
              </div>
              {drop === "valid" && <DropGhost valid />}
            </TimelineLane>
            <TimelineLane kind="A1" name="Audio">
              <div class="absolute inset-y-1 left-0 w-[83%] overflow-hidden rounded-[3px] border border-audio-badge-border bg-clip-audio px-2 text-[9px] text-audio-badge-text">
                <span>city-ambience.wav</span>
                <Waveform />
              </div>
              {drop === "invalid" && <DropGhost valid={false} />}
            </TimelineLane>
            <div class="pointer-events-none absolute inset-y-0 left-[calc(148px_+_(100%_-_148px)*0.16)] w-px bg-text">
              <div class="-ml-1 h-2 w-2 bg-text" />
            </div>
          </div>
        )}
      <div class="flex h-6 items-center justify-between border-t border-border px-3 text-[9px] text-muted">
        <span>
          {drop
            ? drop === "valid"
              ? "Snapped to clip edge · Release to place"
              : "Video needs a video track · No edit on release"
            : empty
            ? "No clips"
            : "3 tracks · 4 clips"}
        </span>
        <span>{empty ? "00:00:00:00" : "Duration 00:00:42:00"}</span>
      </div>
    </section>
  );
}

function TimelineLane(
  { kind, name, children }: { kind: string; name: string; children: Child },
): Markup {
  return (
    <div class="grid h-[51px] grid-cols-[148px_1fr] border-b border-border/70">
      <div class="flex items-center gap-2 border-r border-border bg-panel px-3">
        <span class="rounded-[2px] bg-hover px-1.5 py-1 font-mono text-[9px] text-subtle">
          {kind}
        </span>
        <span class="mr-auto text-[10px] text-subtle">{name}</span>
        <Icon name="lock" width={11} height={11} color="currentColor" />
        <Icon
          name={kind === "A1" ? "unmute" : "eye"}
          width={11}
          height={11}
          color="currentColor"
        />
      </div>
      <div class="relative min-w-0">{children}</div>
    </div>
  );
}

function DropGhost({ valid }: { valid: boolean }): Markup {
  return (
    <div
      class={`absolute inset-y-0 left-[54%] z-10 w-[31%] border-l ${
        valid ? "border-accent-soft" : "border-transparent"
      }`}
    >
      <div
        class={`absolute inset-y-1 left-0 right-1 rounded-[3px] border px-2 pt-1.5 text-[10px] ${
          valid
            ? "border-accent bg-accent/20 text-text"
            : "hatch-rejected border-error bg-error-background text-error-text"
        }`}
      >
        <p>station-arrival.mov</p>
        <p class="mt-1 text-[9px]">
          {valid
            ? "Release to place · Video"
            : "Incompatible track · Video → Audio"}
        </p>
      </div>
    </div>
  );
}

export function WorkspacePlaceholder(
  { workspace }: { workspace: "Color" | "Audio" },
): Markup {
  return (
    <section class="flex h-full flex-col bg-panel text-text">
      <NodaPanelHeader
        title={workspace === "Color" ? "Color grading" : "Audio mixer"}
      />
      <div class="flex flex-1 flex-col items-center justify-center gap-3 px-5 text-center">
        <span class="text-muted">
          <NodaGlyph name="sliders" size={26} />
        </span>
        <p class="text-[13px] font-medium">
          {workspace === "Color"
            ? "A dedicated space for your look"
            : "A dedicated space for your mix"}
        </p>
        <p class="max-w-[280px] text-[11px] leading-5 text-muted">
          {workspace === "Color"
            ? "Color grading tools are coming in a later milestone."
            : "Mixer controls are coming in a later milestone."}
        </p>
        <p class="text-[10px] text-muted">
          Your timeline remains available below.
        </p>
      </div>
    </section>
  );
}

export function NodaShell({
  workspace = "Edit",
  empty = false,
  source = false,
  assistant = false,
  hidden = false,
}: {
  workspace?: Workspace;
  empty?: boolean;
  source?: boolean;
  assistant?: boolean;
  hidden?: boolean;
}): Markup {
  const showAssistant = assistant || workspace === "AI Studio";
  return (
    <div class="min-w-[1080px] overflow-hidden rounded-[5px] border border-border bg-page font-sans text-text">
      <div class="flex h-7 items-center gap-5 border-b border-border bg-surface px-4 text-[10px]">
        <span class="font-semibold">NodaVideo</span>
        {["File", "Edit", "View", "Playback", "Window", "Help"].map((menu) => (
          <span>{menu}</span>
        ))}
      </div>
      <NodaTopBar
        workspace={workspace}
        empty={empty}
        assistant={showAssistant}
        hidden={hidden}
      />
      <div
        class={`grid h-[410px] gap-px bg-border ${
          hidden ? "grid-cols-1" : "grid-cols-[280px_minmax(320px,1fr)_252px]"
        }`}
      >
        {!hidden && <MediaBin state={empty ? "empty" : "grid"} />}
        {workspace === "Color" || workspace === "Audio"
          ? <WorkspacePlaceholder workspace={workspace} />
          : <PreviewMonitor source={source} empty={empty} />}
        {!hidden &&
          (showAssistant ? <AssistantPanel /> : <Inspector empty={empty} />)}
      </div>
      <TimelinePanel empty={empty} />
    </div>
  );
}
