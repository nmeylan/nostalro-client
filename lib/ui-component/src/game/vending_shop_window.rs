use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_titlebar,
    text_color,
};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::{GameEvent, VendorItem};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const VENDING_SHOP_WINDOW_ID: WidgetId = WidgetId(2400);
const BUY_ID: WidgetId = WidgetId(2401);
const CANCEL_ID: WidgetId = WidgetId(2402);
const QTY_MINUS_ID: WidgetId = WidgetId(2403);
const QTY_PLUS_ID: WidgetId = WidgetId(2404);
const SCROLL_UP_ID: WidgetId = WidgetId(2405);
const SCROLL_DOWN_ID: WidgetId = WidgetId(2406);
const SCROLL_THUMB_ID: WidgetId = WidgetId(2407);
const ROW_BASE_ID: u32 = 2410;

const OK_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_ok.bmp",
    hover: "data/texture/유저인터페이스/btn_ok_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_ok_b.bmp",
};
const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_cancel.bmp",
    hover: "data/texture/유저인터페이스/btn_cancel_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_cancel_b.bmp",
};

const WIN_W: f32 = 280.0;
const TITLE_H: f32 = 17.0;
const ROW_H: f32 = 22.0;
const ICON_SIZE: f32 = 18.0;
const PAD: f32 = 6.0;
const QTY_AREA_H: f32 = 22.0;
const FOOTER_H: f32 = 28.0;
const MAX_VISIBLE_ROWS: usize = 8;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;
const SELECTED_COLOR: [f32; 4] = [0.3, 0.3, 0.5, 0.5];
const STEP_SIZE: f32 = 16.0;

#[derive(Clone)]
struct ShopRow {
    item: VendorItem,
    name: String,
    icon: Option<String>,
}

#[derive(Default)]
pub struct VendingShopWindow {
    has_grf_textures: bool,
    open: bool,
    aid: u32,
    unique_id: u32,
    rows: Vec<ShopRow>,
    selected: usize,
    quantity: u16,
    scroll_offset: usize,
    btn_size: (f32, f32),
}

impl VendingShopWindow {
    pub fn new() -> Self {
        Self {
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            quantity: 1,
            ..Default::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, aid: u32, unique_id: u32, rows: Vec<(VendorItem, String, Option<String>)>) {
        self.aid = aid;
        self.unique_id = unique_id;
        self.rows = rows
            .into_iter()
            .map(|(item, name, icon)| ShopRow { item, name, icon })
            .collect();
        self.selected = 0;
        self.quantity = 1;
        self.scroll_offset = 0;
        self.open = !self.rows.is_empty();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.rows.clear();
    }

    pub fn update_stock(&mut self, index: i16, amount: i16) {
        let Some(pos) = self.rows.iter().position(|r| r.item.index == index) else {
            return;
        };
        if amount <= 0 {
            self.rows.remove(pos);
            self.selected = self.selected.min(self.rows.len().saturating_sub(1));
            if self.rows.is_empty() {
                self.open = false;
            }
        } else {
            self.rows[pos].item.amount = amount;
        }
        self.quantity = self.quantity.min(self.selected_stock().max(1));
    }

    fn selected_stock(&self) -> u16 {
        self.rows
            .get(self.selected)
            .map(|r| r.item.amount.max(0) as u16)
            .unwrap_or(0)
    }
}

impl Window for VendingShopWindow {
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
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for VendingShopWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        _character: &mut Character,
        _data: &DataTable,
    ) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let (btn_w, btn_h) = self.btn_size;

        let visible_rows = self.rows.len().min(MAX_VISIBLE_ROWS);
        let max_scroll = self.rows.len().saturating_sub(visible_rows);
        let list_h = visible_rows as f32 * ROW_H;
        let win_h = TITLE_H + PAD + list_h + PAD + QTY_AREA_H + FOOTER_H;

        let win = ui.window_at(VENDING_SHOP_WINDOW_ID, WIN_W, win_h, TITLE_H, 300.0, 120.0);
        let (dx, dy) = (win.x, win.y);
        ui.interact(VENDING_SHOP_WINDOW_ID, Rect::new(dx, dy, WIN_W, win_h));

        draw_titlebar(ui, dx, dy, WIN_W, TITLE_H, grf);
        ui.text(dx + 17.0, dy + TITLE_H - 3.0, "Vending", tc);

