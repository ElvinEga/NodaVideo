import type { Markup } from "@design/jsx/jsx-runtime";
import {
  AssistantPanel,
  type BinState,
  MediaBin,
  PreviewMonitor,
  ReviewFigure,
  ReviewPage,
  ReviewSection,
  TimelinePanel,
} from "../components/noda-shell.tsx";

export default function MediaBrowserPage(): Markup {
  return (
    <ReviewPage
      title="From source to sequence"
      subtitle="One media bin, two ways to edit: drag an asset directly onto a track, or open it in the source monitor and mark the range you need. Media, thumbnails, waveforms, and tool activity below are illustrative design fixtures, not application data."
    >
      <div class="grid gap-x-6 gap-y-2 lg:grid-cols-3">
        <ReviewFigure
          title="Grid · selected asset"
          caption="Cached, aspect-preserving previews with filename, duration, resolution, and an audio indicator. Double-click opens the source."
        >
          <div class="h-[460px] min-w-[280px] border border-border">
            <MediaBin />
          </div>
        </ReviewFigure>
        <ReviewFigure
          title="List · same assets"
          caption="A compact alternative for longer names and larger bins. Search, selection, and category are preserved when switching views."
        >
          <div class="h-[460px] min-w-[280px] border border-border">
            <MediaBin state="list" />
          </div>
        </ReviewFigure>
        <ReviewFigure
          title="Source · marked range"
          caption="In is inclusive; Out is exclusive. I / O or handles mark at least one frame. Insert or drag the range; the original file is unchanged."
        >
          <div class="h-[460px] min-w-[320px] border border-border">
            <PreviewMonitor source />
          </div>
        </ReviewFigure>
      </div>
      <ReviewSection
        title="Import and recovery states"
        caption="The native multi-file picker and operating-system drops share one bounded background import queue. Duplicate references are reported, failures stay visible, and missing media keeps its identity instead of disappearing."
      >
        <div class="grid gap-x-6 gap-y-2 lg:grid-cols-3">
          {states.map(({ state, title, caption }) => (
            <ReviewFigure title={title} caption={caption}>
              <div class="h-[440px] min-w-[280px] border border-border">
                <MediaBin state={state} />
              </div>
            </ReviewFigure>
          ))}
        </div>
      </ReviewSection>
      <ReviewSection
        title="Playlist → timeline"
        caption="Placement uses the existing frame-based edit and undo system. Video and graphics target video tracks; audio targets audio tracks. OS files dropped on the timeline first import asynchronously, then place at the captured target position."
      >
        <div class="grid grid-cols-1 gap-6">
          <div>
            <h3 class="mb-1 text-sm font-semibold">
              Compatible track · snapped to an edge
            </h3>
            <p class="mb-3 text-xs leading-5 text-ink/70">
              The translucent indigo ghost has the actual source-range duration.
              Snap is shared with clip movement, not a separate placement
              algorithm.
            </p>
            <div class="overflow-auto rounded-[4px] border border-border">
              <div class="min-w-[900px]">
                <TimelinePanel drop="valid" />
              </div>
            </div>
          </div>
          <div>
            <h3 class="mb-1 text-sm font-semibold">
              Rejected track · no timeline mutation
            </h3>
            <p class="mb-3 text-xs leading-5 text-ink/70">
              Incompatible types, locked tracks, and overlaps show a reason and
              a blocked cursor. Releasing an invalid drop leaves the timeline
              and undo stack untouched.
            </p>
            <div class="overflow-auto rounded-[4px] border border-border">
              <div class="min-w-[900px]">
                <TimelinePanel drop="invalid" />
              </div>
            </div>
          </div>
        </div>
      </ReviewSection>
      <ReviewSection
        title="AI remains optional"
        caption="The right dock can show Inspector or Assistant. Provider cycling preserves existing integrations. This is the proposed approval state, not a claim that panel tools already execute timeline mutations."
      >
        <div class="grid max-w-[840px] gap-x-6 gap-y-2 md:grid-cols-2">
          <ReviewFigure
            title="Ready for a request"
            caption="No invented conversation. Hide the panel to return all available width to editing."
          >
            <div class="h-[450px] border border-border">
              <AssistantPanel />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="Review a proposed edit"
            caption="An explicit approval card describes the operation before a timeline change. Reject leaves the edit untouched."
          >
            <div class="h-[450px] border border-border">
              <AssistantPanel approval />
            </div>
          </ReviewFigure>
        </div>
      </ReviewSection>
      <p class="mt-8 text-sm">
        <a href="20-top-bar.html" class="underline underline-offset-4">
          Back to the complete desktop shell
        </a>
      </p>
    </ReviewPage>
  );
}

const states: { state: BinState; title: string; caption: string }[] = [
  {
    state: "empty",
    title: "Empty bin",
    caption:
      "Explain both import paths. No sample assets appear in a new project.",
  },
  {
    state: "drop",
    title: "Files over the drop target",
    caption:
      "Highlight only the accepting panel. Release imports references without copying the source files.",
  },
  {
    state: "filtered",
    title: "No search results",
    caption:
      "A search miss is not an empty project. Keep import available and offer a clear search reset.",
  },
  {
    state: "importing",
    title: "Import in progress",
    caption:
      "Completed assets are usable while the queue continues. Metadata and thumbnails do not block input.",
  },
  {
    state: "error",
    title: "One file failed",
    caption:
      "Keep successful imports. Name the failed file and explain the problem without hiding it in a log.",
  },
  {
    state: "missing",
    title: "Missing source on reopen",
    caption:
      "Retain the reference, mark it unavailable, and prevent preview or drag until the file is restored.",
  },
];
