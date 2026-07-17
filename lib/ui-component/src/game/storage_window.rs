use super::cart_window::CART_WINDOW_ID;
use super::inventory_window::INV_WINDOW_ID;
use super::number_input::{NumberInputConfig, NumberInputDialog, NumberInputResult};
use crate::helper::dialog_container::DialogContainer;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    FOOTER_TEX, ITEMWIN_MID_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_titlebar,
    text_color,
};
use super::inventory_window::{TAB_EQUIP_TEX, TAB_ETC_TEX, TAB_USABLE_TEX};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::GameEvent;
use ragnarok_game::item::InventoryTab;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, RESIZE_HANDLE_TEX, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const STORAGE_WINDOW_ID: WidgetId = WidgetId(3800);
const CLOSE_BTN_ID: WidgetId = WidgetId(3801);
const SCROLL_UP_ID: WidgetId = WidgetId(3802);
const SCROLL_DOWN_ID: WidgetId = WidgetId(3803);
const SCROLL_THUMB_ID: WidgetId = WidgetId(3804);
const RESIZE_ID: WidgetId = WidgetId(3805);
const NUM_DIALOG_BASE: u32 = 3806; // uses 3806..3809
const TAB_BASE_ID: u32 = 3810; // 3810..3816, 7 tabs
const ROW_BASE_ID: u32 = 3820;

const CLOSE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/btn_close.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/btn_close_a.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/btn_close_b.bmp",
};
const TAB_DEFS: [(InventoryTab, &str, &str); 3] = [
    (InventoryTab::Usable, TAB_USABLE_TEX, "Use"),
    (InventoryTab::Equip, TAB_EQUIP_TEX, "Eqp"),
    (InventoryTab::Etc, TAB_ETC_TEX, "Etc"),
];

const WIN_W: f32 = 280.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 27.0;
const ROW_H: f32 = 32.0;
const ICON: f32 = 24.0;
const TAB_FALLBACK_W: f32 = 20.0;
const TAB_FALLBACK_H: f32 = 96.0;
const CLOSE_W: f32 = 42.0;
const CLOSE_H: f32 = 20.0;
const RESIZE_SIZE: f32 = 13.0;
const MIN_ROWS: usize = 8;
const MAX_ROWS: usize = 17;
const DEFAULT_ROWS: usize = 8;

const GREY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const WARN_COLOR: [f32; 4] = [0.7, 0.1, 0.1, 1.0];

enum PendingMove {
    Withdraw { index: u16 },
    DepositBody { index: u16 },
    DepositCart { index: u16 },
}

pub struct StorageWindow {
    pub has_grf_textures: bool,
    active_tab: InventoryTab,
    scroll_offset: usize,
    rows: usize,
    tab_size: (f32, f32),
    close_size: (f32, f32),
    resize_start: Option<usize>,
    container: DialogContainer,
    qty_dialog: Option<(PendingMove, NumberInputDialog)>,
}

impl Default for StorageWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            active_tab: InventoryTab::Usable,
            scroll_offset: 0,
            rows: DEFAULT_ROWS,
            tab_size: (TAB_FALLBACK_W, TAB_FALLBACK_H),
            close_size: (CLOSE_W, CLOSE_H),
            resize_start: None,
            container: DialogContainer::new(),
            qty_dialog: None,
        }
    }

    pub fn active_tab(&self) -> InventoryTab {
        self.active_tab
    }

    pub fn set_active_tab(&mut self, tab: InventoryTab) {
        self.active_tab = tab;
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn set_rows(&mut self, rows: usize) {
        self.rows = rows.clamp(MIN_ROWS, MAX_ROWS);
    }

    fn win_h(&self) -> f32 {
        TITLE_H + self.rows as f32 * ROW_H + FOOTER_H
    }

    fn open_qty_dialog(&mut self, kind: PendingMove, max: i16) {
        let mut dialog = NumberInputDialog::new(
            NumberInputConfig {
                label: None,
                show_cancel: true,
                escape_cancels: true,
                default_value: max.to_string(),
                max_len: 6,
            },
            WidgetId(NUM_DIALOG_BASE),
        );
        dialog.init_container(&self.container);
        self.qty_dialog = Some((kind, dialog));
    }
}

