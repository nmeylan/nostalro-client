use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{quad_vertices, text_vertices};
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};

pub enum ViewerAction {
    TriggerScenario(Scenario),
    TogglePause,
    Restart,
    IncreaseValue,
    DecreaseValue,
    NextDirection,
    PrevDirection,
    SpeedUp,
    SpeedDown,
    CycleBackground,
}

#[derive(Clone, Copy)]
pub enum Scenario {
    NormalAttack,
    SkillAttack,
    CriticalHit,
    PlayerDamage,
    SkillMultiHit,
    NormalMultiHit,
    Heal,
    Miss,
    LuckyDodge,
    All,
}

pub fn map_key_press(key: &Key, state: ElementState) -> Option<ViewerAction> {
    if state != ElementState::Pressed {
        return None;
    }
    match key {
        Key::Named(NamedKey::Space) => Some(ViewerAction::TogglePause),
        Key::Named(NamedKey::ArrowUp) => Some(ViewerAction::IncreaseValue),
        Key::Named(NamedKey::ArrowDown) => Some(ViewerAction::DecreaseValue),
        Key::Named(NamedKey::ArrowRight) => Some(ViewerAction::NextDirection),
        Key::Named(NamedKey::ArrowLeft) => Some(ViewerAction::PrevDirection),
        Key::Character(ch) => match ch.as_str() {
            "1" => Some(ViewerAction::TriggerScenario(Scenario::NormalAttack)),
            "2" => Some(ViewerAction::TriggerScenario(Scenario::SkillAttack)),
            "3" => Some(ViewerAction::TriggerScenario(Scenario::CriticalHit)),
            "4" => Some(ViewerAction::TriggerScenario(Scenario::PlayerDamage)),
            "5" => Some(ViewerAction::TriggerScenario(Scenario::SkillMultiHit)),
            "6" => Some(ViewerAction::TriggerScenario(Scenario::NormalMultiHit)),
            "7" => Some(ViewerAction::TriggerScenario(Scenario::Heal)),
            "8" => Some(ViewerAction::TriggerScenario(Scenario::Miss)),
            "9" => Some(ViewerAction::TriggerScenario(Scenario::LuckyDodge)),
            "0" => Some(ViewerAction::TriggerScenario(Scenario::All)),
            "r" | "R" => Some(ViewerAction::Restart),
            "=" | "+" => Some(ViewerAction::SpeedUp),
            "-" => Some(ViewerAction::SpeedDown),
            "b" | "B" => Some(ViewerAction::CycleBackground),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub enum Background {
    Black,
    White,
    Gray,
}

impl Background {
    pub fn next(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Gray,
            Self::Gray => Self::Black,
        }
    }

    pub fn clear_color(self) -> wgpu::Color {
        match self {
            Self::Black => wgpu::Color::BLACK,
            Self::White => wgpu::Color::WHITE,
            Self::Gray => wgpu::Color {
                r: 0.3,
                g: 0.3,
                b: 0.3,
                a: 1.0,
            },
        }
    }
}

const LEGEND_ENTRIES: &[(&str, &str)] = &[
    ("1", "Normal Attack"),
    ("2", "Skill Attack"),
    ("3", "Critical Hit"),
    ("4", "Player Damage"),
    ("5", "Skill Multi-Hit"),
    ("6", "Normal Multi-Hit"),
    ("7", "Heal"),
    ("8", "Miss"),
    ("9", "Lucky Dodge"),
    ("0", "All Scenarios"),
    ("", ""),
    ("Space", "Pause / Resume"),
    ("R", "Restart"),
    ("Up/Down", "Damage Value"),
    ("Left/Right", "Direction"),
    ("+/-", "Speed"),
    ("B", "Background"),
];

const PADDING: f32 = 8.0;
const LINE_HEIGHT: f32 = 16.0;
const KEY_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];
const DESC_COLOR: [f32; 4] = [0.7, 0.7, 0.7, 0.7];
const BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
const STATUS_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];

pub fn build_legend_draw_calls(atlas: &FontAtlas, screen_h: f32) -> Vec<UiDrawCall> {
    let mut calls = Vec::new();
    let line_count = LEGEND_ENTRIES.len() as f32;
    let box_h = line_count * LINE_HEIGHT + PADDING * 2.0;
    let box_w = 200.0;
    let box_x = PADDING;
    let box_y = screen_h - box_h - PADDING;

    let (bg_verts, bg_idx) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bg_verts.to_vec(),
        indices: bg_idx.to_vec(),
        texture: UiTextureRef::White,
    });

    let key_col_x = box_x + PADDING;
    let desc_col_x = box_x + 70.0;

    for (i, (key, desc)) in LEGEND_ENTRIES.iter().enumerate() {
        if key.is_empty() {
            continue;
        }
        let y = box_y + PADDING + i as f32 * LINE_HEIGHT;
        let (kv, ki) = text_vertices(key, key_col_x, y, KEY_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: kv,
            indices: ki,
            texture: UiTextureRef::FontAtlas,
        });
        let (dv, di) = text_vertices(desc, desc_col_x, y, DESC_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: dv,
            indices: di,
            texture: UiTextureRef::FontAtlas,
        });
    }

    calls
}

pub fn build_status_draw_calls(
    atlas: &FontAtlas,
    screen_w: f32,
    damage_value: i32,
    direction: u8,
    speed: f32,
    paused: bool,
) -> Vec<UiDrawCall> {
    let pause_str = if paused { " [PAUSED]" } else { "" };
    let text = format!(
        "Damage: {}  Dir: {}  Speed: {:.1}x{}",
        damage_value, direction, speed, pause_str
    );

    let text_w = atlas.measure_text(&text);
    let box_w = text_w + PADDING * 2.0;
    let box_h = LINE_HEIGHT + PADDING * 2.0;
    let box_x = (screen_w - box_w) / 2.0;
    let box_y = PADDING;

    let mut calls = Vec::new();
    let (bg_verts, bg_idx) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bg_verts.to_vec(),
        indices: bg_idx.to_vec(),
        texture: UiTextureRef::White,
    });
    let (tv, ti) = text_vertices(&text, box_x + PADDING, box_y + PADDING, STATUS_COLOR, atlas);
    calls.push(UiDrawCall {
        vertices: tv,
        indices: ti,
        texture: UiTextureRef::FontAtlas,
    });
    calls
}
