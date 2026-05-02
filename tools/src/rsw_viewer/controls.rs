use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};

// Action codes — must match `rsw-viewer-hot/src/lib.rs::ACTION_*`. Treated as
// part of the FFI contract; keep discriminants stable.
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ViewerAction {
    TogglePause = 1,
    ToggleGrid = 2,
    ToggleHover = 3,
    CycleOverlayMode = 4,
    ResetCamera = 5,
    ZoomIn = 6,
    ZoomOut = 7,
    ShowControls = 8,
    ShowMapInfo = 9,
    CloseInfoPanel = 10,
}

pub fn map_key_press(key: &Key, state: ElementState) -> Option<ViewerAction> {
    if state != ElementState::Pressed {
        return None;
    }
    match key {
        Key::Named(NamedKey::Space) => Some(ViewerAction::TogglePause),
        Key::Named(NamedKey::Escape) => Some(ViewerAction::CloseInfoPanel),
        Key::Character(ch) => match ch.as_str() {
            "g" | "G" => Some(ViewerAction::ToggleGrid),
            "h" | "H" => Some(ViewerAction::ToggleHover),
            "o" | "O" => Some(ViewerAction::CycleOverlayMode),
            "r" | "R" => Some(ViewerAction::ResetCamera),
            "+" | "=" => Some(ViewerAction::ZoomIn),
            "-" => Some(ViewerAction::ZoomOut),
            "1" => Some(ViewerAction::ShowControls),
            "2" => Some(ViewerAction::ShowMapInfo),
            _ => None,
        },
        _ => None,
    }
}

// === Flag decoders (host reads ViewerFlags from dylib) ===

/// Matches `OverlayMode` discriminants in the dylib.
#[derive(Clone, Copy, PartialEq)]
pub enum OverlayMode {
    None,
    Grid,
    Hover,
    Full,
}

impl OverlayMode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Grid,
            2 => Self::Hover,
            3 => Self::Full,
            _ => Self::None,
        }
    }
}
