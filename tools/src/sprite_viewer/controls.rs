use ragnarok_formats::act::SpriteActionType;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{quad_vertices, text_vertices};
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
    NextWeapon,
    PrevWeapon,
    ToggleSex,
    NextHead,
    PrevHead,
    NextHeadgear,
    PrevHeadgear,
    NextShield,
    PrevShield,
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
            "q" => Some(ViewerAction::NextWeapon),
            "w" => Some(ViewerAction::PrevWeapon),
            "s" | "S" => Some(ViewerAction::ToggleSex),
            "h" => Some(ViewerAction::NextHead),
            "g" => Some(ViewerAction::PrevHead),
            "e" => Some(ViewerAction::NextHeadgear),
            "r" => Some(ViewerAction::PrevHeadgear),
            "d" => Some(ViewerAction::NextShield),
            "f" => Some(ViewerAction::PrevShield),
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
            Background::Checkerboard => wgpu::Color {
                r: 0.5,
                g: 0.5,
                b: 0.5,
                a: 1.0,
            },
        }
    }
}

const LEGEND_ENTRIES: &[(&str, &str)] = &[
    ("Space", "Play/Pause"),
    (". / ,", "Step Fwd/Back"),
    ("Left / Right", "Direction"),
    ("Up / Down", "Action"),
    ("+ / -", "Zoom"),
    ("B", "Background"),
    ("S", "Sex"),
    ("H / G", "Head"),
    ("q / w", "Weapon"),
    ("E / R", "Headgear"),
    ("D / F", "Shield"),
    ("Tab", "Browser"),
];

const STATUS_PADDING: f32 = 8.0;
const STATUS_LINE_HEIGHT: f32 = 18.0;
const STATUS_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];
const STATUS_BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];

pub fn build_status_draw_calls(
    atlas: &FontAtlas,
    screen_w: f32,
    action: usize,
    direction: usize,
    motion: usize,
    motion_count: usize,
    paused: bool,
) -> Vec<UiDrawCall> {
    let action_name = SpriteActionType::from_index(action)
        .map(|a| a.name())
        .unwrap_or("?");
    let pause_str = if paused { " [paused]" } else { "" };
    let motion = motion + 1;
    let text = format!(
        "Act: {action} ({action_name})  Dir: {direction}  Frame: {motion}/{motion_count}{pause_str}"
    );

    let text_w = atlas.measure_text(&text);
    let box_w = text_w + STATUS_PADDING * 2.0;
    let box_h = STATUS_LINE_HEIGHT + STATUS_PADDING * 2.0;
    let box_x = (screen_w - box_w) / 2.0;
    let box_y = STATUS_PADDING;

    let mut calls = Vec::new();
    let (bg_verts, bg_idx) = quad_vertices(box_x, box_y, box_w, box_h, STATUS_BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bg_verts.to_vec(),
        indices: bg_idx.to_vec(),
        texture: UiTextureRef::White,
    });
    let (tv, ti) = text_vertices(
        &text,
        box_x + STATUS_PADDING,
        box_y + STATUS_PADDING,
        STATUS_COLOR,
        atlas,
    );
    calls.push(UiDrawCall {
        vertices: tv,
        indices: ti,
        texture: UiTextureRef::FontAtlas,
    });
    calls
}

const LEGEND_PADDING: f32 = 8.0;
const LEGEND_LINE_HEIGHT: f32 = 18.0;
const LEGEND_KEY_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];
const LEGEND_DESC_COLOR: [f32; 4] = [0.7, 0.7, 0.7, 0.7];
const LEGEND_BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];

pub fn build_legend_draw_calls(atlas: &FontAtlas, screen_h: f32) -> Vec<UiDrawCall> {
    let mut calls = Vec::new();
    let line_count = LEGEND_ENTRIES.len() as f32;
    let box_h = line_count * LEGEND_LINE_HEIGHT + LEGEND_PADDING * 2.0;
    let box_w = 200.0;
    let box_x = LEGEND_PADDING;
    let box_y = screen_h - box_h - LEGEND_PADDING;

    let (bg_verts, bg_idx) = quad_vertices(box_x, box_y, box_w, box_h, LEGEND_BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bg_verts.to_vec(),
        indices: bg_idx.to_vec(),
        texture: UiTextureRef::White,
    });

    let key_col_x = box_x + LEGEND_PADDING;
    let desc_col_x = box_x + 80.0;

    for (i, (key, desc)) in LEGEND_ENTRIES.iter().enumerate() {
        let y = box_y + LEGEND_PADDING + i as f32 * LEGEND_LINE_HEIGHT;
        let (kv, ki) = text_vertices(key, key_col_x, y, LEGEND_KEY_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: kv,
            indices: ki,
            texture: UiTextureRef::FontAtlas,
        });
        let (dv, di) = text_vertices(desc, desc_col_x, y, LEGEND_DESC_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: dv,
            indices: di,
            texture: UiTextureRef::FontAtlas,
        });
    }

    calls
}
