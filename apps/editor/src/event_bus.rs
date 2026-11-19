use crate::assistant_history::AssistantAttachment;
use crate::clip_placement::{
    DEFAULT_IMAGE_DURATION_SECONDS, validate_clip_placement, validate_text_clip_placement,
};
use crate::dialog_missing_media::missing_media_dialog;
use crate::edit_action::{EditAction, edit_timeline};
use crate::editing::{next_track, validate_clips_placements};
use crate::editor::Editor;
use crate::explorer::{MediaCategoryFilter, TemplateGalleryState};
use crate::explorer_drag::AssetBeingDragged;
use crate::explorer_file_entry::select_preview_file;
use crate::export_window::{ExportState, ExportWindow};
use crate::global_settings::GlobalEditorSettings;
use crate::layout::{RULER_HEIGHT, TIMELINE_PADDING, TRACK_HEIGHT, track_index_at_y};
use crate::preview::{PreviewTarget, timeline_preview_target};
use crate::preview_events::PreviewEvent;
use ::timeline::{Asset, MediaKind, TimelineEditingState};

use crate::project_settings::{ProjectLocalSettings, save_project_local_settings};
use crate::property_panel::{PropertyPanel, current_property_panel};
use crate::property_panel_srt_file::srt_file_panel;
use crate::property_panel_text::{
    TextEditTarget, TextPropertyChange, apply_text_change, text_change_edit_action, text_clip_panel,
};
use crate::property_panel_video_clip::video_clip_panel;
use crate::property_panel_video_file::VideoFilePanel;
use crate::srt::{srt_text_clips, text_track_to_srt};
use crate::timeline::{PreviewDropAsset, TimelineFrameIndex, TimelineRuntimeState};
use crate::timeline_clip::{AudioClipProperties, Clip, TextClip, VideoClip, VideoClipProperties};
use crate::timeline_document::{relative_asset_path, resolve_asset_path};
use crate::title::{TextTitle, save_titles};
use crate::track::TrackKind;
use crate::transcription_window;
use crate::transcription_window::{TranscriptionScope, TranscriptionStage, TranscriptionWindow};
use crate::ui_components::{HorizontalSplitState, VerticalSplitState};
use crate::{OpenProject, open_editor_window, quit_after_last_window};
use ::waveform::generate_waveform;
use anyhow::{Context as _, Result, anyhow, bail};
use engine::export::{
    ExportCompletion, ExportControl, ExportOption, export_text_system, export_timeline,
    mix_timeline_audio_mono,
};
use engine::media_asset::probe_media_asset;
use gpui::prelude::*;
use gpui::{
    AsyncApp, Bounds, Entity, EventEmitter, MouseMoveEvent, PathPromptOptions, Pixels, WeakEntity,
};
use nodavideo_agent_runtime::PermissionMode;
use nodavideo_hyperframes_bridge::{
    CompositionAsset, save_composition, template_registry, unique_folder,
};
use platform_fs::move_to_trash;
use player_ui::Seeker;
use player_ui::timeline_player::TimelinePlayer;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use timeline::TimelineSerialization;
use ulid::Ulid;

pub(crate) struct EventBus;

#[derive(Clone, Debug)]
pub(crate) enum AppEvent {
    OpenExportWindow {
        timeline_path: PathBuf,
    },
    ExportTimeline {
        timeline_path: PathBuf,
        document: TimelineSerialization,
        output_path: PathBuf,
        video_bitrate: u64,
        overwrite: bool,
        subtitle_track_ids: Vec<Ulid>,
        export_window: WeakEntity<ExportWindow>,
        control: Arc<ExportControl>,
    },
    Preview(PreviewEvent),
    /// The user chose Relink Media… on an offline clip.
    RelinkMediaRequested,
    /// The user chose Relink media for one missing file in the dialog.
    RelinkMediaFileChosen {
        asset_id: Ulid,
    },
    /// The user dismissed the missing media dialog.
    MissingMediaDialogClosed,
    /// The user chose Move to Trash for a file or folder in the explorer.
    MoveToTrash {
        relative_path: PathBuf, // 相对于项目根目录
    },
    /// The user confirmed moving the open timeline to Trash.
    ActiveTimelineTrashConfirmed,
    GenerationWaveformForTimeline {
        timeline: TimelineEditingState,
        timeline_directory: PathBuf,
    },
    SwitchProject {
        project_path: PathBuf,
    },
    OpenTranscribeWindow {
        audio_source_path: PathBuf, // 音频、视频或时间线文件的绝对路径。
        scope: TranscriptionScope,
        project_root: PathBuf,
    },
    /// Transcribe the file at this absolute path within `scope`.
    Transcribe {
        source_path: PathBuf,
        scope: TranscriptionScope,
        project_root: PathBuf,
        window: WeakEntity<TranscriptionWindow>,
    },
    HorizontalSplitResized(HorizontalSplitState),
    VerticalSplitResized(VerticalSplitState),
    Edit(EditAction),
    /// A text property edit aimed at a timeline clip or a reusable pool title.
    EditTextProperty {
        target: TextEditTarget,
        change: TextPropertyChange,
    },
    DragStarted(AssetBeingDragged),
    DragMove(AssetDragMoveEvent),
    DragDrop,
    OpenTimeline {
        path: PathBuf,
    },
    TimelineSeek {
        frame_index: TimelineFrameIndex,
    },
    RefreshPropertyPanel,
    TextClipSelected {
        clip_id: Ulid,
    },
    VideoClipSelected {
        clip_id: Ulid,
    },
    VideoFileSelected {
        path: PathBuf, // 相对于项目根目录
        size_bytes: Option<u64>,
    },
    SrtFileSelected {
        path: PathBuf, // 相对于项目根目录
    },
    /// The user chose New Text: create a reusable pool title with default properties.
    NewTextTitle,
    /// The user selected a reusable pool title in the Media Pool.
    SelectTitle {
        title_id: Ulid,
    },
    /// The user chose Add to Timeline for a pool title: place it at the playhead.
    AddTitleToTimeline {
        title_id: Ulid,
    },
    /// The user sent a message from the AI assistant dock.
    AssistantMessageSubmitted {
        message: String,
        attachments: Vec<AssistantAttachment>,
    },
    /// The user chose a provider from the AI assistant dock's model selector.
    AssistantProviderSelected {
        index: usize,
    },
    /// The user chose a model for the active provider; None restores the provider default.
    AssistantModelSelected {
        model: Option<String>,
    },
    /// The user chose a reasoning-effort variant; None restores the model default.
    AssistantEffortSelected {
        effort: Option<String>,
    },
    /// The user chose how much freedom the agent has to act without approval.
    AssistantPermissionModeSelected {
        mode: PermissionMode,
    },
    /// The user asked to retry loading the active provider's model catalog.
    AssistantModelsRefresh,
    /// The user approved or rejected the pending assistant action.
    AssistantApprovalDecided {
        approved: bool,
    },
    /// The user stopped the running assistant turn from the composer's Stop button or Esc.
    AssistantStopped,
    /// The user chose a production skill from the composer's skill picker.
    AssistantSkillSelected {
        id: String,
    },
    /// The user started a fresh assistant session from the window's New chat control.
    AssistantNewChat,
    /// The user chose a saved assistant session from the sessions rail to replay.
    AssistantSessionSelected {
        id: Ulid,
    },
    /// The user returned from a saved transcript to the live conversation.
    AssistantLiveSessionResumed,
    WaveformsGeneration {
        paths: Vec<PathBuf>, // 素材的绝对路径
    },
    /// The user chose New Motion Graphic: open the template gallery.
    OpenTemplateGallery,
    /// The user dismissed the template gallery without creating.
    CloseTemplateGallery,
    /// The user highlighted a template in the gallery.
    GalleryTemplateSelected {
        index: usize,
    },
    /// The user chose Create in the gallery: instantiate the selected template.
    CreateComposition {
        template_id: String,
    },
    /// The user selected a reusable composition in the Media Pool.
    SelectComposition {
        composition_id: Ulid,
    },
}

