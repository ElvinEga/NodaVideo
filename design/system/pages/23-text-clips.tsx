import type { Markup } from "@design/jsx/jsx-runtime";
import {
  ReviewFigure,
  ReviewPage,
  ReviewSection,
} from "../components/noda-shell.tsx";
import {
  NEW_TITLE,
  TITLE_FIXTURES,
  TextMediaBin,
  TextSourceMonitor,
} from "../components/text-media.tsx";
import { TextClipPanel } from "../components/text-clip-panel.tsx";
import {
  ClipAt,
  COMPACT_RULER,
  Cursor,
  GhostClip,
  HEADER_WIDTH,
  Timeline,
  type TrackSpec,
} from "../components/timeline.tsx";
import { AudioClip, TextClip, VideoClip } from "../components/timeline-clip.tsx";
import { MUSIC_BED } from "../components/waveform-samples.ts";

const TRACKS: TrackSpec[] = [
  { kind: "T", name: "Text 1" },
  { kind: "V", name: "Video 1" },
  { kind: "A", name: "Audio 1" },
];

const VIDEO_LANE = (
  <ClipAt left={20} row={1}>
    <VideoClip width={300} name="street-at-dusk.mov" filmstrip />
  </ClipAt>
);
const AUDIO_LANE = (
  <ClipAt left={20} row={2}>
    <AudioClip width={420} name="city-ambience.wav" waveform={MUSIC_BED} />
  </ClipAt>
);

