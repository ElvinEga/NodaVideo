use crate::editor::Editor;
use crate::theme::{BORDER, MUTED, PANEL, SURFACE, SURFACE_HOVER, TEXT};
use gpui::prelude::*;
use gpui::{AnyElement, CursorStyle, FontWeight, MouseButton, div, px, rgb};

// 与 actions.rs 中的实际绑定保持一致；只列出真实存在的快捷键。
const SECTIONS: &[(&str, &[(&str, &[&str])])] = &[
    (
        "Playback",
        &[
            ("Play / Pause", &["Space"]),
            ("Step backward one frame", &["←"]),
            ("Step forward one frame", &["→"]),
            ("Jump to start", &["Home"]),
            ("Jump to end", &["End"]),
        ],
    ),
    (
        "Editing",
        &[
            ("Undo", &["⌘", "Z"]),
            ("Redo", &["⌘", "⇧", "Z"]),
            ("Cut", &["⌘", "X"]),
            ("Copy", &["⌘", "C"]),
            ("Paste", &["⌘", "V"]),
            ("Duplicate", &["⌘", "D"]),
            ("Delete selected", &["⌫"]),
            ("Select all unlocked clips", &["⌘", "A"]),
            ("Split clip at playhead", &["⌘", "B"]),
            ("Selection tool", &["V"]),
            ("Blade tool", &["B"]),
            ("Select clips left of pointer", &["["]),
            ("Select clips right of pointer", &["]"]),
            ("Mark source in point", &["I"]),
            ("Mark source out point", &["O"]),
            ("Insert source to timeline", &["."]),
        ],
    ),
    (
        "File",
        &[
            ("New project", &["⌘", "N"]),
            ("New timeline", &["⌘", "⇧", "N"]),
            ("Open project folder", &["⌘", "O"]),
            ("Open project picker", &["⌘", "⇧", "O"]),
            ("Save", &["⌘", "S"]),
            ("Import media", &["⌘", "I"]),
            ("Export video", &["⌘", "E"]),
            ("Reveal in Finder", &["⌘", "⌥", "R"]),
            ("Open in default app", &["⌃", "⇧", "↩"]),
            ("Close window", &["⌘", "W"]),
        ],
    ),
    (
        "View",
        &[
            ("Toggle media browser", &["⌘", "1"]),
            ("Toggle inspector", &["⌘", "2"]),
            ("Toggle AI assistant", &["⌘", "⇧", "A"]),
            ("Fullscreen preview", &["⌘", "F"]),
            ("Exit fullscreen / close dialogs", &["Esc"]),
            ("Edit workspace", &["⌘", "⌥", "1"]),
            ("Color workspace", &["⌘", "⌥", "2"]),
            ("Audio workspace", &["⌘", "⌥", "3"]),
            ("AI Studio workspace", &["⌘", "⌥", "4"]),
            ("Settings", &["⌘", ","]),
            ("Debug inspector", &["⌘", "⌥", "I"]),
        ],
    ),
    (
        "Window",
        &[
            ("Minimize", &["⌘", "M"]),
            ("Send assistant message", &["↩"]),
            ("Stop assistant", &["Esc"]),
        ],
    ),
];

pub(crate) fn keyboard_shortcuts_view(cx: &mut Context<Editor>) -> AnyElement {
    div()
        .id("keyboard-shortcuts-overlay")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .occlude()
        .bg(gpui::rgba(0x000000b3))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|editor, _, _, cx| {
                editor.shortcuts_open = false;
                cx.notify();
            }),
        )
        .child(
            div()
                .id("keyboard-shortcuts-modal")
                .w(px(620.0))
                .max_h(px(720.0))
                .flex()
                .flex_col()
                .rounded_xl()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(PANEL))
                .shadow_lg()
                .occlude()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_, _, _, cx| cx.stop_propagation()),
                )
                .child(
                    div()
                        .h(px(58.0))
                        .px_5()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_between()
                        .border_b_1()
                        .border_color(rgb(BORDER))
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Keyboard Shortcuts"),
                        )
                        .child(
                            div()
                                .id("close-keyboard-shortcuts")
                                .size_8()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_md()
                                .cursor(CursorStyle::PointingHand)
                                .text_color(rgb(MUTED))
                                .hover(|style| style.bg(rgb(SURFACE_HOVER)).text_color(rgb(TEXT)))
                                .child("×")
                                .on_click(cx.listener(|editor, _, _, cx| {
                                    editor.shortcuts_open = false;
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .id("keyboard-shortcuts-scroll")
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .px_5()
                        .py_4()
                        .overflow_y_scroll()
                        .children(
                            SECTIONS
                                .iter()
                                .map(|(title, rows)| shortcut_section(title, rows)),
                        ),
                ),
        )
        .into_any_element()
}

fn shortcut_section(title: &str, rows: &[(&str, &[&str])]) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(MUTED))
                .child(title.to_string()),
        )
        .children(rows.iter().map(|(label, keys)| shortcut_row(label, keys)))
}

fn shortcut_row(label: &str, keys: &[&str]) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .py_1()
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(TEXT))
                .child(label.to_string()),
        )
        .child(
            div()
                .flex()
                .gap_1()
                .children(keys.iter().map(|key| key_chip(key))),
        )
}

fn key_chip(key: &str) -> impl IntoElement {
    div()
        .px_1p5()
        .py_0p5()
        .rounded(px(4.0))
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .text_size(px(11.0))
        .text_color(rgb(MUTED))
        .child(key.to_string())
}
