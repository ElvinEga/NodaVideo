use crate::edit_action::EditAction;
use crate::event_bus::{AppEvent, EventBus};
use crate::property_panel::{
    clip_property_panel_frame_view, clip_property_panel_header_view, format_property_panel_time,
    property_panel_divider_view, property_panel_heading_view, readonly_property_panel_view,
};
use crate::theme::{ACCENT, BORDER, MUTED, SURFACE};
use crate::timeline::{FrameRate, TimelineFrameIndex};
use crate::timeline_clip::{TextClip, TextClipProperties};
use crate::title::TextTitle;
use crate::track::TrackKind;
use crate::ui_components::text_area::TextArea;
use crate::ui_components::text_input::{TextInput, TextInputEvent, disabled_text_input_view};
use anyhow::{Result, bail};
use gpui::prelude::*;
use gpui::{
    Anchor, App, BoxShadow, CursorStyle, Entity, FocusHandle, Focusable, FontWeight, MouseButton,
    PathBuilder, Window, anchored, canvas, deferred, div, point, px, rgb, rgba,
};
use std::time::Duration;
use timeline::{TextBackground, TimelineEditingState};
use ulid::Ulid;

/// Which text object a property edit applies to: a placed timeline clip or a
/// reusable pool title. Both share one panel; the target routes the edit.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TextEditTarget {
    Clip(Ulid),
    Title(Ulid),
}

impl TextEditTarget {
    fn key(self) -> String {
        match self {
            Self::Clip(id) => format!("clip-{id}"),
            Self::Title(id) => format!("title-{id}"),
        }
    }
}

/// A text property change independent of its target. The handler maps it to an
/// `EditAction` for clips or applies it to the title store for titles.
#[derive(Clone, Debug)]
pub(crate) enum TextPropertyChange {
    Content(String),
    Font(String),
    FontSize(f64),
    Color(u32),
    Background(Option<TextBackground>),
    BackgroundColor { red: u8, green: u8, blue: u8 },
    BackgroundAlpha(u8),
}

pub(crate) struct TextClipPanel {
    clip: TextClip,
    track_name: String,
    frame_rate: FrameRate,
    locked: bool, // 片段所在轨道已锁定时内容只读
    event_bus: Entity<EventBus>,
    return_focus: FocusHandle,
}

pub(crate) struct TextTitlePanel {
    title: TextTitle,
    frame_rate: FrameRate,
    event_bus: Entity<EventBus>,
    return_focus: FocusHandle,
}

pub(crate) fn text_clip_panel(
    editing_state: &TimelineEditingState,
    clip: &TextClip,
    event_bus: Entity<EventBus>,
    return_focus: FocusHandle,
) -> TextClipPanel {
    let track_name = match editing_state
        .tracks
        .iter()
        .find(|track| track.id == clip.track_id)
    {
        Some(track) => track.name.as_str(),
        None => "Unknown track",
    };
    TextClipPanel {
        clip: clip.clone(),
        track_name: track_name.to_owned(),
        frame_rate: editing_state.settings.frame_rate,
        locked: editing_state.clip_locked(clip.id()),
        event_bus,
        return_focus,
    }
}

pub(crate) fn text_title_panel(
    title: &TextTitle,
    frame_rate: FrameRate,
    event_bus: Entity<EventBus>,
    return_focus: FocusHandle,
) -> TextTitlePanel {
    TextTitlePanel {
        title: title.clone(),
        frame_rate,
        event_bus,
        return_focus,
    }
}

pub(crate) fn text_clip_view(panel: &TextClipPanel) -> gpui::AnyElement {
    let clip = &panel.clip;
    let title = text_panel_heading(&clip.properties.text, "Text clip");
    let header =
        clip_property_panel_header_view(TrackKind::Text, title, &panel.track_name, panel.locked);
    let context = TextEditContext {
        target: TextEditTarget::Clip(clip.id()),
        locked: panel.locked,
        event_bus: panel.event_bus.clone(),
        return_focus: panel.return_focus.clone(),
    };
    let content = text_properties_view(
        &clip.properties,
        &context,
        clip.timeline_start,
        clip.duration,
        panel.frame_rate,
    );
    clip_property_panel_frame_view(
        readonly_property_panel_view("text-clip-property-panel", header, content),
        panel.locked,
    )
}

