//! Application menus and keyboard shortcuts.
//!
//! Exposes File, Edit, View, Playback, Window, and Help menus integrated
//! with native platform menu bars and shortcut dispatch.

use gpui_kit::*;

actions!(
    nodavideo,
    [
        NewProject,
        OpenProject,
        SaveProject,
        ImportMedia,
        ExportVideoAction,
        QuitAction,
        UndoAction,
        RedoAction,
        ToggleMediaBrowser,
        ToggleInspector,
        ToggleTimeline,
        ToggleAssistant,
        ResetWorkspaceLayout,
        SelectEditTab,
        SelectColorTab,
        SelectAudioTab,
        SelectAiStudioTab,
        PlayPauseAction,
        StepForwardAction,
        StepBackwardAction,
        LoopPlaybackAction,
        SplitClipAction,
        DeleteClipAction,
        AboutNodaVideo,
    ]
);

/// Initialize native menus and global key bindings.
pub fn init(cx: &mut App) {
    let is_mac = cx.platform_info().is_mac();

    // 1. Register keybindings first so menu items display shortcuts
    if is_mac {
        cx.bind_keys([
            // File
            KeyBinding::new("cmd-n", NewProject, None),
            KeyBinding::new("cmd-o", OpenProject, None),
            KeyBinding::new("cmd-s", SaveProject, None),
            KeyBinding::new("cmd-i", ImportMedia, None),
            KeyBinding::new("cmd-e", ExportVideoAction, None),
            KeyBinding::new("cmd-q", QuitAction, None),
            // Edit
            KeyBinding::new("cmd-z", UndoAction, None),
            KeyBinding::new("cmd-shift-z", RedoAction, None),
            KeyBinding::new("cmd-k", SplitClipAction, None),
            KeyBinding::new("backspace", DeleteClipAction, None),
            // View / Panels
            KeyBinding::new("cmd-1", ToggleMediaBrowser, None),
            KeyBinding::new("cmd-2", ToggleInspector, None),
            KeyBinding::new("cmd-3", ToggleTimeline, None),
            KeyBinding::new("cmd-4", ToggleAssistant, None),
            // Playback
            KeyBinding::new("space", PlayPauseAction, None),
            KeyBinding::new("right", StepForwardAction, None),
            KeyBinding::new("left", StepBackwardAction, None),
            KeyBinding::new("l", LoopPlaybackAction, None),
        ]);
    } else {
        cx.bind_keys([
            // File
            KeyBinding::new("ctrl-n", NewProject, None),
            KeyBinding::new("ctrl-o", OpenProject, None),
            KeyBinding::new("ctrl-s", SaveProject, None),
            KeyBinding::new("ctrl-i", ImportMedia, None),
            KeyBinding::new("ctrl-e", ExportVideoAction, None),
            KeyBinding::new("ctrl-q", QuitAction, None),
            // Edit
            KeyBinding::new("ctrl-z", UndoAction, None),
            KeyBinding::new("ctrl-shift-z", RedoAction, None),
            KeyBinding::new("ctrl-k", SplitClipAction, None),
            KeyBinding::new("backspace", DeleteClipAction, None),
            // View / Panels
            KeyBinding::new("ctrl-1", ToggleMediaBrowser, None),
            KeyBinding::new("ctrl-2", ToggleInspector, None),
            KeyBinding::new("ctrl-3", ToggleTimeline, None),
            KeyBinding::new("ctrl-4", ToggleAssistant, None),
            // Playback
            KeyBinding::new("space", PlayPauseAction, None),
            KeyBinding::new("right", StepForwardAction, None),
            KeyBinding::new("left", StepBackwardAction, None),
            KeyBinding::new("l", LoopPlaybackAction, None),
        ]);
    }

    // 2. Build the application menu bar
    cx.set_menus(vec![
        // App / File Menu
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Project", NewProject),
                MenuItem::action("Open Project…", OpenProject),
                MenuItem::separator(),
                MenuItem::action("Save Project", SaveProject),
                MenuItem::action("Import Media…", ImportMedia),
                MenuItem::action("Export Video…", ExportVideoAction),
                MenuItem::separator(),
                MenuItem::action("Quit", QuitAction),
            ],
            disabled: false,
        },
        // Edit Menu
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", UndoAction),
                MenuItem::action("Redo", RedoAction),
                MenuItem::separator(),
                MenuItem::action("Split Clip at Playhead", SplitClipAction),
                MenuItem::action("Delete Selected Clip", DeleteClipAction),
            ],
            disabled: false,
        },
        // View Menu
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Media Browser", ToggleMediaBrowser),
                MenuItem::action("Toggle Inspector", ToggleInspector),
                MenuItem::action("Toggle Timeline", ToggleTimeline),
                MenuItem::action("Toggle AI Assistant", ToggleAssistant),
                MenuItem::separator(),
                MenuItem::action("Reset Layout", ResetWorkspaceLayout),
                MenuItem::separator(),
                MenuItem::action("Edit Workspace", SelectEditTab),
                MenuItem::action("Color Workspace", SelectColorTab),
                MenuItem::action("Audio Workspace", SelectAudioTab),
                MenuItem::action("AI Studio Workspace", SelectAiStudioTab),
            ],
            disabled: false,
        },
        // Playback Menu
        Menu {
            name: "Playback".into(),
            items: vec![
                MenuItem::action("Play / Pause", PlayPauseAction),
                MenuItem::action("Step Forward 1 Frame", StepForwardAction),
                MenuItem::action("Step Backward 1 Frame", StepBackwardAction),
                MenuItem::action("Loop Playback", LoopPlaybackAction),
            ],
            disabled: false,
        },
        // Window Menu
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Toggle AI Assistant", ToggleAssistant),
                MenuItem::separator(),
                MenuItem::action("Reset Default Layout", ResetWorkspaceLayout),
            ],
            disabled: false,
        },
        // Help Menu
        Menu {
            name: "Help".into(),
            items: vec![MenuItem::action("About NodaVideo", AboutNodaVideo)],
            disabled: false,
        },
    ]);
}
