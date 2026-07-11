use crate::Window;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const LEVELUP_NOTIFY_WINDOW_ID: WidgetId = WidgetId(2710);
const BASE_BTN_ID: WidgetId = WidgetId(2711);
const JOB_BTN_ID: WidgetId = WidgetId(2712);

const LV_UP_OFF: &str = "data/texture/유저인터페이스/basic_interface/lv_up_off.bmp";
const LV_UP_ON: &str = "data/texture/유저인터페이스/basic_interface/lv_up_on.bmp";

const MARGIN: f32 = 0.0;
const BOTTOM_MARGIN: f32 = 8.0;
const DEFAULT_SIZE: (f32, f32) = (52.0, 20.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelUpClick {
    None,
    Base,
    Job,
}

pub struct LevelUpNotificationWindow {
    pub has_grf_textures: bool,
    show_base: bool,
    show_job: bool,
    icon_size: (f32, f32),
}

impl Default for LevelUpNotificationWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelUpNotificationWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            show_base: false,
            show_job: false,
            icon_size: DEFAULT_SIZE,
        }
    }

    pub fn notify_base_level_up(&mut self) {
        self.show_base = true;
    }

    pub fn notify_job_level_up(&mut self) {
        self.show_job = true;
    }

    fn draw_icon(&self, ui: &mut UiFrame, rect: Rect, hovered: bool) {
        let path = if hovered { LV_UP_ON } else { LV_UP_OFF };
        let (v, idx) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: idx.to_vec(),
            texture: TextureRef::Named(path.to_string()),
        });
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> LevelUpClick {
        let (icon_w, icon_h) = self.icon_size;
        let y = ui.ctx.screen_height - icon_h - BOTTOM_MARGIN;
        let mut result = LevelUpClick::None;

        if self.show_job {
            let rect = Rect::new(MARGIN, y, icon_w, icon_h);
            let resp = ui.interact(JOB_BTN_ID, rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            self.draw_icon(ui, rect, resp.hovered());
            if resp.clicked() {
                self.show_job = false;
                result = LevelUpClick::Job;
            }
        }

        if self.show_base {
            let rect = Rect::new(ui.ctx.screen_width - icon_w - MARGIN, y, icon_w, icon_h);
            let resp = ui.interact(BASE_BTN_ID, rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            self.draw_icon(ui, rect, resp.hovered());
            if resp.clicked() {
                self.show_base = false;
                result = LevelUpClick::Base;
            }
        }

        result
    }
}

impl Window for LevelUpNotificationWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(LV_UP_OFF) {
            self.icon_size = (w as f32, h as f32);
        }
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![LV_UP_OFF, LV_UP_ON]
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
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn clicking_base_icon_returns_base_and_dismisses() {
        let mut win = LevelUpNotificationWindow::new();
        win.notify_base_level_up();

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        let (w, h) = DEFAULT_SIZE;
        ctx.mouse_x = 800.0 - w - MARGIN + w / 2.0;
        ctx.mouse_y = 600.0 - h - BOTTOM_MARGIN + h / 2.0;
        ctx.mouse_clicked = true;

        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(win.build(&mut ui), LevelUpClick::Base);
        assert_eq!(win.build(&mut ui), LevelUpClick::None);
    }
}
