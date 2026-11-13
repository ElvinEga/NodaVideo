use crate::edit_action::EditAction;
use crate::editing::ClipClipboard;
use crate::editor::{Editor, RightDockTab, WorkspaceTab};
use crate::event_bus::{AppEvent, EventBus};
use crate::preview_events::PreviewEvent;
use crate::timeline_interactions::TimelineTool;
use crate::ui_components;
use gpui::prelude::*;
use gpui::{App, KeyBinding, Menu, MenuItem, SystemMenuType, Window, actions};
use std::path::PathBuf;

pub(crate) const EDITOR_KEY_CONTEXT: &str = "Editor";
const EDITOR_SHORTCUT_CONTEXT: &str = "Editor && !TextInput && !TextArea";

actions!(
    nodavideo_editor,
    [
        TogglePlayback,
        StepBackwardFrame,
        StepForwardFrame,
        DeleteSelected,
        SplitClip,
        Undo,
        Redo,
        DuplicateSelected,
        CopySelectedClips,
        CutSelectedClips,
        PasteClips,
        SelectAllUnlockedClips,
        SelectClipsRightOfPointer,
        SelectClipsLeftOfPointer,
        ActivateSelectionTool,
        ActivateBladeTool,
        ToggleFullscreen,
        ExitFullscreen,
        ToggleInspector,
        ToggleAssistant,
        RevealInFinder,
        OpenInDefaultApp,
        QuitApp,
        ImportMedia,
        ToggleMediaBrowser,
        SwitchToEditWorkspace,
        SwitchToColorWorkspace,
        SwitchToAudioWorkspace,
        SwitchToAiStudioWorkspace,
        GoToStart,
        GoToEnd,
        ExportVideoAction,
        OpenProjectFolderAction,
        OpenProjectPickerAction,
        OpenSettingsAction,
        NewTimelineAction,
        NewProjectAction,
        SaveProject,
        CloseWindow,
        MinimizeWindow,
        ZoomWindow,
        BringAllToFront,
        ShowKeyboardShortcuts,
        ShowAbout,
        ToggleDebugInspector,
        MarkSourceIn,
        MarkSourceOut,
        InsertSourceToTimeline
    ]
);

