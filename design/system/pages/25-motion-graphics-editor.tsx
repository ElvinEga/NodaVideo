import type { Markup } from "@design/jsx/jsx-runtime";
import {
  ReviewFigure,
  ReviewPage,
  ReviewSection,
} from "../components/noda-shell.tsx";
import {
  CompositionTimeline,
  EditorCanvas,
  EditorCodePane,
  EditorProperties,
  LayersTree,
  MotionEditorWindow,
} from "../components/motion-editor.tsx";

export default function MotionGraphicsEditorPage(): Markup {
  return (
    <ReviewPage
      title="The Motion Graphics Editor"
      subtitle="A dedicated window, opened from the Inspector like Export or Transcription — not a workspace tab. Five areas: layers on the left, the canvas (or code) in the center, properties and animation on the right, the composition’s own timeline along the bottom, and a toolbar on top. Edit visually or in HTML/CSS/GSAP; both write to one source. NodaVideo still owns undo: visual edits become SDK patches, the bridge turns them into commands, and the native timeline applies them."
    >
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2 text-xs text-ink/70">
        <span>Dedicated window · 1180 × 720 · Back to edit returns to the workspace</span>
        <span>One source, two editing levels, canonical undo</span>
      </div>

      <ReviewSection
        title="A dedicated window"
        caption="The whole editor at its real size. The toolbar names the composition and its template, size and frame rate, with Save and Export. Below it the five areas work together: select a layer, see it on the canvas, edit its properties, and scrub its animation. Back to edit closes the window and returns to the timeline with the composition saved."
      >
        <div class="grid">
          <ReviewFigure
            title="Visual mode"
            caption="Layers · canvas · properties across the middle, the composition timeline along the bottom. The selected element (Name) shows a bounding box on the canvas and drives the properties panel."
          >
            <MotionEditorWindow mode="visual" />
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="The areas up close"
        caption="The three middle areas at their real widths. The layers tree lists the composition’s elements with visibility and lock; the canvas renders the composition over a checkerboard at a local time with selection handles; the properties panel edits the selected element’s transform, typography and animation."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-[220px_660px_260px]">
          <ReviewFigure
            title="Layers"
            caption="The composition root and its elements. Selecting one drives the canvas and properties; eye and lock toggle visibility and editing."
          >
            <div class="h-[464px] w-[220px] border border-border">
              <LayersTree />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="Canvas"
            caption="A rendered frame over alpha at a local time. The selected element shows resize handles and a name tag; the transport scrubs the composition."
          >
            <div class="h-[464px] w-[660px] border border-border">
              <EditorCanvas />
            </div>
          </ReviewFigure>
          <ReviewFigure
            title="Properties"
            caption="Transform, typography and animation for the selected element. Visual edits write back to the HTML source through the SDK."
          >
            <div class="h-[464px] w-[260px] border border-border">
              <EditorProperties />
            </div>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="The code level"
        caption="The center area swaps from the canvas to a code editor: HTML, CSS and JS (GSAP) tabs over the composition source, with Validate. The source stays the source of truth — the visual editor writes through the SDK, and code edits re-render the canvas. Only the center changes; layers, properties and the composition timeline stay."
      >
        <div class="grid items-start gap-x-5 gap-y-6 md:grid-cols-[660px_minmax(0,1fr)]">
          <div class="grid">
            <ReviewFigure
              title="HTML · CSS · JS"
              caption="Line-numbered source with syntax color. Validate checks the composition; the status bar reports elements, size and duration."
            >
              <div class="h-[464px] w-[660px] border border-border">
                <EditorCodePane />
              </div>
            </ReviewFigure>
          </div>
          <div class="max-w-[420px] space-y-3 self-start text-xs leading-5 text-ink/70">
            <p>
              <strong class="text-ink">One source.</strong> The composition is
              <span class="font-mono"> index.html</span>,
              <span class="font-mono"> styles.css</span> and
              <span class="font-mono"> animations.js</span>. Visual and code editing
              are two views of the same files, never divergent copies.
            </p>
            <p>
              <strong class="text-ink">Seekable animation.</strong> GSAP timelines are
              paused and registered with HyperFrames, so the renderer can seek to any
              local time and produce a deterministic frame.
            </p>
            <p>
              <strong class="text-ink">Validate before render.</strong> Validation
              catches broken markup or scripts before the sidecar renders, so a bad
              edit never silently produces empty frames.
            </p>
          </div>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Composition time vs sequence time"
        caption="The bottom timeline is the composition’s own animation: one row per element, keyframe diamonds, a local playhead and local timecode. It is not the native timeline. The native clip owns where the whole composition sits in the sequence, how long it plays and how it is trimmed; this timeline owns how the elements move within it."
      >
        <div class="grid">
          <ReviewFigure
            title="Composition timeline"
            caption="Local time 00:00:01:12 of 00:00:04:00 at 24 fps. Each element animates in with keyframes; the playhead scrubs the composition, independent of the sequence playhead."
          >
            <div class="h-[200px] w-[1140px] border border-border">
              <CompositionTimeline />
            </div>
          </ReviewFigure>
        </div>
        <p class="mt-4 max-w-[1000px] text-xs leading-5 text-ink/70">
          <strong class="text-ink">Two clocks, one owner each.</strong> Scrubbing here
          changes only the previewed local time. Trimming or moving the clip on the
          native timeline changes where the composition appears in the edit — the
          composition’s internal animation is unchanged. This keeps the Rust timeline
          the single authority on sequence time.
        </p>
      </ReviewSection>

      <ReviewSection
        title="Undo and ownership"
        caption="The editor never keeps its own history. Every change — visual or code — becomes a command in NodaVideo’s canonical undo stack, so Cmd-Z works the same here as on the timeline."
      >
        <div class="grid gap-4 text-xs leading-5 text-ink/70 sm:grid-cols-3">
          <p>
            <strong class="text-ink">Visual edits are patches.</strong> Moving a layer
            or changing a property produces an SDK patch describing the change, not a
            direct mutation of the source.
          </p>
          <p>
            <strong class="text-ink">The bridge translates.</strong> The HyperFrames
            bridge turns patches into editor commands. NodaVideo applies them through
            the same transactional API as every other edit.
          </p>
          <p>
            <strong class="text-ink">One undo stack.</strong> There is no second
            history to reconcile. Undo and redo reverse commands in order, whether
            they came from the canvas, the code editor or the AI Assistant.
          </p>
        </div>
        <div class="mt-6">
          <div class="grid">
            <ReviewFigure
              title="Edit path"
              caption="A change flows from either editing level through the bridge into the canonical undo stack, then re-renders. The source files are written only after the command is applied."
            >
              <UndoFlow />
            </ReviewFigure>
          </div>
        </div>
      </ReviewSection>

      <ReviewSection title="Companion reviews" caption="The timeline integration and the shared shell.">
        <div class="flex flex-wrap gap-5 text-sm">
          <a href="24-motion-graphics.html" class="underline underline-offset-4">Motion graphics integration</a>
          <a href="13-export-window.html" class="underline underline-offset-4">Export window</a>
          <a href="14-transcription-window.html" class="underline underline-offset-4">Transcription window</a>
          <a href="20-top-bar.html" class="underline underline-offset-4">Desktop shell</a>
        </div>
      </ReviewSection>
    </ReviewPage>
  );
}

