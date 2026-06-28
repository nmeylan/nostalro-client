use crate::helper::window_chrome::{draw_sys_button, text_color};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const BASIC_INFO_WINDOW_ID: WidgetId = WidgetId(1400);
const MINI_BTN_ID: WidgetId = WidgetId(1401);
const CLOSE_BTN_ID: WidgetId = WidgetId(1402);
const BTN_OPTION_ID: WidgetId = WidgetId(1410);
const BTN_STATUS_ID: WidgetId = WidgetId(1411);
const BTN_EQUIP_ID: WidgetId = WidgetId(1412);
const BTN_INVENTORY_ID: WidgetId = WidgetId(1413);
const BTN_MAP_ID: WidgetId = WidgetId(1414);
const BTN_SKILL_ID: WidgetId = WidgetId(1415);
const BTN_PARTY_ID: WidgetId = WidgetId(1416);

const BG_TEX: &str = "data/texture/유저인터페이스/basic_interface/basewin_bg.bmp";
const BG_MINI_TEX: &str = "data/texture/유저인터페이스/basic_interface/basewin_mini.bmp";

const BAR_RED_LEFT: &str = "data/texture/유저인터페이스/basic_interface/gzered_left.bmp";
const BAR_RED_MID: &str = "data/texture/유저인터페이스/basic_interface/gzered_mid.bmp";
const BAR_RED_RIGHT: &str = "data/texture/유저인터페이스/basic_interface/gzered_right.bmp";
const BAR_BLUE_LEFT: &str = "data/texture/유저인터페이스/basic_interface/gzeblue_left.bmp";
const BAR_BLUE_MID: &str = "data/texture/유저인터페이스/basic_interface/gzeblue_mid.bmp";
const BAR_BLUE_RIGHT: &str = "data/texture/유저인터페이스/basic_interface/gzeblue_right.bmp";

const SYS_BASE_OFF: &str = "data/texture/유저인터페이스/basic_interface/sys_base_off.bmp";
const SYS_BASE_ON: &str = "data/texture/유저인터페이스/basic_interface/sys_base_on.bmp";
const SYS_MINI_OFF: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_off.bmp";
const SYS_MINI_ON: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_on.bmp";

const BTN_OPTION: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/btn_option_off.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/btn_option_on.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/btn_option_on.bmp",
};
const BTN_STATUS: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/btn_status_off.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/btn_status_on.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/btn_status_on.bmp",
};
const BTN_EQUIP: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/btn_equip_off.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/btn_equip_on.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/btn_equip_on.bmp",
};
const BTN_ITEM: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/btn_items_off.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/btn_items_on.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/btn_items_on.bmp",
};
const BTN_MAP: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/btn_map_off.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/btn_map_on.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/btn_map_on.bmp",
};
const BTN_SKILL: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/btn_skill_off.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/btn_skill_on.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/btn_skill_on.bmp",
};
const BTN_PARTY: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/btn_friend_off.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/btn_friend_on.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/btn_friend_on.bmp",
};

const WIN_W: f32 = 280.0;
const WIN_H_LARGE: f32 = 120.0;
const WIN_H_SMALL: f32 = 33.0;
const TITLE_H: f32 = 17.0;

const HP_BAR_X: f32 = 110.0;
const HP_BAR_Y: f32 = 22.0;
const SP_BAR_Y: f32 = 43.0;
const BAR_W: f32 = 85.0;
const BAR_H: f32 = 8.0;
const BAR_CAP_W: f32 = 4.0;
const BAR_MID_MAX: f32 = 77.0; // 85 - 4 - 4

const EXP_BAR_X: f32 = 84.0;
const EXP_BAR_Y: f32 = 77.0;
const JEXP_BAR_Y: f32 = 88.0;
const EXP_BAR_W: f32 = 100.0;
const EXP_BAR_H: f32 = 4.0;

