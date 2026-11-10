//! Main editing workspace coordinating top toolbar, panels, and resizable layout.

pub mod layout;
pub mod menus;
pub mod navigation;
pub mod panels;
pub mod toolbar;

use std::rc::Rc;
use std::sync::{Arc, RwLock};

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::resizable::{ResizableState, h_resizable, resizable_panel, v_resizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use nodavideo_agent_acp::AcpAgentProvider;
use nodavideo_agent_runtime::{AgentRuntime, MockAgentProvider};
use nodavideo_mcp_server::McpExecutor;
use nodavideo_media_engine::MediaProcessor;
use nodavideo_skill_registry::SkillRegistry;

use crate::theme::metrics;
use crate::workspace::layout::WorkspaceLayoutState;
use crate::workspace::menus::*;
use crate::workspace::navigation::WorkspaceTab;
use crate::workspace::panels::*;
use crate::workspace::toolbar::{PanelId, ToolbarActions, render_top_toolbar};

/// Root workspace view managing panels, docking/resizing, and active workspace tab.
pub struct WorkspaceView {
    pub state: WorkspaceLayoutState,
    pub media_browser: Entity<MediaBrowserPanel>,
    pub preview_monitor: Entity<PreviewMonitorPanel>,
    pub inspector: Entity<InspectorPanel>,
    pub timeline: Entity<TimelinePanel>,
    pub assistant: Entity<AssistantPanel>,
    pub h_split_state: Entity<ResizableState>,
    pub v_split_state: Entity<ResizableState>,
    pub processor: Arc<MediaProcessor>,
    pub runtime: Arc<AgentRuntime>,
    pub skill_registry: Arc<RwLock<SkillRegistry>>,
    pub export_status: Option<String>,
    _observe_sub: Subscription,
    _preview_sub: Subscription,
    _timeline_sub: Subscription,
    _assistant_sub: Subscription,
    playback_task: Option<Task<()>>,
    preview_frame_task: Option<Task<()>>,
}

impl std::fmt::Debug for WorkspaceView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceView")
            .field("state", &self.state)
            .field("export_status", &self.export_status)
            .finish_non_exhaustive()
    }
}

