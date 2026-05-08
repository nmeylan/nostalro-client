use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_game::skill::SkillTargetType;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

use crate::helper::dialog_container::DialogContainer;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    TITLEBAR_TEX, FOOTER_TEX,
    draw_titlebar, draw_container, draw_footer, text_color,
};
use crate::{InGameWindow, Window};

// -- Widget IDs --
pub const SKILL_WINDOW_ID: WidgetId = WidgetId(1200);
const SKILL_CLOSE_BTN_ID: WidgetId = WidgetId(1201);
const SKILL_USE_BTN_ID: WidgetId = WidgetId(1202);
const SKILL_SCROLL_UP_ID: WidgetId = WidgetId(1203);
const SKILL_SCROLL_DOWN_ID: WidgetId = WidgetId(1204);
const SKILL_SCROLL_THUMB_ID: WidgetId = WidgetId(1205);
const SKILL_FOOTER_CLOSE_BTN_ID: WidgetId = WidgetId(1206);
const SKILL_ENTRY_BASE_ID: u32 = 1120;
const SKILL_LEVEL_DOWN_BASE_ID: u32 = 1130;
const SKILL_LEVEL_UP_BASE_ID: u32 = 1140;
const SKILL_LEVELUP_BASE_ID: u32 = 1160;

// -- Layout --
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 27.0;
const ROW_H: f32 = 36.0;
const ICON_SIZE: f32 = 24.0;
const WIN_W: f32 = 270.0;
const VISIBLE_ROWS: usize = 7;
const PAD_X: f32 = 6.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";
const ARW_LEFT_TEX: &str = "data/texture/유저인터페이스/basic_interface/arw_left.bmp";
const ARW_RIGHT_TEX: &str = "data/texture/유저인터페이스/basic_interface/arw_right.bmp";
const ARW_SIZE: f32 = 11.0;

const LEVELUP_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/skill_up_a.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/skill_up_b.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/skill_up_c.bmp",
};

const USE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_ok.bmp",
    hover: "data/texture/유저인터페이스/btn_ok_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_ok_b.bmp",
};

const CLOSE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_close.bmp",
    hover: "data/texture/유저인터페이스/btn_close_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_close_b.bmp",
};

pub struct SkillTreeWindow {
    pub has_grf_textures: bool,
    scroll_offset: usize,
    btn_size: (f32, f32),
    levelup_btn_size: (f32, f32),
    tooltip_container: DialogContainer,
}

impl Default for SkillTreeWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillTreeWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            scroll_offset: 0,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            levelup_btn_size: (16.0, 16.0),
            tooltip_container: DialogContainer::new(),
        }
    }
}

impl Window for SkillTreeWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }

    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
        self.tooltip_container.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(CLOSE_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(LEVELUP_BTN.normal) {
            self.levelup_btn_size = (w as f32, h as f32);
        }
        self.tooltip_container.set_texture_sizes(size_fn);
    }

    fn grf_texture_paths() -> Vec<&'static str>
    where
        Self: Sized,
    {
        let mut paths = vec![
            TITLEBAR_TEX, FOOTER_TEX, CLOSE_OFF_TEX, CLOSE_ON_TEX,
            ARW_LEFT_TEX, ARW_RIGHT_TEX,
            LEVELUP_BTN.normal, LEVELUP_BTN.hover, LEVELUP_BTN.pressed,
            USE_BTN.normal, USE_BTN.hover, USE_BTN.pressed,
            CLOSE_BTN.normal, CLOSE_BTN.hover, CLOSE_BTN.pressed,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths.extend(DialogContainer::grf_texture_paths());
        paths
    }
}

