# NodaVideo — GPUI Kit / GPUI Desktop Application Guidelines

**Status:** living guideline, first edition 2026-11-08
**Applies to:** `apps/desktop` and every future GPUI view in the NodaVideo workspace
**Pinned stack at time of writing:** `gpui-kit 0.9.0`, `gpui-component 0.9.0`, `gpui-base 0.9.0`, `gpui-pre 0.3.11`, Rust `1.92.0`, edition 2024
**Product:** NodaVideo — *Your timeline. Your creativity. Your AI.* — a native, cross-platform, professional multi-track video editor with pluggable AI agents.

This document translates the GPUI Kit documentation, the GPUI core documentation and the vendored normative guides into rules for building **NodaVideo**. It is a navigation and decision layer, not a substitute for the authoritative sources below.

---

## 0. How to use this document

### 0.1 Authority order (highest first)

When a signature, behavior or design rule conflicts, resolve it in this order:

| Priority | Source | Why |
| --- | --- | --- |
| 1 | `Cargo.lock` and the pinned crate source in the local Cargo registry | The code that actually compiles. `~/.cargo/registry/src/<index>/gpui-kit-0.9.0`, `gpui-component-0.9.0`, `gpui-base-0.9.0` |
| 2 | `AGENTS.md` at the repo root | NodaVideo's architectural contract; overrides default instincts |
| 3 | `skills/gpui-kit-design-guides/references/design-guides.md` | Normative design guide (verbatim copy of `https://gpui-kit.com/docs/design-guides.md`) |
| 4 | `skills/gpui-kit/references/coding-guides.md` | Normative coding guide (verbatim copy of `https://gpui-kit.com/docs/coding-guides.md`) |
| 5 | `skills/gpui-kit/references/*.md` and `references/gpui/*.md` | Component conventions, tested recipes, GPUI mechanism references |
| 6 | `https://gpui-kit.com/docs/`, `/component/`, `https://gpui.rs/` | Current published documentation |
| 7 | This file | Convenient summary; update it when any of the above changes |

**Never invent an API.** Do not translate a React/CSS/older-GPUI example by analogy. Search the pinned source for the real signature before writing UI code. Some published component pages contain mutually inconsistent, stale examples — see §13.

### 0.2 Reading order for a new screen or feature

1. `AGENTS.md` §2 (architectural contract) and §7 (UI direction).
2. Design Guides: "Design thesis", "Start from the task", then the section for the change.
3. Coding Guides: "Architecture at a glance", "Rules for coding agents", then the relevant section.
4. `skills/gpui-kit/references/conventions.md` to pick the component family.
5. The specific component page (`https://gpui-kit.com/component/{name}.md`) **and** the pinned source.
6. This document §6–§8 for the NLE-specific composition and constraints.
7. Finish with the checklists in §12.

### 0.3 Before generating any UI code, write down

Per the normative Design Guides ("Guidance for AI-generated interfaces"), state these four things explicitly:

- the **primary task** of the screen;
- the **state owner** (which `Entity<T>` or feature crate owns what);
- the **component composition** (which standard components, which custom elements, and why);
- the **keyboard path** through the workflow (focus handle, key context, actions, menu/shortcut exposure).

Then name the behavior owner, presentation owner, layout/overflow owner, theme tokens, and the test that would fail if the behavior regressed.

---

## 1. What we are building: NLE-specific constraints

NodaVideo is a **professional video editor first, an AI assistant second**. That ordering has direct UI consequences:

- The AI workspace is a **dockable panel**; hiding it must leave a fully usable editor.
- The user retains complete control of the timeline. AI-proposed changes are previewed and approved, and every mutation — human or AI — goes through the single `EditorCommand` path in `crates/command-system`.
- The UI is a **dark, dense, resizable, dockable professional workspace**: media bin, large preview monitor, inspector, multi-track timeline, status bar, workspace tabs (Edit / Color / Audio / AI Studio).
- GPUI views **render state and dispatch commands**; they must not contain timeline domain logic, FFmpeg calls, decoding, export, transcription or agent I/O.
- **Time is integer frames / rational time**, never floating-point seconds. Conversions happen at the UI, file-format and FFmpeg boundaries only. Frame rate lives on the sequence.
- **HyperFrames is not a second timeline.** It produces motion-graphics compositions; the Rust timeline owns their timing and placement.

Practical rule: if a view needs to change the project, it emits an `EditorCommand`. It never writes the project document, never mutates a clip model directly, and never calls an agent adapter.

---

## 2. Layer model and the one-dependency rule

### 2.1 The layers

| Layer | Crate / namespace | Owns |
| --- | --- | --- |
| GPUI runtime | `gpui_kit::*` (re-export of the `gpui-pre` snapshot) | Entities, windows, elements, layout, focus, actions, rendering, tasks |
| Unstyled behavior | `gpui_kit::base` (`gpui-base`) | Focus, overlays, virtualization, dock model, resizing, semantic tokens, no product colors |
| Styled components | `gpui_kit::component` (`gpui-component`) | 75+ themed components, themes, dock chrome, data table, editor |
| Assets | `gpui_kit::assets` | Default icon source |
| Optional JS host | `gpui-shell` | Not needed for the native editor |
| NodaVideo | `apps/desktop`, `crates/*` | Domain meaning, orchestration, composition, product semantics |

Dependencies point **downward**. A reusable component must not depend on an application screen; `gpui-base` must not depend on the component theme. In NodaVideo terms: `apps/desktop` composes feature crates, but contains little feature logic.

`gpui-pre` is only the crates.io package name for the pinned GPUI snapshot. It is **not** another renderer. Application code depends on `gpui-kit` alone and imports GPUI through `use gpui_kit::*;`.

### 2.2 Pinned toolchain (verified against `Cargo.toml` / `Cargo.lock`)

| Item | Value | Note |
| --- | --- | --- |
| `gpui-kit` | `0.9.0` | Single dependency in `[workspace.dependencies]`; `apps/desktop` uses `gpui-kit.workspace = true` |
| `gpui-pre` / `gpui-pre-platform` | `0.3.11` | Transitive, pinned by Kit. Never override one GPUI package to a different version |
| Rust | `1.92.0` | `oo7 0.6.0` in the Linux GPUI platform dependency requires 1.92 |
| Edition | `2024` | Workspace-level |
| macOS | 15+, Xcode Command Line Tools | `xcode-select --install`; `gpui-kit` already enables `font-kit` on the platform crate |
| Windows | 10+, VS 2022 Build Tools/Community with "Desktop development with C++", MSVC toolchain (not GNU), CMake on `PATH` | |
| Linux | Ubuntu 24.04 packages: `gcc g++ clang libfontconfig-dev libwayland-dev libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev libssl-dev libzstd-dev libasound2-dev vulkan-validationlayers libvulkan1` | Needs a graphical Wayland/X11 session with a working Vulkan driver |

