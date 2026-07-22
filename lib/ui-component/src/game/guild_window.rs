use crate::helper::CHECKBOX;
use crate::helper::dropdown::{self, Dropdown};
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    FOOTER_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_footer, draw_sys_button,
    draw_titlebar,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_game::guild::{GUILD_PERM_EXPEL, GUILD_PERM_INVITE, Guild, GuildPosition};
use ragnarok_game::job_class::job_class_name;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const GUILD_WINDOW_ID: WidgetId = WidgetId(3400);
const CLOSE_BTN_ID: WidgetId = WidgetId(3401);
const TAB_BASE_ID: u32 = 3402;
const NOTICE_SUBJECT_INPUT: WidgetId = WidgetId(3410);
const NOTICE_BODY_INPUT: WidgetId = WidgetId(3411);
const EMBLEM_EDIT_BTN: WidgetId = WidgetId(3416);
const FOOTER_OK_BTN: WidgetId = WidgetId(3417);
const SKILL_APPLY_BTN: WidgetId = WidgetId(3418);
const SKILL_RESET_BTN: WidgetId = WidgetId(3419);
const MEMBERS_SCROLL: ScrollbarIds = ScrollbarIds {
    up: WidgetId(3430),
    down: WidgetId(3431),
    thumb: WidgetId(3432),
};
const SKILL_SCROLL: ScrollbarIds = ScrollbarIds {
    up: WidgetId(3433),
    down: WidgetId(3434),
    thumb: WidgetId(3435),
};
const POSITION_SCROLL: ScrollbarIds = ScrollbarIds {
    up: WidgetId(3436),
    down: WidgetId(3437),
    thumb: WidgetId(3438),
};
const MEMBER_ROW_BASE: u32 = 3450;
const RELATION_ROW_BASE: u32 = 3520;
const POS_TITLE_INPUT_BASE: u32 = 3600;
const POS_TAX_INPUT_BASE: u32 = 3620;
const POS_INVITE_BASE: u32 = 3640;
const POS_PUNISH_BASE: u32 = 3660;
const SKILL_UP_BASE: u32 = 3700;
const SKILL_DOWN_BASE: u32 = 3720;
const MEMBER_POS_DROPDOWN_BASE: u32 = 3740;
const MEMBER_POS_OPTION_BASE: u32 = 3760;

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";
const GRP_ONLINE_TEX: &str = "data/texture/유저인터페이스/basic_interface/grp_online.bmp";

const EDIT_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_edit.bmp",
    hover: "data/texture/유저인터페이스/btn_edit_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_edit_b.bmp",
};
const OK_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_ok.bmp",
    hover: "data/texture/유저인터페이스/btn_ok_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_ok_b.bmp",
};
const APPLY_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_apply.bmp",
    hover: "data/texture/유저인터페이스/btn_apply_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_apply_b.bmp",
};
const RESET_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_reset.bmp",
    hover: "data/texture/유저인터페이스/btn_reset_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_reset_b.bmp",
};
const SKILL_UP_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/skill_up_a.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/skill_up_b.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/skill_up_c.bmp",
};

const WIN_W: f32 = 500.0;
const TITLE_H: f32 = 17.0;
const TAB_H: f32 = 23.0;
const CONTENT_H: f32 = 244.0;
const FOOTER_H: f32 = 27.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const TAB_COUNT: usize = 6;

const MEMBER_ROW_H: f32 = 35.0;
const POS_ROW_H: f32 = 20.0;
const SKILL_ROW_H: f32 = 28.0;
const HEADER_H: f32 = 18.0;
const POSITION_TITLE_MAX: usize = 24;

// robrowser palette
const TAB_BAR_BG: [f32; 4] = [0.71, 0.714, 0.71, 1.0]; // #b5b6b5
const TAB_INACTIVE: [f32; 4] = [0.808, 0.808, 0.808, 1.0]; // #cecece
const TAB_ACTIVE: [f32; 4] = [1.0, 1.0, 1.0, 1.0]; // #fff
const BORDER: [f32; 4] = [0.761, 0.761, 0.761, 1.0]; // #c2c2c2
const SELECTION_COLOR: [f32; 4] = [0.451, 0.62, 0.937, 1.0]; // #739eef
const ONLINE_ROW_BG: [f32; 4] = [0.933, 1.0, 0.933, 1.0]; // #efe
const LISTBOX_BG: [f32; 4] = [0.808, 0.808, 0.808, 1.0]; // #cecece
const INPUT_BG: [f32; 4] = [0.933, 0.933, 0.933, 1.0]; // #eee
const EMBLEM_BG: [f32; 4] = [0.439, 0.612, 0.906, 1.0]; // #709ce7
const TENDENCY_BORDER: [f32; 4] = [0.808, 0.812, 0.808, 1.0]; // #cecfce
const TENDENCY_AXIS: [f32; 4] = [0.259, 0.38, 0.647, 1.0]; // #4261a5
const TEXT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const OFFLINE_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

