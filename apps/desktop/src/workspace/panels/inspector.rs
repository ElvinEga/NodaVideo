//! Inspector panel for clip attributes, transforms, effects, and audio settings.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Theme;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use nodavideo_project::{FrameRate, MediaAsset, MediaAssetStatus};
use nodavideo_timeline::{Clip, TextAlignment, Transition};

/// Inspector view component.
#[derive(Debug, Default)]
pub struct InspectorPanel {
    pub selected_asset: Option<MediaAsset>,
    pub selected_clip: Option<Clip>,
    pub selected_transition: Option<Transition>,
}

impl InspectorPanel {
    pub fn new() -> Self {
        Self {
            selected_asset: None,
            selected_clip: None,
            selected_transition: None,
        }
    }

    pub fn set_selected_asset(&mut self, asset: Option<MediaAsset>, cx: &mut Context<Self>) {
        self.selected_asset = asset;
        self.selected_clip = None;
        self.selected_transition = None;
        cx.notify();
    }

    pub fn set_selected_clip(&mut self, clip: Option<Clip>, cx: &mut Context<Self>) {
        self.selected_clip = clip;
        self.selected_asset = None;
        self.selected_transition = None;
        cx.notify();
    }

    pub fn set_selected_transition(&mut self, trans: Option<Transition>, cx: &mut Context<Self>) {
        self.selected_transition = trans;
        self.selected_clip = None;
        self.selected_asset = None;
        cx.notify();
    }

    pub fn clear_timeline_selection(&mut self, cx: &mut Context<Self>) {
        self.selected_clip = None;
        self.selected_transition = None;
        cx.notify();
    }
}

impl Render for InspectorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let (header_title, header_subtitle) = if self.selected_clip.is_some() {
            ("Clip Inspector", "Clip Properties")
        } else if self.selected_transition.is_some() {
            ("Transition Inspector", "Transition Properties")
        } else if self.selected_asset.is_some() {
            ("Media Inspector", "Asset Metadata")
        } else {
            ("Inspector", "Properties")
        };

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
                                    .child(header_title),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(header_subtitle),
                    ),
            )
            .child(
                // Contextual Body
                if let Some(ref clip) = self.selected_clip {
                    self.render_clip_properties(clip, &theme).into_any_element()
                } else if let Some(ref trans) = self.selected_transition {
                    self.render_transition_properties(trans, &theme)
                        .into_any_element()
                } else if let Some(ref asset) = self.selected_asset {
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

    fn render_clip_properties(&self, clip: &Clip, theme: &Theme) -> impl IntoElement {
        let mono_font = theme.mono_font_family.clone();
        let transform = clip.transform;
        let audio = clip.audio_params;

        let pos_str = format!(
            "{:.1} px / {:.1} px",
            transform.position_x, transform.position_y
        );
        let scale_str = format!(
            "{:.0}% / {:.0}%",
            transform.scale_x * 100.0,
            transform.scale_y * 100.0
        );
        let rot_str = format!("{:.1}°", transform.rotation_degrees);
        let opacity_str = format!("{:.0}%", transform.opacity * 100.0);
        let transform_status = if transform.is_identity() {
            "Identity (Default)"
        } else {
            "Modified"
        };

        let vol_str = format!("{:.1} dB", audio.volume_db);
        let pan_str = if audio.pan == 0.0 {
            "Center (0.0)".to_string()
        } else if audio.pan < 0.0 {
            format!("L {:.0}%", audio.pan.abs() * 100.0)
        } else {
            format!("R {:.0}%", audio.pan * 100.0)
        };
        let audio_status = if audio.is_default() {
            "Unity Gain (0.0 dB)"
        } else {
            "Adjusted"
        };

        let start_frame_str = clip.start_frame.to_string();
        let end_frame_str = clip.end_frame().to_string();
        let duration_str = format!("{} frames", clip.duration_frames());
        let source_range_str = format!("[{} .. {}]", clip.source_in, clip.source_out);

        div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .p_3()
            .gap_3()
            .child(
                // Clip Header Summary
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
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .truncate()
                                    .child(clip.name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(if clip.enabled {
                                        theme.accent
                                    } else {
                                        theme.muted
                                    })
                                    .text_color(theme.foreground)
                                    .child(if clip.enabled { "Active" } else { "Muted" }),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font)
                            .text_color(theme.muted_foreground)
                            .child(format!("Clip: {}", clip.id)),
                    ),
            )
            .child(self.render_section_preview(
                "Timeline Placement",
                &[
                    ("Start Frame", &start_frame_str),
                    ("End Frame", &end_frame_str),
                    ("Duration", &duration_str),
                    ("Source In/Out", &source_range_str),
                ],
                theme,
            ))
            .child(self.render_section_preview(
                "Video Transform",
                &[
                    ("Position X / Y", &pos_str),
                    ("Scale X / Y", &scale_str),
                    ("Rotation", &rot_str),
                    ("Opacity", &opacity_str),
                    ("Transform Status", transform_status),
                ],
                theme,
            ))
            .child(self.render_section_preview(
                "Audio Parameters",
                &[
                    ("Volume Gain", &vol_str),
                    ("Stereo Pan", &pan_str),
                    ("Channels", "Stereo (L/R)"),
                    ("Audio Status", audio_status),
                ],
                theme,
            ))
            .when_some(clip.text_overlay.as_ref(), |this, overlay| {
                let font_spec = format!(
                    "{} ({}px)",
                    overlay.style.font_family, overlay.style.font_size
                );
                let align_str = match overlay.style.alignment {
                    TextAlignment::Top => "Top",
                    TextAlignment::Center => "Center",
                    TextAlignment::Bottom => "Bottom",
                };
                let bg_color_str = overlay
                    .style
                    .background_color
                    .as_deref()
                    .unwrap_or("None (Transparent)");
                let offset_str = format!("X: {}px, Y: {}px", overlay.offset_x, overlay.offset_y);
                this.child(self.render_section_preview(
                    "Text & Subtitle Styling",
                    &[
                        ("Text Content", &overlay.text),
                        ("Font Spec", &font_spec),
                        ("Text Color", &overlay.style.color),
                        ("Background Box", bg_color_str),
                        ("Canvas Alignment", align_str),
                        ("Position Offsets", &offset_str),
                    ],
                    theme,
                ))
            })
    }

    fn render_transition_properties(&self, trans: &Transition, theme: &Theme) -> impl IntoElement {
        let mono_font = theme.mono_font_family.clone();
        let trans_type_str = format!("{:?}", trans.transition_type);
        let alignment_str = format!("{:?}", trans.alignment);
        let duration_str = format!("{} frames", trans.duration_frames);
        let cut_frame_str = trans.cut_frame.to_string();
        let range_str = format!("[{} .. {}]", trans.start_frame(), trans.end_frame());
        let track_str = trans.track_id.to_string();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .p_3()
            .gap_3()
            .child(
                // Transition Header Summary
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
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .truncate()
                                    .child(format!("{trans_type_str} Transition")),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_sm()
                                    .bg(theme.accent)
                                    .text_color(theme.foreground)
                                    .child("Active"),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family(mono_font)
                            .text_color(theme.muted_foreground)
                            .child(format!("Transition: {}", trans.id)),
                    ),
            )
            .child(self.render_section_preview(
                "Transition Configuration",
                &[
                    ("Transition Effect", &trans_type_str),
                    ("Cut Alignment", &alignment_str),
                    ("Duration", &duration_str),
                    ("Edit Cut Frame", &cut_frame_str),
                    ("Timeline Range", &range_str),
                    ("Track ID", &track_str),
                ],
                theme,
            ))
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
