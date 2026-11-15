use crate::assistant::{AssistantState, register_assistant_providers};
use crate::context_menu::ContextMenu;
use crate::dialog_missing_media::MissingMediaDialog;
use crate::editing::ClipClipboard;
use crate::event_bus::{AppEvent, EventBus};
use crate::explorer::{ExplorerState, save_imported_media};
use crate::explorer_drag::AssetBeingDragged;
use crate::frame_preview::FramePreviewCache;
use crate::preview::{PreviewState, PreviewTarget};
use crate::project_settings::load_project_local_settings;
use crate::property_panel::{PropertyPanel, PropertyPanelState, current_property_panel};
use crate::property_panel_transform::VideoTransformInputs;
use crate::timeline::TimelineRuntimeState;
use crate::ui_components::{HorizontalSplitState, TextInput, VerticalSplitState};
use anyhow::{Result, bail};
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
    pub(super) frame_previews: FramePreviewCache,
    pub(super) property_panel_state: PropertyPanelState,
    pub(super) property_panel: PropertyPanel, // 点击文件或片段后由事件重建，render 只读它。
    pub(super) settings_open: bool,
    pub(super) shortcuts_open: bool,
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
            source_marks: None,
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
            frame_previews: FramePreviewCache::new(),
            property_panel_state,
            property_panel: PropertyPanel::None,
            settings_open: false,
            shortcuts_open: false,
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
        editor.schedule_media_metadata(cx);
        editor.load_assistant_skills(cx);
        editor.load_assistant_history(cx);
        editor.property_panel = current_property_panel(&editor);
        editor
    }

    pub(crate) fn emit_event(&mut self, cx: &mut Context<Self>, event: AppEvent) {
        self.event_bus.update(cx, |_, cx| cx.emit(event));
    }
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
        self.timeline
            .as_ref()
            .is_some_and(|t| !t.undo_stack.is_empty())
    }

    pub(crate) fn can_redo(&self) -> bool {
        self.timeline
            .as_ref()
            .is_some_and(|t| !t.redo_stack.is_empty())
    }

    pub(crate) fn import_media_dialog(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Import Media into NodaVideo".into()),
        });
        cx.spawn(async move |editor, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    log::error!("Import dialog failed: {error:?}");
                    return;
                }
                Err(error) => {
                    log::error!("Import dialog failed: {error:?}");
                    return;
                }
            };
            if paths.is_empty() {
                return;
            }
            if let Err(error) = editor.update(cx, |editor, cx| {
                editor.import_external_paths(&paths, cx);
            }) {
                log::debug!("editor released: {error:?}");
            }
        })
        .detach();
    }

    /// References the original files; nothing is copied into the project.
    pub(crate) fn import_external_paths(&mut self, paths: &[PathBuf], cx: &mut Context<Self>) {
        for path in paths {
            let already_queued = self
                .explorer
                .import_queue
                .iter()
                .any(|(queued, _)| queued == path);
            if !already_queued {
                self.explorer.import_queue.push_back((path.clone(), true));
            }
        }
        self.start_media_import(cx);
        cx.notify();
    }

    /// Probes project media that has no cached metadata yet, on the same bounded queue.
    pub(crate) fn schedule_media_metadata(&mut self, cx: &mut Context<Self>) {
        for entry in &self.explorer.file_tree {
            let is_media = matches!(
                entry.kind,
                crate::explorer_file_entry::FileTreeEntryKind::Video
                    | crate::explorer_file_entry::FileTreeEntryKind::Audio
                    | crate::explorer_file_entry::FileTreeEntryKind::Image
            );
            if !is_media
                || self
                    .explorer
                    .media_metadata
                    .contains_key(&entry.absolute_path)
            {
                continue;
            }
            let already_queued = self
                .explorer
                .import_queue
                .iter()
                .any(|(queued, _)| queued == &entry.absolute_path);
            if !already_queued {
                self.explorer
                    .import_queue
                    .push_back((entry.absolute_path.clone(), false));
            }
        }
        self.start_media_import(cx);
    }

    fn start_media_import(&mut self, cx: &mut Context<Self>) {
        if self.explorer.import_task.is_some() || self.explorer.import_queue.is_empty() {
            return;
        }
        let task = cx.spawn(async move |editor, cx| {
            loop {
                let next = editor.update(cx, |editor, _| editor.explorer.import_queue.pop_front());
                let (path, add_to_library) = match next {
                    Ok(Some(next)) => next,
                    Ok(None) => break,
                    Err(error) => {
                        log::debug!("editor released: {error:?}");
                        return;
                    }
                };
                let probe_path = path.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let canonical = probe_path.canonicalize()?;
                        if !canonical.is_file() {
                            bail!("{} is not a file", probe_path.display());
                        }
                        let mut asset = engine::media_asset::probe_media_asset(&canonical)?;
                        asset.set_id(ulid::Ulid::generate());
                        Ok::<_, anyhow::Error>((canonical, asset))
                    })
                    .await;
                let updated = editor.update(cx, |editor, cx| {
                    match result {
                        Ok((canonical, asset)) => {
                            editor.explorer.media_errors.remove(&canonical);
                            let already_known =
                                editor.explorer.media_metadata.contains_key(&canonical);
                            editor
                                .explorer
                                .media_metadata
                                .insert(canonical.clone(), asset);
                            if add_to_library && !already_known {
                                editor.explorer.imported_paths.insert(canonical);
                                editor.explorer.rebuild_imported_media();
                                let assets: Vec<::timeline::Asset> = editor
                                    .explorer
                                    .imported_paths
                                    .iter()
                                    .filter_map(|path| editor.explorer.media_metadata.get(path))
                                    .cloned()
                                    .collect();
                                if let Err(error) =
                                    save_imported_media(&editor.project_root, &assets)
                                {
                                    log::error!("Could not save media library: {error:?}");
                                    editor
                                        .explorer
                                        .import_errors
                                        .push(format!("Could not save media library: {error:#}"));
                                }
                                editor.schedule_project_waveforms(cx);
                            }
                            editor.schedule_media_thumbnails(cx);
                        }
                        Err(error) => {
                            log::error!("Could not import {}: {error:?}", path.display());
                            editor
                                .explorer
                                .media_errors
                                .insert(path, format!("{error:#}"));
                        }
                    }
                    cx.notify();
                });
                if updated.is_err() {
                    return;
                }
            }
            if let Err(error) = editor.update(cx, |editor, cx| {
                editor.explorer.import_task = None;
                cx.notify();
            }) {
                log::debug!("editor released: {error:?}");
            }
        });
        self.explorer.import_task = Some(task);
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