Keep `Cargo.lock` checked in and build with `--locked` in CI. Do not bump GPUI casually; smoke-test macOS, Windows and Linux at every milestone (M0 acceptance test).

The root `Cargo.toml` already applies the recommended dev-profile overrides (`gpui-pre`, `gpui-component`, `gpui-kit`, `gpui-kit-assets`, `gpui-pre-platform`, `rustybuzz`, `taffy`, `ttf-parser` at `opt-level = 3`). These improve runtime responsiveness in debug builds; they do not make compilation faster. Profile overrides only take effect in the workspace root manifest.

An optional `gpui-fast` backend exists (`gpui-kit = { version = "0.9.0", features = ["gpui-fast"] }`). Treat enabling it as an architecture decision: record it in `AGENTS.md` before adopting, because it changes the rendering/layout engine across the whole graph.

### 2.3 Current bootstrap in the repo

`apps/desktop/src/main.rs` is the M0 hello window: `application()` → `.with_assets(assets::Assets)` → `init(cx)` → `open_window(...)` → a `WelcomeView` implementing `Render`, with `Bounds::centered`, `WindowBounds::Windowed`, `TitlebarOptions`, and `cx.theme().background` / `.foreground`. This is the correct starting shape. Everything in §6 onward is the target, not yet existing code.

---

## 3. Bootstrap and window/root ownership

```rust
use gpui_kit::*;

fn main() {
    application()
        .with_assets(assets::Assets)          // 1. register the default icon source
        .run(|cx| {
            init(cx);                          // 2. MUST be first, once, before any component view

            let (window_handle, workspace) = open_window(
                WindowOptions::default(),
                cx,
                |_, cx| cx.new(|_| WorkspaceView), // 3. return the content view, not another Root
            )
            .expect("failed to open workspace window");
        });
}
```

Rules:

- `gpui_kit::open_window` mounts a Base `Root` around the returned view. It returns `(AnyWindowHandle, Entity<V>)`: the window handle selects the window, the `Entity` selects the state to update. Keep both if later work needs them.
- `Root` is the **only** root per window. It owns dialog, sheet, notification, tooltip/menu layers, focus traps, modal focus restoration, and window-scoped text selection. Do not create a root per page, and do not mount overlay layers in application views.
- `window.open_dialog(...)`, `open_alert_dialog(...)`, `open_sheet(...)`, `push_notification(...)` come from `gpui_kit::component::WindowExt`; import that trait explicitly.
- `Window`, `App` and `Context<Self>` are **temporary callback contexts**. Never store `&mut Window`, `&mut App` or `&mut Context<_>`. Retain typed handles instead: `Entity<T>`, `WeakEntity<T>`, `FocusHandle`, scroll handles, `Task`, `Subscription`, `AnyWindowHandle`, domain IDs.
- Handle-based updates can fail after the window closes; check the result. For multi-window work, use the handle of the specific window whose focus or geometry you mean.
- Window-local, non-entity changes need `window.refresh()`; entity changes need `cx.notify()`. Neither belongs unconditionally in `render`.
- Register `window.on_window_should_close(cx, callback)` for unsaved-work confirmation; the application owns that flow. `cx.on_window_closed(...)` returns a `Subscription` — retain it.

---

## 4. GPUI core mental model

### 4.1 Retained state vs. rebuilt element tree

GPUI is retained state with declarative rendering. An `Entity<T>` survives across frames; the element tree returned by `render` is a fresh description of the current frame.

```text
app shell → feature crate (model, commands, views)
              ├─ Entity<Model>       retained state
              └─ Entity<View>        retained view; implements Render
                    └─ element tree  rebuilt for each render
                         └─ RenderOnce values for reusable pieces
```

| Context | Use for |
| --- | --- |
| `Context<Self>` | Mutating the current entity, listeners, `notify`, `emit`, subscriptions, tasks |
| `App` | Globals and entity reads/updates without implying ownership by the rendered element |
| `Window` | Focus, action dispatch, input, element-keyed state, measurement, animation frames for one window |

Layout, prepaint and paint are later phases; use their hooks only when resolved geometry is genuinely required (custom timeline elements are the main NodaVideo case).

### 4.2 Choose the right unit

| Unit | When | Signature |
| --- | --- | --- |
| `RenderOnce` / `IntoElement` | Value-like, caller supplies all inputs, no retained state | `fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement` |
| `Entity<T>` + `Render` | Behavior spanning frames, observation, subscriptions, focus, async work, history, measurement | `fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement` |
| Custom `Element` | Timeline canvas, preview surface: hit-testing, drawing, resolved bounds | `Element` trait with `request_layout` / `prepaint` / `paint` |

Do not turn every visual fragment into an entity. Do not force every component into one template: the ecosystem has semantic elements (`Button`, `Checkbox`, `Tab`), compound behavior roots (`Dialog`, `Popover`, `Select`), and entity-backed systems (`Input`, `Table`, `Tree`, `Dock`).

Stateful components keep an `Entity<State>` in the owning view, created in `new`, never in `render`:

```rust
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::{Entity, Subscription};

struct AssistantPanel {
    prompt: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl AssistantPanel {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let prompt = cx.new(|cx| InputState::new(window, cx).placeholder("Describe an edit…"));
        let subscription = cx.subscribe_in(&prompt, window, |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                // derive presentation from the controlled state
                cx.notify();
            }
        });
        Self { prompt, _subscriptions: vec![subscription] }
    }
}
```

Retaining the `Subscription` is mandatory: dropping a constructor-local variable unregisters it. Import capabilities explicitly — `ButtonVariants`, `Sizable`, `Disableable`, `Selectable`, `ActiveTheme`, `WindowExt`, `Styled`, `ParentElement` are often extension traits, not inherent methods.

### 4.3 Stable identity

An `ElementId` is part of behavior. It keys element-local state, focus, measurement and animation identity.

- Derive IDs from **domain identity**, never from list positions, translated labels, or fresh random values in `render`: `Button::new(("delete-clip", clip.id))`, `.id(("clip", clip_id))`.
- Namespace child IDs with the owning object when a control repeats across tracks.
- A changed ID means a changed UI identity — treat the reset as deliberate.
- NodaVideo domain IDs (`ClipId`, `TrackId`, `SessionId`, `ProjectId`) are opaque newtypes; map them deliberately into `ElementId`s.
- Preserve selection by domain identity, not index, across filtering, reordering and virtualization.

