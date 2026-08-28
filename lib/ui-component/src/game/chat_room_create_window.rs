use crate::helper::dropdown::{self, Dropdown};
use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_sys_button,
    draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const CHAT_ROOM_CREATE_WINDOW_ID: WidgetId = WidgetId(1710);
const CLOSE_BTN_ID: WidgetId = WidgetId(1711);
const TITLE_INPUT_ID: WidgetId = WidgetId(1712);
const LIMIT_DROPDOWN_ID: WidgetId = WidgetId(1713);
const RADIO_PUBLIC_ID: WidgetId = WidgetId(1714);
const RADIO_PRIVATE_ID: WidgetId = WidgetId(1715);
const PASSWORD_INPUT_ID: WidgetId = WidgetId(1716);
const OK_ID: WidgetId = WidgetId(1717);
const CANCEL_ID: WidgetId = WidgetId(1718);
const TYPE_DROPDOWN_ID: WidgetId = WidgetId(1719);
const LIMIT_OPTION_BASE: u32 = 1720;
const TYPE_OPTION_BASE: u32 = 1730;

const LIMIT_OPTIONS: [i16; 5] = [20, 12, 8, 4, 2];
const TYPE_OPTIONS: [&str; 1] = ["Chat Room"];

const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;
const RADIO_ON_TEX: &str = ragnarok_resources::ui::RADIOBTN_ON;
const RADIO_OFF_TEX: &str = ragnarok_resources::ui::RADIOBTN_OFF;
const OK_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_OK,
    hover: ragnarok_resources::ui::BTN_OK_A,
    pressed: ragnarok_resources::ui::BTN_OK_B,
};
const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_CANCEL,
    hover: ragnarok_resources::ui::BTN_CANCEL_A,
    pressed: ragnarok_resources::ui::BTN_CANCEL_B,
};

const WIN_W: f32 = 330.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 30.0;
const PAD: f32 = 8.0;
const ROW_H: f32 = 22.0;
const LABEL_W: f32 = 66.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;
const BODY_ROWS: f32 = 3.0;

pub struct ChatRoomCreateWindow {
    has_grf_textures: bool,
    open: bool,
    title: TextInput,
    limit: i16,
    public: bool,
    password: TextInput,
    change_room_id: Option<u32>,
    btn_size: (f32, f32),
    limit_dropdown: Dropdown,
    type_dropdown: Dropdown,
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
            limit: 20,
            public: true,
            password: TextInput::new(8, true),
            change_room_id: None,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            limit_dropdown: Dropdown::default(),
            type_dropdown: Dropdown::default(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn reset(&mut self, title: &str, limit: i16, public: bool) {
        self.title.text = title.to_string();
        self.title.cursor_pos = self.title.text.chars().count();
        self.limit = if LIMIT_OPTIONS.contains(&limit) {
            limit
        } else {
            20
        };
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
        (WIN_W, TITLE_H + (PAD + ROW_H * BODY_ROWS + PAD) + FOOTER_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
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
        ];
        paths.extend(dropdown::grf_texture_paths());
        paths
    }
}

impl InGameWindow for ChatRoomCreateWindow {
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
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();
        self.limit_dropdown.begin_frame();
        self.type_dropdown.begin_frame();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let bg = if grf {
            TextInputBg::Gray
        } else {
            TextInputBg::Default
        };

        let body_h = PAD + ROW_H * BODY_ROWS + PAD;
        let win_h = TITLE_H + body_h + FOOTER_H;
        let win = ui.window_at(
            CHAT_ROOM_CREATE_WINDOW_ID,
            WIN_W,
            win_h,
            TITLE_H,
            220.0,
            120.0,
        );
        let (x, y) = (win.x, win.y);
        ui.interact(CHAT_ROOM_CREATE_WINDOW_ID, Rect::new(x, y, WIN_W, win_h));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        let title_label = if self.change_room_id.is_some() {
            "Chat Room Settings"
        } else {
            "Make a Room"
        };
        ui.text(x + 16.0, y + TITLE_H - 3.0, title_label, tc);

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
        let right_label_x = x + 184.0;
        let right_field_x = right_label_x + 42.0;
        let right_field_w = WIN_W - PAD - (right_field_x - x);
        let bounds = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        let (mx, my) = (ui.ctx.mouse_x, ui.ctx.mouse_y);
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

        // Limit / Type
        ui.text(x + PAD, row_y + 12.0, "Limit :", tc);
        let limit_label = format!("{} People", self.limit);
        let limit_dd = self.limit_dropdown.show(
            ui,
            LIMIT_DROPDOWN_ID,
            Rect::new(field_x, row_y, 84.0, 16.0),
            &limit_label,
            LIMIT_OPTIONS.len(),
            bounds,
            false,
        );
        ui.text(right_label_x, row_y + 12.0, "Type :", tc);
        let type_dd = self.type_dropdown.show(
            ui,
            TYPE_DROPDOWN_ID,
            Rect::new(right_field_x, row_y, right_field_w, 16.0),
            TYPE_OPTIONS[0],
            TYPE_OPTIONS.len(),
            bounds,
            false,
        );
        row_y += ROW_H;

