use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_sys_button,
    draw_titlebar, text_color,
};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const CHAT_ROOM_CREATE_WINDOW_ID: WidgetId = WidgetId(1710);
const CLOSE_BTN_ID: WidgetId = WidgetId(1711);
const TITLE_INPUT_ID: WidgetId = WidgetId(1712);
const LIMIT_INPUT_ID: WidgetId = WidgetId(1713);
const RADIO_PUBLIC_ID: WidgetId = WidgetId(1714);
const RADIO_PRIVATE_ID: WidgetId = WidgetId(1715);
const PASSWORD_INPUT_ID: WidgetId = WidgetId(1716);
const OK_ID: WidgetId = WidgetId(1717);
const CANCEL_ID: WidgetId = WidgetId(1718);

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";
const RADIO_ON_TEX: &str = "data/texture/유저인터페이스/radiobtn_on.bmp";
const RADIO_OFF_TEX: &str = "data/texture/유저인터페이스/radiobtn_off.bmp";
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

const WIN_W: f32 = 240.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 30.0;
const PAD: f32 = 8.0;
const ROW_H: f32 = 22.0;
const LABEL_W: f32 = 66.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;
const MAX_LIMIT: i16 = 127;

pub struct ChatRoomCreateWindow {
    has_grf_textures: bool,
    open: bool,
    title: TextInput,
    limit: TextInput,
    public: bool,
    password: TextInput,
    change_room_id: Option<u32>,
    btn_size: (f32, f32),
}

impl Default for ChatRoomCreateWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatRoomCreateWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            open: false,
            title: TextInput::new(60, false),
            limit: TextInput::new(3, false),
            public: true,
            password: TextInput::new(8, true),
            change_room_id: None,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn reset(&mut self, title: &str, limit: i16, public: bool) {
        self.title.text = title.to_string();
        self.title.cursor_pos = self.title.text.chars().count();
        self.limit.text = limit.to_string();
        self.limit.cursor_pos = self.limit.text.chars().count();
        self.public = public;
        self.password.text.clear();
        self.password.cursor_pos = 0;
    }

    pub fn open_create(&mut self) {
        self.reset("", 20, true);
        self.change_room_id = None;
        self.open = true;
    }

    pub fn open_change(&mut self, room_id: u32, title: &str, limit: i16, public: bool) {
        self.reset(title, limit, public);
        self.change_room_id = Some(room_id);
        self.open = true;
    }

    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open_create();
        }
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    fn parsed_limit(&self) -> Option<i16> {
        self.limit
            .text
            .trim()
            .parse::<i16>()
            .ok()
            .filter(|n| (1..=MAX_LIMIT).contains(n))
    }

    fn draw_radio(ui: &mut UiFrame, x: f32, y: f32, on: bool, grf: bool) {
        if grf {
            let tex = if on { RADIO_ON_TEX } else { RADIO_OFF_TEX };
            let (v, i) = draw::quad_vertices(x, y, 12.0, 12.0, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            let c = if on {
                [0.3, 0.6, 0.9, 1.0]
            } else {
                [0.3, 0.3, 0.35, 1.0]
            };
            let (v, i) = draw::quad_vertices(x, y + 2.0, 10.0, 10.0, c);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
    }
}

impl Window for ChatRoomCreateWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(OK_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, TITLE_H + (PAD + ROW_H * 4.0 + PAD) + FOOTER_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            RADIO_ON_TEX,
            RADIO_OFF_TEX,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ]
    }
}

