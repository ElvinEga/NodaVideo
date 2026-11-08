//! Inspector panel for clip attributes, transforms, effects, and audio settings.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Theme;
use gpui_kit::*;

use nodavideo_project::{FrameRate, MediaAsset, MediaAssetStatus};

/// Inspector view component.
#[derive(Debug, Default)]
pub struct InspectorPanel {
    pub selected_asset: Option<MediaAsset>,
}

impl InspectorPanel {
    pub fn new() -> Self {
        Self {
            selected_asset: None,
        }
    }

    pub fn set_selected_asset(&mut self, asset: Option<MediaAsset>, cx: &mut Context<Self>) {
        self.selected_asset = asset;
        cx.notify();
    }
}

impl Render for InspectorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .border_l_1()
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
                            .child(Icon::new(IconName::SlidersHorizontal))
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("Inspector"),
                            ),
                    )
                    .child(div().text_xs().text_color(theme.muted_foreground).child(
                        if self.selected_asset.is_some() {
                            "Asset Metadata"
                        } else {
                            "Properties"
                        },
                    )),
            )
            .child(
                // Contextual Body
                if let Some(ref asset) = self.selected_asset {
                    self.render_asset_properties(asset, &theme)
                        .into_any_element()
                } else {
                    self.render_empty_selection(&theme).into_any_element()
                },
            )
    }
}

impl InspectorPanel {
    fn render_empty_selection(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .p_4()
            .gap_4()
            .child(
                // Empty selection note
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .p_4()
                    .gap_2()
                    .rounded_md()
                    .bg(theme.muted)
                    .border_1()
                    .border_color(theme.border)
                    .child(Icon::new(IconName::SlidersHorizontal))
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(theme.foreground)
                            .child("No Clip Selected"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .text_center()
                            .child("Select a clip on the timeline or in the media bin to inspect its metadata, transform, audio, and effects."),
                    ),
            )
            .child(
                // Preview of structured sections
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(self.render_section_preview(
                        "Video Transform",
                        &[("Position X / Y", "0.0 / 0.0 px"), ("Scale", "100.0%"), ("Rotation", "0.0°"), ("Opacity", "100%")],
                        theme,
                    ))
                    .child(self.render_section_preview(
                        "Audio Parameters",
                        &[("Volume", "0.0 dB"), ("Pan", "Center (0)"), ("Channels", "Stereo (L/R)")],
                        theme,
                    ))
                    .child(self.render_section_preview(
                        "Color & Effects",
                        &[("Color Space", "Rec.709"), ("LUT", "None"), ("Effects Stack", "0 applied")],
                        theme,
                    )),
            )
    }

    fn render_asset_properties(&self, asset: &MediaAsset, theme: &Theme) -> impl IntoElement {
        let mono_font = theme.mono_font_family.clone();
        let duration_text = asset.duration.map_or_else(
            || "N/A".into(),
            |d| d.to_timecode(asset.frame_rate.unwrap_or(FrameRate::FPS_24)),
        );
        let resolution_text = asset.resolution_string().unwrap_or_else(|| "N/A".into());
        let fps_text = asset
            .frame_rate
            .map_or_else(|| "N/A".into(), |f| f.to_string());
        let video_codec = asset.video_codec.clone().unwrap_or_else(|| "None".into());
        let pixel_format = asset.pixel_format.clone().unwrap_or_else(|| "N/A".into());
        let audio_codec = asset.audio_codec.clone().unwrap_or_else(|| "None".into());
        let audio_sample_rate = asset
            .audio_sample_rate
            .map_or_else(|| "N/A".into(), |s| format!("{s} Hz"));
        let audio_channels = asset.audio_channels.map_or_else(
            || "N/A".into(),
            |c| match c {
                1 => "1 (Mono)".into(),
                2 => "2 (Stereo)".into(),
                n => format!("{n} channels"),
            },
        );
        let path_text = asset.file_path.to_string_lossy().to_string();

        let status_text = match asset.status {
            MediaAssetStatus::Ready => "Ready (Verified)",
            MediaAssetStatus::Missing => "Missing (File Not Found)",
            MediaAssetStatus::Failed(_) => "Failed Inspection",
        };

        div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .p_3()
            .gap_3()
            .child(
                // Asset Header Summary
                div()
                    .flex()
                    .flex_col()
                    .p_3()
                    .rounded_md()
                    .bg(theme.secondary)
                    .border_1()
                    .border_color(theme.border)
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .truncate()
                            .child(asset.original_filename.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font.clone())
                            .text_color(theme.muted_foreground)
                            .child(format!("ID: {}", asset.id)),
                    ),
            )
            .child(self.render_section_preview(
                "Format & Timing",
                &[
                    ("Status", status_text),
                    ("Duration", &duration_text),
                    ("Resolution", &resolution_text),
                    ("Frame Rate", &fps_text),
                ],
                theme,
            ))
            .child(self.render_section_preview(
                "Video Stream",
                &[
                    ("Video Codec", &video_codec),
                    ("Pixel Format", &pixel_format),
                ],
                theme,
            ))
            .child(self.render_section_preview(
                "Audio Stream",
                &[
                    ("Audio Codec", &audio_codec),
                    ("Sample Rate", &audio_sample_rate),
                    ("Channels", &audio_channels),
                ],
                theme,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .p_2p5()
                    .rounded_md()
                    .bg(theme.secondary)
                    .border_1()
                    .border_color(theme.border)
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(theme.muted_foreground)
                            .child("FILE LOCATION"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font)
                            .text_color(theme.foreground)
                            .child(path_text),
                    ),
            )
    }

    fn render_section_preview(
        &self,
        title: &'static str,
        fields: &[(&'static str, &str)],
        theme: &Theme,
    ) -> impl IntoElement {
        let mono_font = theme.mono_font_family.clone();

        div()
            .flex()
            .flex_col()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary)
            .p_3()
            .gap_1p5()
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(theme.muted_foreground)
                    .child(title.to_uppercase()),
            )
            .children(fields.iter().map(|(label, val)| {
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .py_0p5()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(*label),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font.clone())
                            .text_color(theme.foreground)
                            .child((*val).to_string()),
                    )
            }))
    }
}
