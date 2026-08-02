use crate::helper::dialog_container::DialogContainer;
use crate::helper::window_chrome::{
    FOOTER_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_sys_button, draw_titlebar,
    text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::companion::HomunculusState;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::{GameEvent, SkillInfo};
use ragnarok_game::skill::SkillTargetType;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const HOMUN_SKILL_WINDOW_ID: WidgetId = WidgetId(2910);
const CLOSE_BTN_ID: WidgetId = WidgetId(2911);
const USE_BTN_ID: WidgetId = WidgetId(2912);
const FOOTER_CLOSE_BTN_ID: WidgetId = WidgetId(2913);
const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;

const USE_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_USE,
    hover: ragnarok_resources::ui::BTN_USE_A,
    pressed: ragnarok_resources::ui::BTN_USE_B,
};
const LEVELUP_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::basic::SKILL_UP_A,
    hover: ragnarok_resources::ui::basic::SKILL_UP_B,
    pressed: ragnarok_resources::ui::basic::SKILL_UP_C,
};
const CLOSE_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::basic::BTN_CLOSE,
    hover: ragnarok_resources::ui::basic::BTN_CLOSE_A,
    pressed: ragnarok_resources::ui::basic::BTN_CLOSE_B,
};
const SKILL_ROW_BASE_ID: u32 = 2920;
const SKILL_LEVELUP_BASE_ID: u32 = 2930;

const WIN_W: f32 = 232.0;
const TITLE_H: f32 = 17.0;
const ROW_H: f32 = 36.0;
const ICON_SIZE: f32 = 24.0;
const VISIBLE_ROWS: usize = 5;
const FOOTER_H: f32 = 24.0;
const PAD: f32 = 8.0;
const WIN_H: f32 = TITLE_H + ROW_H * VISIBLE_ROWS as f32 + FOOTER_H;

pub struct HomunSkillWindow {
    pub has_grf_textures: bool,
    visible: bool,
    selected: usize,
    use_size: (f32, f32),
    close_size: (f32, f32),
    levelup_btn_size: (f32, f32),
    tooltip_container: DialogContainer,
}