pub(crate) fn text_title_view(panel: &TextTitlePanel) -> gpui::AnyElement {
    let title = &panel.title;
    let heading = text_panel_heading(&title.properties.text, "Title");
    let header = clip_property_panel_header_view(TrackKind::Text, heading, &title.name, false);
    let context = TextEditContext {
        target: TextEditTarget::Title(title.id),
        locked: false,
        event_bus: panel.event_bus.clone(),
        return_focus: panel.return_focus.clone(),
    };
    let content = text_properties_view(
        &title.properties,
        &context,
        TimelineFrameIndex::ZERO,
        title.duration,
        panel.frame_rate,
    );
    clip_property_panel_frame_view(
        readonly_property_panel_view("text-title-property-panel", header, content),
        false,
    )
}

fn text_panel_heading(text: &str, fallback: &str) -> String {
    match text.lines().find(|line| !line.trim().is_empty()) {
        Some(line) => line.to_owned(),
        None => fallback.to_owned(),
    }
}

/// The edit-routing context shared by every control in the text panel: where edits
/// go, whether they are allowed, and the handles used to emit and refocus.
#[derive(Clone)]
struct TextEditContext {
    target: TextEditTarget,
    locked: bool,
    event_bus: Entity<EventBus>,
    return_focus: FocusHandle,
}

fn text_properties_view(
    properties: &TextClipProperties,
    context: &TextEditContext,
    start: TimelineFrameIndex,
    duration: Duration,
    frame_rate: FrameRate,
) -> gpui::Div {
    let target = context.target;
    let locked = context.locked;
    let event_bus = &context.event_bus;
    let return_focus = &context.return_focus;
    let alpha = properties.color >> 24;
    let color_rgb = properties.color & 0x00ff_ffff;
    let color_text = if alpha == 0xff {
        format!("{color_rgb:06X}")
    } else {
        format!("{color_rgb:06X}{alpha:02X}")
    };
    let content_box = if locked {
        div()
            .min_h(px(58.0))
            .px(px(10.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .bg(rgb(0x19191d))
            .text_size(px(13.0))
            .line_height(px(19.0))
            .children(
                properties
                    .text
                    .lines()
                    .map(|line| div().min_h(px(19.0)).child(line.to_owned())),
            )
            .into_any_element()
    } else {
        let event_bus = event_bus.clone();
        TextArea::new(
            "text-clip-content",
            properties.text.clone(),
            return_focus.clone(),
        )
        .placeholder("Text")
        .on_text_change(move |text, _, cx| {
            let event = AppEvent::EditTextProperty {
                target,
                change: TextPropertyChange::Content(text.clone()),
            };
            event_bus.update(cx, |_, cx| cx.emit(event));
        })
        .into_any_element()
    };
    let font_size = if locked {
        text_value_view("", properties.font_size.to_string(), "px", true).h(px(34.0))
    } else {
        div().child(FontSizeInput {
            target,
            value: properties.font_size,
            event_bus: event_bus.clone(),
        })
    };
    let background_action = match (locked, properties.background) {
        (true, _) => None,
        (false, None) => {
            let event_bus = event_bus.clone();
            Some(
                section_action_view("text-background-add", "Add").on_click(move |_, _, cx| {
                    event_bus.update(cx, |_, cx| {
                        cx.emit(AppEvent::EditTextProperty {
                            target,
                            change: TextPropertyChange::Background(Some(DEFAULT_TEXT_BACKGROUND)),
                        })
                    })
                }),
            )
        }
        (false, Some(_)) => {
            let event_bus = event_bus.clone();
            Some(
                section_action_view("text-background-remove", "Remove").on_click(
                    move |_, _, cx| {
                        event_bus.update(cx, |_, cx| {
                            cx.emit(AppEvent::EditTextProperty {
                                target,
                                change: TextPropertyChange::Background(None),
                            })
                        })
                    },
                ),
            )
        }
    };
    let background_body = match properties.background {
        None => empty_section_view("No background"),
        Some(background) => {
            let opacity = background_opacity_percent(background.alpha);
            let opaque_color = 0xff00_0000 | background.rgba() >> 8; // 色块显示完整颜色，不透明度只在百分比里体现
            let (color, opacity_field) = if locked {
                (
                    locked_color_view(opaque_color, format!("{:06X}", background.rgba() >> 8))
                        .into_any_element(),
                    text_value_view("", opacity.to_string(), "%", true)
                        .h(px(34.0))
                        .into_any_element(),
                )
            } else {
                (
                    ColorInput {
                        target,
                        value: opaque_color,
                        color_target: ColorTarget::Background,
                        event_bus: event_bus.clone(),
                    }
                    .into_any_element(),
                    BackgroundOpacityInput {
                        target,
                        alpha: background.alpha,
                        event_bus: event_bus.clone(),
                    }
                    .into_any_element(),
                )
            };
            div()
                .mt(px(12.0))
                .flex()
                .gap(px(8.0))
                .child(div().flex_1().min_w_0().child(color))
                .child(div().w(px(80.0)).flex_shrink_0().child(opacity_field))
        }
    };
    div()
        .child(property_panel_heading_view("CONTENT"))
        .child(div().mt(px(12.0)).child(content_box))
        .child(property_panel_divider_view())
        .child(property_panel_heading_view("TIMING"))
        .child(
            div()
                .mt(px(8.0))
                .grid()
                .grid_cols(2)
                .gap(px(8.0))
                .child(
                    div()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(11.0))
                                .line_height(px(16.0))
                                .text_color(rgb(MUTED))
                                .child("Start"),
                        )
                        .child(div().mt(px(4.0)).child(disabled_text_input_view(
                            format_property_panel_time(frame_rate.seconds(start)),
                            "",
                            format!("{} f", i64::from(start)),
                        ))),
                )
                .child(
                    div()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(11.0))
                                .line_height(px(16.0))
                                .text_color(rgb(MUTED))
                                .child("Duration"),
                        )
                        .child(div().mt(px(4.0)).child(disabled_text_input_view(
                            format!("{:.2}", duration.as_secs_f64()),
                            "",
                            "s".into(),
                        ))),
                ),
        )
        .child(property_panel_divider_view())
        .child(property_panel_heading_view("STYLE"))
        .child(
            div().mt(px(12.0)).child(style_field_view(
                "Font",
                div()
                    .when(locked, |field| {
                        field.child(
                            div()
                                .h(px(34.0))
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .px(px(10.0))
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(rgb(0x222226))
                                .bg(rgb(0x131316))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(13.0))
                                        .text_color(rgb(0x55555e))
                                        .text_ellipsis()
                                        .child(properties.font.clone()),
                                )
                                .child(div().flex_shrink_0().text_color(rgb(0x46464e)).child("⌄")),
                        )
                    })
                    .when(!locked, |field| {
                        field.child(FontSelector {
                            target,
                            value: properties.font.clone(),
                            event_bus: event_bus.clone(),
                        })
                    }),
            )),
        )
        .child(
            div()
                .mt(px(12.0))
                .grid()
                .grid_cols(2)
                .gap(px(8.0))
                .child(style_field_view("Font size", font_size))
                .child(style_field_view(
                    "Font color",
                    div()
                        .when(!locked, |field| {
                            field.child(ColorInput {
                                target,
                                value: properties.color,
                                color_target: ColorTarget::Font,
                                event_bus: event_bus.clone(),
                            })
                        })
                        .when(locked, |field| {
                            field.child(locked_color_view(properties.color, color_text))
                        }),
                )),
        )
        .child(property_panel_divider_view())
        .child(section_heading_view("BACKGROUND", background_action))
        .child(background_body)
        .child(property_panel_divider_view())
        .child(property_panel_heading_view("POSITION"))
        .child(
            div()
                .mt(px(12.0))
                .grid()
                .grid_cols(2)
                .gap(px(8.0))
                .child(disabled_text_input_view(
                    format!("{:.2}", properties.position.x),
                    "X",
                    String::new(),
                ))
                .child(disabled_text_input_view(
                    format!("{:.2}", properties.position.y),
                    "Y",
                    String::new(),
                )),
        )
}

