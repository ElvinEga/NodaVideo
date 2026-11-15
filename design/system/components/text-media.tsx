import type { Markup } from "@design/jsx/jsx-runtime";
import { FONT_SANS } from "../theme.ts";
import { NodaButton, NodaGlyph, NodaPanelHeader } from "./noda-shell.tsx";

/// A reusable title kept in the project, not an imported file. It carries the same
/// properties as a timeline text clip; placing it copies these into a new clip.
export type TextTitle = {
  name: string;
  text: string;
  color: string;
  font: string;
  duration: string;
  background?: { hex: string; opacity: number };
};

/// The title created by New Text: the model defaults (text "Text", white, Sans).
export const NEW_TITLE: TextTitle = {
  name: "Text",
  text: "Text",
  color: "#ffffff",
  font: "Sans",
  duration: "00:03.0",
};

/// Illustrative titles a user might create. Design fixtures, not application data.
export const TITLE_FIXTURES: TextTitle[] = [
  {
    name: "Lower third",
    text: "Amara Okafor",
    color: "#ffffff",
    font: "Sans",
    duration: "00:04.0",
    background: { hex: "6366f1", opacity: 80 },
  },
  {
    name: "End title",
    text: "City after hours",
    color: "#f1f3f9",
    font: "Sans",
    duration: "00:03.0",
  },
  {
    name: "Section header",
    text: "Night market",
    color: "#a5b4fc",
    font: "Sans",
    duration: "00:02.0",
  },
  {
    name: "Caption",
    text: "Let's get started",
    color: "#ffffff",
    font: "Sans",
    duration: "00:03.0",
  },
];

function rgba(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  return `rgba(${r},${g},${b},${alpha})`;
}

/// Renders a title's text, centered, at a size derived from the preview height. The
/// container fills its parent and stays transparent so it composes over the monitor
/// black or a card well. No decode is involved: the text is drawn directly.
export function TextThumb(
  { title, size, class: className }: {
    title: TextTitle;
    size: number;
    class?: string;
  },
): Markup {
  const fontSize = Math.max(10, Math.round(size * 0.18));
  const background = title.background
    ? rgba(title.background.hex, title.background.opacity / 100)
    : undefined;
  const box = background
    ? `background:${background};padding:${Math.round(fontSize * 0.25)}px ${
      Math.round(fontSize * 0.5)
    }px;border-radius:3px;`
    : "";
  return (
    <div
      class={`flex size-full items-center justify-center overflow-hidden px-2 text-center ${
        className ?? ""
      }`}
    >
      <span
        class="leading-tight font-semibold whitespace-pre-wrap"
        style={`font-family:${FONT_SANS};font-size:${fontSize}px;color:${title.color};${box}`}
      >
        {title.text}
      </span>
    </div>
  );
}

/// A title card in the Media Pool: a rendered preview, the T badge, name, font and
/// duration. Grid and list share one shape, differing only in layout.
function TextCard(
  { title, selected, list }: {
    title: TextTitle;
    selected: boolean;
    list: boolean;
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
        class={`relative flex shrink-0 items-center justify-center overflow-hidden bg-monitor ${
          list ? "h-9 w-14" : "aspect-video w-full"
        }`}
      >
        <TextThumb title={title} size={list ? 36 : 84} />
        {!list && (
          <span class="absolute top-1 left-1 grid size-4 place-items-center rounded-[2px] bg-clip-title text-[9px] font-bold text-[#e3d4ff]">
            T
          </span>
        )}
        {!list && (
          <span class="absolute right-1 bottom-1 rounded-[2px] bg-monitor/90 px-1 py-0.5 font-mono text-[9px]">
            {title.duration}
          </span>
        )}
      </div>
      <div class={`min-w-0 flex-1 ${list ? "" : "p-2"}`}>
        <span class="block truncate text-[10px]">{title.name}</span>
        <div class="mt-1 flex items-center justify-between text-[9px] text-muted">
          <span>Text · {title.font}</span>
          <span
            class="size-2.5 rounded-[2px] border border-border"
            style={`background:${title.color}`}
            title="Font color"
          />
        </div>
      </div>
      {list && (
        <span class="pr-2 font-mono text-[10px] text-subtle">
          {title.duration}
        </span>
      )}
    </div>
  );
}

