//! Workspace navigation and tab switching.
//!
//! Provides the primary workspace modes: Edit, Color, Audio, and AI Studio.

use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::*;

/// Primary workspace modes in NodaVideo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkspaceTab {
    /// Multi-track timeline and editing workspace (default).
    #[default]
    Edit,
    /// Color grading, scopes, and LUTs.
    Color,
    /// Multitrack audio mixer and mastering.
    Audio,
    /// AI Studio, skills canvas, and agent workspace.
    AiStudio,
}

impl WorkspaceTab {
    /// Return the index of this tab (0..4).
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Edit => 0,
            Self::Color => 1,
            Self::Audio => 2,
            Self::AiStudio => 3,
        }
    }

    /// Convert a 0-based index to a WorkspaceTab variant.
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Color,
            2 => Self::Audio,
            3 => Self::AiStudio,
            _ => Self::Edit,
        }
    }

    /// Return the user-facing label for the tab.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Edit => "Edit",
            Self::Color => "Color",
            Self::Audio => "Audio",
            Self::AiStudio => "AI Studio",
        }
    }
}

/// Render the workspace navigation tab bar.
pub fn render_workspace_tabs(
    current: WorkspaceTab,
    on_change: impl Fn(&WorkspaceTab, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    TabBar::new("workspace-tabs")
        .segmented()
        .selected_index(current.index())
        .child(Tab::new().label(WorkspaceTab::Edit.label()))
        .child(Tab::new().label(WorkspaceTab::Color.label()))
        .child(Tab::new().label(WorkspaceTab::Audio.label()))
        .child(Tab::new().label(WorkspaceTab::AiStudio.label()))
        .on_click(move |index, window, cx| {
            let next = WorkspaceTab::from_index(*index);
            on_change(&next, window, cx);
        })
}