/// Maps a text property change to the clip `EditAction` that performs it, so
/// timeline clips keep their existing undoable mutation path.
pub(crate) fn text_change_edit_action(clip_id: Ulid, change: TextPropertyChange) -> EditAction {
    match change {
        TextPropertyChange::Content(text) => EditAction::SetTextContent { clip_id, text },
        TextPropertyChange::Font(font) => EditAction::SetTextFont { clip_id, font },
        TextPropertyChange::FontSize(font_size) => {
            EditAction::SetTextFontSize { clip_id, font_size }
        }
        TextPropertyChange::Color(color) => EditAction::SetTextColor { clip_id, color },
        TextPropertyChange::Background(background) => EditAction::SetTextBackground {
            clip_id,
            background,
        },
        TextPropertyChange::BackgroundColor { red, green, blue } => {
            EditAction::SetTextBackgroundColor {
                clip_id,
                red,
                green,
                blue,
            }
        }
        TextPropertyChange::BackgroundAlpha(alpha) => {
            EditAction::SetTextBackgroundAlpha { clip_id, alpha }
        }
    }
}

/// Applies a text property change to a pool title's properties, mirroring the
/// clip `EditAction` semantics (a background edit needs an existing background).
pub(crate) fn apply_text_change(
    properties: &mut TextClipProperties,
    change: &TextPropertyChange,
) -> Result<()> {
    match change {
        TextPropertyChange::Content(text) => properties.text = text.clone(),
        TextPropertyChange::Font(font) => properties.font = font.clone(),
        TextPropertyChange::FontSize(font_size) => properties.font_size = *font_size,
        TextPropertyChange::Color(color) => properties.color = *color,
        TextPropertyChange::Background(background) => properties.background = *background,
        TextPropertyChange::BackgroundColor { red, green, blue } => {
            let Some(background) = properties.background.as_mut() else {
                bail!("The title has no background");
            };
            background.red = *red;
            background.green = *green;
            background.blue = *blue;
        }
        TextPropertyChange::BackgroundAlpha(alpha) => {
            let Some(background) = properties.background.as_mut() else {
                bail!("The title has no background");
            };
            background.alpha = *alpha;
        }
    }
    Ok(())
}

