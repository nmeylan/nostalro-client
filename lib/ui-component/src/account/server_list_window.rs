use crate::Window;
use ragnarok_game::event::{GameEvent, ServerInfo};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

const FALLBACK_WIN_W: f32 = 280.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

const HEADER_H: f32 = 22.0;
const LIST_X: f32 = 12.0;
const LIST_BOTTOM: f32 = 32.0;
const ROW_H: f32 = 17.0;
const ROW_PAD_LEFT: f32 = 5.0;
const ROW_PAD_TOP: f32 = 2.0;
const OK_BTN_RIGHT: f32 = 50.0;
const CANCEL_BTN_RIGHT: f32 = 5.0;
const BTN_BOTTOM: f32 = 4.0;

const WINDOW_ID: WidgetId = WidgetId(110);
const FALLBACK_TITLE_BAR_H: f32 = 30.0;

const OK_ID: WidgetId = WidgetId(100);
const CANCEL_ID: WidgetId = WidgetId(101);

const WIN_TEXTURE: &str = "data/texture/유저인터페이스/login_interface/win_service.bmp";

const OK_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_ok.bmp",
    hover: "data/texture/유저인터페이스/btn_ok_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_ok_b.bmp",
};

const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_cancel.bmp",
    hover: "data/texture/유저인터페이스/btn_cancel_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_cancel_b.bmp",
};

const SELECTED_COLOR: [f32; 4] = [0.804, 0.878, 1.0, 1.0];
const LIST_BG_COLOR: [f32; 4] = [0.969, 0.969, 0.969, 1.0];
const LIST_BORDER_COLOR: [f32; 4] = [0.78, 0.78, 0.78, 1.0];

pub struct ServerListWindow {
    pub servers: Vec<ServerInfo>,
    pub selected_index: Option<usize>,
    pub has_grf_textures: bool,
    win_size: (f32, f32),
    btn_size: (f32, f32),
}

