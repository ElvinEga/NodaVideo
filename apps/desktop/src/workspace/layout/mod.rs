//! Workspace layout management and panel state.
//!
//! Owns visibility flags, panel sizing metrics, active workspace mode,
//! and session-level layout persistence.

use gpui_kit::Pixels;
use gpui_kit::SharedString;

use crate::theme::metrics;
use crate::workspace::navigation::WorkspaceTab;

/// State governing workspace layout, panel visibility, and dimensions.
#[derive(Debug, Clone)]
pub struct WorkspaceLayoutState {
    /// Active workspace mode.
    pub active_tab: WorkspaceTab,

    /// Project name displayed in the toolbar.
    pub project_name: SharedString,

    /// Whether the Media Browser panel is visible.
    pub show_media_browser: bool,

    /// Whether the Inspector panel is visible.
    pub show_inspector: bool,

    /// Whether the bottom Timeline panel is visible.
    pub show_timeline: bool,

    /// Whether the dockable AI Assistant panel is visible.
    pub show_assistant: bool,

    /// Current width of the Media Browser.
    pub media_browser_width: Pixels,

    /// Current width of the Inspector.
    pub inspector_width: Pixels,

    /// Current width of the AI Assistant.
    pub assistant_width: Pixels,

    /// Current height of the Timeline panel.
    pub timeline_height: Pixels,

    /// Playback state.
    pub is_playing: bool,

    /// Loop playback state.
    pub is_looping: bool,

    /// Current playhead timecode.
    pub timecode: SharedString,

    /// Selected zoom level string.
    pub zoom_label: SharedString,
}

impl Default for WorkspaceLayoutState {
    fn default() -> Self {
        Self {
            active_tab: WorkspaceTab::Edit,
            project_name: "Untitled Project".into(),
            show_media_browser: true,
            show_inspector: true,
            show_timeline: true,
            show_assistant: true,
            media_browser_width: metrics::MEDIA_BROWSER_DEFAULT_WIDTH,
            inspector_width: metrics::INSPECTOR_DEFAULT_WIDTH,
            assistant_width: metrics::ASSISTANT_DEFAULT_WIDTH,
            timeline_height: metrics::TIMELINE_DEFAULT_HEIGHT,
            is_playing: false,
            is_looping: false,
            timecode: "00:00:00:00".into(),
            zoom_label: "Fit (100%)".into(),
        }
    }
}

impl WorkspaceLayoutState {
    /// Toggle visibility of the Media Browser panel.
    pub fn toggle_media_browser(&mut self) {
        self.show_media_browser = !self.show_media_browser;
    }

    /// Toggle visibility of the Inspector panel.
    pub fn toggle_inspector(&mut self) {
        self.show_inspector = !self.show_inspector;
    }

    /// Toggle visibility of the Timeline panel.
    pub fn toggle_timeline(&mut self) {
        self.show_timeline = !self.show_timeline;
    }

    /// Toggle visibility of the AI Assistant panel.
    pub fn toggle_assistant(&mut self) {
        self.show_assistant = !self.show_assistant;
    }

    /// Reset panels to default proportions and reveal all panels.
    pub fn reset_layout(&mut self) {
        self.show_media_browser = true;
        self.show_inspector = true;
        self.show_timeline = true;
        self.show_assistant = true;
        self.media_browser_width = metrics::MEDIA_BROWSER_DEFAULT_WIDTH;
        self.inspector_width = metrics::INSPECTOR_DEFAULT_WIDTH;
        self.assistant_width = metrics::ASSISTANT_DEFAULT_WIDTH;
        self.timeline_height = metrics::TIMELINE_DEFAULT_HEIGHT;
    }

    /// Toggle playback state.
    pub fn toggle_playback(&mut self) {
        self.is_playing = !self.is_playing;
    }

    /// Toggle looping.
    pub fn toggle_loop(&mut self) {
        self.is_looping = !self.is_looping;
    }
}