fn section_heading_view(
    label: &'static str,
    action: Option<gpui::Stateful<gpui::Div>>,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(property_panel_heading_view(label))
        .when_some(action, |heading, action| heading.child(action))
}

fn section_action_view(id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .text_size(px(12.0))
        .line_height(px(16.0))
        .text_color(rgb(ACCENT))
        .cursor(CursorStyle::PointingHand)
        .child(label)
}

fn locked_color_view(color: u32, text: String) -> gpui::Div {
    div()
        .h(px(34.0))
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(8.0))
        .rounded(px(6.0))
        .bg(rgb(0x19191d))
        .child(
            div()
                .size(px(16.0))
                .flex_shrink_0()
                .rounded(px(4.0))
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgba(color.rotate_left(8))), // ARGB → RGBA。
        )
        .child(
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(2.0))
                .font_family("monospace")
                .text_size(px(13.0))
                .child(div().flex_shrink_0().text_color(rgb(MUTED)).child("#"))
                .child(div().min_w_0().text_ellipsis().child(text)),
        )
}

fn background_opacity_percent(alpha: u8) -> u32 {
    (u32::from(alpha) * 100 + 127) / 255
}

const DEFAULT_TEXT_BACKGROUND: TextBackground = TextBackground {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 128,
};

#[derive(Clone, Copy)]
enum ColorTarget {
    Font,
    Background,
}

fn color_change(color_target: ColorTarget, color: u32) -> TextPropertyChange {
    match color_target {
        ColorTarget::Font => TextPropertyChange::Color(color),
        ColorTarget::Background => {
            let [_, red, green, blue] = color.to_be_bytes();
            TextPropertyChange::BackgroundColor { red, green, blue }
        }
    }
}

fn empty_section_view(text: &'static str) -> gpui::Div {
    div()
        .mt(px(8.0))
        .text_size(px(12.0))
        .text_color(rgb(0x55555e))
        .child(text)
}

fn style_field_view(label: &'static str, value: gpui::Div) -> gpui::Div {
    div()
        .min_w_0()
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(rgb(0xb4b4bc))
                .child(label),
        )
        .child(div().mt(px(8.0)).child(value))
}

fn text_value_view(
    label: &'static str,
    value: String,
    unit: &'static str,
    locked: bool,
) -> gpui::Div {
    div()
        .h(px(30.0))
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .when(!locked, |field| field.border_1().border_color(rgb(BORDER)))
        .bg(rgb(if locked {
            0x19191d
        } else {
            SURFACE
        }))
        .text_size(px(12.0))
        .when(!label.is_empty(), |field| {
            field.child(
                div()
                    .flex_shrink_0()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(MUTED))
                    .child(label),
            )
        })
        .child(
            div()
                .min_w_0()
                .flex_1()
                .font_family("monospace")
                .text_size(px(13.0))
                .text_ellipsis()
                .child(value),
        )
        .when(!unit.is_empty(), |field| {
            field.child(div().flex_shrink_0().text_color(rgb(MUTED)).child(unit))
        })
}

#[derive(IntoElement)]
struct FontSizeInput {
    target: TextEditTarget,
    value: f64,
    event_bus: Entity<EventBus>,
}

