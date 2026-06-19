use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::UiFrame;
use ragnarok_ui::rect::Rect;

/// Draw a textured quad (GRF mode only - no fallback). Replaces inline draw_tex pattern.
pub fn draw_textured_quad(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, path: &str) {
    let (v, i) = draw::quad_vertices(x, y, w, h, [1.0; 4]);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::Named(path.to_string()),
    });
}

/// Draw an interactive system button.
/// GRF mode: renders on/off textures based on hover.
/// Non-GRF mode: renders colored quad with optional fallback character text.
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

pub fn text_color(has_grf: bool) -> [f32; 4] {
    if has_grf {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    }
}

/// Draw a window title bar.
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
