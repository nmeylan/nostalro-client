use super::equipment_window::EQ_WINDOW_ID;
use super::input_dialog::{InputDialog, InputDialogConfig, InputDialogResult};
use crate::helper::dialog_container::DialogContainer;
use crate::helper::window_chrome::{
    FOOTER_TEX, ITEMWIN_MID_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container,
    draw_footer, draw_sys_button, draw_titlebar, text_color,
};
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::GameEvent;
use ragnarok_game::item::{InventoryTab, Item};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{RESIZE_HANDLE_TEX, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

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
const NUM_DIALOG_BASE: u32 = 810; // uses 810..813

const CELL_SIZE: f32 = 32.0;
const ICON_SIZE: f32 = 24.0;
const ICON_PAD: f32 = 4.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 19.0;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::{BuildCtx, InGameWindow, Window};
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 4.0;
const RESIZE_SIZE: f32 = 13.0;
const MINI_BTN_SIZE: f32 = 11.0;

const MIN_COLS: usize = 7;
const MAX_COLS: usize = 10;
const MIN_ROWS: usize = 2;
const MAX_ROWS: usize = 6;
const DEFAULT_COLS: usize = 7;
const DEFAULT_ROWS: usize = 2;

pub const TAB_USABLE_TEX: &str = ragnarok_resources::ui::basic::TAB_ITM_01;
pub const TAB_EQUIP_TEX: &str = ragnarok_resources::ui::basic::TAB_ITM_02;
pub const TAB_ETC_TEX: &str = ragnarok_resources::ui::basic::TAB_ITM_03;
const MINI_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_MINI_OFF;
const MINI_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_MINI_ON;
const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;

pub struct InventoryWindow {
    pub has_grf_textures: bool,
    scroll_offset: usize,
    tab_size: (f32, f32),
    grid_cols: usize,
    grid_rows: usize,
    resize_start: Option<(usize, usize)>,
    minimized: bool,
    container: DialogContainer,
    qty_dialog: Option<(u16, InputDialog)>,
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
            container: DialogContainer::new(),
            qty_dialog: None,
        }
    }

    /// Opens the "how many to retrieve" dialog for a cart→inventory drag of a
    /// stack; `index` is the cart slot.
    fn open_cart_qty_dialog(&mut self, index: u16, max: i16) {
        let mut dialog = InputDialog::new(
            InputDialogConfig {
                label: None,
                show_cancel: true,
                escape_cancels: true,
                default_value: max.to_string(),
                max_len: 6,
                numeric_only: true,
            },
            WidgetId(NUM_DIALOG_BASE),
        );
        dialog.init_container(&self.container);
        self.qty_dialog = Some((index, dialog));
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
        let content_w = tab_strip_w + (PAD_X) + grid_w + (SCROLLBAR_W) + (PAD_X);
        let win_w = content_w;
        let win_h = (TITLE_H) + (PAD_Y) + grid_h + (PAD_Y) + (FOOTER_H);
        (win_w, win_h, tab_strip_w)
    }
}

impl Window for InventoryWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
        self.container.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(TAB_USABLE_TEX) {
            self.tab_size = (w as f32, h as f32);
        }
        self.container.set_texture_sizes(size_fn);
    }

    fn window_size(&self) -> (f32, f32) {
        let (w, h, _) = self.compute_dimensions();
        (w, h)
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
        paths.extend(InputDialog::grf_texture_paths());
        paths
    }
}

impl InGameWindow for InventoryWindow {
    fn owns_keyboard(&self, _ctx: &BuildCtx) -> bool {
        self.qty_dialog.is_some()
    }

    fn wants_escape(&self, ctx: &BuildCtx) -> bool {
        ctx.character.inventory.is_open()
    }