### 4.4 Notify, emit, subscribe

| Want | Use |
| --- | --- |
| Re-render after a state change | `cx.notify()` on the narrowest owning entity |
| Announce a semantic fact | `cx.emit(Event)` + `impl EventEmitter<Event>` |
| React to a child entity's events | `cx.subscribe(&child, ...)` or `cx.subscribe_in(&child, window, ...)` (when `&mut Window` is needed) |
| React to any entity mutation | `cx.observe(...)` |

`cx.emit()` does **not** trigger rendering. Never notify unconditionally from `render` — that produces a permanent redraw loop. When several fields form one invariant, update them together and notify once. Prefer **controlled values**: pass the current value in, receive a requested change through the callback, update the owner, render again. A callback reports intent; it must not create a second hidden source of truth, and it must be re-entrancy-safe.

```rust
Switch::new("snap-enabled")
    .checked(self.snapping)
    .on_change(cx.listener(|this, next, _, cx| {
        this.snapping = *next;
        cx.notify();
    }))
```

`on_change` is the controlled-value contract for `Checkbox`, `Switch`, `Radio` and `RadioGroup` (`&bool` / requested `&usize`). `Button::on_click` is a command. `Input` uses its state entity's event/subscription API instead.

### 4.5 Actions, key bindings, focus

Actions are typed inward commands; Events are outward facts. Model **one logical desktop command once** — a toolbar Button, dropdown item, context-menu item, native menu item and key binding should all dispatch the same Action or call the same owner method.

```rust
use gpui_kit::{actions, KeyBinding};

actions!(timeline, [SplitAtPlayhead, RippleDelete, ToggleSnapping, Undo, Redo]);

const TIMELINE_CONTEXT: &str = "Timeline";

pub fn init(cx: &mut App) {
    // 1. Bindings FIRST — set_menus bakes shortcuts into the native menu at call time.
    cx.bind_keys([
        KeyBinding::new("s", SplitAtPlayhead, Some(TIMELINE_CONTEXT)),
        KeyBinding::new("delete", RippleDelete, Some(TIMELINE_CONTEXT)),
        KeyBinding::new(
            if cx.platform_info().is_mac() { "cmd-z" } else { "ctrl-z" },
            Undo,
            Some(TIMELINE_CONTEXT),
        ),
    ]);

    // 2. THEN the menu bar. Re-call set_menus if the keymap or locale changes later.
    // cx.set_menus(vec![Menu { ... }]);
}
```

Attach `key_context` and `on_action` to the **same focused region**:

```rust
div()
    .id("timeline-root")
    .track_focus(&self.focus_handle)
    .key_context(TIMELINE_CONTEXT)
    .on_action(cx.listener(Self::split_at_playhead))
    .on_action(cx.listener(Self::ripple_delete))
    .child(self.render_tracks(cx))
```

Focus rules that are easy to get wrong:

- Retain a `FocusHandle` in the entity that owns keyboard interaction.
- A tracked handle is a Tab stop **only** if created with `cx.focus_handle().tab_stop(true)`; the element's own `.tab_index()` / `.tab_stop()` do not apply to a handle passed to `track_focus`.
- Stateless components can create a stable handle in `render` via `window.use_keyed_state(id, cx, |_, cx| cx.focus_handle().tab_stop(true))` — this is what `Button` does.
- A handle alone does not create a keyboard dispatch node: it must be rendered with `.track_focus(&handle)`.
- Composite widgets implement the complete navigation model (arrows, Home/End, confirmation, cancellation, Tab), not isolated shortcuts.
- Modals trap focus and restore the previous valid target; nested overlays dismiss from the top. Escape dismisses the topmost dismissible layer.
- When the focused element disappears (e.g. a timeline replaced by an empty state), key dispatch falls back to the tree root. Register one `cx.on_focus_lost` listener per window and restore to `window.focus_lost_restore_target(cx)`.
- Use `window.dispatch_action(Box::new(SomeAction), cx)` when a button, palette or menu item must issue the same command as a key binding; it follows the focused dispatch path.
- GPUI supplies platform `OsAction`s for Undo/Redo/Cut/Copy/Paste in menus; no custom `on_action` is needed for those on focused text controls.

### 4.6 Async work and side effects

Start async work from an event, lifecycle hook or named method — never as an unconditional side effect of `render`.

| Need | API |
| --- | --- |
| Foreground UI-thread task belonging to an entity | `cx.spawn(async move \|this, cx\| { ... })` — receives `(WeakEntity<Self>, &mut AsyncApp)` |
| Task that later needs the window | `cx.spawn_in(window, async move \|this, cx\| ...)` → `this.update_in(cx, ...)` |
| CPU-heavy owned `Send` work | `cx.background_spawn(async move { ... }).await` inside a foreground task |
| Window-owned task, no entity | `window.spawn(cx, ...)` |
| Deferred UI-tree/focus mutation | `window.defer(cx, ...)` / `cx.defer_in(window, ...)` |

```rust
use std::time::Duration;
use gpui_kit::{Context, SharedString, Task};

struct ExportPanel {
    export_task: Option<Task<()>>,
    request_id: u64,
    status: SharedString,
}

impl ExportPanel {
    fn start_export(&mut self, request_id: u64, cx: &mut Context<Self>) {
        self.request_id = request_id;
        self.export_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    // FFmpeg encode runs off the UI thread (placeholder for media-engine call)
                    std::thread::sleep(Duration::from_millis(800));
                    Ok::<_, String>("export.mp4".to_string())
                })
                .await;

            _ = this.update(cx, |view, cx| {
                if view.request_id != request_id {
                    return; // reject stale results: project/selection changed while working
                }
                view.status = match result {
                    Ok(path) => format!("Exported {path}").into(),
                    Err(error) => format!("Export failed: {error}").into(),
                };
                cx.notify();
            });
        }));
    }
}
```

Rules:

- `Task<T>` is an ownership-controlled handle: dropping it cancels unfinished work. Store it on the owning view when its lifetime should follow the view; call `.detach()` only for genuinely independent work.
- Capture `WeakEntity`, not `Entity`, when work must not keep a closed view alive; handle `update`/`update_in` failure after `await`.
- Attach a **revision or request ID** and reject stale results. Agent events, thumbnails, waveform generation and transcription can all arrive after the project, selection or view changed.
- Represent operations with explicit states: idle, loading, loaded, failed. Preserve usable previous data during refresh; prevent duplicate destructive submissions; surface recoverable errors in the UI, not only in logs.
- Never update GPUI entities from a background thread. GPUI has no `Task::then` — await the background result inside `cx.spawn`.
- Inside `cx.defer_in(window, ...)` never call `update`/`read` on the same entity; use the `&mut Self` the callback supplies (re-entering its lock panics).

