use crate::{BuildCtx, InGameWindow, Window};
use crate::helper::window_chrome::{
    GZE_BLUE_LEFT, TITLEBAR_TEX, draw_container, draw_exp_bar, draw_gauge, draw_hline,
    draw_sys_button, draw_titlebar, gauge_texture_paths, label_color, text_color,
};
use ragnarok_game::companion::HomunculusState;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;
use crate::helper::colors;

pub const HOMUN_WINDOW_ID: WidgetId = WidgetId(2900);
const CLOSE_BTN_ID: WidgetId = WidgetId(2901);
const FEED_BTN_ID: WidgetId = WidgetId(2902);
const DEL_BTN_ID: WidgetId = WidgetId(2903);
const RENAME_INPUT_ID: WidgetId = WidgetId(2905);
const RENAME_BTN_ID: WidgetId = WidgetId(2906);
const SKILL_BTN_ID: WidgetId = WidgetId(2907);
const REST_BTN_ID: WidgetId = WidgetId(2908);
const AI_BTN_ID: WidgetId = WidgetId(2909);

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";


const RENAME_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_rewrite.bmp",
    hover: "data/texture/유저인터페이스/btn_rewrite_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_rewrite_b.bmp",
};
const DEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_del.bmp",
    hover: "data/texture/유저인터페이스/btn_del_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_del_b.bmp",
};
const SKILL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_skill.bmp",
    hover: "data/texture/유저인터페이스/btn_skill_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_skill_b.bmp",
};
const FEED_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_feed.bmp",
    hover: "data/texture/유저인터페이스/btn_feed_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_feed_b.bmp",
};

const WIN_W: f32 = 288.0;
const TITLE_H: f32 = 17.0;
const WIN_H: f32 = 224.0;
const PANEL_H: f32 = WIN_H - TITLE_H;
const LEFT_W: f32 = 88.0;
const PAD: f32 = 6.0;
const CELL_H: f32 = 21.0;
const BAR_H: f32 = 11.0;
const EXP_BAR_H: f32 = 4.0;
const BASELINE: f32 = 10.0;

const NOTE_COLOR: [f32; 4] = crate::helper::colors::RED;

pub struct HomunWindow {
    pub has_grf_textures: bool,
    visible: bool,
    rename_input: TextInput,
    bar_cap_w: f32,
    del_size: (f32, f32),
    skill_size: (f32, f32),
    feed_size: (f32, f32),
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
            bar_cap_w: 4.0,
            del_size: (42.0, 20.0),
            skill_size: (42.0, 20.0),
            feed_size: (42.0, 20.0),
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

    fn build_body(
        &mut self,
        ui: &mut UiFrame,
        homun: Option<&HomunculusState>,
    ) -> Vec<GameEvent> {
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
        let lc = label_color(grf);

        let win = ui.window_at(HOMUN_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 200.0, 120.0);
        let x = win.x;
        let y = win.y;
        ui.interact(HOMUN_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 16.0, y + 13.0, "Homunculus Info", tc);

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
        );

        draw_container(ui, x, y + TITLE_H, WIN_W, PANEL_H, grf);

        // Left boxed stat column.
        let stats = [
            ("Atk", homun.atk),
            ("Matk", homun.matk),
            ("Hit", homun.hit),
            ("Critical", homun.critical),
            ("Def", homun.def),
            ("Mdef", homun.mdef),
            ("Flee", homun.flee),
            ("Aspd", homun.aspd),
        ];
        let cell_x = x + PAD;
        let cell_w = LEFT_W - PAD;
        let mut ly = y + TITLE_H + 2.0;
        for (label, value) in stats {
            let by = ly + BASELINE + 2.0;
            ui.text_bold(cell_x + 4.0, by, label, lc);
            ui.text_right(cell_x + cell_w - 4.0, by, &value.to_string(), tc);
            draw_hline(ui, cell_x, ly + CELL_H - 6.0, cell_w);
            ly += CELL_H;
        }

        // Right panel.
        let rx = x + LEFT_W + PAD + 10.0;
        let right_edge = x + WIN_W - PAD;
        let bar_w = right_edge - rx;
        let mut ry = y + TITLE_H + 4.0;

        if homun.renamed {
            ui.text(rx, ry + BASELINE, "Name", tc);
            let (color, shadow) = colors::GREEN_WITH_SHADOW;
            ui.text_with_shadow(rx + 34.0, ry + BASELINE, &homun.name, color, shadow);
            ry += 20.0;
        } else {
            let input_w = bar_w - 44.0;
            let input_rect = Rect::new(rx, ry, input_w, 14.0);
            let bg = if grf {
                TextInputBg::Gray
            } else {
                TextInputBg::Default
            };
            ui.text_input(RENAME_INPUT_ID, input_rect, &mut self.rename_input, bg);
            let btn_rect = Rect::new(rx + input_w + 4.0, ry, 40.0, 14.0);
            if ui.button(RENAME_BTN_ID, btn_rect, &RENAME_BTN, "Name").clicked() {
                let name = self.rename_input.text.trim().to_string();
                if !name.is_empty() {
                    events.push(GameEvent::RequestRenameHomun { name });
                    self.rename_input.text.clear();
                }
            }
            ry += 20.0;
        }

