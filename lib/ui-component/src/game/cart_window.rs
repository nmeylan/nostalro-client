use super::input_dialog::{InputDialog, InputDialogConfig, InputDialogResult};
use super::inventory_window::INV_WINDOW_ID;
use crate::helper::dialog_container::DialogContainer;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    FOOTER_TEX, ITEMWIN_MID_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container,
    draw_footer, draw_sys_button, draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const CART_WINDOW_ID: WidgetId = WidgetId(1800);
const CART_CLOSE_BTN_ID: WidgetId = WidgetId(1801);
const CART_MINI_BTN_ID: WidgetId = WidgetId(1802);
const CART_SCROLL_UP_ID: WidgetId = WidgetId(1803);
const CART_SCROLL_DOWN_ID: WidgetId = WidgetId(1804);
const CART_SCROLL_THUMB_ID: WidgetId = WidgetId(1805);
const CART_ITEM_BASE_ID: u32 = 1820;
const NUM_DIALOG_BASE: u32 = 1810; // uses 1810..1813

const CELL_SIZE: f32 = 32.0;
const ICON_SIZE: f32 = 24.0;
const ICON_PAD: f32 = 4.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 19.0;
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 4.0;
const MINI_BTN_SIZE: f32 = 11.0;

const GRID_COLS: usize = 5;
const GRID_ROWS: usize = 4;

const MINI_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_off.bmp";
const MINI_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_on.bmp";
const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

enum PendingCartMove {
    FromBody { index: u16 },
    FromStore { index: u16 },
}

pub struct CartWindow {
    pub has_grf_textures: bool,
    scroll_offset: usize,
    minimized: bool,
    container: DialogContainer,
    qty_dialog: Option<(PendingCartMove, InputDialog)>,
}

impl Default for CartWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl CartWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            scroll_offset: 0,
            minimized: false,
            container: DialogContainer::new(),
            qty_dialog: None,
        }
    }

    fn open_qty_dialog(&mut self, kind: PendingCartMove, max: i16) {
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
        self.qty_dialog = Some((kind, dialog));
    }

    fn dimensions(&self) -> (f32, f32) {
        let grid_w = GRID_COLS as f32 * CELL_SIZE;
        let grid_h = GRID_ROWS as f32 * CELL_SIZE;
        let win_w = PAD_X + grid_w + SCROLLBAR_W + PAD_X;
        let win_h = TITLE_H + PAD_Y + grid_h + PAD_Y + FOOTER_H;
        (win_w, win_h)
    }
}

