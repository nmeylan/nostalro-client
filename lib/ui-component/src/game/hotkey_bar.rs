use super::homun_skill_window::HOMUN_SKILL_WINDOW_ID;
use super::inventory_window::INV_WINDOW_ID;
use super::mercenary_skill_window::MERCENARY_SKILL_WINDOW_ID;
use super::skill_tree_window::SKILL_WINDOW_ID;
use crate::game::equipment_window::EQ_WINDOW_ID;
use crate::helper::window_chrome::{draw_sys_button, text_color};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::{GameEvent, SkillInfo};
use ragnarok_game::hotkey::{HOTKEY_COLS, HOTKEY_ROWS, HotkeySlotContent};
use ragnarok_game::item::InventoryTab;
use ragnarok_game::skill_action::{SkillCaster, skill_caster};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const HOTKEY_BAR_WINDOW_ID: WidgetId = WidgetId(1300);
const SLOT_BASE_ID: u32 = 1310;
const CLOSE_BTN_ID: WidgetId = WidgetId(1350);
const RESIZE_ID: WidgetId = WidgetId(1351);

const BG_TEX: &str = ragnarok_resources::ui::basic::SHORTITEM_BG;
const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;
const CAT_PAW_TEX: &str = ragnarok_resources::ui::item::CAT_PAW_HAIRPIN;

const ICON_SIZE: f32 = 24.0;
const SLOT_PAD_X: f32 = 16.0;
const SLOT_PAD_Y: f32 = 5.0;
const SLOT_W: f32 = 32.0;
const SLOT_MARGIN: f32 = 2.0;
const ROW_H: f32 = 34.0;
const LABEL_W: f32 = 4.0;
const CLOSE_SIZE: f32 = 12.0;
const RESIZE_SIZE: f32 = 13.0;
const WIN_W: f32 = SLOT_MARGIN + (SLOT_W + SLOT_MARGIN) * HOTKEY_COLS as f32;

const ROW_KEYS: [[&str; 9]; 4] = [
    ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"],
    ["1", "2", "3", "4", "5", "6", "7", "8", "9"],
    ["Q", "W", "E", "R", "T", "Y", "U", "I", "O"],
    ["A", "S", "D", "F", "G", "H", "J", "K", "L"],
];

const ROW2_CHARS: [char; 9] = ['1', '2', '3', '4', '5', '6', '7', '8', '9'];
const ROW3_CHARS: [char; 9] = ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o'];
const ROW4_CHARS: [char; 9] = ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'];

pub struct HotkeyBarWindow {
    pub has_grf_textures: bool,
    pub chat_is_active: bool,
    /// Mercenary + homunculus skills, refreshed each frame, so companion skills
    /// dragged into a slot can resolve their icon and drop level. IDs are
    /// range-disjoint from player skills, so a flat list needs no source tag.
    pub companion_skills: Vec<SkillInfo>,
    pub top_margin: f32,
    bg_size: (f32, f32),
    close_size: (f32, f32),
    resize_start: Option<u8>,
}

