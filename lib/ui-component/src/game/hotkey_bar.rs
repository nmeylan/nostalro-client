use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::GameEvent;
use ragnarok_game::hotkey::{HOTKEY_COLS, HOTKEY_ROWS, HotkeySlotContent};
use ragnarok_game::item::InventoryTab;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use crate::{InGameWindow, Window};
use crate::helper::window_chrome::text_color;
use super::inventory_window::INV_WINDOW_ID;
use super::skill_tree_window::SKILL_WINDOW_ID;

pub const HOTKEY_BAR_WINDOW_ID: WidgetId = WidgetId(1300);
const SLOT_BASE_ID: u32 = 1310;
const CLOSE_BTN_ID: WidgetId = WidgetId(1350);
const RESIZE_ID: WidgetId = WidgetId(1351);

const BG_TEX: &str = "data/texture/유저인터페이스/basic_interface/shortitem_bg.bmp";
const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const ICON_SIZE: f32 = 24.0;
const SLOT_PAD_X: f32 = 16.0;
const SLOT_PAD_Y: f32 = 5.0;
const SLOT_W: f32 = 32.0;
const SLOT_MARGIN: f32 = 2.0;
const ROW_H: f32 = 34.0;
const LABEL_W: f32 = 4.0;
const CLOSE_SIZE: f32 = 12.0;
const RESIZE_SIZE: f32 = 13.0;
const WIN_W: f32 = SLOT_MARGIN + (SLOT_W + SLOT_MARGIN ) * HOTKEY_COLS as f32;

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
    bg_size: (f32, f32),
    close_size: (f32, f32),
    resize_start: Option<u8>,
}