pub(crate) fn set_app_menus(cx: &mut App) {
    cx.set_menus([
        Menu::new("NodaVideo").items([
            MenuItem::action("About NodaVideo", ShowAbout),
            MenuItem::separator(),
            MenuItem::action("Settings...", OpenSettingsAction),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Quit NodaVideo", QuitApp),
        ]),
        Menu::new("File").items([
            MenuItem::action("New Project...", NewProjectAction),
            MenuItem::action("New Timeline...", NewTimelineAction),
            MenuItem::action("Open Project Folder...", OpenProjectFolderAction),
            MenuItem::action("Open Project Picker...", OpenProjectPickerAction),
            MenuItem::separator(),
            MenuItem::action("Save", SaveProject),
            MenuItem::action("Close Window", CloseWindow),
            MenuItem::separator(),
            MenuItem::action("Import Media...", ImportMedia),
            MenuItem::separator(),
            MenuItem::action("Export Video...", ExportVideoAction),
        ]),
        Menu::new("Edit").items([
            MenuItem::action("Undo", Undo),
            MenuItem::action("Redo", Redo),
            MenuItem::separator(),
            MenuItem::action("Cut", CutSelectedClips),
            MenuItem::action("Copy", CopySelectedClips),
            MenuItem::action("Paste", PasteClips),
            MenuItem::action("Duplicate", DuplicateSelected),
            MenuItem::action("Delete", DeleteSelected),
            MenuItem::separator(),
            MenuItem::action("Select All", SelectAllUnlockedClips),
            MenuItem::action("Split Clip", SplitClip),
            MenuItem::separator(),
            MenuItem::action("Mark Source In", MarkSourceIn),
            MenuItem::action("Mark Source Out", MarkSourceOut),
            MenuItem::action("Insert Source to Timeline", InsertSourceToTimeline),
        ]),
        Menu::new("View").items([
            MenuItem::action("Toggle Fullscreen", ToggleFullscreen),
            MenuItem::separator(),
            MenuItem::action("Toggle Media Browser", ToggleMediaBrowser),
            MenuItem::action("Toggle Inspector", ToggleInspector),
            MenuItem::action("Toggle AI Assistant", ToggleAssistant),
            MenuItem::separator(),
            MenuItem::action("Edit Workspace", SwitchToEditWorkspace),
            MenuItem::action("Color Workspace", SwitchToColorWorkspace),
            MenuItem::action("Audio Workspace", SwitchToAudioWorkspace),
            MenuItem::action("AI Studio Workspace", SwitchToAiStudioWorkspace),
        ]),
        Menu::new("Playback").items([
            MenuItem::action("Play / Pause", TogglePlayback),
            MenuItem::separator(),
            MenuItem::action("Mark In", MarkSourceIn),
            MenuItem::action("Mark Out", MarkSourceOut),
            MenuItem::action("Insert to Timeline", InsertSourceToTimeline),
            MenuItem::separator(),
            MenuItem::action("Step Backward One Frame", StepBackwardFrame),
            MenuItem::action("Step Forward One Frame", StepForwardFrame),
            MenuItem::separator(),
            MenuItem::action("Jump to Start", GoToStart),
            MenuItem::action("Jump to End", GoToEnd),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", MinimizeWindow),
            MenuItem::action("Zoom", ZoomWindow),
            MenuItem::separator(),
            MenuItem::action("Bring All to Front", BringAllToFront),
        ]),
        Menu::new("Help").items([
            MenuItem::action("Keyboard Shortcuts", ShowKeyboardShortcuts),
            MenuItem::action("NodaVideo Help", gpui::NoAction).disabled(true),
        ]),
    ]);
}