impl Default for HomunSkillWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl HomunSkillWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            visible: false,
            selected: 0,
            use_size: (42.0, 20.0),
            close_size: (42.0, 20.0),
            levelup_btn_size: (16.0, 16.0),
            tooltip_container: DialogContainer::new(),
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

        let win = ui.window_at(HOMUN_SKILL_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 240.0, 340.0);
        let x = win.x;
        let y = win.y;
        ui.interact(HOMUN_SKILL_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 16.0, y + 13.0, "Homunculus Skill List", tc);

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

        let list_h = ROW_H * VISIBLE_ROWS as f32;
        draw_container(ui, x, y + TITLE_H, WIN_W, list_h, grf);

        let list_top = y + TITLE_H;
        for (idx, skill) in homun.skills.iter().take(VISIBLE_ROWS).enumerate() {
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
                texture: TextureRef::Named(skill.icon_path()),
            });

            let name_x = icon_x + ICON_SIZE + 8.0;
            ui.text(name_x, row_y + 14.0, &skill.name, tc);
            ui.text(name_x, row_y + 28.0, &format!("Lv : {}", skill.level), tc);

            let mut sp_right = x + WIN_W - PAD;
            if skill.upgradable && homun.skill_points > 0 {
                let (lup_w, lup_h) = self.levelup_btn_size;
                let btn_x = x + WIN_W - PAD - lup_w;
                let btn_y = row_y + (ROW_H - lup_h) * 0.5;
                let btn_id = WidgetId(SKILL_LEVELUP_BASE_ID + idx as u32);
                let btn_rect = Rect::new(btn_x, btn_y, lup_w, lup_h);
                if ui.button(btn_id, btn_rect, &LEVELUP_BTN, "+").clicked() {
                    events.push(GameEvent::RequestSkillLevelUp { skill_id: skill.id });
                }
                sp_right = btn_x - 4.0;
            }
            ui.text_right(
                sp_right,
                row_y + 24.0,
                &format!("Sp : {}", skill.sp_cost),
                tc,
            );

            if row_resp.double_clicked() {
                events.push(GameEvent::RequestCompanionUseSkill {
                    is_mercenary: false,
                    skill_id: skill.id,
                    level: skill.level,
                });
            } else if row_resp.clicked() {
                ui.drag_source(
                    HOMUN_SKILL_WINDOW_ID,
                    skill.id as usize,
                    Some(skill.icon_path()),
                    (ICON_SIZE, ICON_SIZE),
                );
            }

            if row_resp.hovered() {
                draw_companion_skill_tooltip(
                    ui,
                    &self.tooltip_container,
                    data,
                    skill,
                    row_rect.x + row_rect.w + 4.0,
                    row_y,
                );
            }
        }

        // Footer.
        let footer_y = y + TITLE_H + list_h;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);
        ui.text(
            x + PAD,
            footer_y + 15.0,
            &format!("Skill Point: {}", homun.skill_points),
            tc,
        );

        let (cw, ch) = self.close_size;
        let (uw, uh) = self.use_size;
        let btn_y = footer_y + (FOOTER_H - ch) * 0.5;
        let close_footer = Rect::new(x + WIN_W - PAD - cw, btn_y, cw, ch);
        let use_rect = Rect::new(x + WIN_W - PAD - cw - 4.0 - uw, btn_y, uw, uh);
        if ui.button(USE_BTN_ID, use_rect, &USE_BTN, "use").clicked()
            && let Some(skill) = homun.skills.get(self.selected)
        {
            events.push(GameEvent::RequestCompanionUseSkill {
                is_mercenary: false,
                skill_id: skill.id,
                level: skill.level,
            });
        }
        if ui
            .button(FOOTER_CLOSE_BTN_ID, close_footer, &CLOSE_BTN, "close")
            .clicked()
        {
            self.visible = false;
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl InGameWindow for HomunSkillWindow {
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

impl Window for HomunSkillWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
        self.tooltip_container.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(USE_BTN.normal) {
            self.use_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(CLOSE_BTN.normal) {
            self.close_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(LEVELUP_BTN.normal) {
            self.levelup_btn_size = (w as f32, h as f32);
        }
        self.tooltip_container.set_texture_sizes(size_fn);
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
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
            LEVELUP_BTN.normal,
            LEVELUP_BTN.hover,
            LEVELUP_BTN.pressed,
        ];
        paths.extend(DialogContainer::grf_texture_paths());
        paths
    }
}

/// Renders a companion skill's tooltip (name, type, level, SP cost, description)
/// anchored at `(anchor_x, anchor_y)` into the frame's tooltip layer. Shared by
/// the homunculus and mercenary skill windows.
pub(crate) fn draw_companion_skill_tooltip(
    ui: &mut UiFrame,
    container: &DialogContainer,
    data: &DataTable,
    skill: &SkillInfo,
    anchor_x: f32,
    anchor_y: f32,
) {
    let display_name = data
        .skill_name
        .as_ref()
        .map(|t| t.get_display_name_or_internal(&skill.name))
        .unwrap_or_else(|| skill.name.clone());
    let mut lines = vec![display_name];

    let type_str = match skill.skill_target_type {
        SkillTargetType::Passive => "Passive",
        SkillTargetType::Target => "Target",
        SkillTargetType::Ground => "Ground",
        SkillTargetType::MySelf => "Self",
        SkillTargetType::Trap => "Trap",
        _ => "Support",
    };
    lines.push(format!("Type: {type_str}"));
    lines.push(format!("Lv: {}", skill.level));
    if skill.sp_cost > 0 {
        lines.push(format!("SP Cost: {}", skill.sp_cost));
    }
    if let Some(desc_lines) = data
        .skill_description
        .as_ref()
        .and_then(|t| t.get_description(&skill.name))
    {
        for line in desc_lines {
            lines.push(line.clone());
        }
    }

    let tooltip_text = lines.join("\n");
    let wrapped = draw::word_wrap(
        &tooltip_text,
        220.0,
        |t| ui.atlas.measure_text(&draw::strip_color_codes(t)),
        false,
    );

    let line_h = ui.atlas.line_height;
    let pad = 8.0;
    let text_h = wrapped.len() as f32 * line_h;
    let max_line_w = wrapped
        .iter()
        .map(|l| ui.atlas.measure_text(&draw::strip_color_codes(l)))
        .fold(0.0f32, f32::max);
    let box_w = max_line_w + pad * 2.0;
    let box_h = text_h + pad * 2.0;

    container.draw(
        &mut ui.tooltip_draw_calls,
        anchor_x,
        anchor_y,
        box_w,
        box_h,
        [1.0; 4],
    );

    let text_color = container.text_color();
    let mut text_y = anchor_y + pad + line_h;
    for line in &wrapped {
        let (v, i) =
            draw::colored_text_vertices(line, anchor_x + pad, text_y, text_color, ui.atlas);
        if !v.is_empty() {
            ui.tooltip_draw_calls.push(DrawCall {
                vertices: v,
                indices: i,
                texture: TextureRef::FontAtlas,
            });
        }
        text_y += line_h;
    }
}
