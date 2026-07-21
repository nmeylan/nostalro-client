use crate::helper::window_chrome::{
    draw_sys_button, draw_textured_quad, draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const STATUS_WINDOW_ID: WidgetId = WidgetId(1500);
const CLOSE_BTN_ID: WidgetId = WidgetId(1501);
const MINI_BTN_ID: WidgetId = WidgetId(1502);
const UP_STR_ID: WidgetId = WidgetId(1510);
const UP_AGI_ID: WidgetId = WidgetId(1511);
const UP_VIT_ID: WidgetId = WidgetId(1512);
const UP_INT_ID: WidgetId = WidgetId(1513);
const UP_DEX_ID: WidgetId = WidgetId(1514);
const UP_LUK_ID: WidgetId = WidgetId(1515);

const BG_TEX: &str = "data/texture/유저인터페이스/basic_interface/statwin0_bg.bmp";
const TITLEBAR_TEX: &str = "data/texture/유저인터페이스/basic_interface/titlebar_mid.bmp";
const SYS_CLOSE_OFF: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const SYS_CLOSE_ON: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";
const SYS_MINI_OFF: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_off.bmp";
const SYS_MINI_ON: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_on.bmp";
const ARW_RIGHT: &str = "data/texture/유저인터페이스/basic_interface/arw_right.bmp";
const ARW_RIGHT_ON: &str = "data/texture/유저인터페이스/basic_interface/arw_right_on.bmp";

const WIN_W: f32 = 280.0;
const TITLE_H: f32 = 17.0;
const PANEL_H: f32 = 103.0;
const WIN_H: f32 = TITLE_H + PANEL_H;

const ROW_H: f32 = 16.0;
const ROWS_TOP: f32 = TITLE_H + 2.0;
const TEXT_BASELINE_OFF: f32 = 13.0;

const STATS_LEFT: f32 = 53.0;
const BONUS_LEFT: f32 = 70.0;
const UP_LEFT: f32 = 89.0;
const UP_TOP: f32 = TITLE_H + 6.0;
const REQ_LEFT: f32 = 96.0;
const REQ_WIDTH: f32 = 12.0;

const COL1_RIGHT: f32 = WIN_W - 87.0;
const COL2_RIGHT: f32 = WIN_W - 5.0;

const STAT_ID_STR: u16 = 13;
const STAT_ID_AGI: u16 = 14;
const STAT_ID_VIT: u16 = 15;
const STAT_ID_INT: u16 = 16;
const STAT_ID_DEX: u16 = 17;
const STAT_ID_LUK: u16 = 18;

pub struct StatusWindow {
    pub has_grf_textures: bool,
    visible: bool,
    minimized: bool,
    bg_size: (f32, f32),
    titlebar_size: (f32, f32),
    sys_btn_size: (f32, f32),
    arrow_size: (f32, f32),
}

impl Default for StatusWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            visible: false,
            minimized: false,
            bg_size: (WIN_W, PANEL_H),
            titlebar_size: (12.0, TITLE_H),
            sys_btn_size: (11.0, 11.0),
            arrow_size: (11.0, 11.0),
        }
    }

    pub fn is_minimized(&self) -> bool {
        self.minimized
    }

    pub fn set_minimized(&mut self, value: bool) {
        self.minimized = value;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    pub fn open(&mut self) {
        self.visible = true;
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    fn fmt_signed(v: i32) -> String {
        if v >= 0 {
            format!("+ {}", v)
        } else {
            format!("- {}", -v)
        }
    }

    fn computed_cost(stat: u8, server_cost: u16) -> u16 {
        if server_cost > 0 {
            server_cost
        } else if stat == 0 {
            2
        } else {
            ((stat as u16 - 1) / 10) + 2
        }
    }
}

impl Window for StatusWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(BG_TEX) {
            self.bg_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(TITLEBAR_TEX) {
            self.titlebar_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(SYS_CLOSE_OFF) {
            self.sys_btn_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(ARW_RIGHT) {
            self.arrow_size = (w as f32, h as f32);
        }
    }

    fn window_size(&self) -> (f32, f32) {
        (WIN_W, WIN_H)
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            BG_TEX,
            TITLEBAR_TEX,
            SYS_CLOSE_OFF,
            SYS_CLOSE_ON,
            SYS_MINI_OFF,
            SYS_MINI_ON,
            ARW_RIGHT,
            ARW_RIGHT_ON,
        ]
    }
}

impl InGameWindow for StatusWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        ctx: &mut BuildCtx,
    ) -> Vec<GameEvent> {
        let character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.visible {
            return Vec::new();
        }
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let mut events = Vec::new();
        let grf = self.has_grf_textures;
        let tc = text_color(grf);

        let win_h = if self.minimized { TITLE_H } else { WIN_H };
        let win = ui.window_at(STATUS_WINDOW_ID, WIN_W, win_h, TITLE_H, 60.0, 140.0);
        let win_rect = Rect::new(win.x, win.y, WIN_W, WIN_H);
        ui.interact(STATUS_WINDOW_ID, win_rect);
        let x = win.x;
        let y = win.y;

        draw_titlebar(ui, win.x, win.y, WIN_W, TITLE_H, grf);

        ui.text(x + 18.0, y + 13.0, "Status", tc);

        let sys_w = self.sys_btn_size.0;
        let sys_h = self.sys_btn_size.1;
        let close_x = x + WIN_W - 3.0 - sys_w;
        let mini_x = close_x - sys_w - 1.0;

        let close_rect = Rect::new(close_x, y + 3.0, sys_w, sys_h);
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if close_resp.clicked() {
            self.visible = false;
        }

        draw_sys_button(
            ui,
            close_rect,
            (sys_w, sys_h),
            close_resp.hovered(),
            grf,
            SYS_CLOSE_ON,
            SYS_CLOSE_OFF,
            Some('x'),
        );

        let mini_rect = Rect::new(mini_x, y + 3.0, sys_w, sys_h);
        let mini_resp = ui.interact(MINI_BTN_ID, mini_rect);
        if mini_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if mini_resp.clicked() {
            self.minimized = !self.minimized;
        }
        draw_sys_button(
            ui,
            mini_rect,
            (sys_w, sys_h),
            mini_resp.hovered(),
            grf,
            SYS_MINI_ON,
            SYS_MINI_OFF,
            Some('_'),
        );
        if self.minimized {
            ui.has_grf_textures = prev_grf;
            return events;
        }

        if grf {
            draw_textured_quad(ui, x, y + TITLE_H, WIN_W, PANEL_H, BG_TEX);
        } else {
            crate::helper::fallback::window_body(ui, x, y + TITLE_H, WIN_W, PANEL_H);
            for (i, label) in ["STR", "AGI", "VIT", "INT", "DEX", "LUK"]
                .iter()
                .enumerate()
            {
                ui.text(
                    x + 10.0,
                    y + ROWS_TOP + i as f32 * ROW_H + TEXT_BASELINE_OFF,
                    label,
                    tc,
                );
            }
            for (i, label) in ["ATK", "MATK", "HIT", "CRIT"].iter().enumerate() {
                ui.text(
                    x + 110.0,
                    y + ROWS_TOP + i as f32 * ROW_H + TEXT_BASELINE_OFF,
                    label,
                    tc,
                );
            }
            for (i, label) in ["DEF", "MDEF", "FLEE", "ASPD", "Status pts"]
                .iter()
                .enumerate()
            {
                ui.text(
                    x + 195.0,
                    y + ROWS_TOP + i as f32 * ROW_H + TEXT_BASELINE_OFF,
                    label,
                    tc,
                );
            }
        }

        let status_point = character.status_point;
        let rows: [(WidgetId, u8, i16, u16, u16); 6] = [
            (
                UP_STR_ID,
                character.str,
                character.str_bonus,
                character.str_cost,
                STAT_ID_STR,
            ),
            (
                UP_AGI_ID,
                character.agi,
                character.agi_bonus,
                character.agi_cost,
                STAT_ID_AGI,
            ),
            (
                UP_VIT_ID,
                character.vit,
                character.vit_bonus,
                character.vit_cost,
                STAT_ID_VIT,
            ),
            (
                UP_INT_ID,
                character.int,
                character.int_bonus,
                character.int_cost,
                STAT_ID_INT,
            ),
            (
                UP_DEX_ID,
                character.dex,
                character.dex_bonus,
                character.dex_cost,
                STAT_ID_DEX,
            ),
            (
                UP_LUK_ID,
                character.luk,
                character.luk_bonus,
                character.luk_cost,
                STAT_ID_LUK,
            ),
        ];

        let arr_w = self.arrow_size.0;
        let arr_h = self.arrow_size.1;
        for (i, &(up_id, base, bonus, cost, status_id)) in rows.iter().enumerate() {
            let baseline = y + ROWS_TOP + i as f32 * ROW_H + TEXT_BASELINE_OFF;
            let display_cost = Self::computed_cost(base, cost);

            ui.text(x + STATS_LEFT, baseline, &base.to_string(), tc);
            if bonus != 0 {
                ui.text(
                    x + BONUS_LEFT,
                    baseline,
                    &Self::fmt_signed(bonus as i32),
                    tc,
                );
            }
            let can_raise = (display_cost as u32) <= status_point;
            if can_raise {
                let arr_y = y + UP_TOP + i as f32 * (arr_h + 5.0);
                let arr_rect = Rect::new(x + UP_LEFT, arr_y, arr_w, arr_h);
                let resp = ui.interact(up_id, arr_rect);
                if resp.hovered() {
                    ui.any_interactive_hovered = true;
                }
                if resp.clicked() {
                    events.push(GameEvent::RequestStatChange {
                        status_id,
                        amount: 1,
                    });
                }
                if grf {
                    let tex = if resp.hovered() {
                        ARW_RIGHT_ON
                    } else {
                        ARW_RIGHT
                    };
                    draw_textured_quad(ui, arr_rect.x, arr_rect.y, arr_w, arr_h, tex);
                } else {
                    let c = if resp.hovered() {
                        [0.95, 0.95, 0.4, 1.0]
                    } else {
                        [0.7, 0.7, 0.3, 1.0]
                    };
                    let (v, ii) = draw::quad_vertices(arr_rect.x, arr_rect.y, arr_w, arr_h, c);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: ii.to_vec(),
                        texture: TextureRef::White,
                    });
                }
            }
            ui.text_right(
                x + REQ_LEFT + REQ_WIDTH,
                baseline,
                &display_cost.to_string(),
                tc,
            );
        }

        let col1_lines: [String; 4] = [
            format!("{} {}", character.atk1, Self::fmt_signed(character.atk2)),
            format!("{} {}", character.matk1, Self::fmt_signed(character.matk2)),
            format!("{}", character.hit),
            format!("{}", character.critical),
        ];
        for (i, t) in col1_lines.iter().enumerate() {
            let baseline = y + ROWS_TOP + i as f32 * ROW_H + TEXT_BASELINE_OFF;
            ui.text_right(x + COL1_RIGHT, baseline, t, tc);
        }

        let aspd_disp = (200 - character.aspd / 10).max(0);
        let col2_lines: [String; 5] = [
            format!("{} {}", character.def1, Self::fmt_signed(character.def2)),
            format!("{} {}", character.mdef1, Self::fmt_signed(character.mdef2)),
            format!("{} {}", character.flee1, Self::fmt_signed(character.flee2)),
            format!("{}", aspd_disp),
            format!("{}", character.status_point),
        ];
        for (i, t) in col2_lines.iter().enumerate() {
            let baseline = y + ROWS_TOP + i as f32 * ROW_H + TEXT_BASELINE_OFF;
            ui.text_right(x + COL2_RIGHT, baseline, t, tc);
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}
