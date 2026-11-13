use crate::assistant::{AssistantState, register_assistant_providers};
use crate::context_menu::ContextMenu;
use crate::dialog_missing_media::MissingMediaDialog;
use crate::editing::ClipClipboard;
use crate::event_bus::{AppEvent, EventBus};
use crate::explorer::ExplorerState;
use crate::explorer_drag::AssetBeingDragged;
use crate::preview::{PreviewState, PreviewTarget};
use crate::project_settings::load_project_local_settings;
use crate::property_panel::{PropertyPanel, PropertyPanelState, current_property_panel};
use crate::property_panel_transform::VideoTransformInputs;
use crate::timeline::TimelineRuntimeState;
use crate::ui_components::{HorizontalSplitState, TextInput, VerticalSplitState};
use anyhow::Result;
use gpui::prelude::*;
use gpui::{Entity, FocusHandle, ScrollHandle};
use std::collections::{HashMap, HashSet};
use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) struct Editor {
    // main UI sections
    pub(super) explorer: ExplorerState,
    pub(super) preview: PreviewState,
    pub(crate) timeline: Option<TimelineRuntimeState>,

    // other
    pub(crate) project_root: PathBuf,
    pub(super) waveform_jobs: HashSet<PathBuf>, // 正在生成波形的素材绝对路径。
    pub(super) waveform_cache: HashMap<PathBuf, Arc<::waveform::WaveformData>>, // 键为素材的绝对路径；不同目录的时间线可能用不同相对路径指向同一文件。
    pub(super) property_panel_state: PropertyPanelState,
    pub(super) property_panel: PropertyPanel, // 点击文件或片段后由事件重建，render 只读它。
    pub(super) settings_open: bool,
    pub(super) right_dock: RightDockTab,
    pub(super) assistant: AssistantState,
    pub(crate) workspace: WorkspaceTab,
    pub(crate) show_media_browser: bool,
    pub(crate) show_inspector: bool,

    pub(super) focus_handle: FocusHandle,
    pub(super) clipboard: Option<ClipClipboard>,
    pub(super) context_menu: ContextMenu,
    pub(crate) active_asset_drag: AssetBeingDragged,
    // entities
    pub(super) event_bus: Entity<EventBus>,
    pub(super) upper_split_state: Entity<HorizontalSplitState>,
    pub(super) workspace_split_state: Entity<VerticalSplitState>,
    pub(crate) global_settings_input: Option<Entity<TextInput>>,
    pub(crate) missing_media_dialog: Option<MissingMediaDialog>,
}

impl Editor {
    pub(crate) fn new(
        project_root: PathBuf,                  // 调用方已转成绝对路径
        timeline: Option<TimelineRuntimeState>, // 调用方已加载好
        event_bus: Entity<EventBus>,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let explorer = ExplorerState::new(
            &project_root,
            timeline.as_ref().and_then(|timeline| {
                timeline
                    .path
                    .strip_prefix(&project_root)
                    .ok()
                    .map(Path::to_path_buf)
            }),
            focus_handle.clone(),
            cx,
        );

        let preview = PreviewState {
            target: PreviewTarget::None, // 实体创建后由调用方设置
            fullscreen: false,
            error_scroll: ScrollHandle::new(),
        };

        let property_panel_state = {
            let video_transform_inputs = VideoTransformInputs::new(focus_handle.clone(), cx);
            Self::observe_video_transform_inputs(&video_transform_inputs, cx);

            PropertyPanelState {
                transform_inputs: video_transform_inputs,
                transform_input_clip_id: None,
                text_input_clip_id: None,
            }
        };

        let project_local_settings = load_project_local_settings(&project_root);
        let assistant = AssistantState::new(cx.new(|cx| {
            TextInput::new_field(
                "assistant-input",
                String::new(),
                "Ask the assistant to edit the timeline…",
                focus_handle.clone(),
                cx,
            )
        }));
        register_assistant_providers(assistant.runtime(), cx);

        let mut editor = Self {
            // Entities
            event_bus,
            upper_split_state: cx.new(|_| project_local_settings.upper_space_split_state),
            workspace_split_state: cx.new(|_| project_local_settings.workspace_split_state),
            //
            project_root,
            explorer,
            preview,
            waveform_jobs: HashSet::new(),
            waveform_cache: HashMap::new(),
            property_panel_state,
            property_panel: PropertyPanel::None,
            settings_open: false,
            right_dock: RightDockTab::Inspector,
            assistant,
            workspace: WorkspaceTab::Edit,
            show_media_browser: true,
            show_inspector: true,
            global_settings_input: None,
            missing_media_dialog: None,
            timeline,
            clipboard: None,

            focus_handle,
            context_menu: ContextMenu::None,
            active_asset_drag: AssetBeingDragged::None,
        };
        if let Some(timeline) = editor.timeline.as_mut() {
            timeline.set_playhead(timeline.playhead());
        }
        editor.property_panel = current_property_panel(&editor);
        editor
    }

