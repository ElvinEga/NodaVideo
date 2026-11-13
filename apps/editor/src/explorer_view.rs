use crate::context_menu::ContextMenu;
use crate::editor::Editor;
use crate::event_bus::AppEvent;
use crate::explorer::{MediaCategoryFilter, MediaViewMode, explorer_file_badge};
use crate::explorer_drag::{AssetBeingDragged, ExplorerDragView};
use crate::explorer_file_entry::{FileTreeEntry, FileTreeEntryKind};
use crate::preview_events::PreviewEvent;
use crate::theme::{
    ACCENT, ACCENT_HOVER, BORDER, MUTED, PANEL, SURFACE, SURFACE_HOVER, TEXT, WARNING,
};
use gpui::prelude::FluentBuilder;
use gpui::{
    AppContext, Context, CursorStyle, ExternalPaths, FontWeight, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, StyledImage,
    div, px, rgb,
};
use std::path::PathBuf;

impl Editor {
    pub(super) fn explorer_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let filter_query = self.explorer.filter.read(cx).text().to_string();
        let filter = filter_query.trim().to_lowercase();
        let show_root_contents = self.explorer.root_expanded || !filter.is_empty();

        let mut visible_entries: Vec<FileTreeEntry> = if filter.is_empty() {
            self.explorer.file_tree.clone()
        } else {
            self.explorer.search_results.clone()
        };
        for entry in &self.explorer.imported_media {
            if entry.absolute_path.starts_with(&self.project_root) {
                continue; // 项目内的导入文件已由文件树展示，避免重复。
            }
            if !filter.is_empty() && !entry.name.to_lowercase().contains(&filter) {
                continue;
            }
            visible_entries.push(entry.clone());
        }

        // Compute counts per category
        let mut count_all = 0;
        let mut count_video = 0;
        let mut count_audio = 0;
        let mut count_image = 0;
        let mut count_captions = 0;

        for entry in &visible_entries {
            count_all += 1;
            match entry.kind {
                FileTreeEntryKind::Video => count_video += 1,
                FileTreeEntryKind::Audio => count_audio += 1,
                FileTreeEntryKind::Image => count_image += 1,
                FileTreeEntryKind::Srt => count_captions += 1,
                _ => {}
            }
        }

