use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const SCROLLBAR_W: f32 = 13.0;
pub const SCROLL_BTN_H: f32 = 13.0;

const TRACK_TILE_H: f32 = 13.0;
const THUMB_PIECE_H: f32 = 4.0;
const MIN_THUMB_H: f32 = 8.0;

const SCROLL_UP_TEX: &str = ragnarok_resources::ui::SCROLL0UP;
const SCROLL_DOWN_TEX: &str = ragnarok_resources::ui::SCROLL0DOWN;
const SCROLL_TRACK_TEX: &str = ragnarok_resources::ui::SCROLL0MID;
const THUMB_TOP_TEX: &str = ragnarok_resources::ui::SCROLL0BAR_UP;
const THUMB_BODY_TEX: &str = ragnarok_resources::ui::SCROLL0BAR_MID;
const THUMB_BOTTOM_TEX: &str = ragnarok_resources::ui::SCROLL0BAR_DOWN;

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

fn blit(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, texture: &str) {
    let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::Named(texture.to_string()),
    });
}

fn fill(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let (v, i) = draw::quad_vertices(x, y, w, h, color);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
}

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

    let bar_rect = Rect::new(x, y, SCROLLBAR_W, h);
    let mut scroll = ui.take_scroll(content_rect);
    if scroll == 0.0 {
        scroll = ui.take_scroll(bar_rect);
    }
    if scroll != 0.0 {
        let delta = if scroll > 0.0 { -1i32 } else { 1 };
        offset = (offset as i32 + delta).clamp(0, max_scroll as i32) as usize;
    }

    if max_scroll == 0 {
        return offset;
    }

    if bar_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
        ui.any_interactive_hovered = true;
    }

    let has_grf = ui.has_grf_textures;
    let track_top = y + SCROLL_BTN_H;
    let track_bottom = y + h - SCROLL_BTN_H;
    let track_h = track_bottom - track_top;

    let up_rect = Rect::new(x, y, SCROLLBAR_W, SCROLL_BTN_H);
    let up_response = ui.interact(ids.up, up_rect);
    if has_grf {
        blit(ui, x, y + 1.0, SCROLLBAR_W, SCROLL_BTN_H, SCROLL_UP_TEX);
    } else {
        let color = if up_response.hovered() {
            [0.5, 0.5, 0.6, 1.0]
        } else {
            [0.3, 0.3, 0.4, 1.0]
        };
        fill(ui, x, y, SCROLLBAR_W, SCROLL_BTN_H, color);
    }
    if up_response.clicked() && offset > 0 {
        offset -= 1;
    }

    if has_grf {
        let mut ty = track_top;
        while ty + TRACK_TILE_H <= track_bottom {
            blit(ui, x, ty, SCROLLBAR_W, TRACK_TILE_H, SCROLL_TRACK_TEX);
            ty += TRACK_TILE_H;
        }
        blit(
            ui,
            x,
            track_bottom - TRACK_TILE_H,
            SCROLLBAR_W,
            TRACK_TILE_H,
            SCROLL_TRACK_TEX,
        );
    } else {
        fill(ui, x, track_top, SCROLLBAR_W, track_h, [0.0, 0.0, 0.0, 0.3]);
    }

    let down_y = y + h - SCROLL_BTN_H;
    let down_rect = Rect::new(x, down_y, SCROLLBAR_W, SCROLL_BTN_H);
    let down_response = ui.interact(ids.down, down_rect);
    if has_grf {
        blit(
            ui,
            x,
            y + h - SCROLL_BTN_H - 1.0,
            SCROLLBAR_W,
            SCROLL_BTN_H,
            SCROLL_DOWN_TEX,
        );
    } else {
        let color = if down_response.hovered() {
            [0.5, 0.5, 0.6, 1.0]
        } else {
            [0.3, 0.3, 0.4, 1.0]
        };
        fill(ui, x, down_y, SCROLLBAR_W, SCROLL_BTN_H, color);
    }
    if down_response.clicked() && offset < max_scroll {
        offset += 1;
    }

    let total = visible_rows + max_scroll;
    let thumb_h = (track_h * visible_rows as f32 / total as f32).max(MIN_THUMB_H);
    let thumb_y = track_top + (track_h - thumb_h) * offset as f32 / max_scroll as f32;
    let thumb_rect = Rect::new(x, thumb_y, SCROLLBAR_W, thumb_h);

    let mouse_clicked = ui.ctx.mouse_clicked;
    let mouse_down = ui.ctx.mouse_down;
    let hovered = thumb_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y);

    let new_scroll = {
        let t_drag = ui.state.get_or_default::<ScrollThumbState>(ids.thumb);
        if hovered && mouse_clicked {
            t_drag.dragging = true;
            t_drag.start_mouse = ui.ctx.mouse_y;
            t_drag.start_value = offset as f32;
        }
        if !mouse_down {
            t_drag.dragging = false;
        }
        if t_drag.dragging {
            let dy = ui.ctx.mouse_y - t_drag.start_mouse;
            let scroll_per_px = max_scroll as f32 / (track_h - thumb_h).max(1.0);
            Some((t_drag.start_value + dy * scroll_per_px).round() as i32)
        } else {
            None
        }
    };

    if let Some(ns) = new_scroll {
        offset = ns.clamp(0, max_scroll as i32) as usize;
    } else if mouse_clicked && !hovered && bar_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
        let page = visible_rows.saturating_sub(1).max(1) as i32;
        if ui.ctx.mouse_y >= track_top && ui.ctx.mouse_y < thumb_y {
            offset = (offset as i32 - page).clamp(0, max_scroll as i32) as usize;
        } else if ui.ctx.mouse_y >= thumb_y + thumb_h && ui.ctx.mouse_y < track_bottom {
            offset = (offset as i32 + page).clamp(0, max_scroll as i32) as usize;
        }
    }

    let thumb_y = track_top + (track_h - thumb_h) * offset as f32 / max_scroll as f32;
    if has_grf {
        blit(ui, x, thumb_y, SCROLLBAR_W, THUMB_PIECE_H, THUMB_TOP_TEX);
        let bottom_y = thumb_y + thumb_h - THUMB_PIECE_H;
        let mut ty = thumb_y + THUMB_PIECE_H;
        while ty < bottom_y {
            blit(ui, x, ty, SCROLLBAR_W, THUMB_PIECE_H, THUMB_BODY_TEX);
            ty += THUMB_PIECE_H;
        }
        blit(
            ui,
            x,
            bottom_y,
            SCROLLBAR_W,
            THUMB_PIECE_H,
            THUMB_BOTTOM_TEX,
        );
    } else {
        let color = if hovered {
            [0.6, 0.6, 0.7, 0.9]
        } else {
            [0.5, 0.5, 0.6, 0.8]
        };
        fill(ui, x + 2.0, thumb_y, SCROLLBAR_W - 4.0, thumb_h, color);
    }

    offset
}

