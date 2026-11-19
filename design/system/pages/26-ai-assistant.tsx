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
  LimitationCard,
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
      subtitle="Target architecture for a full-length assistant window: a sessions sidebar, wide thread, bottom composer and integrated terminal dock. The current editor already provides providers, streamed conversations, approvals, persisted sessions, skills and response actions. Sections for attachments, terminal tabs and tool inspection specify future behavior; they do not claim those backends already ship. Every editor mutation stays behind the host-owned approval gateway."
    >
      <div class="mb-3 flex flex-wrap items-center justify-between gap-2 text-xs text-ink/70">
        <span>Window ≥1080px · sessions sidebar 260px · {TOOL_COUNT} MCP tools · 7 providers · 6 skills</span>
        <span>Existing state first · proposed infrastructure labeled explicitly</span>
      </div>

      <ReviewSection
        title="A window of its own"
        caption="Target terminal-open window. The terminal shell, tabs and shortcut require a real PTY integration before this state ships; it is a planned layout, not a claim of terminal support. The existing editor approval gateway remains the only editor mutation path."
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
          <ReviewFigure title="Error" caption="A failed action carries the actual error and only the recovery affordances its saved operation supports.">
            <AssistantWindow state="error" />
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
        caption="Attachment chips and any warning sit above the field; the field grows with the message; the toolbar stays below it. Attachments persist as references, not copies. Each provider either accepts its documented input format or receives explicit file-reference context; unsupported inputs are rejected with a concrete error rather than silently dropped."
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
          <ReviewFigure title="Attachment limit" caption="A 20 MB total selection-size limit is enforced before send. These are text path references, not binary uploads; media stays at its original location.">
            <Specimen width={560}>
              <Composer state="empty" warning="File references exceed the 20 MB total selection limit. Remove one or send fewer files." attachments={[{ name: "interview.mov", kind: "video", size: "412 MB" }]} />
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
            <p><strong class="text-ink">Existing feature.</strong> Sessions are stored with the project and restored when it reopens. Provider, model and transcript entries travel with the conversation.</p>
            <p><strong class="text-ink">The editor still owns state.</strong> A session is a view over the one timeline. Approved host gateway edits apply in the Edit workspace; there is no separate agent document.</p>
            <p><strong class="text-ink">Future navigation.</strong> Search, session tabs and project switching are proposed additions; the current sidebar supports history and new chat.</p>
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
        caption="Only tools gated by the MCP metadata require a NodaVideo editor approval. Provider command/file approvals are separate; changing autonomy does not grant host bypass permissions. Before/after details come from actual request parameters, never inferred values."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-[repeat(3,340px)]">
          <ReviewFigure title="Trim" caption="A single-clip trim with mono timecodes. The most common gate.">
            <Specimen width={340}>
              <ApprovalCard title="Review timeline change" detail="Trim the end of the opening video clip." tool="trim_clip" diff={<DiffLine before="00:00:18:12" after="00:00:08:00" />} />
            </Specimen>
          </ReviewFigure>
          <ReviewFigure title="Delete" caption="The canonical delete tool requires an explicit editor approval before executing; rejection changes nothing.">
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
          <ReviewFigure title="Export · editor-owned" caption="Export runs only in the editor process; the assistant gateway cannot produce an MP4. This state directs the user to the real action instead of pretending export succeeded.">
            <Specimen width={340}>
              <LimitationCard title="Export is unavailable here" detail="Use Export in the editor's menu. This tool is registered for host integrations, not executed by the assistant gateway." />
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
        caption="The assistant selects one production skill at a time. Choosing another saves the current transcript and starts a new chat; the active skill remains visible in the composer toolbar. These six production skills expose provider-agnostic instructions, not executable code."
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
        caption={`Target inspection of the canonical MCP gateway: ${TOOL_COUNT} tools across project, timeline, captions, changesets, HyperFrames and the editor-owned export schema. Each is marked by mutation/approval metadata. This is a proposed inspector; no enable switches or extra-server controls appear until their enforcement and configuration backends exist.`}
      >
        <div class="grid items-start gap-x-5 gap-y-6 md:grid-cols-[320px_minmax(0,1fr)]">
          <ReviewFigure title="The tool browser" caption="Target inspector for actual tool names, mutation status and approval requirements. Read/view only until per-tool policy is implemented.">
            <div class="h-[560px]"><ToolBrowser /></div>
          </ReviewFigure>
          <div class="max-w-[460px] space-y-3 self-start text-xs leading-5 text-ink/70">
            <p><strong class="text-ink">Read vs write.</strong> Read tools inspect project, timeline and media; write tools change them. The badge makes the distinction visible before a tool runs.</p>
            <p><strong class="text-ink">Known limitation.</strong> <span class="font-mono">export_video</span> is described by the canonical schema but executions remain owned by the editor. The assistant reports that fact instead of claiming an export.</p>
            <p><strong class="text-ink">Inspection only.</strong> Actual enablement and additional-server configuration require an authorized host policy layer; the proposed browser does not draw those controls yet.</p>
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
        caption="Proposed infrastructure: a real PTY shell opened from the status bar or Ctrl+`, with later Codex/Claude tabs. Native CLI output is display-only. Interactive agent launches still require an approved MCP connection and live protocol verification before they can share NodaVideo's approval cards."
      >
        <div class="grid gap-x-5 gap-y-6 md:grid-cols-[repeat(2,620px)]">
          <ReviewFigure title="Codex running · target" caption="Proposed terminal with a verified provider connection. Command/file approvals remain distinct from MCP editor mutations; the editor still owns its own gate.">
            <Terminal state="codex" />
          </ReviewFigure>
          <ReviewFigure title="Claude Code · target" caption="A proposed integration, not current support. An interactive CLI must not be treated as a second way to execute reported editor tool activity.">
            <Terminal state="claude" />
          </ReviewFigure>
          <ReviewFigure title="Plain shell · target" caption="A future real zsh PTY at the project root with correct terminal input/output and ownership—not a fake chat transcript.">
            <Terminal state="shell" />
          </ReviewFigure>
          <div class="max-w-[420px] space-y-3 self-start text-xs leading-5 text-ink/70">
            <p><strong class="text-ink">Separate processes, explicit gate.</strong> Native CLI tool activity is not a host editor command. Only the shared MCP executor may mutate the timeline; native approvals remain command/file approvals.</p>
            <p><strong class="text-ink">One editor undo stack.</strong> Every accepted host edit joins NodaVideo's existing timeline history. A terminal session never invents a second editor document or bypass permission path.</p>
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