        let filtered_entries: Vec<(usize, &FileTreeEntry)> = visible_entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| match self.explorer.category {
                MediaCategoryFilter::All => true,
                MediaCategoryFilter::Video => matches!(entry.kind, FileTreeEntryKind::Video),
                MediaCategoryFilter::Audio => matches!(entry.kind, FileTreeEntryKind::Audio),
                MediaCategoryFilter::Image => matches!(entry.kind, FileTreeEntryKind::Image),
                MediaCategoryFilter::Captions => matches!(entry.kind, FileTreeEntryKind::Srt),
            })
            .collect();

        let import_error = self
            .explorer
            .import_errors
            .first()
            .cloned()
            .or_else(|| self.explorer.media_errors.values().next().cloned());

        let header = self.media_browser_header(count_all, cx);
        let category_bar = self.media_category_bar(
            count_all,
            count_video,
            count_audio,
            count_image,
            count_captions,
            cx,
        );

        let content = if filtered_entries.is_empty() {
            self.media_empty_state(&filter_query, cx)
        } else if self.explorer.view_mode == MediaViewMode::Grid {
            self.media_grid_view(&filtered_entries, cx)
        } else {
            self.media_list_view(&filtered_entries, show_root_contents, cx)
        };

        div()
            .id("editor-media-panel")
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(PANEL))
            .on_drop(cx.listener(|editor, paths: &ExternalPaths, _, cx| {
                editor.import_external_paths(paths.paths(), cx);
            }))
            .child(header)
            .child(category_bar)
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(SURFACE))
                    .child(self.explorer.filter.clone().into_any_element()),
            )
            .when_some(import_error, |this, message| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .border_b_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(0x2a1a1c))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_size(px(10.0))
                                .text_color(rgb(0xfca5a5))
                                .child(message),
                        )
                        .child(
                            div()
                                .id("dismiss-import-error")
                                .cursor_pointer()
                                .text_size(px(10.0))
                                .text_color(rgb(MUTED))
                                .hover(|s| s.text_color(rgb(TEXT)))
                                .on_click(cx.listener(|editor, _, _, cx| {
                                    editor.explorer.import_errors.clear();
                                    editor.explorer.media_errors.clear();
                                    cx.notify();
                                }))
                                .child("Dismiss"),
                        ),
                )
            })
            .child(div().flex_1().min_h_0().overflow_hidden().child(content))
            .into_any_element()
    }

    fn media_browser_header(&self, item_count: usize, cx: &mut Context<Self>) -> gpui::AnyElement {
        let is_list = self.explorer.view_mode == MediaViewMode::List;
        let is_grid = self.explorer.view_mode == MediaViewMode::Grid;
        let importing = self.explorer.import_task.is_some();

        div()
            .h(px(38.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .border_b_1()
            .border_color(rgb(BORDER))
            .bg(rgb(PANEL))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(TEXT))
                            .child("MEDIA POOL"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .px_1p5()
                            .py_0p5()
                            .rounded(px(3.0))
                            .bg(rgb(SURFACE))
                            .text_color(rgb(MUTED))
                            .child(format!("{item_count} items")),
                    )
                    .when(importing, |this| {
                        this.child(
                            div()
                                .text_size(px(10.0))
                                .text_color(rgb(ACCENT))
                                .child("Importing…"),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    // View Mode Toggle (List / Grid)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .rounded(px(4.0))
                            .bg(rgb(SURFACE))
                            .p(px(2.0))
                            .gap_1()
                            .child(
                                div()
                                    .id("view-mode-list")
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(3.0))
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .cursor_pointer()
                                    .when(is_list, |this| {
                                        this.bg(rgb(SURFACE_HOVER)).text_color(rgb(TEXT))
                                    })
                                    .when(!is_list, |this| {
                                        this.text_color(rgb(MUTED))
                                            .hover(|s| s.text_color(rgb(TEXT)))
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        editor.explorer.view_mode = MediaViewMode::List;
                                        cx.notify();
                                    }))
                                    .child("List"),
                            )
                            .child(
                                div()
                                    .id("view-mode-grid")
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(3.0))
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .cursor_pointer()
                                    .when(is_grid, |this| {
                                        this.bg(rgb(SURFACE_HOVER)).text_color(rgb(TEXT))
                                    })
                                    .when(!is_grid, |this| {
                                        this.text_color(rgb(MUTED))
                                            .hover(|s| s.text_color(rgb(TEXT)))
                                    })
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        editor.explorer.view_mode = MediaViewMode::Grid;
                                        cx.notify();
                                    }))
                                    .child("Grid"),
                            ),
                    )
                    // + Import Button
                    .child(
                        div()
                            .id("import-media-button")
                            .h(px(26.0))
                            .px_2p5()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap_1()
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(SURFACE))
                            .hover(|s| s.bg(rgb(SURFACE_HOVER)).border_color(rgb(ACCENT)))
                            .active(|s| s.bg(rgb(ACCENT)).text_color(rgb(0x131417)))
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT))
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.import_media_dialog(cx);
                            }))
                            .child("+ Import"),
                    ),
            )
            .into_any_element()
    }

    fn media_category_bar(
        &self,
        count_all: usize,
        count_video: usize,
        count_audio: usize,
        count_image: usize,
        count_captions: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let current = self.explorer.category;
        div()
            .h(px(30.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .border_b_1()
            .border_color(rgb(BORDER))
            .bg(rgb(PANEL))
            .child(self.category_chip(
                "cat-all",
                "All",
                count_all,
                current == MediaCategoryFilter::All,
                MediaCategoryFilter::All,
                cx,
            ))
            .child(self.category_chip(
                "cat-video",
                "Video",
                count_video,
                current == MediaCategoryFilter::Video,
                MediaCategoryFilter::Video,
                cx,
            ))
            .child(self.category_chip(
                "cat-audio",
                "Audio",
                count_audio,
                current == MediaCategoryFilter::Audio,
                MediaCategoryFilter::Audio,
                cx,
            ))
            .child(self.category_chip(
                "cat-image",
                "Graphics",
                count_image,
                current == MediaCategoryFilter::Image,
                MediaCategoryFilter::Image,
                cx,
            ))
            .child(self.category_chip(
                "cat-captions",
                "Captions",
                count_captions,
                current == MediaCategoryFilter::Captions,
                MediaCategoryFilter::Captions,
                cx,
            ))
            .into_any_element()
    }

    fn category_chip(
        &self,
        id: &'static str,
        label: &'static str,
        count: usize,
        selected: bool,
        category: MediaCategoryFilter,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .id(id)
            .h(px(22.0))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .rounded(px(3.0))
            .cursor_pointer()
            .text_size(px(11.0))
            .font_weight(if selected {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::NORMAL
            })
            .when(selected, |this| {
                this.bg(rgb(0x23243a))
                    .border_1()
                    .border_color(rgb(ACCENT))
                    .text_color(rgb(TEXT))
            })
            .when(!selected, |this| {
                this.text_color(rgb(MUTED))
                    .hover(|s| s.bg(rgb(SURFACE)).text_color(rgb(TEXT)))
            })
            .on_click(cx.listener(move |editor, _, _, cx| {
                editor.explorer.category = category;
                cx.notify();
            }))
            .child(label)
            .when(count > 0, |this| {
                this.child(
                    div()
                        .text_size(px(9.0))
                        .opacity(0.8)
                        .child(format!("({count})")),
                )
            })
            .into_any_element()
    }

    fn media_empty_state(&self, filter_query: &str, cx: &mut Context<Self>) -> gpui::AnyElement {
        let is_searching = !filter_query.is_empty();
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p_6()
            .gap_3()
            .text_center()
            .child(
                div()
                    .w(px(56.0))
                    .h(px(56.0))
                    .rounded_full()
                    .bg(rgb(SURFACE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(24.0))
                    .text_color(rgb(MUTED))
                    .child(
                        gpui::svg()
                            .path(crate::asset::IconName::Folder.path())
                            .size(px(24.0))
                            .text_color(rgb(MUTED)),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT))
                    .child(if is_searching {
                        "No matching media found"
                    } else {
                        "Your Media Bin is empty"
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .max_w(px(240.0))
                    .child(if is_searching {
                        format!("No media assets match \"{filter_query}\".")
                    } else {
                        "Drop video, audio, or graphics files anywhere here, or import from disk."
                            .to_string()
                    }),
            )
            .when(!is_searching, |this| {
                this.child(
                    div()
                        .id("empty-import-button")
                        .h(px(32.0))
                        .px_4()
                        .mt_2()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap_1p5()
                        .rounded(px(6.0))
                        .bg(rgb(ACCENT))
                        .hover(|s| s.bg(rgb(ACCENT_HOVER)))
                        .cursor_pointer()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x131417))
                        .on_click(cx.listener(|editor, _, _, cx| {
                            editor.import_media_dialog(cx);
                        }))
                        .child("+ Import Media"),
                )
            })
            .into_any_element()
    }

    fn media_list_view(
        &self,
        filtered_entries: &[(usize, &FileTreeEntry)],
        show_root_contents: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let project_name = self
            .project_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.project_root.display().to_string());

        let root_context_path = PathBuf::new();
        let root_context_menu_target = matches!(
            &self.context_menu,
            ContextMenu::File(menu) if menu.relative_path == root_context_path
        );

        let root_row = div()
            .id("project-root")
            .h(px(34.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .when(root_context_menu_target, |this| this.bg(rgb(SURFACE_HOVER)))
            .cursor(CursorStyle::Arrow)
            .hover(|style| style.bg(rgb(SURFACE_HOVER)))
            .on_click(cx.listener(|editor, _, _, cx| {
                editor.explorer.root_expanded = !editor.explorer.root_expanded;
                if let Err(error) = editor.save_explorer_expansion() {
                    eprintln!("Could not save explorer expansion: {error}");
                }
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |editor, event: &MouseDownEvent, _, cx| {
                    editor.show_file_context_menu(root_context_path.clone(), true, event, cx);
                }),
            )
            .child(
                div()
                    .w(px(14.0))
                    .flex_shrink_0()
                    .text_color(rgb(MUTED))
                    .child(if show_root_contents {
                        "▾"
                    } else {
                        "▸"
                    }),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .font_family("monospace")
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_ellipsis()
                    .child(project_name),
            );

        let entry_elements = filtered_entries
            .iter()
            .filter(|_| show_root_contents)
            .map(|(index, entry)| self.explorer_file_entry(*index, entry, cx))
            .collect::<Vec<_>>();

        div()
            .id("editor-media-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.explorer.scroll)
            .flex()
            .flex_col()
            .py_1()
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|editor, event: &MouseDownEvent, _, cx| {
                    editor.show_file_context_menu(PathBuf::new(), true, event, cx);
                }),
            )
            .child(root_row)
            .children(entry_elements)
            .into_any_element()
    }

    fn media_grid_view(
        &self,
        filtered_entries: &[(usize, &FileTreeEntry)],
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let cards = filtered_entries
            .iter()
            .map(|(index, entry)| self.media_grid_card(*index, entry, cx))
            .collect::<Vec<_>>();

        div()
            .id("editor-media-grid-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.explorer.scroll)
            .p_2()
            .flex()
            .flex_wrap()
            .items_start()
            .content_start()
            .gap_2()
            .children(cards)
            .into_any_element()
    }

    fn media_grid_card(
        &self,
        index: usize,
        entry: &FileTreeEntry,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let path = crate::timeline_document::relative_asset_path(
            &self.project_root,
            &entry.absolute_path,
        );

        let selected = self.explorer.selected_file.as_ref() == Some(&path);
        let active_timeline = matches!(entry.kind, FileTreeEntryKind::Timeline)
            && self
                .timeline
                .as_ref()
                .is_some_and(|timeline| timeline.path == entry.absolute_path);

        let file_name = entry
            .absolute_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unknown".to_string());

        let draggable = matches!(
            entry.kind,
            FileTreeEntryKind::Video
                | FileTreeEntryKind::Image
                | FileTreeEntryKind::Audio
                | FileTreeEntryKind::Srt
        );

        let event_bus = self.event_bus.clone();
        let drag_editor = cx.entity().downgrade();

        let is_missing = self.explorer.missing_media.contains(&entry.absolute_path);
        let metadata_text = match self.explorer.media_metadata.get(&entry.absolute_path) {
            Some(asset) => crate::explorer_file_entry::media_asset_summary(asset),
            None => format_file_size(entry.size_bytes.unwrap_or(0)),
        };

        let video_thumbnail = match entry.kind {
            FileTreeEntryKind::Video => {
                if let Some(cached) = self.explorer.thumbnail_cache.get(&entry.absolute_path) {
                    Some(cached.clone())
                } else if let Ok(cached_path) =
                    crate::media_thumbnail::thumbnail_cache_path(&entry.absolute_path)
                {
                    if cached_path.is_file() {
                        Some(cached_path)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        let duration_badge = self
            .explorer
            .media_metadata
            .get(&entry.absolute_path)
            .and_then(|a| a.duration())
            .map(format_card_duration);

        div()
            .id(("media-card", index))
            .w(px(136.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .p_1p5()
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(if active_timeline || selected {
                ACCENT
            } else {
                BORDER
            }))
            .bg(rgb(if selected {
                SURFACE_HOVER
            } else {
                SURFACE
            }))
            .hover(|s| s.bg(rgb(SURFACE_HOVER)).border_color(rgb(ACCENT)))
            .cursor(if draggable {
                CursorStyle::OpenHand
            } else {
                CursorStyle::PointingHand
            })
            // Drag support from playlist to timeline!
            .when(draggable, |this| {
                this.on_drag(entry.clone(), move |entry, _, _, app_cx| {
                    let asset = match AssetBeingDragged::from_file_entry(entry) {
                        Ok(asset) => asset,
                        Err(error) => {
                            log::error!(
                                "could not drag {}: {error:?}",
                                entry.absolute_path.display()
                            );
                            return app_cx.new(|_| ExplorerDragView {
                                asset: AssetBeingDragged::None,
                                editor: drag_editor.clone(),
                            });
                        }
                    };
                    event_bus.update(app_cx, |_, cx| {
                        cx.emit(AppEvent::DragStarted(asset.clone()));
                    });
                    app_cx.new(|_| ExplorerDragView {
                        asset,
                        editor: drag_editor.clone(),
                    })
                })
            })
            // Click to preview & select
            .on_click(cx.listener({
                let entry = entry.clone();
                let path = path.clone();
                move |editor, _, _, cx| {
                    match entry.kind {
                        FileTreeEntryKind::Directory { .. } => {
                            let project_root = editor.project_root.clone();
                            let _ = editor.explorer.toggle_directory(&project_root, path.clone());
                        }
                        FileTreeEntryKind::Timeline => {
                            editor.emit_event(cx, AppEvent::OpenTimeline { path: path.clone() });
                            editor.emit_event(cx, AppEvent::RefreshPropertyPanel);
                        }
                        FileTreeEntryKind::Video => {
                            editor.emit_event(
                                cx,
                                AppEvent::Preview(PreviewEvent::SelectFile(path.clone())),
                            );
                            editor.emit_event(
                                cx,
                                AppEvent::VideoFileSelected {
                                    path: path.clone(),
                                    size_bytes: entry.size_bytes,
                                },
                            );
                        }
                        FileTreeEntryKind::Srt => {
                            editor.emit_event(
                                cx,
                                AppEvent::Preview(PreviewEvent::SelectFile(path.clone())),
                            );
                            editor.emit_event(cx, AppEvent::SrtFileSelected { path: path.clone() });
                        }
                        FileTreeEntryKind::Image
                        | FileTreeEntryKind::Audio
                        | FileTreeEntryKind::Other => {
                            editor.emit_event(
                                cx,
                                AppEvent::Preview(PreviewEvent::SelectFile(path.clone())),
                            );
                            editor.emit_event(cx, AppEvent::RefreshPropertyPanel);
                        }
                    }
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener({
                    let entry = entry.clone();
                    let path = path.clone();
                    move |editor, event: &MouseDownEvent, _, cx| {
                        editor.show_file_context_menu(
                            path.clone(),
                            matches!(entry.kind, FileTreeEntryKind::Directory { .. }),
                            event,
                            cx,
                        );
                    }
                }),
            )
            // Aspect box preview
            .child(
                div()
                    .w_full()
                    .h(px(76.0))
                    .rounded(px(4.0))
                    .bg(rgb(0x0e0f12))
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .when(entry.kind == FileTreeEntryKind::Image, |this| {
                        this.child(
                            gpui::img(entry.absolute_path.clone())
                                .size_full()
                                .object_fit(gpui::ObjectFit::Cover),
                        )
                    })
                    .when_some(video_thumbnail, |this, thumbnail| {
                        this.child(
                            gpui::img(thumbnail)
                                .size_full()
                                .object_fit(gpui::ObjectFit::Cover),
                        )
                    })
                    .child(
                        div()
                            .absolute()
                            .top(px(4.0))
                            .left(px(4.0))
                            .child(explorer_file_badge(entry)),
                    )
                    .when_some(duration_badge, |this, duration| {
                        this.child(
                            div()
                                .absolute()
                                .bottom(px(4.0))
                                .right(px(4.0))
                                .px(px(4.0))
                                .py(px(1.0))
                                .rounded(px(3.0))
                                .bg(gpui::rgba(0x000000cc))
                                .font_family("monospace")
                                .text_size(px(9.0))
                                .text_color(rgb(0xd4d4d8))
                                .child(duration),
                        )
                    })
                    .when(is_missing, |this| {
                        this.child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(gpui::rgba(0x131417cc))
                                .text_size(px(10.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(WARNING))
                                .child("MISSING"),
                        )
                    }),
            )
            // Title and metadata
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_ellipsis()
                    .text_color(rgb(TEXT))
                    .child(file_name),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(MUTED))
                    .text_ellipsis()
                    .child(metadata_text),
            )
            .into_any_element()
    }
}

fn format_card_duration(duration: f64) -> String {
    let total_seconds = duration.round() as u64;
    if total_seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            total_seconds / 3600,
            (total_seconds % 3600) / 60,
            total_seconds % 60
        )
    } else {
        format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
    }
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
