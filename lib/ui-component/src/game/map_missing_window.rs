use crate::Window;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const MAP_MISSING_WINDOW_ID: WidgetId = WidgetId(2700);
const WARP_BTN_ID: WidgetId = WidgetId(2701);
const CHARSELECT_BTN_ID: WidgetId = WidgetId(2702);

const WIN_W: f32 = 340.0;
const WIN_H: f32 = 150.0;
const BTN_W: f32 = 300.0;
const BTN_H: f32 = 26.0;
const BTN_SPACING: f32 = 8.0;

const DUMMY_BTN: ButtonTextures = ButtonTextures {
    normal: "",
    hover: "",
    pressed: "",
};

pub struct MapMissingWindow {
    pub has_grf_textures: bool,
    open: bool,
    map_name: String,
}

impl MapMissingWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            open: false,
            map_name: String::new(),
        }
    }

    pub fn show(&mut self, map_name: String) {
        self.map_name = map_name;
        self.open = true;
    }

    pub fn hide(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();
        if !self.open {
            return events;
        }

        // Rendered entirely with fallback (untextured) chrome so it works even
        // when the GRF is unusable.
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = false;

        let sw = ui.ctx.screen_width;
        let sh = ui.ctx.screen_height;
        push_color_quad(ui, 0.0, 0.0, sw, sh, [0.0, 0.0, 0.0, 0.55]);

        let wx = (sw - WIN_W) / 2.0;
        let wy = (sh - WIN_H) / 2.0;
        push_color_quad(ui, wx, wy, WIN_W, WIN_H, [0.11, 0.12, 0.18, 1.0]);
        push_border(ui, Rect::new(wx, wy, WIN_W, WIN_H), [0.5, 0.62, 0.9, 1.0]);

        let lh = ui.atlas.line_height;
        let title = "Map data missing";
        let tw = ui.atlas.measure_text(title);
        ui.text(wx + (WIN_W - tw) / 2.0, wy + 8.0 + lh, title, [1.0, 1.0, 1.0, 1.0]);
        ui.text(
            wx + 16.0,
            wy + 8.0 + lh * 2.6,
            &format!("Map '{}' is not in your game data.", self.map_name),
            [0.85, 0.86, 0.92, 1.0],
        );

        let btn_x = wx + (WIN_W - BTN_W) / 2.0;
        let warp_y = wy + WIN_H - 12.0 - BTN_H * 2.0 - BTN_SPACING;
        let charselect_y = warp_y + BTN_H + BTN_SPACING;

        if ui
            .button(
                WARP_BTN_ID,
                Rect::new(btn_x, warp_y, BTN_W, BTN_H),
                &DUMMY_BTN,
                "Warp to Prontera",
            )
            .clicked()
        {
            events.push(GameEvent::RequestMapRecoveryWarp);
        }
        if ui
            .button(
                CHARSELECT_BTN_ID,
                Rect::new(btn_x, charselect_y, BTN_W, BTN_H),
                &DUMMY_BTN,
                "Return to Character Select",
            )
            .clicked()
        {
            events.push(GameEvent::BackToCharacterSelect);
            self.hide();
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl Default for MapMissingWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl Window for MapMissingWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![]
    }
}

fn push_color_quad(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let (v, i) = draw::quad_vertices(x, y, w, h, color);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
}

fn push_border(ui: &mut UiFrame, rect: Rect, color: [f32; 4]) {
    let b = 1.0;
    for (bx, by, bw, bh) in [
        (rect.x, rect.y, rect.w, b),
        (rect.x, rect.y + rect.h - b, rect.w, b),
        (rect.x, rect.y, b, rect.h),
        (rect.x + rect.w - b, rect.y, b, rect.h),
    ] {
        push_color_quad(ui, bx, by, bw, bh, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn closed_window_emits_nothing_and_draws_nothing() {
        let mut win = MapMissingWindow::new();
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);
        assert!(events.is_empty());
        assert!(ui.draw_calls.is_empty());
    }

    #[test]
    fn shown_window_renders_and_reports_open() {
        let mut win = MapMissingWindow::new();
        win.show("new_1-1".to_string());
        assert!(win.is_open());
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert!(!ui.draw_calls.is_empty());
    }
}