const TAB_LABELS: [&str; TAB_COUNT] = [
    "Guild Info",
    "Guildsmen Info",
    "Position",
    "Guild Skill",
    "Expel History",
    "Guild Notice",
];

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

    /// `type` field for a `CZ_REQ_GUILD_MENU` (0x14f) refresh of this tab's data,
    /// or `None` for tabs the server doesn't answer through that request.
    fn request_type(self) -> Option<i32> {
        match self {
            Self::Info => Some(0),
            Self::Members => Some(1),
            Self::Position => Some(2),
            Self::Skill => Some(3),
            Self::Expel => Some(4),
            Self::Notice => None,
        }
    }
}

struct PosEdit {
    id: i32,
    ranking: i32,
    right: i32,
    title: TextInput,
    tax: TextInput,
}

pub struct GuildWindow {
    pub open: bool,
    pub has_grf_textures: bool,
    just_opened: bool,
    tab: GuildTab,
    members_scroll: usize,
    positions_scroll: usize,
    skills_scroll: usize,
    selected_member: Option<u32>,
    notice_subject_input: TextInput,
    notice_body_input: TextInput,
    last_notice: (String, String),
    pos_edits: Vec<PosEdit>,
    pos_dirty: bool,
    skill_pending: Vec<(u16, i16)>,
    member_head_slots: Vec<(u32, [f32; 2])>,
    head_insert_index: Option<usize>,
    open_member_dropdown: Option<u32>,
    member_pos_overlay: Option<(u32, u32, Rect, Vec<(i32, String)>)>,
    member_pos_dropdown: Dropdown,
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
            members_scroll: 0,
            positions_scroll: 0,
            skills_scroll: 0,
            selected_member: None,
            notice_subject_input: TextInput::new(60, false),
            notice_body_input: TextInput::new(120, false),
            last_notice: (String::new(), String::new()),
            pos_edits: Vec::new(),
            pos_dirty: false,
            skill_pending: Vec::new(),
            member_head_slots: Vec::new(),
            head_insert_index: None,
            open_member_dropdown: None,
            member_pos_overlay: None,
            member_pos_dropdown: Dropdown::default(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Screen-space (gid, cell-center) pairs for visible member rows, so the
    /// client can composite each member's head sprite into the roster.
    pub fn member_head_slots(&self) -> &[(u32, [f32; 2])] {
        &self.member_head_slots
    }

    /// Draw-call index, within this window's own stream, where the client must
    /// splice the member head sprites so they layer at the window's depth
    /// instead of on top of every other window.
    pub fn head_insert_index(&self) -> Option<usize> {
        self.head_insert_index
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.reset_view();
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.reset_view();
    }

    fn reset_view(&mut self) {
        self.just_opened = true;
        self.tab = GuildTab::Info;
        self.pos_dirty = false;
        self.skill_pending.clear();
        self.open_member_dropdown = None;
    }

    fn seed_from_guild(&mut self, guild: Option<&Guild>) {
        if let Some(g) = guild {
            let current = (g.notice_subject.clone(), g.notice_body.clone());
            if current != self.last_notice {
                self.notice_subject_input.text = current.0.clone();
                self.notice_subject_input.cursor_pos = self.notice_subject_input.text.chars().count();
                self.notice_body_input.text = current.1.clone();
                self.notice_body_input.cursor_pos = self.notice_body_input.text.chars().count();
                self.last_notice = current;
            }
        }
        if !self.pos_dirty {
            self.rebuild_pos_edits(guild);
        }
    }

    fn rebuild_pos_edits(&mut self, guild: Option<&Guild>) {
        let Some(g) = guild else {
            self.pos_edits.clear();
            return;
        };
        let mut positions = g.positions.clone();
        positions.sort_by_key(|p| p.id);
        self.pos_edits = positions
            .iter()
            .map(|p| {
                let mut title = TextInput::new(POSITION_TITLE_MAX, false);
                title.text = p.name.clone();
                title.cursor_pos = title.text.chars().count();
                let mut tax = TextInput::new(3, false).with_numeric_only(true);
                tax.text = p.pay_rate.to_string();
                tax.cursor_pos = tax.text.chars().count();
                PosEdit {
                    id: p.id,
                    ranking: p.ranking,
                    right: p.right,
                    title,
                    tax,
                }
            })
            .collect();
    }

    fn staged_positions(&self) -> Vec<GuildPosition> {
        self.pos_edits
            .iter()
            .map(|e| GuildPosition {
                id: e.id,
                name: e.title.text.trim().chars().take(POSITION_TITLE_MAX - 1).collect(),
                right: e.right,
                ranking: e.ranking,
                pay_rate: e.tax.text.trim().parse().unwrap_or(0),
            })
            .collect()
    }

    fn is_master(&self, guild: Option<&Guild>, local_gid: u32, local_name: &str) -> bool {
        let Some(g) = guild else {
            return false;
        };
        g.am_i_master
            || (!g.master_name.is_empty()
                && !local_name.is_empty()
                && g.master_name == local_name)
            || g.member_by_gid(local_gid)
                .map(|m| m.position_id == 0)
                .unwrap_or(false)
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
            GRP_ONLINE_TEX,
            EDIT_BTN.normal,
            EDIT_BTN.hover,
            EDIT_BTN.pressed,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            APPLY_BTN.normal,
            APPLY_BTN.hover,
            APPLY_BTN.pressed,
            RESET_BTN.normal,
            RESET_BTN.hover,
            RESET_BTN.pressed,
            SKILL_UP_BTN.normal,
            SKILL_UP_BTN.hover,
            SKILL_UP_BTN.pressed,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths.extend(dropdown::grf_texture_paths());
        paths
    }
}

impl InGameWindow for GuildWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        ctx: &mut BuildCtx,
    ) -> Vec<GameEvent> {
        let data = ctx.data;
        if !self.open {
            return Vec::new();
        }
        let guild = ctx.guild;
        let local_gid = ctx.local_gid;
        let is_master = self.is_master(guild, local_gid, &ctx.character.name);
        self.seed_from_guild(guild);

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let mut events = Vec::new();
        self.member_head_slots.clear();
        self.head_insert_index = None;
        self.member_pos_overlay = None;
        self.member_pos_dropdown.begin_frame();

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
        if let Some(tab) = self.build_tab_strip(ui, x, tab_y) {
            if let Some(atype) = tab.request_type() {
                events.push(GameEvent::RequestGuildMenu { atype });
            }
        }

        let content_y = tab_y + TAB_H;
        Self::fill(ui, x, content_y, WIN_W, CONTENT_H, TAB_ACTIVE);
        Self::fill(ui, x, content_y, WIN_W, 1.0, TAB_INACTIVE);

        match self.tab {
            GuildTab::Info => events.extend(self.build_info_tab(ui, guild, is_master, x, content_y)),
            GuildTab::Members => events.extend(self.build_members_tab(ui, guild, is_master, x, content_y)),
            GuildTab::Position => self.build_position_tab(ui, guild.is_some(), is_master, x, content_y),
            GuildTab::Skill => events.extend(self.build_skill_tab(ui, guild, is_master, x, content_y, data)),
            GuildTab::Expel => self.build_expel_tab(ui, guild, x, content_y),
            GuildTab::Notice => self.build_notice_tab(ui, guild, is_master, x, content_y),
        }

        let footer_y = content_y + CONTENT_H;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);
        events.extend(self.build_footer(ui, guild.is_some(), is_master, x, footer_y));

        self.head_insert_index = Some(ui.draw_calls.len());
        events.extend(self.render_member_pos_overlay(ui));

        ui.has_grf_textures = prev_grf;
        events
    }
}

