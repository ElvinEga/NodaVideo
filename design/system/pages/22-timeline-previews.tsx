import type { Child, Markup } from "@design/jsx/jsx-runtime";
import {
  ReviewFigure,
  ReviewPage,
  ReviewSection,
} from "../components/noda-shell.tsx";
import {
  At,
  ClipAt,
  COMPACT_RULER,
  Cursor,
  HEADER_WIDTH,
  LaneLine,
  SeekPreview,
  SEEK_POPUP_HEIGHT,
  Timeline,
  type TrackSpec,
} from "../components/timeline.tsx";
import { AudioClip, ImageClip, VideoClip } from "../components/timeline-clip.tsx";
import { INTERVIEW_B, MUSIC_BED, VOICE_OVER } from "../components/waveform-samples.ts";

const TWO_TRACKS: TrackSpec[] = [
  { kind: "V", name: "Video 1" },
  { kind: "A", name: "Audio 1" },
];

const HEADROOM = SEEK_POPUP_HEIGHT + 6;

/// A ruler-hover mock-up: the popup floats above the card, a faint guide marks the
/// hover x in the lane, and the cursor rests on the ruler at that x.
function SeekMock(props: {
  hoverX: number;
  caretX?: number;
  timecode: string;
  kind: "video" | "image" | "empty" | "loading";
  variant?: number;
  lane: Child;
  tracks?: TrackSpec[];
}): Markup {
  const caretX = props.caretX ?? 40;
  return (
    <div class="relative" style="width:620px">
      <div style={`height:${HEADROOM}px`}>
        <At left={HEADER_WIDTH + props.hoverX - caretX} top={0}>
          <SeekPreview
            timecode={props.timecode}
            kind={props.kind}
            variant={props.variant}
            caretX={caretX}
          />
        </At>
      </div>
      <Timeline
        width={620}
        tracks={props.tracks ?? TWO_TRACKS}
        ruler={COMPACT_RULER}
        lane={
          <>
            {props.lane}
            <LaneLine x={props.hoverX} color="#ffffff" opacity={0.35} />
          </>
        }
        overlay={
          <Cursor kind="arrow" left={HEADER_WIDTH + props.hoverX} top={HEADROOM + 4} />
        }
      />
    </div>
  );
}

const VIDEO_LANE = (
  <>
    <ClipAt left={20} row={0}>
      <VideoClip width={200} name="Weza_POS_TikTok" filmstrip />
    </ClipAt>
    <ClipAt left={20} row={1}>
      <AudioClip width={300} name="Voice over" waveform={INTERVIEW_B} />
    </ClipAt>
  </>
);

