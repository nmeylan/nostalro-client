use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_game::npc_shop::{NpcShopData, NpcShopMode};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, RESIZE_HANDLE_TEX, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use crate::{Window, InGameWindow};
use crate::helper::dialog_container::DialogContainer;
use super::number_input::{NumberInputDialog, NumberInputConfig, NumberInputResult};
use crate::helper::window_chrome::{
    ITEMWIN_MID_TEX, TITLEBAR_TEX, FOOTER_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX,
    draw_titlebar, draw_container, draw_footer, text_color,
};

// -- Widget IDs --
pub const OVERLAY_ID: WidgetId = WidgetId(700);
pub const INPUT_WIN_ID: WidgetId = WidgetId(701);
pub const OUTPUT_WIN_ID: WidgetId = WidgetId(702);
const BUY_SELL_BTN_ID: WidgetId = WidgetId(703);
const CANCEL_BTN_ID: WidgetId = WidgetId(704);
const QTY_INPUT_ID: WidgetId = WidgetId(709);
const SCROLL_UP_ID: WidgetId = WidgetId(707);
const SCROLL_DOWN_ID: WidgetId = WidgetId(708);
const SCROLL_THUMB_ID: WidgetId = WidgetId(710);
const OUT_SCROLL_UP_ID: WidgetId = WidgetId(711);
const OUT_SCROLL_DOWN_ID: WidgetId = WidgetId(712);
const OUT_SCROLL_THUMB_ID: WidgetId = WidgetId(713);
const INPUT_RESIZE_ID: WidgetId = WidgetId(714);
const ITEM_BASE_ID: u32 = 720;
const CART_BASE_ID: u32 = 780;

// -- Layout (matches original game NpcStore) --
const WIN_W: f32 = 280.0;
const WIN_GAP: f32 = 10.0;
const TITLE_H: f32 = 17.0;
const CONTAINER_PAD_LEFT: f32 = 16.0;
const CONTAINER_PAD_RIGHT: f32 = 3.0;
const CONTAINER_PAD_Y: f32 = 5.0;
const ITEM_ROW_H: f32 = 32.0;
const FOOTER_H: f32 = 27.0;
const INPUT_MIN_ROWS: usize = 2;
const INPUT_MAX_ROWS: usize = 9;
const INPUT_DEFAULT_ROWS: usize = 7;
const OUTPUT_VISIBLE_ROWS: usize = 2;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
const ICON_SIZE: f32 = 24.0;
const ICON_OFFSET_X: f32 = 4.0;
const ICON_OFFSET_Y: f32 = 2.0;

// -- GRF button textures --
const OK_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_ok.bmp",
    hover: "data/texture/유저인터페이스/btn_ok_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_ok_b.bmp",
};
const CANCEL_BTN_TEX: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_cancel.bmp",
    hover: "data/texture/유저인터페이스/btn_cancel_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_cancel_b.bmp",
};

const RESIZE_SIZE: f32 = 13.0;

pub struct NpcShop {
    pub has_grf_textures: bool,
    pub shop: NpcShopData,
    qty_popup: Option<(usize, NumberInputDialog)>,
    btn_size: (f32, f32),
    scroll_offset: usize,
    output_scroll_offset: usize,
    input_visible_rows: usize,
    resize_start_rows: Option<usize>,
    container: DialogContainer,
}

impl Default for NpcShop {
    fn default() -> Self {
        Self::new()
    }
}

impl NpcShop {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            shop: NpcShopData::new(),
            qty_popup: None,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            scroll_offset: 0,
            output_scroll_offset: 0,
            input_visible_rows: INPUT_DEFAULT_ROWS,
            resize_start_rows: None,
            container: DialogContainer::new(),
        }
    }

}