const TAB_WIDTH: [f32; 6] = [60.0, 100.0, 75.0, 90.0, 80.0, 95.0];
const TAB_PAD: [f32; 6] = [0.0, 10.0, 15.0, 15.0, 5.0, 10.0];
impl GuildWindow {
    fn build_tab_strip(&mut self, ui: &mut UiFrame, x: f32, tab_y: f32) -> Option<GuildTab> {
        Self::fill(ui, x, tab_y, WIN_W, TAB_H, TAB_BAR_BG);
        let mut clicked_tab = None;
        let mut start_x = x;
        for (i, label) in TAB_LABELS.iter().enumerate() {
            let tx = start_x;
            let rect = Rect::new(tx, tab_y + 1.0, TAB_WIDTH[i], TAB_H - 2.0);
            let selected = self.tab as usize == i;
            Self::fill(
                ui,
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                if selected { TAB_ACTIVE } else { TAB_INACTIVE },
            );
            let resp = ui.interact(WidgetId(TAB_BASE_ID + i as u32), rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() && !selected {
                self.tab = GuildTab::from_index(i);
                self.pos_dirty = false;
                self.open_member_dropdown = None;
                clicked_tab = Some(self.tab);
            }
            ui.text(tx + 4.0 + TAB_PAD[i], tab_y + TAB_H - 7.0, label, TEXT);
            start_x = start_x + TAB_WIDTH[i];
        }
        clicked_tab
    }

    fn build_footer(&mut self, ui: &mut UiFrame, guild_present: bool, is_master: bool, x: f32, footer_y: f32) -> Vec<GameEvent> {
        let mut events = Vec::new();
        if !guild_present || !is_master {
            return events;
        }
        let show_ok = matches!(self.tab, GuildTab::Position | GuildTab::Notice);
        if !show_ok {
            return events;
        }
        let ok_rect = Rect::new(x + WIN_W - 46.0, footer_y + 4.0, 42.0, 20.0);
        let resp = ui.button(FOOTER_OK_BTN, ok_rect, &OK_BTN, "OK");
        if resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if resp.clicked() {
            match self.tab {
                GuildTab::Position => {
                    events.push(GameEvent::RequestChangePositionInfo {
                        positions: self.staged_positions(),
                    });
                    self.pos_dirty = false;
                }
                GuildTab::Notice => {
                    events.push(GameEvent::RequestSetGuildNotice {
                        subject: self.notice_subject_input.text.trim().to_string(),
                        body: self.notice_body_input.text.trim().to_string(),
                    });
                }
                _ => {}
            }
        }
        events
    }

    fn build_info_tab(&self, ui: &mut UiFrame, guild: Option<&Guild>, is_master: bool, x: f32, cy: f32) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let Some(g) = guild else {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return events;
        };
        let grf = ui.has_grf_textures;
        let lx = x + 9.0;
        let online = g.members.iter().filter(|m| m.online).count();

        ui.text(lx, cy + 20.0, &format!("Guild Name : {}", g.name), TEXT);
        ui.text(lx, cy + 36.0, &format!("Guild lvl : {}", g.level), TEXT);
        ui.text(lx, cy + 52.0, &format!("Guild Master : {}", g.master_name), TEXT);
        let members_text = format!("Members : {} / {}", g.member_num, g.max_member_num);
        ui.text(lx, cy + 68.0, &members_text, TEXT);
        let online_icon_x = lx + ui.atlas.measure_text(&members_text) + 6.0;
        if grf {
            let (v, i) = draw::quad_vertices(online_icon_x, cy + 57.0, 15.0, 15.0, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(GRP_ONLINE_TEX.to_string()),
            });
        }
        ui.text(online_icon_x + 18.0, cy + 68.0, &online.to_string(), TEXT);
        ui.text(
            lx,
            cy + 84.0,
            &format!("Avg.lvl of Members : {}", g.avg_level),
            TEXT,
        );
        let land = if g.manage_land.is_empty() { "-" } else { &g.manage_land };
        ui.text(lx, cy + 100.0, &format!("Territory : {land}"), TEXT);

