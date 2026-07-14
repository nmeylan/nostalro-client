use super::number_input::{NumberInputConfig, NumberInputDialog, NumberInputResult};
use crate::helper::colors::draw_price_right;
use crate::helper::dialog_container::DialogContainer;
use crate::helper::format::format_thousands;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    ITEMWIN_MID_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_textured_quad, draw_titlebar,
    text_color,
};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::{GameEvent, VendorItem};
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, RESIZE_HANDLE_TEX, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const VENDING_SHOP_WINDOW_ID: WidgetId = WidgetId(2400);
const SCROLL_UP_ID: WidgetId = WidgetId(2405);
const SCROLL_DOWN_ID: WidgetId = WidgetId(2406);
const SCROLL_THUMB_ID: WidgetId = WidgetId(2407);
const ROW_BASE_ID: u32 = 2410;

const LEFT_RESIZE_ID: WidgetId = WidgetId(2408);
pub const VENDING_BUY_WINDOW_ID: WidgetId = WidgetId(2420);
const BUY_ID: WidgetId = WidgetId(2421);
const CANCEL_ID: WidgetId = WidgetId(2422);
const RIGHT_RESIZE_ID: WidgetId = WidgetId(2423);
const STAGED_BASE_ID: u32 = 2430;
const NUM_DIALOG_BASE: u32 = 2440;

const BUY_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_buy.bmp",
    hover: "data/texture/유저인터페이스/btn_buy_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_buy_b.bmp",
};
const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_cancel.bmp",
    hover: "data/texture/유저인터페이스/btn_cancel_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_cancel_b.bmp",
};

const WIN_W: f32 = 280.0;
const TITLE_H: f32 = 17.0;
const ROW_H: f32 = 30.0;
const ICON_SIZE: f32 = 24.0;
const PAD: f32 = 6.0;
const PRICE_AREA_W: f32 = 92.0;
const TOTAL_H: f32 = 20.0;
const FOOTER_H: f32 = 28.0;
const LIST_ROWS: usize = 8;
const STAGED_SLOTS: usize = 8;
const MIN_ROWS: usize = 2;
const MAX_ROWS: usize = 9;
const GRIP: f32 = 16.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

#[derive(Clone)]
struct ShopRow {
    item: VendorItem,
    name: String,
    icon: Option<String>,
}

#[derive(Clone)]
struct StagedBuy {
    index: i16,
    count: i16,
    name: String,
    icon: Option<String>,
    unit_price: i32,
}

#[derive(Default)]
pub struct VendingShopWindow {
    has_grf_textures: bool,
    open: bool,
    aid: u32,
    unique_id: u32,
    title: String,
    rows: Vec<ShopRow>,
    staged: Vec<StagedBuy>,
    scroll_offset: usize,
    list_rows: usize,
    buy_rows: usize,
    resize: Option<(WidgetId, f32, usize)>,
    btn_size: (f32, f32),
    container: DialogContainer,
    qty_dialog: Option<NumberInputDialog>,
    pending_stage: Option<usize>,
}