impl WorkspaceView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let media_browser = cx.new(|cx| MediaBrowserPanel::new(window, cx));
        let preview_monitor = cx.new(|_| PreviewMonitorPanel::new());
        let inspector = cx.new(|_| InspectorPanel::new());
        let timeline = cx.new(|_| TimelinePanel::new());
        let processor = media_browser.read(cx).processor.clone();

        // Initialize AgentRuntime and register pluggable providers
        let runtime = Arc::new(AgentRuntime::new());
        let rt_clone = runtime.clone();
        cx.spawn(async move |_, _| {
            rt_clone
                .register_provider(Arc::new(AcpAgentProvider::antigravity()))
                .await;
            rt_clone
                .register_provider(Arc::new(AcpAgentProvider::gemini()))
                .await;
            rt_clone
                .register_provider(Arc::new(MockAgentProvider::new()))
                .await;
        })
        .detach();

        // Discover available skills from skills/ directory
        let mut skill_reg = SkillRegistry::new();
        let _ = skill_reg.discover_from_dir(std::path::Path::new("skills"));
        let skill_registry = Arc::new(RwLock::new(skill_reg));

        let assistant =
            cx.new(|cx| AssistantPanel::new(runtime.clone(), skill_registry.clone(), window, cx));

        // Wire assistant tool execution callback to apply approved edits directly to the timeline
        let tl_for_tool = timeline.clone();
        let mb_for_tool = media_browser.clone();
        assistant.update(cx, |asst, _| {
            asst.on_tool_call = Some(Rc::new(move |tool_name, params, _, cx| {
                let mut result = None;
                tl_for_tool.update(cx, |tl, cx| {
                    let project = mb_for_tool.read(cx).project.clone();
                    match McpExecutor::execute(
                        tool_name,
                        params,
                        &project,
                        &mut tl.sequence,
                        &mut tl.history,
                        true,
                    ) {
                        Ok(val) => {
                            cx.notify();
                            result = Some(Ok(val));
                        }
                        Err(err) => {
                            result = Some(Err(err.to_string()));
                        }
                    }
                });
                result.unwrap_or_else(|| Err("Timeline unavailable".into()))
            }));
        });

        // Wire transport buttons on Preview Monitor
        let tl_back = timeline.clone();
        let tl_fwd = timeline.clone();
        preview_monitor.update(cx, |mon, _| {
            mon.on_step_backward = Some(std::rc::Rc::new(move |_, cx| {
                tl_back.update(cx, |tl, cx| tl.step_backward(1, cx));
            }));
            mon.on_step_forward = Some(std::rc::Rc::new(move |_, cx| {
                tl_fwd.update(cx, |tl, cx| tl.step_forward(1, cx));
            }));
        });

        let h_split_state = cx.new(|_| ResizableState::default());
        let v_split_state = cx.new(|_| ResizableState::default());

        // Observe media browser updates to synchronize selected asset with Inspector and refresh preview
        let observe_sub = cx.observe(&media_browser, |this, mb, cx| {
            let selected_id = mb.read(cx).selected_asset_id;
            let selected_asset =
                selected_id.and_then(|id| mb.read(cx).project.get_asset(id).cloned());
            this.inspector.update(cx, |insp, cx| {
                if insp.selected_asset != selected_asset {
                    insp.set_selected_asset(selected_asset, cx);
                }
            });
            this.refresh_preview_frame(cx);
        });

        // Observe preview monitor for playback state changes
        let preview_sub = cx.observe(&preview_monitor, |this, pm, cx| {
            let is_playing = pm.read(cx).is_playing;
            if is_playing && this.playback_task.is_none() {
                this.start_playback_loop(cx);
            } else if !is_playing && this.playback_task.is_some() {
                this.stop_playback_loop();
            }
        });

        // Observe timeline for changes in playhead, clips, tracks, or sequence
        let timeline_sub = cx.observe(&timeline, |this, _, cx| {
            this.refresh_preview_frame(cx);
        });

        // Observe assistant panel to update bottom status bar
        let assistant_sub = cx.observe(&assistant, |_, _, cx| {
            cx.notify();
        });

        let mut view = Self {
            state: WorkspaceLayoutState::default(),
            media_browser,
            preview_monitor,
            inspector,
            timeline,
            assistant,
            h_split_state,
            v_split_state,
            processor,
            runtime,
            skill_registry,
            export_status: None,
            _observe_sub: observe_sub,
            _preview_sub: preview_sub,
            _timeline_sub: timeline_sub,
            _assistant_sub: assistant_sub,
            playback_task: None,
            preview_frame_task: None,
        };

        view.refresh_preview_frame(cx);
        view
    }

    /// Refresh current frame and timecodes on Preview Monitor based on current timeline state.
    pub fn refresh_preview_frame(&mut self, cx: &mut Context<Self>) {
        let sequence = self.timeline.read(cx).sequence.clone();
        let playhead = self.timeline.read(cx).playhead();
        let project = self.media_browser.read(cx).project.clone();
        let processor = self.processor.clone();

        let timecode = sequence.frame_to_timecode(playhead);
        let duration = sequence.duration_timecode();
        let seq_name = sequence.name.clone();
        let seq_info = format!(
            "{}×{} {}fps",
            sequence.width,
            sequence.height,
            sequence.fps.to_f64().round() as u64
        );

        self.preview_frame_task = Some(cx.spawn(async move |this, cx| {
            let frame_result = cx
                .background_spawn(
                    async move { processor.render_preview(&sequence, &project, playhead) },
                )
                .await;

            this.update(cx, |this, cx| {
                let frame_path = frame_result.ok().flatten();
                this.preview_monitor.update(cx, |mon, cx| {
                    mon.sequence_name = seq_name.into();
                    mon.sequence_info = seq_info.into();
                    mon.update_frame(frame_path, timecode, duration, cx);
                });
            })
            .ok();
        }));
    }

    /// Start the real-time playback timer advancing the playhead.
    pub fn start_playback_loop(&mut self, cx: &mut Context<Self>) {
        let fps = self.timeline.read(cx).sequence.fps;
        let fps_f64 = fps.to_f64().max(1.0);
        let interval_ms = (1000.0 / fps_f64).round() as u64;

        self.playback_task = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(interval_ms))
                    .await;

                let keep_running = this
                    .update(cx, |this, cx| {
                        if !this.preview_monitor.read(cx).is_playing {
                            return false;
                        }

                        let seq_duration = this.timeline.read(cx).sequence.duration_frames();
                        let current_playhead = this.timeline.read(cx).playhead();
                        let is_looping = this.preview_monitor.read(cx).is_looping;

                        if current_playhead >= seq_duration {
                            if is_looping && seq_duration > 0 {
                                this.timeline.update(cx, |tl, cx| tl.set_playhead(0, cx));
                                true
                            } else {
                                this.preview_monitor
                                    .update(cx, |mon, cx| mon.set_playing(false, cx));
                                false
                            }
                        } else {
                            this.timeline.update(cx, |tl, cx| tl.step_forward(1, cx));
                            true
                        }
                    })
                    .unwrap_or(false);

                if !keep_running {
                    break;
                }
            }
        }));
    }

    /// Stop the playback loop.
    pub fn stop_playback_loop(&mut self) {
        self.playback_task = None;
    }

    /// Prompt user for export path and launch asynchronous MP4 export pipeline.
    pub fn export_video_prompt(&mut self, cx: &mut Context<Self>) {
        let sequence = self.timeline.read(cx).sequence.clone();
        let project = self.media_browser.read(cx).project.clone();

        if sequence.duration_frames() == 0 {
            self.export_status =
                Some("Cannot export empty sequence (add clips to timeline)".into());
            cx.notify();
            return;
        }

        let default_filename = format!("{}.mp4", sequence.name.replace(' ', "_").to_lowercase());
        let receiver = cx.prompt_for_new_path(std::path::Path::new("."), Some(&default_filename));

        let processor = self.processor.clone();
        self.export_status = Some("Selecting export destination...".into());
        cx.notify();

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut output_path))) = receiver.await {
                if output_path.extension().is_none() {
                    output_path.set_extension("mp4");
                }

                let filename = output_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                this.update(cx, |this, cx| {
                    this.export_status = Some(format!("Exporting: {filename}..."));
                    cx.notify();
                })
                .ok();

                let settings = nodavideo_media_engine::ExportSettings {
                    output_path: output_path.clone(),
                    width: sequence.width,
                    height: sequence.height,
                    fps: sequence.fps,
                    ..Default::default()
                };

                let start_time = std::time::Instant::now();
                let export_result = cx
                    .background_spawn(
                        async move { processor.export(&sequence, &project, &settings) },
                    )
                    .await;
                let elapsed = start_time.elapsed().as_secs_f64();

                this.update(cx, |this, cx| {
                    match export_result {
                        Ok(report) => {
                            let size_mb = (report.file_size_bytes as f64) / (1024.0 * 1024.0);
                            this.export_status = Some(format!(
                                "Export complete: {filename} ({size_mb:.1} MB in {elapsed:.1}s)"
                            ));
                        }
                        Err(err) => {
                            this.export_status = Some(format!("Export failed: {err}"));
                        }
                    }
                    cx.notify();
                })
                .ok();
            } else {
                this.update(cx, |this, cx| {
                    this.export_status = None;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
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

    pub fn new_project(&mut self, cx: &mut Context<Self>) {
        self.media_browser.update(cx, |mb, cx| {
            mb.project = nodavideo_project::ProjectDocument::default();
            mb.selected_asset_id = None;
            cx.notify();
        });
        self.timeline.update(cx, |tl, cx| {
            tl.set_sequence(nodavideo_timeline::Sequence::default_1080p24(), cx);
        });
        self.state.project_name = "Untitled Project".into();
        self.inspector.update(cx, |insp, cx| {
            insp.set_selected_asset(None, cx);
        });
        cx.notify();
    }

    pub fn save_current_project(&mut self, cx: &mut Context<Self>) {
        let mb = self.media_browser.read(cx);
        let mut project = mb.project.clone();
        let sequence = self.timeline.read(cx).sequence.clone();
        if let Err(err) = sequence.attach_to_project(&mut project) {
            eprintln!("Failed to attach sequence to project: {err}");
        }
        let receiver = cx.prompt_for_new_path(
            std::path::Path::new("."),
            Some(&format!("{}.nodavideo", project.name)),
        );

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = receiver.await {
                if let Err(err) = project.save_to_file(&path) {
                    eprintln!("Failed to save project: {err}");
                } else {
                    let name = path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Project")
                        .to_string();
                    this.update(cx, |this, cx| {
                        this.state.project_name = name.into();
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    pub fn open_project_prompt(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.into_iter().next() {
                    match nodavideo_project::ProjectDocument::load_from_file(&path) {
                        Ok(loaded_doc) => {
                            let loaded_seq =
                                nodavideo_timeline::Sequence::extract_from_project(&loaded_doc)
                                    .ok()
                                    .flatten()
                                    .unwrap_or_else(nodavideo_timeline::Sequence::default_1080p24);

                            this.update(cx, |this, cx| {
                                this.state.project_name = loaded_doc.name.clone().into();
                                this.timeline.update(cx, |tl, cx| {
                                    tl.set_sequence(loaded_seq, cx);
                                });
                                this.media_browser.update(cx, |mb, cx| {
                                    mb.project = loaded_doc;
                                    mb.selected_asset_id = None;
                                    cx.notify();
                                });
                                this.inspector.update(cx, |insp, cx| {
                                    insp.set_selected_asset(None, cx);
                                });
                                cx.notify();
                            })
                            .ok();
                        }
                        Err(err) => {
                            eprintln!("Failed to load project: {err}");
                        }
                    }
                }
            }
        })
        .detach();
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
            .on_action(cx.listener(|this, _: &NewProject, _, cx| {
                this.new_project(cx);
            }))
            .on_action(cx.listener(|this, _: &SaveProject, _, cx| {
                this.save_current_project(cx);
            }))
            .on_action(cx.listener(|this, _: &OpenProject, _, cx| {
                this.open_project_prompt(cx);
            }))
            .on_action(cx.listener(|this, _: &ImportMedia, _, cx| {
                this.media_browser.update(cx, |mb, cx| {
                    mb.prompt_import(cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ExportVideoAction, _, cx| {
                this.export_video_prompt(cx);
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
            .on_action(cx.listener(|this, _: &UndoAction, _, cx| {
                this.timeline.update(cx, |tl, cx| {
                    tl.undo(cx);
                });
            }))
            .on_action(cx.listener(|this, _: &RedoAction, _, cx| {
                this.timeline.update(cx, |tl, cx| {
                    tl.redo(cx);
                });
            }))
            .on_action(cx.listener(|this, _: &SplitClipAction, _, cx| {
                this.timeline.update(cx, |tl, cx| {
                    tl.split_at_playhead(cx);
                });
            }))
            .on_action(cx.listener(|this, _: &DeleteClipAction, _, cx| {
                this.timeline.update(cx, |tl, cx| {
                    tl.delete_selected(cx);
                });
            }))
            .on_action(cx.listener(|this, _: &StepForwardAction, _, cx| {
                this.timeline.update(cx, |tl, cx| {
                    tl.step_forward(1, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &StepBackwardAction, _, cx| {
                this.timeline.update(cx, |tl, cx| {
                    tl.step_backward(1, cx);
                });
            }))
            .child({
                let can_undo = self.timeline.read(cx).can_undo();
                let can_redo = self.timeline.read(cx).can_redo();
                let tl_undo = self.timeline.clone();
                let tl_redo = self.timeline.clone();
                let this_export = cx.entity();

                let tab_listener = cx.listener(|this, tab: &WorkspaceTab, _, cx| {
                    this.set_workspace_tab(*tab, cx);
                });
                let toggle_listener = cx.listener(|this, panel: &PanelId, _, cx| {
                    this.toggle_panel(*panel, cx);
                });

                let actions = ToolbarActions {
                    can_undo,
                    can_redo,
                    on_undo: std::rc::Rc::new(move |_, cx| {
                        tl_undo.update(cx, |tl, cx| tl.undo(cx));
                    }),
                    on_redo: std::rc::Rc::new(move |_, cx| {
                        tl_redo.update(cx, |tl, cx| tl.redo(cx));
                    }),
                    on_tab_change: std::rc::Rc::new(move |tab, win, app| {
                        tab_listener(tab, win, app);
                    }),
                    on_toggle: std::rc::Rc::new(move |panel, win, app| {
                        toggle_listener(panel, win, app);
                    }),
                    on_export: std::rc::Rc::new(move |_, cx| {
                        this_export.update(cx, |this, cx| {
                            this.export_video_prompt(cx);
                        });
                    }),
                };

                // Top Toolbar
                render_top_toolbar(&self.state, actions, cx)
            })
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
        let export_status = self.export_status.clone();

        let (dot_color, ai_text) = {
            let asst = self.assistant.read(cx);
            if asst.is_processing {
                (gpui::rgb(0xf59e0b), asst.status_text.clone())
            } else {
                (
                    gpui::rgb(0x22c55e),
                    format!("AI Ready ({})", asst.provider_display_name()),
                )
            }
        };

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
                    )
                    .when_some(export_status, |this, status| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .px_2()
                                .py_0p5()
                                .rounded_sm()
                                .bg(theme.muted)
                                .text_xs()
                                .font_medium()
                                .text_color(theme.primary)
                                .child(Icon::new(IconName::Share2))
                                .child(status),
                        )
                    }),
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
                            .child(div().w_2().h_2().rounded_full().bg(dot_color))
                            .child(div().text_xs().text_color(theme.foreground).child(ai_text)),
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