impl Window for CartWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
        self.container.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        self.container.set_texture_sizes(size_fn);
    }

    fn window_size(&self) -> (f32, f32) {
        self.dimensions()
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            ITEMWIN_MID_TEX,
            FOOTER_TEX,
            MINI_OFF_TEX,
            MINI_ON_TEX,
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

impl InGameWindow for CartWindow {
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let character = &mut *ctx.character;
        let data = ctx.data;
        if !character.cart.is_open() {
            self.qty_dialog = None;
            return Vec::new();
        }
        let mut events = Vec::new();
        let slot_count_table = data.item_slot_count.as_ref();
        let card_name_table = data.card_name.as_ref();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let text_color = text_color(grf);

        let (win_w, full_h) = self.dimensions();
        let win_h = if self.minimized { TITLE_H } else { full_h };

        let win = ui.window_at(CART_WINDOW_ID, win_w, win_h, TITLE_H, 80.0, 120.0);
        ui.interact(CART_WINDOW_ID, Rect::new(win.x, win.y, win_w, win_h));

        draw_titlebar(ui, win.x, win.y, win_w, TITLE_H, grf);
        ui.text(win.x + 17.0, win.y + TITLE_H - 3.0, "Cart", text_color);

        let mini_rect = Rect::new(
            win.x + win_w - MINI_BTN_SIZE * 2.0 - 6.0,
            win.y + (TITLE_H - MINI_BTN_SIZE) / 2.0,
            MINI_BTN_SIZE,
            MINI_BTN_SIZE,
        );
        let mini_resp = ui.interact(CART_MINI_BTN_ID, mini_rect);
        if mini_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            mini_rect,
            (MINI_BTN_SIZE, MINI_BTN_SIZE),
            mini_resp.hovered(),
            grf,
            MINI_ON_TEX,
            MINI_OFF_TEX,
            Some('_'),
        );
        if mini_resp.clicked() {
            self.minimized = !self.minimized;
        }

        let close_rect = Rect::new(
            win.x + win_w - MINI_BTN_SIZE - 3.0,
            win.y + (TITLE_H - MINI_BTN_SIZE) / 2.0,
            MINI_BTN_SIZE,
            MINI_BTN_SIZE,
        );
        let close_resp = ui.interact(CART_CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            close_rect,
            (MINI_BTN_SIZE, MINI_BTN_SIZE),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );
        if close_resp.clicked() {
            character.cart.close();
            ui.has_grf_textures = prev_grf;
            return events;
        }

        if self.minimized {
            ui.has_grf_textures = prev_grf;
            return events;
        }

        let container_y = win.y + TITLE_H;
        let container_h = PAD_Y + GRID_ROWS as f32 * CELL_SIZE + PAD_Y;
        draw_container(ui, win.x, container_y, win_w, container_h, grf);

        let grid_x = win.x + PAD_X;
        let grid_y = container_y + PAD_Y;
        let cell = CELL_SIZE;
        let icon = ICON_SIZE;
        let pad = ICON_PAD;

        let item_count = character.cart.all_items().len();
        let total_rows = item_count.div_ceil(GRID_COLS);
        let max_scroll = total_rows.saturating_sub(GRID_ROWS);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        {
            let items = character.cart.all_items();
            let start = self.scroll_offset * GRID_COLS;
            let visible_count = GRID_ROWS * GRID_COLS;

            for slot in 0..visible_count {
                let item_idx = start + slot;
                let col = slot % GRID_COLS;
                let row = slot / GRID_COLS;
                let cx = grid_x + col as f32 * cell;
                let cy = grid_y + row as f32 * cell;

                let cell_rect = Rect::new(cx, cy, cell, cell);
                let response = ui.interact(WidgetId(CART_ITEM_BASE_ID + slot as u32), cell_rect);

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

                if item_idx >= items.len() {
                    continue;
                }
                let item = &items[item_idx];

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
                ui.text(
                    cx + icon - count_w + 2.0,
                    cy + cell - 2.0,
                    &count_str,
                    [0.0, 0.0, 0.0, 1.0],
                );

                if response.hovered() {
                    let display_name =
                        format_equipment_display_name(item, slot_count_table, card_name_table);
                    let tooltip = if item.count > 1 {
                        format!("{display_name} {} ea", item.count)
                    } else {
                        display_name
                    };
                    ui.tooltip(cx, cy - icon, &tooltip);
                }

                if response.clicked() {
                    ui.drag_source(
                        CART_WINDOW_ID,
                        item.index as usize,
                        item.icon_path(),
                        (ICON_SIZE, ICON_SIZE),
                    );
                }
                if response.right_clicked() {
                    events.push(GameEvent::ShowItemInfo { index: item.index });
                }
                if response.double_clicked() {
                    events.push(GameEvent::RequestMoveItemCartToBody {
                        index: item.index,
                        count: item.count,
                    });
                }
            }
        }

        let grid_rect = Rect::new(
            grid_x,
            grid_y,
            GRID_COLS as f32 * CELL_SIZE,
            GRID_ROWS as f32 * CELL_SIZE,
        );
        if let Some((source_id, item_index)) = ui.drop_zone(grid_rect) {
            let index = item_index as u16;
            if source_id == INV_WINDOW_ID {
                if let Some(it) = character.inventory.get_item(index) {
                    let count = it.count;
                    if count > 1 {
                        self.open_qty_dialog(PendingCartMove::FromBody { index }, count);
                    } else {
                        events.push(GameEvent::RequestMoveItemBodyToCart { index, count: 1 });
                    }
                }
            } else if source_id == super::storage_window::STORAGE_WINDOW_ID {
                if let Some(it) = character.storage.get_item(index) {
                    let count = it.count;
                    if count > 1 {
                        self.open_qty_dialog(PendingCartMove::FromStore { index }, count);
                    } else {
                        events.push(GameEvent::RequestMoveItemStoreToCart { index, count: 1 });
                    }
                }
            }
        }

        if total_rows > GRID_ROWS {
            let sb_x = win.x + win_w - SCROLLBAR_W - 1.0;
            let content_rect = Rect::new(win.x, container_y, win_w, container_h);
            self.scroll_offset = scrollbar::scrollbar(
                ui,
                ScrollbarIds {
                    up: CART_SCROLL_UP_ID,
                    down: CART_SCROLL_DOWN_ID,
                    thumb: CART_SCROLL_THUMB_ID,
                },
                self.scroll_offset,
                GRID_ROWS,
                max_scroll,
                content_rect,
                sb_x,
                container_y,
                container_h,
            );
        }

        let footer_y = container_y + container_h;
        draw_footer(ui, win.x, footer_y, win_w, FOOTER_H, grf);
        let cart = &character.cart;
        let footer_label = format!("{}/{}", cart.weight / 10, cart.max_weight / 10);
        ui.text(
            win.x + 4.0,
            footer_y + FOOTER_H - 5.0,
            &footer_label,
            text_color,
        );

        if let Some((kind, dialog)) = &mut self.qty_dialog {
            match dialog.build(ui) {
                InputDialogResult::Submitted => {
                    let qty = dialog.value_i16().unwrap_or(0);
                    if qty > 0 {
                        match kind {
                            PendingCartMove::FromBody { index } => {
                                events.push(GameEvent::RequestMoveItemBodyToCart {
                                    index: *index,
                                    count: qty,
                                });
                            }
                            PendingCartMove::FromStore { index } => {
                                events.push(GameEvent::RequestMoveItemStoreToCart {
                                    index: *index,
                                    count: qty,
                                });
                            }
                        }
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
    use ragnarok_game::item::Item;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::frame::DragState;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = Box::leak(Box::new(FontAtlas::from_embedded(14.0, 1.0)));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

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
    fn dropping_a_stack_opens_dialog_then_moves_entered_count() {
        let mut win = CartWindow::new();
        let mut character = Character::new();
        character.cart.open();
        character.inventory.add_item(potion(3, 10));
        let data = DataTable::new();
        let mut state = StateCache::new();

        // Seed an inventory drag released over the cart grid (WidgetId(u32::MAX)
        // is the shared drag-state slot).
        {
            let drag = state.get_or_default::<DragState>(WidgetId(u32::MAX));
            drag.active = true;
            drag.source_id = INV_WINDOW_ID;
            drag.item_index = 3;
        }

        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 108.0;
        ctx.mouse_y = 157.0;
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestMoveItemBodyToCart { .. })),
            "a stack drop must open the dialog, not move immediately: {events:?}"
        );
        assert!(win.qty_dialog.is_some(), "quantity dialog should be open");

        win.qty_dialog.as_mut().unwrap().1.set_input_text("4");
        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.key_enter = true;
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            events.iter().any(|e| matches!(
                e,
                GameEvent::RequestMoveItemBodyToCart { index: 3, count: 4 }
            )),
            "submitting the dialog moves the entered count: {events:?}"
        );
        assert!(win.qty_dialog.is_none());
    }
}
