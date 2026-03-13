use winit::event::{ElementState, MouseScrollDelta};
use winit::keyboard::{Key, NamedKey};

pub enum ViewerAction {
    NextDirection,
    PrevDirection,
    NextAction,
    PrevAction,
    TogglePause,
    StepForward,
    StepBackward,
    ZoomIn,
    ZoomOut,
    CycleBackground,
    ToggleBrowser,
}

pub fn map_key_press(key: &Key, state: ElementState) -> Option<ViewerAction> {
    if state != ElementState::Pressed {
        return None;
    }
    match key {
        Key::Named(NamedKey::ArrowRight) => Some(ViewerAction::NextDirection),
        Key::Named(NamedKey::ArrowLeft) => Some(ViewerAction::PrevDirection),
        Key::Named(NamedKey::ArrowUp) => Some(ViewerAction::PrevAction),
        Key::Named(NamedKey::ArrowDown) => Some(ViewerAction::NextAction),
        Key::Named(NamedKey::Space) => Some(ViewerAction::TogglePause),
        Key::Named(NamedKey::Tab) => Some(ViewerAction::ToggleBrowser),
        Key::Character(ch) => match ch.as_str() {
            "." => Some(ViewerAction::StepForward),
            "," => Some(ViewerAction::StepBackward),
            "=" | "+" => Some(ViewerAction::ZoomIn),
            "-" => Some(ViewerAction::ZoomOut),
            "b" | "B" => Some(ViewerAction::CycleBackground),
            _ => None,
        },
        _ => None,
    }
}

pub fn map_scroll(delta: MouseScrollDelta) -> Option<ViewerAction> {
    let y = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
    };
    if y > 0.1 {
        Some(ViewerAction::ZoomIn)
    } else if y < -0.1 {
        Some(ViewerAction::ZoomOut)
    } else {
        None
    }
}

#[derive(Clone, Copy)]
pub enum Background {
    Black,
    White,
    Checkerboard,
}

impl Background {
    pub fn next(self) -> Self {
        match self {
            Background::Black => Background::White,
            Background::White => Background::Checkerboard,
            Background::Checkerboard => Background::Black,
        }
    }

    pub fn clear_color(self) -> wgpu::Color {
        match self {
            Background::Black => wgpu::Color::BLACK,
            Background::White => wgpu::Color::WHITE,
            Background::Checkerboard => wgpu::Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 },
        }
    }
}
