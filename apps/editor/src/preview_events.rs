use crate::editor::Editor;
use crate::preview::{PreviewTarget, SourceMarks};
use crate::timeline::TimelineFrameIndex;
use ::timeline::{MediaKind, TrackKind};
use anyhow::{Result, anyhow, bail};
use gpui::prelude::*;
use gpui::{AsyncApp, Entity};
use player_ui::audio_player::AudioPlayer;
use player_ui::video_player::VideoPlayer;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug)]
pub(crate) enum PreviewEvent {
    SelectFile(PathBuf),
    TogglePlayback,
    MarkSourceIn,
    MarkSourceOut,
    InsertSourceToTimeline,
}

impl Editor {
    pub(crate) async fn open_file_preview(
        editor: Entity<Self>,
        project_root: PathBuf,
        relative_path: PathBuf,
        audio_only: bool,
        cx: &mut AsyncApp,
    ) -> Result<()> {
        let source = project_root.join(&relative_path);
        editor.update(cx, |editor, cx| -> Result<()> {
            if !file_preview_requested(
                &editor.project_root,
                editor.explorer.selected_file.as_deref(),
                &editor.preview.target,
                &project_root,
                &relative_path,
            ) {
                return Ok(());
            }
            // media_backend is not Send, so the player opens on the UI thread.
            editor.preview.target = if audio_only {
                let player = AudioPlayer::new(source)?;
                let player = cx.new(move |_| player);
                let task = player.update(cx, |player, cx| player.start(cx));
                PreviewTarget::AudioFile {
                    _task: task,
                    path: relative_path.clone(),
                    player,
                }
            } else {
                let player = VideoPlayer::new(source)?;
                let player = cx.new(move |_| player);
                let task = player.update(cx, |player, cx| player.start(cx));
                PreviewTarget::VideoFile {
                    _task: task,
                    path: relative_path.clone(),
                    player,
                }
            };
            cx.notify();
            Ok(())
        })?;
        Ok(())
    }
}

fn file_preview_requested(
    project_root: &Path,
    selected_file: Option<&Path>,
    target: &PreviewTarget,
    requested_root: &Path,
    requested_file: &Path,
) -> bool {
    project_root == requested_root
        && selected_file == Some(requested_file)
        && matches!(target, PreviewTarget::None)
}

impl Editor {
    pub(crate) fn toggle_preview_playback(&mut self, cx: &mut Context<Self>) -> Result<()> {
        match &self.preview.target {
            PreviewTarget::VideoFile { player, .. } => {
                player.update(cx, |player, cx| player.toggle_playback(cx));
                Ok(())
            }
            PreviewTarget::AudioFile { player, .. } => {
                player.update(cx, |player, cx| player.toggle_playback(cx))
            }
            PreviewTarget::Timeline { player, .. } => {
                player.update(cx, |player, cx| player.toggle_playback(cx))
            }
            PreviewTarget::None | PreviewTarget::Error { .. } | PreviewTarget::ImageFile(_) => {
                Ok(())
            }
        }
    }
}

// Public APIs only
impl Editor {
    pub(crate) fn mark_source_in(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let Some((path, position)) = self.source_preview_position(cx)? else {
            return Ok(());
        };
        let Some(timeline) = self.timeline.as_ref() else {
            return Ok(()); // 没有时间线就没有帧基准，入出点无处可用。
        };
        let frame = timeline.editing_state.nearest_time(position.as_secs_f64());
        let marks = match self.preview.source_marks.take() {
            Some(mut marks) if marks.path == path => {
                marks.in_frame = Some(frame);
                if marks.out_frame.is_some_and(|out_frame| out_frame <= frame) {
                    marks.out_frame = None; // 入点不早于出点时，旧出点失效。
                }
                marks
            }
            _ => SourceMarks {
                path,
                in_frame: Some(frame),
                out_frame: None,
            },
        };
        self.preview.source_marks = Some(marks);
        cx.notify();
        Ok(())
    }

