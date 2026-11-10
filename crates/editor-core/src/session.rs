//! Central editor session holding project document, timeline sequence, command history,
//! selection state, playback transport, and subsystem coordinators.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use nodavideo_agent_runtime::AgentRuntime;
use nodavideo_command_system::{ChangesetDiff, CommandHistory, EditorCommand, StagedChangeset};
use nodavideo_media_engine::{ExportReport, ExportSettings, MediaCacheDb};
use nodavideo_project::{
    AssetId, MediaAsset, MediaAssetStatus, MediaType, ProjectDocument, TimeRational,
};
use nodavideo_skill_registry::SkillRegistry;
use nodavideo_timeline::{Clip, ClipId, Sequence, Track, TrackId, TransitionId};

use crate::error::EditorError;
use crate::export::ExportCoordinator;
use crate::playback::PlaybackController;
use crate::selection::SelectionState;

/// The central state container and coordinator for an open editing session.
#[derive(Debug)]
pub struct EditorSession {
    /// Active project document.
    pub project: ProjectDocument,
    /// Absolute or relative path to the project file on disk, if saved.
    pub project_path: Option<PathBuf>,
    /// Active multitrack timeline sequence.
    pub sequence: Sequence,
    /// Unified undo/redo command history.
    pub history: CommandHistory,
    /// Active timeline selection state.
    pub selection: SelectionState,
    /// Playback transport controller.
    pub playback: PlaybackController,
    /// Background export coordinator.
    pub export: ExportCoordinator,
    /// SQLite media cache index.
    pub media_cache: Option<Arc<MediaCacheDb>>,
    /// Pluggable AI agent runtime.
    pub agent_runtime: Arc<AgentRuntime>,
    /// Production skill registry.
    pub skill_registry: Arc<SkillRegistry>,
    /// Pending staged AI proposal awaiting user approval, if any.
    pub staged_changeset: Option<StagedChangeset>,
}

impl Default for EditorSession {
    fn default() -> Self {
        Self::new_empty("Untitled Project")
    }
}

impl EditorSession {
    /// Create a new editor session with an empty project and default sequence.
    pub fn new_empty(name: impl Into<String>) -> Self {
        let mut project = ProjectDocument::new(name);
        let sequence = Sequence::default_1080p24();
        let _ = sequence.attach_to_project(&mut project);

        Self {
            project,
            project_path: None,
            sequence,
            history: CommandHistory::default(),
            selection: SelectionState::new(),
            playback: PlaybackController::default(),
            export: ExportCoordinator::new(),
            media_cache: None,
            agent_runtime: Arc::new(AgentRuntime::new()),
            skill_registry: Arc::new(SkillRegistry::new()),
            staged_changeset: None,
        }
    }

    /// Create an editor session from an existing project document.
    pub fn from_project(
        mut project: ProjectDocument,
        project_path: Option<PathBuf>,
    ) -> Result<Self, EditorError> {
        let sequence =
            Sequence::extract_from_project(&project)?.unwrap_or_else(Sequence::default_1080p24);
        let _ = sequence.attach_to_project(&mut project);
        let fps = sequence.fps;

        Ok(Self {
            project,
            project_path,
            sequence,
            history: CommandHistory::default(),
            selection: SelectionState::new(),
            playback: PlaybackController::with_fps(fps),
            export: ExportCoordinator::new(),
            media_cache: None,
            agent_runtime: Arc::new(AgentRuntime::new()),
            skill_registry: Arc::new(SkillRegistry::new()),
            staged_changeset: None,
        })
    }

