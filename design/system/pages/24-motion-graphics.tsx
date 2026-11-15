import type { Markup } from "@design/jsx/jsx-runtime";
import {
  NodaButton,
  NodaGlyph,
  NodaPanelHeader,
  ReviewFigure,
  ReviewPage,
  ReviewSection,
} from "../components/noda-shell.tsx";
import {
  ALPHA_STYLE,
  COMPOSITION_FIXTURES,
  CompositionPanel,
  CompositionThumb,
  MotionClip,
  MotionMark,
  MotionMediaBin,
  MotionSourceMonitor,
  TemplateGallery,
} from "../components/motion-media.tsx";
import {
  ClipAt,
  COMPACT_RULER,
  Cursor,
  GhostClip,
  HEADER_WIDTH,
  Timeline,
  type TrackSpec,
} from "../components/timeline.tsx";
import { AudioClip, VideoClip } from "../components/timeline-clip.tsx";
import { MUSIC_BED } from "../components/waveform-samples.ts";

// A composition composites over footage, so it lives on a video track above the footage.
const TRACKS: TrackSpec[] = [
  { kind: "V", name: "Video 2" },
  { kind: "V", name: "Video 1" },
  { kind: "A", name: "Audio 1" },
];

const FOOTAGE_LANE = (
  <ClipAt left={20} row={1}>
    <VideoClip width={300} name="street-at-dusk.mov" filmstrip />
  </ClipAt>
);
const AUDIO_LANE = (
  <ClipAt left={20} row={2}>
    <AudioClip width={420} name="city-ambience.wav" waveform={MUSIC_BED} />
  </ClipAt>
);