impl Window for StorageWindow {
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
        if let Some((w, h)) = size_fn(CLOSE_BTN.normal) {
            self.close_size = (w as f32, h as f32);
        }
        self.container.set_texture_sizes(size_fn);
    }
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, self.win_h())
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            ITEMWIN_MID_TEX,
            FOOTER_TEX,
            RESIZE_HANDLE_TEX,
            CLOSE_BTN.normal,
            CLOSE_BTN.hover,
            CLOSE_BTN.pressed,
            TAB_USABLE_TEX,
            TAB_EQUIP_TEX,
            TAB_ETC_TEX,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths.extend(NumberInputDialog::grf_texture_paths());
        paths
    }
}

impl InGameWindow for StorageWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        data: &DataTable,
    ) -> Vec<GameEvent> {
        if !character.storage.is_open() {
            self.qty_dialog = None;
            return Vec::new();
        }
        let mut events = Vec::new();
        let slot_count_table = data.item_slot_count.as_ref();
        let card_name_table = data.card_name.as_ref();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);

        if let Some((_, dialog)) = &self.qty_dialog {
            ui.set_modal(&[dialog.win_id()]);
        }

        let win_h = self.win_h();
        let win = ui.window_at(STORAGE_WINDOW_ID, WIN_W, win_h, TITLE_H, 320.0, 80.0);
        let (x, y) = (win.x, win.y);
        ui.interact(STORAGE_WINDOW_ID, Rect::new(x, y, WIN_W, win_h));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 17.0, y + TITLE_H - 3.0, "Storage", tc);

        let container_y = y + TITLE_H;
        let list_h = self.rows as f32 * ROW_H;
        draw_container(ui, x, container_y, WIN_W, list_h, grf);

        // --- Tab column (left), reusing the inventory 3-tab strip ---
        let (tab_w, tab_img_h) = self.tab_size;
        let active_tex = match self.active_tab {
            InventoryTab::Usable => TAB_USABLE_TEX,
            InventoryTab::Equip => TAB_EQUIP_TEX,
            InventoryTab::Etc => TAB_ETC_TEX,
        };
        if grf {
            let (v, i) = draw::quad_vertices(x, container_y, tab_w, tab_img_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(active_tex.to_string()),
            });
        }
        let tab_btn_h = tab_img_h / 3.0;
        for (i, (tab, _, label)) in TAB_DEFS.iter().enumerate() {
            let ty = container_y + i as f32 * tab_btn_h;
            let tab_rect = Rect::new(x, ty, tab_w, tab_btn_h);
            let resp = ui.interact(WidgetId(TAB_BASE_ID + i as u32), tab_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if !grf {
                let active = self.active_tab == *tab;
                crate::helper::fallback::cell(ui, x, ty, tab_w, tab_btn_h, active || resp.hovered());
                let tw = ui.atlas.measure_text(label);
                ui.text(x + (tab_w - tw) / 2.0, ty + tab_btn_h / 2.0 + 4.0, label, tc);
            }
            if resp.clicked() && self.active_tab != *tab {
                self.active_tab = *tab;
                self.scroll_offset = 0;
            }
        }

        // --- Item list (client-side filtered by tab) ---
        let list_x = x + tab_w;
        let list_w = WIN_W - tab_w - SCROLLBAR_W;
        let filtered: Vec<&ragnarok_game::item::Item> = character
            .storage
            .all_items()
            .iter()
            .filter(|it| it.tab() == self.active_tab)
            .collect();

        let total_rows = filtered.len();
        let max_scroll = total_rows.saturating_sub(self.rows);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        let mut drag_index: Option<(u16, Option<String>)> = None;
        let mut withdraw: Option<(u16, i16)> = None;
        for row in 0..self.rows {
            let item_idx = self.scroll_offset + row;
            let ry = container_y + row as f32 * ROW_H;
            let icon_y = ry + (ROW_H - ICON) / 2.0;

            if grf {
                let (v, i) = draw::quad_vertices(list_x + 2.0, icon_y, ICON, ICON, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(ITEMWIN_MID_TEX.to_string()),
                });
            } else {
                crate::helper::fallback::slot_cell(ui, list_x + 2.0, icon_y, ICON, ICON);
            }

            let Some(item) = filtered.get(item_idx) else {
                continue;
            };
            let row_rect = Rect::new(list_x, ry, list_w, ROW_H);
            let resp = ui.interact(WidgetId(ROW_BASE_ID + row as u32), row_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }

            if let Some(icon_path) = item.icon_path() {
                let tint = if item.is_identified {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.67, 0.67, 0.67, 1.0]
                };
                let (v, i) = draw::quad_vertices(list_x + 2.0, icon_y, ICON, ICON, tint);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(icon_path),
                });
            }

            let name = format_equipment_display_name(item, slot_count_table, card_name_table);
            let name_color = if !item.is_identified { GREY } else { tc };
            ui.text(list_x + ICON + 6.0, ry + ROW_H / 2.0 + 4.0, &name, name_color);
            if item.count > 1 {
                let count_str = item.count.to_string();
                let cw = ui.atlas.measure_text(&count_str);
                ui.text(list_x + list_w - cw - 4.0, ry + ROW_H / 2.0 + 4.0, &count_str, tc);
            }

            if resp.hovered() {
                let tooltip = if item.count > 1 {
                    format!("{name} {} ea", item.count)
                } else {
                    name.clone()
                };
                ui.tooltip(list_x, ry - 4.0, &tooltip);
            }
            if resp.clicked() {
                drag_index = Some((item.index, item.icon_path()));
            }
            if resp.right_clicked() || resp.double_clicked() {
                withdraw = Some((item.index, item.count));
            }
        }
        if let Some((index, icon)) = drag_index {
            ui.drag_source(STORAGE_WINDOW_ID, index as usize, icon, (ICON, ICON));
        }
        if let Some((index, count)) = withdraw {
            events.extend(self.begin_withdraw(character, index, count));
        }

        // --- Drop zone: deposit from inventory / cart ---
        let list_rect = Rect::new(list_x, container_y, list_w, list_h);
        if let Some((source_id, item_index)) = ui.drop_zone(list_rect) {
            let index = item_index as u16;
            if source_id == INV_WINDOW_ID {
                if let Some(it) = character.inventory.get_item(index) {
                    let count = it.count;
                    if count > 1 {
                        self.open_qty_dialog(PendingMove::DepositBody { index }, count);
                    } else {
                        events.push(GameEvent::RequestMoveItemBodyToStore { index, count: 1 });
                    }
                }
            } else if source_id == CART_WINDOW_ID {
                if let Some(it) = character.cart.get_item(index) {
                    let count = it.count;
                    if count > 1 {
                        self.open_qty_dialog(PendingMove::DepositCart { index }, count);
                    } else {
                        events.push(GameEvent::RequestMoveItemCartToStore { index, count: 1 });
                    }
                }
            }
        }

        // --- Scrollbar ---
        if max_scroll > 0 {
            let content_rect = Rect::new(x, container_y, WIN_W, list_h);
            self.scroll_offset = scrollbar::scrollbar(
                ui,
                ScrollbarIds {
                    up: SCROLL_UP_ID,
                    down: SCROLL_DOWN_ID,
                    thumb: SCROLL_THUMB_ID,
                },
                self.scroll_offset,
                self.rows,
                max_scroll,
                content_rect,
                x + WIN_W - SCROLLBAR_W,
                container_y,
                list_h,
            );
        }

        // --- Footer ---
        let footer_y = container_y + list_h;
        draw_footer(ui, x, footer_y, WIN_W, FOOTER_H, grf);
        let (cur, max) = (character.storage.cur_count, character.storage.max_count);
        let count_color = if max > 0 && cur >= max { WARN_COLOR } else { tc };
        ui.text(x + 6.0, footer_y + FOOTER_H - 9.0, &format!("{cur}/{max}"), count_color);

        let (cw, ch) = self.close_size;
        let close_rect = Rect::new(
            x + WIN_W - cw - RESIZE_SIZE - 4.0,
            footer_y + (FOOTER_H - ch) / 2.0,
            cw,
            ch,
        );
        let close = ui.button(CLOSE_BTN_ID, close_rect, &CLOSE_BTN, "Close");

        let resize_rect = Rect::new(
            x + WIN_W - RESIZE_SIZE,
            y + win_h - RESIZE_SIZE,
            RESIZE_SIZE,
            RESIZE_SIZE,
        );
        let resize = ui.resize_handle(RESIZE_ID, resize_rect);
        if resize.started {
            self.resize_start = Some(self.rows);
        }
        if resize.dragging
            && let Some(start_rows) = self.resize_start
        {
            let new_rows = (start_rows as f32 + resize.delta_y / ROW_H).round() as i32;
            self.rows = (new_rows.clamp(MIN_ROWS as i32, MAX_ROWS as i32)) as usize;
        }

        // --- Quantity dialog (modal) ---
        let dialog_was_open = self.qty_dialog.is_some();
        if let Some((kind, dialog)) = &mut self.qty_dialog {
            match dialog.build(ui) {
                NumberInputResult::Submitted => {
                    let qty = dialog.value_i16().unwrap_or(0);
                    if qty > 0 {
                        match kind {
                            PendingMove::Withdraw { index } => {
                                events.push(GameEvent::RequestMoveItemStoreToBody {
                                    index: *index,
                                    count: qty,
                                });
                            }
                            PendingMove::DepositBody { index } => {
                                events.push(GameEvent::RequestMoveItemBodyToStore {
                                    index: *index,
                                    count: qty,
                                });
                            }
                            PendingMove::DepositCart { index } => {
                                events.push(GameEvent::RequestMoveItemCartToStore {
                                    index: *index,
                                    count: qty,
                                });
                            }
                        }
                    }
                    self.qty_dialog = None;
                }
                NumberInputResult::Cancel => {
                    self.qty_dialog = None;
                }
                NumberInputResult::None => {}
            }
        }

        let esc = !dialog_was_open && ui.ctx.key_escape;
        if close.clicked() || esc {
            events.push(GameEvent::RequestCloseStorage);
            character.storage.clear();
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::enums::item::ItemType;
    use ragnarok_game::item::Item;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = Box::leak(Box::new(FontAtlas::from_embedded(14.0, 1.0)));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    fn open_storage_with_potion() -> Character {
        let mut character = Character::new();
        character.storage.open_with_pending(1, 600);
        character.storage.add_item(Item {
            index: 5,
            item_id: 501,
            item_type: ItemType::Healing,
            count: 1,
            is_identified: true,
            is_damaged: false,
            refining_level: 0,
            slot: [0; 4],
            location: 0,
            wear_state: 0,
            name: "Red Potion".into(),
            resource_name: None,
        });
        character
    }

    #[test]
    fn right_click_single_item_withdraws_and_close_notifies_server() {
        let mut win = StorageWindow::new();
        let mut character = open_storage_with_potion();
        let data = DataTable::new();
        let mut state = StateCache::new();

        // First row sits at container top (window default 320,80; tab column 28 wide).
        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 320.0 + TAB_FALLBACK_W + 20.0;
        ctx.mouse_y = 80.0 + TITLE_H + ROW_H / 2.0;
        ctx.mouse_right_clicked = true;
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut character, &data)
        };
        assert!(
            events.iter().any(|e| matches!(
                e,
                GameEvent::RequestMoveItemStoreToBody { index: 5, count: 1 }
            )),
            "expected single-count withdraw, got {events:?}"
        );

        let footer_y = 80.0 + TITLE_H + DEFAULT_ROWS as f32 * ROW_H;
        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 320.0 + WIN_W - CLOSE_W - RESIZE_SIZE - 4.0 + CLOSE_W / 2.0;
        ctx.mouse_y = footer_y + FOOTER_H / 2.0;
        ctx.mouse_clicked = true;
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut character, &data)
        };
        assert!(
            events.iter().any(|e| matches!(e, GameEvent::RequestCloseStorage)),
            "close must notify the server, got {events:?}"
        );
        assert!(!character.storage.is_open());
    }
}

