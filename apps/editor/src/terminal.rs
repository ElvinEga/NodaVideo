//! Project-owned PTY terminal. Raw reads stay on their bounded worker; UI owns ANSI state.

use crate::actions::ToggleTerminal;
use crate::asset::IconName;
use crate::editor::{Editor, WorkspaceTab};
use crate::theme::{
    ACCENT, ACCENT_BACKGROUND, BACKGROUND, BORDER, MUTED, PANEL, SURFACE, SURFACE_HOVER, TEXT,
};
use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{self, Color, NamedColor};
use anyhow::Result;
use gpui::prelude::*;
use gpui::{
    Bounds, ClipboardItem, Context, CursorStyle, Entity, FocusHandle, Font, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ScrollEvent, TextAlign, TextRun,
    Window, canvas, div, fill, point, px, rgb, size, svg,
};
use portable_pty::{Child, CommandBuilder, MasterPty, native_pty_system};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

const TERMINAL_FONT_SIZE: f32 = 12.0;
const TERMINAL_LINE_HEIGHT: f32 = 18.0;
const TERMINAL_DOCK_HEIGHT: f32 = 260.0;
const TERMINAL_KEY_CONTEXT: &str = "Terminal";

pub(crate) struct TerminalState {
    pub(crate) visible: bool,
    pub(crate) tabs: Vec<TerminalTab>,
    pub(crate) active: usize,
    shell_program: String, // 启动时从 $SHELL 读取，之后不再读环境变量。
    error: Option<String>,
}

pub(crate) struct TerminalTab {
    pub(crate) title: String,
    pub(crate) model: Entity<TerminalModel>,
    pub(crate) focus_handle: FocusHandle,
}

pub(crate) struct TerminalModel {
    term: Term<VoidListener>,
    parser: ansi::Processor,
    child: Option<Box<dyn Child + Send + Sync>>,
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    closed: Arc<AtomicBool>,
    status_text: String,
    layout: Option<TerminalLayout>, // 最近一次 canvas 布局，鼠标坐标换算用。
    selecting: bool,
}

#[derive(Clone, Copy)]
struct TerminalLayout {
    bounds: Bounds<gpui::Pixels>,
    cell_width: f32,
    line_height: f32,
    columns: usize,
    lines: usize,
}

struct TerminalScreen {
    rows: Vec<Vec<TerminalCell>>,
    cursor: Option<(usize, usize)>,
}

struct TerminalCell {
    text: String,
    foreground: u32,
    selected: bool,
}

struct ShellProcess {
    child: Box<dyn Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    reader: Box<dyn Read + Send>,
    closed: Arc<AtomicBool>,
}

#[derive(Clone, Copy)]
struct ScreenSize {
    columns: usize,
    lines: usize,
}

impl Dimensions for ScreenSize {
    fn total_lines(&self) -> usize {
        self.lines
    }
    fn screen_lines(&self) -> usize {
        self.lines
    }
    fn columns(&self) -> usize {
        self.columns
    }
}

// Public APIs only
impl TerminalState {
    pub(crate) fn new() -> Self {
        let shell_program = match std::env::var("SHELL") {
            Ok(program) if !program.is_empty() => program,
            Ok(_) | Err(_) => "/bin/sh".to_string(),
        };
        Self {
            visible: false,
            tabs: Vec::new(),
            active: 0,
            shell_program,
            error: None,
        }
    }

    pub(crate) fn open_shell(&mut self, root: &Path, cx: &mut Context<Editor>) {
        let process = match spawn_shell(&self.shell_program, root) {
            Ok(process) => process,
            Err(error) => {
                self.error = Some(format!("Could not start {}: {error:#}", self.shell_program));
                return;
            }
        };
        self.error = None;
        let base_title = Path::new(&self.shell_program)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("shell")
            .to_string();
        let title = if self.tabs.is_empty() {
            base_title
        } else {
            format!("{base_title} {}", self.tabs.len() + 1)
        };
        let focus_handle = cx.focus_handle();
        let model = cx.new(|cx| TerminalModel::new(process, cx));
        cx.observe(&model, |_, _, cx| cx.notify()).detach();
        self.tabs.push(TerminalTab {
            title,
            model,
            focus_handle,
        });
        self.active = self.tabs.len() - 1;
    }

