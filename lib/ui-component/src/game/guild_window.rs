use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    FOOTER_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_exp_bar,
    draw_footer, draw_hline, draw_sys_button, draw_titlebar, label_color, text_color,
};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_game::guild::{GUILD_PERM_EXPEL, GUILD_PERM_INVITE, Guild, GuildPosition};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const GUILD_WINDOW_ID: WidgetId = WidgetId(3400);
const CLOSE_BTN_ID: WidgetId = WidgetId(3401);
const TAB_BASE_ID: u32 = 3402;
const MEMBERS_SCROLL: ScrollbarIds = ScrollbarIds {
    up: WidgetId(3430),
    down: WidgetId(3431),
    thumb: WidgetId(3432),
};
const POSITION_SCROLL: ScrollbarIds = ScrollbarIds {
    up: WidgetId(3433),
    down: WidgetId(3434),
    thumb: WidgetId(3435),
};
const NOTICE_SUBJECT_INPUT: WidgetId = WidgetId(3410);
const NOTICE_BODY_INPUT: WidgetId = WidgetId(3411);
const NOTICE_SAVE_BTN: WidgetId = WidgetId(3412);
const LEAVE_BTN: WidgetId = WidgetId(3413);
const POS_TITLE_OK_BTN: WidgetId = WidgetId(3414);
const POS_TITLE_INPUT_ID: WidgetId = WidgetId(3415);
const EMBLEM_EDIT_BTN: WidgetId = WidgetId(3416);
const RELATION_ROW_BASE: u32 = 3520;
const MEMBER_ROW_BASE: u32 = 3450;
const POSITION_ROW_BASE: u32 = 3500;
const SKILL_ROW_BASE: u32 = 3560;

const ROW_H: f32 = 15.0;
const MAX_POSITIONS: usize = 20;
const POSITION_TITLE_MAX: usize = 24;

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const WIN_W: f32 = 360.0;
const TITLE_H: f32 = 17.0;
const TAB_H: f32 = 18.0;
const CONTENT_H: f32 = 250.0;
const FOOTER_H: f32 = 22.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const TAB_COUNT: usize = 6;

const SELECTION_COLOR: [f32; 4] = [0.451, 0.612, 0.937, 1.0];
const OFFLINE_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const ONLINE_ROW_BG: [f32; 4] = [0.933, 1.0, 0.933, 1.0];
const ONLINE_TEXT: [f32; 4] = [0.0, 0.298, 0.0, 1.0];
const HEADER_COLOR: [f32; 4] = [0.14, 0.20, 0.38, 1.0];
const ALLY_COLOR: [f32; 4] = [0.031, 0.192, 0.482, 1.0];
const ENEMY_COLOR: [f32; 4] = [0.482, 0.098, 0.098, 1.0];

