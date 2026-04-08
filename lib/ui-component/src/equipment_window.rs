use ragnarok_game::card_name_table::CardNameTable;
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::GameEvent;
use ragnarok_game::inventory::{EquipmentLocation, InventoryData};
use ragnarok_game::item_slot_count_table::ItemSlotCountTable;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use crate::inventory_window::INV_WIN_ID;

const ITEM_INVERT_TEX: &str = "data/texture/유저인터페이스/basic_interface/item_invert.bmp";

// -- Widget IDs --
pub const EQ_WIN_ID: WidgetId = WidgetId(900);
const EQ_CLOSE_BTN_ID: WidgetId = WidgetId(901);
const EQ_MINI_BTN_ID: WidgetId = WidgetId(902);
const EQ_SLOT_BASE_ID: u32 = 910;

// -- Layout (matches official RO client: 3-column layout) --
// 280px wide, 17px titlebar, 130px content, 5 slot rows
const TITLE_H: f32 = 17.0;
const WIN_W: f32 = 280.0;
const CONTENT_H: f32 = 130.0;
const MINI_BTN_SIZE: f32 = 11.0;

// 3 columns: left slots (115px) | center character (50px) | right slots (115px)
const SIDE_COL_W: f32 = 115.0;
const CENTER_COL_W: f32 = 50.0;
const ICON_SIZE: f32 = 24.0;
const SLOT_ROWS: usize = 5;

// -- GRF textures --
const TITLEBAR_TEX: &str = "data/texture/유저인터페이스/basic_interface/titlebar_mid.bmp";
const EQUIP_BG_TEX: &str = "data/texture/유저인터페이스/basic_interface/equipwin_bg.bmp";
const SYS_BASE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_base_off.bmp";
const SYS_BASE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_base_on.bmp";
const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";
const MINI_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_off.bmp";
const MINI_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_on.bmp";

// Slot definitions matching official layout:
//   Left column: Head Top, Head Bottom, Weapon, Garment, Accessory1
//   Right column: Head Mid, Armor, Shield, Shoes, Accessory2
//   Center: Ammo (above character area)
struct EquipSlot {
    label: &'static str,
    location: EquipmentLocation,
    col: u8, // 0=left, 1=right, 2=center
    row: u8,
}

const EQUIP_SLOTS: &[EquipSlot] = &[
    // Left column
    EquipSlot { label: "Head Top", location: EquipmentLocation::HeadTop, col: 0, row: 0 },
    EquipSlot { label: "Head Low", location: EquipmentLocation::HeadLow, col: 0, row: 1 },
    EquipSlot { label: "Weapon", location: EquipmentLocation::HandRight, col: 0, row: 2 },
    EquipSlot { label: "Garment", location: EquipmentLocation::Garment, col: 0, row: 3 },
    EquipSlot { label: "Accessory", location: EquipmentLocation::AccessoryLeft, col: 0, row: 4 },
    // Right column
    EquipSlot { label: "Head Mid", location: EquipmentLocation::HeadMid, col: 1, row: 0 },
    EquipSlot { label: "Armor", location: EquipmentLocation::Armor, col: 1, row: 1 },
    EquipSlot { label: "Shield", location: EquipmentLocation::HandLeft, col: 1, row: 2 },
    EquipSlot { label: "Shoes", location: EquipmentLocation::Shoes, col: 1, row: 3 },
    EquipSlot { label: "Accessory", location: EquipmentLocation::AccessoryRight, col: 1, row: 4 },
    // Center column (ammo, positioned at top of center area)
    EquipSlot { label: "Ammo", location: EquipmentLocation::Ammo, col: 2, row: 0 },
];

pub struct EquipmentWindow {
    pub has_grf_textures: bool,
    pub open: bool,
    minimized: bool,
    bg_size: (f32, f32),
    /// Screen-space center for character sprite, set each frame by build()
    character_center: Option<[f32; 2]>,
    /// Index into the UI draw call list where paperdoll should be inserted
    paperdoll_insert_index: Option<usize>,
}