export default function MotionGraphicsPage(): Markup {
  return (
    <ReviewPage
      title="Motion graphics, native to your timeline"
      subtitle="HTML compositions render through HyperFrames and composite over your footage with alpha. The Rust timeline still owns timing, placement, tracks, trimming, selection and undo — HyperFrames only draws the pixels. Create one from a template, preview it over a checkerboard or over footage, drop it on a video track, then edit its parameters in the Inspector or open the dedicated Motion Graphics Editor. Previews below are illustrative fixtures, not rendered frames."
    >
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2 text-xs text-ink/70">
        <span>Edit workspace · Media Pool, source monitor, timeline, Inspector, AI Assistant</span>
        <span>Compositions are project-local HTML, never copied media</span>
      </div>

      <ReviewSection
        title="Create from a template"
        caption="New Motion Graphic opens the built-in library, grouped as the real templates/ directory is: Titles, Lower Thirds, Captions and Transitions. Choosing one creates a composition with default parameters — nothing is imported and no file is copied. The composition is then editable in the Inspector and the Motion Graphics Editor."
      >
        <div class="grid">
          <ReviewFigure
            title="Template gallery"
            caption="Each card is a rendered still over a checkerboard. The selected template (Lower Third Modern) gets the accent border and a check; Create makes the composition."
          >
            <TemplateGallery selected={2} />
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Media Pool · Motion category"
        caption="A Motion category sits beside All, Video, Audio, Image, Captions and Text. The bin lists project compositions, not files: each card draws the composition itself over a checkerboard with a spark badge, name, template and duration. A new project starts empty and invites creation."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2 xl:grid-cols-3">
          <ReviewFigure
            title="Grid"
            caption="Rendered previews in the source aspect; the selected card uses the accent border."
          >
            <div class="h-[460px] w-[360px] border border-border">
              <MotionMediaBin comps={COMPOSITION_FIXTURES} view="grid" selected={0} />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="List"
            caption="The same compositions as compact rows. Search, selection and category persist across views."
          >
            <div class="h-[460px] w-[360px] border border-border">
              <MotionMediaBin comps={COMPOSITION_FIXTURES} view="list" selected={0} />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="Empty Motion category"
            caption="No sample compositions. The empty state invites creation and explains that graphics are made, not imported."
          >
            <div class="h-[460px] w-[360px] border border-border">
              <MotionMediaBin comps={[]} empty />
            </div>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Preview a composition"
        caption="The source monitor renders a frame through HyperFrames, not by decoding a file. Toggle Alpha to see the composition over a checkerboard (what will composite), or Footage to see it over the clip below. Add to Timeline places it at the playhead on a video track."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure
            title="Over alpha"
            caption="The checkerboard shows transparency: only the bar, name and role are opaque. The rest composites over whatever is below."
          >
            <div class="h-[460px] min-w-[320px] border border-border">
              <MotionSourceMonitor comp={COMPOSITION_FIXTURES[0]} over="alpha" />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="Over footage"
            caption="The same frame composited over street-at-dusk.mov, as the native GPU renderer will draw it on the timeline."
          >
            <div class="h-[460px] min-w-[320px] border border-border">
              <MotionSourceMonitor comp={COMPOSITION_FIXTURES[0]} over="footage" />
            </div>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Place on the timeline"
        caption="A composition is a video-layer clip: it accepts a video track and composites over the tracks below. Add to Timeline places it at the playhead, creating a video track if needed; dragging places it at the drop position. Both are one atomic, undoable edit through the existing timeline API. An audio or text track rejects it and mutates nothing."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure
            title="Drag to a video track"
            caption="The indigo ghost carries the composition’s duration above the footage on Video 2; snap is shared with clip movement."
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
                      name="Speaker card"
                      caption="Release to place · Video"
                    />
                  </ClipAt>
                  {FOOTAGE_LANE}
                  {AUDIO_LANE}
                </>
              }
              overlay={<Cursor kind="hand" left={HEADER_WIDTH + 300} top={70} />}
            />
          </ReviewFigure>
          <ReviewFigure
            title="Rejected on an audio track"
            caption="Incompatible track: a reason and a blocked cursor. Release leaves the timeline and undo stack untouched."
          >
            <Timeline
              width={620}
              tracks={TRACKS}
              ruler={COMPACT_RULER}
              lane={
                <>
                  <ClipAt left={20} row={0}>
                    <MotionClip width={180} name="Speaker card" />
                  </ClipAt>
                  {FOOTAGE_LANE}
                  <ClipAt left={220} row={2}>
                    <GhostClip
                      width={180}
                      accepted={false}
                      name="Speaker card"
                      caption="Motion needs a video track"
                    />
                  </ClipAt>
                </>
              }
              overlay={
                <Cursor kind="blocked" left={HEADER_WIDTH + 300} top={218} />
              }
            />
          </ReviewFigure>
          <ReviewFigure
            title="Placed · selected"
            caption="The amber motion clip on Video 2, its filmstrip animating in over the footage. Selecting it opens the composition Inspector."
          >
            <Timeline
              width={620}
              tracks={TRACKS}
              ruler={COMPACT_RULER}
              playhead={120}
              lane={
                <>
                  <ClipAt left={20} row={0}>
                    <MotionClip width={180} name="Speaker card" selected />
                  </ClipAt>
                  {FOOTAGE_LANE}
                  {AUDIO_LANE}
                </>
              }
            />
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Inspect and edit"
        caption="Selecting a composition clip shows its source facts (template, size, frame rate, duration, project path), the template’s editable parameters, and compositing. Placing copies the composition’s parameters into an independent clip, so editing a placed clip never changes the reusable pool composition. Open Motion Graphics Editor is the door into the dedicated window; Relink reconnects a moved composition folder."
      >
        <div class="grid items-start gap-x-5 gap-y-6 md:grid-cols-2">
          <div class="grid">
            <ReviewFigure
              title="Composition Inspector"
              caption="Parameters edit this clip only. Transparent background keeps alpha so the GPU renderer composites it over the tracks below."
            >
              <CompositionPanel />
            </ReviewFigure>
          </div>
          <div class="max-w-[420px] space-y-3 self-start text-xs leading-5 text-ink/70">
            <p>
              <strong class="text-ink">Two levels of editing.</strong> The Inspector
              changes the template’s parameters — text, colors, numbers — for quick
              tweaks. The Motion Graphics Editor changes the composition itself:
              layers, animation and source.
            </p>
            <p>
              <strong class="text-ink">Copy on place.</strong> Like a text title, a
              placed composition carries its own parameter snapshot. The pool item
              stays reusable and unchanged.
            </p>
            <p>
              <strong class="text-ink">Reference, don’t copy.</strong> The clip points
              at <span class="font-mono">compositions/speaker-card/</span> in the
              project. No media is duplicated and no thumbnail binary is stored in
              project JSON.
            </p>
          </div>
        </div>
      </ReviewSection>

      <ReviewSection
        title="AI generation, with approval"
        caption="The AI Assistant can propose a composition from a prompt. It generates HTML through the same HyperFrames bridge, shows a rendered preview, and waits. Nothing enters the project or the timeline until you approve — the same gating as every other timeline mutation."
      >
        <div class="grid items-start gap-x-5 gap-y-6 md:grid-cols-2">
          <div class="grid">
            <ReviewFigure
              title="Review a generated graphic"
              caption="The assistant proposes a lower third; the approval card shows a rendered preview and the template facts. Approve adds it to the Media Pool; Reject discards it."
            >
              <div class="h-[640px] w-[360px] border border-border">
                <AiApproval />
              </div>
            </ReviewFigure>
          </div>
          <div class="max-w-[420px] space-y-3 self-start text-xs leading-5 text-ink/70">
            <p>
              <strong class="text-ink">Propose, don’t apply.</strong> Generation
              produces a candidate composition. The preview is rendered by
              HyperFrames so you see exactly what will composite.
            </p>
            <p>
              <strong class="text-ink">One undoable add.</strong> Approving creates
              the composition and, if you choose, places it — a single edit through
              the canonical undo system, not a separate AI history.
            </p>
            <p>
              <strong class="text-ink">Provider-independent.</strong> Any agent
              (Claude Code, Codex, ACP, local) drives the same bridge and the same
              approval card.
            </p>
          </div>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Behavior and architecture"
        caption="HyperFrames is a motion-graphics engine inside NodaVideo, not a second video editor. The Rust timeline owns time; HyperFrames owns pixels."
      >
        <div class="grid gap-4 text-xs leading-5 text-ink/70 sm:grid-cols-3">
          <p>
            <strong class="text-ink">Rust owns timing.</strong> Placement, tracks,
            trimming, selection, snapping and undo stay in the native timeline. A
            composition is a clip like any other; the editor never becomes a CLI.
          </p>
          <p>
            <strong class="text-ink">HyperFrames owns pixels.</strong> A pinned Node
            sidecar drives an isolated Chromium to render HTML/CSS/GSAP into RGBA
            frames. The native GPU renderer composites them over footage.
          </p>
          <p>
            <strong class="text-ink">Canonical undo.</strong> Visual edits become SDK
            patches, the bridge turns them into commands, and NodaVideo applies them
            through one undo stack. There is no second history to reconcile.
          </p>
        </div>
        <div class="mt-6 grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure
            title="Render path"
            caption="Commands flow out to the renderer; RGBA frames flow back to the native compositor. The browser is sandboxed with controlled file and network access."
          >
            <ArchitectureFlow />
          </ReviewFigure>
          <ReviewFigure
            title="Project format"
            caption="Each composition is a folder of source plus a cache. Source is the truth; the cache holds rendered frames and is rebuildable."
          >
            <ProjectTree />
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection title="Companion reviews" caption="Shared shell, timeline and panel components, and the dedicated editor.">
        <div class="flex flex-wrap gap-5 text-sm">
          <a href="25-motion-graphics-editor.html" class="underline underline-offset-4">Motion Graphics Editor</a>
          <a href="23-text-clips.html" class="underline underline-offset-4">Text clips</a>
          <a href="21-file-explorer.html" class="underline underline-offset-4">Media browser</a>
          <a href="17-timeline-clips.html" class="underline underline-offset-4">Timeline clips</a>
          <a href="20-top-bar.html" class="underline underline-offset-4">Desktop shell</a>
        </div>
      </ReviewSection>
    </ReviewPage>
  );
}