impl Window for NpcShop {
    fn has_grf_textures(&self) -> bool { self.has_grf_textures }
    fn set_has_grf_textures(&mut self, value: bool) { self.has_grf_textures = value; }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(OK_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
        self.container.has_grf_textures = true;
        self.container.set_texture_sizes(size_fn);
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = DialogContainer::grf_texture_paths();
        paths.extend_from_slice(&[
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN_TEX.normal,
            CANCEL_BTN_TEX.hover,
            CANCEL_BTN_TEX.pressed,
            TITLEBAR_TEX,
            ITEMWIN_MID_TEX,
            FOOTER_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            RESIZE_HANDLE_TEX,
        ]);
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for NpcShop {
    fn setup_modal(&self, ui: &mut UiFrame) {
        if !self.shop.is_open() {
            return;
        }
        let mut modal_ids = vec![INPUT_WIN_ID, OUTPUT_WIN_ID];
        if let Some((_, ref dialog)) = self.qty_popup {
            modal_ids.push(dialog.win_id());
        }
        ui.set_modal(&modal_ids);
    }

    fn build(&mut self, ui: &mut UiFrame, _character: &mut Character, _data: &DataTable) -> Vec<GameEvent> {
        if !self.shop.is_open() {
            return Vec::new();
        }

        let mut events = Vec::new();

        if ui.ctx.key_escape {
            if self.qty_popup.is_some() {
                self.qty_popup = None;
                return events;
            }
            events.push(GameEvent::RequestNpcShopClose);
            return events;
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;

        // Full-screen overlay to block world clicks
        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(OVERLAY_ID, screen);

        // Compute default positions: InputWindow left, OutputWindow right
        let input_default_x = 100.0;
        let input_default_y = 100.0;
        let output_default_x = input_default_x + (WIN_W) + (WIN_GAP);

        // Input window height for output vertical alignment
        let input_item_count = match self.shop.mode {
            Some(NpcShopMode::Sell) => self.shop.visible_sell_indices().len(),
            _ => self.shop.item_count(),
        };
        let input_content_rows = self
            .input_visible_rows
            .min(input_item_count)
            .max(INPUT_MIN_ROWS);
        let input_content_h = input_content_rows as f32 * (ITEM_ROW_H);
        let input_win_h =
            (TITLE_H) + (CONTAINER_PAD_Y) + input_content_h + (CONTAINER_PAD_Y) + (FOOTER_H);
        let output_content_rows = OUTPUT_VISIBLE_ROWS.max(self.shop.cart.len().min(5)).max(2);
        let output_content_h = output_content_rows as f32 * (ITEM_ROW_H);
        let output_win_h =
            (TITLE_H) + (CONTAINER_PAD_Y) + output_content_h + (CONTAINER_PAD_Y) + (FOOTER_H);

        // Output vertically aligned: bottom-aligned with input
        let output_default_y = input_default_y + input_win_h - output_win_h;

        // Build both windows
        self.build_input_window(ui, input_default_x, input_default_y, input_win_h);
        let output_rect = self.build_output_window(
            ui,
            &mut events,
            output_default_x,
            output_default_y,
            output_win_h,
        );

        // Handle drag-drop: item dropped on output window opens qty popup
        if let Some((source_id, item_idx)) = ui.drop_zone(output_rect)
            && source_id == INPUT_WIN_ID {
                if self.shop.mode == Some(NpcShopMode::Sell)
                    && self.shop.sell_item_remaining(item_idx) <= 1
                {
                    self.shop.add_to_cart(item_idx, 1);
                } else {
                    self.open_qty_popup(item_idx);
                }
            }

        if self.qty_popup.is_some() {
            self.build_quantity_popup(ui);
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl NpcShop {
    fn build_input_window(
        &mut self,
        ui: &mut UiFrame,
        default_x: f32,
        default_y: f32,
        win_h: f32,
    ) -> Rect {
        let win_w = WIN_W;
        let title_h = TITLE_H;
        let footer_h = FOOTER_H;
        let row_h = ITEM_ROW_H;
        let pad_left = CONTAINER_PAD_LEFT;
        let pad_right = CONTAINER_PAD_RIGHT;
        let pad_y = CONTAINER_PAD_Y;
        let scrollbar_w = SCROLLBAR_W;

        let visible_indices: Vec<usize> = match self.shop.mode {
            Some(NpcShopMode::Sell) => self.shop.visible_sell_indices(),
            _ => (0..self.shop.item_count()).collect(),
        };
        let item_count = visible_indices.len();
        let visible = self.input_visible_rows.min(item_count).max(INPUT_MIN_ROWS);

        let win = ui.window_at(INPUT_WIN_ID, win_w, win_h, title_h, default_x, default_y);

        let grf = self.has_grf_textures;
        let text_color = text_color(grf);

        // Titlebar
        draw_titlebar(ui, win.x, win.y, win_w, title_h, grf);
        let title = match self.shop.mode {
            Some(NpcShopMode::Buy) => "Shop Items",
            Some(NpcShopMode::Sell) => "Available Items for selling",
            None => "",
        };
        ui.text(win.x + (17.0), win.y + title_h - (3.0), title, text_color);

        // Container
        let container_y = win.y + title_h;
        let container_h = win_h - title_h - footer_h;
        draw_container(ui, win.x, container_y, win_w, container_h, grf);

        let max_scroll = item_count.saturating_sub(self.input_visible_rows);

        // Item rows
        let list_y = container_y + pad_y;
        let icon_size = ICON_SIZE;
        let name_x = win.x + pad_left + (ICON_OFFSET_X) + icon_size + (4.0);
        let row_content_w = win_w - pad_left - pad_right - scrollbar_w;

        for i in 0..visible {
            let list_idx = self.scroll_offset + i;
            if list_idx >= item_count {
                break;
            }
            let item_idx = visible_indices[list_idx];

            let ry = list_y + i as f32 * row_h;
            let row_rect = Rect::new(win.x + pad_left, ry, row_content_w, row_h);
            let widget_id = WidgetId(ITEM_BASE_ID + i as u32);
            let response = ui.interact(widget_id, row_rect);

            // Item shadow (itemwin_mid per row)
            if grf {
                let shadow_x = win.x + pad_left + (ICON_OFFSET_X);
                let shadow_y = ry + (ICON_OFFSET_Y);
                let (v, idx) = draw::quad_vertices(
                    shadow_x,
                    shadow_y,
                    icon_size,
                    icon_size,
                    [1.0, 1.0, 1.0, 1.0],
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::Named(ITEMWIN_MID_TEX.to_string()),
                });
            }

            // Item icon
            if let Some(icon_path) = self.shop.item_icon_path(item_idx) {
                let ix = win.x + pad_left + (ICON_OFFSET_X);
                let iy = ry + (ICON_OFFSET_Y);
                let tint = if self.shop.item_is_identified(item_idx) {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.67, 0.67, 0.67, 1.0]
                };
                let (v, idx) =
                    draw::quad_vertices(ix, iy, icon_size, icon_size, tint);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::Named(icon_path),
                });
            }

            // Selection highlight
            let is_selected = self.shop.selected_index == Some(item_idx);
            if is_selected {
                let (v, idx) = draw::quad_vertices(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    [0.20, 0.42, 0.88, 0.50],
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
            }

            // Item count (sell mode — show remaining quantity)
            if self.shop.mode == Some(NpcShopMode::Sell) {
                let remaining = self.shop.sell_item_remaining(item_idx);
                let count_str = remaining.to_string();
                let count_w = ui.atlas.measure_text(&count_str);
                let count_x = win.x + pad_left + (ICON_OFFSET_X) + icon_size - count_w + 2.0;
                ui.text(count_x, ry + row_h - 2.0, &count_str, text_color);
            }

            // Item name
            let name = self.shop.item_name(item_idx);
            let text_y = ry + row_h - (8.0);
            ui.text(name_x, text_y, name, text_color);

            // Price + "Z"
            let price = self.shop.item_price(item_idx);
            let price_str = format_zeny(price);
            let z_x = win.x + win_w - pad_right - scrollbar_w - (10.0);
            let price_w = ui.atlas.measure_text(&price_str);
            let price_x = z_x - (2.0) - price_w;
            ui.text(price_x, text_y, &price_str, text_color);
            ui.text(z_x, text_y, "Z", text_color);

            if response.clicked() {
                self.shop.selected_index = Some(item_idx);
                // Begin drag tracking for this item
                let drag_icon = self.shop.item_icon_path(item_idx);
                ui.drag_source(INPUT_WIN_ID, item_idx, drag_icon, (icon_size, icon_size));
            }
            if response.double_clicked() {
                if self.shop.mode == Some(NpcShopMode::Sell)
                    && self.shop.sell_item_remaining(item_idx) <= 1
                {
                    self.shop.add_to_cart(item_idx, 1);
                } else {
                    self.open_qty_popup(item_idx);
                }
            }
        }

        // Scrollbar
        if item_count > self.input_visible_rows {
            let sb_x = win.x + win_w - scrollbar_w - (1.0);
            let content_rect = Rect::new(win.x, container_y, win_w, container_h);
            self.scroll_offset = scrollbar::scrollbar(
                ui,
                ScrollbarIds {
                    up: SCROLL_UP_ID,
                    down: SCROLL_DOWN_ID,
                    thumb: SCROLL_THUMB_ID,
                },
                self.scroll_offset,
                self.input_visible_rows,
                max_scroll,
                content_rect,
                sb_x,
                container_y,
                container_h,
            );
        }

        // Footer
        let footer_y = win.y + win_h - footer_h;
        draw_footer(ui, win.x, footer_y, win_w, footer_h, grf);

        // Resize handle (bottom-right of footer)
        let resize_rect = Rect::new(
            win.x + win_w - RESIZE_SIZE,
            footer_y + footer_h - RESIZE_SIZE,
            RESIZE_SIZE,
            RESIZE_SIZE,
        );
        let resize = ui.resize_handle(INPUT_RESIZE_ID, resize_rect);
        if resize.started {
            self.resize_start_rows = Some(self.input_visible_rows);
        }
        if resize.dragging
            && let Some(start_rows) = self.resize_start_rows {
                let new = (start_rows as f32 + resize.delta_y / row_h).round() as i32;
                self.input_visible_rows = new.clamp(INPUT_MIN_ROWS as i32, INPUT_MAX_ROWS as i32) as usize;
            }

        win
    }

    fn build_output_window(
        &mut self,
        ui: &mut UiFrame,
        events: &mut Vec<GameEvent>,
        default_x: f32,
        default_y: f32,
        win_h: f32,
    ) -> Rect {
        let win_w = WIN_W;
        let title_h = TITLE_H;
        let footer_h = FOOTER_H;
        let row_h = ITEM_ROW_H;
        let pad_left = CONTAINER_PAD_LEFT;
        let pad_right = CONTAINER_PAD_RIGHT;
        let pad_y = CONTAINER_PAD_Y;
        let scrollbar_w = SCROLLBAR_W;
        let (btn_w, btn_h) = self.btn_size;

        let win = ui.window_at(OUTPUT_WIN_ID, win_w, win_h, title_h, default_x, default_y);

        let grf = self.has_grf_textures;
        let text_color = text_color(grf);

        // Titlebar
        draw_titlebar(ui, win.x, win.y, win_w, title_h, grf);
        let title = match self.shop.mode {
            Some(NpcShopMode::Buy) => "Buying Items",
            Some(NpcShopMode::Sell) => "Selling Items",
            None => "",
        };
        ui.text(win.x + (17.0), win.y + title_h - (3.0), title, text_color);

        // Container
        let container_y = win.y + title_h;
        let container_h = win_h - title_h - footer_h;
        draw_container(ui, win.x, container_y, win_w, container_h, grf);

        // Cart items
        let list_y = container_y + pad_y;
        let icon_size = ICON_SIZE;
        let name_x = win.x + pad_left + (ICON_OFFSET_X) + icon_size + (4.0);
        let cart_count = self.shop.cart.len();
        let visible = OUTPUT_VISIBLE_ROWS.max(cart_count.min(5)).max(2);
        let has_scrollbar = cart_count > visible;
        let row_content_w =
            win_w - pad_left - pad_right - if has_scrollbar { scrollbar_w } else { 0.0 };

        let max_scroll = cart_count.saturating_sub(visible);

        for i in 0..visible {
            let ci = self.output_scroll_offset + i;
            if ci >= cart_count {
                break;
            }
            let cart_item = &self.shop.cart[ci];
            let ry = list_y + i as f32 * row_h;
            let row_rect = Rect::new(win.x + pad_left, ry, row_content_w, row_h);
            let widget_id = WidgetId(CART_BASE_ID + i as u32);
            let response = ui.interact(widget_id, row_rect);
            if response.hovered() {
                ui.any_interactive_hovered = true;
            }

            // Item shadow (itemwin_mid per row)
            if grf {
                let shadow_x = win.x + pad_left + (ICON_OFFSET_X);
                let shadow_y = ry + (ICON_OFFSET_Y);
                let (v, idx) = draw::quad_vertices(
                    shadow_x,
                    shadow_y,
                    icon_size,
                    icon_size,
                    [1.0, 1.0, 1.0, 1.0],
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::Named(ITEMWIN_MID_TEX.to_string()),
                });
            }

            // Item icon
            if let Some(icon_path) = self.shop.item_icon_path(cart_item.source_index) {
                let ix = win.x + pad_left + (ICON_OFFSET_X);
                let iy = ry + (ICON_OFFSET_Y);
                let tint = if self.shop.item_is_identified(cart_item.source_index) {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.67, 0.67, 0.67, 1.0]
                };
                let (v, idx) =
                    draw::quad_vertices(ix, iy, icon_size, icon_size, tint);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::Named(icon_path),
                });
            }

            if response.hovered() {
                let (v, idx) = draw::quad_vertices(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    [0.88, 0.20, 0.20, 0.15],
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
            }

            let text_y = ry + row_h - (8.0);

            // Quantity badge (overlaid on icon, bottom-right)
            let qty_str = cart_item.quantity.to_string();
            let qty_w = ui.atlas.measure_text(&qty_str);
            let qty_x = win.x + pad_left + (ICON_OFFSET_X) + icon_size - qty_w;
            ui.text(qty_x, ry + icon_size, &qty_str, text_color);

            // Item name
            let name = self.shop.item_name(cart_item.source_index);
            ui.text(name_x, text_y, name, text_color);

            // Subtotal + "Z"
            let price = self.shop.item_price(cart_item.source_index);
            let subtotal = price as i64 * cart_item.quantity as i64;
            let price_str = format_zeny(subtotal as i32);
            let z_x =
                win.x + win_w - pad_right - if has_scrollbar { scrollbar_w } else { 0.0 } - (10.0);
            let price_w = ui.atlas.measure_text(&price_str);
            let price_x = z_x - (2.0) - price_w;
            ui.text(price_x, text_y, &price_str, text_color);
            ui.text(z_x, text_y, "Z", text_color);

            // Click on cart item removes it
            if response.clicked() {
                self.shop.remove_from_cart(ci);
                return win;
            }
        }

        // Scrollbar
        if has_scrollbar {
            let sb_x = win.x + win_w - scrollbar_w - (1.0);
            let content_rect = Rect::new(win.x, container_y, win_w, container_h);
            self.output_scroll_offset = scrollbar::scrollbar(
                ui,
                ScrollbarIds {
                    up: OUT_SCROLL_UP_ID,
                    down: OUT_SCROLL_DOWN_ID,
                    thumb: OUT_SCROLL_THUMB_ID,
                },
                self.output_scroll_offset,
                visible,
                max_scroll,
                content_rect,
                sb_x,
                container_y,
                container_h,
            );
        }

        // Footer
        let footer_y = win.y + win_h - footer_h;
        draw_footer(ui, win.x, footer_y, win_w, footer_h, grf);

        // Total display
        let total = self.shop.cart_total();
        let total_label = format!("Total : {} Zeny", format_zeny(total as i32));
        ui.text(
            win.x + (10.0),
            footer_y + footer_h - (10.0),
            &total_label,
            text_color,
        );

        // Buy/Sell + Cancel buttons
        let btn_y = footer_y + (4.0);
        let cancel_x = win.x + win_w - (15.0) - btn_w;
        let action_x = cancel_x - (5.0) - btn_w;

        let action_label = match self.shop.mode {
            Some(NpcShopMode::Buy) => "Buy",
            Some(NpcShopMode::Sell) => "Sell",
            None => "",
        };
        let action_btn = ui.button(
            BUY_SELL_BTN_ID,
            Rect::new(action_x, btn_y, btn_w, btn_h),
            &OK_BTN,
            action_label,
        );
        let cancel_btn = ui.button(
            CANCEL_BTN_ID,
            Rect::new(cancel_x, btn_y, btn_w, btn_h),
            &CANCEL_BTN_TEX,
            "Cancel",
        );

        if action_btn.clicked() && !self.shop.cart.is_empty() {
            match self.shop.mode {
                Some(NpcShopMode::Buy) => {
                    let items: Vec<(i16, u16)> = self
                        .shop
                        .cart
                        .iter()
                        .map(|c| {
                            let item_id = self.shop.buy_items[c.source_index].item.item_id;
                            (c.quantity, item_id)
                        })
                        .collect();
                    events.push(GameEvent::RequestNpcShopBuy { items });
                    self.close();
                }
                Some(NpcShopMode::Sell) => {
                    let items: Vec<(i16, i16)> = self
                        .shop
                        .cart
                        .iter()
                        .map(|c| {
                            let index = self.shop.sell_items[c.source_index].item.index as i16;
                            (index, c.quantity)
                        })
                        .collect();
                    events.push(GameEvent::RequestNpcShopSell { items });
                    self.close();
                }
                None => {}
            }
        }

        if cancel_btn.clicked() {
            events.push(GameEvent::RequestNpcShopClose);
        }

        win
    }