const BUTTONS_RIGHT: f32 = 8.0;
const BUTTONS_TOP: f32 = 18.0;
const MENU_BTN_W: f32 = 30.0;
const MENU_BTN_H: f32 = 20.0;
const MENU_BTN_SPACING_X: f32 = 4.0;
const MENU_BTN_SPACING_Y: f32 = 4.0;

const SHADOW_DRAIN_SPEED: f32 = 3.0;

pub struct BasicInfoWindow {
    pub has_grf_textures: bool,
    minimized: bool,
    hp_shadow: f32,
    sp_shadow: f32,
    bg_size: (f32, f32),
    bg_mini_size: (f32, f32),
    bar_cap_size: (f32, f32),
    menu_btn_size: (f32, f32),
    sys_btn_size: (f32, f32),
}

impl Default for BasicInfoWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicInfoWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            minimized: false,
            hp_shadow: 1.0,
            sp_shadow: 1.0,
            bg_size: (0.0, 0.0),
            bg_mini_size: (0.0, 0.0),
            bar_cap_size: (BAR_CAP_W, BAR_H),
            menu_btn_size: (MENU_BTN_W, MENU_BTN_H),
            sys_btn_size: (11.0, 11.0),
        }
    }

    fn update_shadow(shadow: &mut f32, current: f32, delta: f32) {
        if *shadow > current {
            *shadow -= (*shadow - current) * delta * SHADOW_DRAIN_SPEED;
            if (*shadow - current).abs() < 0.001 {
                *shadow = current;
            }
        } else {
            *shadow = current;
        }
    }

    fn draw_bar_grf(
        ui: &mut UiFrame,
        x: f32,
        y: f32,
        fill_pct: f32,
        is_red: bool,
        bar_cap_size: (f32, f32),
    ) {
        if fill_pct <= 0.0 {
            return;
        }
        let pct = fill_pct.clamp(0.0, 1.0);
        let mid_w = (pct * BAR_MID_MAX).floor();
        let cap_w = bar_cap_size.0;
        let cap_h = bar_cap_size.1;

        let (left_tex, mid_tex, right_tex) = if is_red {
            (BAR_RED_LEFT, BAR_RED_MID, BAR_RED_RIGHT)
        } else {
            (BAR_BLUE_LEFT, BAR_BLUE_MID, BAR_BLUE_RIGHT)
        };

        let white = [1.0, 1.0, 1.0, 1.0];

        let (v, i) = draw::quad_vertices(x, y, cap_w, cap_h, white);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(left_tex.to_string()),
        });

        if mid_w > 0.0 {
            let (v, i) = draw::quad_vertices(x + cap_w, y, mid_w, cap_h, white);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(mid_tex.to_string()),
            });
        }

        let (v, i) = draw::quad_vertices(x + cap_w + mid_w, y, cap_w, cap_h, white);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(right_tex.to_string()),
        });
    }

    fn draw_bar_fallback(
        ui: &mut UiFrame,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill_pct: f32,
        shadow_pct: f32,
        fill_color: [f32; 4],
        shadow_color: [f32; 4],
    ) {
        let bg_color = [0.15, 0.15, 0.15, 0.9];
        let (v, i) = draw::quad_vertices(x, y, w, h, bg_color);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });

        if shadow_pct > 0.0 {
            let sw = (w * shadow_pct.clamp(0.0, 1.0)).max(0.0);
            let (v, i) = draw::quad_vertices(x, y, sw, h, shadow_color);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        if fill_pct > 0.0 {
            let fw = (w * fill_pct.clamp(0.0, 1.0)).max(0.0);
            let (v, i) = draw::quad_vertices(x, y, fw, h, fill_color);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
    }

    fn draw_exp_bar(ui: &mut UiFrame, x: f32, y: f32, fill_pct: f32, grf: bool) {
        if grf {
            let border_color = [0.69, 0.69, 0.69, 1.0];
            let (v, i) = draw::quad_vertices(x, y, EXP_BAR_W + 2.0, EXP_BAR_H + 2.0, border_color);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
            let (v, i) = draw::quad_vertices(x + 1.0, y + 1.0, EXP_BAR_W, EXP_BAR_H, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        } else {
            let (v, i) = draw::quad_vertices(
                x,
                y,
                EXP_BAR_W + 2.0,
                EXP_BAR_H + 2.0,
                [0.3, 0.3, 0.35, 0.9],
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
        if fill_pct > 0.0 {
            let fw = (EXP_BAR_W * fill_pct.clamp(0.0, 1.0)).floor();
            let fill_color = [0.26, 0.38, 0.65, 1.0];
            let (v, i) = draw::quad_vertices(x + 1.0, y + 1.0, fw, EXP_BAR_H, fill_color);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
    }

    fn build_large(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        win: Rect,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let x = win.x;
        let y = win.y;
        let delta = ui.elapsed_secs;

        if grf && self.bg_size.0 > 0.0 {
            let (v, i) = draw::quad_vertices(x, y, self.bg_size.0, self.bg_size.1, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(BG_TEX.to_string()),
            });
        } else {
            let (v, i) = draw::quad_vertices(x, y, WIN_W, WIN_H_LARGE, [0.12, 0.12, 0.18, 0.92]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
            let bc = [0.4, 0.4, 0.5, 1.0];
            for (bx, by, bw, bh) in [
                (x, y, WIN_W, 1.0),
                (x, y, 1.0, WIN_H_LARGE),
                (x + WIN_W - 1.0, y, 1.0, WIN_H_LARGE),
                (x, y + WIN_H_LARGE - 1.0, WIN_W, 1.0),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
        }

        let close_rect = Rect::new(x + 4.0, y + 3.0, self.sys_btn_size.0, self.sys_btn_size.1);
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            close_rect,
            (self.sys_btn_size.0, self.sys_btn_size.1),
            close_resp.hovered(),
            grf,
            SYS_BASE_ON,
            SYS_BASE_OFF,
            None,
            [1.0; 4],
            [1.0; 4],
        );

        ui.text(x + 18.0, y + 13.0, "Basic Info", tc);

        let mini_rect = Rect::new(
            x + WIN_W - 2.0 - self.sys_btn_size.0,
            y + 3.0,
            self.sys_btn_size.0,
            self.sys_btn_size.1,
        );
        let mini_resp = ui.interact(MINI_BTN_ID, mini_rect);
        if mini_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if mini_resp.clicked() {
            self.minimized = true;
        }
        draw_sys_button(
            ui,
            mini_rect,
            (self.sys_btn_size.0, self.sys_btn_size.1),
            mini_resp.hovered(),
            grf,
            SYS_MINI_ON,
            SYS_MINI_OFF,
            Some('_'),
            [0.8, 0.8, 0.9, 1.0],
            [0.5, 0.5, 0.6, 1.0],
        );

        let name = if character.name.is_empty() {
            "Unknown"
        } else {
            &character.name
        };
        ui.text(x + 10.0, y + 30.0, name, tc);

        let job_name = character.job_class_name();
        ui.text(x + 10.0, y + 43.0, job_name, tc);

        let hp_pct = character.hp_percentage();
        Self::update_shadow(&mut self.hp_shadow, hp_pct, delta);
        let is_red = hp_pct < 0.25;

        ui.text(x + 90.0, y + 39.0, "HP", tc);
        if grf {
            Self::draw_bar_grf(
                ui,
                x + HP_BAR_X,
                y + HP_BAR_Y,
                hp_pct,
                is_red,
                self.bar_cap_size,
            );
        } else {
            let hp_fill = if is_red {
                [0.8, 0.2, 0.2, 1.0]
            } else {
                [0.2, 0.4, 0.8, 1.0]
            };
            Self::draw_bar_fallback(
                ui,
                x + HP_BAR_X,
                y + HP_BAR_Y,
                BAR_W,
                BAR_H,
                hp_pct,
                self.hp_shadow,
                hp_fill,
                [0.4, 0.4, 0.6, 0.7],
            );
        }
        let hp_text = format!("{} / {}", character.hp, character.max_hp);
        ui.text_centered(
            x + HP_BAR_X,
            y + HP_BAR_Y + BAR_H * 2.0 + 1.0,
            BAR_W,
            &hp_text,
            tc,
        );

        let hp_bar_rect = Rect::new(x + HP_BAR_X, y + HP_BAR_Y, BAR_W, BAR_H + 10.0);
        if hp_bar_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
            ui.tooltip(
                ui.ctx.mouse_x + 10.0,
                ui.ctx.mouse_y,
                &format!("{:.1}%", hp_pct * 100.0),
            );
        }

        let sp_pct = character.sp_percentage();
        Self::update_shadow(&mut self.sp_shadow, sp_pct, delta);

        ui.text(x + 90.0, y + 59.0, "SP", tc);
        if grf {
            Self::draw_bar_grf(
                ui,
                x + HP_BAR_X,
                y + SP_BAR_Y,
                sp_pct,
                false,
                self.bar_cap_size,
            );
        } else {
            let sp_fill = [0.2, 0.7, 0.3, 1.0];
            Self::draw_bar_fallback(
                ui,
                x + HP_BAR_X,
                y + SP_BAR_Y,
                BAR_W,
                BAR_H,
                sp_pct,
                self.sp_shadow,
                sp_fill,
                [0.3, 0.5, 0.3, 0.7],
            );
        }
        let sp_text = format!("{} / {}", character.sp, character.max_sp);
        ui.text_centered(
            x + HP_BAR_X,
            y + SP_BAR_Y + BAR_H * 2.0 + 1.0,
            BAR_W,
            &sp_text,
            tc,
        );

        let sp_bar_rect = Rect::new(x + HP_BAR_X, y + SP_BAR_Y, BAR_W, BAR_H + 10.0);
        if sp_bar_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
            ui.tooltip(
                ui.ctx.mouse_x + 10.0,
                ui.ctx.mouse_y,
                &format!("{:.1}%", sp_pct * 100.0),
            );
        }

        let blvl_text = format!("Base Lv. {}", character.base_level);
        ui.text(x + 15.0, y + 80.0, &blvl_text, tc);
        let base_exp_pct = character.base_exp_percentage();
        Self::draw_exp_bar(ui, x + EXP_BAR_X, y + EXP_BAR_Y, base_exp_pct, grf);
        let exp_text = format!("{:.1}%", (base_exp_pct * 1000.0).floor() * 0.1);

        let exp_bar_rect = Rect::new(
            x + EXP_BAR_X,
            y + EXP_BAR_Y,
            EXP_BAR_W + 2.0,
            EXP_BAR_H + 2.0,
        );
        if exp_bar_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
            ui.tooltip(ui.ctx.mouse_x + 10.0, ui.ctx.mouse_y, &exp_text);
        }

        let jlvl_text = format!("Job Lv. {}", character.job_level);
        ui.text(x + 15.0, y + 93.0, &jlvl_text, tc);
        let job_exp_pct = character.job_exp_percentage();
        Self::draw_exp_bar(ui, x + EXP_BAR_X, y + JEXP_BAR_Y, job_exp_pct, grf);
        let jexp_text = format!("{:.1}%", (job_exp_pct * 1000.0).floor() * 0.1);

        let jexp_bar_rect = Rect::new(
            x + EXP_BAR_X,
            y + JEXP_BAR_Y,
            EXP_BAR_W + 2.0,
            EXP_BAR_H + 2.0,
        );
        if jexp_bar_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
            ui.tooltip(ui.ctx.mouse_x + 10.0, ui.ctx.mouse_y, &jexp_text);
        }

        let weight = character.inventory.weight;
        let max_weight = character.inventory.max_weight;
        let weight_over = max_weight > 0 && weight as f32 / max_weight as f32 >= 0.5;
        let weight_color = if weight_over {
            [1.0, 0.0, 0.0, 1.0]
        } else {
            tc
        };
        let weight_text = format!("Weight : {} / {}", weight / 10, max_weight / 10);
        let weight_width = ui.atlas.measure_text(&weight_text);
        ui.text(x + 5.0, y + 115.0, &weight_text, weight_color);

        let zeny_text = format!("Zeny : {}", format_zeny(character.inventory.zeny));
        ui.text(x + 5.0 + weight_width + 8.0, y + 115.0, &zeny_text, tc);

        let btn_w = self.menu_btn_size.0;
        let btn_h = self.menu_btn_size.1;
        let col2_x = x + WIN_W - BUTTONS_RIGHT - btn_w;
        let col1_x = col2_x - MENU_BTN_SPACING_X - btn_w;

        let row_y = |row: usize| y + BUTTONS_TOP + row as f32 * (btn_h + MENU_BTN_SPACING_Y);

        let menu_buttons: &[(WidgetId, f32, f32, &ButtonTextures, &str)] = &[
            (BTN_OPTION_ID, col1_x, row_y(0), &BTN_OPTION, "Option"),
            (BTN_STATUS_ID, col2_x, row_y(0), &BTN_STATUS, "Status"),
            (
                BTN_EQUIP_ID,
                col1_x,
                row_y(1),
                &BTN_EQUIP,
                "Equipment (Alt+Q)",
            ),
            (
                BTN_INVENTORY_ID,
                col2_x,
                row_y(1),
                &BTN_ITEM,
                "Inventory (Alt+E)",
            ),
            (BTN_MAP_ID, col1_x, row_y(2), &BTN_MAP, "Map"),
            (BTN_SKILL_ID, col2_x, row_y(2), &BTN_SKILL, "Skills (Alt+S)"),
            (BTN_PARTY_ID, col1_x, row_y(3), &BTN_PARTY, "Party"),
        ];

        for &(id, bx, by, textures, tooltip_text) in menu_buttons {
            let rect = Rect::new(bx, by, btn_w, btn_h);
            let resp = ui.button(id, rect, textures, "");
            if resp.clicked() {
                match id {
                    BTN_EQUIP_ID => events.push(GameEvent::ToggleEquipment),
                    BTN_INVENTORY_ID => events.push(GameEvent::ToggleInventory),
                    BTN_MAP_ID => events.push(GameEvent::ToggleMinimap),
                    BTN_SKILL_ID => events.push(GameEvent::ToggleSkills),
                    BTN_STATUS_ID => events.push(GameEvent::ToggleStatusWindow),
                    BTN_PARTY_ID => events.push(GameEvent::TogglePartyWindow),
                    _ => {}
                }
            }
            if resp.hovered() {
                ui.tooltip(ui.ctx.mouse_x, ui.ctx.mouse_y, tooltip_text);
            }
        }

        events
    }

    fn build_small(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        win: Rect,
    ) -> Vec<GameEvent> {
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let x = win.x;
        let y = win.y;

        if grf && self.bg_mini_size.0 > 0.0 {
            let (v, i) =
                draw::quad_vertices(x, y, self.bg_mini_size.0, self.bg_mini_size.1, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(BG_MINI_TEX.to_string()),
            });
        } else {
            let (v, i) = draw::quad_vertices(x, y, WIN_W, WIN_H_SMALL, [0.12, 0.12, 0.18, 0.92]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
            let bc = [0.4, 0.4, 0.5, 1.0];
            for (bx, by, bw, bh) in [
                (x, y, WIN_W, 1.0),
                (x, y, 1.0, WIN_H_SMALL),
                (x + WIN_W - 1.0, y, 1.0, WIN_H_SMALL),
                (x, y + WIN_H_SMALL - 1.0, WIN_W, 1.0),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
        }

        let name = if character.name.is_empty() {
            "Unknown"
        } else {
            &character.name
        };
        ui.text(x + 18.0, y + 12.0, name, tc);

        let mini_rect = Rect::new(
            x + WIN_W - 2.0 - self.sys_btn_size.0,
            y + 3.0,
            self.sys_btn_size.0,
            self.sys_btn_size.1,
        );
        let mini_resp = ui.interact(MINI_BTN_ID, mini_rect);
        if mini_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if mini_resp.clicked() {
            self.minimized = false;
        }
        draw_sys_button(
            ui,
            mini_rect,
            (self.sys_btn_size.0, self.sys_btn_size.1),
            mini_resp.hovered(),
            grf,
            SYS_MINI_ON,
            SYS_MINI_OFF,
            Some('+'),
            [0.8, 0.8, 0.9, 1.0],
            [0.5, 0.5, 0.6, 1.0],
        );

        let job_name = character.job_class_name();
        let exp_pct = character.base_exp_percentage() * 100.0;
        let info_text = format!(
            "Lv.{} / {} / Lv.{} / Exp. {:.1}%",
            character.base_level, job_name, character.job_level, exp_pct
        );
        ui.text_right(x + WIN_W - 18.0, y + 12.0, &info_text, tc);

        let hp_sp_text = format!(
            "HP. {} / {} | SP. {} / {}",
            character.hp, character.max_hp, character.sp, character.max_sp
        );
        ui.text_right(x + WIN_W - 5.0, y + 28.0, &hp_sp_text, tc);

        Vec::new()
    }
}

impl Window for BasicInfoWindow {
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
        if let Some((w, h)) = size_fn(BG_MINI_TEX) {
            self.bg_mini_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(BAR_BLUE_LEFT) {
            self.bar_cap_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(BTN_EQUIP.normal) {
            self.menu_btn_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(SYS_MINI_OFF) {
            self.sys_btn_size = (w as f32, h as f32);
        }
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            BG_TEX,
            BG_MINI_TEX,
            BAR_RED_LEFT,
            BAR_RED_MID,
            BAR_RED_RIGHT,
            BAR_BLUE_LEFT,
            BAR_BLUE_MID,
            BAR_BLUE_RIGHT,
            SYS_BASE_OFF,
            SYS_BASE_ON,
            SYS_MINI_OFF,
            SYS_MINI_ON,
            BTN_OPTION.normal,
            BTN_OPTION.hover,
            BTN_STATUS.normal,
            BTN_STATUS.hover,
            BTN_EQUIP.normal,
            BTN_EQUIP.hover,
            BTN_ITEM.normal,
            BTN_ITEM.hover,
            BTN_MAP.normal,
            BTN_MAP.hover,
            BTN_SKILL.normal,
            BTN_SKILL.hover,
            BTN_PARTY.normal,
            BTN_PARTY.hover,
        ]
    }
}

impl InGameWindow for BasicInfoWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        _data: &DataTable,
    ) -> Vec<GameEvent> {
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;

        let win_h = if self.minimized {
            WIN_H_SMALL
        } else {
            WIN_H_LARGE
        };
        let win = ui.window_at(BASIC_INFO_WINDOW_ID, WIN_W, win_h, TITLE_H, 0.0, 0.0);

        let win_rect = Rect::new(win.x, win.y, WIN_W, win_h);
        ui.interact(BASIC_INFO_WINDOW_ID, win_rect);

        let events = if self.minimized {
            self.build_small(ui, character, win)
        } else {
            self.build_large(ui, character, win)
        };

        ui.has_grf_textures = prev_grf;
        events
    }
}

fn format_zeny(value: i32) -> String {
    if value < 0 {
        return format!("-{}", format_zeny(-value));
    }
    let s = value.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}