impl Default for HotkeyBarWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyBarWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            chat_is_active: false,
            companion_skills: Vec::new(),
            top_margin: 0.0,
            bg_size: (0.0, 0.0),
            close_size: (0.0, 0.0),
            resize_start: None,
        }
    }

    fn slot_icon_path(&self, content: HotkeySlotContent, character: &Character) -> Option<String> {
        match content {
            HotkeySlotContent::Empty => None,
            HotkeySlotContent::Skill { skill_id, .. } => character
                .skills
                .get_skill(skill_id)
                .map(|s| s.icon_path())
                .or_else(|| {
                    self.companion_skills
                        .iter()
                        .find(|s| s.id == skill_id)
                        .map(companion_skill_icon_path)
                }),
            HotkeySlotContent::Item { inventory_index } => character
                .inventory
                .get_item(inventory_index)
                .and_then(|item| item.icon_path()),
        }
    }

    fn slot_count_text(&self, content: HotkeySlotContent, character: &Character) -> Option<String> {
        match content {
            HotkeySlotContent::Empty => None,
            HotkeySlotContent::Skill { level, .. } => {
                if level > 0 {
                    Some(format!("{level}"))
                } else {
                    None
                }
            }
            HotkeySlotContent::Item { inventory_index } => {
                let count: i16 = character
                    .inventory
                    .all_items()
                    .iter()
                    .filter(|i| i.index == inventory_index)
                    .map(|i| i.count)
                    .sum();
                if count > 0 {
                    Some(format!("{count}"))
                } else if count == 0 {
                    Some("0".to_string())
                } else {
                    None
                }
            }
        }
    }

    fn execute_slot(&self, index: usize, character: &Character, events: &mut Vec<GameEvent>) {
        let content = character.hotkeys.get_slot(index);
        match content {
            HotkeySlotContent::Empty => {}
            HotkeySlotContent::Skill { skill_id, level } => {
                // Always request the skill, even on cooldown: targeting skills
                // still enter cursor mode (the skill-level ring), and the cast
                // itself is gated when the packet would be sent. The caster is
                // decided from the skill id alone, so a hotkey restored at login
                // resolves correctly before any companion exists.
                match skill_caster(skill_id) {
                    SkillCaster::Mercenary => events.push(GameEvent::RequestCompanionUseSkill {
                        is_mercenary: true,
                        skill_id,
                        level,
                    }),
                    SkillCaster::Homunculus => events.push(GameEvent::RequestCompanionUseSkill {
                        is_mercenary: false,
                        skill_id,
                        level,
                    }),
                    SkillCaster::Player => {
                        events.push(GameEvent::RequestUseSkill { skill_id, level })
                    }
                }
            }
            HotkeySlotContent::Item { inventory_index } => {
                if let Some(item) = character.inventory.get_item(inventory_index) {
                    if item.is_equipment() {
                        events.push(GameEvent::RequestEquipItem {
                            index: inventory_index,
                            location: item.equip_location(),
                        });
                    } else {
                        events.push(GameEvent::RequestUseItem {
                            index: inventory_index,
                        });
                    }
                }
            }
        }
    }

    fn handle_drop(
        &self,
        source_id: WidgetId,
        item_index: usize,
        slot_index: usize,
        character: &mut Character,
        events: &mut Vec<GameEvent>,
    ) {
        if source_id == INV_WINDOW_ID || source_id == EQ_WINDOW_ID {
            if let Some(item) = character.inventory.get_item(item_index as u16) {
                if item.tab() == InventoryTab::Etc && !item.is_ammunition() {
                    return;
                }
                let inventory_index = item.index;
                let content = HotkeySlotContent::Item { inventory_index };
                character.hotkeys.set_slot(slot_index, content);
                events.push(GameEvent::RequestHotkeyChange {
                    index: slot_index as u16,
                    is_skill: false,
                    id: inventory_index as u32,
                    count: 0,
                });
            }
        } else if source_id == SKILL_WINDOW_ID {
            let skill_id = item_index as u16;
            if let Some(skill) = character.skills.get_skill(skill_id) {
                let level = skill.use_level();
                let content = HotkeySlotContent::Skill { skill_id, level };
                character.hotkeys.set_slot(slot_index, content);
                events.push(GameEvent::RequestHotkeyChange {
                    index: slot_index as u16,
                    is_skill: true,
                    id: skill_id as u32,
                    count: level,
                });
            }
        } else if source_id == MERCENARY_SKILL_WINDOW_ID || source_id == HOMUN_SKILL_WINDOW_ID {
            let skill_id = item_index as u16;
            if let Some(skill) = self.companion_skills.iter().find(|s| s.id == skill_id) {
                let level = skill.level;
                let content = HotkeySlotContent::Skill { skill_id, level };
                character.hotkeys.set_slot(slot_index, content);
                events.push(GameEvent::RequestHotkeyChange {
                    index: slot_index as u16,
                    is_skill: true,
                    id: skill_id as u32,
                    count: level,
                });
            }
        } else if source_id == HOTKEY_BAR_WINDOW_ID {
            let src_content = character.hotkeys.get_slot(item_index);
            let dst_content = character.hotkeys.get_slot(slot_index);
            character.hotkeys.set_slot(slot_index, src_content);
            character.hotkeys.set_slot(item_index, dst_content);
            let (is_skill, id, count) = character.hotkeys.to_server_format(slot_index);
            events.push(GameEvent::RequestHotkeyChange {
                index: slot_index as u16,
                is_skill: is_skill != 0,
                id,
                count,
            });
            let (is_skill, id, count) = character.hotkeys.to_server_format(item_index);
            events.push(GameEvent::RequestHotkeyChange {
                index: item_index as u16,
                is_skill: is_skill != 0,
                id,
                count,
            });
        }
    }
}