    fn open_qty_popup(&mut self, item_idx: usize) {
        let name = self.shop.item_name(item_idx);
        let price = self.shop.item_price(item_idx);
        let label = format!("{} ({}z)", name, format_zeny(price));
        let mut dialog = NumberInputDialog::new(
            NumberInputConfig {
                label: Some(label),
                show_cancel: false,
                escape_cancels: false,
                default_value: "1".to_string(),
                max_len: 6,
            },
            WidgetId(QTY_INPUT_ID.0),
        );
        dialog.init_container(&self.container);
        self.qty_popup = Some((item_idx, dialog));
    }

    fn build_quantity_popup(&mut self, ui: &mut UiFrame) {
        let (item_idx, dialog) = self.qty_popup.as_mut().unwrap();
        let item_idx = *item_idx;
        dialog.init_container(&self.container);

        match dialog.build(ui) {
            NumberInputResult::Submitted => {
                let qty: i16 = dialog.value_i16().unwrap_or(0);
                if qty > 0 {
                    self.shop.add_to_cart(item_idx, qty);
                }
                self.qty_popup = None;
            }
            NumberInputResult::Cancel => {
                self.qty_popup = None;
            }
            NumberInputResult::None => {}
        }
    }

    pub fn close(&mut self) {
        self.shop.close();
        self.qty_popup = None;
        self.scroll_offset = 0;
        self.output_scroll_offset = 0;
        self.input_visible_rows = INPUT_DEFAULT_ROWS;
        self.resize_start_rows = None;
    }

}

