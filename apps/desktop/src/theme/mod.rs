//! Theme definitions and styling helpers for NodaVideo.
//!
//! Provides professional NLE color tokens and typography helpers adhering to
//! the GPUI Kit design system.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;

/// Initialize the NodaVideo theme on application startup.
///
/// Sets dark mode as the default aesthetic for professional video editing.
pub fn init(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
}

/// NLE layout metrics and constants.
pub mod metrics {
    use gpui_kit::{Pixels, px};

    /// Height of the top toolbar.
    pub const TOOLBAR_HEIGHT: Pixels = px(42.0);

    /// Height of the window status bar.
    pub const STATUS_BAR_HEIGHT: Pixels = px(24.0);

    /// Minimum width for the Media Browser panel.
    pub const MEDIA_BROWSER_MIN_WIDTH: Pixels = px(200.0);
    pub const MEDIA_BROWSER_DEFAULT_WIDTH: Pixels = px(280.0);
    pub const MEDIA_BROWSER_MAX_WIDTH: Pixels = px(500.0);

    /// Minimum width for the Inspector panel.
    pub const INSPECTOR_MIN_WIDTH: Pixels = px(220.0);
    pub const INSPECTOR_DEFAULT_WIDTH: Pixels = px(290.0);
    pub const INSPECTOR_MAX_WIDTH: Pixels = px(500.0);

    /// Width range for the AI Assistant panel.
    pub const ASSISTANT_MIN_WIDTH: Pixels = px(260.0);
    pub const ASSISTANT_DEFAULT_WIDTH: Pixels = px(330.0);
    pub const ASSISTANT_MAX_WIDTH: Pixels = px(600.0);

    /// Height range for the bottom Timeline panel.
    pub const TIMELINE_MIN_HEIGHT: Pixels = px(180.0);
    pub const TIMELINE_DEFAULT_HEIGHT: Pixels = px(280.0);
    pub const TIMELINE_MAX_HEIGHT: Pixels = px(650.0);

    /// Default track header width on the timeline.
    pub const TIMELINE_TRACK_HEADER_WIDTH: Pixels = px(130.0);

    /// Height of each timeline track lane.
    pub const TIMELINE_TRACK_HEIGHT: Pixels = px(40.0);

    /// Height of the timeline time ruler.
    pub const TIMELINE_RULER_HEIGHT: Pixels = px(28.0);
}