        let overlay_hit = [limit_dd.overlay_rect, type_dd.overlay_rect]
            .into_iter()
            .flatten()
            .any(|r| r.contains(mx, my));

        // Public / Private
        ui.text(x + PAD, row_y + 12.0, "Public :", tc);
        Self::draw_radio(ui, field_x, row_y + 2.0, self.public, grf);
        ui.text(field_x + 16.0, row_y + 12.0, "Public", tc);
        let pub_rect = Rect::new(field_x, row_y, 48.0, ROW_H);
        if !overlay_hit && ui.interact(RADIO_PUBLIC_ID, pub_rect).clicked() {
            self.public = true;
        }
        let priv_x = field_x + 50.0;
        Self::draw_radio(ui, priv_x, row_y + 2.0, !self.public, grf);
        ui.text(priv_x + 16.0, row_y + 12.0, "Private", tc);
        let priv_rect = Rect::new(priv_x, row_y, right_label_x - priv_x, ROW_H);
        if !overlay_hit && ui.interact(RADIO_PRIVATE_ID, priv_rect).clicked() {
            self.public = false;
        }

        // Sign (password)
        ui.text(right_label_x, row_y + 12.0, "Sign :", tc);
        ui.text_input(
            PASSWORD_INPUT_ID,
            Rect::new(right_field_x, row_y, right_field_w, 16.0),
            &mut self.password,
            bg,
        );

        // Footer buttons
        let (btn_w, btn_h) = self.btn_size;
        let footer_y = y + win_h - FOOTER_H;
        let btn_y = footer_y + (FOOTER_H - btn_h) / 2.0;
        let mut bx = x + WIN_W - PAD - btn_w;
        let cancel = ui
            .button(
                CANCEL_ID,
                Rect::new(bx, btn_y, btn_w, btn_h),
                &CANCEL_BTN,
                "cancel",
            )
            .clicked()
            && !overlay_hit;
        bx -= btn_w + 4.0;
        let ok = ui
            .button(OK_ID, Rect::new(bx, btn_y, btn_w, btn_h), &OK_BTN, "OK")
            .clicked()
            && !overlay_hit;

        if let Some(rect) = limit_dd.overlay_rect {
            let labels = limit_option_labels();
            let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            if let Some(idx) =
                self.limit_dropdown
                    .show_overlay(ui, rect, LIMIT_OPTION_BASE, &label_refs)
            {
                self.limit = LIMIT_OPTIONS[idx];
            }
        }
        if let Some(rect) = type_dd.overlay_rect {
            self.type_dropdown
                .show_overlay(ui, rect, TYPE_OPTION_BASE, &TYPE_OPTIONS);
        }

        if cancel {
            self.close();
        } else if ok && !self.title.text.trim().is_empty() {
            let title = self.title.text.trim().to_string();
            let password = if self.public {
                String::new()
            } else {
                self.password.text.clone()
            };
            let limit = self.limit;
            if self.change_room_id.is_some() {
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

fn limit_option_labels() -> [String; LIMIT_OPTIONS.len()] {
    LIMIT_OPTIONS.map(|n| format!("{n} People"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    fn frame(
        win: &mut ChatRoomCreateWindow,
        state: &mut StateCache,
        mx: f32,
        my: f32,
        click: bool,
    ) -> Vec<GameEvent> {
        let mut character = Character::new();
        let data = DataTable::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = mx;
        ctx.mouse_y = my;
        ctx.mouse_clicked = click;
        let mut ui = test_frame(&mut ctx, state);
        win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
    }

    #[test]
    fn open_change_clamps_unknown_limit_to_default() {
        let mut win = ChatRoomCreateWindow::new();
        win.open_change(7, "Room", 99, true);
        assert_eq!(win.limit, 20);
        win.open_change(7, "Room", 8, true);
        assert_eq!(win.limit, 8);
    }

    #[test]
    fn selecting_limit_option_updates_value_and_submits() {
        let mut win = ChatRoomCreateWindow::new();
        win.open_create();
        assert_eq!(win.limit, 20);
        let mut state = StateCache::new();

        // Window opens at (220, 120); Limit dropdown box sits on the second body row.
        frame(&mut win, &mut state, 300.0, 172.0, true);
        assert!(win.limit_dropdown.open);

        // Option list drops below the box; "8 People" is the third entry.
        frame(&mut win, &mut state, 300.0, 223.0, true);
        assert_eq!(win.limit, 8);
        assert!(!win.limit_dropdown.open);

        win.title.text = "Room".into();
        let events = frame(&mut win, &mut state, 475.0, 234.0, true);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestCreateChatRoom { limit: 8, .. })),
            "expected create with limit 8, got {events:?}"
        );
    }
}