        // Level + del / Skill buttons.
        ui.text(rx, ry + BASELINE, &format!("lvl {}", homun.level), tc);
        let (sw, sh) = self.skill_size;
        let (dw, dh) = self.del_size;
        let skill_rect = Rect::new(right_edge - sw, ry, sw, sh);
        let del_rect = Rect::new(right_edge - sw - 4.0 - dw, ry, dw, dh);
        if ui.button(DEL_BTN_ID, del_rect, &DEL_BTN, "Delete").clicked() {
            events.push(GameEvent::RequestHomunDelete);
        }
        if ui.button(SKILL_BTN_ID, skill_rect, &SKILL_BTN, "Skill").clicked() {
            events.push(GameEvent::ToggleHomunSkillWindow);
        }
        ry += 24.0;

        ry = bar(
            ui, rx, ry, bar_w, "HP", homun.hp, homun.max_hp, GaugeKind::Hp, self.bar_cap_w, tc, lc,
            grf,
        );
        ry = bar(
            ui, rx, ry, bar_w, "SP", homun.sp, homun.max_sp, GaugeKind::Sp, self.bar_cap_w, tc, lc,
            grf,
        );
        ry += 3.0;

        // EXP row: label + value, Feed button on the right, gauge below.
        let (fw, fh) = self.feed_size;
        ui.text(rx, ry + BASELINE, "EXP", tc);
        ui.text(rx + 30.0, ry + BASELINE, &homun.exp.max(0).to_string(), tc);
        let feed_rect = Rect::new(right_edge - fw, ry, fw, fh);
        if ui.button(FEED_BTN_ID, feed_rect, &FEED_BTN, "Feed").clicked() {
            events.push(GameEvent::RequestHomunMenu { command: 1 });
        }
        let exp_ratio = if homun.max_exp > 0 {
            (homun.exp.max(0) as f32 / homun.max_exp as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        draw_exp_bar(ui, rx, ry + 14.0, bar_w - fw - 6.0, EXP_BAR_H, exp_ratio, grf);
        ry += fh.max(BAR_H + 14.0) + 3.0;

        ui.text(rx, ry + BASELINE, "Hunger", tc);
        ui.text(rx + 46.0, ry + BASELINE, &format!("{} / 100", homun.hunger), tc);
        let hunger_ratio = (homun.hunger.max(0) as f32 / 100.0).clamp(0.0, 1.0);
        draw_exp_bar(ui, rx, ry + 14.0, bar_w, EXP_BAR_H, hunger_ratio, grf);
        ry += BAR_H + 16.0;

        ui.text(rx, ry + BASELINE, "Intimacy", tc);
        ui.text(rx + 52.0, ry + BASELINE, intimacy_label(homun.intimacy), tc);
        ry += 16.0;


        // Red note, placed below the left stat column.
        let note_y = y + TITLE_H + 2.0 + 8.0 * CELL_H + 4.0;
        ui.text(x + PAD, note_y + BASELINE, "Homunculus get", NOTE_COLOR);
        ui.text(x + PAD, note_y + BASELINE + 13.0, "10% of EXP from player.", NOTE_COLOR);

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl InGameWindow for HomunWindow {
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.build_body(ui, ctx.homunculus)
    }
}

impl Window for HomunWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, _)) = size_fn(GZE_BLUE_LEFT) {
            self.bar_cap_w = w as f32;
        }
        if let Some((w, h)) = size_fn(DEL_BTN.normal) {
            self.del_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(SKILL_BTN.normal) {
            self.skill_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(FEED_BTN.normal) {
            self.feed_size = (w as f32, h as f32);
        }
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, WIN_H)
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            DEL_BTN.normal,
            DEL_BTN.hover,
            DEL_BTN.pressed,
            SKILL_BTN.normal,
            SKILL_BTN.hover,
            SKILL_BTN.pressed,
            FEED_BTN.normal,
            FEED_BTN.hover,
            FEED_BTN.pressed,
        ];
        paths.extend(gauge_texture_paths());
        paths
    }
}

/// Homunculus intimacy grade from the client-scale relationship value (0..1000).
fn intimacy_label(intimacy: i16) -> &'static str {
    match intimacy {
        i if i >= 911 => "Loyal",
        i if i >= 751 => "Cordial",
        i if i >= 251 => "Neutral",
        i if i >= 101 => "Shy",
        i if i >= 11 => "Awkward",
        i if i >= 4 => "Hate",
        _ => "Hate with passion",
    }
}

#[derive(Clone, Copy)]
pub(crate) enum GaugeKind {
    Hp,
    Sp,
}

const HPSP_LABEL_W: f32 = 22.0;

/// Draws a HP/SP gauge with the label to the left of the bar and the value
/// centered over it, returning the next y cursor.
#[allow(clippy::too_many_arguments)]
pub(crate) fn bar(
    ui: &mut UiFrame,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    cur: u32,
    max: u32,
    kind: GaugeKind,
    cap_w: f32,
    tc: [f32; 4],
    _label_c: [f32; 4],
    has_grf: bool,
) -> f32 {
    let ratio = if max > 0 {
        (cur as f32 / max as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let is_red = matches!(kind, GaugeKind::Hp) && ratio < 0.25;
    ui.text(x, y + BAR_H - 2.0, label, tc);
    let bx = x + HPSP_LABEL_W;
    let bw = w - HPSP_LABEL_W;
    draw_gauge(ui, bx, y, bw, BAR_H, cap_w, ratio, is_red, has_grf);
    ui.text_centered(bx, y + BAR_H - 2.0, bw, &format!("{cur} / {max}"), tc);
    y + BAR_H + 3.0
}