/// The Media Browser filtered to the Text category. New Text creates a project-local
/// title; the bin lists titles, not files. With `empty`, it invites the first title
/// rather than showing sample assets.
export function TextMediaBin(
  { titles, view = "grid", selected = 0, empty = false }: {
    titles: TextTitle[];
    view?: "grid" | "list";
    selected?: number;
    empty?: boolean;
  },
): Markup {
  const count = empty ? 0 : titles.length;
  return (
    <section
      aria-label="Media Browser"
      class="flex h-full min-h-[370px] min-w-0 flex-col bg-surface text-text"
    >
      <NodaPanelHeader title="Media Browser" detail={String(count)}>
        <NodaButton label="New Text" primary />
      </NodaPanelHeader>
      <div class="flex items-center gap-2 px-3 pt-3">
        <div class="flex h-7 min-w-0 flex-1 items-center gap-2 rounded-[3px] border border-border bg-page px-2 text-muted">
          <NodaGlyph name="search" />
          <span class="truncate text-[11px]">Search media…</span>
        </div>
        <NodaButton
          label="Grid view"
          icon="grid"
          iconOnly
          state={view === "grid" ? "Pressed" : "Default"}
        />
        <NodaButton
          label="List view"
          icon="list"
          iconOnly
          state={view === "list" ? "Pressed" : "Default"}
        />
      </div>
      <div class="flex gap-1 px-3 py-2.5">
        {["All", "Video", "Audio", "Graphics", "Text"].map((filter) => (
          <button
            type="button"
            aria-pressed={String(filter === "Text")}
            class={`rounded-[3px] px-2.5 py-1 text-[10px] ${
              filter === "Text" ? "bg-hover text-text" : "text-muted"
            }`}
          >
            {filter}
          </button>
        ))}
      </div>
      {empty
        ? (
          <div class="m-3 mt-0 flex flex-1 flex-col items-center justify-center gap-3 rounded-[3px] border border-dashed border-rail px-5 py-8 text-center">
            <span class="grid size-9 place-items-center rounded-[6px] bg-clip-title text-[15px] font-bold text-[#e3d4ff]">
              T
            </span>
            <p class="text-[12px] font-medium">Create your first title</p>
            <p class="max-w-[210px] text-[11px] leading-5 text-muted">
              Titles are made in your project, not imported. Add one, then drag it
              to a Text track.
            </p>
            <NodaButton label="New Text" primary />
          </div>
        )
        : (
          <div
            class={`min-h-0 flex-1 overflow-y-auto px-3 pb-3 ${
              view === "list"
                ? ""
                : "grid auto-rows-max content-start grid-cols-2 gap-2"
            }`}
          >
            {titles.map((title, index) => (
              <TextCard
                title={title}
                selected={index === selected}
                list={view === "list"}
              />
            ))}
          </div>
        )}
      <footer class="flex h-7 shrink-0 items-center justify-between border-t border-border px-3 text-[10px] text-muted">
        <span>
          {empty ? "No titles yet" : `${count} titles · 1 selected`}
        </span>
        <span>Project-local</span>
      </footer>
    </section>
  );
}

/// The source monitor showing a title: the text rendered over black at its font,
/// color, background and position, with Add to Timeline. A title is a still, so the
/// transport reads its duration rather than a source range.
export function TextSourceMonitor({ title }: { title: TextTitle }): Markup {
  return (
    <section
      aria-label="Source monitor"
      class="flex h-full min-h-[370px] min-w-0 flex-col bg-panel text-text"
    >
      <NodaPanelHeader title="Source" detail={title.name}>
        <div class="flex items-center gap-2">
          <span class="text-[10px] text-muted">Fit</span>
          <NodaGlyph name="chevron" size={11} />
          <NodaButton label="Fullscreen · Cmd-F" icon="expand" iconOnly />
        </div>
      </NodaPanelHeader>
      <div class="flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-monitor p-5">
        <div class="relative aspect-video w-full max-w-[440px] overflow-hidden rounded-[2px]">
          <TextThumb title={title} size={248} />
        </div>
      </div>
      <div class="grid h-11 shrink-0 grid-cols-[1fr_auto_1fr] items-center border-t border-border px-3">
        <span class="font-mono text-[11px] text-accent-soft">00:00:00:00</span>
        <div class="flex items-center gap-1">
          <NodaButton label="Previous frame" icon="previous" iconOnly />
          <NodaButton label="Play · Space" icon="play" iconOnly />
          <NodaButton label="Next frame" icon="next" iconOnly />
        </div>
        <span class="text-right font-mono text-[10px] text-muted">
          {title.duration}
        </span>
      </div>
      <div class="flex h-11 shrink-0 items-center justify-between border-t border-border px-3">
        <span class="text-[10px] text-muted">Text · {title.font}</span>
        <NodaButton label="Add to Timeline" primary />
      </div>
    </section>
  );
}