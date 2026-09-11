use crate::helper::colors;
use crate::helper::window_chrome::{
    EXP_BAR_FILL, GZE_BLUE_LEFT, TITLEBAR_TEX, draw_container, draw_exp_bar, draw_gauge,
    draw_hline, draw_sys_button, draw_titlebar, draw_value_bar, gauge_texture_paths, label_color,
    text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::companion::{HomunculusState, aspd_display};
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const HOMUN_WINDOW_ID: WidgetId = WidgetId(2900);
const CLOSE_BTN_ID: WidgetId = WidgetId(2901);
const FEED_BTN_ID: WidgetId = WidgetId(2902);
const DEL_BTN_ID: WidgetId = WidgetId(2903);
const RENAME_INPUT_ID: WidgetId = WidgetId(2905);
const RENAME_BTN_ID: WidgetId = WidgetId(2906);
const SKILL_BTN_ID: WidgetId = WidgetId(2907);
const REST_BTN_ID: WidgetId = WidgetId(2908);
const AI_BTN_ID: WidgetId = WidgetId(2909);

const BG_TEX: &str = ragnarok_resources::ui::basic::HOMUNINFO_BG;
const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;

const RENAME_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_REWRITE,
    hover: ragnarok_resources::ui::BTN_REWRITE_A,
    pressed: ragnarok_resources::ui::BTN_REWRITE_B,
};
const DEL_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_DEL,
    hover: ragnarok_resources::ui::BTN_DEL_A,
    pressed: ragnarok_resources::ui::BTN_DEL_B,
};
const SKILL_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_SKILL,
    hover: ragnarok_resources::ui::BTN_SKILL_A,
    pressed: ragnarok_resources::ui::BTN_SKILL_B,
};
const FEED_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_FEED,
    hover: ragnarok_resources::ui::BTN_FEED_A,
    pressed: ragnarok_resources::ui::BTN_FEED_B,
};

const WIN_W: f32 = 280.0;
const WIN_H: f32 = 180.0;
const TITLE_H: f32 = 17.0;
const BAR_H: f32 = 11.0;
const EXP_BAR_H: f32 = 4.0;
const BASELINE: f32 = 10.0;

/// Right edge the eight stat values are aligned to.
const STAT_VALUE_RIGHT: f32 = 79.0;
const STAT_TOP: f32 = 24.0;
const STAT_PITCH: f32 = 18.0;
const RIGHT_LABEL_X: f32 = 100.0;
const RIGHT_VALUE_X: f32 = 135.0;
const GAUGE_X: f32 = 117.0;
const GAUGE_W: f32 = 85.0;
const GAUGE_H: f32 = 9.0;
const HPSP_TEXT_X: f32 = 218.0;
const SMALL_BAR_X: f32 = 101.0;
const SMALL_BAR_W: f32 = 102.0;
const VALUE_RIGHT_X: f32 = 200.0;

/// Hunger below this draws the gauge red instead of blue.
const HUNGER_LOW: i16 = 25;

pub struct HomunWindow {
    pub has_grf_textures: bool,
    visible: bool,
    rename_input: TextInput,
    bar_cap_w: f32,
    bg_size: (f32, f32),
    rename_size: (f32, f32),
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
            bg_size: (0.0, 0.0),
            rename_size: (40.0, 16.0),
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
        data: &DataTable,
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