pub fn grf_texture_paths() -> Vec<&'static str> {
    vec![
        SCROLL_UP_TEX,
        SCROLL_DOWN_TEX,
        SCROLL_TRACK_TEX,
        THUMB_TOP_TEX,
        THUMB_BODY_TEX,
        THUMB_BOTTOM_TEX,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    fn ids() -> ScrollbarIds {
        ScrollbarIds {
            up: WidgetId(900),
            down: WidgetId(901),
            thumb: WidgetId(902),
        }
    }

    #[test]
    fn mouse_wheel_scrolls_down() {
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.scroll_delta = -1.0; // scroll down
        let mut ui = test_frame(&mut ctx, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 0, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 1);
    }

    #[test]
    fn mouse_wheel_scrolls_up() {
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.scroll_delta = 1.0; // scroll up
        let mut ui = test_frame(&mut ctx, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 5, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 4);
    }

    #[test]
    fn clamps_offset_to_max_scroll() {
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        let mut ui = test_frame(&mut ctx, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 20, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 10);
    }

    #[test]
    fn up_button_click_decrements() {
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 195.0;
        ctx.mouse_y = 5.0;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 5, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 4);
    }

    #[test]
    fn down_button_click_increments() {
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 195.0;
        ctx.mouse_y = 190.0;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 5, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 6);
    }

    #[test]
    fn track_click_below_thumb_pages_down() {
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 195.0;
        ctx.mouse_y = 180.0;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 0, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 4);
    }

    #[test]
    fn nothing_drawn_without_scroll_range() {
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        let mut ui = test_frame(&mut ctx, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        scrollbar(&mut ui, ids(), 0, 5, 0, content, 190.0, 0.0, 200.0);
        assert!(ui.draw_calls.is_empty());
    }

    #[test]
    fn no_scroll_past_zero() {
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 50.0;
        ctx.mouse_y = 50.0;
        ctx.scroll_delta = 1.0; // scroll up
        let mut ui = test_frame(&mut ctx, &mut state);

        let content = Rect::new(0.0, 0.0, 200.0, 200.0);
        let result = scrollbar(&mut ui, ids(), 0, 5, 10, content, 190.0, 0.0, 200.0);
        assert_eq!(result, 0);
    }
}