impl StorageWindow {
    /// Deposit an inventory item (e.g. double-click while storage is open):
    /// stacks open the quantity dialog, singles move immediately.
    pub fn begin_deposit_body(&mut self, character: &Character, index: u16) -> Vec<GameEvent> {
        let count = character
            .inventory
            .get_item(index)
            .map(|i| i.count)
            .unwrap_or(0);
        if count <= 0 {
            return Vec::new();
        }
        if count > 1 {
            self.open_qty_dialog(PendingMove::DepositBody { index }, count);
            Vec::new()
        } else {
            vec![GameEvent::RequestMoveItemBodyToStore { index, count: 1 }]
        }
    }

    /// Withdraw would create a new inventory slot but the player is already at
    /// the 100-distinct-item cap: refuse client-side like the original game.
    fn begin_withdraw(&mut self, character: &Character, index: u16, count: i16) -> Vec<GameEvent> {
        let item_id = character
            .storage
            .get_item(index)
            .map(|i| i.item_id)
            .unwrap_or(0);
        let creates_new_slot = !character
            .inventory
            .all_items()
            .iter()
            .any(|i| i.item_id == item_id && i.item_type.is_stackable());
        if creates_new_slot && character.inventory.all_items().len() >= 100 {
            return vec![GameEvent::ShowSystemMessage {
                message: "Cannot withdraw: inventory is full.".to_string(),
            }];
        }
        if count > 1 {
            self.open_qty_dialog(PendingMove::Withdraw { index }, count);
            Vec::new()
        } else {
            vec![GameEvent::RequestMoveItemStoreToBody { index, count: 1 }]
        }
    }
}