        // The background carries the stat labels, their rules, and the HP and SP
        // labels with their empty gauge troughs.
        if grf && self.bg_size.0 > 0.0 {
            let (v, i) = draw::quad_vertices(x, y, self.bg_size.0, self.bg_size.1, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(BG_TEX.to_string()),
            });
        } else {
            draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
            draw_container(ui, x, y + TITLE_H, WIN_W, WIN_H - TITLE_H, grf);
        }

        ui.text(x + 5.0, y + 2.0 + BASELINE, "Homunculus Info", tc);

        let sys_w = 11.0;
        let close_rect = Rect::new(x + WIN_W - 14.0, y + 3.0, sys_w, sys_w);
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

        let stats = [
            ("Atk", homun.atk),
            ("Matk", homun.matk),
            ("Hit", homun.hit),
            ("Critical", homun.critical),
            ("Def", homun.def),
            ("Mdef", homun.mdef),
            ("Flee", homun.flee),
            ("Aspd", aspd_display(homun.aspd)),
        ];
        for (row, (label, value)) in stats.iter().enumerate() {
            let by = y + STAT_TOP + STAT_PITCH * row as f32 + BASELINE;
            if !grf {
                ui.text_bold(x + 6.0, by, label, lc);
                draw_hline(ui, x + 6.0, by + 4.0, STAT_VALUE_RIGHT - 6.0);
            }
            ui.text_right(x + STAT_VALUE_RIGHT, by, &value.to_string(), tc);
        }

        // Name: an edit box and a Rename button until the server accepts a name.
        if homun.renamed {
            if !grf {
                ui.text(x + RIGHT_LABEL_X, y + 24.0 + BASELINE, "Name", tc);
            }
            let (color, shadow) = colors::GREEN_WITH_SHADOW;
            ui.text_with_shadow(
                x + RIGHT_VALUE_X,
                y + 24.0 + BASELINE,
                &homun.name,
                color,
                shadow,
            );
        } else {
            let input_rect = Rect::new(x + RIGHT_VALUE_X, y + 22.0, 80.0, 16.0);
            let bg = if grf {
                TextInputBg::Gray
            } else {
                TextInputBg::Default
            };
            ui.text_input(RENAME_INPUT_ID, input_rect, &mut self.rename_input, bg);
            let (rw, rh) = self.rename_size;
            let btn_rect = Rect::new(x + 232.0, y + 21.0, rw, rh);
            if ui
                .button(RENAME_BTN_ID, btn_rect, &RENAME_BTN, "Name")
                .clicked()
            {
                let name = self.rename_input.text.trim().to_string();
                if !name.is_empty() {
                    events.push(GameEvent::RequestRenameHomun { name });
                    self.rename_input.text.clear();
                }
            }
        }

        if !grf {
            ui.text(x + RIGHT_LABEL_X, y + 47.0 + BASELINE, "Level", tc);
        }
        ui.text(
            x + RIGHT_VALUE_X,
            y + 47.0 + BASELINE,
            &homun.level.to_string(),
            tc,
        );
        let (dw, dh) = self.del_size;
        let (sw, sh) = self.skill_size;
        if ui
            .button(
                DEL_BTN_ID,
                Rect::new(x + 187.0, y + 44.0, dw, dh),
                &DEL_BTN,
                "Delete",
            )
            .clicked()
        {
            events.push(GameEvent::RequestHomunDelete);
        }
        if ui
            .button(
                SKILL_BTN_ID,
                Rect::new(x + 232.0, y + 44.0, sw, sh),
                &SKILL_BTN,
                "Skill",
            )
            .clicked()
        {
            events.push(GameEvent::ToggleHomunSkillWindow);
        }

        gauge_row(
            ui,
            x,
            y + 73.0,
            "HP",
            homun.hp,
            homun.max_hp,
            true,
            self.bar_cap_w,
            tc,
            grf,
        );
        gauge_row(
            ui,
            x,
            y + 88.0,
            "SP",
            homun.sp,
            homun.max_sp,
            false,
            self.bar_cap_w,
            tc,
            grf,
        );

        // EXP shows what the next level costs, not what has been earned.
        if !grf {
            ui.text(x + RIGHT_LABEL_X, y + 105.0 + BASELINE, "EXP", tc);
        }
        ui.text_right(
            x + VALUE_RIGHT_X,
            y + 104.0 + BASELINE,
            &homun.max_exp.max(0).to_string(),
            tc,
        );
        let exp_ratio = if homun.max_exp > 0 {
            (homun.exp.max(0) as f32 / homun.max_exp as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        draw_exp_bar(
            ui,
            x + SMALL_BAR_X,
            y + 119.0,
            SMALL_BAR_W,
            EXP_BAR_H,
            exp_ratio,
            grf,
        );

        self.accessory(ui, x, y, homun.accessory, data, tc, grf);

        if !grf {
            ui.text(x + RIGHT_LABEL_X, y + 131.0 + BASELINE, "Hunger", tc);
        }
        ui.text_right(
            x + VALUE_RIGHT_X,
            y + 130.0 + BASELINE,
            &format!("{} / 100", homun.hunger),
            tc,
        );
        let hunger_fill = if homun.hunger < HUNGER_LOW {
            colors::RED
        } else {
            EXP_BAR_FILL
        };
        draw_value_bar(
            ui,
            x + SMALL_BAR_X,
            y + 145.0,
            SMALL_BAR_W,
            EXP_BAR_H,
            (homun.hunger.max(0) as f32 / 100.0).clamp(0.0, 1.0),
            hunger_fill,
            grf,
        );

        if !grf {
            ui.text(x + RIGHT_LABEL_X, y + 159.0 + BASELINE, "Intimacy", tc);
        }
        ui.text(x + 140.0, y + 159.0 + BASELINE, ":", tc);
        ui.text(
            x + 150.0,
            y + 159.0 + BASELINE,
            intimacy_label(homun.intimacy),
            tc,
        );

        let (fw, fh) = self.feed_size;
        if ui
            .button(
                FEED_BTN_ID,
                Rect::new(x + 210.0, y + 155.0, fw, fh),
                &FEED_BTN,
                "Feed",
            )
            .clicked()
        {
            events.push(GameEvent::RequestHomunMenu { command: 1 });
        }

        ui.has_grf_textures = prev_grf;
        events
    }

    fn accessory(
        &self,
        ui: &mut UiFrame,
        x: f32,
        y: f32,
        accessory: u16,
        data: &DataTable,
        tc: [f32; 4],
        grf: bool,
    ) {
        if !grf {
            ui.text(x + 216.0, y + 105.0 + BASELINE, "Accessory", tc);
        }
        let icon = (accessory != 0)
            .then(|| {
                data.item_resource
                    .as_ref()
                    .and_then(|t| t.get_resource_name(accessory))
                    .map(ragnarok_resources::ui::item::icon)
            })
            .flatten();
        match icon {
            Some(path) => {
                let (v, i) = draw::quad_vertices(x + 235.0, y + 122.0, 24.0, 24.0, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(path),
                });
            }
            None => ui.text(x + 220.0, y + 130.0 + BASELINE, "Unequipped", tc),
        }
    }
}