pub(crate) fn bind_keys(cx: &mut App) {
    ui_components::text_input::bind_keys(cx);
    ui_components::text_area::bind_keys(cx);
    ui_components::text_input_v2::bind_keys(cx);
    cx.bind_keys([
        KeyBinding::new("space", TogglePlayback, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("left", StepBackwardFrame, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("right", StepForwardFrame, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("home", GoToStart, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-left", GoToStart, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("end", GoToEnd, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-right", GoToEnd, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("backspace", DeleteSelected, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("delete", DeleteSelected, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-b", SplitClip, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-z", Undo, Some("Editor && !TextInput")),
        KeyBinding::new("cmd-shift-z", Redo, Some("Editor && !TextInput")),
        KeyBinding::new("cmd-d", DuplicateSelected, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-c", CopySelectedClips, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-x", CutSelectedClips, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-v", PasteClips, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new(
            "cmd-a",
            SelectAllUnlockedClips,
            Some(EDITOR_SHORTCUT_CONTEXT),
        ),
        KeyBinding::new(
            "]",
            SelectClipsRightOfPointer,
            Some(EDITOR_SHORTCUT_CONTEXT),
        ),
        KeyBinding::new(
            "】", // 中文输入法下 ] 键产生的符号
            SelectClipsRightOfPointer,
            Some(EDITOR_SHORTCUT_CONTEXT),
        ),
        KeyBinding::new("[", SelectClipsLeftOfPointer, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new(
            "【", // 中文输入法下 [ 键产生的符号
            SelectClipsLeftOfPointer,
            Some(EDITOR_SHORTCUT_CONTEXT),
        ),
        KeyBinding::new("v", ActivateSelectionTool, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("b", ActivateBladeTool, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("i", MarkSourceIn, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("o", MarkSourceOut, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new(".", InsertSourceToTimeline, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("f", ToggleFullscreen, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-f", ToggleFullscreen, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("escape", ExitFullscreen, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-alt-i", ToggleDebugInspector, None),
        KeyBinding::new("cmd-shift-a", ToggleAssistant, None),
        KeyBinding::new("cmd-alt-a", ToggleAssistant, None),
        KeyBinding::new("cmd-i", ImportMedia, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-1", ToggleMediaBrowser, None),
        KeyBinding::new("cmd-2", ToggleInspector, None),
        KeyBinding::new("cmd-alt-m", ToggleMediaBrowser, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-alt-1", SwitchToEditWorkspace, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-alt-2", SwitchToColorWorkspace, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-alt-3", SwitchToAudioWorkspace, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-alt-4", SwitchToAiStudioWorkspace, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-e", ExportVideoAction, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-o", OpenProjectFolderAction, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-shift-o", OpenProjectPickerAction, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-n", NewProjectAction, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-shift-n", NewTimelineAction, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-s", SaveProject, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-m", MinimizeWindow, None),
        KeyBinding::new("cmd-,", OpenSettingsAction, None),
        KeyBinding::new("cmd-q", QuitApp, None),
        KeyBinding::new("i", MarkSourceIn, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("o", MarkSourceOut, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("w", InsertSourceToTimeline, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new("cmd-alt-r", RevealInFinder, Some(EDITOR_SHORTCUT_CONTEXT)),
        KeyBinding::new(
            "ctrl-shift-enter",
            OpenInDefaultApp,
            Some(EDITOR_SHORTCUT_CONTEXT),
        ),
    ]);
    cx.on_action::<ToggleDebugInspector>(|_, cx| {
        let Some(window) = cx.active_window() else {
            return;
        };
        let _ = window.update(cx, |_, window, cx| {
            crate::gpui_inspector::toggle(window, cx)
        });
    });
    cx.on_action::<QuitApp>(|_, cx| {
        cx.quit();
    });
}

impl Editor {
    pub(crate) fn action_toggle_playback(
        &mut self,
        _: &TogglePlayback,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.emit_event(cx, AppEvent::Preview(PreviewEvent::TogglePlayback));
        cx.notify();
    }

    pub(crate) fn action_step_backward_frame(
        &mut self,
        _: &StepBackwardFrame,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.step_playhead(-1) {
            log::error!("{error:?}");
        }
        cx.notify();
    }

    pub(crate) fn action_step_forward_frame(
        &mut self,
        _: &StepForwardFrame,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.step_playhead(1) {
            log::error!("{error:?}");
        }
        cx.notify();
    }

    pub(crate) fn action_delete_selected(
        &mut self,
        _: &DeleteSelected,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(timeline) = self.timeline.as_ref() else {
            return;
        };
        if !timeline.selected_clips_editable() {
            return;
        }
        let clip_ids = timeline.interaction.selected_clip_ids.clone();
        let magnet_enabled = timeline.track_magnet_enabled;
        self.emit_event(
            cx,
            AppEvent::Edit(EditAction::RemoveClips {
                clip_ids,
                close_track_gaps: magnet_enabled,
            }),
        );
        self.property_panel_state.transform_input_clip_id = None;
        self.property_panel_state.text_input_clip_id = None;
        cx.notify();
    }

    pub(crate) fn action_split_clip(
        &mut self,
        _: &SplitClip,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(timeline) = self.timeline.as_mut() else {
            return;
        };
        let position = match timeline.interaction.active_tool {
            TimelineTool::Selection => timeline.playhead(),
            TimelineTool::Blade => match timeline.interaction.pointer_guide {
                Some(position) => position,
                None => return, // 指针尚未移到时间线上，pointer_guide 还没出现。
            },
        };
        if let Err(error) = timeline.blade_at(position) {
            log::error!("{error:?}");
        }
        cx.notify();
    }

    pub(crate) fn action_undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        self.emit_event(cx, AppEvent::Edit(EditAction::Undo));
        self.property_panel_state.transform_input_clip_id = None;
        self.property_panel_state.text_input_clip_id = None;
    }

    pub(crate) fn action_redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        self.emit_event(cx, AppEvent::Edit(EditAction::Redo));
        self.property_panel_state.transform_input_clip_id = None;
        self.property_panel_state.text_input_clip_id = None;
    }

    pub(crate) fn action_duplicate_selected(
        &mut self,
        _: &DuplicateSelected,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.duplicate_selected() {
            log::error!("{error:?}");
        }
        cx.notify();
    }

    pub(crate) fn action_copy_selected_clips(
        &mut self,
        _: &CopySelectedClips,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.copy_selected_clips();
        cx.notify();
    }

    pub(crate) fn action_cut_selected_clips(
        &mut self,
        _: &CutSelectedClips,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(timeline) = self.timeline.as_ref() else {
            return;
        };
        if !timeline.selected_clips_editable() {
            eprintln!("Cannot cut clips from a locked track.");
            return;
        }
        let Some(clipboard) = ClipClipboard::from_selection(
            timeline.path.clone(),
            &timeline.editing_state,
            &timeline.interaction.selected_clip_ids,
            timeline.interaction.selected_clip_id,
        ) else {
            return;
        };
        let clip_ids = timeline.interaction.selected_clip_ids.clone();
        self.clipboard = Some(clipboard);
        self.emit_event(
            cx,
            AppEvent::Edit(EditAction::RemoveClips {
                clip_ids,
                close_track_gaps: false,
            }),
        );
        self.property_panel_state.transform_input_clip_id = None;
        self.property_panel_state.text_input_clip_id = None;
        cx.notify();
    }

    pub(crate) fn action_paste_clips(
        &mut self,
        _: &PasteClips,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.paste_clips(cx) {
            log::error!("{error:?}");
        }
        cx.notify();
    }

    pub(crate) fn action_select_clips_right_of_pointer(
        &mut self,
        _: &SelectClipsRightOfPointer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(timeline) = self.timeline.as_mut() else {
            return;
        };
        let pointer = window.mouse_position();
        let bounds = timeline.h_scroll.bounds();
        if !bounds.contains(&pointer) {
            return;
        }
        let content_x = f32::from(pointer.x - bounds.left() - timeline.h_scroll.offset().x)
            - crate::layout::TIMELINE_PADDING;
        let cursor_seconds = content_x.max(0.0) as f64 / timeline.pixels_per_second as f64;
        timeline.interaction.selected_clip_ids = timeline
            .editing_state
            .clips
            .iter()
            .filter(|clip| {
                timeline
                    .editing_state
                    .seconds(clip.timeline_end(timeline.editing_state.settings.frame_rate))
                    > cursor_seconds
                    && timeline
                        .editing_state
                        .track(clip.track_id())
                        .is_some_and(|track| !track.locked)
            })
            .map(|clip| clip.id())
            .collect();
        timeline.interaction.selected_clip_id = timeline
            .editing_state
            .clips
            .iter()
            .filter(|clip| timeline.interaction.selected_clip_ids.contains(&clip.id()))
            .min_by_key(|clip| clip.timeline_start())
            .map(|clip| clip.id());
        self.property_panel_state.transform_input_clip_id = None;
        self.property_panel_state.text_input_clip_id = None;
        cx.notify();
    }

    pub(crate) fn action_select_clips_left_of_pointer(
        &mut self,
        _: &SelectClipsLeftOfPointer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(timeline) = self.timeline.as_mut() else {
            return;
        };
        let pointer = window.mouse_position();
        let bounds = timeline.h_scroll.bounds();
        if !bounds.contains(&pointer) {
            return;
        }
        let content_x = f32::from(pointer.x - bounds.left() - timeline.h_scroll.offset().x)
            - crate::layout::TIMELINE_PADDING;
        let cursor_seconds = content_x.max(0.0) as f64 / timeline.pixels_per_second as f64;
        timeline.interaction.selected_clip_ids = timeline
            .editing_state
            .clips
            .iter()
            .filter(|clip| {
                timeline.editing_state.seconds(clip.timeline_start()) < cursor_seconds
                    && timeline
                        .editing_state
                        .track(clip.track_id())
                        .is_some_and(|track| !track.locked)
            })
            .map(|clip| clip.id())
            .collect();
        timeline.interaction.selected_clip_id = timeline
            .editing_state
            .clips
            .iter()
            .filter(|clip| timeline.interaction.selected_clip_ids.contains(&clip.id()))
            .max_by_key(|clip| clip.timeline_start())
            .map(|clip| clip.id());
        self.property_panel_state.transform_input_clip_id = None;
        self.property_panel_state.text_input_clip_id = None;
        cx.notify();
    }

    pub(crate) fn action_select_all_unlocked_clips(
        &mut self,
        _: &SelectAllUnlockedClips,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_all_unlocked_clips();
        cx.notify();
    }

    pub(crate) fn action_activate_selection_tool(
        &mut self,
        _: &ActivateSelectionTool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(timeline) = self.timeline.as_mut() else {
            return;
        };
        timeline.activate_timeline_tool(TimelineTool::Selection);
        cx.notify();
    }

    pub(crate) fn action_activate_blade_tool(
        &mut self,
        _: &ActivateBladeTool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(timeline) = self.timeline.as_mut() else {
            return;
        };
        timeline.activate_timeline_tool(TimelineTool::Blade);
        cx.notify();
    }

    pub(crate) fn action_toggle_fullscreen(
        &mut self,
        _: &ToggleFullscreen,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.preview.fullscreen = !self.preview.fullscreen;
        cx.notify();
    }

    pub(crate) fn action_exit_fullscreen(
        &mut self,
        _: &ExitFullscreen,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.shortcuts_open {
            self.shortcuts_open = false;
            cx.notify();
            return;
        }
        if self.preview.fullscreen {
            self.preview.fullscreen = false;
            cx.notify();
        }
    }

    pub(crate) fn action_toggle_inspector(
        &mut self,
        _: &ToggleInspector,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.show_inspector && self.right_dock == RightDockTab::Inspector {
            self.show_inspector = false;
        } else {
            self.show_inspector = true;
            self.right_dock = RightDockTab::Inspector;
        }
        cx.notify();
    }

    pub(crate) fn action_toggle_assistant(
        &mut self,
        _: &ToggleAssistant,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.show_inspector && self.right_dock == RightDockTab::Assistant {
            self.show_inspector = false;
        } else {
            self.show_inspector = true;
            self.right_dock = RightDockTab::Assistant;
        }
        cx.notify();
    }

    pub(crate) fn action_switch_to_edit_workspace(
        &mut self,
        _: &SwitchToEditWorkspace,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace = WorkspaceTab::Edit;
        cx.notify();
    }

    pub(crate) fn action_switch_to_color_workspace(
        &mut self,
        _: &SwitchToColorWorkspace,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace = WorkspaceTab::Color;
        cx.notify();
    }

    pub(crate) fn action_switch_to_audio_workspace(
        &mut self,
        _: &SwitchToAudioWorkspace,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace = WorkspaceTab::Audio;
        cx.notify();
    }

    pub(crate) fn action_switch_to_aistudio_workspace(
        &mut self,
        _: &SwitchToAiStudioWorkspace,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace = WorkspaceTab::AiStudio;
        cx.notify();
    }

    pub(crate) fn action_toggle_media_browser(
        &mut self,
        _: &ToggleMediaBrowser,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_media_browser = !self.show_media_browser;
        cx.notify();
    }

    pub(crate) fn action_go_to_start(
        &mut self,
        _: &GoToStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(timeline) = self.timeline.as_mut() {
            timeline.set_playhead(::timeline::TimelineFrameIndex::ZERO);
            let _ = timeline.save();
            cx.notify();
        }
    }

    pub(crate) fn action_go_to_end(
        &mut self,
        _: &GoToEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(timeline) = self.timeline.as_mut() {
            let duration = timeline.editing_state.content_duration();
            timeline.set_playhead(duration);
            let _ = timeline.save();
            cx.notify();
        }
    }

    pub(crate) fn action_export_video(
        &mut self,
        _: &ExportVideoAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(timeline) = self.timeline.as_ref() else {
            return;
        };
        let timeline_path = timeline.path.clone();
        self.emit_event(cx, AppEvent::OpenExportWindow { timeline_path });
    }

    pub(crate) fn action_open_project_folder(
        &mut self,
        _: &OpenProjectFolderAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_project_folder(cx);
    }

    pub(crate) fn action_open_project_picker(
        &mut self,
        _: &OpenProjectPickerAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let event_bus = cx.new(|_| EventBus {});
        if let Err(error) = crate::project_picker::open(cx, event_bus, None) {
            log::error!("Could not open project picker: {error:?}");
        }
    }

    pub(crate) fn action_open_settings(
        &mut self,
        _: &OpenSettingsAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_global_settings(window, cx);
    }

    pub(crate) fn action_new_timeline(
        &mut self,
        _: &NewTimelineAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.begin_create_timeline(PathBuf::new(), window, cx);
    }

    pub(crate) fn action_import_media(
        &mut self,
        _: &ImportMedia,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.import_media_dialog(cx);
    }

    pub(crate) fn action_new_project(
        &mut self,
        _: &NewProjectAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.new_project_dialog(cx);
    }

    pub(crate) fn action_save_project(
        &mut self,
        _: &SaveProject,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        let Some(timeline) = self.timeline.as_ref() else {
            return;
        };
        if let Err(error) = timeline.save() {
            log::error!("Could not save timeline: {error:?}");
        }
    }

    pub(crate) fn action_close_window(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.remove_window();
    }

    pub(crate) fn action_minimize_window(
        &mut self,
        _: &MinimizeWindow,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.minimize_window();
    }

    pub(crate) fn action_zoom_window(
        &mut self,
        _: &ZoomWindow,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.zoom_window();
    }

    pub(crate) fn action_bring_all_to_front(
        &mut self,
        _: &BringAllToFront,
        _: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        #[cfg(target_os = "macos")]
        {
            let Some(main_thread) = objc2_foundation::MainThreadMarker::new() else {
                return;
            };
            let application = objc2_app_kit::NSApplication::sharedApplication(main_thread);
            application.arrangeInFront(None);
        }
        #[cfg(not(target_os = "macos"))]
        {
            for handle in _cx.windows() {
                let _ = handle.update(_cx, |_, window, _| window.activate_window());
            }
        }
    }

    pub(crate) fn action_show_keyboard_shortcuts(
        &mut self,
        _: &ShowKeyboardShortcuts,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shortcuts_open = !self.shortcuts_open;
        cx.notify();
    }

    pub(crate) fn action_show_about(
        &mut self,
        _: &ShowAbout,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        #[cfg(target_os = "macos")]
        {
            let Some(main_thread) = objc2_foundation::MainThreadMarker::new() else {
                return;
            };
            let application = objc2_app_kit::NSApplication::sharedApplication(main_thread);
            application.orderFrontStandardAboutPanel(None);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = _window.prompt(
                gpui::PromptLevel::Info,
                "NodaVideo",
                Some("Your timeline. Your creativity. Your AI."),
                &["OK"],
                _cx,
            );
        }
    }

    pub(crate) fn action_mark_source_in(
        &mut self,
        _: &MarkSourceIn,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.emit_event(cx, AppEvent::Preview(PreviewEvent::MarkSourceIn));
    }

    pub(crate) fn action_mark_source_out(
        &mut self,
        _: &MarkSourceOut,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.emit_event(cx, AppEvent::Preview(PreviewEvent::MarkSourceOut));
    }

    pub(crate) fn action_insert_source_to_timeline(
        &mut self,
        _: &InsertSourceToTimeline,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.emit_event(cx, AppEvent::Preview(PreviewEvent::InsertSourceToTimeline));
    }
}
