//! Professional button components for NodaVideo.

use crate::theme::{ACCENT, ACCENT_HOVER, BORDER, MUTED, SURFACE, SURFACE_HOVER, TEXT};
use gpui::prelude::*;
use gpui::{App, Div, ElementId, FontWeight, Stateful, Window, div, px, rgb};

/// Primary button: Solid Indigo accent with subtle shadow and rounded corners.
pub(crate) fn button_cyberpunk_primary(
    id: impl Into<ElementId>,
    label: &str,
    enabled: bool,
    _window: &mut Window,
    _cx: &mut App,
) -> Stateful<Div> {
    let element_id = id.into();
    if !enabled {
        return div()
            .id(element_id)
            .h_8()
            .px_4()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.0))
            .bg(rgb(SURFACE))
            .text_color(rgb(MUTED))
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .cursor_not_allowed()
            .child(label.to_string());
    }

    div()
        .id(element_id)
        .h_8()
        .px_4()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .bg(rgb(ACCENT))
        .hover(|style| style.bg(rgb(ACCENT_HOVER)))
        .active(|style| style.bg(rgb(ACCENT_HOVER)).opacity(0.9))
        .text_color(rgb(TEXT))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .child(label.to_string())
}

/// Secondary button: Subtle surface with border and hover highlight.
pub(crate) fn button_cyberpunk_secondary(
    id: impl Into<ElementId>,
    label: &str,
    enabled: bool,
    _window: &mut Window,
    _cx: &mut App,
) -> Stateful<Div> {
    let element_id = id.into();
    if !enabled {
        return div()
            .id(element_id)
            .h_8()
            .px_4()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.0))
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(BORDER))
            .text_color(rgb(MUTED))
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .cursor_not_allowed()
            .child(label.to_string());
    }

    div()
        .id(element_id)
        .h_8()
        .px_4()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .bg(rgb(SURFACE))
        .border_1()
        .border_color(rgb(BORDER))
        .hover(|style| style.bg(rgb(SURFACE_HOVER)).border_color(rgb(ACCENT)))
        .active(|style| style.bg(rgb(SURFACE_HOVER)).opacity(0.85))
        .text_color(rgb(TEXT))
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .child(label.to_string())
}