        ui.text(lx, cy + 137.0, "Tendency :", TEXT);
        Self::draw_tendency(ui, lx + 5.0, cy + 150.0);

        // Right column.
        let rx = x + 301.0;
        ui.text(rx, cy + 20.0, &format!("EXP : {} / {}", g.exp, g.max_exp), TEXT);

        ui.text(rx, cy + 52.0, "emblem", TEXT);
        Self::fill(ui, x + 400.0, cy + 32.0, 24.0, 24.0, EMBLEM_BG);
        if g.emblem_bmp.is_some() {
            let key = ragnarok_game::guild::emblem_texture_key(g.gdid, g.emblem_version);
            let (v, i) = draw::quad_vertices(x + 400.0, cy + 32.0, 24.0, 24.0, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(key),
            });
        }
        if is_master {
            let edit_rect = Rect::new(x + 444.0, cy + 36.0, 42.0, 20.0);
            let resp = ui.button(EMBLEM_EDIT_BTN, edit_rect, &EDIT_BTN, "Edit");
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                events.push(GameEvent::RequestSelectEmblem);
            }
        }

        ui.text(rx, cy + 68.0, &format!("Tax Point : {}", g.point), TEXT);

        events.extend(self.build_relation_box(ui, g, rx, cy + 98.0, "Alliance", 0, is_master));
        events.extend(self.build_relation_box(ui, g, rx, cy + 178.0, "Antagonist", 1, is_master));
        events
    }

    fn draw_tendency(ui: &mut UiFrame, gx: f32, gy: f32) {
        let size = 90.0;
        Self::fill(ui, gx, gy, size, size, TENDENCY_BORDER);
        Self::fill(ui, gx + 1.0, gy + 1.0, size - 2.0, size - 2.0, SELECTION_COLOR);
        Self::fill(ui, gx + size / 2.0 - 1.0, gy + 1.0, 2.0, size - 2.0, TENDENCY_AXIS);
        Self::fill(ui, gx + 1.0, gy + size / 2.0 - 1.0, size - 2.0, 2.0, TENDENCY_AXIS);
        Self::fill(ui, gx + size / 2.0 - 1.0, gy + size / 2.0 - 1.0, 2.0, 2.0, [1.0; 4]);
        let black = [0.0, 0.0, 0.0, 1.0];
        ui.text(gx + size / 2.0 - 3.0, gy - 3.0, "R", black);
        ui.text(gx + size / 2.0 - 3.0, gy + size + 6.0, "W", black);
        ui.text(gx - 9.0, gy + size / 2.0 + 3.0, "V", black);
        ui.text(gx + size + 3.0, gy + size / 2.0 + 3.0, "F", black);
    }

    fn build_relation_box(
        &self,
        ui: &mut UiFrame,
        g: &Guild,
        rx: f32,
        top: f32,
        label: &str,
        relation: i32,
        is_master: bool,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();
        ui.text(rx, top - 4.0, label, TEXT);
        let box_y = top;
        let box_w = 168.0;
        let box_h = 54.0;
        Self::fill(ui, rx, box_y, box_w, box_h, LISTBOX_BG);
        let want_ally = relation == 0;
        let rows: Vec<(u32, &str)> = g
            .relations
            .iter()
            .filter(|r| (r.relation == 0) == want_ally)
            .map(|r| (r.gdid as u32, r.name.as_str()))
            .collect();
        let id_off = if want_ally { 0 } else { 3 };
        for (i, (gdid, name)) in rows.iter().take(3).enumerate() {
            let ry = box_y + 2.0 + i as f32 * 14.0;
            ui.text(rx + 4.0, ry + 10.0, name, TEXT);
            if is_master {
                let rect = Rect::new(rx, ry, box_w, 13.0);
                if ui
                    .interact(WidgetId(RELATION_ROW_BASE + id_off + i as u32), rect)
                    .right_clicked()
                {
                    events.push(GameEvent::RequestDeleteGuildRelation {
                        gdid: *gdid,
                        relation,
                    });
                }
            }
        }
        events
    }

    fn build_members_tab(&mut self, ui: &mut UiFrame, guild: Option<&Guild>, is_master: bool, x: f32, cy: f32) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let Some(g) = guild else {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return events;
        };

        let name_x = x + 34.0;
        let pos_x = x + 92.0;
        let job_x = x + 158.0;
        let lv_x = x + 232.0;
        let note_x = x + 260.0;
        let dev_x = x + WIN_W - SCROLLBAR_W - 96.0;
        let tax_x = x + WIN_W - SCROLLBAR_W - 6.0;

        ui.text(name_x, cy + 12.0, "Name", TEXT);
        ui.text(pos_x, cy + 12.0, "Position", TEXT);
        ui.text(job_x, cy + 12.0, "Job", TEXT);
        ui.text(lv_x, cy + 12.0, "Lv", TEXT);
        ui.text(note_x, cy + 12.0, "Note", TEXT);
        ui.text(dev_x, cy + 12.0, "Devotion", TEXT);
        ui.text_right(tax_x, cy + 12.0, "Tax", TEXT);
        Self::fill(ui, x + 2.0, cy + HEADER_H, WIN_W - 4.0, 1.0, BORDER);

        let total_exp: i64 = g.members.iter().map(|m| m.contribution_exp as i64).sum();
        let members = g.sorted_members();
        let mut positions_list: Vec<(i32, String)> =
            g.positions.iter().map(|p| (p.id, p.name.clone())).collect();
        positions_list.sort_by_key(|(id, _)| *id);
        let mut overlay: Option<(u32, u32, Rect, Rect)> = None;
        let list_top = cy + HEADER_H + 1.0;
        let list_h = CONTENT_H - HEADER_H - 1.0;
        let visible = (list_h / MEMBER_ROW_H) as usize;
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
            let row_y = list_top + row as f32 * MEMBER_ROW_H;
            let row_rect = Rect::new(x + 2.0, row_y, WIN_W - 4.0 - bar_w, MEMBER_ROW_H);

            if self.selected_member == Some(m.gid) {
                Self::fill(ui, row_rect.x, row_rect.y, row_rect.w, row_rect.h, SELECTION_COLOR);
            } else if m.online {
                Self::fill(ui, row_rect.x, row_rect.y, row_rect.w, row_rect.h, ONLINE_ROW_BG);
            }
            Self::fill(ui, x + 2.0, row_y + MEMBER_ROW_H - 1.0, WIN_W - 4.0 - bar_w, 1.0, BORDER);

            // Head-sprite cell filled by the client's roster pass.
            self.member_head_slots
                .push((m.gid, [x + 18.0, row_y + 17.0]));

            let (mx, my) = (ui.ctx.mouse_x, ui.ctx.mouse_y);
            let color = if m.online { TEXT } else { OFFLINE_COLOR };
            let baseline = row_y + 22.0;
            ui.text(name_x, baseline, &m.name, color);
            let mut pos_cell_rect: Option<Rect> = None;
            if is_master && m.position_id != 0 {
                let cell = Rect::new(pos_x - 2.0, row_y + 9.0, 62.0, 16.0);
                let dd_blocked = overlay.map(|(_, _, _, lr)| lr.contains(mx, my)).unwrap_or(false);
                self.member_pos_dropdown.open = self.open_member_dropdown == Some(m.gid);
                let dr = self.member_pos_dropdown.show(
                    ui,
                    WidgetId(MEMBER_POS_DROPDOWN_BASE + row as u32),
                    cell,
                    &m.position_name,
                    positions_list.len(),
                    Rect::new(x, cy, WIN_W, CONTENT_H),
                    dd_blocked,
                );
                if dr.toggled {
                    self.open_member_dropdown = self.member_pos_dropdown.open.then_some(m.gid);
                }
                if let Some(list_rect) = dr.overlay_rect {
                    overlay = Some((m.aid, m.gid, cell, list_rect));
                }
                pos_cell_rect = Some(cell);
            } else {
                ui.text(pos_x, baseline, &m.position_name, color);
            }
            ui.text(job_x, baseline, &job_class_name(m.job as u16), color);
            ui.text(lv_x, baseline, &m.level.to_string(), color);
            ui.text(note_x, baseline, &m.note, color);
            let devotion = if total_exp > 0 {
                (m.contribution_exp as i64 * 100 / total_exp) as i64
            } else {
                0
            };
            ui.text(dev_x, baseline, &format!("{devotion} %"), color);
            ui.text_right(tax_x, baseline, &m.contribution_exp.to_string(), color);

            let target_master = m.position_id == 0;
            let resp = ui.interact(WidgetId(MEMBER_ROW_BASE + row as u32), row_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            let suppress = pos_cell_rect.map(|c| c.contains(mx, my)).unwrap_or(false)
                || overlay.map(|(_, _, _, lr)| lr.contains(mx, my)).unwrap_or(false);
            if resp.clicked() && !suppress {
                self.open_member_dropdown = None;
                self.selected_member = Some(m.gid);
                if is_master && !target_master {
                    events.push(GameEvent::ShowGuildMemberMenu {
                        aid: m.aid,
                        gid: m.gid,
                        name: m.name.clone(),
                        x: mx,
                        y: my,
                    });
                }
            }
            if resp.right_clicked() && !suppress {
                events.push(GameEvent::ShowGuildMemberMenu {
                    aid: m.aid,
                    gid: m.gid,
                    name: m.name.clone(),
                    x: mx,
                    y: my,
                });
            }
        }

        if let Some((aid, gid, _cell, list_rect)) = overlay {
            self.member_pos_overlay = Some((aid, gid, list_rect, positions_list));
        }
        events
    }

    fn render_member_pos_overlay(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let Some((aid, gid, list_rect, positions)) = self.member_pos_overlay.take() else {
            return events;
        };
        let labels: Vec<&str> = positions.iter().map(|(_, name)| name.as_str()).collect();
        if let Some(idx) =
            self.member_pos_dropdown
                .show_overlay(ui, list_rect, MEMBER_POS_OPTION_BASE, &labels)
        {
            events.push(GameEvent::RequestChangeMemberPosition {
                aid,
                gid,
                position_id: positions[idx].0,
            });
            self.open_member_dropdown = None;
        }
        events
    }

    fn build_position_tab(&mut self, ui: &mut UiFrame, guild_present: bool, is_master: bool, x: f32, cy: f32) {
        if !guild_present {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return;
        }

        let id_x = x + 8.0;
        let title_x = x + 58.0;
        let inv_x = x + 226.0;
        let pun_x = x + 294.0;
        let tax_x = x + WIN_W - SCROLLBAR_W - 60.0;
        ui.text(id_x, cy + 12.0, "Rank", TEXT);
        ui.text(title_x, cy + 12.0, "Position Title", TEXT);
        ui.text(inv_x, cy + 12.0, "Invite", TEXT);
        ui.text(pun_x, cy + 12.0, "Punish", TEXT);
        ui.text(tax_x, cy + 12.0, "Tax", TEXT);
        Self::fill(ui, x + 2.0, cy + HEADER_H, WIN_W - 4.0, 1.0, BORDER);

        let list_top = cy + HEADER_H + 1.0;
        let list_h = CONTENT_H - HEADER_H - 1.0;
        let visible = (list_h / POS_ROW_H) as usize;
        let count = self.pos_edits.len();
        let max_scroll = count.saturating_sub(visible);
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
            if idx >= count {
                break;
            }
            let row_y = list_top + row as f32 * POS_ROW_H;
            let baseline = row_y + 14.0;
            let pos_id = self.pos_edits[idx].id;
            let is_rank0 = pos_id == 0;

            if is_rank0 {
                Self::fill(ui, x + 2.0, row_y, WIN_W - 2.0 - SCROLLBAR_W, POS_ROW_H - 1.0, SELECTION_COLOR);
            }
            Self::fill(ui, x + 2.0, row_y + POS_ROW_H - 1.0, WIN_W - 4.0, 1.0, BORDER);
            ui.text(id_x, baseline, &pos_id.to_string(), TEXT);

            let editable = is_master;
            let right = self.pos_edits[idx].right;
            if editable {
                let title_rect = Rect::new(title_x, row_y + 1.0, 158.0, POS_ROW_H - 2.0);
                let e = &mut self.pos_edits[idx];
                let r = ui.text_input(
                    WidgetId(POS_TITLE_INPUT_BASE + idx as u32),
                    title_rect,
                    &mut e.title,
                    TextInputBg::Gray,
                );
                if r.has_focus() {
                    self.pos_dirty = true;
                }
            } else {
                let name = self.pos_edits[idx].title.text.clone();
                ui.text(title_x, baseline, &name, TEXT);
            }

            let invite_on = right & GUILD_PERM_INVITE != 0;
            let punish_on = right & GUILD_PERM_EXPEL != 0;
            let inv_rect = Rect::new(inv_x, row_y + 4.0, 11.0, 11.0);
            let pun_rect = Rect::new(pun_x, row_y + 4.0, 11.0, 11.0);

            // Rank 0 permissions are fixed; only lower ranks toggle.
            if is_master && !is_rank0 {
                let mut inv = invite_on;
                if ui.checkbox(WidgetId(POS_INVITE_BASE + idx as u32), inv_rect, &mut inv, &CHECKBOX).clicked() {
                    self.pos_edits[idx].right ^= GUILD_PERM_INVITE;
                    self.pos_dirty = true;
                }
                let mut pun = punish_on;
                if ui.checkbox(WidgetId(POS_PUNISH_BASE + idx as u32), pun_rect, &mut pun, &CHECKBOX).clicked() {
                    self.pos_edits[idx].right ^= GUILD_PERM_EXPEL;
                    self.pos_dirty = true;
                }
            } else {
                ui.checkbox_display(inv_rect, invite_on, &CHECKBOX);
                ui.checkbox_display(pun_rect, punish_on, &CHECKBOX);
            }

            if editable {
                let tax_rect = Rect::new(tax_x, row_y + 1.0, 30.0, POS_ROW_H - 2.0);
                let e = &mut self.pos_edits[idx];
                let r = ui.text_input(
                    WidgetId(POS_TAX_INPUT_BASE + idx as u32),
                    tax_rect,
                    &mut e.tax,
                    TextInputBg::Gray,
                );
                if r.has_focus() {
                    self.pos_dirty = true;
                }
                ui.text(tax_x + 33.0, baseline, "%", TEXT);
            } else {
                let tax_text = self.pos_edits[idx].tax.text.clone();
                ui.text(tax_x, baseline, &format!("{tax_text} %"), TEXT);
            }
        }
    }

    fn build_skill_tab(
        &mut self,
        ui: &mut UiFrame,
        guild: Option<&Guild>,
        is_master: bool,
        x: f32,
        cy: f32,
        data: &DataTable,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let (skills, skill_point) = match guild {
            Some(g) => (g.skills.clone(), g.skill_point),
            None => {
                ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
                return events;
            }
        };
        if skills.is_empty() {
            ui.text(x + 10.0, cy + 20.0, "No guild skills.", OFFLINE_COLOR);
            return events;
        }

        let footer_h = 27.0;
        let list_h = CONTENT_H - footer_h;
        let visible = (list_h / SKILL_ROW_H) as usize;
        let max_scroll = skills.len().saturating_sub(visible);
        if max_scroll > 0 {
            let content_rect = Rect::new(x, cy, WIN_W, list_h);
            self.skills_scroll = scrollbar::scrollbar(
                ui,
                SKILL_SCROLL,
                self.skills_scroll,
                visible,
                max_scroll,
                content_rect,
                x + WIN_W - SCROLLBAR_W,
                cy + 2.0,
                list_h,
            );
        } else {
            self.skills_scroll = 0;
        }

        let pending_total: i16 = self.skill_pending.iter().map(|(_, n)| *n).sum();
        let remaining = skill_point - pending_total;

        for row in 0..visible {
            let idx = self.skills_scroll + row;
            let Some(s) = skills.get(idx) else { break };
            let row_y = cy + 4.0 + row as f32 * SKILL_ROW_H;
            let pending = self
                .skill_pending
                .iter()
                .find(|(id, _)| *id == s.skid)
                .map(|(_, n)| *n)
                .unwrap_or(0);
            let shown_level = s.level + pending;

            let icon_path = format!(
                "data/texture/유저인터페이스/item/{}.bmp",
                s.name.to_lowercase()
            );
            let dim = shown_level <= 0;
            let alpha = if dim { 0.5 } else { 1.0 };
            let (v, i) = draw::quad_vertices(x + 15.0, row_y, 24.0, 24.0, [alpha; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(icon_path),
            });

            let name_color = if dim { OFFLINE_COLOR } else { TEXT };
            let display = data
                .skill_name
                .as_ref()
                .map(|t| t.get_display_name_or_internal(&s.name))
                .unwrap_or_else(|| s.name.clone());
            ui.text(x + 45.0, row_y + 12.0, &display, name_color);
            ui.text(x + 45.0, row_y + 25.0, &format!("Lv : {shown_level}"), name_color);

            if is_master && !s.passive && s.upgradable {

                let up_rect = Rect::new(x + WIN_W - 76.0, row_y + 4.0, 18.0, 18.0);
                let up = ui.button(WidgetId(SKILL_UP_BASE + row as u32), up_rect, &SKILL_UP_BTN, "+");
                if up.hovered() {
                    ui.any_interactive_hovered = true;
                }
                if up.clicked() && remaining > 0 {
                    self.adjust_pending(s.skid, 1);
                }
            }
        }

        let footer_y = cy + CONTENT_H - footer_h;
        Self::fill(ui, x + 2.0, footer_y, WIN_W - 4.0, 1.0, BORDER);
        ui.text(x + 10.0, footer_y + 17.0, &format!("Skill Points: {remaining}"), TEXT);
        if is_master && pending_total > 0 {
            let apply_rect = Rect::new(x + WIN_W - 100.0, footer_y + 4.0, 42.0, 20.0);
            let apply = ui.button(SKILL_APPLY_BTN, apply_rect, &APPLY_BTN, "Apply");
            if apply.hovered() {
                ui.any_interactive_hovered = true;
            }
            if apply.clicked() {
                for (skid, n) in self.skill_pending.drain(..) {
                    for _ in 0..n {
                        events.push(GameEvent::RequestUpgradeGuildSkill { skid });
                    }
                }
            }
            let reset_rect = Rect::new(x + WIN_W - 50.0, footer_y + 4.0, 42.0, 20.0);
            let reset = ui.button(SKILL_RESET_BTN, reset_rect, &RESET_BTN, "Reset");
            if reset.hovered() {
                ui.any_interactive_hovered = true;
            }
            if reset.clicked() {
                self.skill_pending.clear();
            }
        }
        events
    }

    fn adjust_pending(&mut self, skid: u16, delta: i16) {
        if let Some(entry) = self.skill_pending.iter_mut().find(|(id, _)| *id == skid) {
            entry.1 += delta;
            if entry.1 <= 0 {
                self.skill_pending.retain(|(id, _)| *id != skid);
            }
        } else if delta > 0 {
            self.skill_pending.push((skid, delta));
        }
    }

    fn build_expel_tab(&self, ui: &mut UiFrame, guild: Option<&Guild>, x: f32, cy: f32) {
        let Some(g) = guild else {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return;
        };
        let name_x = x + 8.0;
        let reason_x = x + 110.0;
        ui.text(name_x, cy + 12.0, "Name", TEXT);
        ui.text(reason_x, cy + 12.0, "The Reason of Expulsion", TEXT);
        Self::fill(ui, x + 2.0, cy + HEADER_H, WIN_W - 4.0, 1.0, BORDER);
        Self::fill(ui, reason_x - 4.0, cy, 1.0, CONTENT_H, BORDER);

        if g.ban_list.is_empty() {
            ui.text(x + 10.0, cy + HEADER_H + 18.0, "No expelled members.", OFFLINE_COLOR);
            return;
        }
        let row_h = 18.0;
        let visible = ((CONTENT_H - HEADER_H) / row_h) as usize;
        for (i, b) in g.ban_list.iter().take(visible).enumerate() {
            let row_y = cy + HEADER_H + 1.0 + i as f32 * row_h;
            ui.text(name_x, row_y + 13.0, &b.char_name, TEXT);
            ui.text(reason_x, row_y + 13.0, &b.reason, TEXT);
            Self::fill(ui, x + 2.0, row_y + row_h - 1.0, WIN_W - 4.0, 1.0, BORDER);
        }
    }

    fn build_notice_tab(&mut self, ui: &mut UiFrame, guild: Option<&Guild>, is_master: bool, x: f32, cy: f32) {
        if guild.is_none() {
            ui.text(x + 10.0, cy + 20.0, "Not in a guild.", OFFLINE_COLOR);
            return;
        }
        ui.text(x + 9.0, cy + 23.0, "Title", TEXT);
        let subj_rect = Rect::new(x + 50.0, cy + 13.0, WIN_W - 60.0, 16.0);
        ui.text(x + 9.0, cy + 46.0, "Contents", TEXT);
        let body_rect = Rect::new(x + 9.0, cy + 52.0, WIN_W - 18.0, CONTENT_H - 68.0);

        if is_master {
            ui.text_input(NOTICE_SUBJECT_INPUT, subj_rect, &mut self.notice_subject_input, TextInputBg::Gray);
            self.multiline_input(ui, body_rect);
        } else {
            let (subject, body) = {
                let g = guild.unwrap();
                (g.notice_subject.clone(), g.notice_body.clone())
            };
            Self::fill(ui, subj_rect.x, subj_rect.y, subj_rect.w, subj_rect.h, INPUT_BG);
            ui.text(subj_rect.x + 4.0, subj_rect.y + 12.0, &subject, TEXT);
            Self::fill(ui, body_rect.x, body_rect.y, body_rect.w, body_rect.h, INPUT_BG);
            for (i, line) in wrap_text(&body, 60).iter().enumerate() {
                ui.text(body_rect.x + 4.0, body_rect.y + 13.0 + i as f32 * 14.0, line, TEXT);
            }
        }
    }

    fn multiline_input(&mut self, ui: &mut UiFrame, rect: Rect) {
        Self::fill(ui, rect.x, rect.y, rect.w, rect.h, INPUT_BG);
        let resp = ui.interact(NOTICE_BODY_INPUT, rect);
        if resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if resp.has_focus() {
            self.notice_body_input.process_keys(ui.ctx);
        }
        let focused = resp.has_focus();
        let lines = wrap_text(&self.notice_body_input.text, 60);
        let line_count = lines.len().max(1);
        for (i, line) in lines.iter().enumerate() {
            ui.text(rect.x + 4.0, rect.y + 13.0 + i as f32 * 14.0, line, TEXT);
        }
        if focused && (ui.elapsed_secs % 1.0) < 0.5 {
            let last = lines.last().map(|s| s.as_str()).unwrap_or("");
            let caret_x = rect.x + 4.0 + ui.atlas.measure_text(last);
            let caret_y = rect.y + 3.0 + (line_count - 1) as f32 * 14.0;
            Self::fill(ui, caret_x, caret_y, 1.0, 12.0, TEXT);
        }
    }

}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.split('\n') {
        if raw_line.chars().count() <= max_chars {
            lines.push(raw_line.to_string());
            continue;
        }
        let mut current = String::new();
        for word in raw_line.split(' ') {
            if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > max_chars {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_game::guild::GuildRelation;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn right_click_alliance_row_requests_delete() {
        let mut win = GuildWindow::new();
        win.open();
        let mut guild = Guild::default();
        guild.gdid = 1;
        guild.master_name = "Me".to_string();
        guild.relations = vec![GuildRelation {
            gdid: 42,
            name: "Allies".to_string(),
            relation: 0,
        }];
        let mut character = Character::new();
        character.name = "Me".to_string();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        let win_x = 80.0;
        let win_y = 50.0;
        let content_y = win_y + TITLE_H + TAB_H;
        let alliance_row_y = content_y + 98.0 + 2.0;
        ctx.mouse_x = win_x + 301.0 + 168.0 / 2.0;
        ctx.mouse_y = alliance_row_y + 13.0 / 2.0;
        ctx.mouse_right_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        let mut build_ctx = crate::BuildCtx::test(&mut character, &data);
        build_ctx.guild = Some(&guild);
        build_ctx.local_gid = 1;
        let events = win.build(&mut ui, &mut build_ctx);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestDeleteGuildRelation { gdid: 42, relation: 0 })),
            "expected delete-relation event, got {events:?}"
        );
    }
}
