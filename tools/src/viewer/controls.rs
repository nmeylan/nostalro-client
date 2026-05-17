use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};

#[derive(Clone, Copy, Debug)]
pub enum ViewerAction {
    CycleBackground,
    TogglePause,
    NextAction,
    PrevAction,
    NextDirection,
    PrevDirection,
    NextWeapon,
    PrevWeapon,
    ToggleSex,
    NextHead,
    PrevHead,
    NextHeadgear,
    PrevHeadgear,
    NextShield,
    PrevShield,
    NextEffect,
    PrevEffect,
    PlayEffect,
    ReplayEffect,
    ResetCamera,
    ZoomIn,
    ZoomOut,
}

pub fn map_key(key: &Key, state: ElementState) -> Option<ViewerAction> {
    if state != ElementState::Pressed {
        return None;
    }
    match key {
        Key::Named(NamedKey::ArrowRight) => Some(ViewerAction::NextDirection),
        Key::Named(NamedKey::ArrowLeft) => Some(ViewerAction::PrevDirection),
        Key::Named(NamedKey::ArrowDown) => Some(ViewerAction::NextAction),
        Key::Named(NamedKey::ArrowUp) => Some(ViewerAction::PrevAction),
        Key::Named(NamedKey::Space) => Some(ViewerAction::TogglePause),
        Key::Character(ch) => match ch.as_str() {
            "b" | "B" => Some(ViewerAction::CycleBackground),
            "q" | "Q" => Some(ViewerAction::NextWeapon),
            "w" | "W" => Some(ViewerAction::PrevWeapon),
            "s" | "S" => Some(ViewerAction::ToggleSex),
            "h" => Some(ViewerAction::NextHead),
            "H" => Some(ViewerAction::PrevHead),
            "e" => Some(ViewerAction::NextHeadgear),
            "E" => Some(ViewerAction::PrevHeadgear),
            "d" | "D" => Some(ViewerAction::NextShield),
            "f" | "F" => Some(ViewerAction::PrevShield),
            "r" | "R" => Some(ViewerAction::ReplayEffect),
            "n" | "N" => Some(ViewerAction::NextEffect),
            "p" | "P" => Some(ViewerAction::PrevEffect),
            "c" | "C" => Some(ViewerAction::ResetCamera),
            "+" | "=" => Some(ViewerAction::ZoomIn),
            "-" | "_" => Some(ViewerAction::ZoomOut),
            _ => None,
        },
        _ => None,
    }
}
