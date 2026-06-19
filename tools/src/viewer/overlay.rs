use ragnarok_renderer::Camera;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{quad_vertices, text_vertices};

use ragnarok_renderer::BackgroundMode;

const PADDING: f32 = 8.0;
const LINE_HEIGHT: f32 = 16.0;
const BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
const KEY_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.9];
const DESC_COLOR: [f32; 4] = [0.7, 0.7, 0.7, 0.85];
const STATUS_COLOR: [f32; 4] = [0.6, 0.85, 1.0, 0.9];

const LEGEND: &[(&str, &str)] = &[
    ("Tab", "Effect browser"),
    ("N / P", "Next / prev effect"),
    ("R", "Replay effect"),
    ("Space", "Pause"),
    ("B", "Bg: map/proxy/blue/black"),
    ("Left click", "Move character"),
    ("Right drag", "Orbit"),
    ("Scroll / +-", "Zoom"),
    ("C", "Reset camera"),
    ("Arrows", "Action / direction"),
    ("Q / W", "Weapon"),
    ("S", "Toggle sex"),
    ("h / H", "Head"),
    ("e / E", "Headgear"),
    ("D / F", "Shield"),
    ("T", "Place trail target"),
    ("X", "Clear trail target"),
];

pub fn build_legend(atlas: &FontAtlas, screen_w: f32, screen_h: f32) -> Vec<UiDrawCall> {
    let mut calls = Vec::new();
    let lines = LEGEND.len() as f32;
    let box_w = 240.0;
    let box_h = lines * LINE_HEIGHT + PADDING * 2.0;
    let box_x = screen_w - box_w - PADDING;
    let box_y = screen_h - box_h - PADDING;

    let (bv, bi) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bv.to_vec(),
        indices: bi.to_vec(),
        texture: UiTextureRef::White,
    });

    let key_col = box_x + PADDING;
    let desc_col = box_x + 100.0;
    for (i, (key, desc)) in LEGEND.iter().enumerate() {
        let y = box_y + PADDING + i as f32 * LINE_HEIGHT;
        let (kv, ki) = text_vertices(key, key_col, y, KEY_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: kv,
            indices: ki,
            texture: UiTextureRef::FontAtlas,
        });
        let (dv, di) = text_vertices(desc, desc_col, y, DESC_COLOR, atlas);
        calls.push(UiDrawCall {
            vertices: dv,
            indices: di,
            texture: UiTextureRef::FontAtlas,
        });
    }
    calls
}

pub struct StatusLine<'a> {
    pub map_name: &'a str,
    pub effect_label: &'a str,
    pub paused: bool,
    pub background: BackgroundMode,
    pub clear_is_black: bool,
    pub target_mode: bool,
    pub has_target: bool,
}

pub fn build_status(atlas: &FontAtlas, screen_w: f32, status: &StatusLine<'_>) -> Vec<UiDrawCall> {
    let bg_label = match status.background {
        BackgroundMode::RswMap => "map",
        BackgroundMode::GroundProxy => "proxy",
        BackgroundMode::Clear => {
            if status.clear_is_black {
                "black"
            } else {
                "blue"
            }
        }
    };
    let pause = if status.paused { " [PAUSED]" } else { "" };
    let target = if status.target_mode {
        " [TARGET MODE]"
    } else if status.has_target {
        " [TARGET SET]"
    } else {
        ""
    };
    let text = format!(
        "Map: {}  Effect: {}  Bg: {}{}{}",
        status.map_name, status.effect_label, bg_label, pause, target
    );
    let text_w = atlas.measure_text(&text);
    let box_w = text_w + PADDING * 2.0;
    let box_h = LINE_HEIGHT + PADDING * 2.0;
    let box_x = (screen_w - box_w) / 2.0;
    let box_y = PADDING;

    let mut calls = Vec::new();
    let (bv, bi) = quad_vertices(box_x, box_y, box_w, box_h, BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bv.to_vec(),
        indices: bi.to_vec(),
        texture: UiTextureRef::White,
    });
    let (tv, ti) = text_vertices(
        &text,
        box_x + PADDING,
        box_y + PADDING,
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

const CROSSHAIR_COLOR: [f32; 4] = [0.2, 1.0, 0.2, 0.9];
const CROSSHAIR_SIZE: f32 = 10.0;
const CROSSHAIR_THICK: f32 = 2.0;

pub fn build_target_crosshair(
    camera: &Camera,
    target: [f32; 3],
    screen_w: f32,
    screen_h: f32,
) -> Vec<UiDrawCall> {
    let Some((sx, sy)) = camera.world_to_screen(target[0], target[1], target[2], screen_w, screen_h) else {
        return Vec::new();
    };
    let mut calls = Vec::new();
    let (hv, hi) = quad_vertices(
        sx - CROSSHAIR_SIZE,
        sy - CROSSHAIR_THICK * 0.5,
        CROSSHAIR_SIZE * 2.0,
        CROSSHAIR_THICK,
        CROSSHAIR_COLOR,
    );
    calls.push(UiDrawCall {
        vertices: hv.to_vec(),
        indices: hi.to_vec(),
        texture: UiTextureRef::White,
    });
    let (vv, vi) = quad_vertices(
        sx - CROSSHAIR_THICK * 0.5,
        sy - CROSSHAIR_SIZE,
        CROSSHAIR_THICK,
        CROSSHAIR_SIZE * 2.0,
        CROSSHAIR_COLOR,
    );
    calls.push(UiDrawCall {
        vertices: vv.to_vec(),
        indices: vi.to_vec(),
        texture: UiTextureRef::White,
    });
    calls
}
