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

    pub const BTN_BORDER: [f32; 4] = rgb(0xCF, 0xC7, 0xCB);

    pub const BTN_FACE_TOP: [f32; 4] = rgb(0xEE, 0xEA, 0xEC);
    pub const BTN_FACE_MID: [f32; 4] = rgb(0xDA, 0xD4, 0xD7);
    pub const BTN_FACE_BOT: [f32; 4] = rgb(0xF9, 0xF6, 0xF8);

    pub const BTN_HOVER_TOP: [f32; 4] = rgb(0xC6, 0xCF, 0xE4);
    pub const BTN_HOVER_MID: [f32; 4] = rgb(0x97, 0xAA, 0xD1);
    pub const BTN_HOVER_BOT: [f32; 4] = rgb(0xD4, 0xDE, 0xF1);
    pub const BTN_HOVER_BORDER: [f32; 4] = rgb(0x8E, 0xA6, 0xD2);

    pub const BTN_PRESS_TOP: [f32; 4] = rgb(0xAB, 0xBC, 0xE0);
    pub const BTN_PRESS_MID: [f32; 4] = rgb(0x83, 0x98, 0xC6);
    pub const BTN_PRESS_BOT: [f32; 4] = rgb(0xC4, 0xD3, 0xF3);

    pub const BTN_INSET_SHADOW_TOP: [f32; 4] = rgb(0xA9, 0xA2, 0xA5);
    pub const BTN_INSET_SHADOW_BOT: [f32; 4] = rgb(0xC4, 0xBD, 0xC0);
    pub const BTN_INSET_SHADOW_BLUE_TOP: [f32; 4] = rgb(0x88, 0x9C, 0xC6);
    pub const BTN_INSET_SHADOW_BLUE_BOT: [f32; 4] = rgb(0xA6, 0xBD, 0xE8);

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

/// Glossy rounded button that reads recessed: a `border` ring, then a dark inset
/// shadow groove around all four inner edges (darkest at the top, fading down),
/// then a concave top→mid→bottom glossy face, then a centered dark label.
pub fn fallback_button(ui: &mut UiFrame, r: Rect, hovered: bool, pressed: bool, label: &str) {
    let (top, mid, bot, border, sh_top, sh_bot) = if pressed {
        (
            FallbackPalette::BTN_PRESS_TOP,
            FallbackPalette::BTN_PRESS_MID,
            FallbackPalette::BTN_PRESS_BOT,
            FallbackPalette::BTN_HOVER_BORDER,
            FallbackPalette::BTN_INSET_SHADOW_BLUE_TOP,
            FallbackPalette::BTN_INSET_SHADOW_BLUE_BOT,
        )
    } else if hovered {
        (
            FallbackPalette::BTN_HOVER_TOP,
            FallbackPalette::BTN_HOVER_MID,
            FallbackPalette::BTN_HOVER_BOT,
            FallbackPalette::BTN_HOVER_BORDER,
            FallbackPalette::BTN_INSET_SHADOW_BLUE_TOP,
            FallbackPalette::BTN_INSET_SHADOW_BLUE_BOT,
        )
    } else {
        (
            FallbackPalette::BTN_FACE_TOP,
            FallbackPalette::BTN_FACE_MID,
            FallbackPalette::BTN_FACE_BOT,
            FallbackPalette::BTN_BORDER,
            FallbackPalette::BTN_INSET_SHADOW_TOP,
            FallbackPalette::BTN_INSET_SHADOW_BOT,
        )
    };
    let (v, i) = draw::rounded_rect(r.x, r.y, r.w, r.h, CORNER_RADIUS, border);
    push_white(ui, v, i);
    let (v, i) = draw::rounded_rect_vgrad(
        r.x + 1.0,
        r.y + 1.0,
        r.w - 2.0,
        r.h - 2.0,
        (CORNER_RADIUS - 1.0).max(0.0),
        sh_top,
        sh_bot,
    );
    push_white(ui, v, i);

    let (fx, fy, fw, fh) = (r.x + 2.0, r.y + 2.0, r.w - 4.0, r.h - 4.0);
    let fr = (CORNER_RADIUS - 2.0).max(0.0);
    let half = (fh * 0.5).round();
    let (v, i) = draw::rounded_rect_corners_vgrad(fx, fy, fw, half, [fr, fr, 0.0, 0.0], top, mid);
    push_white(ui, v, i);
    let (v, i) =
        draw::rounded_rect_corners_vgrad(fx, fy + half, fw, fh - half, [0.0, 0.0, fr, fr], mid, bot);
    push_white(ui, v, i);

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

/// Rounded panel with a concave glossy fill: `top`→`mid` over the upper half,
/// `mid`→`bot` over the lower half, ringed by a `border`-colored rounded rect.
pub fn fallback_glossy_panel(
    ui: &mut UiFrame,
    r: Rect,
    radius: f32,
    top: [f32; 4],
    mid: [f32; 4],
    bot: [f32; 4],
    border: [f32; 4],
) {
    let (v, i) = draw::rounded_rect(r.x, r.y, r.w, r.h, radius, border);
    push_white(ui, v, i);
    let ir = (radius - 1.0).max(0.0);
    let (x, y, w, h) = (r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0);
    let half = (h * 0.5).round();
    let (v, i) = draw::rounded_rect_corners_vgrad(x, y, w, half, [ir, ir, 0.0, 0.0], top, mid);
    push_white(ui, v, i);
    let (v, i) = draw::rounded_rect_corners_vgrad(x, y + half, w, h - half, [0.0, 0.0, ir, ir], mid, bot);
    push_white(ui, v, i);
}