export default function TimelinePreviewsPage(): Markup {
  return (
    <ReviewPage
      title="See the picture on the timeline"
      subtitle="Two additions: a trick-play popup that previews the frame under the cursor while you hover the ruler, and clip bodies that show source frames for video and the still for images. Audio keeps its waveform. Frames decode off the UI thread and reuse the thumbnail disk cache."
    >
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2 text-xs text-ink/70">
        <span>Edit workspace · timeline ruler and clip bodies</span>
        <span>Illustrative frames only — never seeded into projects</span>
      </div>

      <ReviewSection
        title="Hover seek preview"
        caption="Hovering the ruler (or the monitor scrub bar) shows the frame at that time above the timeline, with the timecode and a caret on the exact hover position. A faint guide line marks the same x in the lane. The popup is an overlay: it never moves the playhead or seeks playback until you click."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure
            title="Over a video clip"
            caption="The decoded frame at the hover time, in the source aspect ratio."
          >
            <SeekMock hoverX={140} timecode="00:03.0" kind="video" variant={1} lane={VIDEO_LANE} />
          </ReviewFigure>
          <ReviewFigure
            title="Over an image clip"
            caption="A still has one frame; the popup shows the image itself."
          >
            <SeekMock
              hoverX={100}
              timecode="00:02.0"
              kind="image"
              lane={
                <>
                  <ClipAt left={20} row={0}>
                    <ImageClip width={160} name="title-card.png" />
                  </ClipAt>
                  <ClipAt left={20} row={1}>
                    <AudioClip width={300} name="Voice over" waveform={INTERVIEW_B} />
                  </ClipAt>
                </>
              }
            />
          </ReviewFigure>
          <ReviewFigure
            title="Over empty lane"
            caption="No clip at the hover time: the frame is black, the timecode still reads."
          >
            <SeekMock hoverX={300} timecode="00:07.0" kind="empty" lane={VIDEO_LANE} />
          </ReviewFigure>
          <ReviewFigure
            title="While the frame decodes"
            caption="First hover at a new time shows a skeleton until the background decode lands; it then swaps in without moving the popup."
          >
            <SeekMock hoverX={140} timecode="00:03.0" kind="loading" lane={VIDEO_LANE} />
          </ReviewFigure>
          <ReviewFigure
            title="Clamped at the right edge"
            caption="Near an edge the card stays inside the timeline; the caret slides to keep pointing at the hover x."
          >
            <SeekMock
              hoverX={410}
              caretX={70}
              timecode="00:09.7"
              kind="video"
              variant={2}
              lane={
                <>
                  <ClipAt left={20} row={0}>
                    <VideoClip width={400} name="Weza_POS_TikTok" filmstrip />
                  </ClipAt>
                  <ClipAt left={20} row={1}>
                    <AudioClip width={300} name="Voice over" waveform={INTERVIEW_B} />
                  </ClipAt>
                </>
              }
            />
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Clip bodies: frames for video and images, waveform for audio"
        caption="Video clips become a filmstrip of source frames with the name on a top scrim and a thin audio band underneath. Image clips show the still. Audio clips are unchanged. The legacy video body (waveform only) is shown first for comparison."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2 xl:grid-cols-4">
          <ReviewFigure title="Today: video" caption="Waveform of the embedded audio, name at the bottom.">
            <div class="bg-[#0a0a0c] p-4">
              <VideoClip width={280} name="Weza_POS_TikTok" waveform={VOICE_OVER} />
            </div>
          </ReviewFigure>
          <ReviewFigure title="Proposed: video" caption="Filmstrip of frames, name on a scrim, thin audio band.">
            <div class="bg-[#0a0a0c] p-4">
              <VideoClip width={280} name="Weza_POS_TikTok" filmstrip waveform={VOICE_OVER} />
            </div>
          </ReviewFigure>
          <ReviewFigure title="Proposed: image" caption="The still fills the body; name on a scrim.">
            <div class="bg-[#0a0a0c] p-4">
              <ImageClip width={160} name="title-card.png" />
            </div>
          </ReviewFigure>
          <ReviewFigure title="Audio: unchanged" caption="Waveform stays the audio representation.">
            <div class="bg-[#0a0a0c] p-4">
              <AudioClip width={280} name="Music bed" waveform={INTERVIEW_B} />
            </div>
          </ReviewFigure>
        </div>
        <div class="mt-6 overflow-auto">
          <Timeline
            width={900}
            tracks={[
              { kind: "V", name: "Video 1" },
              { kind: "V", name: "Video 2" },
              { kind: "A", name: "Audio 1" },
            ]}
            ruler={COMPACT_RULER}
            playhead={250}
            lane={
              <>
                <ClipAt left={20} row={0}>
                  <VideoClip width={420} name="Weza_POS_TikTok" filmstrip waveform={VOICE_OVER} />
                </ClipAt>
                <ClipAt left={20} row={1}>
                  <ImageClip width={160} name="title-card.png" />
                </ClipAt>
                <ClipAt left={190} row={1}>
                  <ImageClip width={120} name="logo.png" selected />
                </ClipAt>
                <ClipAt left={20} row={2}>
                  <AudioClip width={520} name="Music bed" waveform={MUSIC_BED} />
                </ClipAt>
              </>
            }
          />
        </div>
        <p class="mt-3 text-xs leading-5 text-ink/70">
          The same timeline with all three body types together. Filmstrip cell width follows the
          source aspect ratio at the current zoom; zooming re-samples cells rather than stretching
          a single thumbnail.
        </p>
      </ReviewSection>

      <ReviewSection
        title="Behavior and cost"
        caption="How the previews stay cheap and never block input."
      >
        <div class="grid gap-4 text-xs leading-5 text-ink/70 sm:grid-cols-3">
          <p>
            <strong class="text-ink">Off the UI thread.</strong> Hover frames and filmstrip cells
            decode on the background executor through the existing one-at-a-time thumbnail queue.
            The ruler only reads already-decoded frames; a missing frame shows the skeleton.
          </p>
          <p>
            <strong class="text-ink">Cached, not stored in the project.</strong> Frames reuse the
            thumbnail disk cache keyed by path, mtime and size. Nothing is written into project
            JSON and no media is copied. Hover requests are debounced and superseded so a fast
            scrub does not spawn unbounded decodes.
          </p>
          <p>
            <strong class="text-ink">Honest fallbacks.</strong> Empty lane previews black; a failed
            decode keeps the skeleton and the clip's solid fill rather than a fabricated frame.
            Audio clips and the waveform pipeline are untouched.
          </p>
        </div>
      </ReviewSection>

      <ReviewSection title="Companion reviews" caption="Shared timeline and shell components.">
        <div class="flex flex-wrap gap-5 text-sm">
          <a href="17-timeline-clips.html" class="underline underline-offset-4">Timeline clips</a>
          <a href="17-timeline-editing-area.html" class="underline underline-offset-4">Timeline editing area</a>
          <a href="20-top-bar.html" class="underline underline-offset-4">Desktop shell</a>
        </div>
      </ReviewSection>
    </ReviewPage>
  );
}