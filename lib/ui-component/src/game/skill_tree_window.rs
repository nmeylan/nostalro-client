use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_game::skill_tree_table::SkillTreeTable;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    TITLEBAR_TEX, FOOTER_TEX,
    draw_titlebar, draw_container, draw_footer, text_color,
};
use crate::{InGameWindow, Window};

// -- Widget IDs --
pub const SKILL_WINDOW_ID: WidgetId = WidgetId(1000);
const SKILL_CLOSE_BTN_ID: WidgetId = WidgetId(1001);
const SKILL_SCROLL_UP_ID: WidgetId = WidgetId(1003);
const SKILL_SCROLL_DOWN_ID: WidgetId = WidgetId(1004);
const SKILL_SCROLL_THUMB_ID: WidgetId = WidgetId(1005);
const SKILL_TAB_BASE_ID: u32 = 1010;
const SKILL_ENTRY_BASE_ID: u32 = 1020;
const SKILL_LEVELUP_BASE_ID: u32 = 1060;
const SKILL_APPLY_BTN_ID: WidgetId = WidgetId(1096);
const SKILL_CANCEL_BTN_ID: WidgetId = WidgetId(1097);

// -- Layout --
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 19.0;
const TAB_H: f32 = 20.0;
const HEADER_H: f32 = 18.0;
const ROW_H: f32 = 28.0;
const ICON_SIZE: f32 = 24.0;
const PLUS_BTN_W: f32 = 16.0;
const PLUS_BTN_H: f32 = 16.0;
const WIN_W: f32 = 280.0;
const VISIBLE_ROWS: usize = 8;
const PAD_X: f32 = 6.0;
const APPLY_BTN_W: f32 = 52.0;
const APPLY_BTN_H: f32 = 16.0;

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

struct PendingAlloc {
    skill_id: u16,
    skill_name: String,
}

pub struct SkillTreeWindow {
    pub has_grf_textures: bool,
    pub job_class: u16,
    scroll_offset: usize,
    active_tab: usize,
    pending_allocations: Vec<PendingAlloc>,
    pending_skill_point: u16,
    had_pending: bool,
}

impl SkillTreeWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            job_class: 0,
            scroll_offset: 0,
            active_tab: 0,
            pending_allocations: Vec::new(),
            pending_skill_point: 0,
            had_pending: false,
        }
    }

    pub fn is_open(&self) -> bool {
        false // managed by character.skills.is_open()
    }

    fn pending_count_for(&self, skill_name: &str) -> i16 {
        self.pending_allocations
            .iter()
            .filter(|a| a.skill_name == skill_name)
            .count() as i16
    }
}

