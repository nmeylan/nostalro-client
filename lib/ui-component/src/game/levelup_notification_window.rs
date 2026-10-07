use crate::Window;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const LEVELUP_NOTIFY_WINDOW_ID: WidgetId = WidgetId(2710);
const BASE_BTN_ID: WidgetId = WidgetId(2711);
const JOB_BTN_ID: WidgetId = WidgetId(2712);

const LV_UP_OFF: &str = ragnarok_resources::ui::basic::LV_UP_OFF;
const LV_UP_ON: &str = ragnarok_resources::ui::basic::LV_UP_ON;

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
    status_was_open: bool,
    skill_was_open: bool,
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
            status_was_open: false,
            skill_was_open: false,
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

    fn dismiss_answered_icons(&mut self, status_open: bool, skill_open: bool) {
        if status_open && !self.status_was_open {
            self.show_base = false;
        }
        if skill_open && !self.skill_was_open {
            self.show_job = false;
        }
        self.status_was_open = status_open;
        self.skill_was_open = skill_open;
    }

    pub fn build(&mut self, ui: &mut UiFrame, status_open: bool, skill_open: bool) -> LevelUpClick {
        self.dismiss_answered_icons(status_open, skill_open);

        let (icon_w, icon_h) = self.icon_size;
        let y = ui.ctx.screen_height - icon_h - BOTTOM_MARGIN;
        let mut result = LevelUpClick::None;

        if self.show_job {
            let rect = Rect::new(MARGIN, y, icon_w, icon_h);
            if self.icon(ui, JOB_BTN_ID, rect) {
                self.show_job = false;
                result = LevelUpClick::Job;
            }
        }

        if self.show_base {
            let rect = Rect::new(ui.ctx.screen_width - icon_w - MARGIN, y, icon_w, icon_h);
            if self.icon(ui, BASE_BTN_ID, rect) {
                self.show_base = false;
                result = LevelUpClick::Base;
            }
        }

        result
    }

    /// Drawn in a popup layer so the icon stays clickable over any window it
    /// overlaps, and swallows the click instead of sharing it with that window.
    fn icon(&self, ui: &mut UiFrame, id: WidgetId, rect: Rect) -> bool {
        ui.begin_popup_layer(rect);
        let resp = ui.interact(id, rect);
        if resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        self.draw_icon(ui, rect, resp.hovered());
        ui.end_popup_layer();
        resp.clicked()
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

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    /// How many level up icons the frame actually drew.
    fn drawn_icons(ui: &UiFrame) -> usize {
        ui.draw_calls
            .iter()
            .filter(|call| match &call.texture {
                TextureRef::Named(path) => path == LV_UP_OFF || path == LV_UP_ON,
                _ => false,
            })
            .count()
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

        let mut ui = test_frame(&mut ctx, &mut state);
        assert_eq!(win.build(&mut ui, false, false), LevelUpClick::Base);
        assert_eq!(win.build(&mut ui, false, false), LevelUpClick::None);
    }

    #[test]
    fn job_icon_is_clickable_over_another_window() {
        const CHAT_ID: WidgetId = WidgetId(9000);
        const MINIMAP_ID: WidgetId = WidgetId(9001);
        let chat_rect = Rect::new(0.0, 400.0, 500.0, 200.0);
        let minimap_rect = Rect::new(700.0, 0.0, 100.0, 100.0);
        let z_order = [CHAT_ID, MINIMAP_ID];

        let mut win = LevelUpNotificationWindow::new();
        win.notify_job_level_up();

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        let (w, h) = DEFAULT_SIZE;
        ctx.mouse_x = MARGIN + w / 2.0;
        ctx.mouse_y = 600.0 - h - BOTTOM_MARGIN + h / 2.0;

        {
            let mut ui = test_frame(&mut ctx, &mut state);
            ui.compute_hovered_window(&z_order);
            ui.enter_window(CHAT_ID, chat_rect);
            ui.enter_window(MINIMAP_ID, minimap_rect);
            win.build(&mut ui, false, false);
        }

        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        ui.compute_hovered_window(&z_order);
        assert_eq!(ui.hovered_window(), Some(CHAT_ID));
        ui.enter_window(CHAT_ID, chat_rect);
        ui.enter_window(MINIMAP_ID, minimap_rect);
        assert_eq!(win.build(&mut ui, false, false), LevelUpClick::Job);
    }

    #[test]
    fn each_window_only_dismisses_its_own_icon() {
        let mut win = LevelUpNotificationWindow::new();
        win.notify_base_level_up();
        win.notify_job_level_up();

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);

        {
            let mut ui = test_frame(&mut ctx, &mut state);
            win.build(&mut ui, false, false);
            assert_eq!(drawn_icons(&ui), 2);
        }

        // Attributes window answers the base icon; the job icon is untouched.
        {
            let mut ui = test_frame(&mut ctx, &mut state);
            win.build(&mut ui, true, false);
            assert_eq!(drawn_icons(&ui), 1);
        }

        let mut ui = test_frame(&mut ctx, &mut state);
        win.build(&mut ui, true, true);
        assert_eq!(drawn_icons(&ui), 0);
    }

    #[test]
    fn window_kept_open_across_frames_does_not_dismiss_a_later_level_up() {
        let mut win = LevelUpNotificationWindow::new();

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);

        // Window opened before the level up, and left open.
        {
            let mut ui = test_frame(&mut ctx, &mut state);
            win.build(&mut ui, true, false);
            assert_eq!(drawn_icons(&ui), 0);
        }

        win.notify_base_level_up();

        // Still open, but there was no closed -> open transition, so the icon
        // stays up: the player never answered it, they were already there.
        {
            let mut ui = test_frame(&mut ctx, &mut state);
            win.build(&mut ui, true, false);
            assert_eq!(drawn_icons(&ui), 1);
        }

        // Closing and reopening answers it.
        {
            let mut ui = test_frame(&mut ctx, &mut state);
            win.build(&mut ui, false, false);
            assert_eq!(drawn_icons(&ui), 1);
        }

        let mut ui = test_frame(&mut ctx, &mut state);
        win.build(&mut ui, true, false);
        assert_eq!(drawn_icons(&ui), 0);
    }
}