/// The AI Assistant proposing a generated composition: a prompt, the assistant’s reply, and
/// an approval card with a rendered preview. Nothing is added until Approve.
function AiApproval(): Markup {
  const comp = COMPOSITION_FIXTURES[0];
  return (
    <section aria-label="AI Assistant" class="flex h-full min-h-0 flex-col bg-surface text-text">
      <NodaPanelHeader title="AI Assistant">
        <NodaButton label="Hide AI Assistant · Cmd-Shift-A" icon="close" iconOnly />
      </NodaPanelHeader>
      <div class="flex h-9 shrink-0 items-center justify-between border-b border-border px-3 text-[11px]">
        <span class="text-muted">Provider</span>
        <span class="flex items-center gap-2">Claude Code<NodaGlyph name="chevron" size={12} /></span>
      </div>
      <div class="min-h-0 flex-1 space-y-3 overflow-y-auto p-3 text-[11px] leading-5">
        <div class="rounded-[3px] bg-hover p-2.5">
          Make a lower third for Amara Okafor, lead director and editor.
        </div>
        <div>
          <p class="mb-1 text-muted">Assistant</p>
          <p>I’ll generate a lower third from the Modern template with her name and role. Review it before it’s added.</p>
        </div>
        <div class="border-l-2 border-accent bg-page p-3">
          <p class="font-medium">Review motion graphic</p>
          <div class="relative mt-2 aspect-video w-full overflow-hidden rounded-[2px]" style={ALPHA_STYLE}>
            <div class="absolute inset-0">
              <CompositionThumb comp={comp} />
            </div>
            <span class="absolute top-1 left-1 grid size-4 place-items-center rounded-[2px] bg-[#8a5a1e] text-[#f0c987]">
              <MotionMark size={10} />
            </span>
          </div>
          <p class="mt-2 text-muted">Lower Third (Modern) · 1920 × 1080 · 4.0 s</p>
          <div class="mt-3 flex gap-2">
            <NodaButton label="Approve" primary />
            <NodaButton label="Reject" />
          </div>
        </div>
      </div>
      <div class="m-3 shrink-0 rounded-[3px] border border-border bg-page p-2.5">
        <p class="mb-3 text-[11px] text-muted">Ask about your edit…</p>
        <div class="flex justify-between text-[9px] text-muted">
          <span>Timeline changes need approval</span>
          <NodaGlyph name="import" size={13} />
        </div>
      </div>
    </section>
  );
}

