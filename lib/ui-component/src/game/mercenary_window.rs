use crate::Window;
use crate::game::homun_window::{bar, button};
use crate::helper::window_chrome::{draw_sys_button, draw_titlebar, text_color};
use ragnarok_game::companion::MercenaryState;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const MERCENARY_WINDOW_ID: WidgetId = WidgetId(3000);
const CLOSE_BTN_ID: WidgetId = WidgetId(3001);
const DISMISS_BTN_ID: WidgetId = WidgetId(3002);

const WIN_W: f32 = 210.0;
const TITLE_H: f32 = 17.0;
const WIN_H: f32 = 316.0;
const PANEL_H: f32 = WIN_H - TITLE_H;
const PAD: f32 = 8.0;
const ROW_H: f32 = 14.0;
const BASELINE: f32 = 10.0;
const MAX_SKILL_ROWS: usize = 6;

pub struct MercenaryWindow {
    pub has_grf_textures: bool,
    visible: bool,
}

impl Default for MercenaryWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl MercenaryWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            visible: false,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    pub fn set_visible(&mut self, value: bool) {
        self.visible = value;
    }

    pub fn build(&mut self, ui: &mut UiFrame, merc: Option<&MercenaryState>) -> Vec<GameEvent> {
        if !self.visible {
            return Vec::new();
        }
        let Some(merc) = merc else {
            return Vec::new();
        };
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let mut events = Vec::new();
        let tc = text_color(grf);

        let win = ui.window_at(MERCENARY_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 240.0, 140.0);
        let x = win.x;
        let y = win.y;
        ui.interact(MERCENARY_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 8.0, y + 13.0, "Mercenary", tc);

        let sys_w = 11.0;
        let close_rect = Rect::new(x + WIN_W - 3.0 - sys_w, y + 3.0, sys_w, sys_w);
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
            (sys_w, sys_w),
            close_resp.hovered(),
            grf,
            "",
            "",
            Some('x'),
            [0.9, 0.4, 0.4, 1.0],
            [0.6, 0.3, 0.3, 1.0],
        );

        let (v, i) = draw::quad_vertices(x, y + TITLE_H, WIN_W, PANEL_H, [0.10, 0.10, 0.14, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });

        let cx = x + PAD;
        let mut cy = y + TITLE_H + 6.0;

        ui.text(cx, cy + BASELINE, &merc.name, tc);
        ui.text_right(
            x + WIN_W - PAD,
            cy + BASELINE,
            &format!("Lv {}", merc.level),
            tc,
        );
        cy += ROW_H + 2.0;

        cy = bar(
            ui,
            cx,
            cy,
            WIN_W - PAD * 2.0,
            "HP",
            merc.hp,
            merc.max_hp,
            [0.2, 0.75, 0.2, 1.0],
            tc,
        );
        cy = bar(
            ui,
            cx,
            cy,
            WIN_W - PAD * 2.0,
            "SP",
            merc.sp,
            merc.max_sp,
            [0.25, 0.45, 0.9, 1.0],
            tc,
        );

        cy += 2.0;
        ui.text(cx, cy + BASELINE, &format!("Faith: {}", merc.faith), tc);
        ui.text_right(
            x + WIN_W - PAD,
            cy + BASELINE,
            expire_label(merc.expire_date),
            tc,
        );
        cy += ROW_H;
        ui.text(cx, cy + BASELINE, &format!("Summons: {}", merc.calls), tc);
        ui.text_right(
            x + WIN_W - PAD,
            cy + BASELINE,
            &format!("Kills: {}", merc.kills),
            tc,
        );
        cy += ROW_H + 4.0;

        let col2 = x + WIN_W * 0.5;
        let left = [
            ("ATK", merc.atk as i32),
            ("MATK", merc.matk as i32),
            ("HIT", merc.hit as i32),
            ("CRI", merc.critical as i32),
            ("Range", merc.atk_range as i32),
        ];
        let right = [
            ("DEF", merc.def as i32),
            ("MDEF", merc.mdef as i32),
            ("FLEE", merc.flee as i32),
            ("ASPD", merc.aspd as i32),
            ("", 0),
        ];
        for (i, (label, value)) in left.iter().enumerate() {
            let by = cy + i as f32 * ROW_H + BASELINE;
            ui.text(cx, by, label, tc);
            ui.text_right(col2 - 6.0, by, &value.to_string(), tc);
        }
        for (i, (label, value)) in right.iter().enumerate() {
            if label.is_empty() {
                continue;
            }
            let by = cy + i as f32 * ROW_H + BASELINE;
            ui.text(col2 + 4.0, by, label, tc);
            ui.text_right(x + WIN_W - PAD, by, &value.to_string(), tc);
        }
        cy += 5.0 * ROW_H + 4.0;

        // Skill list (info only — mercenary skills are used autonomously by its AI).
        ui.text(cx, cy + BASELINE, "Skills", tc);
        cy += ROW_H;
        if merc.skills.is_empty() {
            ui.text(cx + 4.0, cy + BASELINE, "(none)", tc);
            cy += ROW_H;
        } else {
            for skill in merc.skills.iter().take(MAX_SKILL_ROWS) {
                ui.text(cx + 4.0, cy + BASELINE, &skill.name, tc);
                ui.text_right(
                    x + WIN_W - PAD,
                    cy + BASELINE,
                    &format!("Lv {}", skill.level),
                    tc,
                );
                cy += ROW_H;
            }
        }
        cy += 4.0;

        let dismiss_rect = Rect::new(cx, cy, WIN_W - PAD * 2.0, 16.0);
        if button(ui, DISMISS_BTN_ID, dismiss_rect, "Dismiss", grf) {
            events.push(GameEvent::RequestMercenaryCommand { command: 2 });
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl Window for MercenaryWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![]
    }
}

/// Mercenary contracts carry an absolute expiry timestamp; without a synced
/// server clock we can't compute a live countdown, so just mark the contract active.
fn expire_label(expire_date: i32) -> &'static str {
    if expire_date <= 0 { "Expired" } else { "Active" }
}
