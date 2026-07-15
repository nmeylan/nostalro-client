use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_sys_button,
    draw_titlebar, text_color,
};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::chat_room::ChatRoomMember;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef, word_wrap};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const CHAT_ROOM_MEMBER_WINDOW_ID: WidgetId = WidgetId(1720);
const CLOSE_BTN_ID: WidgetId = WidgetId(1721);
const LEAVE_BTN_ID: WidgetId = WidgetId(1722);
const EDIT_BTN_ID: WidgetId = WidgetId(1723);
const SCROLL_UP_ID: WidgetId = WidgetId(1724);
const SCROLL_DOWN_ID: WidgetId = WidgetId(1725);
const SCROLL_THUMB_ID: WidgetId = WidgetId(1726);
const MEMBER_ROW_BASE_ID: u32 = 1730;

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";
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

pub const OWN_MSG_COLOR: [f32; 4] = [0.55, 1.0, 0.55, 1.0];
pub const OTHER_MSG_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub const SYSTEM_MSG_COLOR: [f32; 4] = [1.0, 0.9, 0.4, 1.0];

const WIN_W: f32 = 330.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 30.0;
const LINE_H: f32 = 14.0;
const PAD: f32 = 6.0;
const GAP: f32 = 6.0;
const MEMBER_PANE_W: f32 = 92.0;
const VISIBLE_ROWS: usize = 8;
const MAX_MESSAGES: usize = 200;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

pub struct ChatRoomMemberWindow {
    has_grf_textures: bool,
    open: bool,
    room_id: u32,
    title: String,
    max_count: i16,
    public: bool,
    members: Vec<ChatRoomMember>,
    messages: Vec<(String, [f32; 4])>,
    local_name: String,
    scroll_offset: usize,
    btn_size: (f32, f32),
}

impl Default for ChatRoomMemberWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatRoomMemberWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            open: false,
            room_id: 0,
            title: String::new(),
            max_count: 0,
            public: true,
            members: Vec::new(),
            messages: Vec::new(),
            local_name: String::new(),
            scroll_offset: 0,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn room_id(&self) -> u32 {
        self.room_id
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn max_count(&self) -> i16 {
        self.max_count
    }
    pub fn public(&self) -> bool {
        self.public
    }

    pub fn open_created(&mut self, room_id: u32, title: &str, max_count: i16, public: bool, local_name: &str) {
        self.open_common(room_id, title, max_count, public, local_name);
        self.members = vec![ChatRoomMember {
            name: local_name.to_string(),
            is_owner: true,
        }];
    }

    pub fn open_joined(
        &mut self,
        room_id: u32,
        title: &str,
        max_count: i16,
        public: bool,
        members: Vec<ChatRoomMember>,
        local_name: &str,
    ) {
        self.open_common(room_id, title, max_count, public, local_name);
        self.members = members;
    }

    fn open_common(&mut self, room_id: u32, title: &str, max_count: i16, public: bool, local_name: &str) {
        self.room_id = room_id;
        self.title = title.to_string();
        self.max_count = max_count;
        self.public = public;
        self.local_name = local_name.to_string();
        self.messages.clear();
        self.scroll_offset = 0;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.members.clear();
        self.messages.clear();
        self.room_id = 0;
    }

    pub fn add_member(&mut self, name: &str) {
        if !self.members.iter().any(|m| m.name == name) {
            self.members.push(ChatRoomMember {
                name: name.to_string(),
                is_owner: false,
            });
        }
    }

    pub fn remove_member(&mut self, name: &str) {
        self.members.retain(|m| m.name != name);
    }

    pub fn set_owner(&mut self, name: &str) {
        for m in &mut self.members {
            m.is_owner = m.name == name;
        }
    }

    pub fn is_local(&self, name: &str) -> bool {
        self.local_name == name
    }

    pub fn has_member(&self, name: &str) -> bool {
        self.members.iter().any(|m| m.name == name)
    }

    pub fn push_message(&mut self, text: String, color: [f32; 4]) {
        self.messages.push((text, color));
        if self.messages.len() > MAX_MESSAGES {
            let overflow = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(0..overflow);
        }
    }

    fn i_am_owner(&self) -> bool {
        self.members
            .iter()
            .any(|m| m.is_owner && m.name == self.local_name)
    }
}

