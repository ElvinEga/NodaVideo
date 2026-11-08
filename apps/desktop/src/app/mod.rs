//! Desktop application lifecycle and window orchestration.

use gpui_kit::*;

use crate::theme;
use crate::workspace::WorkspaceView;
use crate::workspace::menus;

/// Initialize and launch the NodaVideo desktop application.
pub fn run() {
    application().with_assets(assets::AllAssets).run(|cx| {
        // 1. Initialize GPUI Kit framework
        init(cx);

        // 2. Initialize application theme (Dark mode default)
        theme::init(cx);

        // 3. Register keybindings and native application menus
        menus::init(cx);

        // 4. Configure window options with desktop NLE dimensions
        let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("NodaVideo".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        // 5. Open main workspace window
        open_window(options, cx, |window, cx| {
            cx.new(|cx| WorkspaceView::new(window, cx))
        })
        .expect("Failed to open NodaVideo workspace window");
    });
}
