use crate::helper::format::format_thousands;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_titlebar,
    text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::{GameEvent, VendorItem};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const MY_SHOP_WINDOW_ID: WidgetId = WidgetId(2600);
const CLOSE_ID: WidgetId = WidgetId(2601);
const SCROLL_UP_ID: WidgetId = WidgetId(2602);
const SCROLL_DOWN_ID: WidgetId = WidgetId(2603);
const SCROLL_THUMB_ID: WidgetId = WidgetId(2604);

const CLOSE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_cancel.bmp",
    hover: "data/texture/유저인터페이스/btn_cancel_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_cancel_b.bmp",
};

const WIN_W: f32 = 300.0;
const TITLE_H: f32 = 17.0;
const ROW_H: f32 = 22.0;
const ICON_SIZE: f32 = 18.0;
const PAD: f32 = 6.0;
const PRICE_W: f32 = 90.0;
const STOCK_W: f32 = 44.0;
const FOOTER_H: f32 = 28.0;
const VISIBLE_ROWS: usize = 8;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

struct MyShopRow {
    item: VendorItem,
    name: String,
    icon: Option<String>,
}

#[derive(Default)]
pub struct MyShopWindow {
    has_grf_textures: bool,
    open: bool,
    shop_name: String,
    rows: Vec<MyShopRow>,
    scroll_offset: usize,
    btn_size: (f32, f32),
}

impl MyShopWindow {
    pub fn new() -> Self {
        Self {
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            ..Default::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, shop_name: String, rows: Vec<(VendorItem, String, Option<String>)>) {
        self.shop_name = shop_name;
        self.rows = rows
            .into_iter()
            .map(|(item, name, icon)| MyShopRow { item, name, icon })
            .collect();
        self.scroll_offset = 0;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.rows.clear();
    }

    /// `sold` = quantity just sold; subtract it from the displayed stock.
    pub fn record_sale(&mut self, index: i16, sold: i16) {
        if let Some(pos) = self.rows.iter().position(|r| r.item.index == index) {
            self.rows[pos].item.amount -= sold;
            if self.rows[pos].item.amount <= 0 {
                self.rows.remove(pos);
            }
        }
    }
}

impl Window for MyShopWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(CLOSE_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
    }
    fn window_size(&self) -> (f32, f32) {
        let visible_rows = self.rows.len().min(VISIBLE_ROWS).max(1);
        let list_h = visible_rows as f32 * ROW_H;
        (WIN_W, TITLE_H + PAD + list_h + PAD + FOOTER_H)
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_BTN.normal,
            CLOSE_BTN.hover,
            CLOSE_BTN.pressed,
        ];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for MyShopWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        ctx: &mut BuildCtx,
    ) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let (btn_w, btn_h) = self.btn_size;

        let visible_rows = self.rows.len().min(VISIBLE_ROWS).max(1);
        let max_scroll = self.rows.len().saturating_sub(visible_rows);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
        let list_h = visible_rows as f32 * ROW_H;
        let win_h = TITLE_H + PAD + list_h + PAD + FOOTER_H;

        let win = ui.window_at(MY_SHOP_WINDOW_ID, WIN_W, win_h, TITLE_H, 300.0, 100.0);
        let (dx, dy) = (win.x, win.y);
        ui.interact(MY_SHOP_WINDOW_ID, Rect::new(dx, dy, WIN_W, win_h));

        draw_titlebar(ui, dx, dy, WIN_W, TITLE_H, grf);
        let title = if self.shop_name.is_empty() {
            "My Shop".to_string()
        } else {
            format!("My Shop - {}", self.shop_name)
        };
        ui.text(dx + 17.0, dy + TITLE_H - 3.0, &title, tc);

        let body_y = dy + TITLE_H;
        let body_h = PAD + list_h + PAD;
        draw_container(ui, dx, body_y, WIN_W, body_h, grf);
        draw_footer(ui, dx, dy + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        let has_scroll = max_scroll > 0;
        let list_x = dx + PAD;
        let list_y = body_y + PAD;
        let list_w = WIN_W - PAD * 2.0 - if has_scroll { SCROLLBAR_W } else { 0.0 };

        if has_scroll {
            let content_rect = Rect::new(list_x, list_y, WIN_W - PAD * 2.0, list_h);
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
            let Some(row) = self.rows.get(idx) else {
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
            ui.text(text_x, row_y + ROW_H - 6.0, &row.name, tc);

            let price_label = format!("{}z", format_thousands(row.item.price as i64));
            let price_x = list_x + list_w - PRICE_W - STOCK_W;
            let price_y = row_y + ROW_H - 6.0;
            let (price_color, price_shadow) = crate::helper::colors::price_style(row.item.price as i64);
            if let Some(shadow) = price_shadow {
                ui.text(price_x + 1.0, price_y, &price_label, shadow);
            }
            ui.text(price_x, price_y, &price_label, price_color);
            let stock_label = format!("x{}", row.item.amount);
            ui.text(
                list_x + list_w - STOCK_W,
                row_y + ROW_H - 6.0,
                &stock_label,
                tc,
            );
        }

        let win_rect = Rect::new(dx, dy, WIN_W, win_h);
        let btns = win_rect.buttons_bottom_right(1, btn_w, btn_h, 5.0, 5.0, 3.0);
        if ui.button(CLOSE_ID, btns[0], &CLOSE_BTN, "Close").clicked() {
            events.push(GameEvent::RequestCloseStore);
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}
