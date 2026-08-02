use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_sys_button, draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::book::BookContent;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef, strip_color_codes, word_wrap};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const BOOK_WINDOW_ID: WidgetId = WidgetId(2500);
const PREV_BTN_ID: WidgetId = WidgetId(2501);
const NEXT_BTN_ID: WidgetId = WidgetId(2502);
const CLOSE_BTN_ID: WidgetId = WidgetId(2503);

const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;

const PREV_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_BACK,
    hover: ragnarok_resources::ui::BTN_BACK_A,
    pressed: ragnarok_resources::ui::BTN_BACK_B,
};
const NEXT_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_NEXT,
    hover: ragnarok_resources::ui::BTN_NEXT_A,
    pressed: ragnarok_resources::ui::BTN_NEXT_B,
};

const WIN_W: f32 = 555.0;
const WIN_H: f32 = 455.0;
const TITLE_H: f32 = 20.0;
const TEXT_X: f32 = 28.0;
const TEXT_TOP: f32 = 28.0;
const TEXT_W: f32 = 500.0;
const TEXT_LINE_H: f32 = 16.0;
const FOOTER_H: f32 = 34.0;
const CLOSE_SIZE: f32 = 11.0;
const NAV_BTN_W: f32 = 42.0;
const NAV_BTN_H: f32 = 20.0;

pub struct BookWindow {
    pub has_grf_textures: bool,
    content: Option<BookContent>,
    wrapped_lines: Vec<String>,
    page: usize,
}

impl Default for BookWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl BookWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            content: None,
            wrapped_lines: Vec::new(),
            page: 0,
        }
    }

    pub fn show(&mut self, content: BookContent) {
        self.content = Some(content);
        self.wrapped_lines.clear();
        self.page = 0;
    }

    pub fn close(&mut self) {
        self.content = None;
        self.wrapped_lines.clear();
        self.page = 0;
    }

    pub fn is_open(&self) -> bool {
        self.content.is_some()
    }
}

impl Window for BookWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, WIN_H)
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            PREV_BTN.normal,
            PREV_BTN.hover,
            PREV_BTN.pressed,
            NEXT_BTN.normal,
            NEXT_BTN.hover,
            NEXT_BTN.pressed,
        ]
    }
}

impl InGameWindow for BookWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.is_open()
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.close();
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        if self.content.is_none() {
            return Vec::new();
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;

        let bg = self.content.as_ref().unwrap().bg_color;
        if self.wrapped_lines.is_empty() {
            let full = self.content.as_ref().unwrap().lines.join("\n");
            self.wrapped_lines = word_wrap(
                &full,
                TEXT_W,
                |t| ui.atlas.measure_text(&strip_color_codes(t)),
                false,
            );
        }

        let text_area_h = WIN_H - TEXT_TOP - FOOTER_H;
        let lines_per_page = ((text_area_h / TEXT_LINE_H).floor() as usize).max(1);
        let total_pages = self.wrapped_lines.len().div_ceil(lines_per_page).max(1);
        if self.page >= total_pages {
            self.page = total_pages - 1;
        }

        let default_x = (ui.ctx.screen_width - WIN_W) / 2.0;
        let default_y = (ui.ctx.screen_height - WIN_H) / 2.0;
        let win = ui.window_at(BOOK_WINDOW_ID, WIN_W, WIN_H, TITLE_H, default_x, default_y);
        ui.interact(BOOK_WINDOW_ID, Rect::new(win.x, win.y, WIN_W, WIN_H));

        let (v, i) = draw::quad_vertices(
            win.x,
            win.y + TITLE_H,
            WIN_W,
            WIN_H - TITLE_H,
            [bg[0], bg[1], bg[2], 1.0],
        );
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let border = [0.4, 0.35, 0.28, 1.0];
        for (bx, by, bw, bh) in [
            (win.x, win.y, WIN_W, 1.0),
            (win.x, win.y + WIN_H - 1.0, WIN_W, 1.0),
            (win.x, win.y, 1.0, WIN_H),
            (win.x + WIN_W - 1.0, win.y, 1.0, WIN_H),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, border);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        draw_titlebar(ui, win.x, win.y, WIN_W, TITLE_H, grf);
        let title = format!("Book  ({}/{})", self.page + 1, total_pages);
        ui.text(win.x + 20.0, win.y + TITLE_H - 5.0, &title, text_color(grf));

        let close_rect = Rect::new(
            win.x + WIN_W - CLOSE_SIZE - 4.0,
            win.y + 4.0,
            CLOSE_SIZE,
            CLOSE_SIZE,
        );
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            close_rect,
            (CLOSE_SIZE, CLOSE_SIZE),
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

        let start = self.page * lines_per_page;
        let end = (start + lines_per_page).min(self.wrapped_lines.len());
        let text_x = win.x + TEXT_X;
        let mut text_y = win.y + TEXT_TOP + ui.atlas.line_height;
        for line in &self.wrapped_lines[start..end] {
            ui.colored_text(text_x, text_y, line, [0.1, 0.1, 0.1, 1.0]);
            text_y += TEXT_LINE_H;
        }

        let footer_y = win.y + WIN_H - FOOTER_H + (FOOTER_H - NAV_BTN_H) / 2.0;
        let prev_x = win.x + 20.0;
        let next_x = win.x + WIN_W - 20.0 - NAV_BTN_W;
        if self.page > 0 {
            let rect = Rect::new(prev_x, footer_y, NAV_BTN_W, NAV_BTN_H);
            if ui.button(PREV_BTN_ID, rect, &PREV_BTN, "Prev").clicked() {
                self.page -= 1;
            }
        }
        if self.page + 1 < total_pages {
            let rect = Rect::new(next_x, footer_y, NAV_BTN_W, NAV_BTN_H);
            if ui.button(NEXT_BTN_ID, rect, &NEXT_BTN, "Next").clicked() {
                self.page += 1;
            }
        }

        ui.has_grf_textures = prev_grf;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_opens_resets_page_and_close_clears() {
        let mut win = BookWindow::new();
        assert!(!win.is_open());
        win.page = 3;
        win.show(BookContent {
            bg_color: [1.0, 1.0, 1.0],
            lines: vec!["a".to_string(), "b".to_string()],
        });
        assert!(win.is_open());
        assert_eq!(win.page, 0);
        win.close();
        assert!(!win.is_open());
    }
}