impl InGameWindow for SkillTreeWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        data: &DataTable,
    ) -> Vec<GameEvent> {
        if !character.skills.is_open() {
            return vec![];
        }

        let mut events = Vec::new();
        let has_grf = self.has_grf_textures;
        let tc = text_color(has_grf);

        let total_skills = character.skills.skills().len();

        // Window dimensions
        let content_h = VISIBLE_ROWS as f32 * ROW_H;
        let win_h = TITLE_H + content_h + FOOTER_H;

        let win_rect = ui.window_at(
            SKILL_WINDOW_ID,
            WIN_W,
            win_h,
            TITLE_H,
            400.0,
            100.0,
        );
        let x = win_rect.x;
        let y = win_rect.y;

        // Block clicks/scroll through window
        ui.interact(SKILL_WINDOW_ID, win_rect);

        // -- Titlebar --
        draw_titlebar(ui, x, y, WIN_W, TITLE_H, has_grf);
        ui.text(x + 20.0, y + 13.0, "Skill Tree", tc);

        // Close button (titlebar)
        let close_x = x + WIN_W - 17.0;
        let close_y = y + 3.0;
        let close_rect = Rect::new(close_x, close_y, 11.0, 11.0);
        let close_resp = ui.interact(SKILL_CLOSE_BTN_ID, close_rect);
        if has_grf {
            let tex = if close_resp.hovered() { CLOSE_ON_TEX } else { CLOSE_OFF_TEX };
            let (v, i) = draw::quad_vertices(close_x, close_y, 11.0, 11.0, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            let color = if close_resp.hovered() {
                [0.8, 0.2, 0.2, 1.0]
            } else {
                [0.5, 0.5, 0.6, 1.0]
            };
            let (v, i) = draw::quad_vertices(close_x, close_y, 11.0, 11.0, color);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
            ui.text(close_x + 2.0, close_y + 10.0, "x", [1.0; 4]);
        }
        if close_resp.clicked() {
            character.skills.close();
            return events;
        }

        // -- Content area --
        let content_y = y + TITLE_H;
        draw_container(ui, x, content_y, WIN_W, content_h, has_grf);

        // Scrollbar
        let max_scroll = total_skills.saturating_sub(VISIBLE_ROWS);
        let scroll_x = x + WIN_W - SCROLLBAR_W - 1.0;
        let content_area_rect = Rect::new(x, content_y, WIN_W, content_h);
        self.scroll_offset = scrollbar::scrollbar(
            ui,
            ScrollbarIds {
                up: SKILL_SCROLL_UP_ID,
                down: SKILL_SCROLL_DOWN_ID,
                thumb: SKILL_SCROLL_THUMB_ID,
            },
            self.scroll_offset,
            VISIBLE_ROWS,
            max_scroll,
            content_area_rect,
            scroll_x,
            content_y,
            content_h,
        );

        // -- Skill rows --
        let skill_area_w = WIN_W - SCROLLBAR_W - PAD_X * 2.0 - 1.0;

        // Collect visible skill IDs to avoid borrowing character.skills for the whole loop
        let visible_skill_ids: Vec<u16> = character.skills.skills()
            .iter()
            .skip(self.scroll_offset)
            .take(VISIBLE_ROWS)
            .map(|s| s.id)
            .collect();

        let mut level_changes: Vec<(u16, bool)> = Vec::new();

        for (vis_i, &skill_id) in visible_skill_ids.iter().enumerate() {
            let skill = character.skills.get_skill(skill_id).unwrap();
            let row_y = content_y + vis_i as f32 * ROW_H;
            let entry_id = WidgetId(SKILL_ENTRY_BASE_ID + vis_i as u32);

            // Row separator line
            if vis_i > 0 {
                let sep_color = if has_grf {
                    [0.8, 0.8, 0.8, 1.0]
                } else {
                    [0.3, 0.3, 0.4, 1.0]
                };
                let (v, idx) = draw::quad_vertices(x + PAD_X, row_y, skill_area_w, 1.0, sep_color);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
            }

            // Row hover
            let row_rect = Rect::new(x + PAD_X, row_y, skill_area_w, ROW_H);
            let row_resp = ui.interact(entry_id, row_rect);
            if row_resp.hovered() {
                let hover_bg = if has_grf {
                    [0.85, 0.85, 0.8, 0.5]
                } else {
                    [0.3, 0.3, 0.4, 0.3]
                };
                let (v, idx) = draw::quad_vertices(
                    x + PAD_X, row_y, skill_area_w, ROW_H, hover_bg,
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
            }

            // Skill icon
            let icon_x = x + PAD_X + 2.0;
            let icon_y = row_y + (ROW_H - ICON_SIZE) / 2.0;
            let icon_path = skill.icon_path();

            let icon_alpha = if skill.level > 0 { 1.0 } else { 0.5 };
            let icon_color = [icon_alpha; 4];
            let (v, idx) = draw::quad_vertices(icon_x, icon_y, ICON_SIZE, ICON_SIZE, icon_color);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::Named(icon_path),
            });

            // Skill name
            let display_name = data
                .skill_name
                .as_ref()
                .map(|t| t.get_display_name_or_internal(&skill.name))
                .unwrap_or_else(|| skill.name.clone());
            let name_x = icon_x + ICON_SIZE + 6.0;
            let name_y = row_y + 14.0;
            ui.text(name_x, name_y, &display_name, tc);

            // Level text with optional rank selection arrows
            let level_y = row_y + 28.0;
            let level_color = [tc[0] * 0.7, tc[1] * 0.7, tc[2] * 0.7, tc[3]];
            let is_level_selectable = skill.skill_target_type != SkillTargetType::Passive
                && skill.level > 0
                && data.skill_use_level.as_ref()
                    .is_some_and(|t| t.supports_level_select(&skill.name));

            if is_level_selectable {
                let selected = skill.use_level();
                let level_text = format!("Lv : {} / {}", selected, skill.level);

                let arw_y = level_y - ARW_SIZE + 2.0;
                let left_id = WidgetId(SKILL_LEVEL_DOWN_BASE_ID + vis_i as u32);
                let left_rect = Rect::new(name_x, arw_y, ARW_SIZE, ARW_SIZE);
                let left_resp = ui.interact(left_id, left_rect);
                if has_grf {
                    let (v, i) = draw::quad_vertices(name_x, arw_y, ARW_SIZE, ARW_SIZE, [1.0; 4]);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: i.to_vec(),
                        texture: TextureRef::Named(ARW_LEFT_TEX.to_string()),
                    });
                } else {
                    let c = if left_resp.hovered() { [1.0, 1.0, 0.5, 1.0] } else { level_color };
                    ui.text(name_x, level_y, "<", c);
                }
                if left_resp.clicked() {
                    level_changes.push((skill_id, false));
                }

                let text_x = name_x + ARW_SIZE + 2.0;
                ui.text(text_x, level_y, &level_text, level_color);

                let text_w = ui.atlas.measure_text(&level_text);
                let right_x = text_x + text_w + 2.0;
                let right_id = WidgetId(SKILL_LEVEL_UP_BASE_ID + vis_i as u32);
                let right_rect = Rect::new(right_x, arw_y, ARW_SIZE, ARW_SIZE);
                let right_resp = ui.interact(right_id, right_rect);
                if has_grf {
                    let (v, i) = draw::quad_vertices(right_x, arw_y, ARW_SIZE, ARW_SIZE, [1.0; 4]);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: i.to_vec(),
                        texture: TextureRef::Named(ARW_RIGHT_TEX.to_string()),
                    });
                } else {
                    let c = if right_resp.hovered() { [1.0, 1.0, 0.5, 1.0] } else { level_color };
                    ui.text(right_x, level_y, ">", c);
                }
                if right_resp.clicked() {
                    level_changes.push((skill_id, true));
                }
            } else {
                let level_text = format!("Lv : {}", skill.level);
                ui.text(name_x, level_y, &level_text, level_color);
            }

            // Type text (right-aligned): "Passive" or "Sp : XX"
            let type_x = x + WIN_W - SCROLLBAR_W - PAD_X - 50.0;
            let type_y = row_y + ROW_H / 2.0 + 4.0;
            if skill.skill_target_type == SkillTargetType::Passive {
                ui.text(type_x, type_y, "Passive", tc);
            } else if is_level_selectable {
                let sp = data.skill_use_level.as_ref()
                    .and_then(|t| t.sp_at_level(&skill.name, skill.use_level()))
                    .unwrap_or(skill.sp_cost);
                let type_text = format!("Sp : {}", sp);
                ui.text(type_x, type_y, &type_text, tc);
            } else {
                let type_text = format!("Sp : {}", skill.sp_cost);
                ui.text(type_x, type_y, &type_text, tc);
            }

            // Level up button (only when upgradable and has skill points)
            if skill.upgradable && character.skill_point > 0 {
                let (lup_w, lup_h) = self.levelup_btn_size;
                let btn_x = type_x - lup_w - 4.0;
                let btn_y = row_y + (ROW_H - lup_h) / 2.0;
                let btn_id = WidgetId(SKILL_LEVELUP_BASE_ID + vis_i as u32);
                let btn_rect = Rect::new(btn_x, btn_y, lup_w, lup_h);
                let btn_resp = ui.button(btn_id, btn_rect, &LEVELUP_BTN, "+");

                if btn_resp.clicked() {
                    events.push(GameEvent::RequestSkillLevelUp {
                        skill_id: skill.id,
                    });
                }
            }

            // Drag source for usable skills (non-passive, learned)
            if row_resp.clicked() && skill.level > 0 && skill.skill_target_type != SkillTargetType::Passive {
                ui.drag_source(
                    SKILL_WINDOW_ID,
                    skill.id as usize,
                    Some(skill.icon_path()),
                    (ICON_SIZE, ICON_SIZE),
                );
            }

            // Tooltip on hover
            if row_resp.hovered() {
                let mut tooltip_lines = vec![display_name.clone()];

                let type_str = match skill.skill_target_type {
                    SkillTargetType::Passive => "Passive",
                    SkillTargetType::Target => "Target",
                    SkillTargetType::Ground => "Ground",
                    SkillTargetType::MySelf => "Self",
                    SkillTargetType::Trap => "Trap",
                    _ => "Unknown",
                };
                tooltip_lines.push(format!("Type: {type_str}"));

                if skill.sp_cost > 0 {
                    tooltip_lines.push(format!("SP Cost: {}", skill.sp_cost));
                }

                if let Some(desc_lines) = data
                    .skill_description
                    .as_ref()
                    .and_then(|t| t.get_description(&skill.name))
                {
                    for line in desc_lines {
                        tooltip_lines.push(line.clone());
                    }
                }

                let tooltip_text = tooltip_lines.join("\n");
                let tooltip_max_w: f32 = 220.0;
                let wrapped = draw::word_wrap(&tooltip_text, tooltip_max_w, |t| {
                    ui.atlas.measure_text(&draw::strip_color_codes(t))
                }, false);

                let line_h = ui.atlas.line_height;
                let pad = 8.0;
                let text_h = wrapped.len() as f32 * line_h;
                let max_line_w = wrapped.iter()
                    .map(|l| ui.atlas.measure_text(&draw::strip_color_codes(l)))
                    .fold(0.0f32, f32::max);
                let box_w = max_line_w + pad * 2.0;
                let box_h = text_h + pad * 2.0;

                let tx = row_rect.x + row_rect.w + 4.0;
                let ty = row_y;

                self.tooltip_container.draw(
                    &mut ui.tooltip_draw_calls,
                    tx, ty, box_w, box_h,
                    [1.0; 4],
                );

                let text_color = self.tooltip_container.text_color();
                let mut text_y = ty + pad + line_h;
                for line in &wrapped {
                    let (v, i) = draw::colored_text_vertices(line, tx + pad, text_y, text_color, ui.atlas);
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
        }

        // Apply deferred level changes
        for (skill_id, is_increment) in level_changes {
            if let Some(skill) = character.skills.get_skill_mut(skill_id) {
                if is_increment {
                    skill.increment_use_level();
                } else {
                    skill.decrement_use_level();
                }
            }
        }

        // -- Footer --
        let footer_y = content_y + content_h;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, has_grf);

        // Skill points text
        let sp_text = format!("Skill Point : {}", character.skill_point);
        ui.text(x + PAD_X, footer_y + 17.0, &sp_text, tc);

        // Footer buttons (right-aligned)
        let (btn_w, btn_h) = self.btn_size;
        let btn_y = footer_y + (FOOTER_H - btn_h) / 2.0;

        let close_btn_x = x + WIN_W - btn_w - 8.0;
        let close_btn = ui.button(
            SKILL_FOOTER_CLOSE_BTN_ID,
            Rect::new(close_btn_x, btn_y, btn_w, btn_h),
            &CLOSE_BTN,
            "close",
        );
        if close_btn.clicked() {
            character.skills.close();
            return events;
        }

        // TODO see later if we implement this "confirm" allocation feature
        // let use_btn_x = close_btn_x - btn_w - 4.0;
        // let _use_btn = ui.button(
        //     SKILL_USE_BTN_ID,
        //     Rect::new(use_btn_x, btn_y, btn_w, btn_h),
        //     &USE_BTN,
        //     "use",
        // );

        events
    }
}
