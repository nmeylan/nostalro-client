use crate::Window;
use crate::helper::window_chrome::{draw_sys_button, draw_titlebar, text_color};
use ragnarok_game::companion::HomunculusState;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const HOMUN_WINDOW_ID: WidgetId = WidgetId(2900);
const CLOSE_BTN_ID: WidgetId = WidgetId(2901);
const FEED_BTN_ID: WidgetId = WidgetId(2902);
const REST_BTN_ID: WidgetId = WidgetId(2903);
const STANDBY_BTN_ID: WidgetId = WidgetId(2904);
const RENAME_INPUT_ID: WidgetId = WidgetId(2905);
const RENAME_BTN_ID: WidgetId = WidgetId(2906);

const WIN_W: f32 = 210.0;
const TITLE_H: f32 = 17.0;
const WIN_H: f32 = 344.0;
const PANEL_H: f32 = WIN_H - TITLE_H;
const PAD: f32 = 8.0;
const ROW_H: f32 = 14.0;
const BAR_H: f32 = 11.0;
const BASELINE: f32 = 10.0;
const MAX_SKILL_ROWS: usize = 6;

pub struct HomunWindow {
    pub has_grf_textures: bool,
    visible: bool,
    rename_input: TextInput,
}

impl Default for HomunWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl HomunWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            visible: false,
            rename_input: TextInput::new(23, false),
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

    pub fn build(&mut self, ui: &mut UiFrame, homun: Option<&HomunculusState>) -> Vec<GameEvent> {
        if !self.visible {
            return Vec::new();
        }
        let Some(homun) = homun else {
            return Vec::new();
        };
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let mut events = Vec::new();
        let tc = text_color(grf);

        let win = ui.window_at(HOMUN_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 200.0, 120.0);
        let x = win.x;
        let y = win.y;
        ui.interact(HOMUN_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 8.0, y + 13.0, "Homunculus", tc);

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

        // Panel background (fallback).
        let (v, i) = draw::quad_vertices(x, y + TITLE_H, WIN_W, PANEL_H, [0.10, 0.10, 0.14, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });

        let cx = x + PAD;
        let mut cy = y + TITLE_H + 6.0;

        // Name + level.
        ui.text(cx, cy + BASELINE, &homun.name, tc);
        ui.text_right(
            x + WIN_W - PAD,
            cy + BASELINE,
            &format!("Lv {}", homun.level),
            tc,
        );
        cy += ROW_H + 2.0;

        // Rename row (only before the homunculus has been named).
        if !homun.renamed {
            let input_w = WIN_W - PAD * 2.0 - 46.0;
            let input_rect = Rect::new(cx, cy, input_w, 14.0);
            let bg = if grf {
                TextInputBg::Gray
            } else {
                TextInputBg::Default
            };
            ui.text_input(RENAME_INPUT_ID, input_rect, &mut self.rename_input, bg);
            let btn_rect = Rect::new(cx + input_w + 4.0, cy, 40.0, 14.0);
            if button(ui, RENAME_BTN_ID, btn_rect, "Name", grf) {
                let name = self.rename_input.text.trim().to_string();
                if !name.is_empty() {
                    events.push(GameEvent::RequestRenameHomun { name });
                    self.rename_input.text.clear();
                }
            }
            cy += ROW_H + 4.0;
        }

        // HP / SP / EXP bars.
        cy = bar(
            ui,
            cx,
            cy,
            WIN_W - PAD * 2.0,
            "HP",
            homun.hp,
            homun.max_hp,
            [0.2, 0.75, 0.2, 1.0],
            tc,
        );
        cy = bar(
            ui,
            cx,
            cy,
            WIN_W - PAD * 2.0,
            "SP",
            homun.sp,
            homun.max_sp,
            [0.25, 0.45, 0.9, 1.0],
            tc,
        );
        cy = bar(
            ui,
            cx,
            cy,
            WIN_W - PAD * 2.0,
            "EXP",
            homun.exp.max(0) as u32,
            homun.max_exp.max(0) as u32,
            [0.85, 0.7, 0.2, 1.0],
            tc,
        );

        cy += 2.0;
        ui.text(cx, cy + BASELINE, &format!("Hunger: {}", homun.hunger), tc);
        ui.text_right(
            x + WIN_W - PAD,
            cy + BASELINE,
            intimacy_label(homun.intimacy),
            tc,
        );
        cy += ROW_H + 2.0;

        // Stat block, two columns.
        let col2 = x + WIN_W * 0.5;
        let left = [
            ("ATK", homun.atk as i32),
            ("MATK", homun.matk as i32),
            ("HIT", homun.hit as i32),
            ("CRI", homun.critical as i32),
            ("Range", homun.atk_range as i32),
        ];
        let right = [
            ("DEF", homun.def as i32),
            ("MDEF", homun.mdef as i32),
            ("FLEE", homun.flee as i32),
            ("ASPD", homun.aspd as i32),
            ("SkP", homun.skill_points as i32),
        ];
        for (i, (label, value)) in left.iter().enumerate() {
            let by = cy + i as f32 * ROW_H + BASELINE;
            ui.text(cx, by, label, tc);
            ui.text_right(col2 - 6.0, by, &value.to_string(), tc);
        }
        for (i, (label, value)) in right.iter().enumerate() {
            let by = cy + i as f32 * ROW_H + BASELINE;
            ui.text(col2 + 4.0, by, label, tc);
            ui.text_right(x + WIN_W - PAD, by, &value.to_string(), tc);
        }
        cy += 5.0 * ROW_H + 4.0;

        // Skill list (info; homunculus skills are used by its AI / owner skill commands).
        ui.text(cx, cy + BASELINE, "Skills", tc);
        cy += ROW_H;
        if homun.skills.is_empty() {
            ui.text(cx + 4.0, cy + BASELINE, "(none)", tc);
            cy += ROW_H;
        } else {
            for skill in homun.skills.iter().take(MAX_SKILL_ROWS) {
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

        // Action buttons.
        let btn_w = (WIN_W - PAD * 2.0 - 8.0) / 3.0;
        let feed_rect = Rect::new(cx, cy, btn_w, 16.0);
        let rest_rect = Rect::new(cx + btn_w + 4.0, cy, btn_w, 16.0);
        let standby_rect = Rect::new(cx + (btn_w + 4.0) * 2.0, cy, btn_w, 16.0);
        if button(ui, FEED_BTN_ID, feed_rect, "Feed", grf) {
            events.push(GameEvent::RequestHomunMenu { command: 1 });
        }
        if button(ui, REST_BTN_ID, rest_rect, "Rest", grf) {
            events.push(GameEvent::RequestHomunMenu { command: 2 });
        }
        if button(ui, STANDBY_BTN_ID, standby_rect, "Standby", grf) {
            events.push(GameEvent::ToggleCompanionStandby);
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl Window for HomunWindow {
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

fn intimacy_label(intimacy: i16) -> &'static str {
    match intimacy {
        i if i < 100 => "Hate",
        i if i < 500 => "Shy",
        i if i < 750 => "Neutral",
        i if i < 900 => "Cordial",
        _ => "Loyal",
    }
}

/// Draws a labeled value bar and returns the next y cursor.
pub(crate) fn bar(
    ui: &mut UiFrame,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    cur: u32,
    max: u32,
    fill: [f32; 4],
    tc: [f32; 4],
) -> f32 {
    let ratio = if max > 0 {
        (cur as f32 / max as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (v, i) = draw::quad_vertices(x, y, w, BAR_H, [0.05, 0.05, 0.07, 1.0]);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
    if ratio > 0.0 {
        let (v, i) = draw::quad_vertices(x, y, w * ratio, BAR_H, fill);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }
    ui.text(x + 3.0, y + BAR_H - 2.0, label, tc);
    ui.text_right(x + w - 3.0, y + BAR_H - 2.0, &format!("{cur}/{max}"), tc);
    y + BAR_H + 3.0
}

/// A simple fallback button; returns whether it was clicked this frame.
pub(crate) fn button(ui: &mut UiFrame, id: WidgetId, rect: Rect, label: &str, _grf: bool) -> bool {
    let resp = ui.interact(id, rect);
    if resp.hovered() {
        ui.any_interactive_hovered = true;
    }
    let color = if resp.hovered() {
        [0.30, 0.30, 0.38, 1.0]
    } else {
        [0.20, 0.20, 0.26, 1.0]
    };
    let (v, i) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, color);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
    ui.text_centered(
        rect.x,
        rect.y + rect.h * 0.5 + 4.0,
        rect.w,
        label,
        [0.9, 0.9, 0.9, 1.0],
    );
    resp.clicked()
}
