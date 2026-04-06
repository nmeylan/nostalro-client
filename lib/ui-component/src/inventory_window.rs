use crate::equipment_window::EQ_WIN_ID;
use ragnarok_game::event::GameEvent;
use ragnarok_game::inventory::{InventoryItem, InventoryTab};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

// -- Widget IDs --
pub const INV_WIN_ID: WidgetId = WidgetId(800);
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
use crate::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 4.0;
const RESIZE_BTN_SIZE: f32 = 13.0;
const MINI_BTN_SIZE: f32 = 11.0;

// Grid size constraints
const MIN_COLS: usize = 7;
const MAX_COLS: usize = 10;
const MIN_ROWS: usize = 2;
const MAX_ROWS: usize = 6;
const DEFAULT_COLS: usize = 7;
const DEFAULT_ROWS: usize = 2;

// -- GRF textures --
const TITLEBAR_TEX: &str = "data/texture/유저인터페이스/basic_interface/titlebar_mid.bmp";
const ITEMWIN_MID_TEX: &str = "data/texture/유저인터페이스/basic_interface/itemwin_mid.bmp";
const FOOTER_TEX: &str = "data/texture/유저인터페이스/basic_interface/btnbar_mid2.bmp";
const TAB_USABLE_TEX: &str = "data/texture/유저인터페이스/basic_interface/tab_itm_01.bmp";
const TAB_EQUIP_TEX: &str = "data/texture/유저인터페이스/basic_interface/tab_itm_02.bmp";
const TAB_ETC_TEX: &str = "data/texture/유저인터페이스/basic_interface/tab_itm_03.bmp";
const MINI_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_off.bmp";
const MINI_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_on.bmp";
const RESIZE_TEX: &str = "data/texture/유저인터페이스/btn_resize.bmp";
const SYS_BASE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_base_off.bmp";
const SYS_BASE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_base_on.bmp";
const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

#[derive(Default)]
struct ResizeState {
    dragging: bool,
    start_mouse: (f32, f32),
    start_cols: usize,
    start_rows: usize,
}

pub struct InventoryWindow {
    pub has_grf_textures: bool,
    pub inventory: ragnarok_game::inventory::InventoryData,
    scroll_offset: usize,
    tab_size: (f32, f32),
    grid_cols: usize,
    grid_rows: usize,
    minimized: bool,
}