impl RenderOnce for FontSizeInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let return_focus = match window.focused(cx) {
            Some(handle) => handle,
            None => cx.focus_handle(),
        };
        let target = self.target;
        let value = self.value;
        let event_bus = self.event_bus;
        let input = window.use_keyed_state(
            format!("text-font-size-{}", target.key()),
            cx,
            |window, cx| {
                let field =
                    TextInput::new_property_number_field(value.to_string(), return_focus, cx);
                cx.subscribe(&cx.entity(), move |input, _, _: &TextInputEvent, cx| {
                    let Ok(font_size) = input.text().parse::<f64>() else {
                        return;
                    };
                    if !font_size.is_finite() || font_size <= 0.0 {
                        return;
                    }
                    event_bus.update(cx, |_, cx| {
                        cx.emit(AppEvent::EditTextProperty {
                            target,
                            change: TextPropertyChange::FontSize(font_size),
                        })
                    });
                })
                .detach();
                cx.on_blur(&field.focus_handle(cx), window, |_, _, cx| cx.notify())
                    .detach();
                field
            },
        );
        input.update(cx, |field, cx| {
            let focused = field.focus_handle(cx).is_focused(window);
            let parsed = field.text().parse::<f64>();
            let valid = matches!(parsed, Ok(number) if number.is_finite() && number > 0.0);
            // 保留输入中的空值或小数点；失焦时恢复模型值，撤销/重做也同步有效数值。
            if (!focused || valid) && parsed.ok() != Some(value) {
                field.set_text_silently(value.to_string());
            }
        });
        let valid = matches!(input.read(cx).text().parse::<f64>(), Ok(number) if number.is_finite() && number > 0.0);
        div()
            .h(px(34.0))
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(9.0)) // 加 1px 边框后与只读态一致：内容距外缘 10px。
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(if valid {
                BORDER
            } else {
                0xff8b8b
            }))
            .bg(rgb(SURFACE))
            .child(div().min_w_0().flex_1().child(input))
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED))
                    .child("px"),
            )
    }
}

#[derive(IntoElement)]
struct BackgroundOpacityInput {
    target: TextEditTarget,
    alpha: u8,
    event_bus: Entity<EventBus>,
}

impl RenderOnce for BackgroundOpacityInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let return_focus = match window.focused(cx) {
            Some(handle) => handle,
            None => cx.focus_handle(),
        };
        let target = self.target;
        let value = background_opacity_percent(self.alpha);
        let event_bus = self.event_bus;
        let input = window.use_keyed_state(
            format!("text-background-opacity-{}", target.key()),
            cx,
            |window, cx| {
                let field =
                    TextInput::new_property_number_field(value.to_string(), return_focus, cx);
                cx.subscribe(&cx.entity(), move |input, _, _: &TextInputEvent, cx| {
                    let Ok(percent) = input.text().parse::<u32>() else {
                        return;
                    };
                    if percent > 100 {
                        return;
                    }
                    let alpha = ((percent * 255 + 50) / 100) as u8;
                    event_bus.update(cx, |_, cx| {
                        cx.emit(AppEvent::EditTextProperty {
                            target,
                            change: TextPropertyChange::BackgroundAlpha(alpha),
                        })
                    });
                })
                .detach();
                cx.on_blur(&field.focus_handle(cx), window, |_, _, cx| cx.notify())
                    .detach();
                field
            },
        );
        input.update(cx, |field, cx| {
            let focused = field.focus_handle(cx).is_focused(window);
            let parsed = field.text().parse::<u32>();
            let valid = matches!(parsed, Ok(percent) if percent <= 100);
            // 输入中的空值保持原样；失焦或撤销/重做后同步模型值。
            if (!focused || valid) && parsed.ok() != Some(value) {
                field.set_text_silently(value.to_string());
            }
        });
        let valid = matches!(input.read(cx).text().parse::<u32>(), Ok(percent) if percent <= 100);
        div()
            .h(px(34.0))
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(9.0)) // 加 1px 边框后与只读态一致：内容距外缘 10px。
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(if valid {
                BORDER
            } else {
                0xff8b8b
            }))
            .bg(rgb(SURFACE))
            .child(div().min_w_0().flex_1().child(input))
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED))
                    .child("%"),
            )
    }
}

#[derive(IntoElement)]
struct ColorInput {
    target: TextEditTarget,
    value: u32, // 大端 ARGB；只编辑 RGB，透明度保持不变
    color_target: ColorTarget,
    event_bus: Entity<EventBus>,
}