const TAB_LABELS: [&str; TAB_COUNT] = ["Info", "Members", "Position", "Skill", "Expel", "Notice"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GuildTab {
    Info = 0,
    Members = 1,
    Position = 2,
    Skill = 3,
    Expel = 4,
    Notice = 5,
}

impl GuildTab {
    fn from_index(i: usize) -> Self {
        match i {
            1 => Self::Members,
            2 => Self::Position,
            3 => Self::Skill,
            4 => Self::Expel,
            5 => Self::Notice,
            _ => Self::Info,
        }
    }
}

pub struct GuildWindow {
    pub open: bool,
    pub has_grf_textures: bool,
    just_opened: bool,
    tab: GuildTab,
    guild: Option<Guild>,
    local_gid: u32,
    local_name: String,
    members_scroll: usize,
    positions_scroll: usize,
    selected_member: Option<u32>,
    notice_subject_input: TextInput,
    notice_body_input: TextInput,
    last_notice: (String, String),
    position_title_edit: Option<(i32, TextInput)>,
}

impl Default for GuildWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl GuildWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            has_grf_textures: false,
            just_opened: false,
            tab: GuildTab::Info,
            guild: None,
            local_gid: 0,
            local_name: String::new(),
            members_scroll: 0,
            positions_scroll: 0,
            selected_member: None,
            notice_subject_input: TextInput::new(60, false),
            notice_body_input: TextInput::new(120, false),
            last_notice: (String::new(), String::new()),
            position_title_edit: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.just_opened = true;
            self.tab = GuildTab::Info;
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.just_opened = true;
        self.tab = GuildTab::Info;
    }

    pub fn sync(&mut self, guild: Option<&Guild>, local_gid: u32, local_name: &str) {
        self.guild = guild.cloned();
        self.local_gid = local_gid;
        if self.local_name != local_name {
            self.local_name = local_name.to_string();
        }
        // Refresh notice editors only when the server-side text actually changes,
        // so in-progress typing is not clobbered every frame.
        if let Some(g) = &self.guild {
            let current = (g.notice_subject.clone(), g.notice_body.clone());
            if current != self.last_notice {
                self.notice_subject_input.text = current.0.clone();
                self.notice_subject_input.cursor_pos = self.notice_subject_input.text.len();
                self.notice_body_input.text = current.1.clone();
                self.notice_body_input.cursor_pos = self.notice_body_input.text.len();
                self.last_notice = current;
            }
        }
    }

    fn is_master(&self) -> bool {
        let Some(g) = &self.guild else {
            return false;
        };
        g.am_i_master
            || (!g.master_name.is_empty() && !self.local_name.is_empty() && g.master_name == self.local_name)
            || g.member_by_gid(self.local_gid).map(|m| m.position_id == 0).unwrap_or(false)
    }

    fn fill(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let (v, i) = draw::quad_vertices(x, y, w, h, color);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }
}