pub(crate) async fn handle_event(
    cx: &mut AsyncApp,
    project: Entity<OpenProject>,
    event_bus: Entity<EventBus>,
    event: AppEvent,
) {
    log::debug!("handle_event: {event:?}");
    match event {
        AppEvent::ExportTimeline {
            timeline_path,
            document,
            output_path,
            video_bitrate,
            overwrite,
            subtitle_track_ids,
            export_window,
            control,
        } => {
            // macOS 平台必须在主线程创建；线程安全的文字系统交给导出线程。
            let platform_text_system = cx.update(|_| export_text_system());
            let result = cx
                .background_executor()
                .spawn(async move {
                    let directory = timeline_path
                        .parent()
                        .context("Timeline path has no parent directory")?;
                    let completion = export_timeline(
                        &document,
                        &output_path,
                        &ExportOption {
                            project_root: directory.to_path_buf(),
                            video_bitrate,
                            overwrite,
                        },
                        platform_text_system,
                        &control,
                    )?;
                    match completion {
                        ExportCompletion::Stopped => {
                            return Ok(completion); // 停止的导出不留下字幕文件
                        }
                        ExportCompletion::Completed => {
                            let output_stem = output_path
                                .file_stem()
                                .context("Export path has no file name")?
                                .to_string_lossy();
                            let tracks = &document.editing_state.tracks;
                            for track in tracks
                                .iter()
                                .filter(|track| subtitle_track_ids.contains(&track.id))
                            {
                                let srt = text_track_to_srt(&document.editing_state, track.id)?;
                                let srt_path = output_path
                                    .with_file_name(format!("{output_stem}_{}.srt", track.name));
                                srt.write_to(&srt_path, overwrite)?;
                            }
                        }
                    }
                    anyhow::Ok(completion)
                })
                .await;
            let state = match result {
                Ok(ExportCompletion::Completed) => ExportState::Complete,
                Ok(ExportCompletion::Stopped) => ExportState::Stopped,
                Err(error) => {
                    log::error!("Could not export timeline: {error:?}");
                    ExportState::Failed
                }
            };
            let _ = export_window.update(cx, |view, cx| {
                view.state = state;
                cx.notify();
            });
        }
        AppEvent::OpenTranscribeWindow {
            audio_source_path: source_path,
            scope,
            project_root,
        } => {
            let audio_duration = match scope {
                TranscriptionScope::Clip(range) => {
                    range.end.saturating_sub(range.start).as_secs_f64()
                }
                TranscriptionScope::File => match probe_media_asset(&source_path) {
                    Ok(asset) => asset.duration().unwrap_or(0.0),
                    Err(error) => {
                        log::error!("Could not open transcription window: {error:?}");
                        return;
                    }
                },
                TranscriptionScope::Timeline => match TimelineSerialization::load(&source_path) {
                    Ok(document) => document
                        .editing_state
                        .settings
                        .frame_rate
                        .duration(TimelineFrameIndex::from(document.frame_count()))
                        .as_secs_f64(),
                    Err(error) => {
                        log::error!("Could not open transcription window: {error:?}");
                        return;
                    }
                },
            };
            if let Err(error) = cx.update(|cx| {
                transcription_window::open(
                    source_path,
                    scope,
                    audio_duration,
                    project_root,
                    event_bus,
                    cx,
                )
            }) {
                log::error!("Could not open transcription window: {error:?}");
            }
        }
        AppEvent::Transcribe {
            source_path,
            scope,
            project_root,
            window,
        } => {
            let Some(view) = window.upgrade() else {
                return;
            };
            if view.read_with(cx, |view, _| view.stage.is_running()) {
                return;
            }
            view.update(cx, |view, cx| {
                view.stage = TranscriptionStage::Preparing;
                view.started = std::time::Instant::now();
                cx.notify();
                view.task = Some(cx.spawn(async move |view, cx| {
                    let refresh = view.update(cx, |_, cx| {
                        cx.spawn(async move |view, cx| {
                            loop {
                                cx.background_executor()
                                    .timer(std::time::Duration::from_millis(250))
                                    .await;
                                if view.update(cx, |_, cx| cx.notify()).is_err() {
                                    break;
                                }
                            }
                        })
                    });
                    let result: Result<PathBuf> = async {
                        // 先确定输出路径，避免解码和付费请求之后才失败。
                        let stem = source_path
                            .file_stem()
                            .context("Transcription source has no filename")?
                            .to_string_lossy();
                        let file_name = match scope {
                            TranscriptionScope::Clip(range) => {
                                format!("{stem}_{}.srt", range.clip_id)
                            }
                            TranscriptionScope::File | TranscriptionScope::Timeline => {
                                format!("{stem}.srt")
                            }
                        };
                        let srt_path = project_root.join(file_name);
                        let (wav, api_key) = cx
                            .update(|cx| {
                                gpui_tokio::Tokio::spawn(cx, async move {
                                    let settings = GlobalEditorSettings::load()?;
                                    log::info!(
                                        "Transcribing audio with MiniMax: {}",
                                        source_path.display()
                                    );
                                    let wav = match scope {
                                        TranscriptionScope::Clip(range) => {
                                            transcribe::audio::extract_audio_range_as_wav(
                                                &source_path,
                                                range.start,
                                                range.end,
                                            )?
                                        }
                                        TranscriptionScope::File => {
                                            transcribe::audio::extract_audio_as_wav(&source_path)?
                                        }
                                        TranscriptionScope::Timeline => {
                                            let document =
                                                TimelineSerialization::load(&source_path)?;
                                            let timeline_directory = source_path
                                                .parent()
                                                .context("Timeline path has no parent directory")?;
                                            let pcm = mix_timeline_audio_mono(
                                                &document.editing_state,
                                                timeline_directory,
                                                16_000,
                                            )?;
                                            let mut wav = vec![0_u8; 44];
                                            for sample in pcm {
                                                wav.extend_from_slice(&sample.to_le_bytes());
                                            }
                                            transcribe::audio::write_wav_header(wav)?
                                        }
                                    };
                                    Ok::<_, anyhow::Error>((wav, settings.minimax_api_key))
                                })
                            })
                            .await
                            .context("Audio preparation task failed")??;
                        view.update(cx, |view, cx| {
                            view.stage = TranscriptionStage::Transcribing;
                            cx.notify();
                        })?;
                        let srt = cx
                            .update(|cx| {
                                gpui_tokio::Tokio::spawn(cx, async move {
                                    let srt = transcribe::transcribe_wav(
                                        wav,
                                        &api_key,
                                        &transcribe::Options::default(),
                                    )
                                    .await?;
                                    transcribe::subtitles::merge_srt_sections(&srt)
                                })
                            })
                            .await
                            .context("Transcription task failed")??;
                        view.update(cx, |view, cx| {
                            view.stage = TranscriptionStage::Saving;
                            cx.notify();
                        })?;
                        cx.background_executor()
                            .spawn(async move {
                                srt.write_to(&srt_path, true)?;
                                Ok(srt_path)
                            })
                            .await
                    }
                    .await;
                    drop(refresh);
                    let stage = match result {
                        Ok(path) => TranscriptionStage::Complete(path),
                        Err(error) => {
                            log::error!("SRT generation failed: {error:?}");
                            TranscriptionStage::Failed(format!("{error:#}"))
                        }
                    };
                    let _ = view.update(cx, |view, cx| {
                        view.stage = stage;
                        cx.notify();
                    });
                }));
            });
        }

        AppEvent::SwitchProject { project_path } => {
            let root = match std::fs::canonicalize(&project_path) {
                Ok(root) => root,
                Err(error) => panic!("could not open {}: {error}", project_path.display()),
            };
            let switched = project.update(cx, |project, cx| {
                let ready = match project.window.update(cx, |editor, _, cx| {
                    match editor.prepare_project_switch() {
                        Ok(()) => true,
                        Err(error) => {
                            log::error!(
                                "Could not save timeline before switching projects: {error:?}"
                            );
                            cx.notify();
                            false
                        }
                    }
                }) {
                    Ok(ready) => ready,
                    Err(error) => panic!("could not reach the editor: {error}"),
                };
                if !ready {
                    return false;
                }
                drop(project.close_subscription.take());
                if let Err(error) = project
                    .window
                    .update(cx, |_, window, _| window.remove_window())
                {
                    panic!("could not close editor: {error}");
                }
                project.window = match open_editor_window(root.clone(), event_bus, cx) {
                    Ok(window) => window,
                    Err(error) => panic!("could not open editor window: {error:?}"),
                };
                project.close_subscription = Some(cx.on_window_closed(quit_after_last_window));
                if let Err(error) = project
                    .window
                    .update(cx, |editor, _, cx| editor.schedule_project_waveforms(cx))
                {
                    panic!("could not reach the editor: {error}");
                }
                true
            });
            if !switched {
                return;
            }
            let mut settings = match GlobalEditorSettings::load() {
                Ok(settings) => settings,
                Err(error) => {
                    log::error!("Could not load settings: {error:?}");
                    return;
                }
            };
            settings.project_root = root;
            if let Err(error) = settings.save() {
                panic!("could not save project settings: {error}");
            }
        }
        event => {
            let window = project.read_with(cx, |project, _| project.window);
            let editor = match window.entity(cx) {
                Ok(editor) => editor,
                Err(error) => {
                    log::error!("Could not find the editor window: {error:?}");
                    return;
                }
            };
            if let Err(error) = handle_app_event(editor, event, cx).await {
                log::error!("Could not handle the event: {error:?}");
            }
        }
    }
}