---

## 5. Application architecture: feature crates, one command path

Organize by **capability**, not by implementation role. The GPUI Kit Coding Guides explicitly reject global `models/`, `views/`, `modals/` and `commands/` directories for large applications. Keep a feature's model, views, commands and dialogs together behind one public boundary.

```text
apps/desktop/src/
├── app/          bootstrap, menus, key bindings, themes, window lifecycle
├── workspace/    DockArea, panel registry, layout persistence, workspace tabs
├── media/        media bin panel (view + panel adapter only)
├── preview/      preview monitor panel
├── timeline/     timeline panel + custom canvas element
├── inspector/    inspector panel
├── effects/
├── export/
└── assistant/    AI Studio dockable panel, skills picker
```

`apps/desktop` composes; the domain logic lives in `crates/*` per `AGENTS.md` §3–§4. A view must not depend on a sibling feature's internals — communicate through `EditorCommand`, a typed event, or a small shared service.

**One command path.** Every interaction that changes the project, from a drag on the timeline or a menu item to an MCP tool call from an agent, resolves to an `EditorCommand`:

```text
pointer / keyboard / menu / AI tool
        → Action or typed command request
        → crates/command-system dispatch (permission + approval gate)
        → deterministic state transition in crates/timeline / crates/project
        → cx.notify() on affected views + undo/redo history entry
```

The UI never applies the change locally and "also" sends a command. Approval checks live in the editor, not in prompts. This is what makes agent changes visible in history, undoable and replayable.

Naming follows the ecosystem vocabulary: `<Control>State` for retained behavioral models, `<Control>Handle` for imperative references, `<Control>Event` for semantic notifications, `<Role>Delegate` / `<Role>Provider` for pluggable owners, `render_<region>` for named regions. Spell `Context` out for anything that is not GPUI's `cx`. Use `selected` / `focused` / `hovered` / `confirmed` precisely; `index` is positional, `id` is stable identity; `readonly` is one word.

---

## 6. The NLE workspace shell

### 6.1 Dock: long-lived panels

The workspace is a **document workspace / dock area**, not a page-navigation shell. Use `DockArea` for panels users can arrange and that persist across launches.

```rust
use gpui_kit::component::dock::{DockArea, DockSkin, DockLayout, panel_handle};
use std::rc::Rc;

struct Workspace {
    dock_area: Entity<DockArea>,
    dock_skin: Rc<DockSkin>,
}

impl Workspace {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // The optional version belongs to OUR saved-layout schema; bump on a breaking layout change.
        let (dock_area, dock_skin) = DockSkin::dock_area("main-dock", Some(1), window, cx);
        Self { dock_area, dock_skin }
    }
}
```

A styled dock panel implements `BasePanel` (identity/persistence), `Panel` (title, tab, toolbar), `Focusable`, `EventEmitter<PanelEvent>` and `Render`:

```rust
use gpui_kit::component::dock::{BasePanel, Panel, PanelEvent};

impl BasePanel for MediaBinPanel {
    fn panel_name(&self) -> &'static str { "MediaBinPanel" }
}

impl Panel for MediaBinPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        "Media"
    }
}

impl EventEmitter<PanelEvent> for MediaBinPanel {}

impl Focusable for MediaBinPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle { self.focus_handle.clone() }
}
```

Build the initial layout as a value, then install it:

```rust
let layout = DockLayout::h_split()
    .child(
        DockLayout::tabs().panel_view(panel_handle(media_bin), cx),
        Some(px(240.)),               // preferred size
    )
    .child(
        DockLayout::tabs().panel_view(panel_handle(preview), cx),
        None,                          // fills the remaining space
    );

self.dock_area.update(cx, |area, cx| {
    area.set_center(layout, window, cx);
});
```

`h_split()` / `v_split()` are rows/columns, `tabs()` is a tab group; `child()` is only valid on a split, `panel_view()` only on `tabs()`. Left/right/bottom regions use `DockPlacement`. `DockEvent::LayoutChanged` is the persistence trigger.

Persist and restore:

```rust
use gpui_kit::component::dock::{DockAreaState, register_panel};

let state = self.dock_area.read(cx).dump(cx);
let json = serde_json::to_string_pretty(&state)?;

let state: DockAreaState = serde_json::from_str(&json)?;
self.dock_area.update(cx, |area, cx| area.load(state, window, cx))?;
```

Register **every** restorable panel name during application initialization. The factory rebuilds the styled panel behind a `panel_handle`:

```rust
register_panel(cx, "MediaBinPanel", |state, window, cx| {
    let panel = cx.new(|cx| MediaBinPanel::from_state(state, window, cx));
    panel_handle(panel)
});
```

Keep a sensible fallback layout for removed panels. Clamp restored split sizes against the current window. The version passed to `dock_area` is a NodaVideo schema version, not a Kit version. Note the distinction: **Dock panel zoom** makes one tab group fill the `DockArea` while keeping chrome; it must not modify window `rem` size or application zoom.

### 6.2 Resizable: fixed split panes — use the pinned source, not the website examples

Use `Resizable` for pane splits that are *not* dock tabs (media bin width, inspector width, timeline height). Always give each major region a minimum, a comfortable default and a rule for surplus; use `.fixed()` for panes that should keep the width the user gave them.

**Verified against `gpui-base 0.9.0` (`src/resizable/mod.rs`, `panel.rs`)** — this is the correct 0.9.0 shape:

```rust
use gpui_kit::component::resizable::{
    ResizablePanelEvent, ResizableState, h_resizable, resizable_panel,
};
use gpui_kit::{Context, Entity, Subscription, px};

struct WorkspaceLayout {
    main_state: Entity<ResizableState>,
    _subscriptions: Vec<Subscription>,
}

impl WorkspaceLayout {
    fn new(cx: &mut Context<Self>) -> Self {
        let main_state = cx.new(|_| ResizableState::default());
        let subscription = cx.subscribe(&main_state, |this, _, event: &ResizablePanelEvent, cx| {
            if let ResizablePanelEvent::Resized = event {
                this.save_layout(cx);       // persist user preference
            }
        });
        Self { main_state, _subscriptions: vec![subscription] }
    }
}

// In render:
h_resizable("main-split")                       // ONE argument in 0.9.0
    .with_state(&self.main_state)               // attach retained state explicitly
    .on_resize(|state, _window, cx| {
        let sizes = state.read(cx).sizes();
        // react to a completed drag
    })
    .child(
        resizable_panel()
            .size(px(240.))
            .size_range(px(160.)..px(480.))
            .fixed()
            .child(media_bin)
    )
    .child(
        resizable_panel().child(preview_and_timeline)
    )
```