        let body_y = dy + TITLE_H;
        let body_h = PAD + list_h + PAD + QTY_AREA_H;
        draw_container(ui, dx, body_y, WIN_W, body_h, grf);
        draw_footer(ui, dx, dy + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        let has_scroll = max_scroll > 0;
        let list_x = dx + PAD;
        let list_y = body_y + PAD;
        let list_w = WIN_W - PAD * 2.0 - if has_scroll { SCROLLBAR_W } else { 0.0 };

        let content_rect = Rect::new(list_x, list_y, WIN_W - PAD * 2.0, list_h);
        if has_scroll {
            self.scroll_offset = scrollbar::scrollbar(
                ui,
                ScrollbarIds {
                    up: SCROLL_UP_ID,
                    down: SCROLL_DOWN_ID,
                    thumb: SCROLL_THUMB_ID,
                },
                self.scroll_offset,
                visible_rows,
                max_scroll,
                content_rect,
                dx + WIN_W - PAD - SCROLLBAR_W,
                list_y,
                list_h,
            );
        } else {
            self.scroll_offset = 0;
        }

        let mut clicked = None;
        for vis in 0..visible_rows {
            let idx = self.scroll_offset + vis;
            let Some(row) = self.rows.get(idx) else {
                break;
            };
            let row_y = list_y + vis as f32 * ROW_H;
            let row_rect = Rect::new(list_x, row_y, list_w, ROW_H);
            let resp = ui.interact(WidgetId(ROW_BASE_ID + vis as u32), row_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                clicked = Some(idx);
            }
            if idx == self.selected {
                let (v, i) = draw::quad_vertices(
                    row_rect.x,
                    row_rect.y,
                    row_rect.w,
                    row_rect.h,
                    SELECTED_COLOR,
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
            let mut text_x = row_rect.x + 2.0;
            if let Some(icon) = &row.icon {
                let iy = row_y + (ROW_H - ICON_SIZE) / 2.0;
                let (v, i) = draw::quad_vertices(text_x, iy, ICON_SIZE, ICON_SIZE, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(icon.clone()),
                });
                text_x += ICON_SIZE + 4.0;
            }
            let label = format!(
                "{}  {}z  x{}",
                row.name, row.item.price, row.item.amount
            );
            ui.text(text_x, row_y + ROW_H - 6.0, &label, tc);
        }
        if let Some(idx) = clicked {
            self.selected = idx;
            self.quantity = 1;
        }

        // Quantity stepper.
        let qty_y = list_y + list_h + PAD;
        let stock = self.selected_stock();
        let minus_rect = Rect::new(list_x, qty_y, STEP_SIZE, STEP_SIZE);
        let plus_rect = Rect::new(list_x + STEP_SIZE + 44.0, qty_y, STEP_SIZE, STEP_SIZE);
        let minus = ui.interact(QTY_MINUS_ID, minus_rect);
        let plus = ui.interact(QTY_PLUS_ID, plus_rect);
        if minus.hovered() || plus.hovered() {
            ui.any_interactive_hovered = true;
        }
        if minus.clicked() && self.quantity > 1 {
            self.quantity -= 1;
        }
        if plus.clicked() && self.quantity < stock {
            self.quantity += 1;
        }
        self.quantity = self.quantity.clamp(1, stock.max(1));
        for (r, ch) in [(minus_rect, "-"), (plus_rect, "+")] {
            let (v, i) = draw::quad_vertices(r.x, r.y, r.w, r.h, [0.25, 0.25, 0.32, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
            ui.text(r.x + 5.0, r.y + STEP_SIZE - 4.0, ch, tc);
        }
        ui.text(
            list_x + STEP_SIZE + 12.0,
            qty_y + STEP_SIZE - 4.0,
            &self.quantity.to_string(),
            tc,
        );

        let win_rect = Rect::new(dx, dy, WIN_W, win_h);
        let btns = win_rect.buttons_bottom_right(2, btn_w, btn_h, 5.0, 5.0, 3.0);
        let cancel = ui.button(CANCEL_ID, btns[0], &CANCEL_BTN, "Cancel");
        let buy = ui.button(BUY_ID, btns[1], &OK_BTN, "Buy");
        if buy.clicked() {
            if let Some(row) = self.rows.get(self.selected) {
                events.push(GameEvent::RequestPurchaseFromVendor {
                    aid: self.aid,
                    unique_id: self.unique_id,
                    items: vec![(self.quantity as i16, row.item.index)],
                });
            }
            self.close();
        } else if cancel.clicked() {
            self.close();
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn vendor_item(index: i16, amount: i16, price: i32) -> VendorItem {
        VendorItem {
            index,
            item_id: 501,
            amount,
            price,
            refine: 0,
            is_identified: true,
            is_damaged: false,
            item_type: 0,
        }
    }

    #[test]
    fn open_populates_and_clamps_quantity_to_stock() {
        let mut win = VendingShopWindow::new();
        win.open(
            42,
            7,
            vec![(vendor_item(3, 5, 100), "Red Potion".into(), None)],
        );
        assert!(win.is_open());

        let mut character = Character::new();
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);
        win.quantity = 99;
        let _ = win.build(&mut ui, &mut character, &DataTable::new());
        assert_eq!(win.quantity, 5);
    }
}