impl InGameWindow for ChatRoomCreateWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        _character: &mut Character,
        _data: &DataTable,
    ) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let bg = if grf {
            TextInputBg::Gray
        } else {
            TextInputBg::Default
        };

        let body_h = PAD + ROW_H * 4.0 + PAD;
        let win_h = TITLE_H + body_h + FOOTER_H;
        let win = ui.window_at(CHAT_ROOM_CREATE_WINDOW_ID, WIN_W, win_h, TITLE_H, 220.0, 120.0);
        let (x, y) = (win.x, win.y);
        ui.interact(CHAT_ROOM_CREATE_WINDOW_ID, Rect::new(x, y, WIN_W, win_h));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        let title_label = if self.change_room_id.is_some() {
            "Chat Room Settings"
        } else {
            "Make a Room"
        };
        ui.text(x + 6.0, y + TITLE_H - 3.0, title_label, tc);

        let close_rect = Rect::new(
            x + WIN_W - CLOSE_BTN_SIZE - 3.0,
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
            return events;
        }

        let body_y = y + TITLE_H;
        draw_container(ui, x, body_y, WIN_W, body_h, grf);
        draw_footer(ui, x, y + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        let field_x = x + PAD + LABEL_W;
        let field_w = WIN_W - PAD * 2.0 - LABEL_W;
        let mut row_y = body_y + PAD;

        // Title
        ui.text(x + PAD, row_y + 12.0, "Title :", tc);
        ui.text_input(
            TITLE_INPUT_ID,
            Rect::new(field_x, row_y, field_w, 16.0),
            &mut self.title,
            bg,
        );
        row_y += ROW_H;

        // Limit
        ui.text(x + PAD, row_y + 12.0, "Limit :", tc);
        self.limit.text.retain(|c| c.is_ascii_digit());
        ui.text_input(
            LIMIT_INPUT_ID,
            Rect::new(field_x, row_y, 40.0, 16.0),
            &mut self.limit,
            bg,
        );
        row_y += ROW_H;

        // Public / Private
        ui.text(x + PAD, row_y + 12.0, "Type :", tc);
        Self::draw_radio(ui, field_x, row_y + 2.0, self.public, grf);
        ui.text(field_x + 16.0, row_y + 12.0, "Public", tc);
        let pub_rect = Rect::new(field_x, row_y, 66.0, ROW_H);
        if ui.interact(RADIO_PUBLIC_ID, pub_rect).clicked() {
            self.public = true;
        }
        let priv_x = field_x + 74.0;
        Self::draw_radio(ui, priv_x, row_y + 2.0, !self.public, grf);
        ui.text(priv_x + 16.0, row_y + 12.0, "Private", tc);
        let priv_rect = Rect::new(priv_x, row_y, 70.0, ROW_H);
        if ui.interact(RADIO_PRIVATE_ID, priv_rect).clicked() {
            self.public = false;
        }
        row_y += ROW_H;

        // Password (private only)
        ui.text(x + PAD, row_y + 12.0, "Password :", tc);
        if self.public {
            self.password.text.clear();
            self.password.cursor_pos = 0;
        } else {
            ui.text_input(
                PASSWORD_INPUT_ID,
                Rect::new(field_x, row_y, field_w, 16.0),
                &mut self.password,
                bg,
            );
        }

        // Footer buttons
        let (btn_w, btn_h) = self.btn_size;
        let footer_y = y + win_h - FOOTER_H;
        let btn_y = footer_y + (FOOTER_H - btn_h) / 2.0;
        let mut bx = x + WIN_W - PAD - btn_w;
        let cancel = ui
            .button(CANCEL_ID, Rect::new(bx, btn_y, btn_w, btn_h), &CANCEL_BTN, "cancel")
            .clicked();
        bx -= btn_w + 4.0;
        let ok = ui
            .button(OK_ID, Rect::new(bx, btn_y, btn_w, btn_h), &OK_BTN, "OK")
            .clicked();

        if cancel {
            self.close();
        } else if ok
            && let Some(limit) = self.parsed_limit()
            && !self.title.text.trim().is_empty()
        {
            let title = self.title.text.trim().to_string();
            let password = if self.public {
                String::new()
            } else {
                self.password.text.clone()
            };
            if let Some(room_id) = self.change_room_id {
                let _ = room_id;
                events.push(GameEvent::RequestChangeChatRoom {
                    title,
                    limit,
                    public: self.public,
                    password,
                });
            } else {
                events.push(GameEvent::RequestCreateChatRoom {
                    title,
                    limit,
                    public: self.public,
                    password,
                });
            }
            self.close();
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_room_clears_password_on_submit_state() {
        let mut win = ChatRoomCreateWindow::new();
        win.open_create();
        assert!(win.public);
        assert_eq!(win.parsed_limit(), Some(20));
    }

    #[test]
    fn limit_out_of_range_is_rejected() {
        let mut win = ChatRoomCreateWindow::new();
        win.open_create();
        win.limit.text = "0".into();
        assert_eq!(win.parsed_limit(), None);
        win.limit.text = "200".into();
        assert_eq!(win.parsed_limit(), None);
        win.limit.text = "12".into();
        assert_eq!(win.parsed_limit(), Some(12));
    }
}