    pub(crate) fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index); // 实体释放时 Drop 只结束本终端自己的子进程。
        if self.tabs.is_empty() {
            self.active = 0;
            self.visible = false;
            return;
        }
        if self.active > index {
            self.active -= 1;
        }
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }
}

// Public APIs only
impl Editor {
    pub(crate) fn action_toggle_terminal(
        &mut self,
        _: &ToggleTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.workspace != WorkspaceTab::AiStudio {
            self.workspace = WorkspaceTab::AiStudio;
            self.terminal.visible = true;
        } else {
            self.terminal.visible = !self.terminal.visible;
        }
        if self.terminal.visible {
            if self.terminal.tabs.is_empty() {
                self.terminal.open_shell(&self.project_root, cx);
            }
            if let Some(tab) = self.terminal.tabs.get(self.terminal.active) {
                tab.focus_handle.focus(window, cx);
            }
        } else {
            self.focus_handle.focus(window, cx);
        }
        cx.notify();
    }
}

pub(crate) fn terminal_dock_view(editor: &Editor, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let terminal = &editor.terminal;
    let active_tab = terminal.tabs.get(terminal.active);
    let (status_text, has_selection) = match active_tab {
        Some(tab) => {
            let model = tab.model.read(cx);
            (model.status_text.clone(), model.has_selection())
        }
        None => (String::new(), false),
    };

    let tab_strip =
        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .children(terminal.tabs.iter().enumerate().map(|(index, tab)| {
                let is_active = index == terminal.active;
                div()
                    .id(format!("terminal-tab-{index}"))
                    .h(px(24.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(4.0))
                    .cursor(CursorStyle::PointingHand)
                    .border_1()
                    .border_color(rgb(if is_active {
                        BORDER
                    } else {
                        BACKGROUND
                    }))
                    .bg(rgb(if is_active {
                        SURFACE
                    } else {
                        PANEL
                    }))
                    .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                    .on_click(cx.listener(move |editor, _, window, cx| {
                        editor.terminal.active = index;
                        if let Some(tab) = editor.terminal.tabs.get(index) {
                            tab.focus_handle.focus(window, cx);
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(if is_active {
                                TEXT
                            } else {
                                MUTED
                            }))
                            .child(tab.title.clone()),
                    )
                    .child(
                        div()
                            .id(format!("terminal-tab-close-{index}"))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(14.0))
                            .rounded(px(3.0))
                            .cursor(CursorStyle::PointingHand)
                            .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                            .on_click(cx.listener(move |editor, _, _, cx| {
                                cx.stop_propagation();
                                editor.terminal.close_tab(index);
                                cx.notify();
                            }))
                            .child(
                                svg()
                                    .path(IconName::Close.path())
                                    .size(px(9.0))
                                    .text_color(rgb(MUTED)),
                            ),
                    )
            }));

    let header = div()
        .h(px(32.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(8.0))
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(PANEL))
        .child(tab_strip)
        .child(
            div()
                .id("terminal-new-shell")
                .flex()
                .items_center()
                .justify_center()
                .size(px(22.0))
                .rounded(px(4.0))
                .cursor(CursorStyle::PointingHand)
                .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                .on_click(cx.listener(|editor, _, window, cx| {
                    editor.terminal.open_shell(&editor.project_root, cx);
                    if let Some(tab) = editor.terminal.tabs.get(editor.terminal.active) {
                        tab.focus_handle.focus(window, cx);
                    }
                    cx.notify();
                }))
                .child(
                    svg()
                        .path(IconName::Plus.path())
                        .size(px(12.0))
                        .text_color(rgb(MUTED)),
                ),
        )
        .child(div().flex_1())
        .child(
            div()
                .font_family("monospace")
                .text_size(px(10.0))
                .text_color(rgb(MUTED))
                .child(status_text),
        )
        .child(
            div()
                .id("terminal-copy")
                .h(px(22.0))
                .px(px(7.0))
                .flex()
                .items_center()
                .gap(px(5.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(rgb(BORDER))
                .when(has_selection, |this| {
                    this.cursor(CursorStyle::PointingHand)
                        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                        .on_click(cx.listener(|editor, _, _, cx| {
                            let Some(tab) = editor.terminal.tabs.get(editor.terminal.active) else {
                                return;
                            };
                            let copied = tab.model.update(cx, |model, _| model.copy());
                            if let Some(text) = copied {
                                cx.write_to_clipboard(ClipboardItem::new_string(text));
                            }
                        }))
                })
                .child(
                    svg()
                        .path(IconName::Copy.path())
                        .size(px(11.0))
                        .text_color(rgb(if has_selection {
                            TEXT
                        } else {
                            MUTED
                        })),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(if has_selection {
                            TEXT
                        } else {
                            MUTED
                        }))
                        .child("Copy"),
                ),
        )
        .child(
            div()
                .id("terminal-hide")
                .flex()
                .items_center()
                .justify_center()
                .size(px(22.0))
                .rounded(px(4.0))
                .cursor(CursorStyle::PointingHand)
                .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                .on_click(cx.listener(|editor, _, window, cx| {
                    editor.terminal.visible = false;
                    editor.focus_handle.focus(window, cx);
                    cx.notify();
                }))
                .child(
                    svg()
                        .path(IconName::Close.path())
                        .size(px(12.0))
                        .text_color(rgb(MUTED)),
                ),
        );

    let body = match active_tab {
        Some(tab) => terminal_screen_view(tab, cx),
        None => div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .child(
                div().text_size(px(11.0)).text_color(rgb(MUTED)).child(
                    terminal
                        .error
                        .clone()
                        .unwrap_or_else(|| "No shell is running.".to_string()),
                ),
            )
            .child(
                div()
                    .id("terminal-empty-new-shell")
                    .h(px(24.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SURFACE))
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(rgb(SURFACE_HOVER)))
                    .on_click(cx.listener(|editor, _, window, cx| {
                        editor.terminal.open_shell(&editor.project_root, cx);
                        if let Some(tab) = editor.terminal.tabs.get(editor.terminal.active) {
                            tab.focus_handle.focus(window, cx);
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(TEXT))
                            .child("New shell"),
                    ),
            )
            .into_any_element(),
    };

    div()
        .id("terminal-dock")
        .h(px(TERMINAL_DOCK_HEIGHT))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .border_t_1()
        .border_color(rgb(BORDER))
        .bg(rgb(BACKGROUND))
        .child(header)
        .child(body)
        .into_any_element()
}

// Private APIs only
impl Editor {
    fn ui_callback_terminal_key_down(
        &mut self,
        event: &KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.terminal.tabs.get(self.terminal.active) else {
            return;
        };
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if modifiers.platform {
            match key {
                "c" => {
                    let copied = tab.model.update(cx, |model, _| model.copy());
                    if let Some(text) = copied {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    }
                    cx.stop_propagation();
                }
                "a" => {
                    tab.model.update(cx, |model, cx| {
                        model.select_all();
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard()
                        && let Some(text) = item.text()
                    {
                        tab.model.update(cx, |model, cx| {
                            model.paste(&text);
                            cx.notify();
                        });
                    }
                    cx.stop_propagation();
                }
                _ => {}
            }
            return;
        }
        if key == "escape" {
            let cleared = tab.model.update(cx, |model, _| {
                let had_selection = model.has_selection();
                model.clear_selection();
                had_selection
            });
            if cleared {
                cx.notify();
                cx.stop_propagation();
                return;
            }
        }
        if tab.model.update(cx, |model, _| model.input(event)) {
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn ui_callback_terminal_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.terminal.tabs.get(self.terminal.active) else {
            return;
        };
        tab.focus_handle.focus(window, cx);
        tab.model.update(cx, |model, cx| {
            let Some(position) = model.point_for_position(event.position) else {
                return;
            };
            model.start_selection(position);
            cx.notify();
        });
    }

    fn ui_callback_terminal_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.terminal.tabs.get(self.terminal.active) else {
            return;
        };
        tab.model.update(cx, |model, cx| {
            if !model.selecting {
                return;
            }
            let Some(position) = model.point_for_position(event.position) else {
                return;
            };
            model.update_selection(position);
            cx.notify();
        });
    }

    fn ui_callback_terminal_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.terminal.tabs.get(self.terminal.active) else {
            return;
        };
        tab.model.update(cx, |model, _| model.end_selection());
    }

    fn ui_callback_terminal_scroll(
        &mut self,
        event: &ScrollEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.terminal.tabs.get(self.terminal.active) else {
            return;
        };
        let delta = event.delta.pixel_delta(px(TERMINAL_LINE_HEIGHT));
        let lines = (f32::from(delta.y) / TERMINAL_LINE_HEIGHT).round() as i32;
        if lines == 0 {
            return;
        }
        tab.model.update(cx, |model, cx| {
            model.scroll(lines);
            cx.notify();
        });
    }
}

fn terminal_screen_view(tab: &TerminalTab, cx: &mut Context<Editor>) -> gpui::AnyElement {
    let focus_handle = tab.focus_handle.clone();
    let screen_model = tab.model.clone();
    div()
        .id("terminal-screen")
        .flex_1()
        .min_h_0()
        .relative()
        .overflow_hidden()
        .bg(rgb(BACKGROUND))
        .key_context(TERMINAL_KEY_CONTEXT)
        .track_focus(&tab.focus_handle)
        .on_key_down(cx.listener(Editor::ui_callback_terminal_key_down))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(Editor::ui_callback_terminal_mouse_down),
        )
        .on_mouse_move(cx.listener(Editor::ui_callback_terminal_mouse_move))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(Editor::ui_callback_terminal_mouse_up),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(Editor::ui_callback_terminal_mouse_up),
        )
        .on_scroll(cx.listener(Editor::ui_callback_terminal_scroll))
        .child(
            canvas(
                move |bounds, window, cx| {
                    let font = terminal_font();
                    let font_size = px(TERMINAL_FONT_SIZE);
                    let cell_width = f32::from(
                        window
                            .text_system()
                            .shape_line(
                                "M".into(),
                                font_size,
                                &[TextRun {
                                    len: 1,
                                    font: font.clone(),
                                    color: rgb(TEXT).into(),
                                    background_color: None,
                                    underline: None,
                                    strikethrough: None,
                                }],
                                None,
                            )
                            .width(),
                    )
                    .max(1.0);
                    let columns =
                        ((f32::from(bounds.size.width) / cell_width).floor() as usize).max(1);
                    let lines = ((f32::from(bounds.size.height) / TERMINAL_LINE_HEIGHT).floor()
                        as usize)
                        .max(1);
                    let layout = TerminalLayout {
                        bounds,
                        cell_width,
                        line_height: TERMINAL_LINE_HEIGHT,
                        columns,
                        lines,
                    };
                    let focused = focus_handle.is_focused(window);
                    let screen = screen_model.update(cx, |model, _| {
                        let dimensions_changed = model.layout.is_none_or(|previous| {
                            previous.columns != columns || previous.lines != lines
                        });
                        if dimensions_changed && let Err(error) = model.resize(columns, lines) {
                            log::debug!("Terminal resize failed: {error:?}");
                        }
                        model.layout = Some(layout);
                        model.snapshot()
                    });
                    (screen, layout, font, focused)
                },
                move |_bounds, (screen, layout, font, focused), window, cx| {
                    paint_terminal_screen(&screen, &layout, &font, focused, window, cx);
                },
            )
            .size_full(),
        )
        .into_any_element()
}

fn paint_terminal_screen(
    screen: &TerminalScreen,
    layout: &TerminalLayout,
    font: &Font,
    focused: bool,
    window: &mut Window,
    cx: &mut gpui::App,
) {
    let font_size = px(TERMINAL_FONT_SIZE);
    let line_height = px(layout.line_height);
    for (row_index, row) in screen.rows.iter().enumerate() {
        let row_y = layout.bounds.origin.y + px(row_index as f32 * layout.line_height);
        let mut run_start = 0;
        while run_start < row.len() {
            let first = &row[run_start];
            let mut run_end = run_start + 1;
            while run_end < row.len()
                && row[run_end].foreground == first.foreground
                && row[run_end].selected == first.selected
            {
                run_end += 1;
            }
            let run_x = layout.bounds.origin.x + px(run_start as f32 * layout.cell_width);
            let run_width = px((run_end - run_start) as f32 * layout.cell_width);
            if first.selected {
                window.paint_quad(fill(
                    Bounds {
                        origin: point(run_x, row_y),
                        size: size(run_width, line_height),
                    },
                    rgb(ACCENT_BACKGROUND),
                ));
            }
            let run_text: String = row[run_start..run_end]
                .iter()
                .map(|cell| cell.text.as_str())
                .collect();
            let shaped = window.text_system().shape_line(
                run_text.clone().into(),
                font_size,
                &[TextRun {
                    len: run_text.len(),
                    font: font.clone(),
                    color: rgb(first.foreground).into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }],
                None,
            );
            if let Err(error) = shaped.paint(
                point(run_x, row_y),
                line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            ) {
                log::debug!("Terminal text paint failed: {error:?}");
            }
            run_start = run_end;
        }
    }
    let Some((cursor_row, cursor_column)) = screen.cursor else {
        return;
    };
    if !focused {
        return;
    }
    let cursor_origin = point(
        layout.bounds.origin.x + px(cursor_column as f32 * layout.cell_width),
        layout.bounds.origin.y + px(cursor_row as f32 * layout.line_height),
    );
    let cursor_bounds = Bounds {
        origin: cursor_origin,
        size: size(px(layout.cell_width), line_height),
    };
    window.paint_quad(fill(cursor_bounds, rgb(ACCENT)));
    let Some(cell) = screen
        .rows
        .get(cursor_row)
        .and_then(|row| row.get(cursor_column))
    else {
        return;
    };
    let shaped = window.text_system().shape_line(
        cell.text.clone().into(),
        font_size,
        &[TextRun {
            len: cell.text.len(),
            font: font.clone(),
            color: rgb(BACKGROUND).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }],
        None,
    );
    if let Err(error) = shaped.paint(
        cursor_origin,
        line_height,
        TextAlign::Left,
        None,
        window,
        cx,
    ) {
        log::debug!("Terminal cursor paint failed: {error:?}");
    }
}

fn terminal_font() -> Font {
    Font {
        family: "Menlo".into(),
        ..Default::default()
    }
}

// Private APIs only
impl TerminalModel {
    fn new(process: ShellProcess, cx: &mut Context<Self>) -> Self {
        let (bytes_tx, mut bytes_rx) = mpsc::channel::<Vec<u8>>(256);
        let reader_started = spawn_reader(process.reader, bytes_tx, process.closed.clone());
        let mut status_text = "Ready".to_string();
        match reader_started {
            Ok(()) => {
                let worker = cx.spawn(async move |terminal, cx| {
                    loop {
                        match bytes_rx.recv().await {
                            Some(bytes) if !bytes.is_empty() => {
                                if terminal
                                    .update(cx, |terminal, cx| {
                                        terminal.parser.advance(&mut terminal.term, &bytes);
                                        cx.notify();
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Some(_) | None => {
                                if let Err(error) = terminal.update(cx, |terminal, cx| {
                                    terminal.mark_exited(cx);
                                }) {
                                    log::debug!("Terminal gone before exit: {error:?}");
                                }
                                break;
                            }
                        }
                    }
                });
                worker.detach();
            }
            Err(error) => {
                process.closed.store(true, Ordering::Relaxed);
                status_text = format!("Terminal reader failed: {error}");
            }
        }
        Self {
            term: Term::new(
                Config {
                    scrolling_history: 4000,
                    ..Default::default()
                },
                &ScreenSize {
                    columns: 100,
                    lines: 12,
                },
                VoidListener,
            ),
            parser: ansi::Processor::new(),
            child: Some(process.child),
            master: Some(process.master),
            writer: Some(process.writer),
            closed: process.closed,
            status_text,
            layout: None,
            selecting: false,
        }
    }

    fn mark_exited(&mut self, cx: &mut Context<Self>) {
        self.closed.store(true, Ordering::Relaxed);
        self.writer.take();
        self.master.take();
        self.reap_child();
        self.status_text = "Shell exited".into();
        cx.notify();
    }

    fn reap_child(&mut self) {
        let Some(child) = self.child.take() else {
            return;
        };
        let spawned = std::thread::Builder::new()
            .name("terminal-reaper".to_string())
            .spawn(move || {
                let mut child = child;
                let _ = child.wait();
            });
        if let Err(error) = spawned {
            log::debug!("Could not reap terminal child: {error:?}");
        }
    }

    fn stop(&mut self) {
        if self.closed.swap(true, Ordering::Relaxed) {
            return;
        }
        if let Some(child) = self.child.as_mut()
            && let Err(error) = child.kill()
        {
            log::debug!("Terminal child kill failed: {error:?}");
        }
        self.reap_child();
        self.writer.take();
        self.master.take();
        self.status_text = "Shell stopped".into();
    }

    fn input(&mut self, event: &KeyDownEvent) -> bool {
        let Some(bytes) = key_bytes(event) else {
            return false;
        };
        let Some(writer) = self.writer.as_mut() else {
            return false;
        };
        if let Err(error) = writer.write_all(&bytes) {
            self.status_text = format!("Input failed: {error}");
            return true;
        }
        let _ = writer.flush();
        self.term.scroll_display(Scroll::Bottom);
        true
    }

    fn paste(&mut self, text: &str) {
        let Some(writer) = self.writer.as_mut() else {
            return;
        };
        let normalized = text.replace("\r\n", "\r").replace('\n', "\r");
        if let Err(error) = writer.write_all(normalized.as_bytes()) {
            self.status_text = format!("Paste failed: {error}");
            return;
        }
        let _ = writer.flush();
        self.term.scroll_display(Scroll::Bottom);
    }

    fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
    }

    fn resize(&mut self, columns: usize, lines: usize) -> Result<()> {
        self.term.resize(ScreenSize { columns, lines });
        let Some(master) = self.master.as_ref() else {
            return Ok(()); // 已退出的终端只更新显示网格。
        };
        master.resize(portable_pty::PtySize {
            rows: lines.min(u16::MAX as usize) as u16,
            cols: columns.min(u16::MAX as usize) as u16,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    fn snapshot(&self) -> TerminalScreen {
        let content = self.term.renderable_content();
        let columns = self.term.columns();
        let lines = self.term.screen_lines();
        let offset = content.display_offset as i32;
        let mut rows: Vec<Vec<TerminalCell>> = Vec::with_capacity(lines);
        for _ in 0..lines {
            rows.push(Vec::with_capacity(columns));
        }
        for indexed in content.display_iter {
            let row_index = indexed.point.line.0 + offset;
            if row_index < 0 || row_index as usize >= lines {
                continue;
            }
            let column = indexed.point.column.0;
            if column >= columns {
                continue;
            }
            let selected = content
                .selection
                .as_ref()
                .is_some_and(|range| range.contains(indexed.point));
            rows[row_index as usize].push(TerminalCell {
                text: indexed.cell.c.to_string(),
                foreground: ansi_foreground(indexed.cell.fg),
                selected,
            });
        }
        let cursor = if content.display_offset == 0 {
            let cursor_point = content.cursor.point;
            if cursor_point.line.0 >= 0
                && (cursor_point.line.0 as usize) < lines
                && cursor_point.column.0 < columns
            {
                Some((cursor_point.line.0 as usize, cursor_point.column.0))
            } else {
                None
            }
        } else {
            None
        };
        TerminalScreen { rows, cursor }
    }

    fn point_for_position(&self, position: gpui::Point<gpui::Pixels>) -> Option<Point> {
        let layout = self.layout?;
        let local_x = f32::from(position.x - layout.bounds.origin.x);
        let local_y = f32::from(position.y - layout.bounds.origin.y);
        let column =
            ((local_x / layout.cell_width).floor() as usize).min(layout.columns.saturating_sub(1));
        let row =
            ((local_y / layout.line_height).floor() as usize).min(layout.lines.saturating_sub(1));
        let line = row as i32 - self.term.grid().display_offset() as i32;
        Some(Point::new(Line(line), Column(column)))
    }

    fn start_selection(&mut self, position: Point) {
        self.term.selection = Some(Selection::new(SelectionType::Simple, position, Side::Left));
        self.selecting = true;
    }

    fn update_selection(&mut self, position: Point) {
        if let Some(selection) = self.term.selection.as_mut() {
            selection.update(position, Side::Right);
        }
    }

    fn end_selection(&mut self) {
        self.selecting = false;
    }

    fn select_all(&mut self) {
        let mut selection = Selection::new(
            SelectionType::Lines,
            Point::new(self.term.topmost_line(), Column(0)),
            Side::Left,
        );
        selection.include_all();
        self.term.selection = Some(selection);
    }

    fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    fn has_selection(&self) -> bool {
        self.term
            .selection_to_string()
            .is_some_and(|text| !text.is_empty())
    }

    fn copy(&self) -> Option<String> {
        self.term.selection_to_string()
    }
}

impl Drop for TerminalModel {
    fn drop(&mut self) {
        self.stop();
    }
}

fn spawn_shell(program: &str, root: &Path) -> Result<ShellProcess> {
    let pair = native_pty_system().openpty(portable_pty::PtySize {
        rows: 12,
        cols: 100,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut command = CommandBuilder::new(program);
    command.cwd(root);
    let child = pair.slave.spawn_command(command)?;
    let writer = pair.master.take_writer()?;
    let reader = pair.master.try_clone_reader()?;
    Ok(ShellProcess {
        child,
        master: pair.master,
        writer,
        reader,
        closed: Arc::new(AtomicBool::new(false)),
    })
}

fn key_bytes(event: &KeyDownEvent) -> Option<Vec<u8>> {
    let key = event.keystroke.key.as_str();
    let modifiers = event.keystroke.modifiers;
    if modifiers.platform {
        return None;
    }
    if modifiers.control && key.len() == 1 {
        let letter = key.bytes().next()?;
        if letter.is_ascii_alphabetic() {
            return Some(vec![letter.to_ascii_uppercase() - b'A' + 1]);
        }
    }
    match key {
        "enter" => Some(b"\r".to_vec()),
        "space" => Some(b" ".to_vec()),
        "backspace" => Some(vec![0x7f]),
        "tab" => Some(b"\t".to_vec()),
        "escape" => Some(vec![0x1b]),
        "left" => Some(b"\x1b[D".to_vec()),
        "right" => Some(b"\x1b[C".to_vec()),
        "up" => Some(b"\x1b[A".to_vec()),
        "down" => Some(b"\x1b[B".to_vec()),
        "delete" => Some(b"\x1b[3~".to_vec()),
        "home" => Some(b"\x1b[H".to_vec()),
        "end" => Some(b"\x1b[F".to_vec()),
        "pageup" => Some(b"\x1b[5~".to_vec()),
        "pagedown" => Some(b"\x1b[6~".to_vec()),
        _ if !modifiers.control && key.chars().count() == 1 => Some(key.as_bytes().to_vec()),
        _ => None,
    }
}

fn spawn_reader(
    reader: Box<dyn Read + Send>,
    sender: mpsc::Sender<Vec<u8>>,
    closed: Arc<AtomicBool>,
) -> Result<()> {
    std::thread::Builder::new()
        .name("terminal-reader".to_string())
        .spawn(move || {
            let mut reader = reader;
            let mut buffer = [0u8; 8192];
            loop {
                if closed.load(Ordering::Relaxed) {
                    break;
                }
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if sender.blocking_send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        })?;
    Ok(())
}

fn ansi_foreground(color: Color) -> u32 {
    match color {
        Color::Named(named) => named_color(named),
        Color::Spec(spec) => ((spec.r as u32) << 16) | ((spec.g as u32) << 8) | spec.b as u32,
        Color::Indexed(index) => indexed_color(index),
    }
}

fn named_color(named: NamedColor) -> u32 {
    match named {
        NamedColor::Black => 0x3b3e48,
        NamedColor::Red => 0xef4444,
        NamedColor::Green => 0x10b981,
        NamedColor::Yellow => 0xf59e0b,
        NamedColor::Blue => 0x60a5fa,
        NamedColor::Magenta => 0xc084fc,
        NamedColor::Cyan => 0x22d3ee,
        NamedColor::White => 0xd7dae3,
        NamedColor::BrightBlack => 0x6b7280,
        NamedColor::BrightRed => 0xf87171,
        NamedColor::BrightGreen => 0x34d399,
        NamedColor::BrightYellow => 0xfbbf24,
        NamedColor::BrightBlue => 0x93c5fd,
        NamedColor::BrightMagenta => 0xd8b4fe,
        NamedColor::BrightCyan => 0x67e8f9,
        NamedColor::BrightWhite | NamedColor::Foreground | NamedColor::BrightForeground => TEXT,
        NamedColor::Background => BACKGROUND,
        _ => TEXT,
    }
}

fn indexed_color(index: u8) -> u32 {
    const BASE: [u32; 16] = [
        0x3b3e48, 0xef4444, 0x10b981, 0xf59e0b, 0x60a5fa, 0xc084fc, 0x22d3ee, 0xd7dae3, 0x6b7280,
        0xf87171, 0x34d399, 0xfbbf24, 0x93c5fd, 0xd8b4fe, 0x67e8f9, 0xf1f3f9,
    ];
    const LEVELS: [u32; 6] = [0, 95, 135, 175, 215, 255];
    match index {
        0..=15 => BASE[index as usize],
        16..=231 => {
            let value = index - 16;
            let red = LEVELS[(value / 36) as usize];
            let green = LEVELS[((value % 36) / 6) as usize];
            let blue = LEVELS[(value % 6) as usize];
            (red << 16) | (green << 8) | blue
        }
        232..=255 => {
            let gray = 8 + (index as u32 - 232) * 10;
            (gray << 16) | (gray << 8) | gray
        }
    }
}