    /// Load an editor session from a project file on disk.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, EditorError> {
        let path_ref = path.as_ref();
        let project = ProjectDocument::load_from_file(path_ref)?;
        Self::from_project(project, Some(path_ref.to_path_buf()))
    }

    /// Save the project document to its currently associated file path.
    pub fn save(&mut self) -> Result<(), EditorError> {
        let path = self.project_path.clone().ok_or_else(|| {
            EditorError::Other(
                "Cannot save project: no file path specified. Use save_as() instead.".into(),
            )
        })?;
        self.save_as(path)
    }

    /// Save the project document atomically to a designated file path.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<(), EditorError> {
        let path_ref = path.as_ref();
        self.sequence.attach_to_project(&mut self.project)?;
        self.project.modified_at = Utc::now().to_rfc3339();
        self.project.save_to_file(path_ref)?;
        self.project_path = Some(path_ref.to_path_buf());
        Ok(())
    }

    // ── Command Execution & History ──────────────────────────────────────────

    /// Execute a command mutating the active sequence and record it to history.
    pub fn execute_command(&mut self, command: EditorCommand) -> Result<(), EditorError> {
        self.history.execute(command, &mut self.sequence)?;
        let _ = self.sequence.attach_to_project(&mut self.project);
        Ok(())
    }

    /// Undo the most recent command. Returns the description of undone action if successful.
    pub fn undo(&mut self) -> Result<Option<String>, EditorError> {
        let res = self.history.undo(&mut self.sequence)?;
        let _ = self.sequence.attach_to_project(&mut self.project);
        Ok(res)
    }

    /// Redo the most recently undone command. Returns the description of redone action if successful.
    pub fn redo(&mut self) -> Result<Option<String>, EditorError> {
        let res = self.history.redo(&mut self.sequence)?;
        let _ = self.sequence.attach_to_project(&mut self.project);
        Ok(res)
    }

    /// Whether an action is available to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Whether an action is available to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    // ── Staged Changeset (AI Workflows) ──────────────────────────────────────

    /// Stage a new multi-command proposal from an AI workflow.
    pub fn stage_changeset(
        &mut self,
        changeset: StagedChangeset,
    ) -> Result<ChangesetDiff, EditorError> {
        let diff = changeset.compute_diff(&self.sequence)?;
        self.staged_changeset = Some(changeset);
        Ok(diff)
    }

    /// Whether a staged changeset is currently pending review.
    #[must_use]
    pub fn has_staged_changeset(&self) -> bool {
        self.staged_changeset.is_some()
    }

    /// Preview speculative sequence with staged changes applied.
    pub fn preview_staged_sequence(
        &self,
    ) -> Result<Option<(Sequence, ChangesetDiff)>, EditorError> {
        if let Some(staged) = &self.staged_changeset {
            let seq = staged.apply_speculative(&self.sequence)?;
            let diff = staged.compute_diff(&self.sequence)?;
            Ok(Some((seq, diff)))
        } else {
            Ok(None)
        }
    }

    /// Commit the currently staged changeset into the active sequence and command history.
    pub fn commit_staged_changeset(&mut self) -> Result<(), EditorError> {
        let staged = self
            .staged_changeset
            .take()
            .ok_or(EditorError::NoStagedChangeset)?;
        let cmd = staged.into_batch_command();
        self.execute_command(cmd)
    }

    /// Discard the currently staged changeset without modifying timeline state.
    pub fn discard_staged_changeset(&mut self) -> Result<(), EditorError> {
        if self.staged_changeset.take().is_some() {
            Ok(())
        } else {
            Err(EditorError::NoStagedChangeset)
        }
    }

    // ── Selection State ──────────────────────────────────────────────────────

    /// Select a single clip.
    pub fn select_clip(&mut self, clip_id: ClipId) {
        self.selection.select_clip(clip_id);
    }

    /// Toggle clip selection.
    pub fn toggle_clip_selection(&mut self, clip_id: ClipId) {
        self.selection.toggle_clip_selection(clip_id);
    }

    /// Select an edit cut transition.
    pub fn select_transition(&mut self, transition_id: Option<TransitionId>) {
        self.selection.select_transition(transition_id);
    }

    /// Select a track.
    pub fn select_track(&mut self, track_id: Option<TrackId>) {
        self.selection.select_track(track_id);
    }

    /// Clear all selections.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Retrieve all currently selected clips from the active sequence.
    #[must_use]
    pub fn get_selected_clips(&self) -> Vec<Clip> {
        self.selection
            .selected_clips
            .iter()
            .filter_map(|&id| self.sequence.get_clip(id).cloned())
            .collect()
    }

    // ── Transport & Playback ─────────────────────────────────────────────────

    /// Maximum timeline duration in frames.
    #[must_use]
    pub fn duration_frames(&self) -> u64 {
        self.sequence.duration_frames().max(0) as u64
    }

    /// Seek playhead to frame.
    pub fn seek_frame(&mut self, frame: u64) {
        let max_frame = self.duration_frames();
        self.playback.seek_frame(frame, max_frame);
    }

    /// Step playhead forward.
    pub fn step_forward(&mut self, frames: u64) {
        let max_frame = self.duration_frames();
        self.playback.step_forward(frames, max_frame);
    }

    /// Step playhead backward.
    pub fn step_backward(&mut self, frames: u64) {
        self.playback.step_backward(frames);
    }

    /// Jump playhead to start.
    pub fn jump_to_start(&mut self) {
        self.playback.jump_to_start();
    }

    /// Jump playhead to end.
    pub fn jump_to_end(&mut self) {
        let max_frame = self.duration_frames();
        self.playback.jump_to_end(max_frame);
    }

    /// Jump to previous cut point.
    pub fn jump_to_previous_cut(&mut self) {
        self.playback.jump_to_previous_cut(&self.sequence);
    }

    /// Jump to next cut point.
    pub fn jump_to_next_cut(&mut self) {
        self.playback.jump_to_next_cut(&self.sequence);
    }

    /// Start playback.
    pub fn play(&mut self) {
        self.playback.play();
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        self.playback.pause();
    }

    /// Toggle playback.
    pub fn toggle_playback(&mut self) {
        self.playback.toggle_playback();
    }

    /// Advance playhead by real delta seconds.
    pub fn advance_playback(&mut self, delta_seconds: f64) -> u64 {
        let max_frames = self.duration_frames();
        self.playback.advance_delta(delta_seconds, max_frames)
    }

    // ── Asset Management ─────────────────────────────────────────────────────

    /// Register an imported media asset in the project document.
    pub fn add_media_asset(&mut self, asset: MediaAsset) -> AssetId {
        let id = asset.id;
        self.project.assets.push(asset);
        id
    }

    /// Register a media file as an asset.
    pub fn import_file(
        &mut self,
        file_path: PathBuf,
        media_type: MediaType,
        file_size: u64,
        duration: Option<TimeRational>,
        dimensions: Option<(u32, u32)>,
    ) -> MediaAsset {
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("media")
            .to_string();

        let (width, height) = match dimensions {
            Some((w, h)) => (Some(w), Some(h)),
            None => (None, None),
        };

        let asset = MediaAsset {
            id: AssetId::new(),
            original_filename: filename,
            file_path,
            media_type,
            file_size,
            import_timestamp: Utc::now().to_rfc3339(),
            duration,
            width,
            height,
            frame_rate: Some(self.sequence.fps),
            video_codec: None,
            pixel_format: None,
            audio_codec: None,
            audio_sample_rate: Some(48_000),
            audio_channels: Some(2),
            audio_bitrate: None,
            status: MediaAssetStatus::Ready,
            error_message: None,
        };

        self.add_media_asset(asset.clone());
        asset
    }

    /// Retrieve asset by ID.
    #[must_use]
    pub fn get_asset(&self, id: AssetId) -> Option<&MediaAsset> {
        self.project.assets.iter().find(|a| a.id == id)
    }

    // ── Export ───────────────────────────────────────────────────────────────

    /// Execute sequence export using the given settings.
    pub fn export_sequence(
        &mut self,
        settings: &ExportSettings,
    ) -> Result<ExportReport, EditorError> {
        self.export
            .execute_export(&self.sequence, &self.project, settings)
    }

    // ── Track Convenience Helpers ────────────────────────────────────────────

    /// Retrieve active track by ID.
    #[must_use]
    pub fn get_track(&self, track_id: TrackId) -> Option<&Track> {
        self.sequence.get_track(track_id)
    }

    /// Retrieve active clip by ID.
    #[must_use]
    pub fn get_clip(&self, clip_id: ClipId) -> Option<&Clip> {
        self.sequence.get_clip(clip_id)
    }
}
