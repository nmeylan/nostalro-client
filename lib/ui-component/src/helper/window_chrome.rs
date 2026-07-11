use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::UiFrame;
use ragnarok_ui::rect::Rect;

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
    hover_color: [f32; 4],
    normal_color: [f32; 4],
) {
    if has_grf {
        let tex = if hovered { on_tex } else { off_tex };
        draw_textured_quad(ui, rect.x, rect.y, size.0, size.1, tex);
    } else {
        let c = if hovered { hover_color } else { normal_color };
        let (v, i) = draw::quad_vertices(rect.x, rect.y, size.0, size.1, c);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        if let Some(ch) = fallback_char {
            ui.text(rect.x + 2.0, rect.y + size.1 - 1.0, &ch.to_string(), c);
        }
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
        let (v, i) = draw::quad_vertices(x, y, w, h, [0.05, 0.05, 0.07, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        if pct > 0.0 {
            let fill = if is_red {
                [0.85, 0.2, 0.2, 1.0]
            } else {
                [0.25, 0.45, 0.9, 1.0]
            };
            let (v, i) = draw::quad_vertices(x, y, w * pct, h, fill);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
    }
}

pub fn text_color(has_grf: bool) -> [f32; 4] {
    if has_grf {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [1.0, 1.0, 1.0, 1.0]
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
        let (v, i) = draw::quad_vertices(x, y, w, h, [0.20, 0.20, 0.30, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let bc = [0.5, 0.5, 0.6, 1.0];
        for (bx, by, bw, bh) in [(x, y, w, 1.0), (x, y, 1.0, h), (x + w - 1.0, y, 1.0, h)] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
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
        let (v, i) = draw::quad_vertices(x, y, w, h, [0.12, 0.12, 0.18, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let bc = [0.4, 0.4, 0.5, 1.0];
        for (bx, by, bw, bh) in [(x, y, 1.0, h), (x + w - 1.0, y, 1.0, h)] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
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
        let (v, i) = draw::quad_vertices(x, y, w, h, [0.18, 0.18, 0.25, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let bc = [0.5, 0.5, 0.6, 1.0];
        for (bx, by, bw, bh) in [
            (x, y + h - 1.0, w, 1.0),
            (x, y, 1.0, h),
            (x + w - 1.0, y, 1.0, h),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
    }
}