fn format_zeny(amount: i32) -> String {
    if amount < 0 {
        return format!("-{}", format_zeny(-amount));
    }
    let s = amount.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InGameWindow;
    use models::enums::item::ItemType;
    use models::enums::EnumWithNumberValue;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_game::item::Item;
    use ragnarok_game::npc_shop::ShopBuyItem;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn escape_closes_shop() {
        let mut shop_ui = NpcShop::new();
        shop_ui.shop.open_buy(
            100,
            vec![ShopBuyItem {
                item: Item {
                    index: 0, item_id: 501, item_type: ItemType::from_value(0), count: 1,
                    is_identified: true, is_damaged: false, refining_level: 0,
                    slot: [0; 4], location: 0, wear_state: 0,
                    name: "Red Potion".into(), resource_name: None,
                },
                price: 50,
                discount_price: 50,
            }],
        );

        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);

        let events = shop_ui.build(&mut ui, &mut character, &data);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], GameEvent::RequestNpcShopClose));
        // Shop remains open — main.rs closes it after sending the network packet
        assert!(shop_ui.shop.is_open());
    }

    #[test]
    fn escape_closes_qty_popup_first() {
        let mut shop_ui = NpcShop::new();
        shop_ui.shop.open_buy(
            100,
            vec![ShopBuyItem {
                item: Item {
                    index: 0, item_id: 501, item_type: ItemType::from_value(0), count: 1,
                    is_identified: true, is_damaged: false, refining_level: 0,
                    slot: [0; 4], location: 0, wear_state: 0,
                    name: "Red Potion".into(), resource_name: None,
                },
                price: 50,
                discount_price: 50,
            }],
        );
        shop_ui.open_qty_popup(0);

        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);

        let events = shop_ui.build(&mut ui, &mut character, &data);
        assert!(events.is_empty());
        assert!(shop_ui.qty_popup.is_none());
        assert!(shop_ui.shop.is_open());
    }

    #[test]
    fn closed_shop_returns_no_events() {
        let mut shop_ui = NpcShop::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);

        let events = shop_ui.build(&mut ui, &mut character, &data);
        assert!(events.is_empty());
    }

    #[test]
    fn format_zeny_with_commas() {
        assert_eq!(format_zeny(0), "0");
        assert_eq!(format_zeny(999), "999");
        assert_eq!(format_zeny(1000), "1,000");
        assert_eq!(format_zeny(1234567), "1,234,567");
        assert_eq!(format_zeny(-500), "-500");
    }
}
