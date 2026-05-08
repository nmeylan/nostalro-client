use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const SCROLLBAR_W: f32 = 14.0;
pub const SCROLL_BTN_H: f32 = 14.0;

const SCROLL_UP_TEX: &str = "data/texture/유저인터페이스/scroll0up.bmp";
const SCROLL_DOWN_TEX: &str = "data/texture/유저인터페이스/scroll0down.bmp";

pub struct ScrollbarIds {
    pub up: WidgetId,
    pub down: WidgetId,
    pub thumb: WidgetId,
}

#[derive(Default)]
struct ScrollThumbState {
    dragging: bool,
    start_mouse: f32,
    start_value: f32,
}

/// Draws a vertical scrollbar and handles all interaction (buttons, thumb drag, mouse wheel).
/// Returns the updated scroll offset.
pub fn scrollbar(
    ui: &mut UiFrame,
    ids: ScrollbarIds,
    offset: usize,
    visible_rows: usize,
    max_scroll: usize,
    content_rect: Rect,
    x: f32,
    y: f32,
    h: f32,
) -> usize {
    let mut offset = offset.min(max_scroll);

    // Mouse wheel
    if content_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) && ui.ctx.scroll_delta != 0.0 {
        let delta = if ui.ctx.scroll_delta > 0.0 { -1i32 } else { 1 };
        offset = (offset as i32 + delta).clamp(0, max_scroll as i32) as usize;
    }

    let has_grf = ui.has_grf_textures;

    // Track background
    let (v, i) = draw::quad_vertices(x, y, SCROLLBAR_W, h, [0.0, 0.0, 0.0, 0.3]);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });

    // Up button
    let up_rect = Rect::new(x, y, SCROLLBAR_W, SCROLL_BTN_H);
    let up_response = ui.interact(ids.up, up_rect);
    if up_response.hovered() {
        ui.any_interactive_hovered = true;
    }
    if has_grf {
        let (v, i) = draw::quad_vertices(x, y, SCROLLBAR_W, SCROLL_BTN_H, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(SCROLL_UP_TEX.to_string()),
        });
    } else {
        let color = if up_response.hovered() {
            [0.5, 0.5, 0.6, 1.0]
        } else {
            [0.3, 0.3, 0.4, 1.0]
        };
        let (v, i) = draw::quad_vertices(x, y, SCROLLBAR_W, SCROLL_BTN_H, color);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }
    if up_response.clicked() && offset > 0 {
        offset -= 1;
    }

    // Down button
    let down_y = y + h - SCROLL_BTN_H;
    let down_rect = Rect::new(x, down_y, SCROLLBAR_W, SCROLL_BTN_H);
    let down_response = ui.interact(ids.down, down_rect);
    if down_response.hovered() {
        ui.any_interactive_hovered = true;
    }
    if has_grf {
        let (v, i) =
            draw::quad_vertices(x, down_y, SCROLLBAR_W, SCROLL_BTN_H, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(SCROLL_DOWN_TEX.to_string()),
        });
    } else {
        let color = if down_response.hovered() {
            [0.5, 0.5, 0.6, 1.0]
        } else {
            [0.3, 0.3, 0.4, 1.0]
        };
        let (v, i) = draw::quad_vertices(x, down_y, SCROLLBAR_W, SCROLL_BTN_H, color);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }
    if down_response.clicked() && offset < max_scroll {
        offset += 1;
    }

    // Thumb
    if max_scroll > 0 {
        let track_y = y + SCROLL_BTN_H;
        let track_h = h - 2.0 * SCROLL_BTN_H;
        let thumb_ratio = visible_rows as f32 / (visible_rows + max_scroll) as f32;
        let thumb_h = (track_h * thumb_ratio).max(10.0);
        let scroll_ratio = offset as f32 / max_scroll as f32;
        let thumb_y = track_y + scroll_ratio * (track_h - thumb_h);

        let thumb_rect = Rect::new(x, thumb_y, SCROLLBAR_W, thumb_h);
        let hovered = thumb_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y);
        if hovered {
            ui.any_interactive_hovered = true;
        }
        let mouse_clicked = ui.ctx.mouse_clicked;
        let mouse_down = ui.ctx.mouse_down;

        let (thumb_active, new_scroll) = {
            let t_drag = ui.state.get_or_default::<ScrollThumbState>(ids.thumb);
            if hovered && mouse_clicked {
                t_drag.dragging = true;
                t_drag.start_mouse = ui.ctx.mouse_y;
                t_drag.start_value = offset as f32;
            }
            if !mouse_down {
                t_drag.dragging = false;
            }
            let active = t_drag.dragging;
            let new = if t_drag.dragging {
                let dy = ui.ctx.mouse_y - t_drag.start_mouse;
                let scroll_per_px = max_scroll as f32 / (track_h - thumb_h).max(1.0);
                Some((t_drag.start_value + dy * scroll_per_px).round() as i32)
            } else {
                None
            };
            (active, new)
        };

        if let Some(ns) = new_scroll {
            offset = ns.clamp(0, max_scroll as i32) as usize;
        }

        let thumb_color = if thumb_active {
            [0.6, 0.6, 0.7, 0.9]
        } else {
            [0.5, 0.5, 0.6, 0.8]
        };
        let (v, i) = draw::quad_vertices(x + 2.0, thumb_y, SCROLLBAR_W - 4.0, thumb_h, thumb_color);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }

    offset
}

pub fn grf_texture_paths() -> Vec<&'static str> {
    vec![SCROLL_UP_TEX, SCROLL_DOWN_TEX]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(
        ctx: &'a UiContext,
        atlas: &'a FontAtlas,
        state: &'a mut StateCache,
    ) -> UiFrame<'a> {
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    fn ids() -> ScrollbarIds {
        ScrollbarIds {
            up: WidgetId(900),
            down: WidgetId(901),
            thumb: WidgetId(902),
        }
    }

    #[test]
    fn mouse_wheel_scrolls_down() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.scroll_delta = -1.0; // scroll down
        let mut ui = make_frame(&ctx, &atlas, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 0, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 1);
    }

    #[test]
    fn mouse_wheel_scrolls_up() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.scroll_delta = 1.0; // scroll up
        let mut ui = make_frame(&ctx, &atlas, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 5, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 4);
    }

    #[test]
    fn clamps_offset_to_max_scroll() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 20, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 10);
    }

    #[test]
    fn up_button_click_decrements() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        // Click on the up button area
        ctx.mouse_x = 195.0;
        ctx.mouse_y = 5.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 5, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 4);
    }

    #[test]
    fn down_button_click_increments() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        // Click on the down button area (y + h - SCROLL_BTN_H = 200 - 14 = 186)
        ctx.mouse_x = 195.0;
        ctx.mouse_y = 190.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 5, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 6);
    }

    #[test]
    fn no_scroll_past_zero() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.scroll_delta = 1.0; // scroll up
        let mut ui = make_frame(&ctx, &atlas, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 0, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 0);
    }
}
