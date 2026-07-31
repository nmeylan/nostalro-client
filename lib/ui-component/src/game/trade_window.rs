use super::input_dialog::{InputDialog, InputDialogConfig, InputDialogResult};
use super::inventory_window::INV_WINDOW_ID;
use crate::helper::dialog_container::DialogContainer;
use crate::helper::window_chrome::{draw_container, draw_titlebar, text_color};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::data_table::DataTable;
use ragnarok_game::display_name::format_equipment_display_name;
use ragnarok_game::event::GameEvent;
use ragnarok_game::item::Item;
use ragnarok_game::trade::{TRADE_MAX_SLOTS, TRADE_ZENY_INDEX};
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const TRADE_WINDOW_ID: WidgetId = WidgetId(4100);
const LOCK_BTN_ID: WidgetId = WidgetId(4101);
const TRADE_BTN_ID: WidgetId = WidgetId(4102);
const CANCEL_BTN_ID: WidgetId = WidgetId(4103);
const ZENY_INPUT_ID: WidgetId = WidgetId(4105);
const MY_ROW_BASE: u32 = 4110;
const OTHER_ROW_BASE: u32 = 4120;
const NUM_DIALOG_BASE: u32 = 4130;

const BG_TEX: &str = "data/texture/유저인터페이스/basic_interface/exchange_bg2.bmp";
const BOX_TEX: &str = "data/texture/유저인터페이스/basic_interface/itemwin_mid.bmp";

const LOCK_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_ok.bmp",
    hover: "data/texture/유저인터페이스/btn_ok_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_ok_b.bmp",
};
const TRADE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_exchange.bmp",
    hover: "data/texture/유저인터페이스/btn_exchange_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_exchange_b.bmp",
};
const TRADE_BTN_DIS: &str = "data/texture/유저인터페이스/btn_exchange_dis.bmp";
const LOCK_BTN_DIS: &str = "data/texture/유저인터페이스/btn_ok_dis.bmp";
const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_cancel.bmp",
    hover: "data/texture/유저인터페이스/btn_cancel_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_cancel_b.bmp",
};

const WIN_W: f32 = 500.0;
const TITLE_H: f32 = 16.0;
const HEADER_H: f32 = 3.0;
const ROW_H: f32 = 28.0;
const ICON: f32 = 24.0;
const ZENY_H: f32 = 26.0;
const FOOTER_H: f32 = 30.0;
const COL_W: f32 = WIN_W / 2.0;
const BTN_W: f32 = 42.0;
const BTN_H: f32 = 20.0;
const NAME_MAX_CHARS: usize = 22;

const GREY: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
const LOCKED_BG: [f32; 4] = [0.78, 0.78, 0.78, 1.0];

fn encode_gid(gid: u32) -> String {
    const TABLE: &[u8; 10] = b"ROHUTNASEW";
    gid.to_string()
        .bytes()
        .map(|b| TABLE[(b - b'0') as usize] as char)
        .collect()
}

fn win_h() -> f32 {
    TITLE_H + HEADER_H + TRADE_MAX_SLOTS as f32 * ROW_H + ZENY_H + FOOTER_H
}

pub struct TradeWindow {
    pub has_grf_textures: bool,
    zeny_input: TextInput,
    container: DialogContainer,
    qty_dialog: Option<(u16, InputDialog)>,
}