/// The render path as a light diagram: commands out, RGBA frames back. This is commentary
/// about the system, not app UI, so it stays on the sheet.
function ArchitectureFlow(): Markup {
  const stages = [
    { name: "Rust timeline", detail: "timing · placement · undo" },
    { name: "HyperFrames bridge", detail: "commands · SDK patches" },
    { name: "Node sidecar", detail: "pinned deps · sandboxed" },
    { name: "Chromium", detail: "HTML · CSS · GSAP" },
  ];
  return (
    <div class="rounded-[6px] border border-rule bg-white p-4">
      <div class="flex items-stretch gap-1.5">
        {stages.map((stage, index) => (
          <>
            <div class="flex-1 rounded-[4px] border border-rule bg-sheet p-2.5">
              <p class="text-[11px] font-semibold text-ink">{stage.name}</p>
              <p class="mt-1 text-[9px] leading-4 text-ink/55">{stage.detail}</p>
            </div>
            {index < stages.length - 1 && <span class="self-center text-[13px] text-ink/40">→</span>}
          </>
        ))}
      </div>
      <div class="mt-3 flex items-center gap-2 border-t border-rule pt-3 text-[10px] text-ink/60">
        <span class="text-[13px] text-ink/40">←</span>
        <span>RGBA frames return to the native GPU renderer and composite over footage.</span>
      </div>
    </div>
  );
}

/// The on-disk project format as a light monospace tree: source is the truth, cache is
/// rebuildable. Commentary, not app UI.
function ProjectTree(): Markup {
  const lines: [string, string][] = [
    ["compositions/", "dir"],
    ["  speaker-card/", "dir"],
    ["    index.html", "src"],
    ["    styles.css", "src"],
    ["    animations.js", "src"],
    ["    assets/", "dir"],
    ["  intro-title/", "dir"],
    ["cache/", "cache"],
    ["  speaker-card/  (rendered frames)", "cache"],
    ["project.json  (clips reference compositions)", "meta"],
  ];
  const color: Record<string, string> = {
    dir: "text-ink",
    src: "text-ink/70",
    cache: "text-ink/45",
    meta: "text-ink/70",
  };
  return (
    <div class="rounded-[6px] border border-rule bg-white p-4 font-mono text-[11px] leading-6">
      {lines.map(([text, kind]) => <p class={color[kind]}>{text}</p>)}
    </div>
  );
}