impl Window for HotkeyBarWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }

    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some(size) = size_fn(BG_TEX) {
            self.bg_size = (size.0 as f32, size.1 as f32);
        }
        if let Some(size) = size_fn(CLOSE_OFF_TEX) {
            self.close_size = (size.0 as f32, size.1 as f32);
        }
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![BG_TEX, CLOSE_OFF_TEX, CLOSE_ON_TEX, CAT_PAW_TEX]
    }
}

impl InGameWindow for HotkeyBarWindow {
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let character = &mut *ctx.character;
        let data = ctx.data;
        let mut events = Vec::new();

        if ui.ctx.key_f12 {
            if character.hotkeys.visible_rows() == 0 {
                character.hotkeys.set_visible_rows(1);
            } else {
                character.hotkeys.cycle_visibility();
            }
        }

        let visible_rows = character.hotkeys.visible_rows() as usize;
        if visible_rows == 0 {
            return events;
        }

        let win_h = visible_rows as f32 * ROW_H;
        let default_x = (ui.ctx.screen_width - WIN_W) / 2.0;
        let default_y = self.top_margin;
        let win = ui.window_at(
            HOTKEY_BAR_WINDOW_ID,
            WIN_W,
            win_h,
            win_h,
            default_x,
            default_y,
        );

        let has_grf = self.has_grf_textures;