    pub(crate) fn emit_event(&mut self, cx: &mut Context<Self>, event: AppEvent) {
        self.event_bus.update(cx, |_, cx| cx.emit(event));
    }
}

/// Which panel occupies the right dock of the upper workspace.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RightDockTab {
    Inspector,
    Assistant,
}

/// Active top-level workspace mode in NodaVideo.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum WorkspaceTab {
    #[default]
    Edit,
    Color,
    Audio,
    AiStudio,
}

impl Editor {
    pub(crate) fn can_undo(&self) -> bool {
        self.timeline.as_ref().is_some_and(|t| !t.undo_stack.is_empty())
    }

    pub(crate) fn can_redo(&self) -> bool {
        self.timeline.as_ref().is_some_and(|t| !t.redo_stack.is_empty())
    }

    pub(crate) fn import_media_dialog(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Import Media into NodaVideo".into()),
        });
        let project_root = self.project_root.clone();
        cx.spawn(async move |editor, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if paths.is_empty() {
                    return;
                }
                let count = cx
                    .background_executor()
                    .spawn(async move { Self::copy_files_to_project_sync(&project_root, &paths) })
                    .await;
                if count > 0 {
                    let _ = editor.update(cx, |editor, cx| {
                        let _ = editor.explorer.refresh_file_tree(&editor.project_root);
                        editor.schedule_project_waveforms(cx);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    pub(crate) fn import_external_paths(&mut self, paths: &[PathBuf], cx: &mut Context<Self>) {
        let project_root = self.project_root.clone();
        let paths = paths.to_vec();
        cx.spawn(async move |editor, cx| {
            let count = cx
                .background_executor()
                .spawn(async move { Self::copy_files_to_project_sync(&project_root, &paths) })
                .await;
            if count > 0 {
                let _ = editor.update(cx, |editor, cx| {
                    let _ = editor.explorer.refresh_file_tree(&editor.project_root);
                    editor.schedule_project_waveforms(cx);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn copy_files_to_project_sync(project_root: &Path, paths: &[PathBuf]) -> usize {
        let mut count = 0;
        for path in paths {
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name() else {
                continue;
            };
            if path.starts_with(project_root) {
                count += 1;
                continue;
            }
            let target = project_root.join(file_name);
            let target = if target.exists() {
                if let (Ok(m1), Ok(m2)) = (path.metadata(), target.metadata())
                    && m1.len() == m2.len()
                {
                    count += 1;
                    continue;
                }
                let stem = path.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default();
                let ext = path
                    .extension()
                    .map(|s| format!(".{}", s.to_string_lossy()))
                    .unwrap_or_default();
                let mut candidate = project_root.join(format!("{stem}_imported{ext}"));
                let mut idx = 1;
                while candidate.exists() {
                    candidate = project_root.join(format!("{stem}_imported_{idx}{ext}"));
                    idx += 1;
                }
                candidate
            } else {
                target
            };
            if std::fs::copy(path, &target).is_ok() {
                count += 1;
            }
        }
        count
    }
}

/// 读取项目设置中的 active timeline；文件已被移走或删除时返回 None。
pub(crate) fn load_active_timeline(project_root: &Path) -> Result<Option<TimelineRuntimeState>> {
    let project_settings = load_project_local_settings(project_root);
    let Some(timeline_path) = project_settings.active_timeline else {
        return Ok(None);
    };
    match TimelineRuntimeState::load(project_root.join(&timeline_path)) {
        Ok(timeline) => Ok(Some(timeline)),
        Err(error) => {
            // The saved timeline may have been moved or deleted; open
            // the editor without an active timeline in that case.
            if error
                .downcast_ref::<IoError>()
                .is_some_and(|error| error.kind() == ErrorKind::NotFound)
            {
                return Ok(None);
            }
            // We still error out for other kinds of errors.
            Err(error)
        }
    }
}
