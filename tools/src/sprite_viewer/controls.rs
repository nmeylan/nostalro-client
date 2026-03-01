use winit::event::{ElementState, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

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

pub fn map_key_press(key: PhysicalKey, state: ElementState) -> Option<ViewerAction> {
    if state != ElementState::Pressed {
        return None;
    }
    match key {
        PhysicalKey::Code(KeyCode::ArrowRight) => Some(ViewerAction::NextDirection),
        PhysicalKey::Code(KeyCode::ArrowLeft) => Some(ViewerAction::PrevDirection),
        PhysicalKey::Code(KeyCode::ArrowUp) => Some(ViewerAction::PrevAction),
        PhysicalKey::Code(KeyCode::ArrowDown) => Some(ViewerAction::NextAction),
        PhysicalKey::Code(KeyCode::Space) => Some(ViewerAction::TogglePause),
        PhysicalKey::Code(KeyCode::Period) => Some(ViewerAction::StepForward),
        PhysicalKey::Code(KeyCode::Comma) => Some(ViewerAction::StepBackward),
        PhysicalKey::Code(KeyCode::Equal) => Some(ViewerAction::ZoomIn),
        PhysicalKey::Code(KeyCode::Minus) => Some(ViewerAction::ZoomIn),
        PhysicalKey::Code(KeyCode::KeyB) => Some(ViewerAction::CycleBackground),
        PhysicalKey::Code(KeyCode::Tab) => Some(ViewerAction::ToggleBrowser),
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
