use crate::context::UiContext;
use crate::draw::{self, DrawCall, TextureRef};
use crate::rect::Rect;
use crate::state::StateCache;
use crate::text_input::TextInput;
use ragnarok_renderer::font_atlas::FontAtlas;

#[derive(Clone, Copy)]
pub enum TextInputBg<'a> {
    Default,
    Texture(&'a str),
    /// No background drawn; dark text for use over an externally-drawn light bg
    Transparent,
}

pub struct UiFrame<'a> {
    pub ctx: &'a UiContext,
    pub atlas: &'a FontAtlas,
    pub state: &'a mut StateCache,
    pub elapsed_secs: f32,
    pub has_grf_textures: bool,
    pub draw_calls: Vec<DrawCall>,
    pub any_hovered: bool,
    focus: Option<WidgetId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u32);

#[derive(Default)]
pub struct WindowState {
    pub x: f32,
    pub y: f32,
    initialized: bool,
    dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
}

pub struct ButtonTextures {
    pub normal: &'static str,
    pub hover: &'static str,
    pub pressed: &'static str,
}

pub struct Response {
    clicked: bool,
    hovered: bool,
    has_focus: bool,
}

impl Response {
    pub fn clicked(&self) -> bool { self.clicked }
    pub fn hovered(&self) -> bool { self.hovered }
    pub fn has_focus(&self) -> bool { self.has_focus }
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
            any_hovered: false,
            focus: initial_focus,
        }
    }

    pub fn window(&mut self, id: WidgetId, w: f32, h: f32, title_bar_h: f32) -> Rect {
        let state = self.state.get_or_default::<WindowState>(id);
        if !state.initialized {
            state.x = ((self.ctx.screen_width - w) / 2.0).floor();
            state.y = ((self.ctx.screen_height - h) / 2.0).floor();
            state.initialized = true;
        }

        let title_bar = Rect::new(state.x, state.y, w, title_bar_h);

        if self.ctx.mouse_clicked && title_bar.contains(self.ctx.mouse_x, self.ctx.mouse_y) {
            state.dragging = true;
            state.drag_offset_x = self.ctx.mouse_x - state.x;
            state.drag_offset_y = self.ctx.mouse_y - state.y;
        }

        if state.dragging {
            if self.ctx.mouse_down {
                state.x = self.ctx.mouse_x - state.drag_offset_x;
                state.y = self.ctx.mouse_y - state.drag_offset_y;
            } else {
                state.dragging = false;
            }
        }

        Rect::new(state.x, state.y, w, h)
    }

    pub fn interact(&mut self, id: WidgetId, rect: Rect) -> Response {
        let hovered = rect.contains(self.ctx.mouse_x, self.ctx.mouse_y);
        if hovered {
            self.any_hovered = true;
        }
        let clicked = hovered && self.ctx.mouse_clicked;
        if clicked {
            self.focus = Some(id);
        }
        let has_focus = self.focus == Some(id);
        Response { clicked, hovered, has_focus }
    }

    pub fn button(
        &mut self, id: WidgetId, rect: Rect, textures: &ButtonTextures, fallback_label: &str,
    ) -> Response {
        let response = self.interact(id, rect);
        let pressed = response.hovered && (self.ctx.mouse_clicked || self.ctx.mouse_down);

        if self.has_grf_textures {
            let tex = if pressed {
                textures.pressed
            } else if response.hovered {
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
            } else if response.hovered {
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
            let ty = rect.y + rect.h - (self.atlas.line_height / 2.0);
            let (v, i) = draw::text_vertices(fallback_label, tx, ty, [1.0, 1.0, 1.0, 1.0], self.atlas);
            if !v.is_empty() {
                self.draw_calls.push(DrawCall { vertices: v, indices: i, texture: TextureRef::FontAtlas });
            }
        }

        response
    }

    pub fn text_input(
        &mut self, id: WidgetId, rect: Rect, state: &mut TextInput, bg: TextInputBg,
    ) -> Response {
        let response = self.interact(id, rect);

        if response.has_focus {
            state.process_keys(self.ctx);
        }

        if response.clicked {
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
        let dark_text = !matches!(bg, TextInputBg::Default);
        match bg {
            TextInputBg::Texture(tex_name) => {
                let (verts, indices) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
                self.draw_calls.push(DrawCall {
                    vertices: verts.to_vec(),
                    indices: indices.to_vec(),
                    texture: TextureRef::Named(tex_name.to_string()),
                });
            }
            TextInputBg::Default => {
                let bg_color = if response.has_focus {
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

                let border_color = if response.has_focus {
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
            TextInputBg::Transparent => {}
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
            let text_color = if dark_text { [0.0, 0.0, 0.0, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
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
        if response.has_focus && (self.elapsed_secs % 1.0) < 0.5 {
            let cursor_x = (text_x + cursor_px).clamp(clip_left, clip_right);
            let caret_y = rect.y + (rect.h - self.atlas.ascent) / 2.0;
            let caret_color = if dark_text { [0.0, 0.0, 0.0, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
            let (v, i) = draw::quad_vertices(cursor_x, caret_y, 1.0, self.atlas.ascent, caret_color);
            self.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        response
    }

    pub fn text(&mut self, x: f32, y: f32, content: &str, color: [f32; 4]) {
        let (v, i) = draw::text_vertices(content, x, y, color, self.atlas);
        if !v.is_empty() {
            self.draw_calls.push(DrawCall { vertices: v, indices: i, texture: TextureRef::FontAtlas });
        }
    }

    pub fn colored_text(&mut self, x: f32, y: f32, content: &str, default_color: [f32; 4]) {
        let (v, i) = draw::colored_text_vertices(content, x, y, default_color, self.atlas);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::UiContext;
    use crate::state::StateCache;
    use ragnarok_renderer::font_atlas::FontAtlas;

    fn make_frame<'a>(ctx: &'a UiContext, atlas: &'a FontAtlas, state: &'a mut StateCache) -> UiFrame<'a> {
        UiFrame::new(ctx, atlas, state, 0.0, false, None)
    }

    #[test]
    fn window_centers_on_first_call() {
        let atlas = FontAtlas::from_embedded(14.0);
        let ctx = UiContext::new(800.0, 600.0);
        let mut state = StateCache::new();
        let mut ui = make_frame(&ctx, &atlas, &mut state);

        let rect = ui.window(WidgetId(999), 200.0, 100.0, 25.0);
        assert_eq!(rect.x, 300.0);
        assert_eq!(rect.y, 250.0);
        assert_eq!(rect.w, 200.0);
        assert_eq!(rect.h, 100.0);
    }

    #[test]
    fn window_drag_moves_position() {
        let atlas = FontAtlas::from_embedded(14.0);
        let mut state = StateCache::new();
        let id = WidgetId(999);

        // Frame 1: initial centering
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (300.0, 250.0));

        // Frame 2: click inside title bar to start drag
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 350.0;
        ctx.mouse_y = 260.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        ui.window(id, 200.0, 100.0, 25.0);

        // Frame 3: move mouse while held
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 400.0;
        ctx.mouse_y = 280.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (350.0, 270.0));

        // Frame 4: release mouse — position stays
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 400.0;
        ctx.mouse_y = 280.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (350.0, 270.0));
    }

    #[test]
    fn interact_hover_click_and_focus() {
        let atlas = FontAtlas::from_embedded(14.0);
        let mut state = StateCache::new();
        let id_a = WidgetId(50);
        let id_b = WidgetId(51);
        let rect_a = Rect::new(10.0, 10.0, 100.0, 30.0);
        let rect_b = Rect::new(10.0, 50.0, 100.0, 30.0);

        // Hover over A without clicking
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 25.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let r = ui.interact(id_a, rect_a);
        assert!(r.hovered());
        assert!(!r.clicked());
        assert!(!r.has_focus());

        // Click on A — should be clicked + focused
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let r = ui.interact(id_a, rect_a);
        assert!(r.clicked());
        assert!(r.has_focus());

        // Next frame: no click, mouse on B — A retains focus, B is hovered
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 65.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let ra = ui.interact(id_a, rect_a);
        let rb = ui.interact(id_b, rect_b);
        assert!(!ra.hovered());
        assert!(!ra.has_focus()); // focus not carried across frames (no initial_focus)
        assert!(rb.hovered());
        assert!(!rb.clicked());

        // Click on B — focus moves to B
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let ra = ui.interact(id_a, rect_a);
        let rb = ui.interact(id_b, rect_b);
        assert!(!ra.has_focus());
        assert!(rb.has_focus());
        assert!(rb.clicked());

        // Click outside both rects — no response
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 200.0;
        ctx.mouse_y = 200.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let ra = ui.interact(id_a, rect_a);
        let rb = ui.interact(id_b, rect_b);
        assert!(!ra.hovered());
        assert!(!ra.clicked());
        assert!(!rb.hovered());
        assert!(!rb.clicked());
    }

    #[test]
    fn window_click_outside_title_bar_does_not_drag() {
        let atlas = FontAtlas::from_embedded(14.0);
        let mut state = StateCache::new();
        let id = WidgetId(999);

        // Frame 1: initial centering
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        ui.window(id, 200.0, 100.0, 25.0);

        // Frame 2: click inside window body but below title bar (y=250+25=275, click at 290)
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 350.0;
        ctx.mouse_y = 290.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        ui.window(id, 200.0, 100.0, 25.0);

        // Frame 3: move mouse — position should not change
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 500.0;
        ctx.mouse_y = 400.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let rect = ui.window(id, 200.0, 100.0, 25.0);
        assert_eq!((rect.x, rect.y), (300.0, 250.0));
    }

    #[test]
    fn any_hovered_tracks_widget_hover() {
        let atlas = FontAtlas::from_embedded(14.0);
        let mut state = StateCache::new();
        let rect = Rect::new(10.0, 10.0, 100.0, 30.0);

        // Mouse outside — any_hovered stays false
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        ui.interact(WidgetId(1), rect);
        assert!(!ui.any_hovered);

        // Mouse inside — any_hovered becomes true
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 25.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        ui.interact(WidgetId(1), rect);
        assert!(ui.any_hovered);
    }
}
