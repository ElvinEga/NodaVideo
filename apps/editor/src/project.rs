use crate::editor::Editor;
use crate::event_bus::AppEvent;
use crate::explorer::save_imported_media;
use crate::explorer_file_entry::visible_tree;
use crate::preview::{PreviewTarget, timeline_preview_target};
use crate::project_settings::{load_project_local_settings, save_project_local_settings};
use crate::timeline::TimelineRuntimeState;
use crate::timeline_document;
use ::timeline::{MediaKind, TimelineFrameIndex, TimelineSerialization, TrackKind};
use anyhow::{Context as _, Result, bail};
use gpui::PathPromptOptions;
use gpui::prelude::*;
use std::path::{Path, PathBuf};
use ulid::Ulid;

impl Editor {
    pub(crate) fn open_project_folder(&mut self, cx: &mut Context<Self>) {
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open project folder".into()),
        });
        cx.spawn(async move |editor, cx| {
            let paths = match selection.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return,
                result => {
                    log::error!(
                        "Folder dialog failed: {result:?} at {}:{}",
                        file!(),
                        line!()
                    );
                    return;
                }
            };
            let Some(project_path) = paths.into_iter().next() else {
                return;
            };
            let _ = editor.update(cx, |editor, cx| {
                editor.emit_event(cx, AppEvent::SwitchProject { project_path });
            });
        })
        .detach();
    }

    pub(crate) fn new_project_dialog(&mut self, cx: &mut Context<Self>) {
        // 原生目录对话框允许在对话框内新建文件夹；选中的文件夹直接作为项目根目录。
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose a folder for the new project".into()),
        });
        cx.spawn(async move |editor, cx| {
            let paths = match selection.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    log::error!("New project folder dialog failed: {error:?}");
                    return;
                }
                Err(error) => {
                    log::error!("New project folder dialog was dropped: {error:?}");
                    return;
                }
            };
            let Some(project_path) = paths.into_iter().next() else {
                return;
            };
            let _ = editor.update(cx, |editor, cx| {
                editor.emit_event(cx, AppEvent::SwitchProject { project_path });
            });
        })
        .detach();
    }

    pub(crate) fn prepare_project_switch(&mut self) -> Result<()> {
        if let Some(timeline) = self.timeline.as_ref() {
            timeline.save()?;
        }
        Ok(())
    }

    pub(super) fn open_timeline(
        &mut self,
        relative_path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        (|| -> Result<()> {
            if self
                .timeline
                .as_ref()
                .is_some_and(|timeline| timeline.path == self.project_root.join(&relative_path))
            {
                self.select_only_clip(None);
                let timeline = self.timeline.as_mut().expect("timeline was checked above");
                timeline.set_playhead(timeline.playhead());
                if !matches!(self.preview.target, PreviewTarget::Timeline { .. }) {
                    self.preview.target =
                        timeline_preview_target(timeline, &self.project_root, cx)?;
                }
                self.explorer.selected_file = Some(relative_path);
                cx.notify();
                return Ok(());
            }
            let path = self.project_root.join(&relative_path);
            let timeline = TimelineSerialization::load(&path)?;
            if let Some(timeline) = self.timeline.as_ref() {
                timeline.save()?;
            }
            self.activate_timeline(relative_path.clone(), timeline, cx)?;
            self.select_only_clip(None);
            self.explorer.selected_file = Some(relative_path.clone());
            Ok(())
        })()
        .context("open_timeline failed")
    }

    /// Saves the current timeline, switches to a freshly created one, and reveals it in
    /// the file tree. `relative_directory` is only used to expand the containing folder.
    pub(crate) fn activate_created_timeline(
        &mut self,
        relative_directory: PathBuf,
        relative_path: PathBuf,
        timeline: TimelineSerialization,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if let Some(active_timeline) = self.timeline.as_ref() {
            active_timeline.save()?;
        }

        // Expand the target folder so the new timeline is visible in the tree.
        if !relative_directory.as_os_str().is_empty() {
            self.explorer
                .expanded_directories
                .insert(relative_directory);
        }
        self.activate_timeline(relative_path.clone(), timeline, cx)?;
        self.explorer.refresh_file_tree(&self.project_root)?;
        self.save_explorer_expansion()?;
        Ok(())
    }

    fn activate_timeline(
        &mut self,
        timeline_path: PathBuf,
        active_timeline: TimelineSerialization,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let res = (|| -> Result<()> {
            // 先完成所有可能失败的工作，全部成功后才修改 self；失败时编辑器保持原状。
            let mut new_timeline = TimelineRuntimeState::from_serialize(
                active_timeline,
                self.project_root.join(timeline_path),
            )?;
            new_timeline.set_playhead(new_timeline.playhead());
            let new_preview_target =
                timeline_preview_target(&new_timeline, &self.project_root, cx)?;
            let new_relative_timeline_path = new_timeline
                .path
                .strip_prefix(&self.project_root)
                .ok()
                .map(Path::to_path_buf);
            let new_file_tree =
                visible_tree(&self.project_root, &self.explorer.expanded_directories)
                    .context("scanning project file tree")?;
            let mut new_settings = load_project_local_settings(&self.project_root);
            new_settings.active_timeline = new_relative_timeline_path.clone();
            save_project_local_settings(&self.project_root, &new_settings)?;

            self.property_panel_state.transform_input_clip_id = None;
            self.property_panel_state.text_input_clip_id = None;
            self.timeline = Some(new_timeline);
            self.preview.target = new_preview_target;
            self.explorer.search_query = None;
            self.explorer.search_results.clear();
            self.explorer.search_pending = false;
            self.explorer
                .filter
                .update(cx, |filter, cx| filter.clear(cx));
            self.explorer.selected_file = new_relative_timeline_path;
            self.dismiss_context_menu();
            self.explorer.file_tree = new_file_tree;
            self.schedule_active_timeline_waveforms(cx);
            Ok(())
        })();
        res.context("activate_timeline failed")
    }

    /// Creates a new timeline with default tracks (V1, V2, A1, A2) and activates it.
    /// If `stem` is provided, the timeline name will be cleanly derived from it.
    pub(crate) fn create_and_activate_new_timeline(
        &mut self,
        stem: Option<&str>,
        cx: &mut Context<Self>,
    ) -> Result<PathBuf> {
        let relative_dir = PathBuf::new();
        let name = match stem {
            Some(s) if !s.trim().is_empty() => {
                timeline_document::unique_timeline_name_from_stem(
                    &self.project_root,
                    &relative_dir,
                    s,
                )
            }
            _ => timeline_document::default_timeline_name(&self.project_root, &relative_dir),
        };
        let (relative_path, timeline) =
            timeline_document::create(&self.project_root, &relative_dir, &name)?;
        self.activate_created_timeline(relative_dir, relative_path.clone(), timeline, cx)?;
        Ok(relative_path)
    }

    /// Automatically creates a new timeline named after `media_path` and inserts the media asset
    /// at frame 0 on the appropriate track (Video 1 or Audio 1).
    pub(crate) fn auto_create_timeline_and_insert_asset(
        &mut self,
        media_path: &Path,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let absolute_path = if media_path.is_absolute() {
            media_path.to_path_buf()
        } else {
            self.project_root.join(media_path)
        };
        let canonical = absolute_path
            .canonicalize()
            .unwrap_or_else(|_| absolute_path.clone());

        // Probe media asset if not cached
        let asset = if let Some(cached) = self.explorer.media_metadata.get(&canonical) {
            cached.clone()
        } else {
            let mut probed = engine::media_asset::probe_media_asset(&canonical)?;
            probed.set_id(Ulid::generate());
            self.explorer
                .media_metadata
                .insert(canonical.clone(), probed.clone());
            probed
        };

        // Ensure asset is tracked in project's imported library
        if !self.explorer.imported_paths.contains(&canonical) {
            self.explorer.imported_paths.insert(canonical.clone());
            self.explorer.rebuild_imported_media();
            let assets: Vec<::timeline::Asset> = self
                .explorer
                .imported_paths
                .iter()
                .filter_map(|path| self.explorer.media_metadata.get(path))
                .cloned()
                .collect();
            let _ = save_imported_media(&self.project_root, &assets);
            self.schedule_project_waveforms(cx);
            self.schedule_media_thumbnails(cx);
        }

        // Determine timeline name from file stem
        let stem = canonical
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Timeline");

        self.create_and_activate_new_timeline(Some(stem), cx)?;

        let Some(timeline) = self.timeline.as_ref() else {
            bail!("Timeline failed to activate");
        };

        let is_audio = asset.kind() == MediaKind::Audio;
        let target_track_id = if is_audio {
            timeline
                .editing_state
                .tracks
                .iter()
                .find(|t| t.kind == TrackKind::Audio)
                .map(|t| t.id)
        } else {
            timeline
                .editing_state
                .tracks
                .iter()
                .find(|t| t.kind == TrackKind::Video)
                .map(|t| t.id)
        };

        let Some(track_id) =
            target_track_id.or_else(|| timeline.editing_state.tracks.first().map(|t| t.id))
        else {
            bail!("Timeline has no tracks");
        };

        self.place_explorer_asset(
            canonical,
            Some(track_id),
            TimelineFrameIndex::ZERO,
            asset,
            None,
            cx,
        )?;

        cx.notify();
        Ok(())
    }
}
