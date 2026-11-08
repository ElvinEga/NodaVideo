//! Media Browser panel for asset management, import, search, and filtering.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Icon, Sizable, Theme};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use nodavideo_media_engine::MediaProcessor;
use nodavideo_project::{
    AssetId, FrameRate, MediaAsset, MediaAssetStatus, MediaType, ProjectDocument,
};

/// Media category filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaCategory {
    #[default]
    All,
    Video,
    Audio,
    Graphics,
}

/// Format file size into human-readable representation.
fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Determine default platform cache directory for media waveforms and thumbnails.
pub fn default_cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("nodavideo")
}

/// Media Browser panel view state and elements.
#[derive(Debug)]
pub struct MediaBrowserPanel {
    pub search_input: Entity<InputState>,
    _search_sub: Subscription,
    pub selected_category: MediaCategory,
    pub project: ProjectDocument,
    pub processor: Arc<MediaProcessor>,
    pub is_importing: bool,
    pub import_status_text: Option<SharedString>,
    pub last_error: Option<SharedString>,
    pub selected_asset_id: Option<AssetId>,
    pub cache_dir: PathBuf,
}

impl MediaBrowserPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search media bin…"));

        let search_sub = cx.subscribe(&search_input, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });

        let cache_dir = default_cache_dir();
        let processor = Arc::new(
            MediaProcessor::new(&cache_dir)
                .expect("Failed to initialize NodaVideo media processor cache"),
        );

        Self {
            search_input,
            _search_sub: search_sub,
            selected_category: MediaCategory::All,
            project: ProjectDocument::default(),
            processor,
            is_importing: false,
            import_status_text: None,
            last_error: None,
            selected_asset_id: None,
            cache_dir,
        }
    }

    pub fn set_category(&mut self, category: MediaCategory, cx: &mut Context<Self>) {
        self.selected_category = category;
        cx.notify();
    }

    pub fn select_asset(&mut self, id: Option<AssetId>, cx: &mut Context<Self>) {
        self.selected_asset_id = id;
        cx.notify();
    }

    /// Open native file picker to select and import media files.
    pub fn prompt_import(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: None,
        });

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                this.update(cx, |this, cx| {
                    this.import_paths(paths, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// Import media files from filesystem paths asynchronously off the main UI thread.
    pub fn import_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if paths.is_empty() {
            return;
        }

        self.is_importing = true;
        self.import_status_text = Some(format!("Importing {} items…", paths.len()).into());
        self.last_error = None;
        cx.notify();

        let processor = self.processor.clone();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let mut imported = Vec::new();
                    let mut errors = Vec::new();

                    for path in paths {
                        match processor.process_file(&path) {
                            Ok(asset) => imported.push(asset),
                            Err(err) => {
                                let name =
                                    path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                                errors.push(format!("{name}: {err}"));
                            }
                        }
                    }

                    (imported, errors)
                })
                .await;

            this.update(cx, |this, cx| {
                this.is_importing = false;
                this.import_status_text = None;

                let (imported_assets, errors) = result;

                for asset in imported_assets {
                    if let Err(err) = this.project.add_asset(asset) {
                        this.last_error = Some(err.to_string().into());
                    }
                }

                if !errors.is_empty() {
                    this.last_error = Some(errors.join(" | ").into());
                }

                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Clear the current error banner.
    pub fn dismiss_error(&mut self, cx: &mut Context<Self>) {
        self.last_error = None;
        cx.notify();
    }
}

impl Render for MediaBrowserPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let query = self.search_input.read(cx).value().trim().to_lowercase();
        let category = self.selected_category;

        // Filter project assets by search term and selected category
        let filtered_assets: Vec<&MediaAsset> = self
            .project
            .assets
            .iter()
            .filter(|asset| {
                let matches_search = if query.is_empty() {
                    true
                } else {
                    asset.original_filename.to_lowercase().contains(&query)
                };

                let matches_category = match category {
                    MediaCategory::All => true,
                    MediaCategory::Video => asset.media_type == MediaType::Video,
                    MediaCategory::Audio => asset.media_type == MediaType::Audio,
                    MediaCategory::Graphics => asset.media_type == MediaType::Image,
                };

                matches_search && matches_category
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .border_r_1()
            .border_color(theme.border)
            .child(
                // Panel Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h_9()
                    .px_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .bg(theme.secondary)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Icon::new(IconName::Folder))
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child(format!(
                                        "Media Browser ({})",
                                        self.project.assets.len()
                                    )),
                            ),
                    )
                    .child(
                        Button::new("import-btn")
                            .ghost()
                            .xsmall()
                            .label("Import")
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.prompt_import(cx);
                            })),
                    ),
            )
            .when_some(self.import_status_text.clone(), |this, status| {
                // Importing progress indicator banner
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_1p5()
                        .bg(theme.muted)
                        .border_b_1()
                        .border_color(theme.border)
                        .child(div().w_2().h_2().rounded_full().bg(theme.accent))
                        .child(
                            div()
                                .text_xs()
                                .font_medium()
                                .text_color(theme.foreground)
                                .child(status),
                        ),
                )
            })
            .when_some(self.last_error.clone(), |this, err| {
                // Error reporting banner
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py_1p5()
                        .bg(theme.danger.opacity(0.15))
                        .border_b_1()
                        .border_color(theme.danger)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(Icon::new(IconName::TriangleAlert))
                                .child(div().text_xs().text_color(theme.foreground).child(err)),
                        )
                        .child(
                            Button::new("dismiss-error")
                                .ghost()
                                .xsmall()
                                .icon(IconName::X)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.dismiss_error(cx);
                                })),
                        ),
                )
            })
            .child(
                // Search Field & Filter Controls
                div()
                    .flex()
                    .flex_col()
                    .p_2()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(Input::new(&self.search_input))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(self.render_filter_button("All", MediaCategory::All, cx))
                            .child(self.render_filter_button("Video", MediaCategory::Video, cx))
                            .child(self.render_filter_button("Audio", MediaCategory::Audio, cx))
                            .child(self.render_filter_button(
                                "Graphics",
                                MediaCategory::Graphics,
                                cx,
                            )),
                    ),
            )
            .child(
                // Media Asset List / Empty State with Drag-and-Drop support
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_hidden()
                    .p_2()
                    .gap_1p5()
                    .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                        this.import_paths(paths.0.to_vec(), cx);
                    }))
                    .when(filtered_assets.is_empty(), |this| {
                        this.child(self.render_empty_state(&theme, !self.project.assets.is_empty()))
                    })
                    .children(
                        filtered_assets
                            .into_iter()
                            .map(|asset| self.render_asset_card(asset, &theme, cx)),
                    ),
            )
    }
}