    pub(crate) fn mark_source_out(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let Some((path, position)) = self.source_preview_position(cx)? else {
            return Ok(());
        };
        let Some(timeline) = self.timeline.as_ref() else {
            return Ok(());
        };
        let frame = timeline.editing_state.nearest_time(position.as_secs_f64());
        let marks = match self.preview.source_marks.take() {
            Some(mut marks) if marks.path == path => {
                marks.out_frame = Some(frame);
                if marks.in_frame.is_some_and(|in_frame| in_frame >= frame) {
                    marks.in_frame = None; // 出点不晚于入点时，旧入点失效。
                }
                marks
            }
            _ => SourceMarks {
                path,
                in_frame: None,
                out_frame: Some(frame),
            },
        };
        self.preview.source_marks = Some(marks);
        cx.notify();
        Ok(())
    }

    pub(crate) fn insert_source_to_timeline(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let path = match &self.preview.target {
            PreviewTarget::VideoFile { path, .. } | PreviewTarget::AudioFile { path, .. } => {
                path.clone()
            }
            PreviewTarget::None
            | PreviewTarget::Error { .. }
            | PreviewTarget::Timeline { .. }
            | PreviewTarget::ImageFile(_) => return Ok(()), // 没有可插入的源文件。
        };
        let Some(timeline) = self.timeline.as_ref() else {
            bail!("Open a timeline before inserting source media");
        };
        let absolute_path = self.project_root.join(&path);
        let canonical_path = std::fs::canonicalize(&absolute_path)?;
        let asset = self
            .explorer
            .media_metadata
            .get(&canonical_path)
            .cloned()
            .ok_or_else(|| anyhow!("Media info for {} is not ready yet", path.display()))?;
        let full_duration = timeline
            .editing_state
            .nearest_time(asset.duration().unwrap_or(0.0))
            .max(TimelineFrameIndex::ONE_FRAME);
        let marks = self
            .preview
            .source_marks
            .as_ref()
            .filter(|marks| marks.path == path);
        let in_frame = marks
            .and_then(|marks| marks.in_frame)
            .unwrap_or(TimelineFrameIndex::ZERO);
        let out_frame = marks
            .and_then(|marks| marks.out_frame)
            .unwrap_or(full_duration)
            .min(full_duration);
        if out_frame <= in_frame {
            bail!("The source out point must be after the in point");
        }
        let track_kind = match asset.kind() {
            MediaKind::Video => TrackKind::Video,
            MediaKind::Audio => TrackKind::Audio,
            MediaKind::Image => bail!("Image placement is not supported yet"),
        };
        let track_id = timeline
            .editing_state
            .tracks
            .iter()
            .find(|track| track.kind == track_kind && !track.locked)
            .map(|track| track.id)
            .ok_or_else(|| anyhow!("No unlocked {track_kind:?} track is available"))?;
        let start = timeline.playhead();
        self.place_explorer_asset(
            absolute_path,
            track_id,
            start,
            asset,
            Some((in_frame, out_frame)),
            cx,
        )
    }
}

// Private APIs only
impl Editor {
    fn source_preview_position(
        &self,
        cx: &mut Context<Self>,
    ) -> Result<Option<(PathBuf, Duration)>> {
        match &self.preview.target {
            PreviewTarget::VideoFile { path, player, .. } => {
                let position = player.update(cx, |player, _| player.position())?;
                Ok(Some((path.clone(), position)))
            }
            PreviewTarget::AudioFile { path, player, .. } => {
                let position = player.update(cx, |player, _| player.position());
                Ok(Some((path.clone(), position)))
            }
            PreviewTarget::None
            | PreviewTarget::Error { .. }
            | PreviewTarget::Timeline { .. }
            | PreviewTarget::ImageFile(_) => Ok(None),
        }
    }
}

#[cfg(test)]
#[path = "tests/preview_events.test.rs"]
mod tests;
