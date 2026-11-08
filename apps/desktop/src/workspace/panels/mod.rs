//! Editor panels module.

pub mod assistant;
pub mod inspector;
pub mod media_browser;
pub mod placeholder;
pub mod preview_monitor;
pub mod timeline;

pub use assistant::AssistantPanel;
pub use inspector::InspectorPanel;
pub use media_browser::MediaBrowserPanel;
pub use placeholder::render_placeholder_workspace;
pub use preview_monitor::PreviewMonitorPanel;
pub use timeline::TimelinePanel;
