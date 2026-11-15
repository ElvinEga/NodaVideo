import type { Markup } from "@design/jsx/jsx-runtime";
import { NodaGlyph, NodaMark } from "./noda-shell.tsx";

export type DesignPage = { file: string; label: string; group: string };

/// Every design page in reading order. Build injects this index beside each page.
export const DESIGN_PAGES: DesignPage[] = [
  { file: "00-design-system", label: "Design system", group: "Foundation" },
  { file: "20-top-bar", label: "Desktop shell", group: "Workspace" },
  { file: "21-file-explorer", label: "Media browser", group: "Workspace" },
  { file: "23-text-clips", label: "Text clips", group: "Workspace" },
  { file: "24-motion-graphics", label: "Motion graphics", group: "Motion graphics" },
  {
    file: "25-motion-graphics-editor",
    label: "Motion Graphics Editor",
    group: "Motion graphics",
  },
  { file: "26-ai-assistant", label: "AI Assistant", group: "AI assistant" },
  { file: "01-project-picker", label: "Project picker", group: "Workspace" },
  { file: "16-preview-panel", label: "Preview panel", group: "Panels" },
  { file: "15-property-panel", label: "Property panel", group: "Panels" },
  {
    file: "15-property-panel-text-clip",
    label: "Text clip properties",
    group: "Panels",
  },
  {
    file: "15-property-panel-timeline",
    label: "Timeline properties",
    group: "Panels",
  },
  {
    file: "17-timeline-editing-area",
    label: "Timeline editing area",
    group: "Timeline",
  },
  { file: "17-timeline-clips", label: "Timeline clips", group: "Timeline" },
  { file: "17-timeline-track", label: "Timeline track", group: "Timeline" },
  { file: "18-video-player", label: "Video player", group: "Timeline" },
  {
    file: "13-export-window",
    label: "Export window",
    group: "Windows and dialogs",
  },
  {
    file: "14-transcription-window",
    label: "Transcription",
    group: "Windows and dialogs",
  },
  {
    file: "19-missing-media-dialog",
    label: "Missing media",
    group: "Windows and dialogs",
  },
  {
    file: "12-fullscreen-preview",
    label: "Fullscreen preview",
    group: "Windows and dialogs",
  },
  { file: "07-dialogs", label: "Dialogs", group: "Windows and dialogs" },
  {
    file: "11-context-menus",
    label: "Context menus",
    group: "Menus and history",
  },
  {
    file: "06-edit-history",
    label: "Edit history",
    group: "Menus and history",
  },
];

/// A static index drawer. A hidden checkbox drives the state, so the built page
/// keeps its no-JavaScript guarantee; labels are the open and close controls.
export function DesignNavigation(
  { current, pages = DESIGN_PAGES }: {
    current: string;
    pages?: DesignPage[];
  },
): Markup {
  const groups = [...new Set(pages.map((page) => page.group))];
  return (
    <>
      <input
        id="design-nav-toggle"
        type="checkbox"
        class="design-nav-toggle"
        aria-label="Toggle design index"
      />
      <aside class="design-nav" aria-label="Design index">
        <div class="flex h-14 shrink-0 items-center justify-between gap-2 border-b border-rule px-4">
          <div class="flex min-w-0 items-center gap-2.5">
            <NodaMark />
            <div class="min-w-0">
              <p class="truncate text-[12px] font-semibold leading-4">
                Design index
              </p>
              <p class="truncate text-[10px] leading-4 text-ink/55">
                NodaVideo review
              </p>
            </div>
          </div>
          <label
            for="design-nav-toggle"
            class="design-nav-close"
            title="Hide design index"
            aria-label="Hide design index"
          >
            <NodaGlyph name="collapse" size={13} />
          </label>
        </div>
        <nav class="design-nav-list" aria-label="Design pages">
          {groups.map((group) => (
            <section class="mb-5">
              <p class="px-2 pb-1.5 text-[10px] font-semibold tracking-[0.12em] text-ink/45 uppercase">
                {group}
              </p>
              <ul class="space-y-px">
                {pages.filter((page) => page.group === group).map((page) => {
                  const active = page.file === current;
                  return (
                    <li>
                      <a
                        href={`${page.file}.html`}
                        aria-current={active ? "page" : undefined}
                        class={`design-nav-link ${
                          active ? "design-nav-link-current" : ""
                        }`}
                      >
                        <span class="w-6 shrink-0 font-mono text-[10px] text-ink/35">
                          {page.file.slice(0, 2)}
                        </span>
                        <span class="min-w-0 truncate">{page.label}</span>
                      </a>
                    </li>
                  );
                })}
              </ul>
            </section>
          ))}
        </nav>
        <p class="shrink-0 border-t border-rule px-4 py-3 text-[10px] leading-4 text-ink/50">
          Static proposals from{" "}
          <span class="font-mono">design/system/pages</span>
        </p>
      </aside>
      <label
        for="design-nav-toggle"
        class="design-nav-scrim"
        aria-hidden="true"
      />
      <label
        for="design-nav-toggle"
        class="design-nav-open"
        title="Show design index"
      >
        <NodaGlyph name="menu" size={16} />
        <span>Design index</span>
      </label>
    </>
  );
}