/// The edit path as a light diagram: either editing level produces a change that the bridge
/// turns into a command in the canonical undo stack. Commentary, not app UI.
function UndoFlow(): Markup {
  const steps = [
    { name: "Visual or code edit", detail: "canvas handle · property · source" },
    { name: "SDK patch", detail: "describes the change" },
    { name: "Bridge command", detail: "HyperFrames → editor" },
    { name: "NodaVideo undo stack", detail: "one canonical history" },
  ];
  return (
    <div class="rounded-[6px] border border-rule bg-white p-4">
      <div class="flex items-stretch gap-1.5">
        {steps.map((step, index) => (
          <>
            <div class="flex-1 rounded-[4px] border border-rule bg-sheet p-2.5">
              <p class="text-[11px] font-semibold text-ink">{step.name}</p>
              <p class="mt-1 text-[9px] leading-4 text-ink/55">{step.detail}</p>
            </div>
            {index < steps.length - 1 && <span class="self-center text-[13px] text-ink/40">→</span>}
          </>
        ))}
      </div>
      <div class="mt-3 flex items-center gap-2 border-t border-rule pt-3 text-[10px] text-ink/60">
        <span class="text-[13px] text-ink/40">↺</span>
        <span>Applied commands re-render the composition; Cmd-Z reverses them in order.</span>
      </div>
    </div>
  );
}