impl VendingShopWindow {
    pub fn new() -> Self {
        Self {
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            list_rows: LIST_ROWS,
            buy_rows: STAGED_SLOTS,
            ..Default::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(
        &mut self,
        aid: u32,
        unique_id: u32,
        title: String,
        rows: Vec<(VendorItem, String, Option<String>)>,
    ) {
        self.aid = aid;
        self.unique_id = unique_id;
        self.title = title;
        self.rows = rows
            .into_iter()
            .map(|(item, name, icon)| ShopRow { item, name, icon })
            .collect();
        self.staged.clear();
        self.scroll_offset = 0;
        self.qty_dialog = None;
        self.pending_stage = None;
        self.open = !self.rows.is_empty();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.rows.clear();
        self.staged.clear();
        self.qty_dialog = None;
        self.pending_stage = None;
    }

    fn total(&self) -> i64 {
        self.staged
            .iter()
            .map(|s| s.unit_price as i64 * s.count as i64)
            .sum()
    }

    fn stage(&mut self, row_idx: usize, count: i16) {
        let Some(row) = self.rows.get(row_idx) else {
            return;
        };
        let stock = row.item.amount.max(0);
        if count <= 0 || stock <= 0 {
            return;
        }
        let index = row.item.index;
        if let Some(existing) = self.staged.iter_mut().find(|s| s.index == index) {
            existing.count = (existing.count + count).min(stock);
        } else {
            self.staged.push(StagedBuy {
                index,
                count: count.min(stock),
                name: row.name.clone(),
                icon: row.icon.clone(),
                unit_price: row.item.price,
            });
        }
    }

    fn unstage(&mut self, staged_idx: usize) {
        if staged_idx < self.staged.len() {
            self.staged.remove(staged_idx);
        }
    }

    /// `sold` = quantity just purchased; subtract it from the displayed stock.
    pub fn record_sale(&mut self, index: i16, sold: i16) {
        if let Some(pos) = self.rows.iter().position(|r| r.item.index == index) {
            self.rows[pos].item.amount -= sold;
            if self.rows[pos].item.amount <= 0 {
                self.rows.remove(pos);
                self.staged.retain(|s| s.index != index);
            }
        }
        if self.rows.is_empty() {
            self.open = false;
        }
    }

    fn build_left(&mut self, ui: &mut UiFrame, grf: bool) {
        let tc = text_color(grf);
        let rows_shown = self.list_rows;
        let max_scroll = self.rows.len().saturating_sub(rows_shown);
        let list_h = rows_shown as f32 * ROW_H;
        let win_h = TITLE_H + PAD + list_h + PAD + FOOTER_H;

        let win = ui.window_at(VENDING_SHOP_WINDOW_ID, WIN_W, win_h, TITLE_H, 260.0, 120.0);
        let (dx, dy) = (win.x, win.y);
        ui.interact(VENDING_SHOP_WINDOW_ID, Rect::new(dx, dy, WIN_W, win_h));

        draw_titlebar(ui, dx, dy, WIN_W, TITLE_H, grf);
        ui.text(
            dx + 17.0,
            dy + TITLE_H - 4.0,
            &format!("Merchant Shop - {}", self.title),
            tc,
        );

        let body_y = dy + TITLE_H;
        draw_container(ui, dx, body_y, WIN_W, PAD + list_h + PAD, grf);
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
                rows_shown,
                max_scroll,
                content_rect,
                dx + WIN_W - PAD - SCROLLBAR_W,
                list_y,
                list_h,
            );
        } else {
            self.scroll_offset = 0;
        }

        let text_x = list_x + ICON_SIZE + 6.0;
        let price_right = list_x + list_w - 4.0;
        let name_max_w = (list_x + list_w - PRICE_AREA_W) - text_x;

        for vis in 0..rows_shown {
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
                ui.drag_source(
                    VENDING_SHOP_WINDOW_ID,
                    idx,
                    row.icon.clone(),
                    (ICON_SIZE, ICON_SIZE),
                );
            }

            let iy = row_y + (ROW_H - ICON_SIZE) / 2.0;
            draw_item_slot(ui, list_x + 2.0, iy, grf);
            if let Some(icon) = &row.icon {
                let (v, i) = draw::quad_vertices(list_x + 2.0, iy, ICON_SIZE, ICON_SIZE, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(icon.clone()),
                });
            }
            draw_amount(ui, list_x + 2.0, iy, row.item.amount);

            let lines = wrap_name(&row.name, name_max_w, ui.atlas);
            if lines.len() == 1 {
                let y = row_y + (ROW_H + ui.atlas.line_height) / 2.0 - 2.0;
                ui.text(text_x, y, &lines[0], tc);
            } else {
                ui.text(text_x, row_y + 12.0, &lines[0], tc);
                ui.text(text_x, row_y + 24.0, &lines[1], tc);
            }

