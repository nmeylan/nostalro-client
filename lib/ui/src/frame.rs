use crate::context::UiContext;
use crate::draw::{self, DrawCall, TextureRef};
use crate::rect::Rect;
use crate::state::StateCache;
use crate::text_input::TextInput;
use ragnarok_renderer::font_atlas::FontAtlas;

pub struct UiFrame<'a> {
    pub ctx: &'a UiContext,
    pub atlas: &'a FontAtlas,
    pub state: &'a mut StateCache,
    pub elapsed_secs: f32,
    pub has_grf_textures: bool,
    pub draw_calls: Vec<DrawCall>,
    focus: Option<WidgetId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u32);

pub struct ButtonTextures {
    pub normal: &'static str,
    pub hover: &'static str,
    pub pressed: &'static str,
}

pub struct ButtonResponse {
    clicked: bool,
}

impl ButtonResponse {
    pub fn clicked(&self) -> bool {
        self.clicked
    }
}

pub struct TextInputResponse {
    submitted: bool,
}

impl TextInputResponse {
    pub fn submitted(&self) -> bool {
        self.submitted
    }
}

impl<'a> UiFrame<'a> {
    pub fn new(
        ctx: &'a UiContext,
        atlas: &'a FontAtlas,
        state: &'a mut StateCache,
        elapsed_secs: f32,
        has_grf_textures: bool,
        initial_focus: Option<WidgetId>,
    ) -> Self {
        Self {
            ctx,
            atlas,
            state,
            elapsed_secs,
            has_grf_textures,
            draw_calls: Vec::new(),
            focus: initial_focus,
        }
    }

    pub fn button(
        &mut self, _id: WidgetId, rect: Rect, textures: &ButtonTextures, fallback_label: &str,
    ) -> ButtonResponse {
        let hovered = rect.contains(self.ctx.mouse_x, self.ctx.mouse_y);
        let pressed = hovered && (self.ctx.mouse_clicked || self.ctx.mouse_down);
        let clicked = hovered && self.ctx.mouse_clicked;

        if self.has_grf_textures {
            let tex = if pressed {
                textures.pressed
            } else if hovered {
                textures.hover
            } else {
                textures.normal
            };
            let (verts, indices) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
            self.draw_calls.push(DrawCall {
                vertices: verts.to_vec(),
                indices: indices.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            let bg_color = if pressed {
                [0.15, 0.15, 0.25, 1.0]
            } else if hovered {
                [0.35, 0.35, 0.5, 1.0]
            } else {
                [0.25, 0.25, 0.35, 1.0]
            };
            let (v, i) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, bg_color);
            self.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });

            let b = 1.0;
            let bc = [0.5, 0.5, 0.6, 1.0];
            for (bx, by, bw, bh) in [
                (rect.x, rect.y, rect.w, b),
                (rect.x, rect.y + rect.h - b, rect.w, b),
                (rect.x, rect.y, b, rect.h),
                (rect.x + rect.w - b, rect.y, b, rect.h),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
                self.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }

            let tw = self.atlas.measure_text(fallback_label);
            let tx = rect.x + (rect.w - tw) / 2.0;
            let ty = rect.y + (rect.h - self.atlas.line_height) / 2.0;
            let (v, i) = draw::text_vertices(fallback_label, tx, ty, [1.0, 1.0, 1.0, 1.0], self.atlas);
            if !v.is_empty() {
                self.draw_calls.push(DrawCall { vertices: v, indices: i, texture: TextureRef::FontAtlas });
            }
        }