        if has_grf && self.bg_size.0 > 0.0 {
            for row in 0..visible_rows {
                let row_y = win.y + row as f32 * ROW_H;
                let (v, idx) = draw::quad_vertices(win.x, row_y, WIN_W, ROW_H, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::Named(BG_TEX.to_string()),
                });
            }
        } else {
            crate::helper::fallback::panel(ui, win.x, win.y, WIN_W, win_h);
        }

        if has_grf {
            let border_color = [0.3, 0.25, 0.2, 1.0];
            for &(bx, by, bw, bh) in &[
                (win.x, win.y, WIN_W, 1.0),
                (win.x, win.y + win_h - 1.0, WIN_W, 1.0),
                (win.x, win.y, 1.0, win_h),
                (win.x + WIN_W - 1.0, win.y, 1.0, win_h),
            ] {
                let (v, idx) = draw::quad_vertices(bx, by, bw, bh, border_color);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
            }
        }

        let close_size = if has_grf {
            self.close_size.1.max(CLOSE_SIZE)
        } else {
            CLOSE_SIZE
        };
        let close_rect = Rect::new(
            win.x + WIN_W - close_size - 2.0,
            win.y + SLOT_MARGIN,
            close_size,
            close_size,
        );
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        draw_sys_button(
            ui,
            close_rect,
            (close_size, close_size),
            close_resp.hovered(),
            has_grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );
        if close_resp.clicked() {
            character.hotkeys.set_visible_rows(0);
            return events;
        }

        let resize_rect = Rect::new(
            win.x + WIN_W - RESIZE_SIZE,
            win.y + win_h - RESIZE_SIZE,
            RESIZE_SIZE,
            RESIZE_SIZE,
        );
        let resize = ui.resize_handle(RESIZE_ID, resize_rect);
        if resize.started {
            self.resize_start = Some(visible_rows as u8);
            ui.cancel_window_drag(HOTKEY_BAR_WINDOW_ID);
        }
        if resize.dragging
            && let Some(start_rows) = self.resize_start
        {
            let new_rows = (start_rows as f32 + resize.delta_y / ROW_H).round() as i32;
            let new_rows = new_rows.clamp(1, HOTKEY_ROWS as i32) as u8;
            if new_rows != visible_rows as u8 {
                character.hotkeys.set_visible_rows(new_rows);
            }
        }

        let tc = text_color(has_grf);

        for row in 0..visible_rows {
            let row_y = win.y + row as f32 * ROW_H;

            if row > 0 {
                let sep_color = if has_grf {
                    [0.6, 0.55, 0.5, 0.5]
                } else {
                    [0.3, 0.3, 0.4, 0.5]
                };
                let (v, idx) = draw::quad_vertices(win.x + 1.0, row_y, WIN_W - 2.0, 1.0, sep_color);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
            }

            for col in 0..HOTKEY_COLS {
                let slot_index = row * HOTKEY_COLS + col;
                let slot_id = WidgetId(SLOT_BASE_ID + slot_index as u32);
                let content = character.hotkeys.get_slot(slot_index);

                let cell_x = win.x + SLOT_MARGIN + SLOT_MARGIN + col as f32 * (SLOT_W);
                let cell_y = row_y + SLOT_PAD_Y;
                let cell_rect = Rect::new(
                    cell_x,
                    cell_y,
                    SLOT_W - 2.0 * SLOT_MARGIN,
                    SLOT_W - SLOT_MARGIN * 2.0,
                );

                let resp = ui.interact(slot_id, cell_rect);

                if resp.hovered() {
                    let hover_color = [0.71, 1.0, 0.71, 1.0];
                    let (v, idx) = draw::quad_vertices(
                        cell_rect.x + 1.0,
                        cell_rect.y,
                        cell_rect.w - 1.0,
                        cell_rect.h - SLOT_MARGIN * 2.0,
                        hover_color,
                    );
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::White,
                    });
                }

                let label_color = [tc[0] * 0.6, tc[1] * 0.6, tc[2] * 0.6, tc[3]];
                if let Some(icon_path) = self.slot_icon_path(content, character) {
                    let (v, idx) = draw::quad_vertices(
                        cell_rect.x + (SLOT_W - ICON_SIZE) / 2.0 - SLOT_MARGIN,
                        cell_rect.y,
                        ICON_SIZE,
                        ICON_SIZE,
                        [1.0; 4],
                    );
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::Named(icon_path.clone()),
                    });

                    if let Some(count_text) = self.slot_count_text(content, character) {
                        let text_w = ui.atlas.measure_text(&count_text);
                        let tx = cell_rect.x + ICON_SIZE - text_w;
                        let ty = cell_y + ICON_SIZE + 2.0;
                        ui.text(tx, ty, &count_text, label_color);
                    }

                    if let HotkeySlotContent::Skill { skill_id, .. } = content
                        && character
                            .cooldowns
                            .is_on_cooldown(skill_id, ui.elapsed_secs)
                    {
                        let icon_x = cell_rect.x + (SLOT_W - ICON_SIZE) / 2.0 - SLOT_MARGIN;
                        let (v, idx) = draw::quad_vertices(
                            icon_x,
                            cell_rect.y,
                            ICON_SIZE,
                            ICON_SIZE,
                            [0.0, 0.0, 0.0, 0.45],
                        );
                        ui.draw_calls.push(DrawCall {
                            vertices: v.to_vec(),
                            indices: idx.to_vec(),
                            texture: TextureRef::White,
                        });
                        if has_grf {
                            let (v, idx) = draw::quad_vertices(
                                icon_x,
                                cell_rect.y,
                                ICON_SIZE,
                                ICON_SIZE,
                                [1.0; 4],
                            );
                            ui.draw_calls.push(DrawCall {
                                vertices: v.to_vec(),
                                indices: idx.to_vec(),
                                texture: TextureRef::Named(CAT_PAW_TEX.to_string()),
                            });
                        }
                        let remaining = character
                            .cooldowns
                            .remaining_secs(skill_id, ui.elapsed_secs);
                        if remaining > 0.1 {
                            let time_text = if remaining >= 1.0 {
                                format!("{:.0}", remaining)
                            } else {
                                format!("{:.1}", remaining)
                            };
                            let text_w = ui.atlas.measure_text(&time_text);
                            let tx = icon_x + (ICON_SIZE - text_w) / 2.0;
                            let ty = cell_rect.y + ICON_SIZE / 2.0 + 4.0;
                            ui.text(tx, ty, &time_text, [1.0, 1.0, 1.0, 1.0]);
                        }
                    }

                    if resp.double_clicked() {
                        self.execute_slot(slot_index, character, &mut events);
                    } else if resp.clicked() {
                        ui.drag_source(
                            HOTKEY_BAR_WINDOW_ID,
                            slot_index,
                            Some(icon_path),
                            (ICON_SIZE, ICON_SIZE),
                        );
                        ui.cancel_window_drag(HOTKEY_BAR_WINDOW_ID);
                    }
                }

                if let Some((source_id, source_item_index)) = ui.drop_zone(cell_rect) {
                    self.handle_drop(
                        source_id,
                        source_item_index,
                        slot_index,
                        character,
                        &mut events,
                    );
                }

                if resp.hovered() {
                    let tooltip = match content {
                        HotkeySlotContent::Skill { skill_id, level } => character
                            .skills
                            .get_skill(skill_id)
                            .map(|s| s.name.clone())
                            .or_else(|| {
                                self.companion_skills
                                    .iter()
                                    .find(|s| s.id == skill_id)
                                    .map(|s| s.name.clone())
                            })
                            .map(|name| format!("{name} Lv.{level}")),
                        HotkeySlotContent::Item { inventory_index } => {
                            let slot_count_table = data.item_slot_count.as_ref();
                            let card_name_table = data.card_name.as_ref();
                            character.inventory.get_item(inventory_index).map(|item| {
                                format_equipment_display_name(
                                    item,
                                    slot_count_table,
                                    card_name_table,
                                )
                            })
                        }
                        HotkeySlotContent::Empty => None,
                    };
                    if let Some(text) = tooltip {
                        ui.tooltip(cell_x, cell_y - 4.0, &text);
                    }
                }
            }
        }

        let f_keys = [
            ui.ctx.key_f1,
            ui.ctx.key_f2,
            ui.ctx.key_f3,
            ui.ctx.key_f4,
            ui.ctx.key_f5,
            ui.ctx.key_f6,
            ui.ctx.key_f7,
            ui.ctx.key_f8,
            ui.ctx.key_f9,
        ];
        for (i, &pressed) in f_keys.iter().enumerate() {
            if pressed {
                self.execute_slot(i, character, &mut events);
            }
        }

        if character.hotkeys.battle_mode() && !self.chat_is_active {
            for ch in &ui.ctx.typed_chars {
                let lower = ch.to_ascii_lowercase();
                if let Some(col) = ROW2_CHARS.iter().position(|&c| c == lower) {
                    if visible_rows > 1 {
                        self.execute_slot(HOTKEY_COLS + col, character, &mut events);
                    }
                } else if let Some(col) = ROW3_CHARS.iter().position(|&c| c == lower) {
                    if visible_rows > 2 {
                        self.execute_slot(HOTKEY_COLS * 2 + col, character, &mut events);
                    }
                } else if let Some(col) = ROW4_CHARS.iter().position(|&c| c == lower)
                    && visible_rows > 3
                {
                    self.execute_slot(HOTKEY_COLS * 3 + col, character, &mut events);
                }
            }
        }

        events
    }
}