impl MediaBrowserPanel {
    fn render_filter_button(
        &self,
        label: &'static str,
        category: MediaCategory,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let is_selected = self.selected_category == category;

        Button::new(label)
            .xsmall()
            .label(label)
            .when(is_selected, |btn| btn.secondary())
            .when(!is_selected, |btn| btn.ghost())
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_category(category, cx);
            }))
    }

    fn render_empty_state(&self, theme: &Theme, has_unmatched: bool) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .p_4()
            .gap_2()
            .border_1()
            .border_dashed()
            .border_color(theme.border)
            .rounded_md()
            .child(
                div()
                    .p_2()
                    .rounded_full()
                    .bg(theme.muted)
                    .child(Icon::new(IconName::Folder)),
            )
            .child(
                div()
                    .text_xs()
                    .font_medium()
                    .text_color(theme.foreground)
                    .child(if has_unmatched {
                        "No Matching Assets"
                    } else {
                        "Media Bin Empty"
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .text_center()
                    .child(if has_unmatched {
                        "Try clearing your search query or switching categories."
                    } else {
                        "Click Import or drag video, audio, or graphics files here."
                    }),
            )
    }

    fn render_asset_card(
        &self,
        asset: &MediaAsset,
        theme: &Theme,
        cx: &Context<Self>,
    ) -> AnyElement {
        let asset_id = asset.id;
        let is_selected = self.selected_asset_id == Some(asset_id);
        let mono_font = theme.mono_font_family.clone();
        let thumb_path = self
            .cache_dir
            .join("thumbnails")
            .join(format!("{asset_id}.jpg"));
        let has_thumb = thumb_path.exists();

        let icon = match asset.media_type {
            MediaType::Video => IconName::Film,
            MediaType::Audio => IconName::Volume2,
            MediaType::Image => IconName::Image,
        };

        let duration_text = asset.duration.map_or_else(String::new, |d| {
            d.to_timecode(asset.frame_rate.unwrap_or(FrameRate::FPS_24))
        });

        let resolution_text = asset.resolution_string().unwrap_or_default();
        let size_text = format_file_size(asset.file_size);

        let is_missing = asset.status == MediaAssetStatus::Missing;

        div()
            .id(ElementId::Name(format!("asset-{asset_id}").into()))
            .flex()
            .items_center()
            .gap_2p5()
            .p_2()
            .rounded_md()
            .border_1()
            .cursor_pointer()
            .when(is_selected, |this| {
                this.bg(theme.secondary).border_color(theme.accent)
            })
            .when(!is_selected, |this| {
                this.bg(theme.background)
                    .border_color(theme.border)
                    .hover(|hover| hover.bg(theme.muted))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_asset(Some(asset_id), cx);
            }))
            .child(
                // Thumbnail or Type Icon preview box
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(56.0))
                    .h_10()
                    .rounded_sm()
                    .bg(theme.secondary)
                    .border_1()
                    .border_color(theme.border)
                    .overflow_hidden()
                    .when(has_thumb, |this| {
                        this.child(img(thumb_path).size_full().object_fit(ObjectFit::Cover))
                    })
                    .when(!has_thumb, |this| this.child(Icon::new(icon))),
            )
            .child(
                // Metadata and filenames
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_0p5()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(theme.foreground)
                            .truncate()
                            .child(asset.original_filename.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(!duration_text.is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .font_family(mono_font.clone())
                                        .text_color(theme.muted_foreground)
                                        .child(duration_text),
                                )
                            })
                            .when(!resolution_text.is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(resolution_text),
                                )
                            })
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(size_text),
                            ),
                    ),
            )
            .when(is_missing, |this| {
                // Missing asset warning badge
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_1p5()
                        .py_0p5()
                        .rounded_sm()
                        .bg(theme.danger.opacity(0.15))
                        .border_1()
                        .border_color(theme.danger)
                        .child(Icon::new(IconName::TriangleAlert))
                        .child(
                            div()
                                .text_xs()
                                .font_medium()
                                .text_color(theme.danger)
                                .child("Missing"),
                        ),
                )
            })
            .into_any_element()
    }
}
