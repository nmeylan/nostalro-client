use super::equipment_window::EQ_WINDOW_ID;
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::GameEvent;
use ragnarok_game::item::{Item, InventoryTab};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{RESIZE_HANDLE_TEX, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use crate::helper::window_chrome::{
    TITLEBAR_TEX, ITEMWIN_MID_TEX, FOOTER_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX,
    draw_titlebar, draw_container, draw_footer, text_color,
};

// -- Widget IDs --
pub const INV_WINDOW_ID: WidgetId = WidgetId(800);
const INV_TAB_USABLE_ID: WidgetId = WidgetId(801);
const INV_TAB_EQUIP_ID: WidgetId = WidgetId(802);
const INV_TAB_ETC_ID: WidgetId = WidgetId(803);
const INV_CLOSE_BTN_ID: WidgetId = WidgetId(804);
const INV_SCROLL_UP_ID: WidgetId = WidgetId(805);
const INV_SCROLL_DOWN_ID: WidgetId = WidgetId(806);
const INV_SCROLL_THUMB_ID: WidgetId = WidgetId(807);
const INV_MINI_BTN_ID: WidgetId = WidgetId(808);
const INV_RESIZE_ID: WidgetId = WidgetId(809);
const INV_ITEM_BASE_ID: u32 = 820;

// -- Layout --
const CELL_SIZE: f32 = 32.0;
const ICON_SIZE: f32 = 24.0;
const ICON_PAD: f32 = 4.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 19.0;
use crate::{Window, InGameWindow};
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 4.0;
const RESIZE_SIZE: f32 = 13.0;
const MINI_BTN_SIZE: f32 = 11.0;

// Grid size constraints
const MIN_COLS: usize = 7;
const MAX_COLS: usize = 10;
const MIN_ROWS: usize = 2;
const MAX_ROWS: usize = 6;
const DEFAULT_COLS: usize = 7;
const DEFAULT_ROWS: usize = 2;

// -- GRF textures --
const TAB_USABLE_TEX: &str = "data/texture/유저인터페이스/basic_interface/tab_itm_01.bmp";
const TAB_EQUIP_TEX: &str = "data/texture/유저인터페이스/basic_interface/tab_itm_02.bmp";
const TAB_ETC_TEX: &str = "data/texture/유저인터페이스/basic_interface/tab_itm_03.bmp";
const MINI_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_off.bmp";
const MINI_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_on.bmp";
const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

pub struct InventoryWindow {
    pub has_grf_textures: bool,
    scroll_offset: usize,
    tab_size: (f32, f32),
    grid_cols: usize,
    grid_rows: usize,
    resize_start: Option<(usize, usize)>,
    minimized: bool,
}

impl Default for InventoryWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl InventoryWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            scroll_offset: 0,
            tab_size: (0.0, 0.0),
            grid_cols: DEFAULT_COLS,
            grid_rows: DEFAULT_ROWS,
            resize_start: None,
            minimized: false,
        }
    }

    pub fn is_minimized(&self) -> bool {
        self.minimized
    }

    pub fn set_minimized(&mut self, value: bool) {
        self.minimized = value;
    }

    fn compute_dimensions(&self) -> (f32, f32, f32) {
        let grid_w = self.grid_cols as f32 * (CELL_SIZE);
        let grid_h = self.grid_rows as f32 * (CELL_SIZE);
        let tab_strip_w = if self.tab_size.0 > 0.0 {
            self.tab_size.0
        } else {
            20.0
        };
        // layout: tab_strip | pad | grid | scrollbar | pad
        let content_w = tab_strip_w + (PAD_X) + grid_w + (SCROLLBAR_W) + (PAD_X);
        let win_w = content_w;
        let win_h = (TITLE_H) + (PAD_Y) + grid_h + (PAD_Y) + (FOOTER_H);
        (win_w, win_h, tab_strip_w)
    }

}

