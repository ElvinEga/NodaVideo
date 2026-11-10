//! Text overlay and caption styling domain models.

use serde::{Deserialize, Serialize};

/// Text alignment on the video canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlignment {
    Top,
    Center,
    #[default]
    Bottom,
}

/// Visual styling configuration for a text overlay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextStyle {
    /// Font family name (e.g. "Inter", "Arial", "Roboto").
    pub font_family: String,
    /// Font size in canvas pixels.
    pub font_size: u32,
    /// Text color as a hex string (e.g. "#ffffff").
    pub color: String,
    /// Optional background bounding box color (e.g. "#00000088").
    pub background_color: Option<String>,
    /// Optional stroke outline color.
    pub stroke_color: Option<String>,
    /// Stroke outline width in pixels.
    pub stroke_width: u32,
    /// Vertical screen alignment anchor.
    pub alignment: TextAlignment,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "Inter".into(),
            font_size: 48,
            color: "#ffffff".into(),
            background_color: None,
            stroke_color: Some("#000000".into()),
            stroke_width: 2,
            alignment: TextAlignment::Bottom,
        }
    }
}

/// Structured content and placement for a text overlay on a clip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextOverlay {
    /// Text string to render.
    pub text: String,
    /// Visual style parameters.
    pub style: TextStyle,
    /// Horizontal offset in canvas pixels.
    pub offset_x: i32,
    /// Vertical offset in canvas pixels.
    pub offset_y: i32,
}

impl TextOverlay {
    /// Create a new text overlay with default style and position.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::default(),
            offset_x: 0,
            offset_y: 0,
        }
    }

    /// Create a text overlay with a specified style.
    #[must_use]
    pub fn with_style(text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            style,
            offset_x: 0,
            offset_y: 0,
        }
    }
}

/// A timed caption entry for batch caption generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptionSegment {
    /// Text content for this caption subtitle.
    pub text: String,
    /// Sequence timeline start frame where subtitle appears.
    pub start_frame: i64,
    /// Sequence timeline end frame where subtitle disappears (exclusive).
    pub end_frame: i64,
}