impl InventoryWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            inventory: ragnarok_game::inventory::InventoryData::new(),
            scroll_offset: 0,
            tab_size: (0.0, 0.0),
            grid_cols: DEFAULT_COLS,
            grid_rows: DEFAULT_ROWS,
            minimized: false,
        }
    }

    pub fn set_texture_sizes(&mut self, size_fn: impl Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(TAB_USABLE_TEX) {
            self.tab_size = (w as f32, h as f32);
        }
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

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        if !self.inventory.is_open() {
            return Vec::new();
        }

        let mut events = Vec::new();

        if ui.ctx.key_escape {
            self.inventory.close();
            return events;
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;

        let grf = self.has_grf_textures;
        let text_color = text_color(grf);

        let (win_w, full_h, tab_strip_w) = self.compute_dimensions();
        let win_h = if self.minimized { TITLE_H } else { full_h };

        let default_x = ui.ctx.screen_width - win_w - (10.0);
        let default_y = 100.0;
        let win = ui.window_at(INV_WIN_ID, win_w, win_h, TITLE_H, default_x, default_y);

        // Block clicks through window
        let win_rect = Rect::new(win.x, win.y, win_w, win_h);
        ui.interact(INV_WIN_ID, win_rect);

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
            self.inventory.close();
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

        let filtered_count = self.inventory.filtered_items().len();
        let total_rows = (filtered_count + self.grid_cols - 1) / self.grid_cols.max(1);
        let max_scroll = total_rows.saturating_sub(self.grid_rows);

        // Render grid items
        let grid_x = grid_area_x + (PAD_X);
        let grid_y = container_y + (PAD_Y);
        let cell = CELL_SIZE;
        let icon = ICON_SIZE;
        let pad = ICON_PAD;

        let mut hovered_tooltip: Option<(f32, f32, String)> = None;
        {
            let filtered: Vec<&InventoryItem> = self.inventory.filtered_items();
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
                    let (v, idx) =
                        draw::quad_vertices(cx + pad, cy + pad, icon, icon, [1.0, 1.0, 1.0, 1.0]);
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
                    let tooltip_text = if item.count > 1 {
                        format!("{} {} ea", item.name, item.count)
                    } else {
                        item.name.clone()
                    };
                    hovered_tooltip = Some((cx, cy - icon, tooltip_text));
                }

                // Begin drag on click for equipment items
                if response.clicked() && item.is_equipment() && !item.is_equipped() {
                    ui.drag_source(
                        INV_WIN_ID,
                        item.index as usize,
                        item.icon_path(),
                        (ICON_SIZE, ICON_SIZE),
                    );
                }

                // Double-click or right-click: use (consumable) or equip/unequip (equipment)
                if response.double_clicked() || response.right_clicked() {
                    if item.is_equipment() {
                        if item.is_equipped() {
                            events.push(GameEvent::RequestUnequipItem { index: item.index });
                        } else {
                            events.push(GameEvent::RequestEquipItem {
                                index: item.index,
                                location: item.location,
                            });
                        }
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
        if let Some((source_id, item_index)) = ui.drop_zone(grid_rect) {
            if source_id == EQ_WIN_ID {
                events.push(GameEvent::RequestUnequipItem {
                    index: item_index as u16,
                });
            }
        }

        // -- Scrollbar (only when needed) --
        if total_rows > self.grid_rows {
            let sb_x = grid_area_x + (PAD_X) + self.grid_cols as f32 * (CELL_SIZE);
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
        let total_count = self.inventory.all_items().len();
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
        let tab_tex = match self.inventory.active_tab {
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
                let active = self.inventory.active_tab == *tab;
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

            if resp.clicked() && self.inventory.active_tab != *tab {
                self.inventory.active_tab = *tab;
                self.scroll_offset = 0;
            }
        }

        // Resize handle (bottom-right of footer)
        let resize_size = RESIZE_BTN_SIZE;
        let resize_rect = Rect::new(
            win.x + win_w - resize_size,
            footer_y + (FOOTER_H) - resize_size,
            resize_size,
            resize_size,
        );
        let resize_resp = ui.interact(INV_RESIZE_ID, resize_rect);
        if resize_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if grf {
            let (v, idx) = draw::quad_vertices(
                resize_rect.x,
                resize_rect.y,
                resize_size,
                resize_size,
                [1.0, 1.0, 1.0, 1.0],
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::Named(RESIZE_TEX.to_string()),
            });
        } else {
            let c = if resize_resp.hovered() {
                [0.7, 0.7, 0.8, 1.0]
            } else {
                [0.4, 0.4, 0.5, 1.0]
            };
            // Draw a small triangle hint
            let (v, idx) = draw::quad_vertices(
                resize_rect.x + resize_size * 0.5,
                resize_rect.y + resize_size * 0.5,
                resize_size * 0.5,
                resize_size * 0.5,
                c,
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::White,
            });
        }

        // Handle resize dragging
        let cell_s = CELL_SIZE;
        let mouse_x = ui.ctx.mouse_x;
        let mouse_y = ui.ctx.mouse_y;
        let mouse_clicked = ui.ctx.mouse_clicked;
        let mouse_down = ui.ctx.mouse_down;

        let new_size = {
            let rs = ui.state.get_or_default::<ResizeState>(INV_RESIZE_ID);
            if resize_resp.clicked() || (resize_rect.contains(mouse_x, mouse_y) && mouse_clicked) {
                rs.dragging = true;
                rs.start_mouse = (mouse_x, mouse_y);
                rs.start_cols = self.grid_cols;
                rs.start_rows = self.grid_rows;
            }
            if !mouse_down {
                rs.dragging = false;
            }
            if rs.dragging {
                let dx = mouse_x - rs.start_mouse.0;
                let dy = mouse_y - rs.start_mouse.1;
                let new_cols = (rs.start_cols as f32 + dx / cell_s).round() as i32;
                let new_rows = (rs.start_rows as f32 + dy / cell_s).round() as i32;
                Some((
                    new_cols.clamp(MIN_COLS as i32, MAX_COLS as i32) as usize,
                    new_rows.clamp(MIN_ROWS as i32, MAX_ROWS as i32) as usize,
                ))
            } else {
                None
            }
        };
        if let Some((new_cols, new_rows)) = new_size {
            if new_cols != self.grid_cols || new_rows != self.grid_rows {
                self.grid_cols = new_cols;
                self.grid_rows = new_rows;
                // Clamp scroll offset to new max
                let total_rows = (filtered_count + self.grid_cols - 1) / self.grid_cols.max(1);
                let max_scroll = total_rows.saturating_sub(self.grid_rows);
                if self.scroll_offset > max_scroll {
                    self.scroll_offset = max_scroll;
                }
            }
        }

        // Tooltip
        if let Some((mx, my, text)) = hovered_tooltip {
            let tw = ui.atlas.measure_text(&text);
            let th = ui.atlas.line_height;
            let pad_t = 4.0;
            let tx = mx + 12.0;
            let ty = my + 8.0;
            let (v, idx) = draw::quad_vertices(
                tx - pad_t,
                ty,
                tw + pad_t * 2.0,
                th + pad_t * 2.0,
                [0.0, 0.0, 0.0, 0.85],
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: idx.to_vec(),
                texture: TextureRef::White,
            });
            ui.text(tx, ty + th, &text, [1.0, 1.0, 1.0, 1.0]);
        }

        ui.has_grf_textures = prev_grf;
        events
    }

    pub fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            ITEMWIN_MID_TEX,
            FOOTER_TEX,
            TAB_USABLE_TEX,
            TAB_EQUIP_TEX,
            TAB_ETC_TEX,
            MINI_OFF_TEX,
            MINI_ON_TEX,
            RESIZE_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

// -- Window chrome helpers --

fn text_color(has_grf: bool) -> [f32; 4] {
    if has_grf {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    }
}

fn draw_titlebar(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, has_grf: bool) {
    if has_grf {
        let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(TITLEBAR_TEX.to_string()),
        });
        let btn_size = 11.0;
        let btn_x = x + (4.0);
        let btn_y = y + (3.0);
        let tex = if Rect::new(btn_x, btn_y, btn_size, btn_size)
            .contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
        {
            SYS_BASE_ON_TEX
        } else {
            SYS_BASE_OFF_TEX
        };
        let (v, i) = draw::quad_vertices(btn_x, btn_y, btn_size, btn_size, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(tex.to_string()),
        });
    } else {
        let (v, i) = draw::quad_vertices(x, y, w, h, [0.20, 0.20, 0.30, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let bc = [0.5, 0.5, 0.6, 1.0];
        for (bx, by, bw, bh) in [(x, y, w, 1.0), (x, y, 1.0, h), (x + w - 1.0, y, 1.0, h)] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
    }
}

fn draw_container(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, has_grf: bool) {
    if has_grf {
        let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let (v, i) = draw::quad_vertices(x + w - 1.0, y, 1.0, h, [0.8, 0.8, 0.8, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    } else {
        let (v, i) = draw::quad_vertices(x, y, w, h, [0.12, 0.12, 0.18, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let bc = [0.4, 0.4, 0.5, 1.0];
        for (bx, by, bw, bh) in [(x, y, 1.0, h), (x + w - 1.0, y, 1.0, h)] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
    }
}

fn draw_footer(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, has_grf: bool) {
    if has_grf {
        let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(FOOTER_TEX.to_string()),
        });
    } else {
        let (v, i) = draw::quad_vertices(x, y, w, h, [0.18, 0.18, 0.25, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let bc = [0.5, 0.5, 0.6, 1.0];
        for (bx, by, bw, bh) in [
            (x, y + h - 1.0, w, 1.0),
            (x, y, 1.0, h),
            (x + w - 1.0, y, 1.0, h),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, bc);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
    }
}
