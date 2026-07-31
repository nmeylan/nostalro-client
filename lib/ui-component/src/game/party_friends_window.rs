use crate::helper::window_chrome::{
    FOOTER_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer,
    draw_sys_button, draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_game::friends::Friend;
use ragnarok_game::party::{Party, PartyMember};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const PARTY_FRIENDS_WINDOW_ID: WidgetId = WidgetId(2000);
const CLOSE_BTN_ID: WidgetId = WidgetId(2001);
const TAB_FRIEND_BTN_ID: WidgetId = WidgetId(2002);
const TAB_PARTY_BTN_ID: WidgetId = WidgetId(2003);
const NAV_CHAT_ID: WidgetId = WidgetId(2010);
const NAV_SETUP_ID: WidgetId = WidgetId(2011);
const NAV_REMOVE_ID: WidgetId = WidgetId(2012);
const NAV_CREATE_ID: WidgetId = WidgetId(2013);
const NAV_INVITE_ID: WidgetId = WidgetId(2014);
const NAV_LEAVE_ID: WidgetId = WidgetId(2015);
const NAV_LEADER_ID: WidgetId = WidgetId(2016);
const NAV_ADD_ID: WidgetId = WidgetId(2017);
const ROW_BASE_ID: u32 = 2030;

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";
const GRP_ONLINE_TEX: &str = "data/texture/유저인터페이스/basic_interface/grp_online.bmp";
const GRP_LEADER_TEX: &str = "data/texture/유저인터페이스/basic_interface/grp_leader.bmp";
const RADIO_ON_TEX: &str = "data/texture/유저인터페이스/radiobtn_on.bmp";
const RADIO_OFF_TEX: &str = "data/texture/유저인터페이스/radiobtn_off.bmp";

const MESBTN_CHAT: ButtonTextures =
    nav_tex_const("data/texture/유저인터페이스/basic_interface/mesbtn_02.bmp");
const MESBTN_SETUP: ButtonTextures =
    nav_tex_const("data/texture/유저인터페이스/basic_interface/mesbtn_04.bmp");
const MESBTN_REMOVE: ButtonTextures =
    nav_tex_const("data/texture/유저인터페이스/basic_interface/mesbtn_05.bmp");
const MESBTN_CREATE: ButtonTextures =
    nav_tex_const("data/texture/유저인터페이스/basic_interface/mesbtn_08.bmp");
const MESBTN_LEAVE: ButtonTextures =
    nav_tex_const("data/texture/유저인터페이스/basic_interface/mesbtn_09.bmp");
const MESBTN_INVITE: ButtonTextures =
    nav_tex_const("data/texture/유저인터페이스/basic_interface/mesbtn_010.bmp");

const fn nav_tex_const(base: &'static str) -> ButtonTextures {
    ButtonTextures {
        normal: base,
        hover: base,
        pressed: base,
    }
}

const WIN_W: f32 = 240.0;
const TITLE_H: f32 = 17.0;
const CONTENT_H: f32 = 120.0;
const NAV_H: f32 = 20.0;
const FOOTER_H: f32 = 21.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const ROW_H: f32 = 24.0;
const ICON_SIZE: f32 = 16.0;
const HP_BAR_W: f32 = 60.0;
const HP_BAR_H: f32 = 5.0;
const NAV_BTN_W: f32 = 26.0;

const MEMBER_COLOR: [f32; 4] = [0.0, 0.482, 0.482, 1.0];
const LEADER_COLOR: [f32; 4] = [0.031, 0.192, 0.482, 1.0];
const FRIEND_COLOR: [f32; 4] = [0.063, 0.145, 0.314, 1.0];
const SELECTION_COLOR: [f32; 4] = [0.451, 0.612, 0.937, 1.0];
const OFFLINE_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

pub struct PartyFriendsWindow {
    pub open: bool,
    pub has_grf_textures: bool,
    friend_tab: bool,
    selected: Option<usize>,
}

impl Default for PartyFriendsWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl PartyFriendsWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            has_grf_textures: false,
            friend_tab: false,
            selected: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open_party_tab(&mut self) {
        if self.open && !self.friend_tab {
            self.open = false;
        } else {
            self.open = true;
            self.friend_tab = false;
            self.selected = None;
        }
    }

    pub fn open_friend_tab(&mut self) {
        if self.open && self.friend_tab {
            self.open = false;
        } else {
            self.open = true;
            self.friend_tab = true;
            self.selected = None;
        }
    }

    fn is_leader(&self, party: Option<&Party>, local_aid: u32) -> bool {
        party
            .and_then(|p| p.leader_aid())
            .map(|aid| aid == local_aid)
            .unwrap_or(false)
    }

    fn draw_icon(ui: &mut UiFrame, x: f32, y: f32, tex: &str, grf: bool) {
        if grf {
            let (v, i) = draw::quad_vertices(x, y, ICON_SIZE, ICON_SIZE, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        }
    }

    fn fill(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let (v, i) = draw::quad_vertices(x, y, w, h, color);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }

    fn nav_button(
        ui: &mut UiFrame,
        id: WidgetId,
        x: f32,
        y: f32,
        textures: &ButtonTextures,
        label: &str,
        tooltip: &str,
    ) -> bool {
        let rect = Rect::new(x, y, NAV_BTN_W, NAV_H);
        let resp = ui.button(id, rect, textures, label);
        if resp.hovered() {
            ui.any_interactive_hovered = true;
            ui.tooltip(ui.ctx.mouse_x, ui.ctx.mouse_y, tooltip);
        }
        resp.clicked()
    }
}

impl Window for PartyFriendsWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, TITLE_H + CONTENT_H + NAV_H + FOOTER_H)
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            FOOTER_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            GRP_ONLINE_TEX,
            GRP_LEADER_TEX,
            RADIO_ON_TEX,
            RADIO_OFF_TEX,
            MESBTN_CHAT.normal,
            MESBTN_SETUP.normal,
            MESBTN_REMOVE.normal,
            MESBTN_CREATE.normal,
            MESBTN_LEAVE.normal,
            MESBTN_INVITE.normal,
        ]
    }
}