impl Default for TradeWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            zeny_input: TextInput::new(10, false).with_numeric_only(true),
            container: DialogContainer::new(),
            qty_dialog: None,
        }
    }

    pub fn reset_input(&mut self) {
        self.zeny_input.text.clear();
        self.zeny_input.cursor_pos = 0;
        self.qty_dialog = None;
    }

    fn open_qty_dialog(&mut self, index: u16, max: i16) {
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

    fn draw_item_column(
        &self,
        ui: &mut UiFrame,
        items: &[Item],
        col_x: f32,
        top_y: f32,
        row_base: u32,
        data: &DataTable,
        grf: bool,
        tc: [f32; 4],
        locked: bool,
    ) {
        let slot_count_table = data.item_slot_count.as_ref();
        let card_name_table = data.card_name.as_ref();
        if locked {
            let (v, i) = draw::quad_vertices(
                col_x + 2.0,
                top_y,
                COL_W - 4.0,
                TRADE_MAX_SLOTS as f32 * ROW_H,
                LOCKED_BG,
            );
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }
        for row in 0..TRADE_MAX_SLOTS {
            let ry = top_y + row as f32 * ROW_H;
            let icon_y = ry + (ROW_H - ICON) / 2.0;
            if !locked && grf {
                let (v, i) = draw::quad_vertices(col_x + 2.0, icon_y, ICON, ICON, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(BOX_TEX.to_string()),
                });
            } else if !locked {
                crate::helper::fallback::slot_cell(ui, col_x + 2.0, icon_y, ICON, ICON);
            }
            let Some(item) = items.get(row) else {
                continue;
            };
            let row_rect = Rect::new(col_x, ry, COL_W, ROW_H);
            let resp = ui.interact(WidgetId(row_base + row as u32), row_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if let Some(icon_path) = item.icon_path() {
                let (v, i) = draw::quad_vertices(col_x + 2.0, icon_y, ICON, ICON, [1.0; 4]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::Named(icon_path),
                });
            }
            let name = format_equipment_display_name(item, slot_count_table, card_name_table);
            let short: String = name.chars().take(NAME_MAX_CHARS).collect();
            let name_color = if item.is_identified { tc } else { GREY };
            ui.text(
                col_x + ICON + 5.0,
                ry + ROW_H / 2.0 + 4.0,
                &short,
                name_color,
            );
            let cnt = item.count.to_string();
            let cw = ui.atlas.measure_text(&cnt);
            ui.text(
                col_x + 2.0 + ICON - cw + 2.0,
                icon_y + ICON - 2.0,
                &cnt,
                [0.0, 0.0, 0.0, 1.0],
            );
            if resp.hovered() {
                let tip = if item.count > 1 {
                    format!("{name} {} ea", item.count)
                } else {
                    name.clone()
                };
                ui.tooltip(col_x, ry - 4.0, &tip);
            }
        }
    }
}

fn draw_disabled(ui: &mut UiFrame, rect: Rect, tex: &str, label: &str, grf: bool, tc: [f32; 4]) {
    if grf {
        let (v, i) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, [1.0; 4]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(tex.to_string()),
        });
    } else {
        crate::helper::fallback::cell(ui, rect.x, rect.y, rect.w, rect.h, false);
        let lw = ui.atlas.measure_text(label);
        ui.text(
            rect.x + (rect.w - lw) / 2.0,
            rect.y + rect.h / 2.0 + 4.0,
            label,
            tc,
        );
    }
}

impl Window for TradeWindow {
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
        (WIN_W, win_h())
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            BG_TEX,
            BOX_TEX,
            LOCK_BTN.normal,
            LOCK_BTN.hover,
            LOCK_BTN.pressed,
            LOCK_BTN_DIS,
            TRADE_BTN.normal,
            TRADE_BTN.hover,
            TRADE_BTN.pressed,
            TRADE_BTN_DIS,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ];
        paths.extend(InputDialog::grf_texture_paths());
        paths
    }
}

impl InGameWindow for TradeWindow {
    fn owns_keyboard(&self, _ctx: &BuildCtx) -> bool {
        self.qty_dialog.is_some()
    }

    fn wants_escape(&self, ctx: &BuildCtx) -> bool {
        ctx.character.trade.is_active()
    }

    fn on_escape(&mut self, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        if self.qty_dialog.is_some() {
            self.qty_dialog = None;
            return Vec::new();
        }
        ctx.character.trade.reset();
        self.reset_input();
        vec![GameEvent::RequestCancelExchange]
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let character = &mut *ctx.character;
        let data = ctx.data;
        if !character.trade.is_active() {
            self.reset_input();
            return Vec::new();
        }
        let mut events = Vec::new();
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);

