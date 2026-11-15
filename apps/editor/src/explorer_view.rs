use crate::context_menu::ContextMenu;
use crate::editor::Editor;
use crate::event_bus::AppEvent;
use crate::explorer::{MediaCategoryFilter, MediaViewMode, explorer_file_badge};
use crate::explorer_drag::{AssetBeingDragged, DraggedTitle, ExplorerDragView};
use crate::explorer_file_entry::{FileTreeEntry, FileTreeEntryKind};
use crate::preview_events::PreviewEvent;
use crate::preview_text::title_preview_view;
use crate::theme::{
    ACCENT, ACCENT_HOVER, BORDER, CLIP_TEXT, MUTED, PANEL, SURFACE, SURFACE_HOVER, TEXT, WARNING,
};
use crate::title::TextTitle;
use gpui::prelude::FluentBuilder;
use gpui::{
    AppContext, Context, CursorStyle, ExternalPaths, FontWeight, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, StyledImage,
    div, px, rgb,
};
use nodavideo_hyperframes_bridge::{CompositionAsset, TemplateDefinition};
use std::path::PathBuf;
use std::time::Duration;

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

        // 标题是项目内数据，不是文件；All 与 Text 分类展示，可按名称或正文搜索。
        let show_titles = matches!(
            self.explorer.category,
            MediaCategoryFilter::All | MediaCategoryFilter::Text
        );
        let visible_titles: Vec<TextTitle> = if show_titles {
            self.explorer
                .titles
                .iter()
                .filter(|title| {
                    filter.is_empty()
                        || title.name.to_lowercase().contains(&filter)
                        || title.properties.text.to_lowercase().contains(&filter)
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        // 合成是项目内模板实例，不是文件；All 与 Motion 分类展示，可按名称搜索。
        let show_compositions = matches!(
            self.explorer.category,
            MediaCategoryFilter::All | MediaCategoryFilter::Motion
        );
        let visible_compositions: Vec<CompositionAsset> = if show_compositions {
            self.explorer
                .compositions
                .iter()
                .filter(|asset| filter.is_empty() || asset.name.to_lowercase().contains(&filter))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        // Compute counts per category
        let mut counts = CategoryCounts {
            all: 0,
            video: 0,
            audio: 0,
            image: 0,
            captions: 0,
            text: 0,
            motion: 0,
        };
        for entry in &visible_entries {
            match entry.kind {
                FileTreeEntryKind::Video => counts.video += 1,
                FileTreeEntryKind::Audio => counts.audio += 1,
                FileTreeEntryKind::Image => counts.image += 1,
                FileTreeEntryKind::Srt => counts.captions += 1,
                _ => {}
            }
        }
        counts.text = visible_titles.len();
        counts.motion = visible_compositions.len();
        counts.all = visible_entries.len() + counts.text + counts.motion;

        let filtered_entries: Vec<(usize, &FileTreeEntry)> = visible_entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| match self.explorer.category {
                MediaCategoryFilter::All => true,
                MediaCategoryFilter::Video => matches!(entry.kind, FileTreeEntryKind::Video),
                MediaCategoryFilter::Audio => matches!(entry.kind, FileTreeEntryKind::Audio),
                MediaCategoryFilter::Image => matches!(entry.kind, FileTreeEntryKind::Image),
                MediaCategoryFilter::Captions => matches!(entry.kind, FileTreeEntryKind::Srt),
                MediaCategoryFilter::Text => false, // 标题单独渲染，不是文件条目。
                MediaCategoryFilter::Motion => false, // 合成单独渲染，不是文件条目。
            })
            .collect();

        let import_error = self
            .explorer
            .import_errors
            .first()
            .cloned()
            .or_else(|| self.explorer.media_errors.values().next().cloned());

        let header = self.media_browser_header(cx);
        let category_bar = self.media_category_bar(counts, cx);

        let content = if self.explorer.category == MediaCategoryFilter::Text
            && visible_titles.is_empty()
            && filter.is_empty()
        {
            self.text_empty_state(cx)
        } else if self.explorer.category == MediaCategoryFilter::Motion
            && visible_compositions.is_empty()
            && filter.is_empty()
        {
            self.motion_empty_state(cx)
        } else if filtered_entries.is_empty()
            && visible_titles.is_empty()
            && visible_compositions.is_empty()
        {
            self.media_empty_state(&filter_query, cx)
        } else if self.explorer.view_mode == MediaViewMode::Grid {
            self.media_grid_view(
                &filtered_entries,
                &visible_titles,
                &visible_compositions,
                cx,
            )
        } else {
            self.media_list_view(
                &filtered_entries,
                show_root_contents,
                &visible_titles,
                &visible_compositions,
                cx,
            )
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

    fn media_browser_header(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let is_list = self.explorer.view_mode == MediaViewMode::List;
        let is_grid = self.explorer.view_mode == MediaViewMode::Grid;

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
                div().flex().items_center().child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(TEXT))
                        .child("MEDIA"),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
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
                    )
                    // + Text Button: create a reusable pool title
                    .child(
                        div()
                            .id("new-text-button")
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
                            .hover(|s| s.bg(rgb(SURFACE_HOVER)).border_color(rgb(CLIP_TEXT)))
                            .active(|s| s.bg(rgb(CLIP_TEXT)).text_color(rgb(0xe3d4ff)))
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT))
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.emit_event(cx, AppEvent::NewTextTitle);
                                cx.notify();
                            }))
                            .child("+ Text"),
                    )
                    // + Motion Button: open the template gallery to create a composition
                    .child(
                        div()
                            .id("new-motion-button")
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
                            .hover(|s| s.bg(rgb(SURFACE_HOVER)).border_color(rgb(WARNING)))
                            .active(|s| s.bg(rgb(WARNING)).text_color(rgb(0x131417)))
                            .cursor_pointer()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT))
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.emit_event(cx, AppEvent::OpenTemplateGallery);
                                cx.notify();
                            }))
                            .child("+ Motion"),
                    ),
            )
            .into_any_element()
    }

    fn media_category_bar(
        &self,
        counts: CategoryCounts,
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
                counts.all,
                current == MediaCategoryFilter::All,
                MediaCategoryFilter::All,
                cx,
            ))
            .child(self.category_chip(
                "cat-video",
                "Video",
                counts.video,
                current == MediaCategoryFilter::Video,
                MediaCategoryFilter::Video,
                cx,
            ))
            .child(self.category_chip(
                "cat-audio",
                "Audio",
                counts.audio,
                current == MediaCategoryFilter::Audio,
                MediaCategoryFilter::Audio,
                cx,
            ))
            .child(self.category_chip(
                "cat-text",
                "Text",
                counts.text,
                current == MediaCategoryFilter::Text,
                MediaCategoryFilter::Text,
                cx,
            ))
            .child(self.category_chip(
                "cat-motion",
                "Motion",
                counts.motion,
                current == MediaCategoryFilter::Motion,
                MediaCategoryFilter::Motion,
                cx,
            ))
            .child(self.category_chip(
                "cat-image",
                "Image",
                counts.image,
                current == MediaCategoryFilter::Image,
                MediaCategoryFilter::Image,
                cx,
            ))
            .child(self.category_chip(
                "cat-captions",
                "Captions",
                counts.captions,
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
                editor.schedule_composition_thumbnails(cx);
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
        titles: &[TextTitle],
        compositions: &[CompositionAsset],
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

        let title_rows = titles
            .iter()
            .map(|title| self.media_list_title_row(title, cx))
            .collect::<Vec<_>>();

        let composition_rows = compositions
            .iter()
            .map(|asset| self.media_list_composition_row(asset, cx))
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
            // Text 与 Motion 分类只列项目内资产，不展示项目文件夹树。
            .when(
                !matches!(
                    self.explorer.category,
                    MediaCategoryFilter::Text | MediaCategoryFilter::Motion
                ),
                |this| this.child(root_row),
            )
            .children(entry_elements)
            .children(title_rows)
            .children(composition_rows)
            .into_any_element()
    }

    fn media_grid_view(
        &self,
        filtered_entries: &[(usize, &FileTreeEntry)],
        titles: &[TextTitle],
        compositions: &[CompositionAsset],
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mut cards = filtered_entries
            .iter()
            .map(|(index, entry)| self.media_grid_card(*index, entry, cx))
            .collect::<Vec<_>>();
        cards.extend(
            titles
                .iter()
                .map(|title| self.media_grid_title_card(title, cx)),
        );
        cards.extend(
            compositions
                .iter()
                .map(|asset| self.media_grid_composition_card(asset, cx)),
        );

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
        let path =
            crate::timeline_document::relative_asset_path(&self.project_root, &entry.absolute_path);

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
                            let _ = editor
                                .explorer
                                .toggle_directory(&project_root, path.clone());
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

    fn media_grid_title_card(&self, title: &TextTitle, cx: &mut Context<Self>) -> gpui::AnyElement {
        let title_id = title.id;
        let selected = self.explorer.selected_title == Some(title_id);
        let duration = format_title_duration(title.duration);
        let event_bus = self.event_bus.clone();
        let drag_editor = cx.entity().downgrade();
        div()
            .id(gpui::SharedString::from(format!("title-card-{title_id}")))
            .w(px(136.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .p_1p5()
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(if selected {
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
            .cursor(CursorStyle::OpenHand)
            .on_drag(title.clone(), move |title, _, _, app_cx| {
                let asset = AssetBeingDragged::Title(DraggedTitle {
                    title_id: title.id,
                    name: title.name.clone(),
                    duration: title.duration,
                });
                event_bus.update(app_cx, |_, cx| {
                    cx.emit(AppEvent::DragStarted(asset.clone()));
                });
                app_cx.new(|_| ExplorerDragView {
                    asset,
                    editor: drag_editor.clone(),
                })
            })
            .on_click(cx.listener(move |editor, _, _, cx| {
                editor.emit_event(cx, AppEvent::SelectTitle { title_id });
                cx.notify();
            }))
            // Rendered preview: the title itself over black, no decoded thumbnail.
            .child(
                div()
                    .w_full()
                    .h(px(76.0))
                    .rounded(px(4.0))
                    .relative()
                    .overflow_hidden()
                    .child(title_preview_view(&title.properties))
                    .child(
                        div()
                            .absolute()
                            .top(px(4.0))
                            .left(px(4.0))
                            .size(px(16.0))
                            .rounded(px(2.0))
                            .bg(rgb(CLIP_TEXT))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(9.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xe3d4ff))
                            .child("T"),
                    )
                    .child(
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
                    ),
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_ellipsis()
                    .text_color(rgb(TEXT))
                    .child(title.name.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_1()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(10.0))
                            .text_color(rgb(MUTED))
                            .text_ellipsis()
                            .child(format!("Text · {}", title.properties.font)),
                    )
                    .child(
                        div()
                            .size(px(10.0))
                            .flex_shrink_0()
                            .rounded(px(2.0))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(title.properties.color & 0xFFFFFF)),
                    ),
            )
            .into_any_element()
    }

    fn media_list_title_row(&self, title: &TextTitle, cx: &mut Context<Self>) -> gpui::AnyElement {
        let title_id = title.id;
        let selected = self.explorer.selected_title == Some(title_id);
        let duration = format_title_duration(title.duration);
        let event_bus = self.event_bus.clone();
        let drag_editor = cx.entity().downgrade();
        div()
            .id(gpui::SharedString::from(format!("title-row-{title_id}")))
            .h(px(38.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .rounded(px(4.0))
            .when(selected, |this| this.bg(rgb(SURFACE_HOVER)))
            .hover(|s| s.bg(rgb(SURFACE_HOVER)))
            .cursor(CursorStyle::OpenHand)
            .on_drag(title.clone(), move |title, _, _, app_cx| {
                let asset = AssetBeingDragged::Title(DraggedTitle {
                    title_id: title.id,
                    name: title.name.clone(),
                    duration: title.duration,
                });
                event_bus.update(app_cx, |_, cx| {
                    cx.emit(AppEvent::DragStarted(asset.clone()));
                });
                app_cx.new(|_| ExplorerDragView {
                    asset,
                    editor: drag_editor.clone(),
                })
            })
            .on_click(cx.listener(move |editor, _, _, cx| {
                editor.emit_event(cx, AppEvent::SelectTitle { title_id });
                cx.notify();
            }))
            .child(
                div()
                    .w(px(56.0))
                    .h(px(32.0))
                    .flex_shrink_0()
                    .rounded(px(3.0))
                    .overflow_hidden()
                    .child(title_preview_view(&title.properties)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_ellipsis()
                            .text_color(rgb(TEXT))
                            .child(title.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(rgb(MUTED))
                            .text_ellipsis()
                            .child(format!("Text · {}", title.properties.font)),
                    ),
            )
            .child(
                div()
                    .size(px(10.0))
                    .flex_shrink_0()
                    .rounded(px(2.0))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(title.properties.color & 0xFFFFFF)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .font_family("monospace")
                    .text_size(px(10.0))
                    .text_color(rgb(MUTED))
                    .child(duration),
            )
            .into_any_element()
    }

    fn media_grid_composition_card(
        &self,
        asset: &CompositionAsset,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let composition_id = asset.id;
        let selected = self.explorer.selected_composition == Some(composition_id);
        let duration = format_title_duration(asset.fps.duration(asset.duration_frames.into()));
        let thumbnail = self
            .explorer
            .composition_thumbs
            .get(&composition_id)
            .cloned();
        let rendering = thumbnail.is_none();
        div()
            .id(gpui::SharedString::from(format!(
                "composition-card-{composition_id}"
            )))
            .w(px(136.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .p_1p5()
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(if selected {
                WARNING
            } else {
                BORDER
            }))
            .bg(rgb(if selected {
                SURFACE_HOVER
            } else {
                SURFACE
            }))
            .hover(|s| s.bg(rgb(SURFACE_HOVER)).border_color(rgb(WARNING)))
            .cursor_pointer()
            .on_click(cx.listener(move |editor, _, _, cx| {
                editor.emit_event(cx, AppEvent::SelectComposition { composition_id });
                cx.notify();
            }))
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
                    .when_some(thumbnail, |this, path| {
                        this.child(
                            gpui::img(path)
                                .size_full()
                                .object_fit(gpui::ObjectFit::Cover),
                        )
                    })
                    .when(rendering, |this| {
                        this.child(
                            div()
                                .text_size(px(10.0))
                                .text_color(rgb(MUTED))
                                .child("Rendering…"),
                        )
                    })
                    .child(
                        div()
                            .absolute()
                            .top(px(4.0))
                            .left(px(4.0))
                            .size(px(16.0))
                            .rounded(px(2.0))
                            .bg(rgb(WARNING))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(8.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x131417))
                            .child("MG"),
                    )
                    .child(
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
                    ),
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_ellipsis()
                    .text_color(rgb(TEXT))
                    .child(asset.name.clone()),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(MUTED))
                    .text_ellipsis()
                    .child(format!("Motion · {}×{}", asset.width, asset.height)),
            )
            .into_any_element()
    }

    fn media_list_composition_row(
        &self,
        asset: &CompositionAsset,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let composition_id = asset.id;
        let selected = self.explorer.selected_composition == Some(composition_id);
        let duration = format_title_duration(asset.fps.duration(asset.duration_frames.into()));
        let thumbnail = self
            .explorer
            .composition_thumbs
            .get(&composition_id)
            .cloned();
        let rendering = thumbnail.is_none();
        div()
            .id(gpui::SharedString::from(format!(
                "composition-row-{composition_id}"
            )))
            .h(px(38.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .rounded(px(4.0))
            .when(selected, |this| this.bg(rgb(SURFACE_HOVER)))
            .hover(|s| s.bg(rgb(SURFACE_HOVER)))
            .cursor_pointer()
            .on_click(cx.listener(move |editor, _, _, cx| {
                editor.emit_event(cx, AppEvent::SelectComposition { composition_id });
                cx.notify();
            }))
            .child(
                div()
                    .w(px(56.0))
                    .h(px(32.0))
                    .flex_shrink_0()
                    .rounded(px(3.0))
                    .bg(rgb(0x0e0f12))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .when_some(thumbnail, |this, path| {
                        this.child(
                            gpui::img(path)
                                .size_full()
                                .object_fit(gpui::ObjectFit::Cover),
                        )
                    })
                    .when(rendering, |this| {
                        this.child(
                            div()
                                .text_size(px(8.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(WARNING))
                                .child("MG"),
                        )
                    }),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_ellipsis()
                            .text_color(rgb(TEXT))
                            .child(asset.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(rgb(MUTED))
                            .text_ellipsis()
                            .child(format!("Motion · {}×{}", asset.width, asset.height)),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .font_family("monospace")
                    .text_size(px(10.0))
                    .text_color(rgb(MUTED))
                    .child(duration),
            )
            .into_any_element()
    }

    fn motion_empty_state(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
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
                    .size(px(56.0))
                    .rounded(px(10.0))
                    .bg(rgb(WARNING))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(20.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x131417))
                    .child("MG"),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT))
                    .child("Create your first motion graphic"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .max_w(px(240.0))
                    .child(
                        "Motion graphics are made from templates in your project. Pick one, then drag it to a video track.",
                    ),
            )
            .child(
                div()
                    .id("empty-new-motion-button")
                    .h(px(32.0))
                    .px_4()
                    .mt_2()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_1p5()
                    .rounded(px(6.0))
                    .bg(rgb(WARNING))
                    .hover(|s| s.bg(rgb(0xfbbf24)))
                    .cursor_pointer()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x131417))
                    .on_click(cx.listener(|editor, _, _, cx| {
                        editor.emit_event(cx, AppEvent::OpenTemplateGallery);
                        cx.notify();
                    }))
                    .child("+ New Motion Graphic"),
            )
            .into_any_element()
    }

    pub(super) fn template_gallery_dialog(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let gallery = self
            .explorer
            .template_gallery
            .as_ref()
            .expect("template gallery rendered without state");
        let cards = gallery
            .templates
            .iter()
            .enumerate()
            .map(|(index, template)| {
                self.template_gallery_card(template, index, index == gallery.selected, cx)
            })
            .collect::<Vec<_>>();

        div()
            .id("template-gallery-overlay")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgba(0x00000088))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|editor, _, _, cx| {
                    editor.emit_event(cx, AppEvent::CloseTemplateGallery);
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("template-gallery-dialog")
                    .w(px(720.0))
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(PANEL))
                    .shadow_lg()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT))
                                    .child("New motion graphic"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child("Pick a template to add to your project."),
                            ),
                    )
                    .child(
                        div()
                            .id("template-gallery-grid")
                            .max_h(px(440.0))
                            .overflow_y_scroll()
                            .flex()
                            .flex_wrap()
                            .items_start()
                            .content_start()
                            .gap_3()
                            .children(cards),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                div()
                                    .id("gallery-cancel")
                                    .h(px(32.0))
                                    .px_4()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(rgb(BORDER))
                                    .bg(rgb(SURFACE))
                                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT))
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        editor.emit_event(cx, AppEvent::CloseTemplateGallery);
                                        cx.notify();
                                    }))
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("gallery-create")
                                    .h(px(32.0))
                                    .px_4()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.0))
                                    .bg(rgb(WARNING))
                                    .hover(|s| s.bg(rgb(0xfbbf24)))
                                    .cursor_pointer()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0x131417))
                                    .on_click(cx.listener(|editor, _, _, cx| {
                                        let Some(gallery) =
                                            editor.explorer.template_gallery.as_ref()
                                        else {
                                            return;
                                        };
                                        let Some(template) =
                                            gallery.templates.get(gallery.selected)
                                        else {
                                            return;
                                        };
                                        let template_id = template.id.clone();
                                        editor.emit_event(
                                            cx,
                                            AppEvent::CreateComposition { template_id },
                                        );
                                        cx.notify();
                                    }))
                                    .child("Create"),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn template_gallery_card(
        &self,
        template: &TemplateDefinition,
        index: usize,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let thumbnail = self.explorer.gallery_thumbs.get(&template.id).cloned();
        let rendering = thumbnail.is_none();
        div()
            .id(gpui::SharedString::from(format!("gallery-card-{index}")))
            .w(px(200.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1p5()
            .p_2()
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(if selected {
                WARNING
            } else {
                BORDER
            }))
            .bg(rgb(if selected {
                SURFACE_HOVER
            } else {
                SURFACE
            }))
            .hover(|s| s.bg(rgb(SURFACE_HOVER)).border_color(rgb(WARNING)))
            .cursor_pointer()
            .on_click(cx.listener(move |editor, _, _, cx| {
                editor.emit_event(cx, AppEvent::GalleryTemplateSelected { index });
                cx.notify();
            }))
            .child(
                div()
                    .w_full()
                    .h(px(96.0))
                    .rounded(px(4.0))
                    .bg(rgb(0x0e0f12))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .when_some(thumbnail, |this, path| {
                        this.child(
                            gpui::img(path)
                                .size_full()
                                .object_fit(gpui::ObjectFit::Cover),
                        )
                    })
                    .when(rendering, |this| {
                        this.child(
                            div()
                                .text_size(px(10.0))
                                .text_color(rgb(MUTED))
                                .child("Rendering…"),
                        )
                    }),
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_ellipsis()
                    .text_color(rgb(TEXT))
                    .child(template.name.clone()),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(MUTED))
                    .text_ellipsis()
                    .child(template.category.clone()),
            )
            .into_any_element()
    }

    fn text_empty_state(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
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
                    .size(px(56.0))
                    .rounded(px(10.0))
                    .bg(rgb(CLIP_TEXT))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(24.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xe3d4ff))
                    .child("T"),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT))
                    .child("Create your first title"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .max_w(px(240.0))
                    .child(
                        "Titles are made in your project, not imported. Add one, then drag it to a Text track.",
                    ),
            )
            .child(
                div()
                    .id("empty-new-text-button")
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
                        editor.emit_event(cx, AppEvent::NewTextTitle);
                        cx.notify();
                    }))
                    .child("+ New Text"),
            )
            .into_any_element()
    }
}

fn format_title_duration(duration: Duration) -> String {
    let tenths = (duration.as_secs_f64() * 10.0).round() as u64;
    format!(
        "{:02}:{:02}.{}",
        tenths / 600,
        (tenths / 10) % 60,
        tenths % 10
    )
}

#[derive(Clone, Copy)]
struct CategoryCounts {
    all: usize,
    video: usize,
    audio: usize,
    image: usize,
    captions: usize,
    text: usize,
    motion: usize,
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
