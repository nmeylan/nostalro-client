use super::homun_skill_window::{HOMUN_SKILL_WINDOW_ID, draw_companion_skill_tooltip};
use super::inventory_window::INV_WINDOW_ID;
use super::mercenary_skill_window::MERCENARY_SKILL_WINDOW_ID;
use super::skill_tree_window::{SKILL_WINDOW_ID, draw_skill_tooltip};
use crate::game::equipment_window::EQ_WINDOW_ID;
use crate::helper::colors;
use crate::helper::dialog_container::DialogContainer;
use crate::helper::window_chrome::{draw_sys_button, text_color};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::skill_name_table::format_skill_display_name;
use ragnarok_game::data_table::skill_use_level_table::SkillUseLevelTable;
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::{GameEvent, SkillInfo};
use ragnarok_game::hotkey::{HOTKEY_COLS, HOTKEY_ROWS, HotkeySlotContent};
use ragnarok_game::item::InventoryTab;
use ragnarok_game::skill::{SkillEnum, skill_icon_path};
use ragnarok_game::skill_action::{SkillCaster, skill_caster};
use ragnarok_ui::context::PhysicalKeyCode as KeyCode;
use ragnarok_ui::draw::{self, DrawCall, TextOutline, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId, WindowOrder};
use ragnarok_ui::rect::Rect;

pub const HOTKEY_BAR_WINDOW_ID: WidgetId = WidgetId(1300);
const SLOT_BASE_ID: u32 = 1310;
const CLOSE_BTN_ID: WidgetId = WidgetId(1350);
const RESIZE_ID: WidgetId = WidgetId(1351);
const DESCRIBE_ID: WidgetId = WidgetId(1352);
const DESCRIBE_CLOSE_ID: WidgetId = WidgetId(1353);

const BG_TEX: &str = ragnarok_resources::ui::basic::SHORTITEM_BG;
const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;
const CAT_PAW_TEX: &str = ragnarok_resources::ui::item::CAT_PAW_HAIRPIN;

const COUNT_OUTLINE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
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

/// Physical keys, not the characters they print: a slot keeps its place on the
/// keyboard on every layout.
const ROW1_KEYS: [KeyCode; 9] = [
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
];
const ROW2_KEYS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];
const ROW3_KEYS: [KeyCode; 9] = [
    KeyCode::KeyQ,
    KeyCode::KeyW,
    KeyCode::KeyE,
    KeyCode::KeyR,
    KeyCode::KeyT,
    KeyCode::KeyY,
    KeyCode::KeyU,
    KeyCode::KeyI,
    KeyCode::KeyO,
];
const ROW4_KEYS: [KeyCode; 9] = [
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
];

fn slot_key(slot: usize) -> Option<KeyCode> {
    let row = match slot / HOTKEY_COLS {
        0 => &ROW1_KEYS,
        1 => &ROW2_KEYS,
        2 => &ROW3_KEYS,
        3 => &ROW4_KEYS,
        _ => return None,
    };
    row.get(slot % HOTKEY_COLS).copied()
}

/// Passive skills cost no SP, so they never fit.
fn fits_hotkey_bar(level: i16, sp_cost: i16) -> bool {
    level > 0 && sp_cost > 0
}

