use crate::draw::{self, DrawCall, TextureRef};
use crate::frame::UiFrame;
use crate::rect::Rect;
use ragnarok_renderer::ui_renderer::UiVertex;

const fn rgb(r: u8, g: u8, b: u8) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

/// Palette reproducing the light, glossy, blue-tinted `basic_interface` look for
/// the no-GRF fallback path. Values sampled from the real interface BMPs.
pub struct FallbackPalette;

impl FallbackPalette {
    pub const WIN_BODY: [f32; 4] = rgb(0xFA, 0xFB, 0xFF);
    pub const WIN_BODY_ALT: [f32; 4] = rgb(0xF2, 0xF2, 0xF2);
    pub const WIN_BORDER: [f32; 4] = rgb(0xC2, 0xC2, 0xC2);
    pub const WIN_BORDER_HI: [f32; 4] = rgb(0xFF, 0xFF, 0xFF);

    pub const TITLE_TOP: [f32; 4] = rgb(0x82, 0x94, 0xC8);
    pub const TITLE_BOT: [f32; 4] = rgb(0xD2, 0xEA, 0xFC);
    pub const TITLE_BASELINE: [f32; 4] = rgb(0x00, 0x00, 0x00);

    pub const SLOT_FILL: [f32; 4] = rgb(0xFF, 0xFF, 0xFF);
    pub const SLOT_INSET: [f32; 4] = rgb(0xCF, 0xD6, 0xE5);

    pub const BTN_FACE_TOP: [f32; 4] = rgb(0xDC, 0xDC, 0xDC);
    pub const BTN_FACE_BOT: [f32; 4] = rgb(0xFA, 0xFA, 0xFA);
    pub const BTN_BEVEL_HI: [f32; 4] = rgb(0xAE, 0xAE, 0xAE);
    pub const BTN_BEVEL_LO: [f32; 4] = rgb(0x6E, 0x6E, 0x6E);
    pub const BTN_HOVER_TOP: [f32; 4] = rgb(0xC6, 0xCD, 0xDB);
    pub const BTN_HOVER_BOT: [f32; 4] = rgb(0xDA, 0xE4, 0xF7);
    pub const BTN_PRESS_TOP: [f32; 4] = rgb(0xCB, 0xDC, 0xFE);
    pub const BTN_PRESS_BOT: [f32; 4] = rgb(0xE7, 0xF0, 0xFF);

    pub const SYS_BTN: [f32; 4] = rgb(0x3E, 0x6B, 0xC7);
    pub const SYS_BTN_HOVER: [f32; 4] = rgb(0x57, 0x96, 0xFE);
    pub const SYS_GLYPH: [f32; 4] = rgb(0x09, 0x23, 0x5A);

    pub const TEXT_ON_LIGHT: [f32; 4] = rgb(0x20, 0x20, 0x20);
}

pub const CORNER_RADIUS: f32 = 3.0;

fn push_white(ui: &mut UiFrame, verts: Vec<UiVertex>, indices: Vec<u32>) {
    ui.draw_calls.push(DrawCall {
        vertices: verts,
        indices,
        texture: TextureRef::White,
    });
}

/// 1px raised bevel: `hi` on top+left edges, `lo` on bottom+right. Edge segments
/// are inset by `radius` so they stay inside the rounded silhouette.
pub fn bevel(ui: &mut UiFrame, r: Rect, radius: f32, hi: [f32; 4], lo: [f32; 4]) {
    let rad = radius.max(0.0);
    let seg = |ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]| {
        if w > 0.0 && h > 0.0 {
            let (v, i) = draw::quad_vertices(x, y, w, h, c);
            push_white(ui, v.to_vec(), i.to_vec());
        }
    };
    seg(ui, r.x + rad, r.y, r.w - 2.0 * rad, 1.0, hi);
    seg(ui, r.x, r.y + rad, 1.0, r.h - 2.0 * rad, hi);
    seg(ui, r.x + rad, r.y + r.h - 1.0, r.w - 2.0 * rad, 1.0, lo);
    seg(ui, r.x + r.w - 1.0, r.y + rad, 1.0, r.h - 2.0 * rad, lo);
}

/// Light glossy rounded button with a raised bevel (sunken when pressed) and a
/// centered dark label.
pub fn fallback_button(ui: &mut UiFrame, r: Rect, hovered: bool, pressed: bool, label: &str) {
    let (face_top, face_bot) = if pressed {
        (FallbackPalette::BTN_PRESS_TOP, FallbackPalette::BTN_PRESS_BOT)
    } else if hovered {
        (FallbackPalette::BTN_HOVER_TOP, FallbackPalette::BTN_HOVER_BOT)
    } else {
        (FallbackPalette::BTN_FACE_TOP, FallbackPalette::BTN_FACE_BOT)
    };
    fallback_panel(ui, r, CORNER_RADIUS, face_top, face_bot, FallbackPalette::WIN_BORDER);
    let (hi, lo) = if pressed {
        (FallbackPalette::BTN_BEVEL_LO, FallbackPalette::BTN_BEVEL_HI)
    } else {
        (FallbackPalette::BTN_BEVEL_HI, FallbackPalette::BTN_BEVEL_LO)
    };
    bevel(ui, r, CORNER_RADIUS, hi, lo);

    if !label.is_empty() {
        let tw = ui.atlas.measure_text(label);
        let tx = r.x + (r.w - tw) / 2.0;
        let ty = r.y + r.h - (ui.atlas.line_height / 2.0);
        let (v, i) = draw::text_vertices(label, tx, ty, FallbackPalette::TEXT_ON_LIGHT, ui.atlas);
        if !v.is_empty() {
            ui.draw_calls.push(DrawCall {
                vertices: v,
                indices: i,
                texture: TextureRef::FontAtlas,
            });
        }
    }
}

/// Light rounded text field; border tints blue while focused.
pub fn fallback_text_input(ui: &mut UiFrame, r: Rect, has_focus: bool) {
    let border = if has_focus {
        FallbackPalette::TITLE_TOP
    } else {
        FallbackPalette::WIN_BORDER
    };
    fallback_panel(
        ui,
        r,
        CORNER_RADIUS,
        FallbackPalette::SLOT_FILL,
        FallbackPalette::SLOT_FILL,
        border,
    );
}

/// Rounded panel: a border-colored rounded rect with a 1px-inset gradient fill.
pub fn fallback_panel(
    ui: &mut UiFrame,
    r: Rect,
    radius: f32,
    fill_top: [f32; 4],
    fill_bot: [f32; 4],
    border: [f32; 4],
) {
    let (v, i) = draw::rounded_rect(r.x, r.y, r.w, r.h, radius, border);
    push_white(ui, v, i);
    let (v, i) = draw::rounded_rect_vgrad(
        r.x + 1.0,
        r.y + 1.0,
        r.w - 2.0,
        r.h - 2.0,
        (radius - 1.0).max(0.0),
        fill_top,
        fill_bot,
    );
    push_white(ui, v, i);
}