fn companion_skill_icon_path(skill: &SkillInfo) -> String {
    ragnarok_resources::ui::item::icon(&skill.name.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::character::Character;
    use ragnarok_game::skill::SkillTargetType;

    fn merc_skill(id: u16, name: &str, level: i16) -> SkillInfo {
        SkillInfo {
            id,
            name: name.to_string(),
            level,
            sp_cost: 12,
            attack_range: 9,
            upgradable: false,
            skill_target_type: SkillTargetType::Target,
        }
    }

    #[test]
    fn dropping_mercenary_skill_assigns_and_persists_slot() {
        let mut bar = HotkeyBarWindow::new();
        bar.companion_skills = vec![merc_skill(8201, "MS_BASH", 5)];
        let mut character = Character::new();
        let mut events = Vec::new();

        bar.handle_drop(
            MERCENARY_SKILL_WINDOW_ID,
            8201,
            3,
            &mut character,
            &mut events,
        );

        assert_eq!(
            character.hotkeys.get_slot(3),
            HotkeySlotContent::Skill {
                skill_id: 8201,
                level: 5,
            }
        );
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestHotkeyChange {
                index: 3,
                is_skill: true,
                id: 8201,
                count: 5,
            }]
        ));
    }

    #[test]
    fn executing_a_companion_skill_hotkey_commands_the_companion() {
        // Empty companion list: the caster is resolved from the skill id alone,
        // as it must be for a hotkey restored at login before a companion exists.
        let bar = HotkeyBarWindow::new();
        let mut character = Character::new();
        character.hotkeys.set_slot(
            0,
            HotkeySlotContent::Skill {
                skill_id: 8201,
                level: 5,
            },
        );
        character.hotkeys.set_slot(
            1,
            HotkeySlotContent::Skill {
                skill_id: 5,
                level: 1,
            },
        );

        let mut events = Vec::new();
        bar.execute_slot(0, &character, &mut events);
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestCompanionUseSkill {
                is_mercenary: true,
                skill_id: 8201,
                ..
            }]
        ));

        // A player skill on a hotkey still casts from the main character.
        let mut events = Vec::new();
        bar.execute_slot(1, &character, &mut events);
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestUseSkill { skill_id: 5, .. }]
        ));
    }
}
