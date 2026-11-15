import type { Child, Markup } from "@design/jsx/jsx-runtime";
import {
  ReviewFigure,
  ReviewPage,
  ReviewSection,
} from "../components/noda-shell.tsx";
import {
  ApprovalCard,
  AssistantMessage,
  AssistantWindow,
  AttachmentChip,
  AutonomyMenu,
  Composer,
  ComposerContext,
  CompositionResultCard,
  DiffLine,
  ErrorCard,
  ModelMenu,
  PROVIDERS,
  ProviderDot,
  SessionsSidebar,
  SKILLS,
  SkillCard,
  SkillMenu,
  Terminal,
  ThoughtBlock,
  ToolBrowser,
  ToolCallCard,
  ToolRunRow,
  TOOL_COUNT,
  UserMessage,
} from "../components/assistant.tsx";

export default function AiAssistantPage(): Markup {
  return (
    <ReviewPage
      title="The AI Assistant"
      subtitle="A full-length window of its own, not a dock. Tool output, plans and motion results need room, so the assistant gets the whole window: a sessions sidebar, a wide thread, a bottom composer whose attach / model / autonomy controls sit below the field, an integrated terminal dock and a status bar. The editor's timeline stays in the Edit workspace; every change the agent proposes here is approved there and undoable with one stack. One AgentProvider abstraction serves Claude Code, Codex, Gemini CLI, Antigravity and an offline Mock, all with the same tools."
    >
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2 text-xs text-ink/70">
        <span>Window ≥1080px · sessions sidebar 260px · {TOOL_COUNT} MCP tools · 5 providers · 6 skills</span>
        <span>Native chrome · indigo accent · amber motion results</span>
      </div>

      <ReviewSection
        title="A window of its own"
        caption="The whole window is the assistant. Sessions on the left, the conversation in the middle, the composer pinned to the bottom with its controls under the field, and a terminal dock that opens from the status bar when a CLI agent needs room. Nothing is squeezed into a 300px rail: a turn can show reasoning, a tool run, an approval and a rendered composition without scrolling sideways."
      >
        <div class="grid gap-6">
          <ReviewFigure
            title="Conversation · terminal open"
            caption="A completed turn with an edit plan awaiting approval and a HyperFrames result, while the terminal dock below runs a shell. The status bar lights the Terminal toggle and shows the gateway revision."
          >
            <AssistantWindow state="chat" terminal />
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="The window, state by state"
        caption="First run centres the composer under a single question and says there are no sessions yet. An approval gates a trim and dims the composer until decided. Streaming keeps the thread live, swaps send for stop, and tints autonomy amber when a plan runs unattended."
      >
        <div class="grid gap-6">
          <ReviewFigure title="Empty" caption="No sessions, no invented history. The composer is ready; send is disabled until there is text.">
            <AssistantWindow state="empty" />
          </ReviewFigure>
          <ReviewFigure title="Approval" caption="A destructive edit waits for a decision. The status bar counts the pending approval; the composer dims so nothing is sent mid-decision.">
            <AssistantWindow state="approval" />
          </ReviewFigure>
          <ReviewFigure title="Streaming" caption="Reasoning expanded, a tool run reading files, a running tool. Send becomes stop; the context line reports Working…">
            <AssistantWindow state="streaming" />
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="The composer, up close"
        caption="Attachment chips and any warning sit above the field; the field grows with the message; the toolbar stays below it — attach, skill, model with reasoning effort, autonomy, then send or stop at the right. Controls under the text, never above it. Attachments are referenced, not copied."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure title="Empty" caption="Placeholder text, send disabled. Attach, skill, model and autonomy are always reachable below the field.">
            <Specimen width={560}><Composer state="empty" /><div class="mt-2"><ComposerContext /></div></Specimen>
          </ReviewFigure>
          <ReviewFigure title="Typing" caption="A caret and an active Send. Enter sends; the model button names the current agent and effort.">
            <Specimen width={560}><Composer state="typing" /><div class="mt-2"><ComposerContext /></div></Specimen>
          </ReviewFigure>
          <ReviewFigure title="Attachments + skill" caption="Referenced clips above the field; the skill button takes the accent when set. The toolbar does not move.">
            <Specimen width={560}>
              <Composer state="typing" skill={SKILLS[0]} attachments={[{ name: "interview.mov", kind: "video", size: "412 MB" }, { name: "notes.txt", kind: "file", size: "2 KB" }]} />
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Streaming" caption="Send becomes stop (Esc). Autonomy tints amber in Autonomous; destructive edits still gate.">
            <Specimen width={560}><Composer state="streaming" mode="autonomous" provider="codex" /></Specimen>
          </ReviewFigure>
          <ReviewFigure title="Disabled" caption="While a decision is pending the composer dims and send is disabled; the controls stay visible but inert.">
            <Specimen width={560}><Composer state="disabled" /></Specimen>
          </ReviewFigure>
          <ReviewFigure title="Attachment limit" caption="A honest warning above the field when attachments exceed the limit, in the interface's voice.">
            <Specimen width={560}>
              <Composer state="empty" warning="Attachments must total 20 MB or less." attachments={[{ name: "interview.mov", kind: "video", size: "412 MB" }]} />
            </Specimen>
          </ReviewFigure>
        </div>
        <div class="mt-6 grid gap-x-5 gap-y-6 md:grid-cols-2">
          <ReviewFigure title="Attachment chips" caption="Kind icon, name, size and remove. Video, audio, image and generic files share one shape.">
            <Specimen width={560}>
              <div class="flex flex-wrap gap-1.5">
                <AttachmentChip name="interview.mov" kind="video" size="412 MB" />
                <AttachmentChip name="city-ambience.wav" kind="audio" size="38 MB" />
                <AttachmentChip name="end-title.png" kind="image" size="1.2 MB" />
                <AttachmentChip name="notes.txt" kind="file" size="2 KB" />
              </div>
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Context line" caption="Which project and sequence the agent acts on, and whether it is working. References, never copies.">
            <Specimen width={560}><ComposerContext working /></Specimen>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Model, effort and autonomy"
        caption="Both choosers open from the composer toolbar below the field. One chooser switches the whole agent behind a single AgentProvider abstraction; the other tunes how often it asks. Ready, sign-in and offline are distinct, and destructive edits gate in every autonomy mode."
      >
        <div class="grid items-start gap-x-5 gap-y-6 md:grid-cols-[300px_300px_minmax(0,1fr)]">
          <ReviewFigure title="The provider chooser" caption="Every registered agent with transport, model and readiness; the selected one is checked.">
            <ModelMenu selected="claude" />
          </ReviewFigure>
          <ReviewFigure title="The autonomy chooser" caption="Assist proposes, Collaborate applies safe edits, Autonomous runs a plan. The footer repeats the invariant.">
            <AutonomyMenu selected="collaborate" />
          </ReviewFigure>
          <div class="max-w-[420px] space-y-3 self-start text-xs leading-5 text-ink/70">
            <p><strong class="text-ink">One abstraction.</strong> The editor talks to an <span class="font-mono">AgentProvider</span>, not to a specific CLI. Adding a provider adds a row here, not a new code path.</p>
            <p><strong class="text-ink">Same tools everywhere.</strong> Whether the agent is Claude Code or the offline Mock, it reaches the editor through the same MCP gateway and the same {TOOL_COUNT} tools.</p>
            <p><strong class="text-ink">Honest readiness.</strong> A provider that is not installed or signed in says so; it is never silently skipped.</p>
            <div class="flex flex-wrap items-center gap-3 rounded-[6px] border border-rule bg-white p-3">
              {PROVIDERS.map((p) => (
                <span class="flex items-center gap-1.5 text-[11px] text-ink/70"><ProviderDot provider={p} size={8} />{p.name}</span>
              ))}
            </div>
          </div>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Sessions"
        caption="Conversations live with the project, grouped by day with the provider that ran them. New chat sits on top; an empty project says there are no sessions instead of inventing threads. Reopening the project restores the sessions that shaped it."
      >
        <div class="grid items-start gap-x-5 gap-y-6 md:grid-cols-[260px_260px_minmax(0,1fr)]">
          <ReviewFigure title="With history" caption="The active session is filled; each row carries its provider dot and time.">
            <div class="h-[420px] overflow-hidden rounded-[6px] border border-border"><SessionsSidebar /></div>
          </ReviewFigure>
          <ReviewFigure title="Empty" caption="First run: no sessions yet, and the new-chat affordance still present.">
            <div class="h-[420px] overflow-hidden rounded-[6px] border border-border"><SessionsSidebar empty /></div>
          </ReviewFigure>
          <div class="max-w-[420px] space-y-3 self-start text-xs leading-5 text-ink/70">
            <p><strong class="text-ink">Per-project storage.</strong> Sessions are part of the project folder, so the history travels with the edit.</p>
            <p><strong class="text-ink">The editor still owns state.</strong> A session is a view over the one timeline. Approving a plan applies it in the Edit workspace; there is no separate agent document.</p>
            <p><strong class="text-ink">One undo stack.</strong> Whether an edit came from a session or a CLI agent in the terminal, Cmd-Z reverses it in order.</p>
          </div>
        </div>
      </ReviewSection>

      <ReviewSection
        title="The vocabulary of a turn"
        caption="Every message is built from a small set of parts. User turns are quiet indigo bubbles; assistant turns carry provider, runtime and elapsed time on one line. Reasoning and tool runs collapse to a single quiet line; tool calls, composition results and errors are structured cards, not chat text — so the user sees exactly what the agent did to the timeline."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-[repeat(3,340px)]">
          <ReviewFigure title="User turn" caption="A right-aligned bubble with a referenced attachment and a Show more affordance for long prompts.">
            <Specimen width={340}>
              <UserMessage
                text="Cut the silence from the interview and caption it for social. Keep the best takes and match the grade to the opening."
                attachments={[{ name: "interview.mov", kind: "video", size: "412 MB" }]}
                truncated
              />
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Assistant turn" caption="Provider, runtime and elapsed time, then prose, then turn actions. No avatar tile — the thread is wide.">
            <Specimen width={340}>
              <AssistantMessage provider="codex" time="08:12">
                <p>I found six silent gaps. Here's the cut plan — approve it and I'll apply the cuts atomically, then generate captions.</p>
              </AssistantMessage>
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Reasoning" caption="Collapsed it is one quiet line with a duration; expanded it shows the thought. Never louder than the answer.">
            <Specimen width={340}>
              <div class="space-y-2">
                <ThoughtBlock text="Analyzing the interview audio for silence regions." />
                <ThoughtBlock text="The interview is 6:12 with 1:48 of silence. I'll propose six cuts, then caption the kept takes at 9:16." expanded duration="3.4s" />
              </div>
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Tool runs" caption="A collapsed run stays one line; expanded it lists the individual reads and commands with a tree rail.">
            <Specimen width={340}>
              <div class="space-y-2">
                <ToolRunRow label="Ran 2 commands · Read ref1.png" />
                <ToolRunRow label="Reading 2 files" icon="file" expanded>
                  <p class="border-l border-border pl-3">Read <span class="font-mono text-accent-soft">system/components/assistant.tsx</span></p>
                  <p class="border-l border-border pl-3">Read <span class="font-mono text-accent-soft">system/components/noda-shell.tsx</span></p>
                </ToolRunRow>
              </div>
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Tool calls" caption="The MCP tool name, a status pill, arguments and result. Running, done and failed read at a glance.">
            <Specimen width={340}>
              <div class="space-y-2">
                <ToolCallCard tool="analyze_media" status="running" icon="mic" args="interview.mov · audio: true" />
                <ToolCallCard tool="trim_clip" status="done" icon="scissors" args="street-at-dusk.mov · end 00:00:08:00" result="Applied · undo available" />
                <ToolCallCard tool="render_composition" status="failed" icon="sparkle" result="Template missing a font file." />
              </div>
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Motion result" caption="A HyperFrames composition the agent created: rendered over alpha, amber identity, Add to Timeline and Open in Editor.">
            <Specimen width={340}><CompositionResultCard /></Specimen>
          </ReviewFigure>
          <ReviewFigure title="Failure" caption="What went wrong and how to recover, in the interface's voice. Retry re-runs the tool; Dismiss clears it.">
            <Specimen width={340}>
              <ErrorCard title="Export couldn't start" detail="No video tracks in the sequence. Add a clip, then export again." />
            </Specimen>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Approval and control"
        caption="The user owns the timeline. Splits, deletes, exports, multi-step plans and composition placement surface as approval cards with a concrete before → after. Approve applies atomically with one undo; Reject changes nothing. A standing 'always allow' rule and three autonomy modes tune how often the agent asks — but destructive edits always ask, in every mode."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-[repeat(3,340px)]">
          <ReviewFigure title="Trim" caption="A single-clip trim with mono timecodes. The most common gate.">
            <Specimen width={340}>
              <ApprovalCard title="Review timeline change" detail="Trim the end of the opening video clip." tool="trim_clip" diff={<DiffLine before="00:00:18:12" after="00:00:08:00" />} />
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Delete" caption="Removing a clip is destructive, so it always asks — even in Autonomous.">
            <Specimen width={340}>
              <ApprovalCard title="Delete clip" detail="Remove the selected clip from Video 1. This can be undone." tool="delete_clip" diff={<DiffLine before="market-walk.mp4 · 00:00:24:08" after="removed" />} />
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Place composition" caption="A motion graphic lands on a video track at a frame position, with alpha over the tracks below.">
            <Specimen width={340}>
              <ApprovalCard title="Add to timeline" detail="Place the speaker card on Video 2 at the playhead." tool="add_composition_to_timeline" diff={<DiffLine before="Video 2 · empty" after="00:00:06:12 · 4.0s" />} />
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Edit plan" caption="A changeset bundles many commands into one atomic, undoable decision.">
            <Specimen width={340}>
              <ApprovalCard
                title="Review edit plan"
                detail="Six smart cuts remove 1:48 of silence. Captions follow."
                tool="propose_changeset"
                diff={
                  <div class="space-y-1 rounded-[4px] bg-hover px-2 py-1.5 font-mono text-[10px] text-subtle">
                    <p>6 × split_clip + delete_clip</p>
                    <p class="text-muted">06:12 → 04:24 · atomic · one undo</p>
                  </div>
                }
              />
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Export" caption="Export renders the sequence to a file; it asks before writing to disk.">
            <Specimen width={340}>
              <ApprovalCard title="Export sequence" detail="Render Sequence 01 to an MP4 in the project folder." tool="export_video" diff={<DiffLine before="Sequence 01 · 00:00:42:00" after="city-after-hours.mp4 · 1080p" />} />
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Decided" caption="After a decision the card records the outcome: applied with undo, or rejected with no change.">
            <Specimen width={340}>
              <div class="space-y-2">
                <ApprovalCard title="Review timeline change" detail="Trim the opening clip." tool="trim_clip" diff={<DiffLine before="00:00:18:12" after="00:00:08:00" />} decided="approved" />
                <ApprovalCard title="Delete clip" detail="Remove the selected clip." tool="delete_clip" diff={<DiffLine before="market-walk.mp4" after="removed" />} decided="rejected" always={false} />
              </div>
            </Specimen>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Skills"
        caption="Skills are provider-agnostic production capabilities loaded from skills/. Picking one shapes the conversation and the tools the agent reaches for. They are instructions, not code, so every provider can use them — and the active skill is visible in the composer toolbar."
      >
        <div class="grid items-start gap-x-5 gap-y-6 md:grid-cols-[300px_minmax(0,1fr)]">
          <ReviewFigure title="The skill picker" caption="Opened from the composer's skill button. Each skill names what it does; the active one is checked.">
            <SkillMenu selected="smart-cuts" />
          </ReviewFigure>
          <ReviewFigure title="The skill library" caption="The six production skills. Each maps to a SKILL.md and a set of MCP tools; selecting one primes the assistant for that job.">
            <div class="flex flex-wrap gap-3">
              {SKILLS.map((skill, i) => <SkillCard skill={skill} active={i === 0} />)}
            </div>
          </ReviewFigure>
        </div>
      </ReviewSection>

      <ReviewSection
        title="Tools and plugins"
        caption={`The assistant reaches the editor through an MCP gateway: ${TOOL_COUNT} tools across project & media, timeline, captions, changesets, HyperFrames and export. Each is marked read or write, and the destructive ones carry an approval shield. Per-tool switches and additional MCP servers make the surface extensible without changing the editor.`}
      >
        <div class="grid items-start gap-x-5 gap-y-6 md:grid-cols-[320px_minmax(0,1fr)]">
          <ReviewFigure title="The tool browser" caption="Opened from the composer or the status bar. Groups collapse; each tool shows read/write, an approval shield and an enable switch.">
            <div class="h-[560px]"><ToolBrowser /></div>
          </ReviewFigure>
          <div class="max-w-[460px] space-y-3 self-start text-xs leading-5 text-ink/70">
            <p><strong class="text-ink">Read vs write.</strong> Read tools inspect project, timeline and media; write tools change them. The badge makes the distinction visible before a tool runs.</p>
            <p><strong class="text-ink">The approval shield.</strong> <span class="font-mono">split_clip</span>, <span class="font-mono">delete_clip</span>, <span class="font-mono">propose_changeset</span>, <span class="font-mono">add_composition_to_timeline</span> and <span class="font-mono">export_video</span> always gate behind a card.</p>
            <p><strong class="text-ink">HyperFrames is a tool group.</strong> Creating, rendering, validating and placing compositions are MCP tools like any other — the assistant makes motion graphics through the same gateway.</p>
            <p><strong class="text-ink">Extensible.</strong> Additional MCP servers can be added; the editor's own tools stay canonical and revision-safe.</p>
            <div class="rounded-[6px] border border-rule bg-white p-3 text-[11px] text-ink/70">
              <p class="mb-2 font-semibold text-ink">Gateway</p>
              <p class="font-mono">nodavideo · hyperframes</p>
              <p class="mt-1">State-aware · revision-safe · atomic rollback</p>
            </div>
          </div>
        </div>
      </ReviewSection>

      <ReviewSection
        title="The integrated terminal"
        caption="For users who prefer the CLI, Codex and Claude Code run in the terminal dock with a tab per agent — there is room for it now that the assistant owns the window. Self-executing agents stream their own output here, but their timeline edits still surface as NodaVideo approval cards, so the editor keeps the canonical gate and the single undo stack. A plain shell tab is always available."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-[repeat(2,620px)]">
          <ReviewFigure title="Codex running" caption="The app-server agent analyzes media and previews a changeset, then requests approval in NodaVideo. The terminal narrates; the editor decides.">
            <Terminal state="codex" />
          </ReviewFigure>
          <ReviewFigure title="Claude Code · approval" caption="Claude creates and renders a composition, then asks to place it. The approval appears in the Assistant window, not as a terminal y/N.">
            <Terminal state="claude" />
          </ReviewFigure>
          <ReviewFigure title="Plain shell" caption="A normal zsh tab for git, scripts and launching agents by hand. The terminal is a real shell, not a chat box.">
            <Terminal state="shell" />
          </ReviewFigure>
          <div class="max-w-[420px] space-y-3 self-start text-xs leading-5 text-ink/70">
            <p><strong class="text-ink">Two ways in, one gate.</strong> Chat and terminal are two surfaces over the same MCP gateway. A mutation proposed in the terminal still raises an approval card in the editor.</p>
            <p><strong class="text-ink">One undo stack.</strong> Whether an edit came from a session or a CLI agent, Cmd-Z reverses it in order. There is no second history to reconcile.</p>
            <p><strong class="text-ink">Tabs per agent.</strong> Codex, Claude Code and a shell coexist; the user can watch one agent while working in another.</p>
          </div>
        </div>
      </ReviewSection>

      <ReviewSection title="Companion reviews" caption="The motion graphics the assistant creates, and the shell it lives in.">
        <div class="flex flex-wrap gap-5 text-sm">
          <a href="24-motion-graphics.html" class="underline underline-offset-4">Motion graphics integration</a>
          <a href="25-motion-graphics-editor.html" class="underline underline-offset-4">Motion Graphics Editor</a>
          <a href="20-top-bar.html" class="underline underline-offset-4">Desktop shell</a>
          <a href="11-context-menus.html" class="underline underline-offset-4">Context menus</a>
        </div>
      </ReviewSection>
    </ReviewPage>
  );
}

/// A dark specimen frame for an isolated component, on the panel background.
function Specimen({ children, width = 340 }: { children: Child; width?: number }): Markup {
  return (
    <div class="rounded-[6px] border border-border bg-surface p-3 text-text" style={`width:${width}px`}>
      {children}
    </div>
  );
}