impl EquipmentWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            open: false,
            minimized: false,
            bg_size: (0.0, 0.0),
            character_center: None,
            paperdoll_insert_index: None,
        }
    }

    pub fn character_center(&self) -> Option<[f32; 2]> {
        self.character_center
    }

    pub fn paperdoll_insert_index(&self) -> Option<usize> {
        self.paperdoll_insert_index
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_visible(&self) -> bool {
        self.open && !self.minimized
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn set_texture_sizes(&mut self, size_fn: impl Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(EQUIP_BG_TEX) {
            self.bg_size = (w as f32, h as f32);
        }
    }

    pub fn build(
        &mut self,
        ui: &mut UiFrame,
        inventory: &InventoryData,
        slot_count_table: Option<&ItemSlotCountTable>,
        card_name_table: Option<&CardNameTable>,
    ) -> Vec<GameEvent> {
        self.character_center = None;
        self.paperdoll_insert_index = None;

        if !self.open {
            return Vec::new();
        }

        let mut events = Vec::new();

        if ui.ctx.key_escape {
            self.open = false;
            return events;
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;

        let grf = self.has_grf_textures;
        let text_color = text_color(grf);

        let win_w = WIN_W ;
        let content_h = if self.bg_size.1 > 0.0 { self.bg_size.1 } else { CONTENT_H  };
        let win_h = if self.minimized { TITLE_H  } else { (TITLE_H) + content_h };

        let default_x = 0.0;
        let default_y = 210.0 ;
        let win = ui.window_at(EQ_WIN_ID, win_w, win_h, TITLE_H , default_x, default_y);

        // Block clicks through window
        let win_rect = Rect::new(win.x, win.y, win_w, win_h);
        ui.interact(EQ_WIN_ID, win_rect);

        // -- Titlebar --
        draw_titlebar(ui, win.x, win.y, win_w, TITLE_H , grf);
        ui.text(win.x + (17.0), win.y + (TITLE_H) - (3.0), "Equipment", text_color);

        // Minimize button (left of close button)
        let btn_size = MINI_BTN_SIZE ;
        let mini_rect = Rect::new(
            win.x + win_w - btn_size * 2.0 - (6.0),
            win.y + ((TITLE_H) - btn_size) / 2.0,
            btn_size, btn_size,
        );
        let mini_resp = ui.interact(EQ_MINI_BTN_ID, mini_rect);
        if mini_resp.hovered() { ui.any_interactive_hovered = true; }
        if grf {
            let tex = if mini_resp.hovered() { MINI_ON_TEX } else { MINI_OFF_TEX };
            let (v, idx) = draw::quad_vertices(mini_rect.x, mini_rect.y, btn_size, btn_size, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: idx.to_vec(), texture: TextureRef::Named(tex.to_string()) });
        } else {
            let c = if mini_resp.hovered() { [0.8, 0.8, 0.2, 1.0] } else { text_color };
            ui.text(mini_rect.x + (2.0), mini_rect.y + btn_size - (1.0), "_", c);
        }
        if mini_resp.clicked() {
            self.minimized = !self.minimized;
        }

        // Close button
        let close_rect = Rect::new(
            win.x + win_w - btn_size - (3.0),
            win.y + ((TITLE_H) - btn_size) / 2.0,
            btn_size, btn_size,
        );
        let close_resp = ui.interact(EQ_CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() { ui.any_interactive_hovered = true; }
        if grf {
            let tex = if close_resp.hovered() { CLOSE_ON_TEX } else { CLOSE_OFF_TEX };
            let (v, idx) = draw::quad_vertices(close_rect.x, close_rect.y, btn_size, btn_size, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: idx.to_vec(), texture: TextureRef::Named(tex.to_string()) });
        } else {
            let c = if close_resp.hovered() { [1.0, 0.3, 0.3, 1.0] } else { text_color };
            ui.text(close_rect.x + (2.0), close_rect.y + btn_size - (1.0), "x", c);
        }
        if close_resp.clicked() {
            self.open = false;
            ui.has_grf_textures = prev_grf;
            return events;
        }

        if self.minimized {
            ui.has_grf_textures = prev_grf;
            return events;
        }

        // -- Content background (equipwin_bg.bmp) --
        let content_y = win.y + (TITLE_H);
        if grf {
            let (v, idx) = draw::quad_vertices(win.x, content_y, win_w, content_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: idx.to_vec(), texture: TextureRef::Named(EQUIP_BG_TEX.to_string()) });
        } else {
            let (v, idx) = draw::quad_vertices(win.x, content_y, win_w, content_h, [0.12, 0.12, 0.18, 0.95]);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: idx.to_vec(), texture: TextureRef::White });
            let bc = [0.4, 0.4, 0.5, 1.0];
            for (bx, by, bw, bh) in [
                (win.x, content_y, 1.0, content_h),
                (win.x + win_w - 1.0, content_y, 1.0, content_h),
                (win.x, content_y + content_h - 1.0, win_w, 1.0),
            ] {
                let (v, idx) = draw::quad_vertices(bx, by, bw, bh, bc);
                ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: idx.to_vec(), texture: TextureRef::White });
            }
        }

        let slot_h = content_h / SLOT_ROWS as f32;
        let character_x = win.x + SIDE_COL_W + CENTER_COL_W / 2.0;
        let character_y = content_y + content_h - slot_h;
        self.character_center = Some([character_x, character_y]);
        self.paperdoll_insert_index = Some(ui.draw_calls.len());

        // Highlight valid slots when dragging an equipment item from inventory
        let highlight_location: Option<u16> = ui.drag_info()
            .filter(|(src, _)| *src == INV_WIN_ID)
            .and_then(|(_, idx)| inventory.get_item(idx as u16))
            .filter(|item| item.is_equipment() && !item.is_equipped())
            .map(|item| item.location);

        // -- Equipment slots --
        let side_col_w = SIDE_COL_W ;
        let icon = ICON_SIZE ;

        for (i, slot) in EQUIP_SLOTS.iter().enumerate() {
            let (slot_x, slot_w) = match slot.col {
                0 => (win.x, side_col_w),                          // Left column
                1 => (win.x + win_w - side_col_w, side_col_w),     // Right column
                _ => (win.x + side_col_w, (CENTER_COL_W)),        // Center column
            };
            let slot_y = content_y + slot.row as f32 * slot_h;

            let slot_rect = Rect::new(slot_x, slot_y, slot_w, slot_h);
            let widget_id = WidgetId(EQ_SLOT_BASE_ID + i as u32);
            let response = ui.interact(widget_id, slot_rect);

            // Icon position: left-aligned for col0, right-aligned for col1
            let icon_pad = (slot_h - icon) / 2.0;
            let (icon_x, text_x, right_align, show_text) = match slot.col {
                0 => {
                    // Left column: icon on left, text left-aligned after icon
                    let ix = slot_x + (4.0);
                    let tx = ix + icon + (3.0);
                    (ix, tx, false, true)
                }
                1 => {
                    // Right column: icon on right, text right-aligned before icon
                    let ix = slot_x + slot_w - icon - (4.0);
                    (ix, ix - 3.0, true, true)
                }
                _ => {
                    // Center (ammo): icon centered, no text
                    let ix = slot_x + (slot_w - icon) / 2.0;
                    (ix, ix, false, false)
                }
            };
            let icon_y = slot_y + icon_pad;

            let text_y = slot_y + slot_h / 2.0 + (4.0);

            if let Some(item) = inventory.equipped_in_slot(slot.location) {
                // Item icon
                if let Some(icon_path) = item.icon_path() {
                    let (v, idx) = draw::quad_vertices(icon_x, icon_y, icon, icon, [1.0, 1.0, 1.0, 1.0]);
                    ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: idx.to_vec(), texture: TextureRef::Named(icon_path) });
                }

                // Item name
                if show_text {
                    let slot_count = slot_count_table
                        .map(|t| t.get_slot_count(item.item_id))
                        .unwrap_or(0);
                    let display_name = format_equipment_display_name(
                        item, slot_count, card_name_table,
                    );
                    if right_align {
                        let text_w = ui.atlas.measure_text(&display_name);
                        ui.text(text_x - text_w, text_y, &display_name, text_color);
                    } else {
                        ui.text(text_x, text_y, &display_name, text_color);
                    }
                }

                // Begin drag on click (to unequip by dragging to inventory)
                if response.clicked() {
                    ui.drag_source(EQ_WIN_ID, item.index as usize, item.icon_path(), (ICON_SIZE, ICON_SIZE));
                }

                // Unequip on double-click or right-click
                if response.double_clicked() || response.right_clicked() {
                    events.push(GameEvent::RequestUnequipItem { index: item.index });
                }
            }

            if let Some(loc) = highlight_location {
                if loc & InventoryData::slot_mask(slot.location) != 0 {
                    if grf {
                        let (v, idx) = draw::quad_vertices(slot_rect.x, slot_rect.y, slot_rect.w, slot_rect.h, [1.0, 1.0, 1.0, 1.0]);
                        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: idx.to_vec(), texture: TextureRef::Named(ITEM_INVERT_TEX.to_string()) });
                    } else {
                        let (v, idx) = draw::quad_vertices(slot_rect.x, slot_rect.y, slot_rect.w, slot_rect.h, [0.20, 0.42, 0.88, 0.35]);
                        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: idx.to_vec(), texture: TextureRef::White });
                    }
                }
            }

        }

        // Drop zone: accept drags from inventory (equip)
        let content_rect = Rect::new(win.x, content_y, win_w, content_h);
        if let Some((source_id, item_index)) = ui.drop_zone(content_rect) {
            if source_id == INV_WIN_ID {
                if let Some(item) = inventory.get_item(item_index as u16) {
                    if item.is_equipment() && !item.is_equipped() {
                        events.push(GameEvent::RequestEquipItem {
                            index: item.index,
                            location: item.location,
                        });
                    }
                }
            }
        }

        ui.has_grf_textures = prev_grf;
        events
    }

    pub fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX, EQUIP_BG_TEX, ITEM_INVERT_TEX,
            SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX, CLOSE_ON_TEX,
            MINI_OFF_TEX, MINI_ON_TEX,
        ]
    }
}

// -- Window chrome helpers --

fn text_color(has_grf: bool) -> [f32; 4] {
    if has_grf { [0.0, 0.0, 0.0, 1.0] } else { [1.0, 1.0, 1.0, 1.0] }
}

fn draw_titlebar(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, has_grf: bool) {
    if has_grf {
        let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::Named(TITLEBAR_TEX.to_string()) });
        let btn_size = 11.0 ;
        let btn_x = x + (4.0);
        let btn_y = y + (3.0);
        let tex = if Rect::new(btn_x, btn_y, btn_size, btn_size).contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
            SYS_BASE_ON_TEX
        } else {
            SYS_BASE_OFF_TEX
        };
        let (v, i) = draw::quad_vertices(btn_x, btn_y, btn_size, btn_size, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::Named(tex.to_string()) });
    } else {
        let (v, i) = draw::quad_vertices(x, y, w, h, [0.20, 0.20, 0.30, 0.95]);
        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
        let bc = [0.5, 0.5, 0.6, 1.0];
        for (bx, by, bw, bh) in [(x, y, w, 1.0), (x, y, 1.0, h), (x + w - 1.0, y, 1.0, h)] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
        }
    }
}