impl RenderOnce for ColorInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let target = self.target;
        let value = self.value;
        let color_target = self.color_target;
        let (key, swatch_id, palette_id, close_id) = match color_target {
            ColorTarget::Font => (
                "text-color",
                "font-color-swatch",
                "font-color-palette",
                "close-font-color-palette",
            ),
            ColorTarget::Background => (
                "text-background-color",
                "background-color-swatch",
                "background-color-palette",
                "close-background-color-palette",
            ),
        };
        let alpha = value & 0xff00_0000;
        let event_bus = self.event_bus;
        let input_bus = event_bus.clone();
        let return_focus = match window.focused(cx) {
            Some(handle) => handle,
            None => cx.focus_handle(),
        };
        let input = window.use_keyed_state(
            format!("{key}-{}-{alpha}", target.key()),
            cx,
            |window, cx| {
                let field = TextInput::new_property_hex_field(
                    format!("{:06X}", value & 0xffffff),
                    return_focus,
                    cx,
                );
                cx.subscribe(&cx.entity(), move |field, _, _: &TextInputEvent, cx| {
                    // 未完成的十六进制输入是正常草稿；只提交完整的 RGB，保留原有透明度。
                    let Some(color) = parse_hex_color(field.text()) else {
                        return;
                    };
                    input_bus.update(cx, |_, cx| {
                        cx.emit(AppEvent::EditTextProperty {
                            target,
                            change: color_change(color_target, alpha | color),
                        })
                    });
                })
                .detach();
                cx.on_blur(&field.focus_handle(cx), window, |_, _, cx| cx.notify())
                    .detach();
                field
            },
        );
        input.update(cx, |field, cx| {
            let parsed = parse_hex_color(field.text());
            if (!field.focus_handle(cx).is_focused(window) || parsed.is_some())
                && parsed != Some(value & 0xffffff)
            {
                field.set_text_silently(format!("{:06X}", value & 0xffffff));
            }
        });
        let open =
            window.use_keyed_state(format!("{key}-palette-{}", target.key()), cx, |_, _| false);
        let is_open = *open.read(cx);
        let toggle = open.clone();
        let valid = parse_hex_color(input.read(cx).text()).is_some();
        let mut field = div()
            .relative()
            .h(px(34.0))
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(7.0)) // 加 1px 边框后与只读态一致：色块距外缘 8px。
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(if valid {
                BORDER
            } else {
                0xff8b8b
            }))
            .bg(rgb(SURFACE))
            .child(
                div()
                    .id(swatch_id)
                    .size(px(16.0))
                    .flex_shrink_0()
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgba(value.rotate_left(8)))
                    .cursor(CursorStyle::PointingHand)
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        cx.stop_propagation();
                        toggle.update(cx, |shown, cx| {
                            *shown = !*shown;
                            cx.notify();
                        });
                        window.refresh();
                    }),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_family("monospace")
                            .text_size(px(13.0))
                            .text_color(rgb(MUTED))
                            .child("#"),
                    )
                    .child(div().min_w_0().flex_1().child(input.clone())),
            );
        if is_open {
            let close = open.clone();
            let outside = open.clone();
            let mut palette = div()
                .id(palette_id)
                .relative()
                .w(px(248.0))
                .p(px(15.0))
                .flex()
                .flex_col()
                .gap(px(16.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(rgb(0x45454d))
                .bg(rgb(0x202024))
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down_out(move |_, window, cx| {
                    outside.update(cx, |shown, cx| {
                        *shown = false;
                        cx.notify();
                    });
                    window.refresh();
                })
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .text_size(px(13.0))
                        .text_color(rgb(0xf2f2f4))
                        .child("Color families")
                        .child(
                            div()
                                .id(close_id)
                                .cursor(CursorStyle::PointingHand)
                                .child("×")
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    cx.stop_propagation();
                                    close.update(cx, |shown, cx| {
                                        *shown = false;
                                        cx.notify();
                                    });
                                    window.refresh();
                                }),
                        ),
                );
            let colors = [
                0xffffff, 0xcccccc, 0x999999, 0x666666, 0x333333, 0x000000, 0xff8b8b, 0xf0b75e,
                0xffe58a, 0x9bdca4, 0x91b8df, 0xb69bdc, 0xe84545, 0xe69324, 0xe5c638, 0x41ac62,
                0x438fd4, 0x9164ce, 0x8e2828, 0x915617, 0x8a7621, 0x256b3a, 0x285886, 0x58377f,
            ];
            let mut grid = div().flex().flex_col().gap(px(8.0));
            for row in colors.chunks(6) {
                let mut line = div().flex().gap(px(6.0));
                for &color in row {
                    let selected = value & 0xffffff == color;
                    let select_bus = event_bus.clone();
                    let select_input = input.clone();
                    let contrast = if ((color >> 16) & 255) * 299
                        + ((color >> 8) & 255) * 587
                        + (color & 255) * 114
                        > 128000
                    {
                        0x17120a
                    } else {
                        0xffffff
                    };
                    line = line.child(
                        div()
                            .id(("preset-color", color as usize))
                            .size(px(30.0))
                            .flex_shrink_0()
                            .rounded(px(4.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgb(color))
                            .border_1()
                            .border_color(rgb(if selected {
                                0xf2f2f4
                            } else {
                                0x45454d
                            }))
                            .hover(|style| {
                                style.shadow(vec![
                                    BoxShadow::new(px(0.0), px(0.0), rgb(0x080809).into())
                                        .spread_radius(px(4.0)),
                                    BoxShadow::new(px(0.0), px(0.0), rgb(0xf2f2f4).into())
                                        .spread_radius(px(3.0)),
                                    BoxShadow::new(px(0.0), px(0.0), rgb(0x080809).into())
                                        .spread_radius(px(1.0)),
                                ])
                            })
                            .cursor(CursorStyle::PointingHand)
                            .text_color(rgb(contrast))
                            .when(selected, |swatch| swatch.child("✓"))
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                cx.stop_propagation();
                                select_input.update(cx, |field, cx| {
                                    field.set_text_silently(format!("{color:06X}"));
                                    cx.notify();
                                });
                                select_bus.update(cx, |_, cx| {
                                    cx.emit(AppEvent::EditTextProperty {
                                        target,
                                        change: color_change(color_target, alpha | color),
                                    })
                                });
                                window.refresh();
                            }),
                    );
                }
                grid = grid.child(line);
            }
            palette = palette
                .child(grid)
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(MUTED))
                        .child("Neutrals, then light, vivid and dark."),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(MUTED))
                        .child("Custom color? Edit the hex value."),
                );
            palette = palette.child(
                canvas(
                    |_, _, _| (),
                    |bounds, (), window, _| {
                        let left = bounds.origin + point(px(0.0), px(0.0));
                        let tip = bounds.origin + point(px(7.0), px(8.0));
                        let right = bounds.origin + point(px(14.0), px(0.0));
                        let mut fill = PathBuilder::fill();
                        fill.move_to(left);
                        fill.line_to(tip);
                        fill.line_to(right);
                        fill.close();
                        window.paint_path(
                            fill.build().expect("valid palette pointer triangle"),
                            rgb(0x202024),
                        );
                        let mut outline = PathBuilder::stroke(px(1.0));
                        outline.move_to(left);
                        outline.line_to(tip);
                        outline.line_to(right);
                        window.paint_path(
                            outline.build().expect("valid palette pointer outline"),
                            rgb(0x45454d),
                        );
                    },
                )
                .absolute()
                .right(px(96.0))
                .bottom(px(-7.0))
                .w(px(14.0))
                .h(px(8.0)),
            );
            field = field.child(
                div().absolute().right_0().top_0().child(
                    deferred(
                        anchored()
                            .anchor(Anchor::BottomRight)
                            .offset(point(px(0.0), px(-12.0)))
                            .child(palette),
                    )
                    .with_priority(1),
                ),
            );
        }
        field
    }
}

