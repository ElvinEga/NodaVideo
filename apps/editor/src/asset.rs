use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

#[derive(Clone, Copy)]
pub(crate) enum IconName {
    AlertTriangle,
    App,
    ArrowDown,
    ArrowUp,
    Edit,
    Expand,
    Eye,
    EyeOff,
    Folder,
    Lock,
    Magnet,
    Mic,
    MicOff,
    Minus,
    Mute,
    Next,
    NodaMark,
    PanelLeft,
    Pause,
    PickerGlow,
    Play,
    Playhead,
    Plug,
    Plus,
    Pointer,
    Previous,
    Scissors,
    Shield,
    Snap,
    Sparkle,
    Spinner,
    Text,
    Trash,
    Unmute,
    Video,
    Warning,
}

// Public APIs only
impl IconName {
    pub(crate) const fn path(self) -> &'static str {
        match self {
            Self::AlertTriangle => "icons/alert-triangle.svg",
            Self::App => "icons/app-icon.svg",
            Self::ArrowDown => "icons/arrow-down.svg",
            Self::ArrowUp => "icons/arrow-up.svg",
            Self::Edit => "icons/edit.svg",
            Self::Expand => "icons/expand.svg",
            Self::Eye => "icons/eye.svg",
            Self::EyeOff => "icons/eye-off.svg",
            Self::Folder => "icons/folder.svg",
            Self::Lock => "icons/lock.svg",
            Self::Magnet => "icons/magnet.svg",
            Self::Mic => "icons/mic.svg",
            Self::MicOff => "icons/mic-off.svg",
            Self::Minus => "icons/minus.svg",
            Self::Mute => "icons/mute.svg",
            Self::Next => "icons/next.svg",
            Self::NodaMark => "icons/noda-mark.svg",
            Self::PanelLeft => "icons/panel-left.svg",
            Self::Pause => "icons/pause.svg",
            Self::PickerGlow => "icons/picker-glow.svg",
            Self::Play => "icons/play.svg",
            Self::Playhead => "icons/playhead.svg",
            Self::Plug => "icons/plug.svg",
            Self::Plus => "icons/plus.svg",
            Self::Pointer => "icons/pointer.svg",
            Self::Previous => "icons/previous.svg",
            Self::Scissors => "icons/scissors.svg",
            Self::Shield => "icons/shield.svg",
            Self::Snap => "icons/snap.svg",
            Self::Sparkle => "icons/sparkle.svg",
            Self::Spinner => "icons/spinner.svg",
            Self::Text => "icons/text.svg",
            Self::Trash => "icons/trash.svg",
            Self::Unmute => "icons/unmute.svg",
            Self::Video => "icons/video.svg",
            Self::Warning => "icons/warning.svg",
        }
    }
}

pub(crate) struct EditorAssets;

impl AssetSource for EditorAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let Some(icon) = IconName::from_path(path) else {
            return Ok(None);
        };
        Ok(Some(Cow::Borrowed(icon.bytes())))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(match path {
            "icons" => IconName::ALL
                .into_iter()
                .map(|icon| icon.path().trim_start_matches("icons/").into())
                .collect(),
            _ => Vec::new(),
        })
    }
}

// Private APIs only
impl IconName {
    const ALL: [Self; 36] = [
        Self::AlertTriangle,
        Self::App,
        Self::ArrowDown,
        Self::ArrowUp,
        Self::Edit,
        Self::Expand,
        Self::Eye,
        Self::EyeOff,
        Self::Folder,
        Self::Lock,
        Self::Magnet,
        Self::Mic,
        Self::MicOff,
        Self::Minus,
        Self::Mute,
        Self::Next,
        Self::NodaMark,
        Self::PanelLeft,
        Self::Pause,
        Self::PickerGlow,
        Self::Play,
        Self::Playhead,
        Self::Plug,
        Self::Plus,
        Self::Pointer,
        Self::Previous,
        Self::Scissors,
        Self::Shield,
        Self::Snap,
        Self::Sparkle,
        Self::Spinner,
        Self::Text,
        Self::Trash,
        Self::Unmute,
        Self::Video,
        Self::Warning,
    ];

    const fn bytes(self) -> &'static [u8] {
        match self {
            Self::AlertTriangle => include_bytes!("../../../icons/alert-triangle.svg"),
            Self::App => include_bytes!("../../../icons/app-icon.svg"),
            Self::ArrowDown => include_bytes!("../../../icons/arrow-down.svg"),
            Self::ArrowUp => include_bytes!("../../../icons/arrow-up.svg"),
            Self::Edit => include_bytes!("../../../icons/edit.svg"),
            Self::Expand => include_bytes!("../../../icons/expand.svg"),
            Self::Eye => include_bytes!("../../../icons/eye.svg"),
            Self::EyeOff => include_bytes!("../../../icons/eye-off.svg"),
            Self::Folder => include_bytes!("../../../icons/folder.svg"),
            Self::Lock => include_bytes!("../../../icons/lock.svg"),
            Self::Magnet => include_bytes!("../../../icons/magnet.svg"),
            Self::Mic => include_bytes!("../../../icons/mic.svg"),
            Self::MicOff => include_bytes!("../../../icons/mic-off.svg"),
            Self::Minus => include_bytes!("../../../icons/minus.svg"),
            Self::Mute => include_bytes!("../../../icons/mute.svg"),
            Self::Next => include_bytes!("../../../icons/next.svg"),
            Self::NodaMark => include_bytes!("../../../icons/noda-mark.svg"),
            Self::PanelLeft => include_bytes!("../../../icons/panel-left.svg"),
            Self::Pause => include_bytes!("../../../icons/pause.svg"),
            Self::PickerGlow => include_bytes!("../../../icons/picker-glow.svg"),
            Self::Play => include_bytes!("../../../icons/play.svg"),
            Self::Playhead => include_bytes!("../../../icons/playhead.svg"),
            Self::Plug => include_bytes!("../../../icons/plug.svg"),
            Self::Plus => include_bytes!("../../../icons/plus.svg"),
            Self::Pointer => include_bytes!("../../../icons/pointer.svg"),
            Self::Previous => include_bytes!("../../../icons/previous.svg"),
            Self::Scissors => include_bytes!("../../../icons/scissors.svg"),
            Self::Shield => include_bytes!("../../../icons/shield.svg"),
            Self::Snap => include_bytes!("../../../icons/snap.svg"),
            Self::Sparkle => include_bytes!("../../../icons/sparkle.svg"),
            Self::Spinner => include_bytes!("../../../icons/spinner.svg"),
            Self::Text => include_bytes!("../../../icons/text.svg"),
            Self::Trash => include_bytes!("../../../icons/trash.svg"),
            Self::Unmute => include_bytes!("../../../icons/unmute.svg"),
            Self::Video => include_bytes!("../../../icons/video.svg"),
            Self::Warning => include_bytes!("../../../icons/warning.svg"),
        }
    }

    fn from_path(path: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|icon| icon.path() == path)
    }
}
