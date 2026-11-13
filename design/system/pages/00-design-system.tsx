import type { Markup } from "@design/jsx/jsx-runtime";
import { COLOR, FONT_MONO, FONT_SANS } from "../theme.ts";
import {
  type ControlState,
  NodaButton,
  NodaGlyph,
  NodaMark,
  ReviewPage,
  ReviewSection,
} from "../components/noda-shell.tsx";

export default function NodaDesignSystem(): Markup {
  return (
    <ReviewPage
      title="Noda Obsidian"
      subtitle="The shared visual language for NodaVideo: compact native controls, crisp neutral separators, and one focused indigo accent. No neon glow, cut-corner buttons, gradients, or decorative chrome."
    >
      <div class="grid gap-6 lg:grid-cols-[1fr_2fr]">
        <div class="flex flex-col justify-between gap-8 rounded-[4px] border border-border bg-page p-7 text-text">
          <div class="flex items-center gap-3">
            <NodaMark />
            <span class="text-2xl font-semibold tracking-tight">NodaVideo</span>
          </div>
          <p class="text-sm text-subtle">
            Your timeline.<br />Your creativity.<br />Your AI.
          </p>
          <p class="font-mono text-[11px] text-muted">
            00:00:06:12 <span class="ml-3 text-accent-soft">24 fps</span>
          </p>
        </div>
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {swatches.map(([label, color]) => (
            <div>
              <div
                class="h-16 rounded-[3px] border border-ink/10"
                style={`background:${color}`}
              />
              <p class="mt-2 text-xs font-medium">{label}</p>
              <p class="mt-1 font-mono text-[10px] text-ink/65">{color}</p>
            </div>
          ))}
        </div>
      </div>
      <ReviewSection
        title="Controls"
        caption="One 28 px shape, a 3 px corner radius, and a 1 px border. States change fill and border, never dimensions. The deeper indigo action fill keeps small white labels legible; the brand accent remains #6366f1."
      >
        <div class="overflow-auto rounded-[4px] border border-border bg-page p-5 text-text">
          <div class="grid min-w-[860px] grid-cols-[110px_repeat(5,1fr)] items-center gap-x-6 gap-y-6">
            <span />
            {states.map((state) => (
              <span class="text-[11px] text-muted">{state}</span>
            ))}
            {(["Primary", "Secondary", "Icon"] as const).map((kind) => (
              <>
                <span class="text-xs text-subtle">{kind}</span>
                {states.map((state) => (
                  <div>
                    <NodaButton
                      label={kind === "Primary"
                        ? "Import Media"
                        : kind === "Icon"
                        ? "Media Browser"
                        : "Cancel"}
                      icon={kind === "Primary"
                        ? "import"
                        : kind === "Icon"
                        ? "media"
                        : undefined}
                      primary={kind === "Primary"}
                      iconOnly={kind === "Icon"}
                      state={state}
                    />
                  </div>
                ))}
              </>
            ))}
          </div>
        </div>
      </ReviewSection>
      <ReviewSection
        title="Fields and selection"
        caption="Search and numeric controls use the same matte inset surface. Focus uses indigo; selected text uses a translucent indigo fill. No animation is needed to communicate either state."
      >
        <div class="overflow-auto rounded-[4px] border border-border bg-page p-5">
          <div class="grid min-w-[820px] grid-cols-5 gap-5">
            {["Placeholder", "Entered", "Focused", "Selected", "Disabled"].map((
              state,
            ) => (
              <div>
                <p class="mb-3 text-[11px] text-muted">{state}</p>
                <div
                  class={`flex h-7 items-center gap-2 rounded-[3px] border bg-page px-2 text-[11px] ${
                    state === "Focused"
                      ? "border-accent outline-1 outline-accent"
                      : "border-border"
                  } ${state === "Disabled" ? "text-dim" : "text-muted"}`}
                >
                  <NodaGlyph name="search" />
                  <span
                    class={state === "Selected"
                      ? "bg-accent-background text-text"
                      : state === "Entered" || state === "Focused"
                      ? "text-text"
                      : ""}
                  >
                    {state === "Placeholder" || state === "Disabled"
                      ? "Search media…"
                      : "street"}
                  </span>
                  {state === "Focused" && <span class="h-3 w-px bg-text" />}
                </div>
              </div>
            ))}
          </div>
        </div>
      </ReviewSection>
      <ReviewSection
        title="Type and density"
        caption="Use the operating-system sans face for the interface and monospace only for frame positions, durations, and numeric values. Branding is a compact wordmark, not a display headline inside the editor."
      >
        <div class="grid gap-6 md:grid-cols-2">
          <div class="rounded-[4px] border border-border bg-surface p-6 text-text">
            <p class="text-sm font-semibold">NodaVideo</p>
            <p class="mt-5 text-xs font-medium">Media Browser</p>
            <p class="mt-3 text-[11px] text-muted">
              Drop video, audio, or images here to import.
            </p>
            <p class="mt-5 font-mono text-[11px] text-accent-soft">
              00:00:06:12
            </p>
          </div>
          <dl class="grid min-w-0 grid-cols-[120px_minmax(0,1fr)] gap-3 text-xs leading-5">
            <dt class="font-medium">Toolbar</dt>
            <dd>48 px; workspace tabs remain centered.</dd>
            <dt class="font-medium">Panel header</dt>
            <dd>40 px; 12 px title, 10 px metadata.</dd>
            <dt class="font-medium">Controls</dt>
            <dd>28 px; icon controls are square.</dd>
            <dt class="font-medium">Panel gutters</dt>
            <dd>1 px separators; resizable hit targets remain wider.</dd>
            <dt class="font-medium">Corners</dt>
            <dd>3 px controls, 5 px outer window.</dd>
            <dt class="font-medium">UI font</dt>
            <dd class="[overflow-wrap:anywhere]">{FONT_SANS}</dd>
            <dt class="font-medium">Time font</dt>
            <dd class="[overflow-wrap:anywhere]">{FONT_MONO}</dd>
          </dl>
        </div>
      </ReviewSection>
      <ReviewSection
        title="Semantic feedback"
        caption="Status never relies on hue alone. Valid drops combine a ghost with placement text; blocked drops have a reason and hatching. Track fills are deliberately subdued to leave selection visible."
      >
        <div class="grid gap-3 md:grid-cols-3">
          <div class="rounded-[3px] border border-border bg-surface p-4 text-text">
            <div class="flex items-center gap-2 text-xs">
              <span class="text-success">
                <NodaGlyph name="check" />
              </span>Import complete
            </div>
            <p class="mt-2 text-[11px] text-muted">All 6 files are ready.</p>
          </div>
          <div class="rounded-[3px] border border-border bg-surface p-4 text-text">
            <div class="flex items-center gap-2 text-xs">
              <span class="text-warning">!</span>Source missing
            </div>
            <p class="mt-2 text-[11px] text-muted">
              Restore the file to use this asset.
            </p>
          </div>
          <div class="rounded-[3px] border border-error/50 bg-error-background p-4 text-error-text">
            <div class="flex items-center gap-2 text-xs">
              <NodaGlyph name="close" />Incompatible track
            </div>
            <p class="mt-2 text-[11px]">Video needs a video track.</p>
          </div>
        </div>
        <div class="mt-4 grid gap-3 md:grid-cols-3">
          {[["Video", COLOR.clipVideo], ["Audio", COLOR.clipAudio], [
            "Text",
            COLOR.clipTitle,
          ]].map(([label, color]) => (
            <div
              class="rounded-[3px] border border-border px-3 py-3 text-xs text-text"
              style={`background:${color}`}
            >
              {label}
            </div>
          ))}
        </div>
      </ReviewSection>
      <p class="mt-8 text-sm">
        <a href="20-top-bar.html" class="underline underline-offset-4">
          Review the desktop shell
        </a>
        <span class="mx-3">·</span>
        <a href="21-file-explorer.html" class="underline underline-offset-4">
          Review media interactions
        </a>
      </p>
    </ReviewPage>
  );
}

const states: ControlState[] = [
  "Default",
  "Hover",
  "Pressed",
  "Focused",
  "Disabled",
];
const swatches = [
  ["Background", COLOR.page],
  ["Surface", COLOR.surface],
  ["Hover", COLOR.hover],
  ["Border", COLOR.border],
  ["Brand accent", COLOR.accent],
  ["Action / hover", COLOR.accentHover],
  ["Primary text", COLOR.text],
  ["Muted text", COLOR.muted],
];