/// The key is shown for rows 2-4 only in battle mode: outside it, they trigger
/// nothing.
fn slot_tooltip(key: Option<String>, body: Option<String>) -> Option<String> {
    match (key, body) {
        (Some(key), Some(body)) => Some(format!("[{key}] {body}")),
        (Some(key), None) => Some(format!("[{key}]")),
        (None, body) => body,
    }
}

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
    described_skill: Option<(SkillEnum, usize)>,
    tooltip_container: DialogContainer,
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
            described_skill: None,
            tooltip_container: DialogContainer::new(),
        }
    }

    fn slot_icon_path(&self, content: HotkeySlotContent, character: &Character) -> Option<String> {
        match content {
            HotkeySlotContent::Empty => None,
            HotkeySlotContent::Skill { skill, .. } => self
                .learned_skill(skill, character)
                .map(|_| skill_icon_path(skill)),
            HotkeySlotContent::Item { item_id } => character
                .inventory
                .find_by_item_id(item_id)
                .and_then(|item| item.icon_path()),
        }
    }

    /// Learned level and SP cost of a player or companion skill; `None` when it
    /// cannot sit on the bar.
    fn learned_skill(&self, skill: SkillEnum, character: &Character) -> Option<(i16, i16)> {
        character
            .skills
            .get_skill(skill)
            .map(|s| (s.level, s.sp_cost))
            .or_else(|| {
                self.companion_skills
                    .iter()
                    .find(|s| s.skill == skill)
                    .map(|s| (s.level, s.sp_cost))
            })
            .filter(|&(level, sp_cost)| fits_hotkey_bar(level, sp_cost))
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
            HotkeySlotContent::Item { item_id } => character
                .inventory
                .find_by_item_id(item_id)
                .map(|item| item.count)
                .filter(|count| *count > 0)
                .map(|count| format!("{count}")),
        }
    }

    fn execute_slot(
        &self,
        index: usize,
        character: &Character,
        use_levels: Option<&SkillUseLevelTable>,
        events: &mut Vec<GameEvent>,
    ) {
        let content = character.hotkeys.get_slot(index);
        match content {
            HotkeySlotContent::Empty => {}
            HotkeySlotContent::Skill { skill, level } => {
                let Some((learned, _)) = self.learned_skill(skill, character) else {
                    return;
                };
                let level = if use_levels.is_some_and(|t| t.supports_level_select(skill)) {
                    level
                } else {
                    learned
                };
                // Always request the skill, even on cooldown: targeting skills
                // still enter cursor mode (the skill-level ring), and the cast
                // itself is gated when the packet would be sent.
                match skill_caster(skill) {
                    SkillCaster::Mercenary => events.push(GameEvent::RequestCompanionUseSkill {
                        is_mercenary: true,
                        skill,
                        level,
                    }),
                    SkillCaster::Homunculus => events.push(GameEvent::RequestCompanionUseSkill {
                        is_mercenary: false,
                        skill,
                        level,
                    }),
                    SkillCaster::Player => events.push(GameEvent::RequestUseSkill { skill, level }),
                }
            }
            HotkeySlotContent::Item { item_id } => {
                if let Some(item) = character.inventory.find_by_item_id(item_id) {
                    if item.is_equipment() {
                        events.push(GameEvent::RequestEquipItem {
                            index: item.index,
                            location: item.equip_location(),
                        });
                    } else {
                        events.push(GameEvent::RequestUseItem { index: item.index });
                    }
                }
            }
        }
    }

    fn describe_close_clicked(
        &self,
        ui: &mut UiFrame,
        rect: Rect,
        has_grf: bool,
        close_size: f32,
    ) -> bool {
        ui.begin_popup_layer(rect);
        ui.interact(DESCRIBE_ID, rect);
        let close_rect = Rect::new(
            rect.x + rect.w - close_size - 3.0,
            rect.y + 3.0,
            close_size,
            close_size,
        );
        let close_resp = ui.interact(DESCRIBE_CLOSE_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
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
        ui.end_popup_layer();
        close_resp.clicked()
    }

    fn handle_drop(
        &self,
        source_id: WidgetId,
        item_index: usize,
        slot_index: usize,
        character: &mut Character,
        use_levels: Option<&SkillUseLevelTable>,
        events: &mut Vec<GameEvent>,
    ) {
        let saved_level = |skill: SkillEnum, level: i16| {
            if use_levels.is_some_and(|t| t.supports_level_select(skill)) {
                level
            } else {
                0
            }
        };
        if source_id == INV_WINDOW_ID || source_id == EQ_WINDOW_ID {
            if let Some(item) = character.inventory.get_item(item_index as u16) {
                if item.tab() == InventoryTab::Etc && !item.is_ammunition() {
                    return;
                }
                let item_id = item.item_id;
                let content = HotkeySlotContent::Item { item_id };
                character.hotkeys.set_slot(slot_index, content);
                events.push(GameEvent::RequestHotkeyChange {
                    index: slot_index as u16,
                    is_skill: false,
                    id: item_id as u32,
                    count: 0,
                });
            }
        } else if source_id == SKILL_WINDOW_ID {
            let skill = SkillEnum::from_id(item_index as u32);
            if let Some(learned) = character
                .skills
                .get_skill(skill)
                .filter(|s| fits_hotkey_bar(s.level, s.sp_cost))
            {
                let level = saved_level(skill, learned.use_level());
                character
                    .hotkeys
                    .set_slot(slot_index, HotkeySlotContent::Skill { skill, level });
                events.push(GameEvent::RequestHotkeyChange {
                    index: slot_index as u16,
                    is_skill: true,
                    id: skill.id(),
                    count: level,
                });
            }
        } else if source_id == MERCENARY_SKILL_WINDOW_ID || source_id == HOMUN_SKILL_WINDOW_ID {
            let skill = SkillEnum::from_id(item_index as u32);
            if let Some(known) = self
                .companion_skills
                .iter()
                .find(|s| s.skill == skill && fits_hotkey_bar(s.level, s.sp_cost))
            {
                let level = saved_level(skill, known.level);
                character
                    .hotkeys
                    .set_slot(slot_index, HotkeySlotContent::Skill { skill, level });
                events.push(GameEvent::RequestHotkeyChange {
                    index: slot_index as u16,
                    is_skill: true,
                    id: skill.id(),
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
        self.tooltip_container.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        self.tooltip_container.set_texture_sizes(size_fn);
        if let Some(size) = size_fn(BG_TEX) {
            self.bg_size = (size.0 as f32, size.1 as f32);
        }
        if let Some(size) = size_fn(CLOSE_OFF_TEX) {
            self.close_size = (size.0 as f32, size.1 as f32);
        }
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![BG_TEX, CLOSE_OFF_TEX, CLOSE_ON_TEX, CAT_PAW_TEX];
        paths.extend(DialogContainer::grf_texture_paths());
        paths
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
        ui.ensure_in_z_order_with(HOTKEY_BAR_WINDOW_ID, WindowOrder::Foreground);
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
                        if matches!(content, HotkeySlotContent::Skill { .. }) {
                            ui.text_with_outline(
                                tx,
                                ty,
                                &count_text,
                                colors::BLACK,
                                &TextOutline::cross(COUNT_OUTLINE_COLOR),
                            );
                        } else {
                            ui.text(tx + 5.0, ty + 2.5, &count_text, label_color);
                        }
                    }

                    if let HotkeySlotContent::Skill { skill, .. } = content
                        && character.cooldowns.is_on_cooldown(skill, ui.elapsed_secs)
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
                        let remaining = character.cooldowns.remaining_secs(skill, ui.elapsed_secs);
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
                        self.execute_slot(
                            slot_index,
                            character,
                            data.skill_use_level.as_ref(),
                            &mut events,
                        );
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

                if resp.right_clicked() {
                    match content {
                        HotkeySlotContent::Skill { skill, .. } => {
                            self.described_skill = match self.described_skill {
                                Some((shown, _)) if shown == skill => None,
                                _ => Some((skill, slot_index)),
                            };
                        }
                        HotkeySlotContent::Item { item_id } => {
                            if let Some(item) = character.inventory.find_by_item_id(item_id) {
                                events.push(GameEvent::ShowItemInfo { index: item.index });
                            }
                        }
                        HotkeySlotContent::Empty => {}
                    }
                }

                if let Some((source_id, source_item_index)) = ui.drop_zone(cell_rect) {
                    self.handle_drop(
                        source_id,
                        source_item_index,
                        slot_index,
                        character,
                        data.skill_use_level.as_ref(),
                        &mut events,
                    );
                }

                if resp.hovered() {
                    let tooltip = match content {
                        HotkeySlotContent::Skill { skill, level } => {
                            self.learned_skill(skill, character).map(|(_, sp_cost)| {
                                let display =
                                    format_skill_display_name(&skill, data.skill_name.as_ref());
                                if level > 0 {
                                    let sp = data
                                        .skill_use_level
                                        .as_ref()
                                        .and_then(|t| t.sp_at_level(skill, level))
                                        .unwrap_or(0);
                                    format!("{display} Use Lv {level} (Sp : {sp})")
                                } else {
                                    format!("{display} (Sp : {sp_cost})")
                                }
                            })
                        }
                        HotkeySlotContent::Item { item_id } => {
                            let slot_count_table = data.item_slot_count.as_ref();
                            let card_name_table = data.card_name.as_ref();
                            let producers = &character.char_names;
                            character.inventory.find_by_item_id(item_id).map(|item| {
                                let name = format_equipment_display_name(
                                    item,
                                    slot_count_table,
                                    card_name_table,
                                    producers,
                                );
                                if item.count > 1 {
                                    format!("{}: {} ea.", name, item.count)
                                } else {
                                    name
                                }
                            })
                        }
                        HotkeySlotContent::Empty => None,
                    };
                    let key = slot_key(slot_index)
                        .filter(|_| row == 0 || character.hotkeys.battle_mode())
                        .map(|code| ui.ctx.key_labels.display(code));
                    if let Some(text) = slot_tooltip(key, tooltip) {
                        ui.tooltip(cell_x, cell_y - 4.0, &text);
                    }
                }
            }
        }

        if let Some((skill, slot_index)) = self.described_skill {
            let icon_x = win.x
                + SLOT_MARGIN * 2.0
                + (slot_index % HOTKEY_COLS) as f32 * SLOT_W
                + (SLOT_W - ICON_SIZE) / 2.0
                - SLOT_MARGIN;
            let icon_y = win.y + (slot_index / HOTKEY_COLS) as f32 * ROW_H + SLOT_PAD_Y;
            let anchor_x = icon_x + ICON_SIZE / 2.0;
            let anchor_y = icon_y + ICON_SIZE / 2.0;
            let first_tooltip_call = ui.tooltip_draw_calls.len();
            let tooltip_rect = if let Some(learned) = character.skills.get_skill(skill) {
                Some(draw_skill_tooltip(
                    ui,
                    &self.tooltip_container,
                    data,
                    learned,
                    anchor_x,
                    anchor_y,
                ))
            } else {
                self.companion_skills
                    .iter()
                    .find(|s| s.skill == skill)
                    .map(|learned| {
                        draw_companion_skill_tooltip(
                            ui,
                            &self.tooltip_container,
                            data,
                            learned,
                            anchor_x,
                            anchor_y,
                        )
                    })
            };
            let pinned_calls = ui.tooltip_draw_calls.split_off(first_tooltip_call);
            ui.draw_calls.extend(pinned_calls);
            match tooltip_rect {
                Some(rect) => {
                    if self.describe_close_clicked(ui, rect, has_grf, close_size) {
                        self.described_skill = None;
                    }
                }
                None => self.described_skill = None,
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
                self.execute_slot(i, character, data.skill_use_level.as_ref(), &mut events);
            }
        }

        let modified = ui.ctx.alt_pressed || ui.ctx.ctrl_pressed;
        if character.hotkeys.battle_mode() && !self.chat_is_active && !modified {
            for &code in &ui.ctx.pressed_codes {
                if let Some(col) = ROW2_KEYS.iter().position(|&c| c == code) {
                    if visible_rows > 1 {
                        self.execute_slot(
                            HOTKEY_COLS + col,
                            character,
                            data.skill_use_level.as_ref(),
                            &mut events,
                        );
                    }
                } else if let Some(col) = ROW3_KEYS.iter().position(|&c| c == code) {
                    if visible_rows > 2 {
                        self.execute_slot(
                            HOTKEY_COLS * 2 + col,
                            character,
                            data.skill_use_level.as_ref(),
                            &mut events,
                        );
                    }
                } else if let Some(col) = ROW4_KEYS.iter().position(|&c| c == code)
                    && visible_rows > 3
                {
                    self.execute_slot(
                        HOTKEY_COLS * 3 + col,
                        character,
                        data.skill_use_level.as_ref(),
                        &mut events,
                    );
                }
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::item::ItemType;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_game::item::Item;
    use ragnarok_game::skill::{SkillData, SkillTargetType};
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::key_layout::KeyLabels;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    fn potion(index: u16, count: i16) -> Item {
        Item {
            index,
            item_id: 501,
            item_type: ItemType::Healing,
            count,
            is_identified: true,
            is_damaged: false,
            refining_level: 0,
            slot: [0; 4],
            location: 0,
            wear_state: 0,
            name: "Red Potion".into(),
            resource_name: None,
        }
    }

    fn merc_skill(skill: SkillEnum, level: i16) -> SkillInfo {
        SkillInfo {
            skill,
            level,
            sp_cost: 12,
            attack_range: 9,
            upgradable: false,
            skill_target_type: SkillTargetType::Target,
        }
    }

    fn learn(character: &mut Character, skill: SkillEnum, level: i16, sp_cost: i16) {
        character.skills.add_skill(SkillData {
            skill,
            level,
            selected_level: level,
            sp_cost,
            attack_range: 9,
            upgradable: false,
            skill_target_type: SkillTargetType::Target,
        });
    }

    #[test]
    fn slot_labels_follow_the_resolved_layout() {
        let labels = KeyLabels::from_map(std::collections::HashMap::from([
            (KeyCode::KeyQ, "a".to_string()),
            (KeyCode::Digit1, "&".to_string()),
        ]));

        let row3 = slot_key(HOTKEY_COLS * 2).unwrap();
        assert_eq!(
            slot_tooltip(Some(labels.display(row3)), Some("Fire Bolt Lv.5".into())),
            Some("[A] Fire Bolt Lv.5".to_string())
        );

        let row2 = slot_key(HOTKEY_COLS).unwrap();
        assert_eq!(
            slot_tooltip(Some(labels.display(row2)), None),
            Some("[&]".to_string())
        );

        // Unresolved keys keep the US name of the position.
        let row1 = slot_key(0).unwrap();
        assert_eq!(labels.display(row1), "F1");
        assert_eq!(
            slot_tooltip(None, Some("Red Potion".into())),
            Some("Red Potion".to_string())
        );
    }

    #[test]
    fn battle_mode_rows_trigger_on_the_physical_key() {
        let mut bar = HotkeyBarWindow::new();
        let mut character = Character::new();
        character.inventory.add_item(potion(7, 3));
        character.hotkeys.set_visible_rows(3);
        character
            .hotkeys
            .set_slot(HOTKEY_COLS * 2, HotkeySlotContent::Item { item_id: 501 });
        character.hotkeys.toggle_battle_mode();

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.pressed_codes = vec![KeyCode::KeyQ];
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = bar.build(
            &mut ui,
            &mut crate::BuildCtx::test(&mut character, &DataTable::default()),
        );

        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestUseItem { index: 7 }]
        ));

        // Alt+Q belongs to the equipment window, not to the slot under Q.
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.pressed_codes = vec![KeyCode::KeyQ];
        ctx.alt_pressed = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = bar.build(
            &mut ui,
            &mut crate::BuildCtx::test(&mut character, &DataTable::default()),
        );
        assert!(events.is_empty());
    }

    #[test]
    fn right_clicking_a_slot_toggles_the_skill_tooltip_or_asks_for_item_info() {
        let mut bar = HotkeyBarWindow::new();
        let mut character = Character::new();
        character.inventory.add_item(potion(9, 3));
        learn(&mut character, SkillEnum::MgFirebolt, 5, 12);
        character.hotkeys.set_visible_rows(1);
        character.hotkeys.set_slot(
            0,
            HotkeySlotContent::Skill {
                skill: SkillEnum::MgFirebolt,
                level: 0,
            },
        );
        character
            .hotkeys
            .set_slot(1, HotkeySlotContent::Item { item_id: 501 });

        let mut state = StateCache::new();
        let mut frame = |right_click_col: Option<usize>, character: &mut Character| {
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.mouse_y = 590.0;
            if let Some(col) = right_click_col {
                ctx.mouse_x =
                    (800.0 - WIN_W) / 2.0 + 2.0 * SLOT_MARGIN + col as f32 * SLOT_W + 10.0;
                ctx.mouse_y = SLOT_PAD_Y + 10.0;
                ctx.mouse_right_clicked = true;
            }
            let mut ui = test_frame(&mut ctx, &mut state);
            let events = bar.build(
                &mut ui,
                &mut crate::BuildCtx::test(character, &DataTable::default()),
            );
            (
                events,
                ui.draw_calls.len(),
                ui.tooltip_draw_calls.is_empty(),
            )
        };

        let (_, closed_calls, _) = frame(None, &mut character);
        assert!(frame(Some(0), &mut character).0.is_empty());
        let (_, pinned_calls, no_overlay) = frame(None, &mut character);
        assert!(pinned_calls > closed_calls);
        assert!(no_overlay, "the pinned tooltip must stay under the cursor");
        frame(Some(0), &mut character);
        assert_eq!(frame(None, &mut character).1, closed_calls);

        assert!(matches!(
            frame(Some(1), &mut character).0.as_slice(),
            [GameEvent::ShowItemInfo { index: 9 }]
        ));
    }

    #[test]
    fn dropping_mercenary_skill_assigns_and_persists_slot() {
        let mut bar = HotkeyBarWindow::new();
        bar.companion_skills = vec![merc_skill(SkillEnum::MsBash, 5)];
        let mut character = Character::new();
        let mut events = Vec::new();

        bar.handle_drop(
            MERCENARY_SKILL_WINDOW_ID,
            SkillEnum::MsBash.id() as usize,
            3,
            &mut character,
            None,
            &mut events,
        );

        assert_eq!(
            character.hotkeys.get_slot(3),
            HotkeySlotContent::Skill {
                skill: SkillEnum::MsBash,
                level: 0,
            }
        );
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestHotkeyChange {
                index: 3,
                is_skill: true,
                id: 8201,
                count: 0,
            }]
        ));
    }

    #[test]
    fn a_fixed_level_skill_keeps_its_saved_level_but_casts_at_the_learned_one() {
        let bar = HotkeyBarWindow::new();
        let use_levels = SkillUseLevelTable::from_entries(std::collections::HashMap::from([(
            "SM_BASH".to_string(),
            vec![8; 10],
        )]));
        let mut character = Character::new();
        learn(&mut character, SkillEnum::AmAcidterror, 5, 15);
        learn(&mut character, SkillEnum::SmBash, 10, 8);
        character.hotkeys.set_from_server(&[
            (1, SkillEnum::AmAcidterror.id(), 1),
            (1, SkillEnum::SmBash.id(), 3),
        ]);

        assert_eq!(
            bar.slot_count_text(character.hotkeys.get_slot(0), &character),
            Some("1".to_string())
        );

        let mut events = Vec::new();
        bar.execute_slot(0, &character, Some(&use_levels), &mut events);
        bar.execute_slot(1, &character, Some(&use_levels), &mut events);
        assert!(matches!(
            events.as_slice(),
            [
                GameEvent::RequestUseSkill {
                    skill: SkillEnum::AmAcidterror,
                    level: 5,
                },
                GameEvent::RequestUseSkill {
                    skill: SkillEnum::SmBash,
                    level: 3,
                },
            ]
        ));

        let mut events = Vec::new();
        bar.handle_drop(
            SKILL_WINDOW_ID,
            SkillEnum::AmAcidterror.id() as usize,
            2,
            &mut character,
            Some(&use_levels),
            &mut events,
        );
        assert_eq!(
            bar.slot_count_text(character.hotkeys.get_slot(2), &character),
            None
        );
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestHotkeyChange {
                index: 2,
                count: 0,
                ..
            }]
        ));
    }

    #[test]
    fn an_unlearned_or_passive_skill_stays_saved_but_off_the_bar() {
        let bar = HotkeyBarWindow::new();
        let mut character = Character::new();
        learn(&mut character, SkillEnum::AmAcidterror, 0, 0);
        learn(&mut character, SkillEnum::SmSword, 10, 0);
        character
            .hotkeys
            .set_from_server(&[(1, SkillEnum::AmAcidterror.id(), 1)]);

        assert_eq!(
            bar.slot_icon_path(character.hotkeys.get_slot(0), &character),
            None
        );
        let mut events = Vec::new();
        bar.execute_slot(0, &character, None, &mut events);
        assert!(events.is_empty());

        bar.handle_drop(
            SKILL_WINDOW_ID,
            SkillEnum::SmSword.id() as usize,
            1,
            &mut character,
            None,
            &mut events,
        );
        assert!(events.is_empty());
        assert_eq!(character.hotkeys.get_slot(1), HotkeySlotContent::Empty);

        learn(&mut character, SkillEnum::AmAcidterror, 1, 15);
        assert!(
            bar.slot_icon_path(character.hotkeys.get_slot(0), &character)
                .is_some()
        );
        assert_eq!(
            bar.slot_count_text(character.hotkeys.get_slot(0), &character),
            Some("1".to_string())
        );
    }

    #[test]
    fn executing_a_companion_skill_hotkey_commands_the_companion() {
        let mut bar = HotkeyBarWindow::new();
        bar.companion_skills = vec![merc_skill(SkillEnum::MsBash, 5)];
        let mut character = Character::new();
        learn(&mut character, SkillEnum::SmBash, 10, 8);
        character.hotkeys.set_slot(
            0,
            HotkeySlotContent::Skill {
                skill: SkillEnum::MsBash,
                level: 5,
            },
        );
        character.hotkeys.set_slot(
            1,
            HotkeySlotContent::Skill {
                skill: SkillEnum::SmBash,
                level: 1,
            },
        );

        let mut events = Vec::new();
        bar.execute_slot(0, &character, None, &mut events);
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestCompanionUseSkill {
                is_mercenary: true,
                skill: SkillEnum::MsBash,
                ..
            }]
        ));

        // A player skill on a hotkey still casts from the main character.
        let mut events = Vec::new();
        bar.execute_slot(1, &character, None, &mut events);
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestUseSkill {
                skill: SkillEnum::SmBash,
                ..
            }]
        ));
    }

    #[test]
    fn an_item_hotkey_survives_the_server_reindexing_the_inventory() {
        let bar = HotkeyBarWindow::new();
        let mut character = Character::new();
        character.inventory.add_item(potion(12, 25));

        let mut events = Vec::new();
        bar.handle_drop(INV_WINDOW_ID, 12, 4, &mut character, None, &mut events);
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestHotkeyChange {
                index: 4,
                is_skill: false,
                id: 501,
                count: 0,
            }]
        ));

        let persisted = character.hotkeys.to_server_format(4);
        let mut relogged = Character::new();
        relogged.inventory.add_item(potion(3, 25));
        relogged
            .hotkeys
            .set_from_server(&[(0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), persisted]);

        let mut events = Vec::new();
        bar.execute_slot(4, &relogged, None, &mut events);
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestUseItem { index: 3 }]
        ));
        assert_eq!(
            bar.slot_count_text(relogged.hotkeys.get_slot(4), &relogged),
            Some("25".to_string())
        );

        // Nothing is drawn for an item the character no longer owns.
        let empty = Character::new();
        assert_eq!(
            bar.slot_count_text(HotkeySlotContent::Item { item_id: 501 }, &empty),
            None
        );
    }
}
