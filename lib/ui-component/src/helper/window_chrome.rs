use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::UiFrame;
use ragnarok_ui::rect::Rect;

fn push_quad(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let (v, i) = draw::quad_vertices(x, y, w, h, color);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
}

pub fn draw_textured_quad(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, path: &str) {
    let (v, i) = draw::quad_vertices(x, y, w, h, [1.0; 4]);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::Named(path.to_string()),
    });
}

pub fn draw_sys_button(
    ui: &mut UiFrame,
    rect: Rect,
    size: (f32, f32),
    hovered: bool,
    has_grf: bool,
    on_tex: &str,
    off_tex: &str,
    fallback_char: Option<char>,
) {
    if has_grf {
        let tex = if hovered { on_tex } else { off_tex };
        draw_textured_quad(ui, rect.x, rect.y, size.0, size.1, tex);
    } else {
        crate::helper::fallback::sys_button(ui, rect.x, rect.y, size.0, hovered, fallback_char);
    }
}

pub const TITLEBAR_TEX: &str = "data/texture/유저인터페이스/basic_interface/titlebar_mid.bmp";
pub const ITEMWIN_MID_TEX: &str = "data/texture/유저인터페이스/basic_interface/itemwin_mid.bmp";
pub const FOOTER_TEX: &str = "data/texture/유저인터페이스/basic_interface/btnbar_mid2.bmp";
pub const SYS_BASE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_base_off.bmp";
pub const SYS_BASE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_base_on.bmp";

pub const GZE_RED_LEFT: &str = "data/texture/유저인터페이스/basic_interface/gzered_left.bmp";
pub const GZE_RED_MID: &str = "data/texture/유저인터페이스/basic_interface/gzered_mid.bmp";
pub const GZE_RED_RIGHT: &str = "data/texture/유저인터페이스/basic_interface/gzered_right.bmp";
pub const GZE_BLUE_LEFT: &str = "data/texture/유저인터페이스/basic_interface/gzeblue_left.bmp";
pub const GZE_BLUE_MID: &str = "data/texture/유저인터페이스/basic_interface/gzeblue_mid.bmp";
pub const GZE_BLUE_RIGHT: &str = "data/texture/유저인터페이스/basic_interface/gzeblue_right.bmp";

pub fn gauge_texture_paths() -> Vec<&'static str> {
    vec![
        GZE_RED_LEFT,
        GZE_RED_MID,
        GZE_RED_RIGHT,
        GZE_BLUE_LEFT,
        GZE_BLUE_MID,
        GZE_BLUE_RIGHT,
    ]
}

/// Draws a 3-slice HP/SP gauge (red or blue) with fixed-width caps and a
/// stretched middle. Falls back to a flat fill bar when GRF textures are absent.
#[allow(clippy::too_many_arguments)]
pub fn draw_gauge(
    ui: &mut UiFrame,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    cap_w: f32,
    fill_pct: f32,
    is_red: bool,
    has_grf: bool,
) {
    let pct = fill_pct.clamp(0.0, 1.0);
    if has_grf {
        let (left, mid, right) = if is_red {
            (GZE_RED_LEFT, GZE_RED_MID, GZE_RED_RIGHT)
        } else {
            (GZE_BLUE_LEFT, GZE_BLUE_MID, GZE_BLUE_RIGHT)
        };
        let white = [1.0; 4];
        let mid_max = (w - cap_w * 2.0).max(0.0);
        let filled = (pct * w).max(0.0);
        if pct <= 0.0 {
            return;
        }
        let (v, i) = draw::quad_vertices(x, y, cap_w, h, white);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(left.to_string()),
        });
        let mid_w = (filled - cap_w * 2.0).clamp(0.0, mid_max);
        if mid_w > 0.0 {
            let (v, i) = draw::quad_vertices(x + cap_w, y, mid_w, h, white);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(mid.to_string()),
            });
        }
        let (v, i) = draw::quad_vertices(x + cap_w + mid_w, y, cap_w, h, white);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(right.to_string()),
        });
    } else {
        crate::helper::fallback::gauge(ui, x, y, w, h, pct, is_red);
    }
}

pub fn text_color(_has_grf: bool) -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}


/// Dark navy blue used for field labels (Name/HP/SP/stat names) in info windows.
pub fn label_color(has_grf: bool) -> [f32; 4] {
    if has_grf {
        [0.14, 0.20, 0.38, 1.0]
    } else {
        [0.45, 0.65, 1.0, 1.0]
    }
}

pub fn draw_hline(ui: &mut UiFrame, x: f32, y: f32, w: f32) {
    push_quad(ui, x, y, w, 1.0, [0.7, 0.7, 0.72, 1.0]);
}

/// Thin flat EXP-style bar (gray border, white fill area, blue fill) matching the
/// character info window's exp gauge.
pub fn draw_exp_bar(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, fill_pct: f32, has_grf: bool) {
    if has_grf {
        push_quad(ui, x, y, w + 2.0, h + 2.0, [0.69, 0.69, 0.69, 1.0]);
        push_quad(ui, x + 1.0, y + 1.0, w, h, [1.0; 4]);
    } else {
        push_quad(ui, x, y, w + 2.0, h + 2.0, [0.3, 0.3, 0.35, 0.9]);
    }
    if fill_pct > 0.0 {
        let fw = (w * fill_pct.clamp(0.0, 1.0)).floor();
        push_quad(ui, x + 1.0, y + 1.0, fw, h, [0.26, 0.38, 0.65, 1.0]);
    }
}

pub fn draw_titlebar(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, has_grf: bool) {
    if has_grf {
        let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(TITLEBAR_TEX.to_string()),
        });
        let btn_size = 11.0;
        let btn_x = x + 4.0;
        let btn_y = y + 3.0;
        let tex = if Rect::new(btn_x, btn_y, btn_size, btn_size)
            .contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
        {
            SYS_BASE_ON_TEX
        } else {
            SYS_BASE_OFF_TEX
        };
        let (v, i) = draw::quad_vertices(btn_x, btn_y, btn_size, btn_size, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(tex.to_string()),
        });
    } else {
        crate::helper::fallback::titlebar(ui, x, y, w, h);
    }
}

pub fn draw_container(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, has_grf: bool) {
    if has_grf {
        let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let (v, i) = draw::quad_vertices(x + w - 1.0, y, 1.0, h, [0.8, 0.8, 0.8, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    } else {
        crate::helper::fallback::container(ui, x, y, w, h);
    }
}

pub fn draw_footer(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, has_grf: bool) {
    if has_grf {
        let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(FOOTER_TEX.to_string()),
        });
    } else {
        crate::helper::fallback::footer(ui, x, y, w, h);
    }
}