Verified 0.9.0 signatures:

| API | Signature / note |
| --- | --- |
| `h_resizable(id)` / `v_resizable(id)` | `fn(impl Into<ElementId>) -> ResizablePanelGroup` — **one** argument |
| `ResizablePanelGroup::with_state(&Entity<ResizableState>)` | how retained state is attached |
| `ResizablePanelGroup::on_resize(fn(&Entity<ResizableState>, &mut Window, &mut App))` | drag callback |
| `resizable_panel()` | `-> ResizablePanel`, with `.size()`, `.size_range()`, `.fixed()`, `.visible()` |
| `ResizableState` | `Default` (no `new(cx)` in 0.9.0), `.sizes()`, emits `ResizablePanelEvent::Resized` |
| Nested layouts | pass a nested group through `.child(...)` (`From<ResizablePanelGroup> for ResizablePanel`); there is no `.group()` in 0.9.0 |
| `resize_handle(id, axis)` | standalone edge handle with `HandleEdge`, `.inside(...)`, `.on_drag(...)` |

Use a separate `ResizableState` per independent layout. Every scrollable region has one owner; add `min_w_0()` / `min_h_0()` on flexible children allowed to shrink, and attach `Scrollable` to the element that owns the full panel so its scrollbar resolves against the panel edge.

### 6.3 Chrome: toolbar, status bar, tabs, menus, command palette

| Surface | Component | NodaVideo use |
| --- | --- | --- |
| Panel headers | `Toolbar::new("...")` / `ToolbarGroup` | Transport controls, import, split, snapping. `.child()` for sized controls (propagates size, forces ghost+compact), `.content()` for strings, `Separator::vertical()`, `div().flex_1()` spacers. Icon-only buttons **must** have a tooltip (it is the accessible name). Roving arrow-key focus is built in |
| Window footer | `StatusBar::new()` | Frame rate, timecode `00:00:12:08`, sequence resolution, render/agent status. `.left()` / `.right()` / `.child()` for the three regions; plain strings for read-only text, ghost xsmall `Button` for clickable items |
| Workspace modes | `TabBar` / `Tab` | Edit / Color / Audio / AI Studio |
| Secondary commands | `DropdownMenu`, `ContextMenuExt` | Prefer a visible primary action plus a menu, not a hover-revealed icon strip. Action-backed items automatically show bound shortcuts |
| Global commands | `Command` + `CommandState` + `CommandGroup` + `CommandItem` | Command palette for "Export…", "Add audio track", skills, agent tasks. Bind real Actions so hints and dispatch stay in sync |
| Overlays | `Dialog`, `AlertDialog`, `Sheet`, `Popover`, `Notification`, `Tooltip` | Export settings, destructive confirmations, AI change preview, render progress, errors |

Menu items are Action-backed: `.menu("Copy", Box::new(Copy))`, `.menu_with_icon(...)`, `.separator()`, `.submenu(...)`, `.action_context(focus_handle)`. Bind keys **before** `cx.set_menus`. The menu owns navigation and dismissal; the feature owner still owns whether a command is allowed. Do not make a context menu the only path to an essential command.

```rust
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};

let state = cx.new(|cx| CommandState::new(window, cx));

Command::new(&state)
    .group(
        CommandGroup::new().label("Timeline")
            .item(CommandItem::new().label("Split at playhead").action(Box::new(SplitAtPlayhead)))
            .item(CommandItem::new().label("Ripple delete").action(Box::new(RippleDelete))),
    )
    .group(
        CommandGroup::new().label("Project")
            .item(CommandItem::new().label("Export…").action(Box::new(OpenExport))),
    )
    .placeholder("Type a command or search…");
```

`Command` owns entries and presentation; `CommandState` owns query, focus, selection and scrolling. Create one state per independently rendered palette. Dynamic/async entries live in the owner view and are rebuilt into a fresh `Command` on render.

### 6.4 Panel-to-component map

| Panel | Composition | Watch out for |
| --- | --- | --- |
| Media bin | `Tree` for folders, `DataTable` / `v_virtual_list` for asset rows, `Tooltip`, `ContextMenu`, `Progress` for import | Virtualize from the start; stable IDs from `MediaId`; thumbnails arrive asynchronously, so represent loading/empty/failed states |
| Preview monitor | Custom `Element`/canvas for the video frame, `Toolbar` for transport, `Slider` or custom playhead scrubber | Never decode on the UI thread; playback frames come from `gpu-renderer` / `media-engine`. Timecode from integer frames |
| Timeline | Custom `Element` with `ElementId`s derived from `TrackId` / `ClipId`, `v_virtual_list` for many tracks, `Scrollable` with region-owned scroll | Trimming, snapping and ripple logic live in `crates/timeline`. The view only dispatches `EditorCommand`s. No float-second time |
| Inspector | `Form` / `Field`, `NumberInput`, `Slider`, `ColorPicker`, `Select`, `Switch`, `Accordion`, `Tabs` | Controlled values; label every field; validation next to the field; multi-select must show a consistent state |
| Effects / Color | `Tabs`, `Accordion`, `Slider`, `Select`, `Chart` (histogram) | Compact density for data-dense work, medium elsewhere |
| Export | `Dialog`, `Form`, `Progress`, `AlertDialog`, `Notification` | `Export…` with an ellipsis; disable duplicate submission; keep user input on failure |
| AI Studio | `Message`, `Bubble`, `MessageScroller`, `Input`/`Textarea`, `Button`, skill picker (`List`/`Command`), `Diff`-like review of proposed commands | Dockable and optional. Agent disconnect must leave the editor fully functional. Approval gates live in the command layer, not in the panel |
| Workspace shell | `DockArea` + `DockSkin` + `Resizable` + `Toolbar` + `StatusBar` + `TabBar` | Persist both dock layout and resizable sizes; clamp on restore |

---

## 7. Design system rules (normative)

These are condensed from the vendored Design Guides. **Must** is a correctness/ecosystem constraint; **should** is the default needing a concrete reason to override.

### 7.1 Desktop before web convention