impl Window for InventoryWindow {
    fn has_grf_textures(&self) -> bool { self.has_grf_textures }
    fn set_has_grf_textures(&mut self, value: bool) { self.has_grf_textures = value; }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(TAB_USABLE_TEX) {
            self.tab_size = (w as f32, h as f32);
        }
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            ITEMWIN_MID_TEX,
            FOOTER_TEX,
            TAB_USABLE_TEX,
            TAB_EQUIP_TEX,
            TAB_ETC_TEX,
            MINI_OFF_TEX,
            MINI_ON_TEX,
            RESIZE_HANDLE_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for InventoryWindow {
    fn build(&mut self, ui: &mut UiFrame, character: &mut Character, data: &DataTable) -> Vec<GameEvent> {
        if !character.inventory.is_open() {
            return Vec::new();
        }

        let slot_count_table = data.item_slot_count.as_ref();
        let card_name_table = data.card_name.as_ref();
        let mut events = Vec::new();


        let scrollbar_w = SCROLLBAR_W;
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;

        let grf = self.has_grf_textures;
        let text_color = text_color(grf);

        let (win_w, full_h, tab_strip_w) = self.compute_dimensions();
        let win_h = if self.minimized { TITLE_H } else { full_h };

        let default_x = 0.0;
        let default_y = 100.0;
        let win = ui.window_at(INV_WINDOW_ID, win_w, win_h, TITLE_H, default_x, default_y);

        // Block clicks through window
        let win_rect = Rect::new(win.x, win.y, win_w, win_h);
        ui.interact(INV_WINDOW_ID, win_rect);

        // -- Titlebar --
        draw_titlebar(ui, win.x, win.y, win_w, TITLE_H, grf);
        ui.text(
            win.x + (17.0),
            win.y + (TITLE_H) - (3.0),
            "Inventory",
            text_color,
        );

        // Minimize button (left of close button)
        let mini_size = MINI_BTN_SIZE;
        let mini_rect = Rect::new(
            win.x + win_w - mini_size * 2.0 - (6.0),
            win.y + ((TITLE_H) - mini_size) / 2.0,
            mini_size,
            mini_size,
        );
        let mini_resp = ui.interact(INV_MINI_BTN_ID, mini_rect);
        if mini_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if grf {
            let tex = if mini_resp.hovered() {
                MINI_ON_TEX
            } else {
                MINI_OFF_TEX
            };
            let (v, idx) = draw::quad_vertices(
                mini_rect.x,
                mini_rect.y,
                mini_size,
                mini_size,
                [1.0, 1.0, 1.0, 1.0],
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            let c = if mini_resp.hovered() {
                [0.8, 0.8, 0.2, 1.0]
            } else {
                text_color
            };
            ui.text(mini_rect.x + (2.0), mini_rect.y + mini_size - (1.0), "_", c);
        }
        if mini_resp.clicked() {
            self.minimized = !self.minimized;
        }

        // Close button
        let close_size = MINI_BTN_SIZE;
        let close_rect = Rect::new(
            win.x + win_w - close_size - (3.0),
            win.y + ((TITLE_H) - close_size) / 2.0,
            close_size,
            close_size,
        );
        let close_resp = ui.interact(INV_CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if grf {
            let tex = if close_resp.hovered() {
                CLOSE_ON_TEX
            } else {
                CLOSE_OFF_TEX
            };
            let (v, idx) = draw::quad_vertices(
                close_rect.x,
                close_rect.y,
                close_size,
                close_size,
                [1.0, 1.0, 1.0, 1.0],
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            let close_color = if close_resp.hovered() {
                [1.0, 0.3, 0.3, 1.0]
            } else {
                text_color
            };
            ui.text(
                close_rect.x + (2.0),
                close_rect.y + close_size - (1.0),
                "x",
                close_color,
            );
        }
        if close_resp.clicked() {
            character.inventory.close();
            ui.has_grf_textures = prev_grf;
            return events;
        }

        if self.minimized {
            ui.has_grf_textures = prev_grf;
            return events;
        }

        // -- Grid container (right of tab strip) --
        let grid_area_x = win.x + tab_strip_w;
        let grid_area_w = (PAD_X) + self.grid_cols as f32 * (CELL_SIZE) + (SCROLLBAR_W) + (PAD_X);
        let grid_h = self.grid_rows as f32 * (CELL_SIZE);
        let container_y = win.y + (TITLE_H);
        let container_h = (PAD_Y) + grid_h + (PAD_Y);
        draw_container(ui, grid_area_x, container_y, grid_area_w, container_h, grf);

        let filtered_count = character.inventory.filtered_items().len();
        let total_rows = (filtered_count + self.grid_cols - 1) / self.grid_cols.max(1);
        let max_scroll = total_rows.saturating_sub(self.grid_rows);

        // Render grid items
        let grid_x = grid_area_x + (PAD_X);
        let grid_y = container_y + (PAD_Y);
        let cell = CELL_SIZE;
        let icon = ICON_SIZE;
        let pad = ICON_PAD;

        {
            let filtered: Vec<&Item> = character.inventory.filtered_items();
            let start = self.scroll_offset * self.grid_cols;
            let visible_count = self.grid_rows * self.grid_cols;

            for slot in 0..visible_count {
                let item_idx = start + slot;
                let col = slot % self.grid_cols;
                let row = slot / self.grid_cols;
                let cx = grid_x + col as f32 * cell;
                let cy = grid_y + row as f32 * cell;

                let cell_rect = Rect::new(cx, cy, cell, cell);
                let widget_id = WidgetId(INV_ITEM_BASE_ID + slot as u32);
                let response = ui.interact(widget_id, cell_rect);

                // Cell background shadow
                if grf {
                    let (v, idx) =
                        draw::quad_vertices(cx + pad, cy + pad, icon, icon, [1.0, 1.0, 1.0, 1.0]);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::Named(ITEMWIN_MID_TEX.to_string()),
                    });
                } else {
                    let (v, idx) = draw::quad_vertices(
                        cx + pad,
                        cy + pad,
                        icon,
                        icon,
                        [0.08, 0.08, 0.12, 0.8],
                    );
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::White,
                    });
                }

                if item_idx >= filtered.len() {
                    continue;
                }
                let item = filtered[item_idx];

                // Item icon
                if let Some(icon_path) = item.icon_path() {
                    let tint = if item.is_identified {
                        [1.0, 1.0, 1.0, 1.0]
                    } else {
                        [0.67, 0.67, 0.67, 1.0]
                    };
                    let (v, idx) =
                        draw::quad_vertices(cx + pad, cy + pad, icon, icon, tint);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::Named(icon_path),
                    });
                }

                // Stack count
                let count_str = item.count.to_string();
                let count_w = ui.atlas.measure_text(&count_str);
                let count_x = cx + icon - count_w + 2.0;
                let count_y = cy + cell - 2.0;

                ui.text(count_x, count_y, &count_str, [0.0, 0.0, 0.0, 1.0]);

                if response.hovered() {
                    let display_name = format_equipment_display_name(item, slot_count_table, card_name_table);
                    let tooltip_text = if item.count > 1 {
                        format!("{display_name} {} ea", item.count)
                    } else {
                        display_name
                    };
                    ui.tooltip(cx, cy - icon, &tooltip_text);
                }

                // Begin drag on click (ammo stays draggable even when equipped)
                if response.clicked() && (!item.is_equipped() || item.is_ammunition()) {
                    ui.drag_source(
                        INV_WINDOW_ID,
                        item.index as usize,
                        item.icon_path(),
                        (ICON_SIZE, ICON_SIZE),
                    );
                }

                // Right-click: show item info
                if response.right_clicked() {
                    events.push(GameEvent::ShowItemInfo { index: item.index });
                }
                // Double-click: use (consumable), equip/unequip (equipment), or insert (card)
                if response.double_clicked() {
                    if item.is_equipment() {
                        if item.is_equipped() {
                            events.push(GameEvent::RequestUnequipItem { index: item.index });
                        } else {
                            events.push(GameEvent::RequestEquipItem {
                                index: item.index,
                                location: item.equip_location(),
                            });
                        }
                    } else if item.is_card() {
                        events.push(GameEvent::RequestCardInsertList { card_index: item.index });
                    } else {
                        events.push(GameEvent::RequestUseItem { index: item.index });
                    }
                }
            }
        } // filtered dropped here

        // Drop zone: accept drags from equipment window (unequip)
        let grid_rect = Rect::new(
            grid_x,
            grid_y,
            self.grid_cols as f32 * CELL_SIZE,
            self.grid_rows as f32 * CELL_SIZE,
        );
        if let Some((source_id, item_index)) = ui.drop_zone(grid_rect)
            && source_id == EQ_WINDOW_ID {
                events.push(GameEvent::RequestUnequipItem {
                    index: item_index as u16,
                });
            }

        // -- Scrollbar (only when needed) --
        if total_rows > self.grid_rows {
            let sb_x = win.x + win_w - scrollbar_w - (1.0);
            let content_rect = Rect::new(grid_area_x, container_y, grid_area_w, container_h);
            self.scroll_offset = scrollbar::scrollbar(
                ui,
                ScrollbarIds {
                    up: INV_SCROLL_UP_ID,
                    down: INV_SCROLL_DOWN_ID,
                    thumb: INV_SCROLL_THUMB_ID,
                },
                self.scroll_offset,
                self.grid_rows,
                max_scroll,
                content_rect,
                sb_x,
                container_y,
                container_h,
            );
        }

        // -- Tab strip (left side, one image for all 3 tabs) --
        let tab_x = win.x;
        let tab_img_w = tab_strip_w;
        let tab_img_h = if self.tab_size.1 > 0.0 {
            self.tab_size.1
        } else {
            container_h
        };

        // -- Footer --
        let footer_y = container_y + container_h;
        draw_footer(ui, win.x, footer_y, win_w, FOOTER_H, grf);
        let total_count = character.inventory.all_items().len();
        let item_count_label = format!("Num: {total_count}/100");
        ui.text(
            win.x + tab_img_w + (4.0),
            footer_y + (FOOTER_H) - (5.0),
            &item_count_label,
            text_color,
        );

        // White background for the full tab column height (below the texture)
        let (v, idx) = draw::quad_vertices(
            tab_x,
            container_y,
            tab_img_w,
            container_h,
            [1.0, 1.0, 1.0, 1.0],
        );
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: idx.to_vec(),
            texture: TextureRef::White,
        });

        // Tab image on top (only covers its native height)
        let tab_tex = match character.inventory.active_tab {
            InventoryTab::Usable => TAB_USABLE_TEX,
            InventoryTab::Equip => TAB_EQUIP_TEX,
            InventoryTab::Etc => TAB_ETC_TEX,
        };
        if grf {
            let (v, idx) = draw::quad_vertices(
                tab_x,
                container_y,
                tab_img_w,
                tab_img_h,
                [1.0, 1.0, 1.0, 1.0],
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::Named(tab_tex.to_string()),
            });
        }

        // Invisible click zones for each tab (divide tab strip into 3 equal parts)
        let tab_btn_h = tab_img_h / 3.0;
        let tab_defs = [
            (INV_TAB_USABLE_ID, InventoryTab::Usable, "Use"),
            (INV_TAB_EQUIP_ID, InventoryTab::Equip, "Eqp"),
            (INV_TAB_ETC_ID, InventoryTab::Etc, "Etc"),
        ];
        for (i, (id, tab, label)) in tab_defs.iter().enumerate() {
            let ty = container_y + i as f32 * tab_btn_h;
            let tab_rect = Rect::new(tab_x, ty, tab_img_w, tab_btn_h);
            let resp = ui.interact(*id, tab_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }

            // Fallback rendering when no GRF textures
            if !grf {
                let active = character.inventory.active_tab == *tab;
                let bg = if active {
                    [0.25, 0.25, 0.38, 1.0]
                } else if resp.hovered() {
                    [0.20, 0.20, 0.30, 1.0]
                } else {
                    [0.15, 0.15, 0.22, 1.0]
                };
                let (v, idx) = draw::quad_vertices(tab_x, ty, tab_img_w, tab_btn_h, bg);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
                let tw = ui.atlas.measure_text(label);
                ui.text(
                    tab_x + (tab_img_w - tw) / 2.0,
                    ty + tab_btn_h / 2.0 + (4.0),
                    label,
                    text_color,
                );
            }

            if resp.clicked() && character.inventory.active_tab != *tab {
                character.inventory.active_tab = *tab;
                self.scroll_offset = 0;
            }
        }

        // Resize handle (bottom-right of footer)
        let resize_rect = Rect::new(
            win.x + win_w - RESIZE_SIZE,
            footer_y + FOOTER_H - RESIZE_SIZE,
            RESIZE_SIZE,
            RESIZE_SIZE,
        );
        let resize = ui.resize_handle(INV_RESIZE_ID, resize_rect);
        if resize.started {
            self.resize_start = Some((self.grid_cols, self.grid_rows));
        }
        if resize.dragging
            && let Some((start_cols, start_rows)) = self.resize_start {
                let new_cols = (start_cols as f32 + resize.delta_x / CELL_SIZE).round() as i32;
                let new_rows = (start_rows as f32 + resize.delta_y / CELL_SIZE).round() as i32;
                let new_cols = new_cols.clamp(MIN_COLS as i32, MAX_COLS as i32) as usize;
                let new_rows = new_rows.clamp(MIN_ROWS as i32, MAX_ROWS as i32) as usize;
                if new_cols != self.grid_cols || new_rows != self.grid_rows {
                    self.grid_cols = new_cols;
                    self.grid_rows = new_rows;
                    let total_rows = (filtered_count + self.grid_cols - 1) / self.grid_cols.max(1);
                    let max_scroll = total_rows.saturating_sub(self.grid_rows);
                    if self.scroll_offset > max_scroll {
                        self.scroll_offset = max_scroll;
                    }
                }
            }

        ui.has_grf_textures = prev_grf;
        events
    }
}