async fn handle_app_event(
    editor: Entity<Editor>,
    event: AppEvent,
    cx: &mut AsyncApp,
) -> Result<()> {
    match event {
        AppEvent::OpenExportWindow { timeline_path } => {
            editor.update(cx, |editor, cx| {
                editor.open_export_window(timeline_path, cx)
            })?;
        }
        AppEvent::Preview(preview_event) => match preview_event {
            PreviewEvent::SelectFile(path) => {
                select_preview_file(editor, path, cx).await?;
            }
            PreviewEvent::TogglePlayback => {
                editor.update(cx, |editor, cx| -> Result<()> {
                    editor.toggle_preview_playback(cx)?;
                    cx.notify();
                    Ok(())
                })?;
            }
            PreviewEvent::MarkSourceIn => {
                editor.update(cx, |editor, cx| editor.mark_source_in(cx))?;
            }
            PreviewEvent::MarkSourceOut => {
                editor.update(cx, |editor, cx| editor.mark_source_out(cx))?;
            }
            PreviewEvent::InsertSourceToTimeline => {
                editor.update(cx, |editor, cx| editor.insert_source_to_timeline(cx))?;
            }
        },
        AppEvent::RelinkMediaRequested => {
            editor.update(cx, |editor, cx| -> Result<()> {
                let timeline = editor
                    .timeline
                    .as_ref()
                    .context("Relink Media… needs an open timeline")?;
                editor.missing_media_dialog = Some(missing_media_dialog(timeline));
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::RelinkMediaFileChosen { asset_id } => {
            let selection = cx.update(|cx| {
                cx.prompt_for_paths(PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: false,
                    prompt: Some("Relink media".into()),
                })
            });
            #[rustfmt::skip]
            let media_path = match selection.await {
                Ok(Ok(Some(paths))) => {
                    match paths.into_iter().next() {
                        Some(path) => {
                            path
                        }
                        None => {
                            bail!("The file picker returned no path")
                        }
                    }
                }
                Ok(Ok(None)) => {
                    return Ok(()); // 用户取消
                }
                Ok(Err(error)) => {
                    return Err(error.context("choosing a file to relink"));
                }
                Err(error) => {
                    return Err(anyhow!("file picker closed unexpectedly: {error}"));
                }
            };
            let path = editor.update(cx, |editor, _| -> Result<PathBuf> {
                let timeline = editor
                    .timeline
                    .as_ref()
                    .context("Relink media needs an open timeline")?;
                let timeline_directory = timeline
                    .path
                    .parent()
                    .context("Timeline path has no parent directory")?;
                Ok(relative_asset_path(timeline_directory, &media_path))
            })?;
            handle_app_event_edit(
                &editor,
                EditAction::UpdateAssetPaths {
                    paths: vec![(asset_id, path)],
                },
                cx,
            )?;
            let paths = editor.update(cx, |editor, _| -> Result<Vec<PathBuf>> {
                let timeline = editor
                    .timeline
                    .as_ref()
                    .context("Relink media needs an open timeline")?;
                let timeline_directory = timeline
                    .path
                    .parent()
                    .context("Timeline path has no parent directory")?;
                Ok(timeline_waveform_paths(
                    &timeline.editing_state,
                    timeline_directory,
                ))
            })?;
            generate_waveforms(&editor, paths, cx).await;
        }
        AppEvent::GenerationWaveformForTimeline {
            timeline,
            timeline_directory,
        } => {
            let paths = timeline_waveform_paths(&timeline, &timeline_directory);
            generate_waveforms(&editor, paths, cx).await;
        }
        AppEvent::WaveformsGeneration { paths } => {
            generate_waveforms(&editor, paths, cx).await;
        }
        AppEvent::MissingMediaDialogClosed => {
            editor.update(cx, |editor, cx| {
                editor.missing_media_dialog = None;
                cx.notify();
            });
        }
        AppEvent::MoveToTrash { relative_path } => {
            editor.update(cx, |editor, cx| -> Result<()> {
                // The project root is the workspace itself, not an entry within it.
                if relative_path.as_os_str().is_empty() {
                    bail!("The project folder cannot be moved to Trash here.");
                }
                let path = editor.project_root.join(&relative_path);
                if editor
                    .timeline
                    .as_ref()
                    .is_some_and(|timeline| timeline.path.starts_with(&path))
                {
                    bail!("The active timeline cannot be moved to Trash.");
                }
                let display_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| relative_path.display().to_string());
                if let Err(error) = move_to_trash(&path) {
                    return Err(anyhow!("Could not move {display_name} to Trash: {error}"));
                }
                editor
                    .explorer
                    .expanded_directories
                    .retain(|directory| !directory.starts_with(&relative_path));
                editor.explorer.selected_file = None;
                editor.property_panel = current_property_panel(editor);
                editor.explorer.search_query = None;
                editor.explorer.search_results.clear();
                editor.explorer.search_pending = false;
                editor.explorer.refresh_file_tree(&editor.project_root)?;
                editor.save_explorer_expansion()?;
                editor.schedule_explorer_search(cx);
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::ActiveTimelineTrashConfirmed => {
            editor.update(cx, |editor, cx| -> Result<()> {
                editor.trash_active_timeline(cx)?;
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::SwitchProject { .. }
        | AppEvent::OpenTranscribeWindow { .. }
        | AppEvent::Transcribe { .. }
        | AppEvent::ExportTimeline { .. } => {
            editor.update(cx, |_, cx| cx.notify());
        }
        AppEvent::AssistantMessageSubmitted {
            message,
            attachments,
        } => {
            editor.update(cx, |editor, cx| {
                editor.send_assistant_message(message, attachments, cx)
            });
        }
        AppEvent::AssistantProviderSelected { index } => {
            editor.update(cx, |editor, cx| editor.select_assistant_provider(index, cx));
        }
        AppEvent::AssistantModelSelected { model } => {
            editor.update(cx, |editor, cx| editor.select_assistant_model(model, cx));
        }
        AppEvent::AssistantEffortSelected { effort } => {
            editor.update(cx, |editor, cx| editor.select_assistant_effort(effort, cx));
        }
        AppEvent::AssistantPermissionModeSelected { mode } => {
            editor.update(cx, |editor, cx| {
                editor.select_assistant_permission_mode(mode, cx)
            });
        }
        AppEvent::AssistantModelsRefresh => {
            editor.update(cx, |editor, cx| editor.refresh_assistant_models(cx));
        }
        AppEvent::AssistantApprovalDecided { approved } => {
            editor.update(cx, |editor, cx| {
                editor.submit_assistant_approval(approved, cx)
            });
        }
        AppEvent::AssistantStopped => {
            editor.update(cx, |editor, cx| editor.stop_assistant(cx));
        }
        AppEvent::AssistantSkillSelected { id } => {
            editor.update(cx, |editor, cx| editor.select_assistant_skill(id, cx));
        }
        AppEvent::AssistantNewChat => {
            editor.update(cx, |editor, cx| editor.new_assistant_chat(cx));
        }
        AppEvent::AssistantSessionSelected { id } => {
            editor.update(cx, |editor, cx| editor.select_assistant_session(id, cx));
        }
        AppEvent::AssistantLiveSessionResumed => {
            editor.update(cx, |editor, cx| editor.resume_live_assistant(cx));
        }
        AppEvent::HorizontalSplitResized(state) => {
            editor.update(cx, |editor, cx| -> Result<()> {
                save_project_local_settings(
                    &editor.project_root,
                    &ProjectLocalSettings {
                        active_timeline: editor.timeline.as_ref().and_then(|timeline| {
                            timeline
                                .path
                                .strip_prefix(&editor.project_root)
                                .ok()
                                .map(Path::to_path_buf)
                        }),
                        upper_space_split_state: state,
                        workspace_split_state: editor.workspace_split_state.read(cx).clone(),
                    },
                )?;
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::VerticalSplitResized(state) => {
            editor.update(cx, |editor, cx| -> Result<()> {
                save_project_local_settings(
                    &editor.project_root,
                    &ProjectLocalSettings {
                        active_timeline: editor.timeline.as_ref().and_then(|timeline| {
                            timeline
                                .path
                                .strip_prefix(&editor.project_root)
                                .ok()
                                .map(Path::to_path_buf)
                        }),
                        upper_space_split_state: editor.upper_split_state.read(cx).clone(),
                        workspace_split_state: state,
                    },
                )?;
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::Edit(edit_action) => {
            handle_app_event_edit(&editor, edit_action, cx)?;
        }
        AppEvent::EditTextProperty { target, change } => match target {
            TextEditTarget::Clip(clip_id) => {
                handle_app_event_edit(&editor, text_change_edit_action(clip_id, change), cx)?;
            }
            TextEditTarget::Title(title_id) => {
                editor.update(cx, |editor, cx| -> Result<()> {
                    // 先在副本上改动并落盘，全部成功后再替换标题库，失败时保持原状。
                    let mut new_titles = editor.explorer.titles.clone();
                    let Some(title) = new_titles.iter_mut().find(|title| title.id == title_id)
                    else {
                        bail!("Title {title_id} no longer exists");
                    };
                    apply_text_change(&mut title.properties, &change)?;
                    save_titles(&editor.project_root, &new_titles)?;
                    editor.explorer.titles = new_titles;
                    editor.property_panel = current_property_panel(editor);
                    cx.notify();
                    Ok(())
                })?;
            }
        },
        AppEvent::DragStarted(asset) => {
            editor.update(cx, |editor, cx| {
                editor.active_asset_drag = asset;
                cx.notify();
            });
        }
        AppEvent::RefreshPropertyPanel => {
            editor.update(cx, |editor, cx| {
                editor.property_panel = current_property_panel(editor);
                cx.notify();
            });
        }
        AppEvent::TextClipSelected { clip_id } => {
            editor.update(cx, |editor, cx| -> Result<()> {
                let timeline = editor.timeline.as_ref().context("No timeline is open")?;
                let Some(Clip::Text(clip)) = timeline.editing_state.clip(clip_id) else {
                    bail!("Text clip {clip_id} no longer exists");
                };
                editor.property_panel = PropertyPanel::TextClip(text_clip_panel(
                    &timeline.editing_state,
                    clip,
                    editor.event_bus.clone(),
                    editor.focus_handle.clone(),
                ));
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::VideoClipSelected { clip_id } => {
            editor.update(cx, |editor, cx| -> Result<()> {
                let timeline = editor.timeline.as_ref().context("No timeline is open")?;
                let Some(Clip::Video(clip)) = timeline.editing_state.clip(clip_id) else {
                    bail!("Video clip {clip_id} no longer exists");
                };
                editor.property_panel = PropertyPanel::VideoClip(video_clip_panel(
                    &timeline.editing_state,
                    clip,
                    editor.event_bus.clone(),
                    editor.focus_handle.clone(),
                ));
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::VideoFileSelected { path, size_bytes } => {
            editor.update(cx, |editor, cx| {
                editor.property_panel = PropertyPanel::VideoFile(VideoFilePanel {
                    path: editor.project_root.join(path),
                    size_bytes,
                });
                cx.notify();
            });
        }
        AppEvent::SrtFileSelected { path } => {
            editor.update(cx, |editor, cx| {
                editor.property_panel =
                    PropertyPanel::SrtFile(srt_file_panel(editor.project_root.join(path)));
                cx.notify();
            });
        }
        AppEvent::NewTextTitle => {
            editor.update(cx, |editor, cx| -> Result<()> {
                // 先在副本上追加并落盘，全部成功后再替换标题库，失败时保持原状。
                let new_title = TextTitle::new_default();
                let title_id = new_title.id;
                let mut new_titles = editor.explorer.titles.clone();
                new_titles.push(new_title);
                save_titles(&editor.project_root, &new_titles)?;
                editor.explorer.titles = new_titles;
                editor.explorer.category = MediaCategoryFilter::Text;
                select_title(editor, title_id);
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::SelectTitle { title_id } => {
            editor.update(cx, |editor, cx| -> Result<()> {
                if !editor
                    .explorer
                    .titles
                    .iter()
                    .any(|title| title.id == title_id)
                {
                    bail!("Title {title_id} no longer exists");
                }
                select_title(editor, title_id);
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::AddTitleToTimeline { title_id } => {
            editor.update(cx, |editor, cx| -> Result<()> {
                let Some(timeline) = editor.timeline.as_ref() else {
                    bail!("Open a timeline before adding a title");
                };
                let start = timeline.playhead();
                editor.place_title(title_id, None, start, cx)?;
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::OpenTemplateGallery => {
            editor.update(cx, |editor, cx| {
                let mut templates = template_registry()
                    .list()
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                templates.sort_by(|a, b| a.name.cmp(&b.name));
                editor.explorer.template_gallery = Some(TemplateGalleryState {
                    selected: 0,
                    templates,
                });
                editor.schedule_composition_thumbnails(cx);
                cx.notify();
            });
        }
        AppEvent::CloseTemplateGallery => {
            editor.update(cx, |editor, cx| {
                editor.explorer.template_gallery = None;
                cx.notify();
            });
        }
        AppEvent::GalleryTemplateSelected { index } => {
            editor.update(cx, |editor, cx| {
                if let Some(gallery) = editor.explorer.template_gallery.as_mut() {
                    gallery.selected = index;
                }
                cx.notify();
            });
        }
        AppEvent::CreateComposition { template_id } => {
            editor.update(cx, |editor, cx| -> Result<()> {
                let registry = template_registry();
                let Some(template) = registry.get(&template_id) else {
                    bail!("Motion graphics template '{template_id}' not found");
                };
                // 先在副本上追加并落盘，全部成功后再替换库，失败时保持原状。
                let folder = unique_folder(&editor.project_root, &template.name);
                let asset = CompositionAsset::new_from_template(template, &template.name, folder);
                let composition_id = asset.id;
                let mut new_compositions = editor.explorer.compositions.clone();
                new_compositions.push(asset);
                save_composition(
                    &editor.project_root,
                    new_compositions.last().expect("just pushed"),
                )?;
                editor.explorer.compositions = new_compositions;
                editor.explorer.category = MediaCategoryFilter::Motion;
                editor.explorer.template_gallery = None;
                select_composition(editor, composition_id);
                editor.schedule_composition_thumbnails(cx);
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::SelectComposition { composition_id } => {
            editor.update(cx, |editor, cx| -> Result<()> {
                if !editor
                    .explorer
                    .compositions
                    .iter()
                    .any(|asset| asset.id == composition_id)
                {
                    bail!("Composition {composition_id} no longer exists");
                }
                select_composition(editor, composition_id);
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::DragMove(event) => {
            editor.update(cx, |editor, cx| {
                let Some(timeline) = editor.timeline.as_mut() else {
                    return;
                };
                let (new_preview_drop_asset, new_snap_guide) =
                    (|timeline: &TimelineRuntimeState| {
                        let pointer = event.event.position;
                        if !event.bounds.contains(&pointer) {
                            return (None, None);
                        }
                        let track_index = track_index_at_y(
                            f32::from(pointer.y),
                            f32::from(event.bounds.top()) + RULER_HEIGHT,
                            TRACK_HEIGHT,
                        );
                        let Ok(target_track_index) = usize::try_from(track_index) else {
                            return (None, None);
                        };
                        let Some(track) = timeline.editing_state.tracks.get(target_track_index)
                        else {
                            return (None, None);
                        };
                        let local_x = f32::from(pointer.x) - f32::from(event.bounds.left());
                        let raw_start = timeline.editing_state.nearest_time(
                            ((local_x - TIMELINE_PADDING) / timeline.pixels_per_second).max(0.0)
                                as f64,
                        );
                        let (start_time, snap_guide) = match &editor.active_asset_drag {
                            AssetBeingDragged::Media(asset) => {
                                let duration = timeline
                                    .editing_state
                                    .nearest_time(
                                        asset
                                            .metadata
                                            .duration()
                                            .unwrap_or(DEFAULT_IMAGE_DURATION_SECONDS),
                                    )
                                    .max(TimelineFrameIndex::ONE_FRAME);
                                timeline.snap_clip_start_ignoring(
                                    raw_start,
                                    duration,
                                    &HashSet::new(),
                                )
                            }
                            AssetBeingDragged::Srt(_) => (raw_start, None),
                            AssetBeingDragged::Title(title) => {
                                let clip_length = timeline
                                    .editing_state
                                    .settings
                                    .frame_rate
                                    .frames_from_duration_nearest(title.duration)
                                    .max(TimelineFrameIndex::ONE_FRAME);
                                timeline.snap_clip_start_ignoring(
                                    raw_start,
                                    clip_length,
                                    &HashSet::new(),
                                )
                            }
                            AssetBeingDragged::None => return (None, None),
                        };
                        let preview = PreviewDropAsset {
                            track_id: track.id,
                            start_time,
                            asset: editor.active_asset_drag.clone(),
                        };
                        let valid_snap_guide =
                            if preview.validate_placement(&timeline.editing_state).is_ok() {
                                snap_guide
                            } else {
                                None
                            };
                        (Some(preview), valid_snap_guide)
                    })(timeline);

                timeline.preview_drop_asset = new_preview_drop_asset;
                timeline.interaction.snap_guide = new_snap_guide;
                cx.notify();
            });
        }
        AppEvent::DragDrop => {
            editor.update(cx, |editor, cx| -> Result<()> {
                editor.active_asset_drag = AssetBeingDragged::None;
                cx.notify();
                let Some(timeline) = editor.timeline.as_mut() else {
                    return Ok(());
                };
                timeline.interaction.snap_guide = None;
                let Some(preview) = timeline.preview_drop_asset.take() else {
                    return Ok(());
                };

                if preview.validate_placement(&timeline.editing_state).is_err() {
                    return Ok(());
                }

                match preview.asset {
                    AssetBeingDragged::Srt(srt) => {
                        let mut text_clips =
                            srt_text_clips(&srt.srt, timeline.editing_state.settings.frame_rate);
                        for clip in &mut text_clips {
                            clip.track_id = preview.track_id;
                            clip.timeline_start += preview.start_time;
                        }
                        let clips = text_clips.into_iter().map(Clip::Text).collect::<Vec<_>>();
                        validate_clips_placements(&timeline.editing_state, &clips)?;

                        let selected_clip_ids = clips.iter().map(Clip::id).collect::<HashSet<_>>();
                        let selected_clip_id = clips.first().map(Clip::id);
                        let should_save = edit_timeline(
                            timeline,
                            EditAction::AddClips {
                                clips,
                                assets: Vec::new(),
                            },
                        )?;
                        timeline.interaction.selected_clip_ids = selected_clip_ids;
                        timeline.interaction.selected_clip_id = selected_clip_id;
                        if should_save {
                            timeline.save()?;
                            match editor.preview.target {
                                PreviewTarget::Timeline { .. } => {
                                    editor.preview.target = timeline_preview_target(
                                        timeline,
                                        &editor.project_root,
                                        cx,
                                    )?;
                                }
                                _ => {
                                    // do nothing
                                }
                            }
                        }
                    }
                    AssetBeingDragged::Media(asset) => {
                        editor.place_explorer_asset(
                            asset.absolute_path.clone(),
                            Some(preview.track_id),
                            preview.start_time,
                            asset.metadata,
                            None,
                            cx,
                        )?;
                    }
                    AssetBeingDragged::Title(title) => {
                        editor.place_title(
                            title.title_id,
                            Some(preview.track_id),
                            preview.start_time,
                            cx,
                        )?;
                    }
                    AssetBeingDragged::None => return Ok(()),
                }
                cx.notify();
                Ok(())
            })?;
        }
        AppEvent::TimelineSeek { frame_index } => {
            editor.update(cx, |editor, cx| -> Result<()> {
                let Some(timeline) = editor.timeline.as_ref() else {
                    return Err(anyhow!("TimelineSeek arrived while no timeline is open"));
                };
                if timeline.has_offline_clips() {
                    editor.preview.target = PreviewTarget::None;
                    cx.notify();
                    return Ok(());
                }
                match &editor.preview.target {
                    PreviewTarget::Timeline { player, .. } => {
                        player.update(cx, |player, cx| {
                            let position =
                                player.backend().timeline().position_at_frame(frame_index);
                            let result = player.seek(position);
                            cx.notify();
                            result
                        })?;
                    }
                    _ => {
                        let relative_path = match timeline.path.strip_prefix(&editor.project_root) {
                            Ok(relative_path) => relative_path.to_path_buf(),
                            Err(_) => timeline.path.clone(),
                        };
                        let timeline_directory = timeline
                            .path
                            .parent()
                            .context("Timeline path has no parent directory")?; // 素材路径相对于时间线文件所在目录。
                        let mut timeline_player = TimelinePlayer::from_editing_state(
                            timeline.editing_state.clone(),
                            timeline_directory,
                        )?;
                        timeline_player.title = timeline
                            .path
                            .file_name()
                            .context("Timeline path has no file name")?
                            .to_string_lossy()
                            .into_owned();
                        let position = timeline_player
                            .backend()
                            .timeline()
                            .position_at_frame(frame_index);
                        timeline_player.backend_mut().seek(position)?;
                        let player = cx.new(move |_| timeline_player);
                        editor.preview.target = PreviewTarget::Timeline {
                            _task: player.update(cx, |player, cx| player.start(cx)),
                            _subscription: cx.observe(&player, |_, _, cx| cx.notify()),
                            path: relative_path,
                            player,
                        };
                    }
                }
                Ok(())
            })?;
        }
        AppEvent::OpenTimeline { path } => {
            editor.update(cx, |editor, cx| -> Result<()> {
                if let Err(error) = editor.open_timeline(path.clone(), cx) {
                    log::error!("Could not open timeline: {error:?}");
                    editor.preview.target = PreviewTarget::Error {
                        path,
                        error: format!("{error:?}"),
                    };
                }
                cx.notify();
                Ok(())
            })?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub(crate) struct AssetDragMoveEvent {
    pub(crate) event: MouseMoveEvent,
    pub(crate) bounds: Bounds<Pixels>,
}

impl EventEmitter<AppEvent> for EventBus {}

// Private and internal APIs
impl Editor {
    pub(crate) fn place_explorer_asset(
        &mut self,
        media_path: PathBuf,
        track_id: Option<Ulid>, // None 表示没有同类轨道，连同片段一起原子新建。
        start: TimelineFrameIndex,
        mut asset: Asset,
        source_range: Option<(TimelineFrameIndex, TimelineFrameIndex)>, // None 表示使用素材完整时长。
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let Some(timeline) = self.timeline.as_ref() else {
            return Ok(());
        };
        let full_duration = timeline
            .editing_state
            .nearest_time(asset.duration().unwrap_or(DEFAULT_IMAGE_DURATION_SECONDS))
            .max(TimelineFrameIndex::ONE_FRAME);
        let (source_in, source_out) = match source_range {
            Some((in_frame, out_frame)) => {
                let out_frame = out_frame.min(full_duration);
                if out_frame <= in_frame {
                    bail!("The source out point must be after the in point");
                }
                (in_frame, out_frame)
            }
            None => (TimelineFrameIndex::ZERO, full_duration),
        };
        let duration = source_out - source_in;
        let (track_id, track_kind, new_track) = match track_id {
            Some(track_id) => {
                validate_clip_placement(
                    &timeline.editing_state,
                    track_id,
                    asset.kind(),
                    duration,
                    start,
                    &HashSet::new(),
                )?;
                let track_kind = timeline
                    .editing_state
                    .tracks
                    .iter()
                    .find(|track| track.id == track_id)
                    .map(|track| track.kind);
                (track_id, track_kind, None)
            }
            None => {
                let kind = match asset.kind() {
                    MediaKind::Audio => TrackKind::Audio,
                    MediaKind::Video | MediaKind::Image => TrackKind::Video,
                };
                let track = next_track(&timeline.editing_state.tracks, kind);
                (track.id, Some(kind), Some(track))
            }
        };

        let Some(timeline) = self.timeline.as_mut() else {
            return Ok(());
        };
        let timeline_directory = timeline
            .path
            .parent()
            .context("Timeline path has no parent directory")?;
        let asset_path = relative_asset_path(timeline_directory, &media_path); // 素材路径相对于时间线文件所在目录。
        let (asset_id, assets) = if let Some(asset_id) = timeline
            .editing_state
            .assets
            .iter()
            .find(|existing| existing.path() == &asset_path)
            .map(|existing| existing.id())
        {
            (asset_id, Vec::new())
        } else {
            asset.set_id(Ulid::generate());
            asset.set_path(asset_path);
            let asset_id = asset.id();
            (asset_id, vec![asset])
        };
        let clip_id = Ulid::generate();
        let media_clip = match track_kind {
            Some(TrackKind::Video) => Clip::Video(VideoClip::new(
                clip_id,
                track_id,
                asset_id,
                start,
                source_in,
                source_out,
                VideoClipProperties::default(),
                AudioClipProperties::default(),
            )),
            Some(TrackKind::Audio) => Clip::Audio(timeline::AudioClip::new(
                clip_id,
                track_id,
                asset_id,
                start,
                source_in,
                source_out,
                AudioClipProperties::default(),
            )),
            _ => bail!("the drop target is not a media track"),
        };

        let should_save = match new_track {
            Some(track) => edit_timeline(
                timeline,
                EditAction::AddTrackWithClips {
                    track,
                    clips: vec![media_clip],
                    assets,
                },
            )?,
            None => edit_timeline(
                timeline,
                EditAction::AddClips {
                    clips: vec![media_clip],
                    assets,
                },
            )?,
        };

        self.explorer.selected_file = media_path
            .strip_prefix(&self.project_root)
            .ok()
            .map(Path::to_path_buf); // Explorer 的选中项仍相对于项目根目录。
        self.select_only_clip(Some(clip_id));
        let Some(timeline) = self.timeline.as_ref() else {
            return Ok(());
        };
        if should_save {
            timeline.save()?;
            match self.preview.target {
                PreviewTarget::Timeline { .. } => {
                    self.preview.target =
                        timeline_preview_target(timeline, &self.project_root, cx)?;
                }
                _ => {
                    // do nothing
                }
            }
        }

        self.property_panel = current_property_panel(self);
        self.schedule_active_timeline_waveforms(cx);
        Ok(())
    }

    /// Places a reusable pool title as an independent text clip. The title's properties
    /// are copied, so editing the placed clip never changes the pool title. `track_id`
    /// None finds the first unlocked Text track, creating one atomically if needed.
    pub(crate) fn place_title(
        &mut self,
        title_id: Ulid,
        track_id: Option<Ulid>,
        start: TimelineFrameIndex,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let Some(title) = self
            .explorer
            .titles
            .iter()
            .find(|title| title.id == title_id)
        else {
            bail!("Title {title_id} no longer exists");
        };
        let properties = title.properties.clone();
        let duration = title.duration;
        let Some(timeline) = self.timeline.as_ref() else {
            bail!("Open a timeline before adding a title");
        };
        let clip_length = timeline
            .editing_state
            .settings
            .frame_rate
            .frames_from_duration_nearest(duration)
            .max(TimelineFrameIndex::ONE_FRAME);
        let (track_id, new_track) = match track_id {
            Some(track_id) => {
                validate_text_clip_placement(
                    &timeline.editing_state,
                    track_id,
                    clip_length,
                    start,
                    &HashSet::new(),
                )?;
                (track_id, None)
            }
            None => {
                match timeline
                    .editing_state
                    .tracks
                    .iter()
                    .find(|track| track.kind == TrackKind::Text && !track.locked)
                {
                    Some(track) => {
                        validate_text_clip_placement(
                            &timeline.editing_state,
                            track.id,
                            clip_length,
                            start,
                            &HashSet::new(),
                        )?;
                        (track.id, None)
                    }
                    None => {
                        let track = next_track(&timeline.editing_state.tracks, TrackKind::Text);
                        (track.id, Some(track))
                    }
                }
            }
        };

        let Some(timeline) = self.timeline.as_mut() else {
            return Ok(());
        };
        let clip_id = Ulid::generate();
        let text_clip = Clip::Text(TextClip::new(
            clip_id, track_id, start, duration, properties,
        ));
        let should_save = match new_track {
            Some(track) => edit_timeline(
                timeline,
                EditAction::AddTrackWithClips {
                    track,
                    clips: vec![text_clip],
                    assets: Vec::new(),
                },
            )?,
            None => edit_timeline(
                timeline,
                EditAction::AddClips {
                    clips: vec![text_clip],
                    assets: Vec::new(),
                },
            )?,
        };

        // 放置后选中的是时间线片段，不是池标题；预览切回时间线展示新片段。
        self.explorer.selected_title = None;
        self.select_only_clip(Some(clip_id));
        let Some(timeline) = self.timeline.as_ref() else {
            return Ok(());
        };
        if should_save {
            timeline.save()?;
            self.preview.target = timeline_preview_target(timeline, &self.project_root, cx)?;
        }
        self.property_panel = current_property_panel(self);
        Ok(())
    }
}

/// Selects a reusable pool title: clears clip and file selection, points the source
/// monitor at the title and rebuilds the Inspector. Shared by New Text and pool clicks.
fn select_title(editor: &mut Editor, title_id: Ulid) {
    editor.select_only_clip(None);
    editor.explorer.selected_file = None;
    editor.explorer.selected_title = Some(title_id);
    editor.preview.target = PreviewTarget::TextTitle(title_id);
    editor.property_panel = current_property_panel(editor);
}

/// Selects a reusable pool composition: clears clip, file and title selection so the
/// pool highlight and the Inspector agree on what is selected.
fn select_composition(editor: &mut Editor, composition_id: Ulid) {
    editor.select_only_clip(None);
    editor.explorer.selected_file = None;
    editor.explorer.selected_title = None;
    editor.explorer.selected_composition = Some(composition_id);
    editor.property_panel = current_property_panel(editor);
}

fn handle_app_event_edit(
    editor: &Entity<Editor>,
    edit_action: EditAction,
    cx: &mut AsyncApp,
) -> Result<()> {
    editor.update(cx, |editor, cx| -> Result<()> {
        let Some(timeline) = editor.timeline.as_mut() else {
            return Ok(());
        };
        // 播放和 preview 内的 seek 只移动 player；
        // edit 前把 timeline 的 playhead 同步到 player 的 frame_index。
        match &editor.preview.target {
            PreviewTarget::Timeline { path, player, .. }
                if editor.project_root.join(path) == timeline.path =>
            {
                timeline.set_playhead(player.read(cx).backend().frame_index());
            }
            _ => {
                // do nothing
            }
        }
        let track_lock_changed = matches!(edit_action, EditAction::ToggleTrackLock { .. });
        let mute_change = match &edit_action {
            EditAction::SetClipMuted { clip_id, muted } => Some((*clip_id, *muted)),
            _ => None,
        };
        let should_save = edit_timeline(timeline, edit_action)?;
        if should_save {
            timeline.save()?;
            // 预览播放器持有编辑前的快照；编辑提交后按新状态重建。
            // 有 offline clip 时 preview 是 None；relink 之后要重建回 timeline。
            match editor.preview.target {
                PreviewTarget::Timeline {
                    ref player,
                    ref path,
                    ..
                } if mute_change.is_some() && editor.project_root.join(path) == timeline.path => {
                    let (clip_id, muted) = mute_change.expect("mute change was checked");
                    player.update(cx, |player, cx| {
                        let result = player.set_clip_muted(clip_id, muted);
                        cx.notify();
                        result
                    })?;
                }
                PreviewTarget::Timeline { .. } | PreviewTarget::None if !track_lock_changed => {
                    editor.preview.target =
                        timeline_preview_target(timeline, &editor.project_root, cx)?;
                }
                _ => {
                    // do nothing
                }
            }
        }
        editor.property_panel = current_property_panel(editor);
        match (&editor.missing_media_dialog, editor.timeline.as_ref()) {
            (Some(_), Some(timeline)) => {
                let dialog = missing_media_dialog(timeline);
                editor.missing_media_dialog = match dialog.files.is_empty() {
                    true => None,
                    false => Some(dialog),
                };
            }
            _ => {
                // do nothing
            }
        }
        editor.schedule_composition_thumbnails(cx);
        cx.notify();
        Ok(())
    })
}

async fn generate_waveforms(editor: &Entity<Editor>, paths: Vec<PathBuf>, cx: &mut AsyncApp) {
    // 跳过已缓存或正在生成的素材，其余记为生成中。
    let new_paths = editor.update(cx, |editor, _| {
        let mut new_paths = paths
            .into_iter()
            .filter(|path| {
                !editor.waveform_cache.contains_key(path) && !editor.waveform_jobs.contains(path)
            })
            .collect::<Vec<_>>();
        new_paths.sort();
        new_paths.dedup();
        editor.waveform_jobs.extend(new_paths.iter().cloned());
        new_paths
    });
    for media_path in new_paths {
        let source = media_path.clone();
        let result = cx
            .background_executor()
            .spawn(async move { generate_waveform(&source) })
            .await;
        editor.update(cx, |editor, cx| {
            editor.waveform_jobs.remove(&media_path);
            match result {
                Ok(waveform) => {
                    editor.waveform_cache.insert(media_path, Arc::new(waveform));
                }
                Err(error) => log::error!("Waveform: {error:?}"),
            }
            cx.notify();
        });
    }
}

fn timeline_waveform_paths(
    timeline: &TimelineEditingState,
    timeline_directory: &Path,
) -> Vec<PathBuf> {
    let referenced_assets = timeline
        .clips
        .iter()
        .filter_map(|clip| clip.asset_id())
        .collect::<HashSet<_>>();
    timeline
        .assets
        .iter()
        .filter(|asset| asset.has_audio() && referenced_assets.contains(&asset.id()))
        .map(|asset| resolve_asset_path(timeline_directory, asset.path()))
        .collect()
}
