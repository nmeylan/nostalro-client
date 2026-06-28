use crate::helper::window_chrome::{
    FOOTER_TEX, ITEMWIN_MID_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container,
    draw_footer, draw_sys_button, draw_titlebar, text_color,
};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_game::party::Party;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const PARTY_WINDOW_ID: WidgetId = WidgetId(2000);
const CLOSE_BTN_ID: WidgetId = WidgetId(2001);
const LEAVE_BTN_ID: WidgetId = WidgetId(2003);
const EXP_SHARE_BTN_ID: WidgetId = WidgetId(2004);
const KICK_BASE_ID: u32 = 2020;

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const WIN_W: f32 = 220.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 24.0;
const ROW_H: f32 = 34.0;
const ROW_PAD: f32 = 6.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const HP_BAR_W: f32 = 120.0;
const HP_BAR_H: f32 = 7.0;

pub struct PartyWindow {
    pub open: bool,
    pub has_grf_textures: bool,
    party: Option<Party>,
    local_aid: u32,
}

impl Default for PartyWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl PartyWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            has_grf_textures: false,
            party: None,
            local_aid: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn sync_party(&mut self, party: Option<&Party>, local_aid: u32) {
        self.party = party.cloned();
        self.local_aid = local_aid;
    }

    fn is_leader(&self) -> bool {
        self.party
            .as_ref()
            .and_then(|p| p.leader_aid())
            .map(|aid| aid == self.local_aid)
            .unwrap_or(false)
    }

    /// Colored hit-tested text button (independent of GRF chrome so it renders in both modes).
    fn text_button(ui: &mut UiFrame, id: WidgetId, rect: Rect, label: &str) -> bool {
        let resp = ui.interact(id, rect);
        if resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        let bg = if resp.hovered() {
            [0.35, 0.35, 0.5, 1.0]
        } else {
            [0.22, 0.22, 0.32, 1.0]
        };
        let (v, i) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, bg);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let tw = ui.atlas.measure_text(label);
        let tx = rect.x + (rect.w - tw) / 2.0;
        let ty = rect.y + rect.h - (ui.atlas.line_height / 2.0);
        ui.text(tx, ty, label, [1.0, 1.0, 1.0, 1.0]);
        resp.clicked()
    }
}

impl Window for PartyWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            ITEMWIN_MID_TEX,
            FOOTER_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
        ]
    }
}

impl InGameWindow for PartyWindow {
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

        let member_count = self.party.as_ref().map(|p| p.members.len()).unwrap_or(0);
        let body_h = (member_count.max(1) as f32) * ROW_H;
        let win_h = TITLE_H + body_h + FOOTER_H;

        let win = ui.window_at(PARTY_WINDOW_ID, WIN_W, win_h, TITLE_H, 60.0, 60.0);
        let x = win.x;
        let y = win.y;
        ui.interact(PARTY_WINDOW_ID, Rect::new(x, y, WIN_W, win_h));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        let title = match &self.party {
            Some(p) if !p.name.is_empty() => format!("Party - {}", p.name),
            _ => "Party".to_string(),
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
            [1.0, 0.3, 0.3, 1.0],
            tc,
        );
        if close_resp.clicked() {
            self.open = false;
            ui.has_grf_textures = prev_grf;
            return events;
        }

        let container_y = y + TITLE_H;
        draw_container(ui, x, container_y, WIN_W, body_h, grf);

        let is_leader = self.is_leader();
        let local_aid = self.local_aid;
        let members = self
            .party
            .as_ref()
            .map(|p| p.members.clone())
            .unwrap_or_default();

        if members.is_empty() {
            ui.text(x + 10.0, container_y + 18.0, "Not in a party", tc);
        }

        for (idx, m) in members.iter().enumerate() {
            let row_y = container_y + idx as f32 * ROW_H;
            let name_color = if !m.online {
                [0.5, 0.5, 0.5, 1.0]
            } else if m.aid == local_aid {
                [0.1, 0.4, 0.7, 1.0]
            } else {
                tc
            };

            let name_label = if m.leader {
                format!("* {}", m.name)
            } else {
                m.name.clone()
            };
            ui.text(x + ROW_PAD, row_y + 12.0, &name_label, name_color);
            ui.text_right(x + WIN_W - ROW_PAD, row_y + 12.0, &m.map, [0.45, 0.45, 0.5, 1.0]);

            let bar_x = x + ROW_PAD;
            let bar_y = row_y + 17.0;
            let (v, i) =
                draw::quad_vertices(bar_x, bar_y, HP_BAR_W, HP_BAR_H, [0.15, 0.15, 0.15, 0.9]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
            if let (Some(hp), Some(max_hp)) = (m.hp, m.max_hp)
                && max_hp > 0
            {
                let pct = (hp as f32 / max_hp as f32).clamp(0.0, 1.0);
                let fill = if pct < 0.25 {
                    [0.8, 0.2, 0.2, 1.0]
                } else {
                    [0.2, 0.6, 0.3, 1.0]
                };
                let (v, i) = draw::quad_vertices(bar_x, bar_y, HP_BAR_W * pct, HP_BAR_H, fill);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
                let hp_text = format!("{hp} / {max_hp}");
                ui.text(bar_x + HP_BAR_W + 6.0, bar_y + HP_BAR_H, &hp_text, tc);
            }

            if is_leader && m.aid != local_aid {
                let kick_rect = Rect::new(x + WIN_W - 44.0, row_y + 16.0, 40.0, 14.0);
                if Self::text_button(ui, WidgetId(KICK_BASE_ID + idx as u32), kick_rect, "Kick") {
                    events.push(GameEvent::RequestExpelMember {
                        aid: m.aid,
                        name: m.name.clone(),
                    });
                }
            }
        }

        let footer_y = container_y + body_h;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);
        if self.party.is_some() {
            let exp_share = self.party.as_ref().map(|p| p.exp_share).unwrap_or(false);
            let exp_label = if exp_share { "EXP: Even" } else { "EXP: Each" };
            if is_leader {
                let exp_rect = Rect::new(x + ROW_PAD, footer_y + 4.0, 96.0, 16.0);
                if Self::text_button(ui, EXP_SHARE_BTN_ID, exp_rect, exp_label) {
                    events.push(GameEvent::RequestPartyExpOption {
                        exp_share: !exp_share,
                    });
                }
            } else {
                ui.text(x + ROW_PAD, footer_y + 15.0, exp_label, tc);
            }

            let leave_rect = Rect::new(x + WIN_W - ROW_PAD - 72.0, footer_y + 4.0, 72.0, 16.0);
            let leave_label = if is_leader { "Disband" } else { "Leave" };
            if Self::text_button(ui, LEAVE_BTN_ID, leave_rect, leave_label) {
                events.push(GameEvent::RequestLeaveParty);
            }
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}
