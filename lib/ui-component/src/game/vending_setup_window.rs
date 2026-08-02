use crate::helper::format::format_thousands;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    ITEMWIN_MID_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer,
    draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const VENDING_SETUP_WINDOW_ID: WidgetId = WidgetId(2300);
const OK_ID: WidgetId = WidgetId(2301);
const CANCEL_ID: WidgetId = WidgetId(2302);
const TITLE_INPUT_ID: WidgetId = WidgetId(2303);
const SCROLL_UP_ID: WidgetId = WidgetId(2304);
const SCROLL_DOWN_ID: WidgetId = WidgetId(2305);
const SCROLL_THUMB_ID: WidgetId = WidgetId(2306);
const RESET_ID: WidgetId = WidgetId(2307);
const PRICE_BASE_ID: u32 = 2320;
const ROW_DRAG_BASE_ID: u32 = 2340;

const OK_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_OK,
    hover: ragnarok_resources::ui::BTN_OK_A,
    pressed: ragnarok_resources::ui::BTN_OK_B,
};
const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_CANCEL,
    hover: ragnarok_resources::ui::BTN_CANCEL_A,
    pressed: ragnarok_resources::ui::BTN_CANCEL_B,
};
const RESET_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::basic::seekparty::BTN_CLEAR_A,
    hover: ragnarok_resources::ui::basic::seekparty::BTN_CLEAR_B,
    pressed: ragnarok_resources::ui::basic::seekparty::BTN_CLEAR_C,
};
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

pub const VENDING_AVAILABLE_WINDOW_ID: WidgetId = WidgetId(4900);
const AVAIL_SCROLL_UP_ID: WidgetId = WidgetId(4901);
const AVAIL_SCROLL_DOWN_ID: WidgetId = WidgetId(4902);
const AVAIL_SCROLL_THUMB_ID: WidgetId = WidgetId(4903);
const AVAIL_CELL_BASE_ID: u32 = 4920;

// "Open a Shop" (staging list) layout.
const WIN_W: f32 = 440.0;
const TITLE_H: f32 = 17.0;
const ROW_H: f32 = 32.0;
const ICON_SIZE: f32 = 22.0;
const PAD: f32 = 6.0;
const NAME_ROW_H: f32 = 22.0;
const PRICE_LABEL_W: f32 = 42.0;
const PRICE_W: f32 = 96.0;
const FOOTER_H: f32 = 30.0;
const VISIBLE_ROWS: usize = 4;

// "Available items for Vending" grid layout.
const AVAIL_COLS: usize = 8;
const AVAIL_ROWS: usize = 4;
const CELL: f32 = 32.0;
const CELL_ICON: f32 = 24.0;
const CELL_PAD: f32 = 4.0;
const AVAIL_PAD: f32 = 8.0;

struct StagedItem {
    index: u16,
    amount: i16,
    icon: Option<String>,
}

struct SetupSlot {
    item: Option<StagedItem>,
    price: TextInput,
}

pub struct VendingSetupWindow {
    has_grf_textures: bool,
    open: bool,
    max_items: usize,
    slots: Vec<SetupSlot>,
    title: TextInput,
    scroll_offset: usize,
    avail_scroll_offset: usize,
    btn_size: (f32, f32),
    reset_size: (f32, f32),
}

