#![recursion_limit = "512"]

pub mod app;
pub mod theme;
pub mod workspace;

fn main() {
    app::run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[core::prelude::v1::test]
    fn test_workspace_tab_indexing() {
        use workspace::navigation::WorkspaceTab;
        assert_eq!(WorkspaceTab::Edit.index(), 0);
        assert_eq!(WorkspaceTab::Color.index(), 1);
        assert_eq!(WorkspaceTab::Audio.index(), 2);
        assert_eq!(WorkspaceTab::AiStudio.index(), 3);
        assert_eq!(WorkspaceTab::from_index(0), WorkspaceTab::Edit);
        assert_eq!(WorkspaceTab::from_index(1), WorkspaceTab::Color);
        assert_eq!(WorkspaceTab::from_index(2), WorkspaceTab::Audio);
        assert_eq!(WorkspaceTab::from_index(3), WorkspaceTab::AiStudio);
        assert_eq!(WorkspaceTab::from_index(99), WorkspaceTab::Edit);
    }

    #[core::prelude::v1::test]
    fn test_workspace_layout_state_toggles() {
        use workspace::layout::WorkspaceLayoutState;
        let mut state = WorkspaceLayoutState::default();
        assert!(state.show_media_browser);
        assert!(state.show_inspector);
        assert!(state.show_timeline);
        assert!(state.show_assistant);

        state.toggle_media_browser();
        assert!(!state.show_media_browser);

        state.toggle_assistant();
        assert!(!state.show_assistant);

        state.reset_layout();
        assert!(state.show_media_browser);
        assert!(state.show_assistant);
    }
}