impl Window for ChatRoomMemberWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(CANCEL_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
    }
    fn window_size(&self) -> (f32, f32) {
        let pane_h = PAD + VISIBLE_ROWS as f32 * LINE_H + PAD;
        (WIN_W, TITLE_H + pane_h + FOOTER_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for ChatRoomMemberWindow {
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

        let pane_h = PAD + VISIBLE_ROWS as f32 * LINE_H + PAD;
        let body_h = pane_h;
        let win_h = TITLE_H + body_h + FOOTER_H;
        let win = ui.window_at(CHAT_ROOM_MEMBER_WINDOW_ID, WIN_W, win_h, TITLE_H, 260.0, 120.0);
        let (x, y) = (win.x, win.y);
        ui.interact(CHAT_ROOM_MEMBER_WINDOW_ID, Rect::new(x, y, WIN_W, win_h));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        let header = format!("{} ({}/{})", self.title, self.members.len(), self.max_count);
        ui.text(x + 6.0, y + TITLE_H - 3.0, &header, tc);

        let close_rect = Rect::new(x + WIN_W - 14.0, y + 3.0, 11.0, 11.0);
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            close_rect,
            (11.0, 11.0),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );

        let body_y = y + TITLE_H;
        draw_container(ui, x, body_y, WIN_W, body_h, grf);
        draw_footer(ui, x, y + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        let content_y = body_y + PAD;
        let content_h = VISIBLE_ROWS as f32 * LINE_H;

        // --- Message pane (left) ---
        let msg_x = x + PAD;
        let msg_w = WIN_W - PAD * 2.0 - GAP - MEMBER_PANE_W;
        let (v, i) = draw::quad_vertices(msg_x, content_y, msg_w, content_h, [0.0, 0.0, 0.0, 0.8]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let text_w = msg_w - 4.0;
        let mut rendered: Vec<(String, [f32; 4])> = Vec::new();
        for (text, color) in &self.messages {
            for line in word_wrap(text, text_w, |t| ui.atlas.measure_text(t), false) {
                rendered.push((line, *color));
            }
        }
        let start = rendered.len().saturating_sub(VISIBLE_ROWS);
        for (row, (line, color)) in rendered[start..].iter().enumerate() {
            let ty = content_y + row as f32 * LINE_H + ui.atlas.line_height;
            ui.text(msg_x + 2.0, ty, line, *color);
        }

        // --- Member pane (right) ---
        let i_am_owner = self.i_am_owner();
        let total = self.members.len();
        let max_scroll = total.saturating_sub(VISIBLE_ROWS);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
        let has_scroll = max_scroll > 0;
        let list_x = x + WIN_W - PAD - MEMBER_PANE_W;
        let list_w = MEMBER_PANE_W - if has_scroll { SCROLLBAR_W } else { 0.0 };

        for vis in 0..VISIBLE_ROWS {
            let idx = self.scroll_offset + vis;
            let Some(member) = self.members.get(idx) else {
                break;
            };
            let row_y = content_y + vis as f32 * LINE_H;
            let label = if member.is_owner {
                format!("* {}", member.name)
            } else {
                format!("  {}", member.name)
            };
            ui.text(list_x + 2.0, row_y + ui.atlas.line_height, &label, tc);

            let row_rect = Rect::new(list_x, row_y, list_w, LINE_H);
            let resp = ui.interact(WidgetId(MEMBER_ROW_BASE_ID + vis as u32), row_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.right_clicked() && i_am_owner && member.name != self.local_name {
                events.push(GameEvent::RequestOpenChatMemberMenu {
                    name: member.name.clone(),
                    x: ui.ctx.mouse_x,
                    y: ui.ctx.mouse_y,
                });
            }
        }

        if has_scroll {
            let content_rect = Rect::new(list_x, content_y, MEMBER_PANE_W, content_h);
            self.scroll_offset = scrollbar::scrollbar(
                ui,
                ScrollbarIds {
                    up: SCROLL_UP_ID,
                    down: SCROLL_DOWN_ID,
                    thumb: SCROLL_THUMB_ID,
                },
                self.scroll_offset,
                VISIBLE_ROWS,
                max_scroll,
                content_rect,
                x + WIN_W - PAD - SCROLLBAR_W,
                content_y,
                content_h,
            );
        }

        // --- Footer: Edit (owner only) + Leave ---
        let (btn_w, btn_h) = self.btn_size;
        let footer_y = y + win_h - FOOTER_H;
        let btn_y = footer_y + (FOOTER_H - btn_h) / 2.0;
        let mut bx = x + WIN_W - PAD - btn_w;
        let leave = ui
            .button(LEAVE_BTN_ID, Rect::new(bx, btn_y, btn_w, btn_h), &CANCEL_BTN, "Leave")
            .clicked();
        if i_am_owner {
            bx -= btn_w + 4.0;
            let edit = ui
                .button(EDIT_BTN_ID, Rect::new(bx, btn_y, btn_w, btn_h), &OK_BTN, "Edit")
                .clicked();
            if edit {
                events.push(GameEvent::RequestEditChatRoomSettings);
            }
        }
        if close_resp.clicked() || leave {
            events.push(GameEvent::RequestLeaveChatRoom);
            self.close();
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, owner: bool) -> ChatRoomMember {
        ChatRoomMember {
            name: name.to_string(),
            is_owner: owner,
        }
    }

    #[test]
    fn roster_tracks_join_leave_and_owner() {
        let mut win = ChatRoomMemberWindow::new();
        win.open_joined(
            5,
            "Room",
            20,
            true,
            vec![member("Alice", true), member("Bob", false)],
            "Bob",
        );
        assert!(!win.i_am_owner());
        win.add_member("Carol");
        assert_eq!(win.members.len(), 3);
        assert!(win.has_member("Carol"));
        assert!(!win.has_member("Stranger"));
        win.remove_member("Alice");
        win.set_owner("Bob");
        assert!(win.i_am_owner());
        assert_eq!(win.members.len(), 2);
        assert!(!win.has_member("Alice"));
    }

    #[test]
    fn created_room_makes_local_player_owner() {
        let mut win = ChatRoomMemberWindow::new();
        win.open_created(7, "My Room", 10, false, "Me");
        assert!(win.i_am_owner());
        assert!(win.is_local("Me"));
        assert_eq!(win.max_count(), 10);
    }

    #[test]
    fn message_buffer_is_capped_and_cleared_on_open() {
        let mut win = ChatRoomMemberWindow::new();
        win.open_created(1, "R", 20, true, "Me");
        for n in 0..(MAX_MESSAGES + 20) {
            win.push_message(format!("msg {n}"), OTHER_MSG_COLOR);
        }
        assert_eq!(win.messages.len(), MAX_MESSAGES);
        assert_eq!(win.messages[0].0, format!("msg {}", 20));
        win.open_joined(2, "R2", 20, true, vec![member("Me", true)], "Me");
        assert!(win.messages.is_empty());
    }
}