impl ServerListWindow {
    pub fn new(servers: Vec<ServerInfo>) -> Self {
        let selected_index = if servers.is_empty() { None } else { Some(0) };
        Self {
            servers,
            selected_index,
            has_grf_textures: false,
            win_size: (FALLBACK_WIN_W, FALLBACK_WIN_W),
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if ui.ctx.key_down
            && let Some(idx) = self.selected_index
            && idx + 1 < self.servers.len()
        {
            self.selected_index = Some(idx + 1);
        }
        if ui.ctx.key_up
            && let Some(idx) = self.selected_index
            && idx > 0
        {
            self.selected_index = Some(idx - 1);
        }

        if self.has_grf_textures {
            self.build_grf(ui, &mut events);
        } else {
            self.build_fallback(ui, &mut events);
        }

        if ui.ctx.key_enter && self.selected_index.is_some() {
            events.push(GameEvent::RequestSelectServer {
                index: self.selected_index.unwrap(),
            });
        }
        if ui.ctx.key_escape {
            events.push(GameEvent::BackToLogin);
        }

        events
    }

    fn build_grf(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let (win_w, win_h) = self.win_size;
        let (btn_w, btn_h) = self.btn_size;
        let header_h = HEADER_H;
        let list_x = LIST_X;
        let row_h = ROW_H;
        let win = ui.window(WINDOW_ID, win_w, win_h, header_h);

        let (v, i) = draw::quad_vertices(win.x, win.y, win_w, win_h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(WIN_TEXTURE.to_string()),
        });

        let list_w = win_w - list_x * 2.0;
        let list_h = win_h - header_h - (LIST_BOTTOM);
        let list_rect = Rect::new(win.x + list_x, win.y + header_h, list_w, list_h);
        let (v, i) = draw::quad_vertices(
            list_rect.x,
            list_rect.y,
            list_rect.w,
            list_rect.h,
            LIST_BG_COLOR,
        );
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });

        let b = 1.0;
        for (bx, by, bw, bh) in [
            (list_rect.x, list_rect.y, list_rect.w, b),
            (list_rect.x, list_rect.y + list_rect.h - b, list_rect.w, b),
            (list_rect.x, list_rect.y, b, list_rect.h),
            (list_rect.x + list_rect.w - b, list_rect.y, b, list_rect.h),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, LIST_BORDER_COLOR);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        let text_color = [0.0, 0.0, 0.0, 1.0];
        for (idx, server) in self.servers.iter().enumerate() {
            let row_y = list_rect.y + (ROW_PAD_TOP) + idx as f32 * row_h;
            if row_y + row_h > list_rect.y + list_rect.h {
                break;
            }
            let row_rect = Rect::new(list_rect.x + 1.0, row_y, list_w - 2.0, row_h);
            let row = ui.interact(WidgetId(WINDOW_ID.0 + 10 + idx as u32), row_rect);
            if row.hovered() {
                ui.any_interactive_hovered = true;
            }
            if row.clicked() {
                self.selected_index = Some(idx);
            }

            if self.selected_index == Some(idx) {
                let (v, i) = draw::quad_vertices(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    SELECTED_COLOR,
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }

            let text_y = row_y + row_h - (3.0);
            ui.text(
                list_rect.x + (ROW_PAD_LEFT),
                text_y,
                &server.name,
                text_color,
            );

            let count = format!("{}", server.user_count);
            let count_w = ui.atlas.measure_text(&count);
            ui.text(
                list_rect.x + list_w - (ROW_PAD_LEFT) - count_w - (2.0),
                text_y,
                &count,
                text_color,
            );
        }

        let btn_y = win.y + win_h - (BTN_BOTTOM) - btn_h;
        let ok_rect = Rect::new(win.x + win_w - (OK_BTN_RIGHT) - btn_w, btn_y, btn_w, btn_h);
        let cancel_rect = Rect::new(
            win.x + win_w - (CANCEL_BTN_RIGHT) - btn_w,
            btn_y,
            btn_w,
            btn_h,
        );
        let ok = ui.button(OK_ID, ok_rect, &OK_BTN, "OK");
        let cancel = ui.button(CANCEL_ID, cancel_rect, &CANCEL_BTN, "Cancel");

        if ok.clicked() && self.selected_index.is_some() {
            events.push(GameEvent::RequestSelectServer {
                index: self.selected_index.unwrap(),
            });
        }
        if cancel.clicked() {
            events.push(GameEvent::BackToLogin);
        }
    }

    fn build_fallback(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let row_h = ROW_H;
        let btn_w = FALLBACK_BTN_W;
        let btn_h = FALLBACK_BTN_H;
        let win_w = FALLBACK_WIN_W;
        let list_h = self.servers.len() as f32 * row_h;
        let padding = 8.0;
        let title_h = 30.0;
        let win_h = title_h + list_h + padding + btn_h + padding;
        let win = ui.window(WINDOW_ID, win_w, win_h, FALLBACK_TITLE_BAR_H);

        let (v, i) = draw::quad_vertices(win.x, win.y, win.w, win_h, [0.08, 0.08, 0.12, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });

        let border_color = [0.4, 0.4, 0.5, 1.0];
        let b = 1.0;
        for (bx, by, bw, bh) in [
            (win.x, win.y, win.w, b),
            (win.x, win.y + win_h - b, win.w, b),
            (win.x, win.y, b, win_h),
            (win.x + win.w - b, win.y, b, win_h),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        let title = "Select Server";
        let title_w = ui.atlas.measure_text(title);
        let title_x = win.x + (win.w - title_w) / 2.0;
        let title_y = win.y + padding + ui.atlas.line_height;
        ui.text(title_x, title_y, title, [1.0, 1.0, 1.0, 1.0]);

        let list_y = win.y + title_h;
        for (idx, server) in self.servers.iter().enumerate() {
            let row_y = list_y + idx as f32 * row_h;
            let row_rect = Rect::new(win.x + padding, row_y, win.w - padding * 2.0, row_h);
            let row = ui.interact(WidgetId(WINDOW_ID.0 + 10 + idx as u32), row_rect);
            if row.hovered() {
                ui.any_interactive_hovered = true;
            }
            if row.clicked() {
                self.selected_index = Some(idx);
            }

            let bg_color = if self.selected_index == Some(idx) {
                [0.2, 0.2, 0.4, 1.0]
            } else if row.hovered() {
                [0.15, 0.15, 0.25, 1.0]
            } else {
                [0.0, 0.0, 0.0, 0.0]
            };
            if bg_color[3] > 0.0 {
                let (v, i) =
                    draw::quad_vertices(row_rect.x, row_rect.y, row_rect.w, row_rect.h, bg_color);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }

            let text_y = row_y + (row_h + ui.atlas.line_height) / 2.0 - (2.0);
            ui.text(
                win.x + padding + (4.0),
                text_y,
                &server.name,
                [1.0, 1.0, 1.0, 1.0],
            );

            let count = format!("{}", server.user_count);
            let count_w = ui.atlas.measure_text(&count);
            ui.text(
                win.x + win.w - padding - (4.0) - count_w,
                text_y,
                &count,
                [0.7, 0.7, 0.7, 1.0],
            );
        }

        let btn_y = list_y + list_h + padding;
        let btn_spacing = 8.0;
        let total_btn_w = btn_w * 2.0 + btn_spacing;
        let btn_start_x = win.x + (win.w - total_btn_w) / 2.0;

        let ok_rect = Rect::new(btn_start_x, btn_y, btn_w, btn_h);
        let cancel_rect = Rect::new(btn_start_x + btn_w + btn_spacing, btn_y, btn_w, btn_h);
        let ok = ui.button(OK_ID, ok_rect, &OK_BTN, "OK");
        let cancel = ui.button(CANCEL_ID, cancel_rect, &CANCEL_BTN, "Cancel");

        if ok.clicked() && self.selected_index.is_some() {
            events.push(GameEvent::RequestSelectServer {
                index: self.selected_index.unwrap(),
            });
        }
        if cancel.clicked() {
            events.push(GameEvent::BackToLogin);
        }
    }
}

impl Window for ServerListWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(WIN_TEXTURE) {
            self.win_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(OK_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
            WIN_TEXTURE,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_servers() -> Vec<ServerInfo> {
        vec![
            ServerInfo {
                ip: 0x0100007F,
                port: 6121,
                name: "Loki".into(),
                user_count: 42,
            },
            ServerInfo {
                ip: 0x0100007F,
                port: 6122,
                name: "Thor".into(),
                user_count: 17,
            },
            ServerInfo {
                ip: 0x0100007F,
                port: 6123,
                name: "Odin".into(),
                user_count: 5,
            },
        ]
    }

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn arrow_keys_navigate_selection() {
        let mut win = ServerListWindow::new(make_servers());
        let mut state = StateCache::new();
        assert_eq!(win.selected_index, Some(0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_down = true;
        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(1));

        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(2));

        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(2));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_up = true;
        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(1));

        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(0));

        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(0));
    }

    #[test]
    fn enter_with_selection_emits_request_select_server() {
        let mut win = ServerListWindow::new(make_servers());
        let mut state = StateCache::new();
        win.selected_index = Some(1);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestSelectServer { index: 1 }))
        );
    }

    #[test]
    fn escape_emits_back_to_login() {
        let mut win = ServerListWindow::new(make_servers());
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(events.iter().any(|e| matches!(e, GameEvent::BackToLogin)));
    }
}