/// One HP or SP row: the bar at the official offset with the value to its right.
#[allow(clippy::too_many_arguments)]
fn gauge_row(
    ui: &mut UiFrame,
    x: f32,
    y: f32,
    label: &str,
    cur: u32,
    max: u32,
    is_hp: bool,
    cap_w: f32,
    tc: [f32; 4],
    grf: bool,
) {
    let ratio = if max > 0 {
        (cur as f32 / max as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if !grf {
        ui.text(x + RIGHT_LABEL_X, y + BASELINE, label, tc);
    }
    draw_gauge(
        ui,
        x + GAUGE_X,
        y,
        GAUGE_W,
        GAUGE_H,
        cap_w,
        ratio,
        is_hp && ratio < 0.25,
        grf,
    );
    ui.text(
        x + HPSP_TEXT_X,
        y - 3.0 + BASELINE,
        &format!("{cur} / {max}"),
        tc,
    );
}

impl InGameWindow for HomunWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.visible
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.visible = false;
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.build_body(ui, ctx.homunculus, ctx.data)
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
        if let Some((w, h)) = size_fn(BG_TEX) {
            self.bg_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(RENAME_BTN.normal) {
            self.rename_size = (w as f32, h as f32);
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
            BG_TEX,
            TITLEBAR_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            DEL_BTN.normal,
            DEL_BTN.hover,
            DEL_BTN.pressed,
            SKILL_BTN.normal,
            SKILL_BTN.hover,
            SKILL_BTN.pressed,
            RENAME_BTN.normal,
            RENAME_BTN.hover,
            RENAME_BTN.pressed,
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
        i if i > 1000 => "Unknown",
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

#[cfg(test)]
mod tests {
    use super::intimacy_label;

    #[test]
    fn intimacy_grades_span_every_band() {
        assert_eq!(intimacy_label(3), "Hate with passion");
        assert_eq!(intimacy_label(4), "Hate");
        assert_eq!(intimacy_label(11), "Awkward");
        assert_eq!(intimacy_label(101), "Shy");
        assert_eq!(intimacy_label(251), "Neutral");
        assert_eq!(intimacy_label(751), "Cordial");
        assert_eq!(intimacy_label(1000), "Loyal");
        assert_eq!(intimacy_label(1001), "Unknown");
    }
}