    fn on_escape(&mut self, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        if self.qty_dialog.is_some() {
            self.qty_dialog = None;
        } else {
            ctx.character.inventory.close();
        }
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let character = &mut *ctx.character;
        let data = ctx.data;
        if !character.inventory.is_open() {
            self.qty_dialog = None;
            return Vec::new();
        }

        let slot_count_table = data.item_slot_count.as_ref();
        let card_name_table = data.card_name.as_ref();
        let producers = &character.char_names;
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

        let win_rect = Rect::new(win.x, win.y, win_w, win_h);
        ui.interact(INV_WINDOW_ID, win_rect);

        draw_titlebar(ui, win.x, win.y, win_w, TITLE_H, grf);
        ui.text(
            win.x + (17.0),
            win.y + (TITLE_H) - (3.0),
            "Inventory",
            text_color,
        );

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
        draw_sys_button(
            ui,
            mini_rect,
            (mini_size, mini_size),
            mini_resp.hovered(),
            grf,
            MINI_ON_TEX,
            MINI_OFF_TEX,
            Some('_'),
        );
        if mini_resp.clicked() {
            self.minimized = !self.minimized;
        }

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
        draw_sys_button(
            ui,
            close_rect,
            (close_size, close_size),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );
        if close_resp.clicked() {
            character.inventory.close();
            ui.has_grf_textures = prev_grf;
            return events;
        }

        if self.minimized {
            ui.has_grf_textures = prev_grf;
            return events;
        }

        let grid_area_x = win.x + tab_strip_w;
        let grid_area_w = (PAD_X) + self.grid_cols as f32 * (CELL_SIZE) + (SCROLLBAR_W) + (PAD_X);
        let grid_h = self.grid_rows as f32 * (CELL_SIZE);
        let container_y = win.y + (TITLE_H);
        let container_h = (PAD_Y) + grid_h + (PAD_Y);
        draw_container(ui, grid_area_x, container_y, grid_area_w, container_h, grf);

        let filtered_count = character.inventory.filtered_items().len();
        let total_rows = (filtered_count + self.grid_cols - 1) / self.grid_cols.max(1);
        let max_scroll = total_rows.saturating_sub(self.grid_rows);

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

                if grf {
                    let (v, idx) =
                        draw::quad_vertices(cx + pad, cy + pad, icon, icon, [1.0, 1.0, 1.0, 1.0]);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::Named(ITEMWIN_MID_TEX.to_string()),
                    });
                } else {
                    crate::helper::fallback::slot_cell(ui, cx + pad, cy + pad, icon, icon);
                }

                if item_idx >= filtered.len() {
                    continue;
                }
                let item = filtered[item_idx];

                if let Some(icon_path) = item.icon_path() {
                    let tint = if item.is_identified {
                        [1.0, 1.0, 1.0, 1.0]
                    } else {
                        [0.67, 0.67, 0.67, 1.0]
                    };
                    let (v, idx) = draw::quad_vertices(cx + pad, cy + pad, icon, icon, tint);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: idx.to_vec(),
                        texture: TextureRef::Named(icon_path),
                    });
                }

                let count_str = item.count.to_string();
                let count_w = ui.atlas.measure_text(&count_str);
                let count_x = cx + icon - count_w + 2.0;
                let count_y = cy + cell - 2.0;

                ui.text(count_x, count_y, &count_str, [0.0, 0.0, 0.0, 1.0]);

                if response.hovered() {
                    let display_name = format_equipment_display_name(
                        item,
                        slot_count_table,
                        card_name_table,
                        producers,
                    );
                    let tooltip_text = if item.count > 1 {
                        format!("{display_name}: {} ea.", item.count)
                    } else {
                        display_name
                    };
                    ui.tooltip(cx, cy - icon, &tooltip_text);
                }

                if response.clicked() && (!item.is_equipped() || item.is_ammunition()) {
                    ui.drag_source(
                        INV_WINDOW_ID,
                        item.index as usize,
                        item.icon_path(),
                        (ICON_SIZE, ICON_SIZE),
                    );
                }

                if response.right_clicked() {
                    events.push(GameEvent::ShowItemInfo { index: item.index });
                }
                if response.double_clicked() {
                    if character.storage.is_open() {
                        events.push(GameEvent::RequestDepositItem { index: item.index });
                    } else if item.is_ammunition() {
                        // Ammo only ever goes on from here; it comes off through
                        // the equipment window.
                        if !item.is_equipped() {
                            events.push(GameEvent::RequestEquipItem {
                                index: item.index,
                                location: item.equip_location(),
                            });
                        }
                    } else if item.is_equipment() {
                        if item.is_equipped() {
                            events.push(GameEvent::RequestUnequipItem { index: item.index });
                        } else {
                            events.push(GameEvent::RequestEquipItem {
                                index: item.index,
                                location: item.equip_location(),
                            });
                        }
                    } else if item.is_card() {
                        events.push(GameEvent::RequestCardInsertList {
                            card_index: item.index,
                        });
                    } else {
                        events.push(GameEvent::RequestUseItem { index: item.index });
                    }
                }
            }
        } // filtered dropped here

        let grid_rect = Rect::new(
            grid_x,
            grid_y,
            self.grid_cols as f32 * CELL_SIZE,
            self.grid_rows as f32 * CELL_SIZE,
        );
        if let Some((source_id, item_index)) = ui.drop_zone(grid_rect) {
            if source_id == EQ_WINDOW_ID {
                events.push(GameEvent::RequestUnequipItem {
                    index: item_index as u16,
                });
            } else if source_id == super::cart_window::CART_WINDOW_ID {
                let index = item_index as u16;
                if let Some(it) = character.cart.get_item(index) {
                    let count = it.count;
                    if count > 1 {
                        self.open_cart_qty_dialog(index, count);
                    } else {
                        events.push(GameEvent::RequestMoveItemCartToBody { index, count: 1 });
                    }
                }
            } else if source_id == super::storage_window::STORAGE_WINDOW_ID {
                events.push(GameEvent::RequestWithdrawItem {
                    index: item_index as u16,
                });
            }
        }

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

        let tab_x = win.x;
        let tab_img_w = tab_strip_w;
        let tab_img_h = if self.tab_size.1 > 0.0 {
            self.tab_size.1
        } else {
            container_h
        };

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

            if !grf {
                let active = character.inventory.active_tab == *tab;
                crate::helper::fallback::cell(
                    ui,
                    tab_x,
                    ty,
                    tab_img_w,
                    tab_btn_h,
                    active || resp.hovered(),
                );
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
            && let Some((start_cols, start_rows)) = self.resize_start
        {
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

        if let Some((index, dialog)) = &mut self.qty_dialog {
            match dialog.build(ui) {
                InputDialogResult::Submitted => {
                    let qty = dialog.value_i16().unwrap_or(0);
                    if qty > 0 {
                        events.push(GameEvent::RequestMoveItemCartToBody {
                            index: *index,
                            count: qty,
                        });
                    }
                    self.qty_dialog = None;
                }
                InputDialogResult::Cancel => {
                    self.qty_dialog = None;
                }
                InputDialogResult::None => {}
            }
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::item::ItemType;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::frame::DragState;
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

    #[test]
    fn dropping_a_cart_stack_opens_dialog_then_retrieves_entered_count() {
        let mut win = InventoryWindow::new();
        let mut character = Character::new();
        character.inventory.open();
        character.cart.open();
        character.cart.add_item(potion(3, 10));
        let data = DataTable::new();
        let mut state = StateCache::new();

        // Seed a cart drag released over the inventory grid (WidgetId(u32::MAX)
        // is the shared drag-state slot).
        {
            let drag = state.get_or_default::<DragState>(WidgetId(u32::MAX));
            drag.active = true;
            drag.source_id = super::super::cart_window::CART_WINDOW_ID;
            drag.item_index = 3;
        }

        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 48.0;
        ctx.mouse_y = 137.0;
        let events = {
            let mut ui = test_frame(&mut ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestMoveItemCartToBody { .. })),
            "a stack drop must open the dialog, not move immediately: {events:?}"
        );
        assert!(win.qty_dialog.is_some(), "quantity dialog should be open");

        win.qty_dialog.as_mut().unwrap().1.set_input_text("4");
        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.key_enter = true;
        let events = {
            let mut ui = test_frame(&mut ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            events.iter().any(|e| matches!(
                e,
                GameEvent::RequestMoveItemCartToBody { index: 3, count: 4 }
            )),
            "submitting the dialog retrieves the entered count: {events:?}"
        );
        assert!(win.qty_dialog.is_none());
    }
}
