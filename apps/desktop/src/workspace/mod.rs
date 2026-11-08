//! Main editing workspace coordinating top toolbar, panels, and resizable layout.

pub mod layout;
pub mod menus;
pub mod navigation;
pub mod panels;
pub mod toolbar;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::resizable::{ResizableState, h_resizable, resizable_panel, v_resizable};
use gpui_kit::*;

use crate::theme::metrics;
use crate::workspace::layout::WorkspaceLayoutState;
use crate::workspace::menus::*;
use crate::workspace::navigation::WorkspaceTab;
use crate::workspace::panels::*;
use crate::workspace::toolbar::{PanelId, render_top_toolbar};

/// Root workspace view managing panels, docking/resizing, and active workspace tab.
#[derive(Debug)]
pub struct WorkspaceView {
    pub state: WorkspaceLayoutState,
    pub media_browser: Entity<MediaBrowserPanel>,
    pub preview_monitor: Entity<PreviewMonitorPanel>,
    pub inspector: Entity<InspectorPanel>,
    pub timeline: Entity<TimelinePanel>,
    pub assistant: Entity<AssistantPanel>,
    pub h_split_state: Entity<ResizableState>,
    pub v_split_state: Entity<ResizableState>,
}

impl WorkspaceView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let media_browser = cx.new(|cx| MediaBrowserPanel::new(window, cx));
        let preview_monitor = cx.new(|_| PreviewMonitorPanel::new());
        let inspector = cx.new(|_| InspectorPanel::new());
        let timeline = cx.new(|_| TimelinePanel::new());
        let assistant = cx.new(|cx| AssistantPanel::new(window, cx));

        let h_split_state = cx.new(|_| ResizableState::default());
        let v_split_state = cx.new(|_| ResizableState::default());

        Self {
            state: WorkspaceLayoutState::default(),
            media_browser,
            preview_monitor,
            inspector,
            timeline,
            assistant,
            h_split_state,
            v_split_state,
        }
    }

    pub fn set_workspace_tab(&mut self, tab: WorkspaceTab, cx: &mut Context<Self>) {
        self.state.active_tab = tab;
        cx.notify();
    }

    pub fn toggle_panel(&mut self, panel: PanelId, cx: &mut Context<Self>) {
        match panel {
            PanelId::Media => self.state.toggle_media_browser(),
            PanelId::Inspector => self.state.toggle_inspector(),
            PanelId::Timeline => self.state.toggle_timeline(),
            PanelId::Assistant => self.state.toggle_assistant(),
        }
        cx.notify();
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let active_tab = self.state.active_tab;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            // Menu / Action Listeners
            .on_action(cx.listener(|_, _: &QuitAction, _, cx| {
                cx.quit();
            }))
            .on_action(cx.listener(|this, _: &ToggleMediaBrowser, _, cx| {
                this.toggle_panel(PanelId::Media, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleInspector, _, cx| {
                this.toggle_panel(PanelId::Inspector, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleTimeline, _, cx| {
                this.toggle_panel(PanelId::Timeline, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleAssistant, _, cx| {
                this.toggle_panel(PanelId::Assistant, cx);
            }))
            .on_action(cx.listener(|this, _: &ResetWorkspaceLayout, _, cx| {
                this.state.reset_layout();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectEditTab, _, cx| {
                this.set_workspace_tab(WorkspaceTab::Edit, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectColorTab, _, cx| {
                this.set_workspace_tab(WorkspaceTab::Color, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectAudioTab, _, cx| {
                this.set_workspace_tab(WorkspaceTab::Audio, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectAiStudioTab, _, cx| {
                this.set_workspace_tab(WorkspaceTab::AiStudio, cx);
            }))
            .on_action(cx.listener(|this, _: &PlayPauseAction, _, cx| {
                this.preview_monitor.update(cx, |mon, cx| {
                    mon.toggle_playback(cx);
                });
            }))
            .on_action(cx.listener(|this, _: &LoopPlaybackAction, _, cx| {
                this.preview_monitor.update(cx, |mon, cx| {
                    mon.toggle_loop(cx);
                });
            }))
            .child(
                // Top Toolbar
                render_top_toolbar(
                    &self.state,
                    cx.listener(|this, tab: &WorkspaceTab, _, cx| {
                        this.set_workspace_tab(*tab, cx);
                    }),
                    cx.listener(|this, panel: &PanelId, _, cx| {
                        this.toggle_panel(*panel, cx);
                    }),
                    cx,
                ),
            )
            .child(
                // Workspace Body (Edit vs Other modes)
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(if active_tab == WorkspaceTab::Edit {
                        self.render_edit_workspace(cx).into_any_element()
                    } else {
                        render_placeholder_workspace(active_tab, cx).into_any_element()
                    }),
            )
            .child(
                // Bottom Window Status Bar
                self.render_status_bar(cx),
            )
    }
}

impl WorkspaceView {
    fn render_edit_workspace(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let show_media = self.state.show_media_browser;
        let show_inspector = self.state.show_inspector;
        let show_timeline = self.state.show_timeline;
        let show_assistant = self.state.show_assistant;

        // Build the top horizontal split group
        let mut top_split = h_resizable("h-top-split").with_state(&self.h_split_state);

        if show_media {
            top_split = top_split.child(
                resizable_panel()
                    .size(metrics::MEDIA_BROWSER_DEFAULT_WIDTH)
                    .size_range(metrics::MEDIA_BROWSER_MIN_WIDTH..metrics::MEDIA_BROWSER_MAX_WIDTH)
                    .fixed()
                    .child(self.media_browser.clone()),
            );
        }

        // Center Preview Monitor takes flexible remaining space
        top_split = top_split.child(resizable_panel().child(self.preview_monitor.clone()));

        if show_inspector {
            top_split = top_split.child(
                resizable_panel()
                    .size(metrics::INSPECTOR_DEFAULT_WIDTH)
                    .size_range(metrics::INSPECTOR_MIN_WIDTH..metrics::INSPECTOR_MAX_WIDTH)
                    .fixed()
                    .child(self.inspector.clone()),
            );
        }

        if show_assistant {
            top_split = top_split.child(
                resizable_panel()
                    .size(metrics::ASSISTANT_DEFAULT_WIDTH)
                    .size_range(metrics::ASSISTANT_MIN_WIDTH..metrics::ASSISTANT_MAX_WIDTH)
                    .fixed()
                    .child(self.assistant.clone()),
            );
        }

        // Now compose with vertical split if Timeline is visible
        if show_timeline {
            v_resizable("v-main-split")
                .with_state(&self.v_split_state)
                .child(resizable_panel().child(top_split))
                .child(
                    resizable_panel()
                        .size(metrics::TIMELINE_DEFAULT_HEIGHT)
                        .size_range(metrics::TIMELINE_MIN_HEIGHT..metrics::TIMELINE_MAX_HEIGHT)
                        .fixed()
                        .child(self.timeline.clone()),
                )
        } else {
            v_resizable("v-main-split")
                .with_state(&self.v_split_state)
                .child(resizable_panel().child(top_split))
        }
    }

    fn render_status_bar(&self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        let mono_font = theme.mono_font_family.clone();

        div()
            .flex()
            .items_center()
            .justify_between()
            .h(metrics::STATUS_BAR_HEIGHT)
            .px_3()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.secondary)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font.clone())
                            .text_color(theme.muted_foreground)
                            .child("24.00 fps"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("1920×1080"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("48 kHz 24-bit Stereo"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(div().w_2().h_2().rounded_full().bg(theme.muted_foreground))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("AI Offline (M4)"),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font)
                            .text_color(theme.muted_foreground)
                            .child("NodaVideo v0.1.0-alpha"),
                    ),
            )
    }
}
