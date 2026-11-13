import type { Markup } from "@design/jsx/jsx-runtime";
import {
  AssistantPanel,
  NodaShell,
  NodaTopBar,
  ReviewFigure,
  ReviewPage,
  ReviewSection,
  WorkspacePlaceholder,
} from "../components/noda-shell.tsx";

export default function DesktopShellPage(): Markup {
  return (
    <ReviewPage
      title="Your timeline. Your workspace."
      subtitle="Noda Obsidian: matte surfaces, quiet borders, and indigo reserved for focus and intent. Native Rust + GPUI; the existing video, audio, timeline, and AI engines stay in place."
    >
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2 text-xs text-ink/70">
        <span>
          Edit workspace · 48 px toolbar · resizable media, monitor, and right
          dock
        </span>
        <span>Illustrative media only — never seeded into projects</span>
      </div>
      <div class="overflow-auto">
        <NodaShell />
      </div>
      <div class="mt-4 grid gap-4 text-xs leading-5 text-ink/70 sm:grid-cols-3">
        <p>
          <strong class="text-ink">A quiet identity.</strong>{" "}
          The two-stroke N mark echoes adjacent cuts. System typography stays
          compact; frame timecodes use a fixed-width face.
        </p>
        <p>
          <strong class="text-ink">An editor first.</strong>{" "}
          The timeline is persistent. Media and Inspector have independent
          visibility controls; AI can take the right dock without displacing the
          edit.
        </p>
        <p>
          <strong class="text-ink">Direct manipulation.</strong>{" "}
          Double-click media to open the source monitor. Drag the whole asset or
          its marked range onto a compatible track.
        </p>
      </div>
      <ReviewSection
        title="Start with an empty project"
        caption="No fabricated media, sample timelines, or ready-made clips. Import opens the native multi-file picker; file drops use the same background import pipeline. Export and history actions are disabled until applicable."
      >
        <div class="overflow-auto">
          <NodaShell empty />
        </div>
      </ReviewSection>
      <ReviewSection
        title="Workspace navigation"
        caption="Edit is the default. Color and Audio are honest placeholders, not simulated tools. AI Studio opens the assistant-focused workspace. These static states show the selected tabs; application behavior follows after design approval."
      >
        <div class="space-y-4">
          {(["Color", "Audio", "AI Studio"] as const).map((workspace) => (
            <div class="overflow-auto rounded-[4px] border border-border">
              <div class="min-w-[1080px]">
                <NodaTopBar
                  workspace={workspace}
                  assistant={workspace === "AI Studio"}
                />
                <div class="h-[320px]">
                  {workspace === "AI Studio"
                    ? (
                      <div class="mx-auto h-full max-w-[540px] [&>section]:min-h-0">
                        <AssistantPanel />
                      </div>
                    )
                    : <WorkspacePlaceholder workspace={workspace} />}
                </div>
              </div>
            </div>
          ))}
        </div>
        <p class="mt-3 text-xs leading-5 text-ink/70">
          The same timeline remains beneath every workspace. Workspace selection
          does not change playback position, timeline history, or the active
          project.
        </p>
      </ReviewSection>
      <ReviewSection
        title="Panels out of the way"
        caption="Media, Inspector, and Assistant can all be hidden. Keep panel widths and visibility in the session, restore them when reopened, and enforce minimum sizes rather than overlapping the monitor."
      >
        <div class="overflow-auto">
          <NodaShell hidden />
        </div>
      </ReviewSection>
      <ReviewSection
        title="Native menu suite"
        caption="Menus use native platform controls, not an in-app imitation. The menu strip in the mockup locates them conceptually. macOS conventions are shown; other platforms use their equivalent modifiers. Unavailable actions are disabled, never successful no-ops."
      >
        <div class="grid gap-x-5 gap-y-2 md:grid-cols-2 xl:grid-cols-3">
          {menus.map((menu) => (
            <ReviewFigure title={menu.title} caption={menu.caption}>
              <div class="rounded-[4px] border border-border bg-surface py-1.5 text-text">
                {menu.items.map(([label, shortcut], index) =>
                  label === "—"
                    ? (
                      <hr
                        key={`${menu.title}-${index}`}
                        class="my-1.5 border-border"
                      />
                    )
                    : (
                      <div class="flex min-h-7 items-center justify-between gap-5 px-3 text-[11px]">
                        <span>{label}</span>
                        <kbd class="shrink-0 font-mono text-[10px] text-muted">
                          {shortcut}
                        </kbd>
                      </div>
                    )
                )}
              </div>
            </ReviewFigure>
          ))}
        </div>
      </ReviewSection>
      <ReviewSection
        title="Companion reviews"
        caption="Shared control states and media interactions use the same components as this shell."
      >
        <div class="flex flex-wrap gap-5 text-sm">
          <a href="00-design-system.html" class="underline underline-offset-4">
            Noda Obsidian design system
          </a>
          <a href="21-file-explorer.html" class="underline underline-offset-4">
            Media bin, source ranges, and drop states
          </a>
        </div>
      </ReviewSection>
    </ReviewPage>
  );
}

const menus = [
  {
    title: "File",
    caption: "Project lifecycle, import, and export.",
    items: [
      ["New Project", "⌘ N"],
      ["Open Project…", "⌘ O"],
      ["Save Project", "⌘ S"],
      ["—", ""],
      ["Import Media…", "⌘ I"],
      ["Export Video…", "⌘ E"],
      ["—", ""],
      ["Close Window", "⌘ W"],
    ],
  },
  {
    title: "Edit",
    caption: "Timeline actions yield one undo step each.",
    items: [
      ["Undo", "⌘ Z"],
      ["Redo", "⇧ ⌘ Z"],
      ["—", ""],
      ["Cut", "⌘ X"],
      ["Copy", "⌘ C"],
      ["Paste", "⌘ V"],
      ["Delete", "⌫"],
      ["Select All", "⌘ A"],
    ],
  },
  {
    title: "View",
    caption: "Independent panel toggles and full screen.",
    items: [
      ["Media Browser", "⌘ 1"],
      ["Inspector", "⌘ 2"],
      ["AI Assistant", "⇧ ⌘ A"],
      ["—", ""],
      ["Enter Full Screen", "⌘ F"],
    ],
  },
  {
    title: "Playback",
    caption: "Frame-based transport follows the active monitor.",
    items: [
      ["Play / Pause", "Space"],
      ["Step Forward", "→"],
      ["Step Backward", "←"],
      ["Go to Start", "Home / ↑"],
      ["Go to End", "End / ↓"],
      ["—", ""],
      ["Mark In", "I"],
      ["Mark Out", "O"],
    ],
  },
  {
    title: "Window",
    caption: "Delegate window behavior to the platform.",
    items: [["Minimize", "⌘ M"], ["Zoom", ""], ["—", ""], [
      "Bring All to Front",
      "",
    ]],
  },
  {
    title: "Help",
    caption: "About resides in the app menu on macOS.",
    items: [["Keyboard Shortcuts", ""], ["Documentation", ""], ["—", ""], [
      "About NodaVideo",
      "",
    ]],
  },
];