impl Default for VendingSetupWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl VendingSetupWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            open: false,
            max_items: 0,
            slots: Vec::new(),
            title: TextInput::new(35, false),
            scroll_offset: 0,
            avail_scroll_offset: 0,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            reset_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, max_items: usize) {
        self.max_items = max_items.max(1);
        self.slots = (0..self.max_items)
            .map(|_| SetupSlot {
                item: None,
                price: TextInput::new(13, false).with_numeric_only(true),
            })
            .collect();
        self.title.text.clear();
        self.scroll_offset = 0;
        self.avail_scroll_offset = 0;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.slots.clear();
    }

    fn staged_indices(&self) -> Vec<u16> {
        self.slots
            .iter()
            .filter_map(|s| s.item.as_ref().map(|it| it.index))
            .collect()
    }

    fn place_item(&mut self, slot: usize, index: u16, amount: i16, icon: Option<String>) {
        if self.staged_indices().contains(&index) {
            return;
        }
        if let Some(s) = self.slots.get_mut(slot)
            && s.item.is_none()
        {
            s.item = Some(StagedItem {
                index,
                amount,
                icon,
            });
        }
    }

    fn remove_index(&mut self, index: u16) {
        if let Some(s) = self
            .slots
            .iter_mut()
            .find(|s| s.item.as_ref().is_some_and(|it| it.index == index))
        {
            s.item = None;
            s.price.text.clear();
        }
    }

    fn collect_items(&self) -> Vec<(i16, i16, i32)> {
        self.slots
            .iter()
            .filter_map(|s| {
                let item = s.item.as_ref()?;
                let price: i32 = s.price.text.replace(',', "").trim().parse().ok()?;
                (price > 0).then_some((item.index as i16, item.amount, price))
            })
            .collect()
    }

    pub fn build_available(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.open {
            return Vec::new();
        }
        let events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);

        let grid_w = AVAIL_COLS as f32 * CELL;
        let grid_h = AVAIL_ROWS as f32 * CELL;
        let win_w = AVAIL_PAD + grid_w + SCROLLBAR_W + AVAIL_PAD;
        let win_h = TITLE_H + AVAIL_PAD + grid_h + AVAIL_PAD;

        let win = ui.window_at(
            VENDING_AVAILABLE_WINDOW_ID,
            win_w,
            win_h,
            TITLE_H,
            560.0,
            70.0,
        );
        let (wx, wy) = (win.x, win.y);
        ui.interact(VENDING_AVAILABLE_WINDOW_ID, Rect::new(wx, wy, win_w, win_h));

        draw_titlebar(ui, wx, wy, win_w, TITLE_H, grf);
        ui.text(
            wx + 17.0,
            wy + TITLE_H - 3.0,
            "Available items for Vending",
            tc,
        );

        let body_y = wy + TITLE_H;
        draw_container(ui, wx, body_y, win_w, AVAIL_PAD + grid_h + AVAIL_PAD, grf);

        let staged = self.staged_indices();
        let available: Vec<(u16, i16, Option<String>, bool)> = character
            .cart
            .all_items()
            .iter()
            .filter(|it| !staged.contains(&it.index))
            .map(|it| (it.index, it.count, it.icon_path(), it.is_identified))
            .collect();

        let total_rows = available.len().div_ceil(AVAIL_COLS);
        let max_scroll = total_rows.saturating_sub(AVAIL_ROWS);
        if self.avail_scroll_offset > max_scroll {
            self.avail_scroll_offset = max_scroll;
        }

        let grid_x = wx + AVAIL_PAD;
        let grid_y = body_y + AVAIL_PAD;
        let start = self.avail_scroll_offset * AVAIL_COLS;

        for slot in 0..(AVAIL_COLS * AVAIL_ROWS) {
            let col = slot % AVAIL_COLS;
            let row = slot / AVAIL_COLS;
            let cx = grid_x + col as f32 * CELL;
            let cy = grid_y + row as f32 * CELL;
            let cell_rect = Rect::new(cx, cy, CELL, CELL);
            let resp = ui.interact(WidgetId(AVAIL_CELL_BASE_ID + slot as u32), cell_rect);

            if grf {
                let (v, i) = draw::quad_vertices(
                    cx + CELL_PAD,
                    cy + CELL_PAD,
                    CELL_ICON,
                    CELL_ICON,
                    [1.0; 4],
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(ITEMWIN_MID_TEX.to_string()),
                });
            } else {
                crate::helper::fallback::slot_cell(
                    ui,
                    cx + CELL_PAD,
                    cy + CELL_PAD,
                    CELL_ICON,
                    CELL_ICON,
                );
            }

            let item_idx = start + slot;
            let Some((index, count, icon, identified)) = available.get(item_idx) else {
                continue;
            };
            if let Some(icon_path) = icon {
                let tint = if *identified {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.67, 0.67, 0.67, 1.0]
                };
                let (v, i) =
                    draw::quad_vertices(cx + CELL_PAD, cy + CELL_PAD, CELL_ICON, CELL_ICON, tint);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(icon_path.clone()),
                });
            }
            let count_str = count.to_string();
            let count_w = ui.atlas.measure_text(&count_str);
            ui.text(
                cx + CELL_ICON - count_w + 2.0,
                cy + CELL - 2.0,
                &count_str,
                [0.0, 0.0, 0.0, 1.0],
            );

            if resp.clicked() {
                ui.drag_source(
                    VENDING_AVAILABLE_WINDOW_ID,
                    *index as usize,
                    icon.clone(),
                    (CELL_ICON, CELL_ICON),
                );
            }
        }

        let grid_rect = Rect::new(grid_x, grid_y, grid_w, grid_h);
        if let Some((source_id, staged_index)) = ui.drop_zone(grid_rect)
            && source_id == VENDING_SETUP_WINDOW_ID
        {
            self.remove_index(staged_index as u16);
        }

        if total_rows > AVAIL_ROWS {
            let content_rect = Rect::new(wx, body_y, win_w, AVAIL_PAD + grid_h + AVAIL_PAD);
            self.avail_scroll_offset = scrollbar::scrollbar(
                ui,
                ScrollbarIds {
                    up: AVAIL_SCROLL_UP_ID,
                    down: AVAIL_SCROLL_DOWN_ID,
                    thumb: AVAIL_SCROLL_THUMB_ID,
                },
                self.avail_scroll_offset,
                AVAIL_ROWS,
                max_scroll,
                content_rect,
                wx + win_w - SCROLLBAR_W - 1.0,
                body_y,
                AVAIL_PAD + grid_h + AVAIL_PAD,
            );
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl Window for VendingSetupWindow {
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
        if let Some((w, h)) = size_fn(RESET_BTN.normal) {
            self.reset_size = (w as f32, h as f32);
        }
    }
    fn window_size(&self) -> (f32, f32) {
        let list_h = VISIBLE_ROWS as f32 * ROW_H;
        (
            WIN_W,
            TITLE_H + PAD + NAME_ROW_H + PAD + list_h + PAD + FOOTER_H,
        )
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            ITEMWIN_MID_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
            RESET_BTN.normal,
            RESET_BTN.hover,
            RESET_BTN.pressed,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for VendingSetupWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.is_open()
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.close();
        vec![GameEvent::RequestCancelVendingSetup]
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let bg = if grf {
            TextInputBg::Gray
        } else {
            TextInputBg::Default
        };

        let max_scroll = self.max_items.saturating_sub(VISIBLE_ROWS);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
        let list_h = VISIBLE_ROWS as f32 * ROW_H;
        let win_h = TITLE_H + PAD + NAME_ROW_H + PAD + list_h + PAD + FOOTER_H;

        let win = ui.window_at(VENDING_SETUP_WINDOW_ID, WIN_W, win_h, TITLE_H, 100.0, 70.0);
        let (dx, dy) = (win.x, win.y);
        ui.interact(VENDING_SETUP_WINDOW_ID, Rect::new(dx, dy, WIN_W, win_h));

        draw_titlebar(ui, dx, dy, WIN_W, TITLE_H, grf);
        ui.text(dx + 6.0, dy + TITLE_H - 3.0, "Open a Shop", tc);

        let body_y = dy + TITLE_H;
        let body_h = PAD + NAME_ROW_H + PAD + list_h + PAD;
        draw_container(ui, dx, body_y, WIN_W, body_h, grf);
        draw_footer(ui, dx, dy + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        let name_y = body_y + PAD;
        ui.text(dx + PAD, name_y + NAME_ROW_H - 6.0, "Shop Name", tc);
        let name_rect = Rect::new(dx + PAD + 70.0, name_y, WIN_W - PAD * 2.0 - 70.0, 16.0);
        ui.text_input(TITLE_INPUT_ID, name_rect, &mut self.title, bg);

        let has_scroll = max_scroll > 0;
        let list_x = dx + PAD;
        let list_y = name_y + NAME_ROW_H + PAD;
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
                VISIBLE_ROWS,
                max_scroll,
                content_rect,
                dx + WIN_W - PAD - SCROLLBAR_W,
                list_y,
                list_h,
            );
        } else {
            self.scroll_offset = 0;
        }

        for vis in 0..VISIBLE_ROWS {
            let slot = self.scroll_offset + vis;
            if slot >= self.max_items {
                break;
            }
            let row_y = list_y + vis as f32 * ROW_H;

            // Item cell doubles as a drag handle: drag a staged item onto the
            // Available window to un-stage it.
            let icon_y = row_y + (ROW_H - ICON_SIZE) / 2.0;
            let cell_rect = Rect::new(list_x + 2.0, icon_y, ICON_SIZE, ICON_SIZE);
            if grf {
                let (v, i) =
                    draw::quad_vertices(cell_rect.x, cell_rect.y, ICON_SIZE, ICON_SIZE, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(ITEMWIN_MID_TEX.to_string()),
                });
            } else {
                crate::helper::fallback::slot_cell(
                    ui,
                    cell_rect.x,
                    cell_rect.y,
                    ICON_SIZE,
                    ICON_SIZE,
                );
            }

            let handle = ui.interact(WidgetId(ROW_DRAG_BASE_ID + vis as u32), cell_rect);
            if let Some(item) = self.slots[slot].item.as_ref() {
                if let Some(icon) = &item.icon {
                    let (v, i) = draw::quad_vertices(
                        cell_rect.x,
                        cell_rect.y,
                        ICON_SIZE,
                        ICON_SIZE,
                        [1.0; 4],
                    );
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: i.to_vec(),
                        texture: TextureRef::Named(icon.clone()),
                    });
                }
                let qty = item.amount.to_string();
                ui.text(
                    cell_rect.x,
                    cell_rect.y + ICON_SIZE - 1.0,
                    &qty,
                    [0.0, 0.0, 0.0, 1.0],
                );
                if handle.clicked() {
                    ui.drag_source(
                        VENDING_SETUP_WINDOW_ID,
                        item.index as usize,
                        item.icon.clone(),
                        (ICON_SIZE, ICON_SIZE),
                    );
                }
            }

            let price_label_x = list_x + list_w - PRICE_W - PRICE_LABEL_W;
            ui.text(price_label_x, row_y + ROW_H - 12.0, "Price:", tc);
            let price_rect = Rect::new(
                list_x + list_w - PRICE_W,
                row_y + (ROW_H - 16.0) / 2.0,
                PRICE_W,
                16.0,
            );
            ui.text_input(
                WidgetId(PRICE_BASE_ID + vis as u32),
                price_rect,
                &mut self.slots[slot].price,
                bg,
            );
            reformat_price(&mut self.slots[slot].price);
            if self.slots[slot].price.text.is_empty() {
                ui.text(
                    price_rect.x + 4.0,
                    price_rect.y + 12.0,
                    "0,000",
                    [0.5, 0.5, 0.5, 1.0],
                );
            }

            let row_rect = Rect::new(list_x, row_y, list_w, ROW_H);
            if let Some((source_id, cart_index)) = ui.drop_zone(row_rect)
                && source_id == VENDING_AVAILABLE_WINDOW_ID
                && let Some(cart_item) = character.cart.get_item(cart_index as u16)
            {
                self.place_item(
                    slot,
                    cart_item.index,
                    cart_item.count,
                    cart_item.icon_path(),
                );
            }
        }

        // Footer: Reset / OK / cancel on the right.
        let (btn_w, btn_h) = self.btn_size;
        let (reset_w, reset_h) = self.reset_size;
        let footer_y = dy + win_h - FOOTER_H;
        let btn_y = footer_y + (FOOTER_H - btn_h) / 2.0;
        let reset_y = footer_y + (FOOTER_H - reset_h) / 2.0;

        let mut bx = dx + WIN_W - PAD - btn_w;
        let cancel = ui
            .button(
                CANCEL_ID,
                Rect::new(bx, btn_y, btn_w, btn_h),
                &CANCEL_BTN,
                "cancel",
            )
            .clicked();
        bx -= btn_w + 4.0;
        let ok = ui
            .button(OK_ID, Rect::new(bx, btn_y, btn_w, btn_h), &OK_BTN, "OK")
            .clicked();
        bx -= reset_w + 4.0;
        let reset = ui
            .button(
                RESET_ID,
                Rect::new(bx, reset_y, reset_w, reset_h),
                &RESET_BTN,
                "Reset",
            )
            .clicked();

        if ok {
            let items = self.collect_items();
            if !items.is_empty() {
                events.push(GameEvent::RequestOpenStore {
                    shop_name: self.title.text.clone(),
                    items,
                });
                self.close();
            }
        } else if cancel {
            events.push(GameEvent::RequestCancelVendingSetup);
            self.close();
        } else if reset {
            self.open(self.max_items);
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

/// Rewrite a price field to comma-grouped digits (max 9 digits), keeping the cursor
/// at the end. Commas are stripped again when the price is parsed.
fn reformat_price(input: &mut TextInput) {
    let digits: String = input
        .text
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(9)
        .collect();
    let formatted = match digits.parse::<i64>() {
        Ok(n) => format_thousands(n),
        Err(_) => String::new(),
    };
    if input.text != formatted {
        input.cursor_pos = formatted.chars().count();
        input.text = formatted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_items_only_returns_priced_staged_slots() {
        let mut win = VendingSetupWindow::new();
        win.open(4);
        win.place_item(0, 10, 3, None);
        win.place_item(2, 11, 1, None);
        win.slots[0].price.text = "100".into();
        // slot 2 left unpriced → excluded; empty slots 1 and 3 → excluded
        let items = win.collect_items();
        assert_eq!(items, vec![(10, 3, 100)]);
    }

    #[test]
    fn place_item_ignores_duplicate_index_and_occupied_slot() {
        let mut win = VendingSetupWindow::new();
        win.open(4);
        win.place_item(0, 10, 3, None);
        win.place_item(1, 10, 3, None); // same item id already staged → ignored
        win.place_item(0, 11, 1, None); // slot 0 occupied → ignored
        assert_eq!(win.staged_indices(), vec![10]);
    }
}