        if let Some((_, dialog)) = &self.qty_dialog {
            ui.set_modal(&[dialog.win_id()]);
        }

        let h = win_h();
        let win = ui.window_at(TRADE_WINDOW_ID, WIN_W, h, TITLE_H, 300.0, 100.0);
        let (x, y) = (win.x, win.y);
        ui.interact(TRADE_WINDOW_ID, Rect::new(x, y, WIN_W, h));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        let title = format!(
            "Trade : {}  Lv{} ({})",
            character.trade.partner_name(),
            character.trade.partner_level(),
            encode_gid(character.trade.partner_aid())
        );
        ui.text(x + 15.0, y + TITLE_H - 4.0, &title, tc);

        let body_y = y + TITLE_H;
        let body_h = HEADER_H + TRADE_MAX_SLOTS as f32 * ROW_H + ZENY_H + FOOTER_H;
        if grf {
            let (v, i) = draw::quad_vertices(x, body_y, WIN_W, body_h, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(BG_TEX.to_string()),
            });
        } else {
            draw_container(ui, x, body_y, WIN_W, body_h, grf);
        }

        let right_x = x + COL_W;
        let my_locked = character.trade.my_locked();
        let other_locked = character.trade.other_locked();

        // --- Item columns ---
        let items_top = body_y + HEADER_H;
        let my_items: Vec<Item> = character.trade.my_items().to_vec();
        let other_items: Vec<Item> = character.trade.other_items().to_vec();
        self.draw_item_column(
            ui,
            &my_items,
            x,
            items_top,
            MY_ROW_BASE,
            data,
            grf,
            tc,
            my_locked,
        );
        self.draw_item_column(
            ui,
            &other_items,
            right_x,
            items_top,
            OTHER_ROW_BASE,
            data,
            grf,
            tc,
            other_locked,
        );

        // --- Add items from inventory onto my column (drop zone) ---
        let my_col_rect = Rect::new(x, items_top, COL_W, TRADE_MAX_SLOTS as f32 * ROW_H);
        if !my_locked
            && let Some((source_id, item_index)) = ui.drop_zone(my_col_rect)
            && source_id == INV_WINDOW_ID
        {
            let index = item_index as u16;
            if !character.trade.has_my_index(index)
                && !character.trade.my_slots_full()
                && let Some(it) = character.inventory.get_item(index)
            {
                let count = it.count;
                if count > 1 {
                    self.open_qty_dialog(index, count);
                } else {
                    character.trade.set_pending_add(index, 1);
                    events.push(GameEvent::RequestAddExchangeItem { index, count: 1 });
                }
            }
        }

        // --- Zeny (last row of each column; committed on Lock) ---
        let zeny_y = items_top + (1 + TRADE_MAX_SLOTS) as f32 * ROW_H;
        let baseline = zeny_y - 2.0;
        if my_locked {
            let my_zeny_txt = format!("{}z", character.trade.my_zeny());
            let mw = ui.atlas.measure_text(&my_zeny_txt);
            ui.text(x + COL_W - 36.0 - mw, baseline, &my_zeny_txt, tc);
        } else {
            let input_rect = Rect::new(x + COL_W / 2.0 - 32.0, zeny_y + -18.0, COL_W / 2.0, 18.0);
            ui.text_input(
                ZENY_INPUT_ID,
                input_rect,
                &mut self.zeny_input,
                TextInputBg::Gray,
            );
        }
        let other_zeny_txt = format!("{}z", character.trade.other_zeny());
        let ow = ui.atlas.measure_text(&other_zeny_txt);
        ui.text(x + WIN_W - ow - 36.0, baseline, &other_zeny_txt, tc);

        // --- Footer buttons ---
        let footer_y = body_y + body_h - FOOTER_H;
        let by = footer_y + (FOOTER_H - BTN_H) / 2.0 + 2.0;
        let lock_rect = Rect::new(x + 6.0, by, BTN_W, BTN_H);
        if my_locked {
            draw_disabled(ui, lock_rect, LOCK_BTN_DIS, "Lock", grf, tc);
        } else if ui
            .button(LOCK_BTN_ID, lock_rect, &LOCK_BTN, "Lock")
            .clicked()
        {
            let amount: i64 = self
                .zeny_input
                .text
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0);
            let capped = amount.min(character.inventory.zeny as i64).max(0);
            if capped > 0 {
                character
                    .trade
                    .set_pending_add(TRADE_ZENY_INDEX, capped as i32);
                events.push(GameEvent::RequestAddExchangeItem {
                    index: TRADE_ZENY_INDEX,
                    count: capped as i32,
                });
            }
            events.push(GameEvent::RequestConcludeExchange);
        }

        let trade_rect = Rect::new(x + (WIN_W - BTN_W) / 2.0, by, BTN_W, BTN_H);
        if character.trade.both_locked() {
            if ui
                .button(TRADE_BTN_ID, trade_rect, &TRADE_BTN, "Trade")
                .clicked()
            {
                events.push(GameEvent::RequestExecExchange);
            }
        } else {
            draw_disabled(ui, trade_rect, TRADE_BTN_DIS, "Trade", grf, tc);
        }

        let cancel_rect = Rect::new(x + WIN_W - BTN_W - 6.0, by, BTN_W, BTN_H);
        let cancel = ui.button(CANCEL_BTN_ID, cancel_rect, &CANCEL_BTN, "Cancel");

        // --- Quantity dialog (modal) ---
        if let Some((index, dialog)) = &mut self.qty_dialog {
            match dialog.build(ui) {
                InputDialogResult::Submitted => {
                    let qty = dialog.value_i16().unwrap_or(0);
                    if qty > 0 {
                        let index = *index;
                        character.trade.set_pending_add(index, qty as i32);
                        events.push(GameEvent::RequestAddExchangeItem {
                            index,
                            count: qty as i32,
                        });
                    }
                    self.qty_dialog = None;
                }
                InputDialogResult::Cancel => self.qty_dialog = None,
                InputDialogResult::None => {}
            }
        }

        if cancel.clicked() {
            events.push(GameEvent::RequestCancelExchange);
            character.trade.reset();
            self.zeny_input.text.clear();
            self.zeny_input.cursor_pos = 0;
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = Box::leak(Box::new(FontAtlas::from_embedded(14.0, 1.0)));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    fn active_trade() -> Character {
        let mut c = Character::new();
        c.name = "Me".into();
        c.trade.begin("Bob".into(), 2000, 55, 60);
        c
    }

    #[test]
    fn lock_button_sends_conclude_then_cancel_resets() {
        let mut win = TradeWindow::new();
        let mut character = active_trade();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let footer_y = 100.0 + TITLE_H + HEADER_H + TRADE_MAX_SLOTS as f32 * ROW_H + ZENY_H;
        let by = footer_y + (FOOTER_H - BTN_H) / 2.0;

        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 300.0 + 6.0 + BTN_W / 2.0;
        ctx.mouse_y = by + BTN_H / 2.0;
        ctx.mouse_clicked = true;
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestConcludeExchange)),
            "lock must send conclude, got {events:?}"
        );

        // Cancel closes the window locally and notifies the server.
        let mut ctx = UiContext::new(1024.0, 768.0);
        ctx.mouse_x = 300.0 + WIN_W - BTN_W - 6.0 + BTN_W / 2.0;
        ctx.mouse_y = by + BTN_H / 2.0;
        ctx.mouse_clicked = true;
        let events = {
            let mut ui = make_frame(&ctx, &mut state);
            win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
        };
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestCancelExchange)),
            "cancel must notify server, got {events:?}"
        );
        assert!(!character.trade.is_active());
    }
}