export default function TextClipsPage(): Markup {
  return (
    <ReviewPage
      title="Make a title, keep it in the Media Pool"
      subtitle="Text clips become reusable, project-local titles: create one with New Text, preview it rendered over black, then add it to a Text track or drag it there. Titles are data in the project, not imported files — nothing is copied and no thumbnail binaries are stored. Previews below are illustrative fixtures, not application data."
    >
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2 text-xs text-ink/70">
        <span>Edit workspace · Media Pool, source monitor, timeline, Inspector</span>
        <span>Titles are created in-project, never seeded</span>
      </div>

      <ReviewSection
        title="Create a title"
        caption="New Text adds a title with the default properties (text “Text”, white, Sans, 3s), selects it, and opens the Inspector. The timeline toolbar’s + Text adds a Text track; New Text adds the reusable title item. A new project starts with no titles."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2 xl:grid-cols-3">
          <ReviewFigure
            title="New Text · just created"
            caption="The default title appears in the Text category, selected and ready to edit."
          >
            <div class="h-[460px] w-[320px] border border-border">
              <TextMediaBin titles={[NEW_TITLE]} selected={0} />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="Empty Text category"
            caption="No sample titles. The empty state invites creation and explains that titles are made, not imported."
          >
            <div class="h-[460px] w-[320px] border border-border">
              <TextMediaBin titles={[]} empty />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="Preview the new title"
            caption="The source monitor renders the text over black. Add to Timeline places it at the playhead."
          >
            <div class="h-[460px] min-w-[320px] border border-border">
              <TextSourceMonitor title={NEW_TITLE} />
            </div>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Media Pool · Text category"
        caption="A new Text category sits beside All, Video, Audio and Graphics (and titles also appear in All). Each card draws the title itself — no decoded thumbnail — with a T badge, name, font, font-color chip and duration."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure
            title="Grid"
            caption="Rendered previews in the source aspect; the selected card uses the accent border."
          >
            <div class="h-[460px] w-[320px] border border-border">
              <TextMediaBin titles={TITLE_FIXTURES} view="grid" selected={0} />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="List"
            caption="The same titles as compact rows. Search, selection and category persist across views."
          >
            <div class="h-[460px] w-[320px] border border-border">
              <TextMediaBin titles={TITLE_FIXTURES} view="list" selected={0} />
            </div>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Preview a title"
        caption="The monitor renders the title with its font, size, color, background and position. A background box draws behind the text at its opacity; without one, the text sits over black."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure
            title="Lower third · background"
            caption="A semi-transparent indigo bar renders behind the white text at 80%."
          >
            <div class="h-[460px] min-w-[320px] border border-border">
              <TextSourceMonitor title={TITLE_FIXTURES[0]} />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="Section header · color"
            caption="Plain colored text over black, centered at its position."
          >
            <div class="h-[460px] min-w-[320px] border border-border">
              <TextSourceMonitor title={TITLE_FIXTURES[2]} />
            </div>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Place on the timeline"
        caption="Add to Timeline places the title at the playhead on a Text track, creating one if needed. Dragging places it at the drop position. Both are one atomic, undoable edit through the existing timeline API. A title only accepts a Text track."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure
            title="Drag to a Text track"
            caption="The indigo ghost carries the title’s duration; snap is shared with clip movement."
          >
            <Timeline
              width={620}
              tracks={TRACKS}
              ruler={COMPACT_RULER}
              lane={
                <>
                  <ClipAt left={220} row={0}>
                    <GhostClip
                      width={180}
                      accepted
                      name="Lower third"
                      caption="Release to place · Text"
                    />
                  </ClipAt>
                  {VIDEO_LANE}
                  {AUDIO_LANE}
                </>
              }
              overlay={<Cursor kind="hand" left={HEADER_WIDTH + 300} top={70} />}
            />
          </ReviewFigure>
          <ReviewFigure
            title="Rejected on a video track"
            caption="Incompatible track: a reason and a blocked cursor. Release leaves the timeline and undo stack untouched."
          >
            <Timeline
              width={620}
              tracks={TRACKS}
              ruler={COMPACT_RULER}
              lane={
                <>
                  <ClipAt left={20} row={0}>
                    <TextClip width={160} name="End title" />
                  </ClipAt>
                  <ClipAt left={220} row={1}>
                    <GhostClip
                      width={180}
                      accepted={false}
                      name="Lower third"
                      caption="Text needs a Text track"
                    />
                  </ClipAt>
                  {AUDIO_LANE}
                </>
              }
              overlay={
                <Cursor kind="blocked" left={HEADER_WIDTH + 300} top={144} />
              }
            />
          </ReviewFigure>
          <ReviewFigure
            title="Placed · selected"
            caption="The purple text clip on Text 1. Selecting it opens the Inspector; double-click edits the content in place."
          >
            <Timeline
              width={620}
              tracks={TRACKS}
              ruler={COMPACT_RULER}
              playhead={120}
              lane={
                <>
                  <ClipAt left={20} row={0}>
                    <TextClip width={200} name="End title" selected />
                  </ClipAt>
                  {VIDEO_LANE}
                  {AUDIO_LANE}
                </>
              }
            />
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Edit properties"
        caption="Selecting a title in the pool or a clip on the timeline shows the text clip panel: content, timing, font, size, color, background and position. Placing copies the title’s properties into an independent clip, so editing a placed clip never changes the reusable pool title. Every panel state is on the text clip properties page."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure
            title="Default"
            caption="The selected title’s properties, ready to edit."
          >
            <TextClipPanel shift={-8} />
          </ReviewFigure>
          <ReviewFigure
            title="Editing content"
            caption="Click the content field to type; the caret and accent border show focus."
          >
            <TextClipPanel shift={-8} editingContent />
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Behavior and cost"
        caption="How titles stay cheap, honest and fully under the user’s control."
      >
        <div class="grid gap-4 text-xs leading-5 text-ink/70 sm:grid-cols-3">
          <p>
            <strong class="text-ink">Project-local data.</strong> A title is text,
            font, color, background and position stored in project JSON. It has no
            source path, copies no file, and stores no thumbnail binary. Deleting a
            title never touches media on disk.
          </p>
          <p>
            <strong class="text-ink">Preview by rendering.</strong> Pool cards and
            the source monitor draw the text directly; there is no decode queue and
            nothing blocks input. The preview updates as properties change.
          </p>
          <p>
            <strong class="text-ink">One undoable placement.</strong> Add to
            Timeline and drag both create a single text clip through the existing
            edit and undo system, creating a Text track only when needed. An
            invalid drop mutates nothing.
          </p>
        </div>
      </ReviewSection>

      <ReviewSection title="Companion reviews" caption="Shared shell, timeline and panel components.">
        <div class="flex flex-wrap gap-5 text-sm">
          <a href="21-file-explorer.html" class="underline underline-offset-4">Media browser</a>
          <a href="15-property-panel-text-clip.html" class="underline underline-offset-4">Text clip properties</a>
          <a href="17-timeline-clips.html" class="underline underline-offset-4">Timeline clips</a>
          <a href="22-timeline-previews.html" class="underline underline-offset-4">Timeline previews</a>
          <a href="20-top-bar.html" class="underline underline-offset-4">Desktop shell</a>
        </div>
      </ReviewSection>
    </ReviewPage>
  );
}