fn parse_hex_color(text: &str) -> Option<u32> {
    if text.len() != 6 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(text, 16).ok()
}

#[derive(IntoElement)]
struct FontSelector {
    target: TextEditTarget,
    value: String,
    event_bus: Entity<EventBus>,
}

impl RenderOnce for FontSelector {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // 字体列表只在弹层打开时保留快照，每次打开重新查询系统。
        let available =
            window.use_keyed_state(format!("font-picker-{}", self.target.key()), cx, |_, _| {
                None::<Vec<String>>
            });
        let open = available.clone();
        let mut field = div().relative().child(
            div()
                .id("font-selector")
                .group("property-font-dropdown")
                .h(px(34.0))
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(10.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(SURFACE))
                .hover(|style| style.bg(rgb(0x2a241b)).border_color(rgb(0xf0b75e)))
                .cursor(CursorStyle::PointingHand)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(13.0))
                        .text_color(rgb(0xf2f2f4))
                        .text_ellipsis()
                        .child(self.value.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_color(rgb(MUTED))
                        .group_hover("property-font-dropdown", |style| {
                            style.text_color(rgb(0xf0b75e))
                        })
                        .child("⌄"),
                )
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    let fonts = cx.text_system().all_font_names();
                    open.update(cx, |snapshot, cx| {
                        *snapshot = Some(fonts);
                        cx.notify();
                    });
                    window.refresh();
                }),
        );
        let Some(fonts) = available.read(cx).as_ref() else {
            return field;
        };
        let families = fonts.clone();
        let return_focus = match window.focused(cx) {
            Some(handle) => handle,
            None => cx.focus_handle(),
        };
        let search =
            window.use_keyed_state(format!("font-search-{}", self.target.key()), cx, |_, cx| {
                TextInput::new_search("font-search", "Search fonts…", return_focus, cx)
            });
        let query = search.read(cx).text().to_lowercase();
        let close = available.clone();
        let outside = available.clone();
        let mut list = div()
            .id("font-family-list")
            .max_h(px(304.0))
            .min_h_0()
            .gap(px(8.0))
            .overflow_y_scroll()
            .flex()
            .flex_col();
        let mut matches = 0;
        for family in families {
            if !family.to_lowercase().contains(&query) {
                continue;
            }
            matches += 1;
            let selected = family == self.value;
            let event_bus = self.event_bus.clone();
            let target = self.target;
            list = list.child(
                div()
                    .id(format!("font-family-{family}"))
                    .min_h(px(96.0))
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .rounded(px(7.0))
                    .border_1()
                    .border_color(rgb(if selected {
                        0xf0b75e
                    } else {
                        0x34343b
                    }))
                    .bg(rgb(0x19191d))
                    .cursor(CursorStyle::PointingHand)
                    .text_color(rgb(0xf2f2f4))
                    .when(!selected, |card| {
                        card.hover(|style| style.border_color(rgb(0x8a8a93)))
                    })
                    .child(
                        div()
                            .h(px(33.0))
                            .flex_shrink_0()
                            .px(px(11.0))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .border_b_1()
                            .border_color(rgb(if selected {
                                0x3b3327
                            } else {
                                0x34343b
                            }))
                            .text_size(px(12.0))
                            .text_color(rgb(if selected {
                                0xf0b75e
                            } else {
                                0xb4b4bc
                            }))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_ellipsis()
                                    .child(family.clone()),
                            )
                            .when(selected, |header| header.child("✓")),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .px(px(11.0))
                            .py(px(12.0))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .min_w_0()
                                    .font_family(family.clone())
                                    .text_size(px(17.0))
                                    .whitespace_normal()
                                    .child("你好，世界 · Hello,world"),
                            ),
                    )
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        cx.stop_propagation();
                        event_bus.update(cx, |_, cx| {
                            cx.emit(AppEvent::EditTextProperty {
                                target,
                                change: TextPropertyChange::Font(family.clone()),
                            })
                        });
                    }),
            );
        }
        if matches == 0 {
            list = list.child(
                div()
                    .p(px(8.0))
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED))
                    .child("No matching fonts"),
            );
        }
        let popup = div()
            .id("font-picker-popup")
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(rgb(0x45454d))
            .bg(rgb(0x202024))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down_out(move |_, window, cx| {
                outside.update(cx, |snapshot, cx| {
                    *snapshot = None;
                    cx.notify();
                });
                window.refresh();
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.0))
                    .text_color(rgb(0xf2f2f4))
                    .child("Font")
                    .child(
                        div()
                            .id("close-font-picker")
                            .cursor(CursorStyle::PointingHand)
                            .child("×")
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                cx.stop_propagation();
                                close.update(cx, |snapshot, cx| {
                                    *snapshot = None;
                                    cx.notify();
                                });
                                window.refresh();
                            }),
                    ),
            )
            .child(search)
            .child(list)
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED))
                    .child("Click a font to apply."),
            );
        field = field.child(
            canvas(
                move |bounds, window, cx| {
                    // 直接用本帧字体输入框的宽度定位弹层，不存储派生的布局状态。
                    let mut overlay = anchored()
                        .anchor(Anchor::TopLeft)
                        .position(bounds.origin + point(px(0.0), px(-240.0)))
                        .snap_to_window_with_margin(px(16.0))
                        .child(
                            popup
                                .w(bounds.size.width)
                                .max_h((window.viewport_size().height - px(32.0)).max(px(0.0))),
                        )
                        .into_any_element();
                    // defer_draw 只延迟预绘制和绘制；动态创建的弹层必须先完成布局。
                    overlay.layout_as_root(window.viewport_size().into(), window, cx);
                    window.defer_draw(overlay, point(px(0.0), px(0.0)), 1, None);
                },
                |_, (), _, _| {},
            )
            .absolute()
            .inset_0(),
        );
        field
    }
}
