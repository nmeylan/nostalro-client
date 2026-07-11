use crate::Window;
use crate::helper::window_chrome::{
    FOOTER_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_sys_button, draw_titlebar,
    text_color,
};
use ragnarok_game::companion::MercenaryState;
use ragnarok_game::event::{GameEvent, SkillInfo};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const MERCENARY_SKILL_WINDOW_ID: WidgetId = WidgetId(3100);
const CLOSE_BTN_ID: WidgetId = WidgetId(3101);
const USE_BTN_ID: WidgetId = WidgetId(3102);
const FOOTER_CLOSE_BTN_ID: WidgetId = WidgetId(3103);
const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const USE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_use.bmp",
    hover: "data/texture/유저인터페이스/btn_use_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_use_b.bmp",
};
const CLOSE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/btn_close.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/btn_close_a.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/btn_close_b.bmp",
};
const SKILL_ROW_BASE_ID: u32 = 3110;

const WIN_W: f32 = 232.0;
const TITLE_H: f32 = 17.0;
const ROW_H: f32 = 36.0;
const ICON_SIZE: f32 = 24.0;
const VISIBLE_ROWS: usize = 5;
const FOOTER_H: f32 = 24.0;
const PAD: f32 = 8.0;
const WIN_H: f32 = TITLE_H + ROW_H * VISIBLE_ROWS as f32 + FOOTER_H;

pub struct MercenarySkillWindow {
    pub has_grf_textures: bool,
    visible: bool,
    selected: usize,
    use_size: (f32, f32),
    close_size: (f32, f32),
}

impl Default for MercenarySkillWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl MercenarySkillWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            visible: false,
            selected: 0,
            use_size: (42.0, 20.0),
            close_size: (42.0, 20.0),
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
        let events = Vec::new();
        let tc = text_color(grf);

        let win = ui.window_at(MERCENARY_SKILL_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 240.0, 340.0);
        let x = win.x;
        let y = win.y;
        ui.interact(MERCENARY_SKILL_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 8.0, y + 13.0, "Mercenary Soldier Skill List", tc);

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
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
            [0.9, 0.4, 0.4, 1.0],
            [0.6, 0.3, 0.3, 1.0],
        );

        let list_h = ROW_H * VISIBLE_ROWS as f32;
        draw_container(ui, x, y + TITLE_H, WIN_W, list_h, grf);

        let list_top = y + TITLE_H;
        for (idx, skill) in merc.skills.iter().take(VISIBLE_ROWS).enumerate() {
            let row_y = list_top + idx as f32 * ROW_H;
            let row_rect = Rect::new(x, row_y, WIN_W, ROW_H);
            let row_resp = ui.interact(WidgetId(SKILL_ROW_BASE_ID + idx as u32), row_rect);
            if row_resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if row_resp.clicked() {
                self.selected = idx;
            }
            if self.selected == idx || row_resp.hovered() {
                let hl = if grf {
                    [0.85, 0.85, 0.8, 0.4]
                } else {
                    [0.3, 0.3, 0.4, 0.4]
                };
                let (v, i) = draw::quad_vertices(x, row_y, WIN_W, ROW_H, hl);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }

            let icon_x = x + PAD;
            let icon_y = row_y + (ROW_H - ICON_SIZE) * 0.5;
            let (v, i) = draw::quad_vertices(icon_x, icon_y, ICON_SIZE, ICON_SIZE, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(icon_path(skill)),
            });

            let name_x = icon_x + ICON_SIZE + 8.0;
            ui.text(name_x, row_y + 14.0, &skill.name, tc);
            ui.text(name_x, row_y + 28.0, &format!("Lv : {}", skill.level), tc);
            ui.text_right(
                x + WIN_W - PAD,
                row_y + 24.0,
                &format!("Sp : {}", skill.sp_cost),
                tc,
            );
        }

        // Footer.
        let footer_y = y + TITLE_H + list_h;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);
        ui.text(x + PAD, footer_y + 15.0, "Skill Point: 0", tc);

        let (cw, ch) = self.close_size;
        let (uw, uh) = self.use_size;
        let btn_y = footer_y + (FOOTER_H - ch) * 0.5;
        let close_footer = Rect::new(x + WIN_W - PAD - cw, btn_y, cw, ch);
        let use_rect = Rect::new(x + WIN_W - PAD - cw - 4.0 - uw, btn_y, uw, uh);
        // Mercenary skills are driven by its AI; the button exists for parity only.
        ui.button(USE_BTN_ID, use_rect, &USE_BTN, "use");
        if ui.button(FOOTER_CLOSE_BTN_ID, close_footer, &CLOSE_BTN, "close").clicked() {
            self.visible = false;
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl Window for MercenarySkillWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(USE_BTN.normal) {
            self.use_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(CLOSE_BTN.normal) {
            self.close_size = (w as f32, h as f32);
        }
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            FOOTER_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            USE_BTN.normal,
            USE_BTN.hover,
            USE_BTN.pressed,
            CLOSE_BTN.normal,
            CLOSE_BTN.hover,
            CLOSE_BTN.pressed,
        ]
    }
}

fn icon_path(skill: &SkillInfo) -> String {
    format!(
        "data/texture/유저인터페이스/item/{}.bmp",
        skill.name.to_lowercase()
    )
}
