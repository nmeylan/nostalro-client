use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_sys_button,
    draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_game::monster_info::MonsterInfo;
use ragnarok_ui::draw::strip_color_codes;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const MONSTER_INFO_WINDOW_ID: WidgetId = WidgetId(5900);
const CLOSE_BTN_ID: WidgetId = WidgetId(5901);

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const TITLE_H: f32 = 17.0;
const PAD: f32 = 8.0;
const LINE_H: f32 = 14.0;
const MIN_W: f32 = 150.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const ANCHOR_X: f32 = 0.0;
const ANCHOR_Y: f32 = 120.0;
const NOMINAL_SIZE: (f32, f32) = (MIN_W, TITLE_H + PAD * 2.0 + LINE_H * 18.0);

pub struct MonsterInfoWindow {
    pub has_grf_textures: bool,
    info: Option<MonsterInfo>,
    lines: Vec<String>,
    measured: Option<(f32, f32)>,
}

impl Default for MonsterInfoWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl MonsterInfoWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            info: None,
            lines: Vec::new(),
            measured: None,
        }
    }

    pub fn show(&mut self, info: MonsterInfo) {
        self.lines = info.info_lines();
        self.info = Some(info);
        self.measured = None;
    }

    pub fn close(&mut self) {
        self.info = None;
        self.lines.clear();
        self.measured = None;
    }

    pub fn is_open(&self) -> bool {
        self.info.is_some()
    }
}

impl Window for MonsterInfoWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn window_size(&self) -> (f32, f32) {
        self.measured.unwrap_or(NOMINAL_SIZE)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
        ]
    }
}

impl InGameWindow for MonsterInfoWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.is_open()
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.close();
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        if self.info.is_none() {
            return Vec::new();
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;

        if self.measured.is_none() {
            let widest = self
                .lines
                .iter()
                .map(|l| ui.atlas.measure_text(&strip_color_codes(l)))
                .fold(0.0f32, f32::max);
            self.measured = Some((
                (widest + PAD * 2.0).max(MIN_W),
                TITLE_H + PAD * 2.0 + self.lines.len() as f32 * LINE_H,
            ));
        }
        let (w, h) = self.measured.unwrap();

        let win = ui.window_at(MONSTER_INFO_WINDOW_ID, w, h, TITLE_H, ANCHOR_X, ANCHOR_Y);
        let (x, y) = (win.x, win.y);
        ui.interact(MONSTER_INFO_WINDOW_ID, Rect::new(x, y, w, h));

        draw_titlebar(ui, x, y, w, TITLE_H, grf);
        ui.text(x + 16.0, y + TITLE_H - 3.0, "Monster Info", text_color(grf));

        let close_rect = Rect::new(
            x + w - CLOSE_BTN_SIZE - 3.0,
            y + (TITLE_H - CLOSE_BTN_SIZE) / 2.0,
            CLOSE_BTN_SIZE,
            CLOSE_BTN_SIZE,
        );
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            close_rect,
            (CLOSE_BTN_SIZE, CLOSE_BTN_SIZE),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );
        if close_resp.clicked() {
            self.close();
            ui.has_grf_textures = prev_grf;
            return Vec::new();
        }

        let body_y = y + TITLE_H;
        draw_container(ui, x, body_y, w, h - TITLE_H, grf);

        let mut text_y = body_y + PAD + LINE_H - 3.0;
        for line in &self.lines {
            ui.colored_text(x + PAD, text_y, line, text_color(grf));
            text_y += LINE_H;
        }

        ui.has_grf_textures = prev_grf;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_caches_lines_and_close_clears_them() {
        let mut win = MonsterInfoWindow::new();
        assert!(!win.is_open());
        win.show(MonsterInfo {
            name: "Poring".to_string(),
            property: 21,
            ..Default::default()
        });
        assert!(win.is_open());
        assert_eq!(win.lines.len(), 18);
        assert_eq!(win.lines[0], "^FFFF00Name^FF00FF: Poring");
        win.close();
        assert!(!win.is_open());
        assert!(win.lines.is_empty());
    }
}