        ButtonResponse { clicked }
    }

    pub fn text_input(
        &mut self, id: WidgetId, rect: Rect, state: &mut TextInput, bg_texture: Option<&str>,
    ) -> TextInputResponse {
        let clicked_inside = self.ctx.mouse_clicked && rect.contains(self.ctx.mouse_x, self.ctx.mouse_y);
        if clicked_inside {
            self.focus = Some(id);
        }

        let is_focused = self.focus == Some(id);

        if is_focused {
            state.process_keys(self.ctx);
        }

        if clicked_inside {
            let text = state.display_text();
            let padding = 4.0;
            let available_w = rect.w - padding * 2.0;
            let cur_text = &text[..state.display_cursor_offset()];
            let scroll = (self.atlas.measure_text(cur_text) - available_w).max(0.0);
            let click_rel = self.ctx.mouse_x - (rect.x + padding) + scroll;
            let mut acc = 0.0;
            let mut best_pos = 0;
            for (i, ch) in text.chars().enumerate() {
                let advance = self.atlas.glyph(ch).advance;
                if click_rel < acc + advance * 0.5 {
                    break;
                }
                acc += advance;
                best_pos = i + 1;
            }
            state.cursor_pos = best_pos;
        }

        // Background
        if let Some(tex_name) = bg_texture {
            let (verts, indices) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
            self.draw_calls.push(DrawCall {
                vertices: verts.to_vec(),
                indices: indices.to_vec(),
                texture: TextureRef::Named(tex_name.to_string()),
            });
        } else {
            let bg_color = if is_focused {
                [0.15, 0.15, 0.2, 1.0]
            } else {
                [0.1, 0.1, 0.15, 1.0]
            };
            let (verts, indices) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, bg_color);
            self.draw_calls.push(DrawCall {
                vertices: verts.to_vec(),
                indices: indices.to_vec(),
                texture: TextureRef::White,
            });

            let border_color = if is_focused {
                [0.5, 0.5, 0.7, 1.0]
            } else {
                [0.3, 0.3, 0.4, 1.0]
            };
            let border = 1.0;
            for (bx, by, bw, bh) in [
                (rect.x, rect.y, rect.w, border),
                (rect.x, rect.y + rect.h - border, rect.w, border),
                (rect.x, rect.y, border, rect.h),
                (rect.x + rect.w - border, rect.y, border, rect.h),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
                self.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
        }

        // Text
        let text = state.display_text();
        let padding = 4.0;
        let available_w = rect.w - padding * 2.0;
        let text_y = rect.y + self.atlas.line_height;

        // Compute offset so cursor is always visible within the field
        let cursor_text = &text[..state.display_cursor_offset()];
        let cursor_px = self.atlas.measure_text(cursor_text);
        let scroll = (cursor_px - available_w).max(0.0);
        let text_x = rect.x + padding - scroll;

        let clip_left = rect.x + padding;
        let clip_right = rect.x + rect.w - padding;

        if !text.is_empty() {
            let text_color = if bg_texture.is_some() { [0.0, 0.0, 0.0, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
            let (verts, indices) = draw::text_vertices_clipped(
                &text, text_x, text_y, text_color, self.atlas, clip_left, clip_right,
            );
            if !verts.is_empty() {
                self.draw_calls.push(DrawCall {
                    vertices: verts,
                    indices,
                    texture: TextureRef::FontAtlas,
                });
            }
        }

        // Cursor blink
        if is_focused && (self.elapsed_secs % 1.0) < 0.5 {
            let cursor_x = (text_x + cursor_px).clamp(clip_left, clip_right);
            let caret_y = rect.y + (rect.h - self.atlas.ascent) / 2.0;
            let caret_color = if bg_texture.is_some() { [0.0, 0.0, 0.0, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
            let (v, i) = draw::quad_vertices(cursor_x, caret_y, 1.0, self.atlas.ascent, caret_color);
            self.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        let submitted = is_focused && self.ctx.key_enter;
        TextInputResponse { submitted }
    }

    pub fn text(&mut self, x: f32, y: f32, content: &str, color: [f32; 4]) {
        let (v, i) = draw::text_vertices(content, x, y, color, self.atlas);
        if !v.is_empty() {
            self.draw_calls.push(DrawCall { vertices: v, indices: i, texture: TextureRef::FontAtlas });
        }
    }

    pub fn set_focus(&mut self, id: WidgetId) {
        self.focus = Some(id);
    }

    pub fn focused(&self) -> Option<WidgetId> {
        self.focus
    }
}