            let price_str = format!("{} Z", format_thousands(row.item.price as i64));
            let py = row_y + (ROW_H + ui.atlas.line_height) / 2.0 - 2.0;
            draw_price_right(ui, price_right, py, &price_str, row.item.price as i64);
        }

        resize_grip(
            &mut self.resize,
            &mut self.list_rows,
            ui,
            LEFT_RESIZE_ID,
            dx,
            dy,
            WIN_W,
            win_h,
            grf,
        );
    }

    fn build_right(&mut self, ui: &mut UiFrame, grf: bool) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let tc = text_color(grf);
        let (btn_w, btn_h) = self.btn_size;

        let slots_shown = self.buy_rows;
        let list_h = slots_shown as f32 * ROW_H;
        let win_h = TITLE_H + PAD + list_h + PAD + TOTAL_H + FOOTER_H;

        let win = ui.window_at(VENDING_BUY_WINDOW_ID, WIN_W, win_h, TITLE_H, 550.0, 120.0);
        let (dx, dy) = (win.x, win.y);
        ui.interact(VENDING_BUY_WINDOW_ID, Rect::new(dx, dy, WIN_W, win_h));

        draw_titlebar(ui, dx, dy, WIN_W, TITLE_H, grf);
        ui.text(dx + 17.0, dy + TITLE_H - 4.0, "Buying Items", tc);

        let body_y = dy + TITLE_H;
        let body_h = PAD + list_h + PAD + TOTAL_H;
        draw_container(ui, dx, body_y, WIN_W, body_h, grf);
        draw_footer(ui, dx, dy + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        let slot_x = dx + PAD;
        let slot_w = WIN_W - PAD * 2.0;
        let slots_y = body_y + PAD;

        let mut to_remove = None;
        for slot in 0..slots_shown {
            let sy = slots_y + slot as f32 * ROW_H;
            let slot_rect = Rect::new(slot_x, sy, slot_w, ROW_H);
            let iy = sy + (ROW_H - ICON_SIZE) / 2.0;
            draw_item_slot(ui, slot_x + 2.0, iy, grf);

            let Some(item) = self.staged.get(slot) else {
                continue;
            };
            let resp = ui.interact(WidgetId(STAGED_BASE_ID + slot as u32), slot_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() {
                to_remove = Some(slot);
            }

            if let Some(icon) = &item.icon {
                let (v, i) = draw::quad_vertices(slot_x + 2.0, iy, ICON_SIZE, ICON_SIZE, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(icon.clone()),
                });
            }
            draw_amount(ui, slot_x + 2.0, iy, item.count);

            let tx = slot_x + ICON_SIZE + 6.0;
            let price_right = slot_x + slot_w - 4.0;
            let name_max_w = (slot_x + slot_w - PRICE_AREA_W) - tx;
            let y = sy + (ROW_H + ui.atlas.line_height) / 2.0 - 2.0;
            let label = truncate_to_width(&item.name, name_max_w, ui.atlas);
            ui.text(tx, y, &label, tc);
            let line_price = item.unit_price as i64 * item.count as i64;
            draw_price_right(
                ui,
                price_right,
                y,
                &format!("{} Z", format_thousands(line_price)),
                line_price,
            );
        }
        if let Some(slot) = to_remove {
            self.unstage(slot);
        }

        let slots_rect = Rect::new(slot_x, slots_y, slot_w, list_h);
        if let Some((source_id, row_idx)) = ui.drop_zone(slots_rect)
            && source_id == VENDING_SHOP_WINDOW_ID
        {
            let stock = self.rows.get(row_idx).map(|r| r.item.amount.max(0)).unwrap_or(0);
            if stock > 1 {
                let mut dialog = NumberInputDialog::new(
                    NumberInputConfig {
                        label: None,
                        show_cancel: true,
                        escape_cancels: true,
                        default_value: stock.to_string(),
                        max_len: 6,
                    },
                    WidgetId(NUM_DIALOG_BASE),
                );
                dialog.init_container(&self.container);
                self.qty_dialog = Some(dialog);
                self.pending_stage = Some(row_idx);
            } else {
                self.stage(row_idx, 1);
            }
        }

        let total_str = format!("Total : {} Zeny", format_thousands(self.total()));

        let win_rect = Rect::new(dx, dy, WIN_W, win_h);
        ui.text(dx + PAD, win_rect.y + win_rect.h - 10.0, &total_str, tc);
        let btns = win_rect.buttons_bottom_right(2, btn_w, btn_h, 5.0, GRIP + 3.0, 3.0);
        let cancel = ui.button(CANCEL_ID, btns[0], &CANCEL_BTN, "Cancel");
        let buy = ui.button(BUY_ID, btns[1], &BUY_BTN, "Buy");
        if buy.clicked() && !self.staged.is_empty() {
            let items = self.staged.iter().map(|s| (s.count, s.index)).collect();
            events.push(GameEvent::RequestPurchaseFromVendor {
                aid: self.aid,
                unique_id: self.unique_id,
                items,
            });
            self.close();
        } else if cancel.clicked() {
            self.close();
        }

        resize_grip(
            &mut self.resize,
            &mut self.buy_rows,
            ui,
            RIGHT_RESIZE_ID,
            dx,
            dy,
            WIN_W,
            win_h,
            grf,
        );

        events
    }
}