impl HotkeyBarWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            chat_is_active: false,
            bg_size: (0.0, 0.0),
            close_size: (0.0, 0.0),
            resize_start: None,
        }
    }

    fn slot_icon_path(&self, content: HotkeySlotContent, character: &Character, data: &DataTable) -> Option<String> {
        match content {
            HotkeySlotContent::Empty => None,
            HotkeySlotContent::Skill { skill_id, .. } => {
                character.skills.get_skill(skill_id).map(|s| s.icon_path())
            }
            HotkeySlotContent::Item { item_id, inventory_index } => {
                character.inventory.get_item(inventory_index)
                    .and_then(|item| item.icon_path())
                    .or_else(|| data.item_resource.as_ref().and_then(|t| t.item_icon_path(item_id)))
            }
        }
    }

    fn slot_count_text(&self, content: HotkeySlotContent, character: &Character) -> Option<String> {
        match content {
            HotkeySlotContent::Empty => None,
            HotkeySlotContent::Skill { level, .. } => {
                if level > 0 { Some(format!("{level}")) } else { None }
            }
            HotkeySlotContent::Item { item_id, inventory_index } => {
                let count: i16 = character.inventory.all_items().iter()
                    .filter(|i| i.index == inventory_index)
                    .map(|i| i.count)
                    .sum();
                if count > 1 { Some(format!("{count}")) } else if count == 0 { Some("0".to_string()) } else { None }
            }
        }
    }

    fn execute_slot(&self, index: usize, character: &Character, events: &mut Vec<GameEvent>) {
        let content = character.hotkeys.get_slot(index);
        match content {
            HotkeySlotContent::Empty => {}
            HotkeySlotContent::Skill { skill_id, level } => {
                events.push(GameEvent::RequestUseSkill { skill_id, level });
            }
            HotkeySlotContent::Item { inventory_index, .. } => {
                if character.inventory.get_item(inventory_index).is_some() {
                    events.push(GameEvent::RequestUseItem { index: inventory_index });
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
        if source_id == INV_WINDOW_ID {
            if let Some(item) = character.inventory.get_item(item_index as u16) {
                if item.tab() == InventoryTab::Etc {
                    return;
                }
                let item_id = item.item_id;
                let inventory_index = item.index;
                let content = HotkeySlotContent::Item { item_id, inventory_index };
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
                id: id,
                count,
            });
            let (is_skill, id, count) = character.hotkeys.to_server_format(item_index);
            events.push(GameEvent::RequestHotkeyChange {
                index: item_index as u16,
                is_skill: is_skill != 0,
                id: id,
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
        vec![BG_TEX, CLOSE_OFF_TEX, CLOSE_ON_TEX]
    }
}

impl InGameWindow for HotkeyBarWindow {
    fn build(&mut self, ui: &mut UiFrame, character: &mut Character, data: &DataTable) -> Vec<GameEvent> {
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
        let default_y = 0.0;
        let win = ui.window_at(HOTKEY_BAR_WINDOW_ID, WIN_W, win_h, win_h, default_x, default_y);

        let has_grf = self.has_grf_textures;

        // Draw background
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
            let bg_color = [0.15, 0.12, 0.10, 0.9];
            let (v, idx) = draw::quad_vertices(win.x, win.y, WIN_W, win_h, bg_color);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::White,
            });
        }

        // Draw border
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

        // Close button (top-right)
        let close_size = if has_grf { self.close_size.1.max(CLOSE_SIZE) } else { CLOSE_SIZE };
        let close_rect = Rect::new(
            win.x + WIN_W - close_size - 2.0,
            win.y + SLOT_MARGIN,
            close_size,
            close_size,
        );
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if has_grf {
            let tex = if close_resp.hovered() { CLOSE_ON_TEX } else { CLOSE_OFF_TEX };
            let (v, idx) = draw::quad_vertices(close_rect.x, close_rect.y, close_size, close_size, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            let c = if close_resp.hovered() { [1.0, 0.3, 0.3, 1.0] } else { text_color(false) };
            ui.text(close_rect.x + 2.0, close_rect.y + close_size - 1.0, "x", c);
        }
        if close_resp.clicked() {
            character.hotkeys.set_visible_rows(0);
            return events;
        }

        // Resize handle (bottom-right)
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
        if resize.dragging {
            if let Some(start_rows) = self.resize_start {
                let new_rows = (start_rows as f32 + resize.delta_y / ROW_H).round() as i32;
                let new_rows = new_rows.clamp(1, HOTKEY_ROWS as i32) as u8;
                if new_rows != visible_rows as u8 {
                    character.hotkeys.set_visible_rows(new_rows);
                }
            }
        }

        let tc = text_color(has_grf);

        for row in 0..visible_rows {
            let row_y = win.y + row as f32 * ROW_H;

            // Row separator
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
                let cell_rect = Rect::new(cell_x, cell_y, SLOT_W - 2.0 * SLOT_MARGIN, SLOT_W - SLOT_MARGIN * 2.0);

                let resp = ui.interact(slot_id, cell_rect);

                // Hover highlight
                if resp.hovered() {
                    let hover_color = [0.71, 1.0, 0.71, 1.0];
                    let (v, idx) = draw::quad_vertices(cell_rect.x + 1.0, cell_rect.y, cell_rect.w - 1.0,  cell_rect.h - SLOT_MARGIN * 2.0, hover_color);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::White,
                    });
                }

                let label_color = [tc[0] * 0.6, tc[1] * 0.6, tc[2] * 0.6, tc[3]];
                // Slot icon
                if let Some(icon_path) = self.slot_icon_path(content, character, data) {
                    let (v, idx) = draw::quad_vertices(cell_rect.x + (SLOT_W - ICON_SIZE) / 2.0 - SLOT_MARGIN, cell_rect.y, ICON_SIZE, ICON_SIZE, [1.0; 4]);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::Named(icon_path.clone()),
                    });

                    // Count/level text
                    if let Some(count_text) = self.slot_count_text(content, character) {
                        let text_w = ui.atlas.measure_text(&count_text);
                        let tx = cell_rect.x + ICON_SIZE - text_w;
                        let ty = cell_y + ICON_SIZE + 2.0;
                        ui.text(tx, ty, &count_text, label_color);
                    }

                    // Drag source for occupied slots
                    if resp.clicked() {
                        ui.drag_source(
                            HOTKEY_BAR_WINDOW_ID,
                            slot_index,
                            Some(icon_path),
                            (ICON_SIZE, ICON_SIZE),
                        );
                        ui.cancel_window_drag(HOTKEY_BAR_WINDOW_ID);
                    }
                }

                // TODO make Key label optional?
                // let key_label = ROW_KEYS[row][col];
                // ui.text(cell_x + 1.0, cell_y + 8.0, key_label, label_color);

                // Drop zone
                if let Some((source_id, source_item_index)) = ui.drop_zone(cell_rect) {
                    self.handle_drop(source_id, source_item_index, slot_index, character, &mut events);
                }

                // Tooltip on hover
                if resp.hovered() {
                    let tooltip = match content {
                        HotkeySlotContent::Skill { skill_id, level } => {
                            character.skills.get_skill(skill_id)
                                .map(|s| format!("{} Lv.{}", s.name, level))
                        }
                        HotkeySlotContent::Item { item_id, inventory_index } => {
                            let slot_count_table = data.item_slot_count.as_ref();
                            let card_name_table = data.card_name.as_ref();
                            character.inventory.get_item(inventory_index)
                                .map(|item| format_equipment_display_name(item, slot_count_table, card_name_table))
                                .or_else(|| data.item_name.as_ref()
                                    .and_then(|t| t.get_name(item_id))
                                    .map(|name| name.to_string()))
                        }
                        HotkeySlotContent::Empty => None,
                    };
                    if let Some(text) = tooltip {
                        ui.tooltip(cell_x, cell_y - 4.0, &text);
                    }
                }
            }
        }

        // Keyboard handling - Row 0: F1-F9 (always active)
        let f_keys = [
            ui.ctx.key_f1, ui.ctx.key_f2, ui.ctx.key_f3,
            ui.ctx.key_f4, ui.ctx.key_f5, ui.ctx.key_f6,
            ui.ctx.key_f7, ui.ctx.key_f8, ui.ctx.key_f9,
        ];
        for (i, &pressed) in f_keys.iter().enumerate() {
            if pressed {
                self.execute_slot(i, character, &mut events);
            }
        }

        // Rows 1-3: only when battle mode is on and chat is not active
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
                } else if let Some(col) = ROW4_CHARS.iter().position(|&c| c == lower) {
                    if visible_rows > 3 {
                        self.execute_slot(HOTKEY_COLS * 3 + col, character, &mut events);
                    }
                }
            }
        }

        events
    }
}