- Keyboard access, window chrome, menus, dense data views, resizable regions, persistent navigation.
- Medium density is the default. Compact/**small** is for toolbars, menus, tables and repeated professional data (the timeline, media bin and inspector qualify); **xsmall** is exceptional, never whole-application.
- Default arrow cursor for buttons/menu items/tabs; pointing hand only for links and link-like content; text/resize/grab cursors only when they describe an active manipulation.
- Explicit focus ownership and region-owned scrolling. No hover-only critical actions, no link-styled application commands, no page-navigation-first shell, no CSS-style descendant overrides.
- A visible primary action plus `DropdownMenu`/`ContextMenu` for secondary commands, not a row of hover-only icon buttons.

### 7.2 Tokens, `rem`, density and the dark theme

- Read colors from `cx.theme()` by **semantic role**: `background`, `foreground`, `muted`/`muted_foreground`, `primary`, `danger`, `warning`, `success`, `info`, `border`, plus named surfaces (`group_box`, `popover`, `sidebar`, `status_bar`). Never hard-code hex, `rgb`/`rgba` or `hsla` in application UI. If a role is missing, add it to NodaVideo's theme/token layer.
- NodaVideo's dark, dense NLE look must be implemented as a **theme** (e.g. `Theme::change(ThemeMode::Dark, None, cx)` / a registered product theme), not as scattered color literals. Verify every custom surface in light, dark and custom themes; never assume foreground is black or background is white.
- The theme's base `font_size` is the window `rem` and the application zoom control: `Theme::update(cx, |theme| theme.font_size = px(18.))`. Use rem-based helpers (`p_2()`, `gap_3()`, `w_64()`, `text_sm()`, `h_8()`, `size_4()`) for ordinary layout so type, spacing, controls and icons scale together. Direct `px(...)` is allowed only for a documented physical/raster/platform boundary (hairline, bitmap dimension, measured geometry, hit-test tolerance). Every direct `px()` and raw color in application UI is a review finding.
- Anything cached from resolved layout (wrapped heights, virtual-list measurement, popup geometry, timeline lane metrics) must include `window.rem_size()` in its invalidation key.
- Use the component `Size` API (`xsmall`/`small`/`medium`/`large`) so typography, icons, padding and hit targets change together. Use named radius tiers; nested surfaces read as concentric shapes. Do not assume a rounded parent clips descendants.
- Hierarchy: window title → section title → body → muted → labels. Use size, weight and spacing before color or containers. Emphasis is a limited budget: one focal point per local surface. Keep badges/alerts semantic and scarce.
- Alignment spines are quality invariants: sibling regions share a content inset; repeated rows keep stable columns; equal gaps must be equal to the rendered pixel, derived from one shared token rather than patched with raw offsets. A scrollbar sits at the trailing edge of the region that scrolls.
- `h_flex()` centers children on the cross axis; `v_flex()` leaves them stretched. A row of full-height columns needs `.items_stretch()` (or `h_full()` on the column), or tall columns are centered and their headers clipped.

### 7.3 Buttons, states, overlays, copy, accessibility

- **`Button` for every in-app command** (primary / default / outline / ghost / danger). `Link` is only for external URLs, web resources and email addresses. "View" of an in-app destination is still a Button/row/tab. Primary is the default commitment in a decision area (normally Enter), not merely the only or most frequent action.
- Every interactive control needs designed rest, hover, pressed, open/pressed, focus-visible, selected/checked, disabled, loading and error states. Show selection through the item's own surface — no leading-edge bar. Distinguish `selected` (persistent), `focused` (keyboard target) and `hovered`.
- Choose the smallest overlay that fits: tooltip → popover → menu → notification → dialog → alert dialog → sheet. Escape dismisses the topmost dismissible layer and returns focus to the trigger. Avoid stacking overlays. Use `AlertDialog` only for consequential, irreversible work; prefer undo/notification for reversible changes.
- Copy: name the object and the verb (`Delete "Roadmap"?` with a `Delete` button), sentence case, `Cancel` consistently, `…` (single ellipsis character) on commands that open a dialog/sheet or need more input (`Export…`, `Settings…`). Avoid "Are you sure…", generic titles, `OK` except pure acknowledgement, and "successfully" toasts when the saved state is already visible. Errors say what happened and the recovery action.
- Motion explains change; honor reduced motion. Never require animation to understand state.
- Accessibility checklist: every action keyboard-reachable; focus order follows visual/task order; focus visible and restored after overlays; accessible names on icon-only controls; sufficient contrast; status not by color alone; disabled vs readonly distinguishable; labels/errors near controls; usable with longer translations and larger text; comfortable pointer targets even in dense layouts.
- Platform fit: Command vs Control, native window decorations, system appearance, scrollbar behavior and menus. Keep the information architecture stable across platforms without erasing familiar behavior.
- For AI-generated UI, state primary task, state owner, composition and keyboard path first (§0.3), inspect the nearest feature and theme tokens, and never infer an API from React/Shadcn examples.

---

## 8. Timeline and preview: custom elements, frame-exact time

The timeline is the one place where standard components are insufficient and a custom GPUI `Element` is justified: dragging clip edges, snapping, hit-testing clip bodies, multi-track lanes and zoom all depend on resolved geometry.

Rules:

1. **Model purity.** `crates/timeline` owns trim/split/ripple/snap logic and is synchronous and pure. Given `(project_state, command)` the result is deterministic — no wall-clock reads, no random IDs, no I/O. Pass IDs, timestamps and resolved assets in from the caller.
2. **Frame-exact time.** Store and compute in integer frames or rational time. Convert to timecode (`HH:MM:SS:FF`) for display and to seconds only at the FFmpeg boundary. Sequence frame rate lives on the sequence, not on a clip. Test trim/split/ripple/snapping at frame boundaries.
3. **One timeline.** HyperFrames produces compositions; the Rust timeline owns their timing and placement. No second playhead, no second clock.
4. **Stable identity.** `.id(("clip", clip.id))`, `.id(("track", track.id))`, namespaced by sequence where needed. Never key a clip by its position in a track — ripple deletes and reorders would silently reset its UI state.
5. **Drag gestures.** Use `on_drag` / `on_drag_move` payloads with explicit modifier handling. While dragging, derive presentation from a single draft state; commit exactly one `EditorCommand` on release so undo has one entry, not one per pointer move.
6. **Snapping and zoom.** Keep zoom in view state (frames-per-pixel, integer-scaled where possible), never in the project model. Clamp restored sizes and zoom against the current window.
7. **Virtualization.** Use `v_virtual_list` / `h_virtual_list` for long track or media lists. Precompute `Rc<Vec<Size<Pixels>>>` item sizes, render only `visible_range`, keep row render closures side-effect-free, and invalidate measurements when width, typography, `rem` size or content changes. Keyboard selection and scroll-to-item must operate in model coordinates even when most rows are not rendered.
8. **Preview frames.** The preview surface draws frames supplied by `gpu-renderer` / `media-engine`; it never decodes or blocks. A/V sync and playback position are owned by the editor core, not by the widget.
9. **AI-proposed changes.** Render them as a preview layer with explicit approve/reject. Approval gates live in `crates/command-system`; the panel is only a presentation of the pending commands.

---

## 9. Heavy work and agent I/O stay off the UI thread

`AGENTS.md` §2.7 is a hard constraint. Map it to GPUI:

| Work | Runs where | UI contract |
| --- | --- | --- |
| FFmpeg decode/encode, probing, thumbnails, waveforms | `media-engine` worker threads, awaited via `cx.background_spawn` inside `cx.spawn` | idle/loading/loaded/failed states; request IDs to reject stale results |
| wgpu compositing, effects, preview frames | render worker / GPU | view only displays the latest frame; never blocks `render` |
| Audio processing and playback sync | `audio-engine` threads | timecode and playhead from the editor clock |
| Transcription, captions, exports | background tasks with cancellation | `Progress`, cancellable `Task`, no duplicate submission |
| Agent CLI I/O (ACP, Codex app-server, Claude stream-JSON, MCP) | tokio tasks in `agent-runtime` / `agent-*` | events are converted into typed application events, then into `EditorCommand`s; never mutate GPUI state from the adapter |

Keep `Subscription` and `Task` handles on the owning view. Use `WeakEntity` for work that should not keep a closed panel alive. A disconnected agent must leave the editor fully functional (M4 acceptance test).

---

## 10. Performance

- GPUI is **invalidation-driven**. Notify the narrowest owning entity after a coherent state change; never mutate or notify unconditionally in `render`.
- Avoid rebuilding entities, subscriptions, focus handles and expensive data structures per frame. Avoid cloning large strings/collections solely to satisfy a closure; capture stable handles or shared data.
- Virtualize long collections; render only the visible range. Keep item renderers cheap and side-effect-free.
- Frame budget: ~8.33 ms at 120 Hz, ~16.67 ms at 60 Hz. A video editor adds decode/composite work on top of UI, so the UI thread must stay lean.
- Measured `FRAME`, `P95`, `DROP`, `INV`, `FPS`, `INTERVAL` figures require **release builds** and representative whole-window workloads — not a debug hello window. The dev-profile overrides in `Cargo.toml` are for responsive debugging, not benchmarking.
- Measure before adding caches; every cache needs a clear invalidation owner. Keep animation work bounded and honor reduced motion.
- Common failure modes: one entity holding all unrelated state; business logic in a long `render`; index-based IDs for reorderable content; literal colors; custom clickable `div`s where a semantic component exists; duplicated local state; `notify` loops; nested scroll containers; a new component variant for one screen; confirmation dialogs for reversible actions; tests that never exercise keyboard/pointer behavior.

---

## 11. Testing

Test at the lowest layer that can prove the behavior:

1. **Pure Rust `#[test]`** — timeline invariants (trim, split, ripple, snapping, frame-exact arithmetic), project schema migrations, command determinism, adapter JSON-RPC fixtures.
2. **GPUI context tests** — entities, events, subscriptions, actions, async tasks using `#[gpui_kit::test]` and `TestAppContext`.
3. **UI integration tests** — real components in a headless window; dispatch clicks/keyboard/scrolling and check state, focus, layout and owner callbacks. Use `gpui_kit::test::TestWindowExt` on the real `Window`, `.id("status").test_support()` via `TestSupportExt` for custom native elements (place before `.track_focus()`), then `window.find(...)`, `click`, `input`, `press`, `render_frame`, `wait_for`.
4. **Application smoke tests** — the milestone acceptance tests from `AGENTS.md` §8.

Setup notes:

- Add `test-support` to the **same source and version** of `gpui-kit` as a dev-dependency. Import Kit types explicitly in test modules; `use gpui_kit::*;` can shadow Rust's built-in `#[test]` with GPUI's macro.
- Initialize with `cx.update(gpui_kit::init)`, open a headless window of explicit size through `gpui_kit::open_window`; `Root` mounts overlay layers automatically.
- Headless tests still need the platform's native build dependencies. Windows explicitly skips Kit's headless rendering target (no Windows headless renderer in `gpui-pre 0.3.11`); macOS uses Metal, Linux WGPU. Pixel comparisons need that separate renderer — interaction/state tests and pixel checks are different evidence.
- Native accessibility properties (`checked()`, `value()`, `label()`, `focused()`, `bounds()`, `visible()`) are *observed facts from the last completed frame*, not live elements. Re-query after each interaction and pair them with real interactions and the application result. A label is not rendered text.
- CI must not require live CLI logins or network access (AGENTS.md §9). Add an acceptance test with every milestone.
- Intended CI contract: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Set up in M0.

---

## 12. Checklists

### 12.1 Before writing a view (Coding Guides "Rules for coding agents")

- [ ] Nearest implementation, its tests, the re-export seam and the component docs were read.
- [ ] Real signatures were searched in the pinned 0.9.0 source, not inferred from web docs or another framework.
- [ ] Behavior owner and presentation owner are named.
- [ ] `RenderOnce` vs `Entity<T>` vs custom `Element` is chosen deliberately.
- [ ] Retained identity and state lifecycle are explicit; repeated elements have stable domain IDs.
- [ ] Pointer, keyboard, focus and accessibility contract is defined.
- [ ] Layout and overflow owner is defined (one scroll owner per region; `min_w_0()`/`min_h_0()` where needed).
- [ ] Theme tokens are used; every `px()`/raw color is a documented exception.
- [ ] The test that would fail on regression is identified.
- [ ] Every mutation resolves to one `EditorCommand`, with permission/approval enforced in the command layer.

### 12.2 Before finishing a screen (Design review checklist)

- [ ] Is the primary task and next action clear without guessing?
- [ ] Does every action keep its promise (label, state, scope, feedback, result are one outcome)?
- [ ] Is hierarchy decisive and restrained (space to the core feature; scarce primary Buttons, badges, alerts, strong color)?
- [ ] Could the interface do less, better?
- [ ] Is the structure exact (alignment spines, equal gaps to the rendered pixel, scrollbars at the right edge)?
- [ ] Does it follow the component system (standard geometry, states, keyboard, dismissal, theme tokens)?
- [ ] Does it remain usable in every state and constraint (keyboard/focus, empty/loading/failure/permission, longer translations, zoom, minimum window size, reduced motion)?
- [ ] Has the task been completed in a real window with real components and representative content?
- [ ] Light, dark and custom themes verified; the AI panel can be hidden and the editor still works.

---

## 13. Known documentation drift — verify against the pinned 0.9.0 source

These were found while writing this guideline. They are the reason §0.1 puts the pinned source first.

| Topic | Published doc says | Pinned `0.9.0` source / correct reading |
| --- | --- | --- |
| `h_resizable` / `v_resizable` | Three incompatible forms on one page: `h_resizable("id")`, `h_resizable("id", state)`, `h_resizable("id", window, cx)` | `pub fn h_resizable(id: impl Into<ElementId>) -> ResizablePanelGroup` — **one** argument (`gpui-base/src/resizable/mod.rs`, and the component wrapper). Attach state with `.with_state(&Entity<ResizableState>)` |
| `ResizableState` construction | `ResizableState::new(cx)` in the "Settings Panel" example | `ResizableState: Default`; use `cx.new(\|_\| ResizableState::default())`. `sizes()` returns `&Vec<Pixels>` |
| Nested resizable groups | `.group(v_resizable(...))` | No `group()` on `ResizablePanelGroup` in 0.9.0. Nest via `.child(nested_group)` (`From<ResizablePanelGroup> for ResizablePanel`) |
| `on_resize` signature | `.on_resize(\|state, window, cx\| ...)` with `state.read(cx).sizes()` | `Fn(&Entity<ResizableState>, &mut Window, &mut App)` on the group; correct as shown |
| Kit version in the vendored `usage.md` | `gpui-kit = "0.7"` | `Cargo.toml` / `Cargo.lock` pin **0.9.0**; `gpui-pre 0.3.11`. Treat `usage.md` examples as older-generation API hints |
| `MenuItem::action` | Shown in the GPUI Action reference for `cx.set_menus` native menus | The component menu page uses `PopupMenu` with `.menu("Copy", Box::new(Copy))` / `PopupMenuItem`; no `gpui_kit::component::MenuItem::action`. Native menu items (`MenuItem::action`) and component popup menus are different APIs — do not mix them |
| Menu import paths | One example imports `gpui_kit::component::popup_menu::{PopupMenuExt, PopupMenuItem}`, another `component::menu::{...}` | Verify the re-export seam in `gpui-component-0.9.0` before copying |
| Tabs | API reference lists `with_menu(bool)`, examples use `.menu(true)` | Verify against pinned source |
| `Scrollable` vs `VirtualList` | Scrollable examples show a `VirtualList::new(...)` form; the VirtualList page shows `v_virtual_list(cx.entity().clone(), id, sizes, closure)` | Use the `v_virtual_list` / `h_virtual_list` call shape from the VirtualList page, then verify |
| Virtual-list example | The "ChatWindow" example uses `self.item_sizes` without declaring the field | Example is illustrative, not compilable; do not copy verbatim |
| Menu check side | Prose says `check_side`, example calls `.check_size(Side::Right)` | Verify the exact builder name |
| `VisualTestContext` | Mentioned in the Coding Guides testing ladder | The Kit testing reference says it is available for existing GPUI window helpers; prefer `#[gpui_kit::test]` + `TestWindowExt` for UI integration tests |

**Rule:** a snippet from a website page is a hint until it compiles against the pinned source. When in doubt, read the crate source in `~/.cargo/registry/src/`. Update this table whenever a new discrepancy is found.

---

## 14. GPUI deliverables by milestone

| Milestone | GPUI work in `apps/desktop` | Acceptance evidence |
| --- | --- | --- |
| **M0** (current) | Bootstrap, `Root`, native menus, actions/key bindings, `DockArea` + `Resizable` shell, themes, layout persistence, command palette, project open/create | App launches on macOS, Windows and Linux; panels resize and persist; new project can be created and reopened. `cargo fmt --all -- --check`, `clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` set up in CI |
| **M1** | Media bin (`Tree`, `DataTable`/virtual list), async thumbnail/waveform states, import progress, context menus | Import a clip; see thumbnail, duration and waveform; relaunch retains the media index |
| **M2** | Custom timeline `Element`, drag/trim/split gestures, virtualized tracks, snapping, region-owned scroll, undo/redo UI | Drag to a track, trim both edges, split at playhead, ripple delete; undo/redo restores exact state; times frame-exact |
| **M3** | Preview canvas, transport toolbar, scrubbing, export dialog/progress, status-bar timecode | First vertical slice: import → drag → trim → preview → export MP4, playable in a third-party player |
| **M4** | Dockable AI Studio panel, provider selector, conversation UI, skills picker, MCP tool results surfaced in history | Two providers run the same task; every agent mutation appears in undo history; disconnect leaves the editor fully functional |
| **M5** | Review/approve UI for proposed command sequences, preview renders, workflow states | A prompt produces a reviewable sequence and preview render, committing only after user approval |

---

## 15. Sources

**Vendored in this repository (read first):**
- `AGENTS.md` — NodaVideo architectural contract
- `skills/gpui-kit-design-guides/references/design-guides.md` — normative Design Guides
- `skills/gpui-kit/references/coding-guides.md` — normative Coding Guides
- `skills/gpui-kit/references/conventions.md`, `recipes.md`, `usage.md`, `gpui/*.md` — conventions, tested recipe, GPUI mechanisms

**Pinned crate source (authority for signatures):**
- `~/.cargo/registry/src/<index>/gpui-kit-0.9.0`, `gpui-component-0.9.0`, `gpui-base-0.9.0`

**Online documentation:**
- `https://gpui-kit.com/docs/` — installation, getting-started, coding-guides, design-guides, window/root, render, render-once, entity, context, action, keybinding, focus, event, task, element-id, test, fps, assets, fonts
- `https://gpui-kit.com/component/` — component catalog; per-component Markdown at `/component/{name}.md`; index at `https://gpui-kit.com/llms.txt`, full reference at `https://gpui-kit.com/llms-full.txt`
- `https://gpui.rs/` — GPUI core concepts and examples

**License note:** GPUI Kit's original prose and illustrations are offered under CC BY 4.0 (credit GPUI Kit, link the source page and `https://creativecommons.org/licenses/by/4.0/`, indicate changes); code examples use Apache-2.0. This document paraphrases and cites those sources rather than reproducing them verbatim, except for short API signatures needed for accuracy.