impl Window for VendingShopWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
        self.container.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(BUY_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
        self.container.set_texture_sizes(size_fn);
    }
    fn window_size(&self) -> (f32, f32) {
        let list_h = self.list_rows as f32 * ROW_H;
        (WIN_W, TITLE_H + PAD + list_h + PAD + FOOTER_H)
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            BUY_BTN.normal,
            BUY_BTN.hover,
            BUY_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ];
        paths.push(ITEMWIN_MID_TEX);
        paths.push(RESIZE_HANDLE_TEX);
        paths.extend(scrollbar::grf_texture_paths());
        paths.extend(NumberInputDialog::grf_texture_paths());
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

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        self.container.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;

        if let Some(dialog) = &self.qty_dialog {
            ui.set_modal(&[dialog.win_id()]);
        }

        self.build_left(ui, grf);
        let events = self.build_right(ui, grf);

        if let Some(dialog) = &mut self.qty_dialog {
            match dialog.build(ui) {
                NumberInputResult::Submitted => {
                    let qty = dialog.value_i16().unwrap_or(0);
                    if let Some(row_idx) = self.pending_stage.take() {
                        self.stage(row_idx, qty);
                    }
                    self.qty_dialog = None;
                }
                NumberInputResult::Cancel => {
                    self.pending_stage = None;
                    self.qty_dialog = None;
                }
                NumberInputResult::None => {}
            }
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

fn draw_item_slot(ui: &mut UiFrame, x: f32, y: f32, grf: bool) {
    if grf {
        draw_textured_quad(ui, x, y, ICON_SIZE, ICON_SIZE, ITEMWIN_MID_TEX);
    } else {
        crate::helper::fallback::slot_cell(ui, x, y, ICON_SIZE, ICON_SIZE);
    }
}

fn draw_amount(ui: &mut UiFrame, icon_x: f32, icon_y: f32, amount: i16) {
    if amount <= 0 {
        return;
    }
    let s = amount.to_string();
    let w = ui.atlas.measure_text(&s);
    ui.text(icon_x + ICON_SIZE - w, icon_y + ICON_SIZE - 1.0, &s, [0.0, 0.0, 0.0, 1.0]);
}

#[allow(clippy::too_many_arguments)]
fn resize_grip(
    resize: &mut Option<(WidgetId, f32, usize)>,
    rows: &mut usize,
    ui: &mut UiFrame,
    resize_id: WidgetId,
    dx: f32,
    dy: f32,
    win_w: f32,
    win_h: f32,
    grf: bool,
) {
    let grip = Rect::new(dx + win_w - GRIP, dy + win_h - GRIP, GRIP, GRIP);
    ui.interact(resize_id, grip);
    let hovered = grip.contains(ui.ctx.mouse_x, ui.ctx.mouse_y);
    if hovered {
        ui.any_interactive_hovered = true;
    }
    if hovered && ui.ctx.mouse_clicked {
        *resize = Some((resize_id, ui.ctx.mouse_y, *rows));
    }
    if !ui.ctx.mouse_down && matches!(resize, Some((id, _, _)) if *id == resize_id) {
        *resize = None;
    }
    if let Some((id, start_mouse, start_rows)) = *resize
        && id == resize_id
    {
        let delta = ((ui.ctx.mouse_y - start_mouse) / ROW_H).round() as i32;
        *rows = (start_rows as i32 + delta).clamp(MIN_ROWS as i32, MAX_ROWS as i32) as usize;
    }
    if grf {
        draw_textured_quad(ui, grip.x, grip.y, GRIP, GRIP, RESIZE_HANDLE_TEX);
    } else {
        let c = if hovered {
            [0.6, 0.6, 0.7, 0.9]
        } else {
            [0.5, 0.5, 0.6, 0.8]
        };
        let (v, i) = draw::quad_vertices(grip.x, grip.y, GRIP, GRIP, c);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }
}

fn wrap_name(name: &str, max_w: f32, atlas: &FontAtlas) -> Vec<String> {
    if atlas.measure_text(name) <= max_w {
        return vec![name.to_string()];
    }
    let mut line1 = String::new();
    let mut rest = String::new();
    for word in name.split_whitespace() {
        if line1.is_empty() {
            line1 = word.to_string();
        } else if rest.is_empty() {
            let trial = format!("{line1} {word}");
            if atlas.measure_text(&trial) <= max_w {
                line1 = trial;
            } else {
                rest = word.to_string();
            }
        } else {
            rest.push(' ');
            rest.push_str(word);
        }
    }
    if rest.is_empty() {
        return vec![truncate_to_width(name, max_w, atlas)];
    }
    vec![line1, truncate_to_width(&rest, max_w, atlas)]
}

fn truncate_to_width(text: &str, max_w: f32, atlas: &FontAtlas) -> String {
    if atlas.measure_text(text) <= max_w {
        return text.to_string();
    }
    let ellipsis = "…";
    let ellipsis_w = atlas.measure_text(ellipsis);
    let mut out = String::new();
    for ch in text.chars() {
        let mut trial = out.clone();
        trial.push(ch);
        if atlas.measure_text(&trial) + ellipsis_w > max_w {
            break;
        }
        out.push(ch);
    }
    out.push_str(ellipsis);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn staging_then_buy_emits_all_items() {
        let mut win = VendingShopWindow::new();
        win.open(
            42,
            7,
            "store02".into(),
            vec![
                (vendor_item(3, 5, 100), "Red Potion".into(), None),
                (vendor_item(4, 2, 200), "Blue Potion".into(), None),
            ],
        );
        win.stage(0, 1);
        win.stage(1, 2);
        assert_eq!(win.total(), 100 + 400);

        let mut character = Character::new();
        let mut state = StateCache::new();
        let win_h = TITLE_H + PAD + STAGED_SLOTS as f32 * ROW_H + PAD + TOTAL_H + FOOTER_H;
        let buy_rect = Rect::new(550.0, 120.0, WIN_W, win_h)
            .buttons_bottom_right(2, FALLBACK_BTN_W, FALLBACK_BTN_H, 5.0, 5.0, 3.0)[1];
        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = buy_rect.x + buy_rect.w / 2.0;
        ctx.mouse_y = buy_rect.y + buy_rect.h / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui, &mut character, &DataTable::new());
        assert_eq!(events.len(), 1);
        match &events[0] {
            GameEvent::RequestPurchaseFromVendor { items, .. } => {
                assert_eq!(items.len(), 2);
                assert!(items.contains(&(1, 3)));
                assert!(items.contains(&(2, 4)));
            }
            _ => panic!("expected purchase event"),
        }
        assert!(!win.is_open());
    }

    #[test]
    fn record_sale_subtracts_then_removes_sold_out_row() {
        let mut win = VendingShopWindow::new();
        win.open(
            42,
            7,
            "store02".into(),
            vec![
                (vendor_item(3, 5, 100), "Red Potion".into(), None),
                (vendor_item(4, 2, 200), "Blue Potion".into(), None),
            ],
        );

        win.record_sale(3, 3);
        assert_eq!(win.rows[0].item.amount, 2);
        assert_eq!(win.rows.len(), 2);

        win.record_sale(4, 2);
        assert_eq!(win.rows.len(), 1);
        assert_eq!(win.rows[0].item.index, 3);
        assert!(win.is_open());

        win.record_sale(3, 2);
        assert!(win.rows.is_empty());
        assert!(!win.is_open());
    }
}