impl Window for SkillTreeWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }

    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn grf_texture_paths() -> Vec<&'static str>
    where
        Self: Sized,
    {
        let mut paths = vec![TITLEBAR_TEX, FOOTER_TEX, CLOSE_OFF_TEX, CLOSE_ON_TEX];
        paths.extend(scrollbar::grf_texture_paths());
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
            self.pending_allocations.clear();
            self.had_pending = false;
            return vec![];
        }

        let mut events = Vec::new();
        let has_grf = self.has_grf_textures;
        let tc = text_color(has_grf);

        let tabs = SkillTreeTable::job_skill_tabs(self.job_class);
        if tabs.is_empty() {
            return vec![];
        }
        if self.active_tab >= tabs.len() {
            self.active_tab = tabs.len() - 1;
        }

        // Reset pending state when no pending allocs and skill points may have changed
        if self.pending_allocations.is_empty() {
            self.pending_skill_point = character.skill_point;
            self.had_pending = false;
        }

        // Get the current tab's skill tree entries
        let (current_job_id, _) = tabs[self.active_tab];
        let tree_entries = data
            .skill_tree
            .as_ref()
            .and_then(|t| t.get_tree(current_job_id));
        let total_skills = tree_entries.map(|t| t.len()).unwrap_or(0);

        // Window dimensions
        let content_h = VISIBLE_ROWS as f32 * ROW_H;
        let has_pending = !self.pending_allocations.is_empty();
        let footer_extra = if has_pending { APPLY_BTN_H + 4.0 } else { 0.0 };
        let win_h = TITLE_H + TAB_H + HEADER_H + content_h + footer_extra + FOOTER_H;

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

        // Draw window chrome
        draw_titlebar(ui, x, y, WIN_W, TITLE_H, has_grf);
        ui.text(x + 20.0, y + 13.0, "Skill", tc);

        // Close button
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
            self.pending_allocations.clear();
            self.had_pending = false;
            return events;
        }

        // Tab bar
        let tab_y = y + TITLE_H;
        let tab_w = (WIN_W / tabs.len() as f32).floor();
        for (i, (_, label)) in tabs.iter().enumerate() {
            let tx = x + i as f32 * tab_w;
            let tab_rect = Rect::new(tx, tab_y, tab_w, TAB_H);
            let tab_id = WidgetId(SKILL_TAB_BASE_ID + i as u32);
            let resp = ui.interact(tab_id, tab_rect);
            let is_active = i == self.active_tab;

            let bg = if is_active {
                if has_grf { [0.9, 0.9, 0.85, 1.0] } else { [0.25, 0.25, 0.35, 1.0] }
            } else if resp.hovered() {
                if has_grf { [0.85, 0.85, 0.8, 1.0] } else { [0.2, 0.2, 0.3, 1.0] }
            } else {
                if has_grf { [0.75, 0.75, 0.7, 1.0] } else { [0.15, 0.15, 0.22, 1.0] }
            };
            let (v, idx) = draw::quad_vertices(tx, tab_y, tab_w, TAB_H, bg);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::White,
            });
            ui.text(tx + 4.0, tab_y + 15.0, label, tc);

            if resp.clicked() && !is_active {
                self.active_tab = i;
                self.scroll_offset = 0;
            }
        }

        // Header: skill points
        let header_y = tab_y + TAB_H;
        draw_container(ui, x, header_y, WIN_W, HEADER_H, has_grf);
        let sp_text = format!("Skill Points: {}", self.pending_skill_point);
        ui.text(x + PAD_X, header_y + 14.0, &sp_text, tc);

        // Content area
        let content_y = header_y + HEADER_H;
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

        // Render skill rows
        if let Some(entries) = tree_entries {
            let skill_area_w = WIN_W - SCROLLBAR_W - PAD_X * 2.0 - 1.0;
            for (vis_i, entry) in entries
                .iter()
                .skip(self.scroll_offset)
                .take(VISIBLE_ROWS)
                .enumerate()
            {
                let row_y = content_y + vis_i as f32 * ROW_H;
                let entry_id = WidgetId(SKILL_ENTRY_BASE_ID + vis_i as u32);

                let known_skill = character.skills.get_skill_by_name(&entry.skill_name);
                let current_level = known_skill.map(|s| s.level).unwrap_or(0);
                let pending_extra = self.pending_count_for(&entry.skill_name);
                let effective_level = current_level + pending_extra;
                let skill_id = known_skill.map(|s| s.id).unwrap_or(0);

                // Check prerequisites
                let prereqs_met = entry.prerequisite_positions.iter().all(|&prereq_pos| {
                    entries.iter().any(|e| {
                        e.position == prereq_pos && {
                            let lvl = character
                                .skills
                                .get_skill_by_name(&e.skill_name)
                                .map(|s| s.level)
                                .unwrap_or(0)
                                + self.pending_count_for(&e.skill_name);
                            lvl > 0
                        }
                    })
                });

                let is_learned = effective_level > 0;
                let row_alpha = if is_learned || prereqs_met { 1.0 } else { 0.5 };

                // Row background on hover
                let row_rect = Rect::new(x + PAD_X, row_y, skill_area_w, ROW_H);
                let row_resp = ui.interact(entry_id, row_rect);
                if row_resp.hovered() {
                    let hover_bg = if has_grf {
                        [0.85, 0.85, 0.8, 0.5]
                    } else {
                        [0.3, 0.3, 0.4, 0.3]
                    };
                    let (v, idx) = draw::quad_vertices(
                        x + PAD_X,
                        row_y,
                        skill_area_w,
                        ROW_H,
                        hover_bg,
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
                let icon_path = format!(
                    "data/texture/유저인터페이스/item/{}.bmp",
                    entry.skill_name.to_lowercase()
                );
                let icon_color = [row_alpha, row_alpha, row_alpha, row_alpha];
                let (v, idx) =
                    draw::quad_vertices(icon_x, icon_y, ICON_SIZE, ICON_SIZE, icon_color);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::Named(icon_path),
                });

                // Skill name
                let display_name = data
                    .skill_name
                    .as_ref()
                    .map(|t| t.get_display_name_or_internal(&entry.skill_name))
                    .unwrap_or_else(|| entry.skill_name.clone());
                let name_x = icon_x + ICON_SIZE + 4.0;
                let name_y = row_y + 12.0;
                let name_color = [tc[0], tc[1], tc[2], row_alpha];
                ui.text(name_x, name_y, &display_name, name_color);

                // Level text
                let level_text = format!("Lv {}/{}", effective_level, entry.max_level);
                let level_x = icon_x + ICON_SIZE + 4.0;
                let level_y = row_y + 24.0;
                let level_color = if pending_extra > 0 {
                    [0.0, 0.5, 0.0, row_alpha]
                } else {
                    [tc[0] * 0.6, tc[1] * 0.6, tc[2] * 0.6, row_alpha]
                };
                ui.text(level_x, level_y, &level_text, level_color);

                // "+" button
                let can_upgrade = self.pending_skill_point > 0
                    && prereqs_met
                    && effective_level < entry.max_level as i16
                    && (known_skill.map(|s| s.upgradable).unwrap_or(true) || pending_extra > 0);

                if can_upgrade {
                    let btn_x = x + WIN_W - SCROLLBAR_W - PLUS_BTN_W - PAD_X - 2.0;
                    let btn_y = row_y + (ROW_H - PLUS_BTN_H) / 2.0;
                    let btn_rect = Rect::new(btn_x, btn_y, PLUS_BTN_W, PLUS_BTN_H);
                    let btn_id = WidgetId(SKILL_LEVELUP_BASE_ID + vis_i as u32);
                    let btn_resp = ui.interact(btn_id, btn_rect);

                    let btn_color = if btn_resp.hovered() {
                        [0.3, 0.6, 0.3, 1.0]
                    } else {
                        [0.2, 0.45, 0.2, 1.0]
                    };
                    let (v, idx) =
                        draw::quad_vertices(btn_x, btn_y, PLUS_BTN_W, PLUS_BTN_H, btn_color);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::White,
                    });
                    ui.text(
                        btn_x + 4.0,
                        btn_y + 12.0,
                        "+",
                        [1.0, 1.0, 1.0, 1.0],
                    );

                    if btn_resp.clicked() {
                        self.pending_allocations.push(PendingAlloc {
                            skill_id,
                            skill_name: entry.skill_name.clone(),
                        });
                        self.pending_skill_point -= 1;
                        self.had_pending = true;
                    }
                }

                // Tooltip on hover
                if row_resp.hovered() {
                    let mut tooltip_lines = Vec::new();
                    let name_for_tooltip = data
                        .skill_name
                        .as_ref()
                        .map(|t| t.get_display_name_or_internal(&entry.skill_name))
                        .unwrap_or_else(|| entry.skill_name.clone());
                    tooltip_lines.push(name_for_tooltip);

                    let type_str = match known_skill.map(|s| s.skill_type).unwrap_or(-1) {
                        0 => "Passive",
                        1 => "Offensive",
                        2 => "Supportive",
                        _ => "Unknown",
                    };
                    tooltip_lines.push(format!("Type: {type_str}"));

                    if let Some(s) = known_skill {
                        if s.sp_cost > 0 {
                            tooltip_lines.push(format!("SP Cost: {}", s.sp_cost));
                        }
                    }

                    if let Some(desc_lines) = data
                        .skill_description
                        .as_ref()
                        .and_then(|t| t.get_description(&entry.skill_name))
                    {
                        for line in desc_lines {
                            tooltip_lines.push(line.clone());
                        }
                    }

                    let tooltip_text = tooltip_lines.join("\n");
                    ui.tooltip(row_rect.x + row_rect.w, row_y, &tooltip_text);
                }
            }
        }

        // Footer
        let footer_y = content_y + content_h + footer_extra;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, has_grf);

        // Apply / Cancel buttons when pending
        if has_pending {
            let btn_area_y = content_y + content_h + 2.0;

            // Apply button
            let apply_x = x + WIN_W / 2.0 - APPLY_BTN_W - 4.0;
            let apply_rect = Rect::new(apply_x, btn_area_y, APPLY_BTN_W, APPLY_BTN_H);
            let apply_resp = ui.interact(SKILL_APPLY_BTN_ID, apply_rect);
            let apply_color = if apply_resp.hovered() {
                [0.25, 0.55, 0.25, 1.0]
            } else {
                [0.2, 0.4, 0.2, 1.0]
            };
            let (v, idx) = draw::quad_vertices(
                apply_x,
                btn_area_y,
                APPLY_BTN_W,
                APPLY_BTN_H,
                apply_color,
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::White,
            });
            ui.text(apply_x + 10.0, btn_area_y + 12.0, "Apply", [1.0; 4]);

            if apply_resp.clicked() {
                for alloc in self.pending_allocations.drain(..) {
                    events.push(GameEvent::RequestSkillLevelUp {
                        skill_id: alloc.skill_id,
                    });
                }
                self.had_pending = false;
            }

            // Cancel button
            let cancel_x = x + WIN_W / 2.0 + 4.0;
            let cancel_rect = Rect::new(cancel_x, btn_area_y, APPLY_BTN_W, APPLY_BTN_H);
            let cancel_resp = ui.interact(SKILL_CANCEL_BTN_ID, cancel_rect);
            let cancel_color = if cancel_resp.hovered() {
                [0.55, 0.25, 0.25, 1.0]
            } else {
                [0.4, 0.2, 0.2, 1.0]
            };
            let (v, idx) = draw::quad_vertices(
                cancel_x,
                btn_area_y,
                APPLY_BTN_W,
                APPLY_BTN_H,
                cancel_color,
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::White,
            });
            ui.text(cancel_x + 6.0, btn_area_y + 12.0, "Cancel", [1.0; 4]);

            if cancel_resp.clicked() {
                self.pending_allocations.clear();
                self.pending_skill_point = character.skill_point;
                self.had_pending = false;
            }
        }

        events
    }
}
