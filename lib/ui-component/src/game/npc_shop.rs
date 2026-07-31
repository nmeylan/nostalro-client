use super::input_dialog::{InputDialog, InputDialogConfig, InputDialogResult};
use super::item_info_window::ITEM_INFO_WINDOW_ID;
use crate::helper::dialog_container::DialogContainer;
use crate::helper::format::format_thousands;
use crate::helper::window_chrome::{
    FOOTER_TEX, ITEMWIN_MID_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container,
    draw_footer, draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_game::npc_shop::{NpcShopData, NpcShopMode};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, RESIZE_HANDLE_TEX, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

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
const BASKET_BASE_ID: u32 = 780;

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
    qty_popup: Option<(usize, InputDialog)>,
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
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

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
    fn owns_keyboard(&self, _ctx: &BuildCtx) -> bool {
        self.shop.is_open()
    }

    fn setup_modal(&self, ui: &mut UiFrame) {
        if !self.shop.is_open() {
            return;
        }
        let mut modal_ids = vec![INPUT_WIN_ID, OUTPUT_WIN_ID, ITEM_INFO_WINDOW_ID];
        if let Some((_, ref dialog)) = self.qty_popup {
            modal_ids.push(dialog.win_id());
        }
        ui.set_modal(&modal_ids);
    }

    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.shop.is_open()
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        if self.qty_popup.is_some() {
            self.qty_popup = None;
            return Vec::new();
        }
        vec![GameEvent::RequestNpcShopClose]
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.shop.is_open() {
            return Vec::new();
        }

        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;

        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(OVERLAY_ID, screen);

        let input_default_x = 100.0;
        let input_default_y = 100.0;
        let output_default_x = input_default_x + (WIN_W) + (WIN_GAP);

        let (input_win_h, output_win_h) = self.window_heights();

        let output_default_y = input_default_y + input_win_h - output_win_h;

        self.build_input_window(
            ui,
            &mut events,
            input_default_x,
            input_default_y,
            input_win_h,
        );
        let output_rect = self.build_output_window(
            ui,
            &mut events,
            output_default_x,
            output_default_y,
            output_win_h,
        );

        if let Some((source_id, item_idx)) = ui.drop_zone(output_rect)
            && source_id == INPUT_WIN_ID
        {
            if self.shop.needs_quantity_prompt(item_idx) {
                self.open_qty_popup(item_idx);
            } else {
                self.shop.add_to_basket(item_idx, 1);
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
    fn window_heights(&self) -> (f32, f32) {
        let input_item_count = match self.shop.mode {
            Some(NpcShopMode::Sell) => self.shop.visible_sell_indices().len(),
            _ => self.shop.item_count(),
        };
        let input_rows = self
            .input_visible_rows
            .min(input_item_count)
            .max(INPUT_MIN_ROWS);
        let input_h =
            TITLE_H + CONTAINER_PAD_Y + input_rows as f32 * ITEM_ROW_H + CONTAINER_PAD_Y + FOOTER_H;
        let output_rows = OUTPUT_VISIBLE_ROWS
            .max(self.shop.basket.len().min(5))
            .max(2);
        let output_h = TITLE_H
            + CONTAINER_PAD_Y
            + output_rows as f32 * ITEM_ROW_H
            + CONTAINER_PAD_Y
            + FOOTER_H;
        (input_h, output_h)
    }

    /// The input (item list) and output (basket) windows with their current
    /// sizes, for a gallery/packer to lay out. Empty while the shop is closed.
    pub fn gallery_windows(&self) -> Vec<(WidgetId, (f32, f32))> {
        if !self.shop.is_open() {
            return Vec::new();
        }
        let (input_h, output_h) = self.window_heights();
        vec![
            (INPUT_WIN_ID, (WIN_W, input_h)),
            (OUTPUT_WIN_ID, (WIN_W, output_h)),
        ]
    }

    fn build_input_window(
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

        let visible_indices: Vec<usize> = match self.shop.mode {
            Some(NpcShopMode::Sell) => self.shop.visible_sell_indices(),
            _ => (0..self.shop.item_count()).collect(),
        };
        let item_count = visible_indices.len();
        let visible = self.input_visible_rows.min(item_count).max(INPUT_MIN_ROWS);

        let win = ui.window_at(INPUT_WIN_ID, win_w, win_h, title_h, default_x, default_y);

        let grf = self.has_grf_textures;
        let text_color = text_color(grf);

        draw_titlebar(ui, win.x, win.y, win_w, title_h, grf);
        let title = match self.shop.mode {
            Some(NpcShopMode::Buy) => "Shop Items",
            Some(NpcShopMode::Sell) => "Available Items for selling",
            None => "",
        };
        ui.text(win.x + (17.0), win.y + title_h - (3.0), title, text_color);

        let container_y = win.y + title_h;
        let container_h = win_h - title_h - footer_h;
        draw_container(ui, win.x, container_y, win_w, container_h, grf);

        let max_scroll = item_count.saturating_sub(self.input_visible_rows);

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

            if let Some(icon_path) = self.shop.item_icon_path(item_idx) {
                let ix = win.x + pad_left + (ICON_OFFSET_X);
                let iy = ry + (ICON_OFFSET_Y);
                let tint = if self.shop.item_is_identified(item_idx) {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.67, 0.67, 0.67, 1.0]
                };
                let (v, idx) = draw::quad_vertices(ix, iy, icon_size, icon_size, tint);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::Named(icon_path),
                });
            }

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

            if self.shop.mode == Some(NpcShopMode::Sell) {
                let remaining = self.shop.sell_item_remaining(item_idx);
                let count_str = remaining.to_string();
                let count_w = ui.atlas.measure_text(&count_str);
                let count_x = win.x + pad_left + (ICON_OFFSET_X) + icon_size - count_w + 2.0;
                ui.text(count_x, ry + row_h - 2.0, &count_str, text_color);
            }

            let name = self.shop.item_name(item_idx);
            let text_y = ry + row_h - (8.0);
            ui.text(name_x, text_y, name, text_color);

            let price_right = win.x + win_w - pad_right - scrollbar_w - (8.0);
            draw_shop_price(
                ui,
                price_right,
                text_y,
                self.shop.item_base_price(item_idx),
                self.shop.item_price(item_idx),
                text_color,
            );

            if response.clicked() {
                self.shop.selected_index = Some(item_idx);
                let drag_icon = self.shop.item_icon_path(item_idx);
                ui.drag_source(INPUT_WIN_ID, item_idx, drag_icon, (icon_size, icon_size));
            }
            if response.double_clicked() {
                if self.shop.needs_quantity_prompt(item_idx) {
                    self.open_qty_popup(item_idx);
                } else {
                    self.shop.add_to_basket(item_idx, 1);
                }
            }
            if response.right_clicked()
                && let Some(item) = self.shop.item_at(item_idx)
            {
                events.push(GameEvent::ShowItemInfoDirect {
                    item: Box::new(item.clone()),
                });
            }
        }

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

        let footer_y = win.y + win_h - footer_h;
        draw_footer(ui, win.x, footer_y, win_w, footer_h, grf);

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
            && let Some(start_rows) = self.resize_start_rows
        {
            let new = (start_rows as f32 + resize.delta_y / row_h).round() as i32;
            self.input_visible_rows =
                new.clamp(INPUT_MIN_ROWS as i32, INPUT_MAX_ROWS as i32) as usize;
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

        draw_titlebar(ui, win.x, win.y, win_w, title_h, grf);
        let title = match self.shop.mode {
            Some(NpcShopMode::Buy) => "Buying Items",
            Some(NpcShopMode::Sell) => "Selling Items",
            None => "",
        };
        ui.text(win.x + (17.0), win.y + title_h - (3.0), title, text_color);

        let container_y = win.y + title_h;
        let container_h = win_h - title_h - footer_h;
        draw_container(ui, win.x, container_y, win_w, container_h, grf);

        let list_y = container_y + pad_y;
        let icon_size = ICON_SIZE;
        let name_x = win.x + pad_left + (ICON_OFFSET_X) + icon_size + (4.0);
        let basket_count = self.shop.basket.len();
        let visible = OUTPUT_VISIBLE_ROWS.max(basket_count.min(5)).max(2);
        let has_scrollbar = basket_count > visible;
        let row_content_w =
            win_w - pad_left - pad_right - if has_scrollbar { scrollbar_w } else { 0.0 };

        let max_scroll = basket_count.saturating_sub(visible);

        for i in 0..visible {
            let ci = self.output_scroll_offset + i;
            if ci >= basket_count {
                break;
            }
            let basket_item = &self.shop.basket[ci];
            let ry = list_y + i as f32 * row_h;
            let row_rect = Rect::new(win.x + pad_left, ry, row_content_w, row_h);
            let widget_id = WidgetId(BASKET_BASE_ID + i as u32);
            let response = ui.interact(widget_id, row_rect);
            if response.hovered() {
                ui.any_interactive_hovered = true;
            }

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

            if let Some(icon_path) = self.shop.item_icon_path(basket_item.source_index) {
                let ix = win.x + pad_left + (ICON_OFFSET_X);
                let iy = ry + (ICON_OFFSET_Y);
                let tint = if self.shop.item_is_identified(basket_item.source_index) {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.67, 0.67, 0.67, 1.0]
                };
                let (v, idx) = draw::quad_vertices(ix, iy, icon_size, icon_size, tint);
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

            let qty_str = basket_item.quantity.to_string();
            let qty_w = ui.atlas.measure_text(&qty_str);
            let qty_x = win.x + pad_left + (ICON_OFFSET_X) + icon_size - qty_w;
            ui.text(qty_x, ry + icon_size, &qty_str, text_color);

            let name = self.shop.item_name(basket_item.source_index);
            ui.text(name_x, text_y, name, text_color);

            let price = self.shop.item_price(basket_item.source_index);
            let subtotal = price as i64 * basket_item.quantity as i64;
            let price_str = format_zeny(subtotal as i32);
            let z_x =
                win.x + win_w - pad_right - if has_scrollbar { scrollbar_w } else { 0.0 } - (10.0);
            let price_w = ui.atlas.measure_text(&price_str);
            let price_x = z_x - (2.0) - price_w;
            ui.text(price_x, text_y, &price_str, text_color);
            ui.text(z_x, text_y, "Z", text_color);

            if response.right_clicked()
                && let Some(item) = self.shop.item_at(basket_item.source_index)
            {
                events.push(GameEvent::ShowItemInfoDirect {
                    item: Box::new(item.clone()),
                });
            }

            if response.clicked() {
                self.shop.remove_from_basket(ci);
                return win;
            }
        }

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

        let footer_y = win.y + win_h - footer_h;
        draw_footer(ui, win.x, footer_y, win_w, footer_h, grf);

        let total = self.shop.basket_total();
        let total_label = format!("Total : {} Zeny", format_thousands(total));
        ui.text(
            win.x + (10.0),
            footer_y + footer_h - (10.0),
            &total_label,
            text_color,
        );

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

        if action_btn.clicked() && !self.shop.basket.is_empty() {
            match self.shop.mode {
                Some(NpcShopMode::Buy) => {
                    let items: Vec<(i16, u16)> = self
                        .shop
                        .basket
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
                        .basket
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
        let label = format!("{} ({}z)", name, format_thousands(price as i64));
        let default_qty = match self.shop.mode {
            Some(NpcShopMode::Sell) => self.shop.sell_item_remaining(item_idx).max(1),
            _ => 1,
        };
        let mut dialog = InputDialog::new(
            InputDialogConfig {
                label: Some(label),
                show_cancel: false,
                escape_cancels: false,
                default_value: default_qty.to_string(),
                max_len: 6,
                numeric_only: true,
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
            InputDialogResult::Submitted => {
                let qty: i16 = dialog.value_i16().unwrap_or(0);
                if qty > 0 {
                    self.shop.add_to_basket(item_idx, qty);
                }
                self.qty_popup = None;
            }
            InputDialogResult::Cancel => {
                self.qty_popup = None;
            }
            InputDialogResult::None => {}
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

/// Right-aligned shop price. When `base` differs from `final_` (Discount/Overcharge
/// skill in play) it renders `base -> final_`, the base in `muted` and the final in the
/// digit-bucket price color.
fn draw_shop_price(
    ui: &mut UiFrame,
    right_x: f32,
    y: f32,
    base: i32,
    final_: i32,
    muted: [f32; 4],
) {
    let final_str = format!("{} Z", format_thousands(final_ as i64));
    if base != final_ {
        let base_str = format!("{} -> {}", format_thousands(base as i64), final_str);
        let base_w = ui.atlas.measure_text(&base_str);
        ui.text(right_x - base_w, y, &base_str, muted);
    } else {
        let x = right_x - ui.atlas.measure_text(&final_str);
        ui.text(x, y, &final_str, muted);
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
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_game::item::Item;
    use ragnarok_game::npc_shop::{ShopBuyItem, ShopSellItem};
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn escape_closes_shop() {
        let mut shop_ui = NpcShop::new();
        shop_ui.shop.open_buy(
            100,
            vec![ShopBuyItem {
                item: Item {
                    index: 0,
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
                },
                price: 50,
                discount_price: 50,
            }],
        );

        let mut character = Character::new();
        let data = DataTable::new();
        let mut ctx = crate::BuildCtx::test(&mut character, &data);

        assert!(shop_ui.wants_escape(&ctx));
        let events = shop_ui.on_escape(&mut ctx);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], GameEvent::RequestNpcShopClose));
        assert!(shop_ui.shop.is_open());
    }

    #[test]
    fn sell_qty_popup_defaults_to_remaining_stack() {
        let mut shop_ui = NpcShop::new();
        shop_ui.shop.open_sell(
            100,
            vec![ShopSellItem {
                item: Item {
                    index: 3,
                    item_id: 501,
                    item_type: ItemType::Healing,
                    count: 16,
                    is_identified: true,
                    is_damaged: false,
                    refining_level: 0,
                    slot: [0; 4],
                    location: 0,
                    wear_state: 0,
                    name: "Red Potion".into(),
                    resource_name: None,
                },
                price: 25,
                overcharge_price: 25,
            }],
        );

        shop_ui.open_qty_popup(0);
        assert_eq!(shop_ui.qty_popup.as_ref().unwrap().1.value_i16(), Some(16));

        shop_ui.qty_popup = None;
        shop_ui.shop.add_to_basket(0, 6);
        shop_ui.open_qty_popup(0);
        assert_eq!(
            shop_ui.qty_popup.as_ref().unwrap().1.value_i16(),
            Some(10),
            "default must follow what is left after the basket"
        );
    }

    #[test]
    fn escape_closes_qty_popup_first() {
        let mut shop_ui = NpcShop::new();
        shop_ui.shop.open_buy(
            100,
            vec![ShopBuyItem {
                item: Item {
                    index: 0,
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
                },
                price: 50,
                discount_price: 50,
            }],
        );
        shop_ui.open_qty_popup(0);

        let mut character = Character::new();
        let data = DataTable::new();
        let mut ctx = crate::BuildCtx::test(&mut character, &data);

        let events = shop_ui.on_escape(&mut ctx);
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

        let events = shop_ui.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(events.is_empty());
    }

    #[test]
    fn right_click_shop_item_shows_info() {
        let mut shop_ui = NpcShop::new();
        shop_ui.shop.open_buy(
            100,
            vec![ShopBuyItem {
                item: Item {
                    index: 0,
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
                },
                price: 50,
                discount_price: 50,
            }],
        );

        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 150.0;
        ctx.mouse_y = 138.0;
        ctx.mouse_right_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);

        let events = shop_ui.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::ShowItemInfoDirect { item } if item.item_id == 501
        )));
    }
}