impl InGameWindow for PartyFriendsWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.open
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.open = false;
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }
        let party = ctx.party;
        let friends = &ctx.friends.friends;
        let local_aid = ctx.local_aid;

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let mut events = Vec::new();

        let win_h = TITLE_H + CONTENT_H + NAV_H + FOOTER_H;
        let win = ui.window_at(PARTY_FRIENDS_WINDOW_ID, WIN_W, win_h, TITLE_H, 60.0, 60.0);
        let x = win.x;
        let y = win.y;
        ui.interact(PARTY_FRIENDS_WINDOW_ID, Rect::new(x, y, WIN_W, win_h));

        // Titlebar
        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        let title = if self.friend_tab {
            format!("Friends ({})", friends.len())
        } else {
            match party {
                Some(p) if !p.name.is_empty() => format!("Party  {}", p.name),
                _ => "Party".to_string(),
            }
        };
        ui.text(x + 17.0, y + TITLE_H - 3.0, &title, tc);

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
            self.open = false;
            ui.has_grf_textures = prev_grf;
            return events;
        }

        // Content (white list area)
        let content_y = y + TITLE_H;
        draw_container(ui, x, content_y, WIN_W, CONTENT_H, grf);

        if self.friend_tab {
            self.build_friend_list(ui, friends, x, content_y);
        } else {
            self.build_party_list(ui, party, x, content_y, tc);
        }

        // Navigation bar
        let nav_y = content_y + CONTENT_H;
        draw_footer(ui, x, nav_y, WIN_W, NAV_H, grf);
        events.extend(self.build_nav_bar(ui, party, friends, local_aid, x, nav_y));

        // Footer: tab switch
        let footer_y = nav_y + NAV_H;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);
        self.build_tab_switch(ui, x, footer_y, tc);

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl PartyFriendsWindow {
    fn build_party_list(
        &mut self,
        ui: &mut UiFrame,
        party: Option<&Party>,
        x: f32,
        content_y: f32,
        tc: [f32; 4],
    ) {
        let members: &[PartyMember] = party.map(|p| p.members.as_slice()).unwrap_or(&[]);
        if members.is_empty() {
            ui.text(x + 8.0, content_y + 18.0, "Not in a party", OFFLINE_COLOR);
            return;
        }
        let grf = self.has_grf_textures;
        for (idx, m) in members.iter().enumerate() {
            let row_y = content_y + 4.0 + idx as f32 * ROW_H;
            if row_y + ROW_H > content_y + CONTENT_H {
                break;
            }
            let row_rect = Rect::new(x + 2.0, row_y, WIN_W - 4.0, ROW_H - 2.0);
            let resp = ui.interact(WidgetId(ROW_BASE_ID + idx as u32), row_rect);
            if resp.clicked() {
                self.selected = Some(idx);
            }
            if self.selected == Some(idx) {
                Self::fill(
                    ui,
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    SELECTION_COLOR,
                );
            }

            let icon = if m.leader {
                GRP_LEADER_TEX
            } else {
                GRP_ONLINE_TEX
            };
            if m.online {
                Self::draw_icon(ui, x + 3.0, row_y, icon, grf);
            }

            let name_color = if !m.online {
                OFFLINE_COLOR
            } else if m.leader {
                LEADER_COLOR
            } else {
                MEMBER_COLOR
            };
            ui.text(x + 22.0, row_y + 11.0, &m.name, name_color);
            let map = m.map.trim_end_matches(".gat").trim_end_matches(".rsw");
            ui.text_right(
                x + WIN_W - 8.0,
                row_y + 11.0,
                &format!("({map})"),
                OFFLINE_COLOR,
            );

            if m.online {
                let bar_x = x + 22.0;
                let bar_y = row_y + 14.0;
                Self::fill(
                    ui,
                    bar_x,
                    bar_y,
                    HP_BAR_W,
                    HP_BAR_H,
                    [0.15, 0.15, 0.15, 0.9],
                );
                if let (Some(hp), Some(max_hp)) = (m.hp, m.max_hp)
                    && max_hp > 0
                {
                    let pct = (hp as f32 / max_hp as f32).clamp(0.0, 1.0);
                    let fill = if pct < 0.25 {
                        [0.8, 0.2, 0.2, 1.0]
                    } else {
                        [0.2, 0.6, 0.3, 1.0]
                    };
                    Self::fill(ui, bar_x, bar_y, HP_BAR_W * pct, HP_BAR_H, fill);
                    ui.text(
                        bar_x + HP_BAR_W + 6.0,
                        bar_y + HP_BAR_H + 2.0,
                        &format!("{hp}/{max_hp}"),
                        tc,
                    );
                }
            }
        }
    }

    fn build_friend_list(&mut self, ui: &mut UiFrame, friends: &[Friend], x: f32, content_y: f32) {
        if friends.is_empty() {
            ui.text(x + 8.0, content_y + 18.0, "No friends", OFFLINE_COLOR);
            return;
        }
        let grf = self.has_grf_textures;
        for (idx, f) in friends.iter().enumerate() {
            let row_y = content_y + 4.0 + idx as f32 * 18.0;
            if row_y + 18.0 > content_y + CONTENT_H {
                break;
            }
            let row_rect = Rect::new(x + 2.0, row_y, WIN_W - 4.0, 16.0);
            let resp = ui.interact(WidgetId(ROW_BASE_ID + idx as u32), row_rect);
            if resp.clicked() {
                self.selected = Some(idx);
            }
            if self.selected == Some(idx) {
                Self::fill(
                    ui,
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    SELECTION_COLOR,
                );
            }
            if f.online {
                Self::draw_icon(ui, x + 3.0, row_y, GRP_ONLINE_TEX, grf);
            }
            let color = if f.online {
                FRIEND_COLOR
            } else {
                OFFLINE_COLOR
            };
            ui.text(x + 22.0, row_y + 12.0, &f.name, color);
        }
    }

    fn build_nav_bar(
        &mut self,
        ui: &mut UiFrame,
        party: Option<&Party>,
        friends: &[Friend],
        local_aid: u32,
        x: f32,
        nav_y: f32,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut bx = x + 3.0;
        if self.friend_tab {
            if Self::nav_button(
                ui,
                NAV_ADD_ID,
                bx,
                nav_y,
                &MESBTN_CREATE,
                "Add",
                "Add Friend",
            ) {
                events.push(GameEvent::ShowPartyHelper { mode: 3 });
            }
            bx += NAV_BTN_W;
            if Self::nav_button(ui, NAV_CHAT_ID, bx, nav_y, &MESBTN_CHAT, "Chat", "1:1 Chat") {
                if let Some(name) = self.selected_friend_name(friends) {
                    events.push(GameEvent::RequestWhisper { name });
                }
            }
            bx += NAV_BTN_W;
            if Self::nav_button(
                ui,
                NAV_REMOVE_ID,
                bx,
                nav_y,
                &MESBTN_REMOVE,
                "Del",
                "Delete",
            ) {
                if let Some((aid, gid)) = self.selected_friend_ids(friends) {
                    events.push(GameEvent::RequestDeleteFriend { aid, gid });
                }
            }
            return events;
        }

        let has_party = party.map(|p| !p.members.is_empty()).unwrap_or(false);
        let is_leader = self.is_leader(party, local_aid);
        if !has_party {
            if Self::nav_button(
                ui,
                NAV_CREATE_ID,
                bx,
                nav_y,
                &MESBTN_CREATE,
                "New",
                "Create Party",
            ) {
                events.push(GameEvent::ShowPartyHelper { mode: 0 });
            }
            return events;
        }
        if is_leader
            && Self::nav_button(
                ui,
                NAV_INVITE_ID,
                bx,
                nav_y,
                &MESBTN_INVITE,
                "Inv",
                "Party Invitation",
            )
        {
            events.push(GameEvent::ShowPartyHelper { mode: 1 });
        }
        bx += NAV_BTN_W;
        if is_leader
            && Self::nav_button(
                ui,
                NAV_SETUP_ID,
                bx,
                nav_y,
                &MESBTN_SETUP,
                "Set",
                "Party Setup",
            )
        {
            events.push(GameEvent::ShowPartyHelper { mode: 2 });
        }
        bx += NAV_BTN_W;
        if Self::nav_button(ui, NAV_CHAT_ID, bx, nav_y, &MESBTN_CHAT, "Chat", "1:1 Chat") {
            if let Some(name) = self.selected_member_name(party) {
                events.push(GameEvent::RequestWhisper { name });
            }
        }
        bx += NAV_BTN_W;
        if is_leader
            && Self::nav_button(
                ui,
                NAV_REMOVE_ID,
                bx,
                nav_y,
                &MESBTN_REMOVE,
                "Kick",
                "Expel from party",
            )
        {
            if let Some((aid, name)) = self.selected_member_kick(party, local_aid) {
                events.push(GameEvent::RequestExpelMember { aid, name });
            }
        }
        bx += NAV_BTN_W;
        if is_leader
            && Self::nav_button(
                ui,
                NAV_LEADER_ID,
                bx,
                nav_y,
                &MESBTN_SETUP,
                "Lead",
                "Delegate leader",
            )
        {
            if let Some((aid, _)) = self.selected_member_kick(party, local_aid) {
                events.push(GameEvent::RequestChangePartyLeader { aid });
            }
        }
        bx += NAV_BTN_W;
        let leave_label = if is_leader { "End" } else { "Out" };
        let leave_tip = if is_leader {
            "Disband party"
        } else {
            "Leave Party"
        };
        if Self::nav_button(
            ui,
            NAV_LEAVE_ID,
            bx,
            nav_y,
            &MESBTN_LEAVE,
            leave_label,
            leave_tip,
        ) {
            events.push(GameEvent::RequestLeaveParty);
        }
        events
    }

    fn build_tab_switch(&mut self, ui: &mut UiFrame, x: f32, footer_y: f32, tc: [f32; 4]) {
        let grf = self.has_grf_textures;
        let by = footer_y + 3.0;
        // Friends radio
        let friend_rect = Rect::new(x + 6.0, by, 60.0, 15.0);
        Self::draw_radio(ui, friend_rect.x, friend_rect.y, self.friend_tab, grf);
        ui.text(friend_rect.x + 16.0, by + 11.0, "Friends", tc);
        let fresp = ui.interact(TAB_FRIEND_BTN_ID, friend_rect);
        if fresp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if fresp.clicked() {
            self.friend_tab = true;
            self.selected = None;
        }
        // Party radio
        let party_rect = Rect::new(x + 76.0, by, 60.0, 15.0);
        Self::draw_radio(ui, party_rect.x, party_rect.y, !self.friend_tab, grf);
        ui.text(party_rect.x + 16.0, by + 11.0, "Party", tc);
        let presp = ui.interact(TAB_PARTY_BTN_ID, party_rect);
        if presp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if presp.clicked() {
            self.friend_tab = false;
            self.selected = None;
        }
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
            Self::fill(ui, x, y + 2.0, 10.0, 10.0, c);
        }
    }

    fn selected_member<'p>(&self, party: Option<&'p Party>) -> Option<&'p PartyMember> {
        let idx = self.selected?;
        party?.members.get(idx)
    }

    fn selected_member_name(&self, party: Option<&Party>) -> Option<String> {
        self.selected_member(party).map(|m| m.name.clone())
    }

    fn selected_member_kick(&self, party: Option<&Party>, local_aid: u32) -> Option<(u32, String)> {
        let m = self.selected_member(party)?;
        if m.aid == local_aid {
            return None;
        }
        Some((m.aid, m.name.clone()))
    }

    fn selected_friend<'f>(&self, friends: &'f [Friend]) -> Option<&'f Friend> {
        friends.get(self.selected?)
    }

    fn selected_friend_name(&self, friends: &[Friend]) -> Option<String> {
        self.selected_friend(friends).map(|f| f.name.clone())
    }

    fn selected_friend_ids(&self, friends: &[Friend]) -> Option<(u32, u32)> {
        self.selected_friend(friends).map(|f| (f.aid, f.gid))
    }
}
