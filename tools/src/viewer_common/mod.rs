pub mod shader_watcher;

use ragnarok_renderer::Camera;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::{UiDrawCall, UiTextureRef};
use ragnarok_ui::draw::{quad_vertices, text_vertices};

const FPS_PADDING: f32 = 8.0;
const FPS_LINE_HEIGHT: f32 = 16.0;
const FPS_BG_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
const FPS_COLOR: [f32; 4] = [0.6, 1.0, 0.6, 0.95];

/// Top-left readout: frames-per-second and the live effect count. Shared by
/// both viewers (and reusable by the game client).
pub fn build_fps(atlas: &FontAtlas, fps: f32, live_effects: usize) -> Vec<UiDrawCall> {
    let text = format!("FPS: {:.0}   effects: {}", fps, live_effects);
    let text_w = atlas.measure_text(&text);
    let box_w = text_w + FPS_PADDING * 2.0;
    let box_h = FPS_LINE_HEIGHT + FPS_PADDING * 2.0;
    let mut calls = Vec::new();
    let (bv, bi) = quad_vertices(FPS_PADDING, FPS_PADDING, box_w, box_h, FPS_BG_COLOR);
    calls.push(UiDrawCall {
        vertices: bv.to_vec(),
        indices: bi.to_vec(),
        texture: UiTextureRef::White,
    });
    let (tv, ti) = text_vertices(&text, FPS_PADDING * 2.0, FPS_PADDING * 2.0, FPS_COLOR, atlas);
    calls.push(UiDrawCall {
        vertices: tv,
        indices: ti,
        texture: UiTextureRef::FontAtlas,
    });
    calls
}

/// Intersect the camera ray through a screen pixel with the horizontal plane
/// `y = plane_y`, returning the world hit point. `None` when the ray is
/// parallel to the plane or points away from it (e.g. toward the sky).
pub fn screen_to_ground(
    camera: &Camera,
    screen_x: f32,
    screen_y: f32,
    screen_w: f32,
    screen_h: f32,
    plane_y: f32,
) -> Option<[f32; 3]> {
    let (origin, dir) = camera.screen_to_ray(screen_x, screen_y, screen_w, screen_h);
    if dir.y.abs() < 1e-6 {
        return None;
    }
    let t = (plane_y - origin.y) / dir.y;
    if t < 0.0 {
        return None;
    }
    let hit = origin + dir * t;
    Some([hit.x, plane_y, hit.z])
}
