//! Top toolbar containing branding, project status, workspace tabs, and layout toggles.

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::theme::metrics;
use crate::workspace::layout::WorkspaceLayoutState;
use crate::workspace::navigation::{WorkspaceTab, render_workspace_tabs};

/// Identifiers for toggleable workspace panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Media,
    Inspector,
    Timeline,
    Assistant,
}

/// Actions and callback dispatchers for the top toolbar.
pub struct ToolbarActions {
    pub can_undo: bool,
    pub can_redo: bool,
    pub on_undo: Rc<dyn Fn(&mut Window, &mut App)>,
    pub on_redo: Rc<dyn Fn(&mut Window, &mut App)>,
    pub on_tab_change: Rc<dyn Fn(&WorkspaceTab, &mut Window, &mut App)>,
    pub on_toggle: Rc<dyn Fn(&PanelId, &mut Window, &mut App)>,
    pub on_export: Rc<dyn Fn(&mut Window, &mut App)>,
}

impl std::fmt::Debug for ToolbarActions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolbarActions")
            .field("can_undo", &self.can_undo)
            .field("can_redo", &self.can_redo)
            .finish_non_exhaustive()
    }
}

/// Render the compact top toolbar.
pub fn render_top_toolbar(
    state: &WorkspaceLayoutState,
    actions: ToolbarActions,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let project_name = state.project_name.clone();
    let show_media = state.show_media_browser;
    let show_inspector = state.show_inspector;
    let show_timeline = state.show_timeline;
    let show_assistant = state.show_assistant;
    let on_toggle = actions.on_toggle;
    let on_undo = actions.on_undo;
    let on_redo = actions.on_redo;
    let on_tab_change = actions.on_tab_change;
    let on_export = actions.on_export;
    let can_undo = actions.can_undo;
    let can_redo = actions.can_redo;

    div()
        .flex()
        .items_center()
        .justify_between()
        .h(metrics::TOOLBAR_HEIGHT)
        .px_3()
        .border_b_1()
        .border_color(theme.border)
        .bg(theme.secondary)
        .child(
            // Left region: Brand, Project name, Undo/Redo
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .child(Icon::new(IconName::Film))
                        .child(
                            div()
                                .text_sm()
                                .font_bold()
                                .text_color(theme.foreground)
                                .child("NodaVideo"),
                        ),
                )
                .child(div().h_4().w(px(1.0)).bg(theme.border))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .px_2()
                        .py_0p5()
                        .rounded_sm()
                        .bg(theme.muted)
                        .child(
                            div()
                                .text_xs()
                                .font_medium()
                                .text_color(theme.foreground)
                                .child(project_name),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_0p5()
                        .child(
                            Button::new("tb-undo")
                                .ghost()
                                .xsmall()
                                .icon(IconName::Undo)
                                .tooltip("Undo (Cmd+Z)")
                                .when(!can_undo, |btn| btn.ghost())
                                .on_click(move |_, window, cx| on_undo(window, cx)),
                        )
                        .child(
                            Button::new("tb-redo")
                                .ghost()
                                .xsmall()
                                .icon(IconName::Redo)
                                .tooltip("Redo (Cmd+Shift+Z)")
                                .when(!can_redo, |btn| btn.ghost())
                                .on_click(move |_, window, cx| on_redo(window, cx)),
                        ),
                ),
        )
        .child(
            // Center region: Workspace navigation tabs
            div()
                .flex()
                .items_center()
                .justify_center()
                .child(render_workspace_tabs(
                    state.active_tab,
                    move |tab, win, app| on_tab_change(tab, win, app),
                )),
        )
        .child(
            // Right region: Export & Panel visibility toggles
            div()
                .flex()
                .items_center()
                .gap_1()
                .child({
                    let export = on_export;
                    Button::new("tb-export")
                        .primary()
                        .xsmall()
                        .icon(IconName::Share2)
                        .label("Export")
                        .tooltip("Export Video (Cmd+E / Ctrl+E)")
                        .on_click(move |_, window, cx| {
                            export(window, cx);
                        })
                })
                .child(div().h_4().w(px(1.0)).bg(theme.border).mx_1())
                .child({
                    let toggle = on_toggle.clone();
                    Button::new("toggle-media-bin")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Folder)
                        .when(show_media, |btn| btn.outline())
                        .on_click(move |_, window, cx| {
                            toggle(&PanelId::Media, window, cx);
                        })
                })
                .child({
                    let toggle = on_toggle.clone();
                    Button::new("toggle-inspector")
                        .ghost()
                        .xsmall()
                        .icon(IconName::SlidersHorizontal)
                        .when(show_inspector, |btn| btn.outline())
                        .on_click(move |_, window, cx| {
                            toggle(&PanelId::Inspector, window, cx);
                        })
                })
                .child({
                    let toggle = on_toggle.clone();
                    Button::new("toggle-timeline")
                        .ghost()
                        .xsmall()
                        .icon(IconName::LayoutGrid)
                        .when(show_timeline, |btn| btn.outline())
                        .on_click(move |_, window, cx| {
                            toggle(&PanelId::Timeline, window, cx);
                        })
                })
                .child(div().h_4().w(px(1.0)).bg(theme.border).mx_1())
                .child({
                    let toggle = on_toggle;
                    Button::new("toggle-assistant")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Sparkles)
                        .label("Assistant")
                        .when(show_assistant, |btn| btn.primary())
                        .on_click(move |_, window, cx| {
                            toggle(&PanelId::Assistant, window, cx);
                        })
                }),
        )
}