impl Window for GuildWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, TITLE_H + TAB_H + CONTENT_H + FOOTER_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            FOOTER_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for GuildWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        _character: &mut Character,
        _data: &DataTable,
    ) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let mut events = Vec::new();

        if self.just_opened {
            self.just_opened = false;
            events.push(GameEvent::RequestGuildInfoBurst);
        }

        let win_h = TITLE_H + TAB_H + CONTENT_H + FOOTER_H;
        let win = ui.window_at(GUILD_WINDOW_ID, WIN_W, win_h, TITLE_H, 80.0, 50.0);
        let x = win.x;
        let y = win.y;
        ui.interact(GUILD_WINDOW_ID, Rect::new(x, y, WIN_W, win_h));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        let title = match &self.guild {
            Some(g) if !g.name.is_empty() => format!("Guild - {}", g.name),
            _ => "Guild".to_string(),
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

        let tab_y = y + TITLE_H;
        self.build_tab_strip(ui, x, tab_y, tc);

        let content_y = tab_y + TAB_H;
        draw_container(ui, x, content_y, WIN_W, CONTENT_H, grf);

        match self.tab {
            GuildTab::Info => events.extend(self.build_info_tab(ui, x, content_y, tc, grf)),
            GuildTab::Members => events.extend(self.build_members_tab(ui, x, content_y)),
            GuildTab::Position => events.extend(self.build_position_tab(ui, x, content_y, tc)),
            GuildTab::Skill => events.extend(self.build_skill_tab(ui, x, content_y, tc)),
            GuildTab::Expel => self.build_expel_tab(ui, x, content_y, tc),
            GuildTab::Notice => events.extend(self.build_notice_tab(ui, x, content_y, tc)),
        }

        let footer_y = content_y + CONTENT_H;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);
        if !self.is_master() && self.guild.is_some() {
            let leave_rect = Rect::new(x + WIN_W - 90.0, footer_y + 3.0, 84.0, 16.0);
            let resp = ui.text_button(LEAVE_BTN, leave_rect, "Leave Guild");
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                events.push(GameEvent::RequestGuildLeave);
            }
        }
        if let Some(g) = &self.guild {
            ui.text(x + 8.0, footer_y + 14.0, &format!("Skill Points: {}", g.skill_point), tc);
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl GuildWindow {
    fn build_tab_strip(&mut self, ui: &mut UiFrame, x: f32, tab_y: f32, tc: [f32; 4]) {
        let grf = self.has_grf_textures;
        draw_footer(ui, x, tab_y, WIN_W, TAB_H, grf);
        let tab_w = WIN_W / TAB_COUNT as f32;
        for (i, label) in TAB_LABELS.iter().enumerate() {
            let tx = x + i as f32 * tab_w;
            let rect = Rect::new(tx, tab_y, tab_w, TAB_H);
            let selected = self.tab as usize == i;
            if selected {
                Self::fill(ui, tx + 1.0, tab_y + 1.0, tab_w - 2.0, TAB_H - 2.0, SELECTION_COLOR);
            }
            let resp = ui.interact(WidgetId(TAB_BASE_ID + i as u32), rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                self.tab = GuildTab::from_index(i);
            }
            let color = if selected { [1.0, 1.0, 1.0, 1.0] } else { tc };
            ui.text(tx + 6.0, tab_y + TAB_H - 5.0, label, color);
        }
    }

    fn build_info_tab(&self, ui: &mut UiFrame, x: f32, cy: f32, tc: [f32; 4], grf: bool) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let Some(g) = &self.guild else {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return events;
        };
        let lc = label_color(grf);
        let is_master = self.is_master();

        // Emblem placeholder (server BMP pixel rendering is not wired yet).
        Self::fill(ui, x + 10.0, cy + 10.0, 24.0, 24.0, [0.2, 0.2, 0.24, 1.0]);
        ui.text(x + 42.0, cy + 22.0, &g.name, tc);
        ui.text(x + 42.0, cy + 36.0, &format!("Lv. {}", g.level), lc);
        if is_master {
            let edit_rect = Rect::new(x + WIN_W - 96.0, cy + 10.0, 86.0, 16.0);
            let resp = ui.text_button(EMBLEM_EDIT_BTN, edit_rect, "Edit Emblem");
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                events.push(GameEvent::RequestSelectEmblem);
            }
        }

        let mut ly = cy + 56.0;
        let line = 16.0;
        let field = |ui: &mut UiFrame, y: f32, label: &str, value: &str| {
            ui.text(x + 10.0, y, label, lc);
            ui.text(x + 110.0, y, value, tc);
        };
        field(ui, ly, "Master", &g.master_name);
        ly += line;
        field(
            ui,
            ly,
            "Members",
            &format!("{} / {}", g.member_num, g.max_member_num),
        );
        ly += line;
        field(ui, ly, "Avg. Level", &g.avg_level.to_string());
        ly += line;
        field(ui, ly, "Tax Point", &g.point.to_string());
        ly += line;
        field(
            ui,
            ly,
            "Territory",
            if g.manage_land.is_empty() { "-" } else { &g.manage_land },
        );
        ly += line + 2.0;

        ui.text(x + 10.0, ly, "Exp", lc);
        let bar_w = WIN_W - 130.0;
        let pct = if g.max_exp > 0 {
            g.exp as f32 / g.max_exp as f32
        } else {
            0.0
        };
        draw_exp_bar(ui, x + 110.0, ly - 8.0, bar_w, 8.0, pct, grf);
        ui.text_right(
            x + WIN_W - 10.0,
            ly,
            &format!("{} / {}", g.exp, g.max_exp),
            tc,
        );
        ly += line + 4.0;

        draw_hline(ui, x + 8.0, ly - 6.0, WIN_W - 16.0);

        // Alliance / antagonist columns. Master right-clicks an entry to remove it.
        let col_w = (WIN_W - 24.0) / 2.0;
        ui.text(x + 10.0, ly + 8.0, "Alliance", lc);
        ui.text(x + 14.0 + col_w, ly + 8.0, "Antagonist", lc);
        let list_top = ly + 22.0;
        let allies: Vec<(u32, &str)> = g
            .relations
            .iter()
            .filter(|r| r.relation == 0)
            .map(|r| (r.gdid as u32, r.name.as_str()))
            .collect();
        let enemies: Vec<(u32, &str)> = g
            .relations
            .iter()
            .filter(|r| r.relation != 0)
            .map(|r| (r.gdid as u32, r.name.as_str()))
            .collect();
        for (i, (gdid, name)) in allies.iter().take(3).enumerate() {
            let ry = list_top + i as f32 * 14.0;
            ui.text(x + 14.0, ry, name, ALLY_COLOR);
            if is_master {
                let rect = Rect::new(x + 10.0, ry - 10.0, col_w - 4.0, 13.0);
                if ui.interact(WidgetId(RELATION_ROW_BASE + i as u32), rect).right_clicked() {
                    events.push(GameEvent::RequestDeleteGuildRelation { gdid: *gdid, relation: 0 });
                }
            }
        }
        for (i, (gdid, name)) in enemies.iter().take(3).enumerate() {
            let ry = list_top + i as f32 * 14.0;
            ui.text(x + 18.0 + col_w, ry, name, ENEMY_COLOR);
            if is_master {
                let rect = Rect::new(x + 14.0 + col_w, ry - 10.0, col_w - 4.0, 13.0);
                if ui.interact(WidgetId(RELATION_ROW_BASE + 3 + i as u32), rect).right_clicked() {
                    events.push(GameEvent::RequestDeleteGuildRelation { gdid: *gdid, relation: 1 });
                }
            }
        }
        events
    }

    fn build_notice_tab(&mut self, ui: &mut UiFrame, x: f32, cy: f32, tc: [f32; 4]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        if self.guild.is_none() {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return events;
        }
        let lc = [0.45, 0.65, 1.0, 1.0];

        if self.is_master() {
            ui.text(x + 10.0, cy + 14.0, "Subject", lc);
            let subj_rect = Rect::new(x + 10.0, cy + 18.0, WIN_W - 20.0, 18.0);
            ui.text_input(NOTICE_SUBJECT_INPUT, subj_rect, &mut self.notice_subject_input, TextInputBg::Gray);
            ui.text(x + 10.0, cy + 52.0, "Notice", lc);
            let body_rect = Rect::new(x + 10.0, cy + 56.0, WIN_W - 20.0, 18.0);
            ui.text_input(NOTICE_BODY_INPUT, body_rect, &mut self.notice_body_input, TextInputBg::Gray);

            let save_rect = Rect::new(x + WIN_W - 66.0, cy + CONTENT_H - 24.0, 60.0, 18.0);
            let resp = ui.text_button(NOTICE_SAVE_BTN, save_rect, "Save");
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                events.push(GameEvent::RequestSetGuildNotice {
                    subject: self.notice_subject_input.text.trim().to_string(),
                    body: self.notice_body_input.text.trim().to_string(),
                });
            }
            return events;
        }

        let g = self.guild.as_ref().unwrap();
        ui.text(x + 10.0, cy + 18.0, "Subject", lc);
        ui.text(x + 10.0, cy + 34.0, &g.notice_subject, tc);
        draw_hline(ui, x + 8.0, cy + 42.0, WIN_W - 16.0);
        ui.text(x + 10.0, cy + 58.0, "Notice", lc);
        for (i, wrapped) in wrap_text(&g.notice_body, 56).iter().enumerate() {
            ui.text(x + 10.0, cy + 74.0 + i as f32 * 14.0, wrapped, tc);
        }
        events
    }

    fn build_members_tab(&mut self, ui: &mut UiFrame, x: f32, cy: f32) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let Some(g) = &self.guild else {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return events;
        };

        let header_h = 15.0;
        let name_x = x + 24.0;
        let pos_x = x + 108.0;
        let lv_x = x + 176.0;
        let loc_x = x + 202.0;
        let tax_x = x + WIN_W - SCROLLBAR_W - 6.0;
        ui.text(name_x, cy + 11.0, "Name", HEADER_COLOR);
        ui.text(pos_x, cy + 11.0, "Position", HEADER_COLOR);
        ui.text(lv_x, cy + 11.0, "Lv", HEADER_COLOR);
        ui.text(loc_x, cy + 11.0, "Location", HEADER_COLOR);
        ui.text_right(tax_x, cy + 11.0, "Tax", HEADER_COLOR);
        draw_hline(ui, x + 4.0, cy + header_h, WIN_W - 8.0);

        let members = g.sorted_members();
        let list_top = cy + header_h + 2.0;
        let list_h = CONTENT_H - header_h - 4.0;
        let visible = (list_h / ROW_H) as usize;
        let max_scroll = members.len().saturating_sub(visible);
        let has_bar = max_scroll > 0;
        let bar_w = if has_bar { SCROLLBAR_W } else { 0.0 };

        if has_bar {
            let content_rect = Rect::new(x, list_top, WIN_W, list_h);
            self.members_scroll = scrollbar::scrollbar(
                ui,
                MEMBERS_SCROLL,
                self.members_scroll,
                visible,
                max_scroll,
                content_rect,
                x + WIN_W - SCROLLBAR_W,
                list_top,
                list_h,
            );
        } else {
            self.members_scroll = 0;
        }

        for row in 0..visible {
            let idx = self.members_scroll + row;
            let Some(m) = members.get(idx) else { break };
            let row_y = list_top + row as f32 * ROW_H;
            let row_rect = Rect::new(x + 2.0, row_y, WIN_W - 4.0 - bar_w, ROW_H);

            if self.selected_member == Some(m.gid) {
                Self::fill(ui, row_rect.x, row_rect.y, row_rect.w, row_rect.h, SELECTION_COLOR);
            } else if m.online {
                Self::fill(ui, row_rect.x, row_rect.y, row_rect.w, row_rect.h, ONLINE_ROW_BG);
            }

            // Head-sprite cell (filled by the paperdoll pass in a later phase).
            Self::fill(ui, x + 4.0, row_y + 1.0, 13.0, 13.0, [0.2, 0.2, 0.24, 1.0]);

            let text_color = if m.online { ONLINE_TEXT } else { OFFLINE_COLOR };
            let baseline = row_y + 11.0;
            ui.text(name_x, baseline, &m.name, text_color);
            ui.text(pos_x, baseline, &m.position_name, text_color);
            ui.text(lv_x, baseline, &m.level.to_string(), text_color);
            let location = if m.online {
                m.cur_map.trim_end_matches(".gat").to_string()
            } else {
                "(offline)".to_string()
            };
            ui.text(loc_x, baseline, &location, text_color);
            ui.text_right(tax_x, baseline, &m.contribution_exp.to_string(), text_color);

            let resp = ui.interact(WidgetId(MEMBER_ROW_BASE + row as u32), row_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                self.selected_member = Some(m.gid);
            }
            if resp.right_clicked() {
                events.push(GameEvent::ShowGuildMemberMenu {
                    aid: m.aid,
                    gid: m.gid,
                    name: m.name.clone(),
                    x: ui.ctx.mouse_x,
                    y: ui.ctx.mouse_y,
                });
            }
        }
        events
    }

    fn build_position_tab(&mut self, ui: &mut UiFrame, x: f32, cy: f32, tc: [f32; 4]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let is_master = self.is_master();
        let positions: Vec<GuildPosition> = match &self.guild {
            Some(g) => g.positions.clone(),
            None => {
                ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
                return events;
            }
        };

        let header_h = 15.0;
        let rank_x = x + 8.0;
        let title_x = x + 34.0;
        let inv_x = x + 170.0;
        let expel_x = x + 232.0;
        let tax_x = x + WIN_W - SCROLLBAR_W - 6.0;
        ui.text(rank_x, cy + 11.0, "#", HEADER_COLOR);
        ui.text(title_x, cy + 11.0, "Position", HEADER_COLOR);
        ui.text(inv_x, cy + 11.0, "Invite", HEADER_COLOR);
        ui.text(expel_x, cy + 11.0, "Expel", HEADER_COLOR);
        ui.text_right(tax_x, cy + 11.0, "Tax%", HEADER_COLOR);
        draw_hline(ui, x + 4.0, cy + header_h, WIN_W - 8.0);

        let list_top = cy + header_h + 2.0;
        let list_h = CONTENT_H - header_h - 4.0;
        let visible = (list_h / ROW_H) as usize;
        let max_scroll = MAX_POSITIONS.saturating_sub(visible);
        if max_scroll > 0 {
            let content_rect = Rect::new(x, list_top, WIN_W, list_h);
            self.positions_scroll = scrollbar::scrollbar(
                ui,
                POSITION_SCROLL,
                self.positions_scroll,
                visible,
                max_scroll,
                content_rect,
                x + WIN_W - SCROLLBAR_W,
                list_top,
                list_h,
            );
        } else {
            self.positions_scroll = 0;
        }

        for row in 0..visible {
            let idx = self.positions_scroll + row;
            if idx >= MAX_POSITIONS {
                break;
            }
            let row_y = list_top + row as f32 * ROW_H;
            let baseline = row_y + 11.0;
            ui.text(rank_x, baseline, &idx.to_string(), tc);

            let Some(p) = positions.iter().find(|p| p.id as usize == idx).cloned() else {
                ui.text(title_x, baseline, "-", OFFLINE_COLOR);
                continue;
            };

            // Title: inline editor when master is renaming this rank, else label.
            let editing = matches!(&self.position_title_edit, Some((eid, _)) if *eid == p.id);
            if editing {
                let title_rect = Rect::new(title_x, row_y, 110.0, ROW_H - 1.0);
                if let Some((_, input)) = self.position_title_edit.as_mut() {
                    ui.text_input(POS_TITLE_INPUT_ID, title_rect, input, TextInputBg::Gray);
                }
                let ok_rect = Rect::new(title_x + 114.0, row_y, 24.0, ROW_H - 1.0);
                let ok = ui.text_button(POS_TITLE_OK_BTN, ok_rect, "OK");
                if ok.hovered() {
                    ui.any_interactive_hovered = true;
                }
                if ok.clicked() {
                    if let Some((_, input)) = self.position_title_edit.take() {
                        let mut rows = positions.clone();
                        if let Some(r) = rows.iter_mut().find(|r| r.id == p.id) {
                            r.name = input.text.trim().chars().take(POSITION_TITLE_MAX - 1).collect();
                        }
                        events.push(GameEvent::RequestChangePositionInfo { positions: rows });
                    }
                }
            } else {
                ui.text(title_x, baseline, &p.name, tc);
                if is_master {
                    let title_rect = Rect::new(title_x, row_y, 130.0, ROW_H - 1.0);
                    let resp = ui.interact(WidgetId(POSITION_ROW_BASE + (row as u32) * 4 + 2), title_rect);
                    if resp.hovered() {
                        ui.any_interactive_hovered = true;
                    }
                    if resp.clicked() {
                        let mut input = TextInput::new(POSITION_TITLE_MAX, false);
                        input.text = p.name.clone();
                        input.cursor_pos = input.text.len();
                        self.position_title_edit = Some((p.id, input));
                    }
                }
            }

            let invite_on = p.right & GUILD_PERM_INVITE != 0;
            let expel_on = p.right & GUILD_PERM_EXPEL != 0;
            Self::checkbox(ui, inv_x, row_y + 2.0, invite_on);
            Self::checkbox(ui, expel_x, row_y + 2.0, expel_on);
            ui.text_right(tax_x, baseline, &format!("{}%", p.pay_rate), tc);

            // Rank 0 (master) permissions are fixed; only lower ranks are editable.
            if is_master && p.id != 0 {
                let inv_rect = Rect::new(inv_x, row_y + 2.0, 11.0, 11.0);
                let inv_resp = ui.interact(WidgetId(POSITION_ROW_BASE + (row as u32) * 4), inv_rect);
                if inv_resp.hovered() {
                    ui.any_interactive_hovered = true;
                }
                let exp_rect = Rect::new(expel_x, row_y + 2.0, 11.0, 11.0);
                let exp_resp = ui.interact(WidgetId(POSITION_ROW_BASE + (row as u32) * 4 + 1), exp_rect);
                if exp_resp.hovered() {
                    ui.any_interactive_hovered = true;
                }
                if inv_resp.clicked() || exp_resp.clicked() {
                    let mut rows = positions.clone();
                    if let Some(r) = rows.iter_mut().find(|r| r.id == p.id) {
                        if inv_resp.clicked() {
                            r.right ^= GUILD_PERM_INVITE;
                        }
                        if exp_resp.clicked() {
                            r.right ^= GUILD_PERM_EXPEL;
                        }
                    }
                    events.push(GameEvent::RequestChangePositionInfo { positions: rows });
                }
            }
        }
        events
    }

    fn build_skill_tab(&mut self, ui: &mut UiFrame, x: f32, cy: f32, tc: [f32; 4]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let is_master = self.is_master();
        let Some(g) = &self.guild else {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return events;
        };
        if g.skills.is_empty() {
            ui.text(x + 10.0, cy + 20.0, "No guild skills.", OFFLINE_COLOR);
            return events;
        }
        let can_upgrade = is_master && g.skill_point > 0;
        for (i, s) in g.skills.iter().enumerate() {
            let row_y = cy + 6.0 + i as f32 * (ROW_H + 2.0);
            if row_y + ROW_H > cy + CONTENT_H {
                break;
            }
            let baseline = row_y + 12.0;
            let color = if s.level > 0 { tc } else { OFFLINE_COLOR };
            ui.text(x + 26.0, baseline, &s.name, color);
            ui.text_right(x + WIN_W - 60.0, baseline, &format!("Lv {}", s.level), color);
            if can_upgrade && s.upgradable && !s.passive {
                let up_rect = Rect::new(x + WIN_W - 40.0, row_y, 24.0, ROW_H);
                let resp = ui.text_button(WidgetId(SKILL_ROW_BASE + i as u32), up_rect, "+");
                if resp.hovered() {
                    ui.any_interactive_hovered = true;
                }
                if resp.clicked() {
                    events.push(GameEvent::RequestUpgradeGuildSkill { skid: s.skid });
                }
            }
        }
        events
    }

    fn build_expel_tab(&mut self, ui: &mut UiFrame, x: f32, cy: f32, tc: [f32; 4]) {
        let Some(g) = &self.guild else {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return;
        };
        if g.ban_list.is_empty() {
            ui.text(x + 10.0, cy + 20.0, "No expelled members.", OFFLINE_COLOR);
            return;
        }
        let visible = ((CONTENT_H - 8.0) / ROW_H) as usize;
        for (i, b) in g.ban_list.iter().take(visible).enumerate() {
            let baseline = cy + 12.0 + i as f32 * ROW_H;
            ui.text(x + 8.0, baseline, &b.char_name, tc);
            ui.text(x + 120.0, baseline, &b.reason, OFFLINE_COLOR);
        }
    }

    fn checkbox(ui: &mut UiFrame, x: f32, y: f32, on: bool) {
        Self::fill(ui, x, y, 11.0, 11.0, [0.6, 0.6, 0.65, 1.0]);
        Self::fill(ui, x + 1.0, y + 1.0, 9.0, 9.0, [1.0, 1.0, 1.0, 1.0]);
        if on {
            Self::fill(ui, x + 2.0, y + 2.0, 7.0, 7.0, SELECTION_COLOR);
        }
    }
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.split('\n') {
        if raw_line.len() <= max_chars {
            lines.push(raw_line.to_string());
            continue;
        }
        let mut current = String::new();
        for word in raw_line.split(' ') {
            if !current.is_empty() && current.len() + 1 + word.len() > max_chars {
                lines.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    lines
}
