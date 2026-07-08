use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_titlebar,
    text_color,
};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const VENDING_SETUP_WINDOW_ID: WidgetId = WidgetId(2300);
const START_ID: WidgetId = WidgetId(2301);
const CANCEL_ID: WidgetId = WidgetId(2302);
const TITLE_INPUT_ID: WidgetId = WidgetId(2303);
const SCROLL_UP_ID: WidgetId = WidgetId(2304);
const SCROLL_DOWN_ID: WidgetId = WidgetId(2305);
const SCROLL_THUMB_ID: WidgetId = WidgetId(2306);
const PRICE_BASE_ID: u32 = 2320;

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

const WIN_W: f32 = 300.0;
const TITLE_H: f32 = 17.0;
const ROW_H: f32 = 22.0;
const ICON_SIZE: f32 = 18.0;
const PAD: f32 = 6.0;
const TITLE_ROW_H: f32 = 22.0;
const PRICE_W: f32 = 80.0;
const FOOTER_H: f32 = 28.0;
const MAX_VISIBLE_ROWS: usize = 7;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

struct SetupRow {
    index: u16,
    amount: i16,
    name: String,
    icon: Option<String>,
    price: TextInput,
}

pub struct VendingSetupWindow {
    has_grf_textures: bool,
    open: bool,
    max_items: usize,
    rows: Vec<SetupRow>,
    title: TextInput,
    scroll_offset: usize,
    btn_size: (f32, f32),
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
            rows: Vec::new(),
            title: TextInput::new(35, false),
            scroll_offset: 0,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// `cart` = (inventory index, amount, display name, icon path).
    pub fn open(&mut self, max_items: usize, cart: Vec<(u16, i16, String, Option<String>)>) {
        self.max_items = max_items;
        self.rows = cart
            .into_iter()
            .map(|(index, amount, name, icon)| SetupRow {
                index,
                amount,
                name,
                icon,
                price: TextInput::new(9, false),
            })
            .collect();
        self.scroll_offset = 0;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.rows.clear();
    }

    fn collect_items(&self) -> Vec<(i16, i16, i32)> {
        self.rows
            .iter()
            .filter_map(|r| {
                let price: i32 = r.price.text.trim().parse().ok()?;
                if price > 0 {
                    Some((r.index as i16, r.amount, price))
                } else {
                    None
                }
            })
            .take(self.max_items.max(1))
            .collect()
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

impl InGameWindow for VendingSetupWindow {
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
        let bg = if grf {
            TextInputBg::Gray
        } else {
            TextInputBg::Default
        };
        let (btn_w, btn_h) = self.btn_size;

        let visible_rows = self.rows.len().min(MAX_VISIBLE_ROWS);
        let max_scroll = self.rows.len().saturating_sub(visible_rows);
        let list_h = visible_rows as f32 * ROW_H;
        let win_h = TITLE_H + PAD + TITLE_ROW_H + PAD + list_h + PAD + FOOTER_H;

        let win = ui.window_at(VENDING_SETUP_WINDOW_ID, WIN_W, win_h, TITLE_H, 280.0, 100.0);
        let (dx, dy) = (win.x, win.y);
        ui.interact(VENDING_SETUP_WINDOW_ID, Rect::new(dx, dy, WIN_W, win_h));

        draw_titlebar(ui, dx, dy, WIN_W, TITLE_H, grf);
        ui.text(dx + 17.0, dy + TITLE_H - 3.0, "Vending", tc);

        let body_y = dy + TITLE_H;
        let body_h = PAD + TITLE_ROW_H + PAD + list_h + PAD;
        draw_container(ui, dx, body_y, WIN_W, body_h, grf);
        draw_footer(ui, dx, dy + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        // Shop-title input.
        let title_y = body_y + PAD;
        ui.text(dx + PAD, title_y + TITLE_ROW_H - 6.0, "Shop:", tc);
        let title_rect = Rect::new(dx + PAD + 40.0, title_y, WIN_W - PAD * 2.0 - 40.0, 16.0);
        ui.text_input(TITLE_INPUT_ID, title_rect, &mut self.title, bg);

        let has_scroll = max_scroll > 0;
        let list_x = dx + PAD;
        let list_y = title_y + TITLE_ROW_H + PAD;
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

        for vis in 0..visible_rows {
            let idx = self.scroll_offset + vis;
            let Some(row) = self.rows.get_mut(idx) else {
                break;
            };
            let row_y = list_y + vis as f32 * ROW_H;
            let mut text_x = list_x + 2.0;
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
            let name_w = list_w - PRICE_W - (text_x - list_x) - 4.0;
            let label = format!("{} (x{})", row.name, row.amount);
            let _ = name_w;
            ui.text(text_x, row_y + ROW_H - 6.0, &label, tc);
            let price_rect = Rect::new(list_x + list_w - PRICE_W, row_y + 2.0, PRICE_W, 16.0);
            ui.text_input(
                WidgetId(PRICE_BASE_ID + vis as u32),
                price_rect,
                &mut row.price,
                bg,
            );
        }

        let win_rect = Rect::new(dx, dy, WIN_W, win_h);
        let btns = win_rect.buttons_bottom_right(2, btn_w, btn_h, 5.0, 5.0, 3.0);
        let cancel = ui.button(CANCEL_ID, btns[0], &CANCEL_BTN, "Cancel");
        let start = ui.button(START_ID, btns[1], &OK_BTN, "Start");
        if start.clicked() {
            let items = self.collect_items();
            if !items.is_empty() {
                events.push(GameEvent::RequestOpenStore {
                    shop_name: self.title.text.clone(),
                    items,
                });
                self.close();
            }
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

    #[test]
    fn collect_items_keeps_priced_rows_and_caps_to_max() {
        let mut win = VendingSetupWindow::new();
        win.open(
            2,
            vec![
                (10, 3, "Apple".into(), None),
                (11, 1, "Pear".into(), None),
                (12, 5, "Plum".into(), None),
            ],
        );
        win.rows[0].price.text = "100".into();
        win.rows[1].price.text = "".into();
        win.rows[2].price.text = "50".into();
        let items = win.collect_items();
        assert_eq!(items, vec![(10, 3, 100), (12, 5, 50)]);
    }
}
