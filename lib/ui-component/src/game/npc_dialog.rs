use super::number_input::{NumberInputConfig, NumberInputDialog, NumberInputResult};
use crate::helper::dialog_container::DialogContainer;
use crate::helper::scrollbar::{self, ScrollbarIds};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_game::npc_dialog::{NpcDialogData, NpcDialogState};
use ragnarok_ui::draw::{self, DrawCall, TextureRef, strip_color_codes, word_wrap};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

const OVERLAY_ID: WidgetId = WidgetId(600);
pub const NPC_DIALOG_WINDOW_ID: WidgetId = WidgetId(610);
const NEXT_BTN_ID: WidgetId = WidgetId(601);
const CLOSE_BTN_ID: WidgetId = WidgetId(602);
const INPUT_ID: WidgetId = WidgetId(603);
const OK_BTN_ID: WidgetId = WidgetId(604);
const CANCEL_BTN_ID: WidgetId = WidgetId(605);
const MENU_OK_BTN_ID: WidgetId = WidgetId(606);
const BUY_BTN_ID: WidgetId = WidgetId(607);
const SELL_BTN_ID: WidgetId = WidgetId(608);
const DEAL_CANCEL_BTN_ID: WidgetId = WidgetId(609);
const MENU_BASE_ID: u32 = 620;
const SCROLL_UP_ID: WidgetId = WidgetId(640);
const SCROLL_DOWN_ID: WidgetId = WidgetId(641);
const SCROLL_THUMB_ID: WidgetId = WidgetId(642);

const DIALOG_W: f32 = 276.0;
const DIALOG_H: f32 = 176.0;
const MENU_W: f32 = 276.0;
const MENU_MIN_H: f32 = 116.0;
const PADDING: f32 = 8.0;
const TEXT_LINE_HEIGHT: f32 = 16.0;
const MENU_ITEM_HEIGHT: f32 = 18.0;
const MENU_VISIBLE_ROWS: usize = 5;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

const NEXT_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_next.bmp",
    hover: "data/texture/유저인터페이스/btn_next_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_next_b.bmp",
};

const CLOSE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_close.bmp",
    hover: "data/texture/유저인터페이스/btn_close_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_close_b.bmp",
};

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

const BUY_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_buy.bmp",
    hover: "data/texture/유저인터페이스/btn_buy_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_buy_b.bmp",
};

const SELL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_sell.bmp",
    hover: "data/texture/유저인터페이스/btn_sell_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_sell_b.bmp",
};

const WIN_TEXTURE: &str = "data/texture/유저인터페이스/win_msgbox.bmp";
const BTN_BOTTOM: f32 = 4.0;
const BTN_FIRST_RIGHT: f32 = 5.0;
const BTN_SPACING: f32 = 3.0;

pub struct NpcDialog {
    pub has_grf_textures: bool,
    pub movable: bool,
    pub dialog: NpcDialogData,
    pub string_input: TextInput,
    number_input_dialog: Option<NumberInputDialog>,
    btn_size: (f32, f32),
    container: DialogContainer,
    win_size: (f32, f32),
}

impl Default for NpcDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl NpcDialog {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            movable: false,
            dialog: NpcDialogData::new(),
            string_input: TextInput::new(70, false),
            number_input_dialog: None,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            container: DialogContainer::new(),
            win_size: (280.0, 120.0),
        }
    }
}

impl Window for NpcDialog {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(NEXT_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
        self.container.set_texture_sizes(size_fn);
        if let Some((w, h)) = size_fn(WIN_TEXTURE) {
            self.win_size = (w as f32, h as f32);
        }
    }

    fn window_size(&self) -> (f32, f32) {
        (DIALOG_W, DIALOG_H)
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = DialogContainer::grf_texture_paths();
        paths.extend_from_slice(&[
            WIN_TEXTURE,
            NEXT_BTN.normal,
            NEXT_BTN.hover,
            NEXT_BTN.pressed,
            CLOSE_BTN.normal,
            CLOSE_BTN.hover,
            CLOSE_BTN.pressed,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
            BUY_BTN.normal,
            BUY_BTN.hover,
            BUY_BTN.pressed,
            SELL_BTN.normal,
            SELL_BTN.hover,
            SELL_BTN.pressed,
        ]);
        paths.extend_from_slice(&scrollbar::grf_texture_paths());
        paths
    }
}

impl InGameWindow for NpcDialog {
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.dialog.is_open() {
            return Vec::new();
        }

        let mut events = Vec::new();
        let state = self.dialog.state;

        match state {
            NpcDialogState::WaitingForNext => {
                if ui.ctx.key_enter {
                    events.push(GameEvent::RequestNpcNext {
                        npc_id: self.dialog.npc_id,
                    });
                    self.dialog.advance_next();
                    return events;
                }
            }
            NpcDialogState::WaitingForClose => {
                if ui.ctx.key_enter || ui.ctx.key_escape {
                    events.push(GameEvent::RequestNpcClose {
                        npc_id: self.dialog.npc_id,
                    });
                    self.dialog.close();
                    return events;
                }
            }
            NpcDialogState::WaitingForMenu => {
                if ui.ctx.key_escape {
                    events.push(GameEvent::RequestNpcMenuSelect {
                        npc_id: self.dialog.npc_id,
                        choice: 255,
                    });
                    self.dialog.close();
                    return events;
                }
                let total_items = self.dialog.menu_items.len();
                if ui.ctx.key_up && self.dialog.selected_menu_index > 0 {
                    self.dialog.selected_menu_index -= 1;
                }
                if ui.ctx.key_down && self.dialog.selected_menu_index + 1 < total_items {
                    self.dialog.selected_menu_index += 1;
                }
                if ui.ctx.key_up || ui.ctx.key_down {
                    let offset = &mut self.dialog.menu_scroll_offset;
                    if *offset > self.dialog.selected_menu_index {
                        *offset = self.dialog.selected_menu_index;
                    } else if self.dialog.selected_menu_index >= *offset + MENU_VISIBLE_ROWS
                        && total_items > MENU_VISIBLE_ROWS
                    {
                        *offset = self.dialog.selected_menu_index + 1 - MENU_VISIBLE_ROWS;
                    }
                    let max_offset = total_items.saturating_sub(MENU_VISIBLE_ROWS);
                    *offset = (*offset).min(max_offset);
                }
                if ui.ctx.key_enter {
                    let choice = (self.dialog.selected_menu_index + 1) as u8;
                    events.push(GameEvent::RequestNpcMenuSelect {
                        npc_id: self.dialog.npc_id,
                        choice,
                    });
                    self.dialog.close();
                    return events;
                }
            }
            NpcDialogState::WaitingForNumberInput => {}
            NpcDialogState::WaitingForStringInput => {
                if ui.ctx.key_enter {
                    let text = self.string_input.text.clone();
                    events.push(GameEvent::RequestNpcInputString {
                        npc_id: self.dialog.npc_id,
                        text,
                    });
                    self.string_input.text.clear();
                    self.string_input.cursor_pos = 0;
                    self.dialog.close();
                    return events;
                }
            }
            NpcDialogState::WaitingForDealType => {
                if ui.ctx.key_escape {
                    self.dialog.close();
                    return events;
                }
            }
            _ => {}
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        self.container.has_grf_textures = self.has_grf_textures;

        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(OVERLAY_ID, screen);

        if state == NpcDialogState::WaitingForDealType {
            let result = self.build_deal_type_popup(ui);
            ui.has_grf_textures = prev_grf;
            return result;
        }

        let default_dx = (ui.ctx.screen_width / 3.0).max(20.0).floor();
        let default_dy = (ui.ctx.screen_height / 2.0 - 200.0).max(100.0).floor();

        let has_text = !self.dialog.text.is_empty();
        let menu_only = state == NpcDialogState::WaitingForMenu && !has_text;

        let padding = PADDING;
        let dialog_w = DIALOG_W;

        let mut dx = default_dx;

        if !menu_only {
            let text_area_w = dialog_w - padding * 2.0;
            let wrapped_lines = word_wrap(
                &self.dialog.text,
                text_area_w,
                |t| ui.atlas.measure_text(&strip_color_codes(t)),
                false,
            );
            let text_line_h = TEXT_LINE_HEIGHT;
            let text_h = (wrapped_lines.len().max(1) as f32) * text_line_h;

            let input_h = if state == NpcDialogState::WaitingForStringInput {
                30.0
            } else {
                0.0
            };

            let (btn_w, btn_h) = self.btn_size;
            let has_button = matches!(
                state,
                NpcDialogState::WaitingForNext
                    | NpcDialogState::WaitingForClose
                    | NpcDialogState::WaitingForStringInput
            );
            let btn_area_h = if has_button { btn_h + padding } else { 0.0 };

            let dialog_h = (padding + text_h + input_h + btn_area_h + padding).max(DIALOG_H);

            let dy = if self.movable {
                let win = ui.window_at(
                    NPC_DIALOG_WINDOW_ID,
                    dialog_w,
                    dialog_h,
                    dialog_h,
                    default_dx,
                    default_dy,
                );
                dx = win.x;
                win.y
            } else {
                default_dy
            };

            self.container.draw(
                &mut ui.draw_calls,
                dx,
                dy,
                dialog_w,
                dialog_h,
                [1.0, 1.0, 1.0, 0.95],
            );

            let text_color = self.container.text_color();
            let mut text_y = dy + padding + ui.atlas.line_height;
            for line in &wrapped_lines {
                ui.colored_text(dx + padding, text_y, line, text_color);
                text_y += text_line_h;
            }

            if state == NpcDialogState::WaitingForStringInput {
                let input_y = text_y + padding;
                let input_rect =
                    Rect::new(dx + padding, input_y, text_area_w - btn_w - padding, 22.0);
                if ui.focused() != Some(INPUT_ID) {
                    ui.set_focus(INPUT_ID);
                }
                ui.text_input(
                    INPUT_ID,
                    input_rect,
                    &mut self.string_input,
                    TextInputBg::Default,
                );

                let ok_rect = Rect::new(dx + dialog_w - padding - btn_w, input_y, btn_w, btn_h);
                let ok = ui.button(OK_BTN_ID, ok_rect, &OK_BTN, "OK");
                if ok.clicked() {
                    let text = self.string_input.text.clone();
                    events.push(GameEvent::RequestNpcInputString {
                        npc_id: self.dialog.npc_id,
                        text,
                    });
                    self.string_input.text.clear();
                    self.string_input.cursor_pos = 0;
                    self.dialog.close();
                    ui.has_grf_textures = prev_grf;
                    return events;
                }
            }

            let dialog_rect = Rect::new(dx, dy, dialog_w, dialog_h);
            let btns = dialog_rect.buttons_bottom_right(
                1,
                btn_w,
                btn_h,
                BTN_BOTTOM,
                BTN_FIRST_RIGHT,
                BTN_SPACING,
            );

            if state == NpcDialogState::WaitingForNext {
                let response = ui.button(NEXT_BTN_ID, btns[0], &NEXT_BTN, "Next");
                if response.clicked() {
                    events.push(GameEvent::RequestNpcNext {
                        npc_id: self.dialog.npc_id,
                    });
                    self.dialog.advance_next();
                }
            }
            if state == NpcDialogState::WaitingForClose {
                let response = ui.button(CLOSE_BTN_ID, btns[0], &CLOSE_BTN, "Close");
                if response.clicked() {
                    events.push(GameEvent::RequestNpcClose {
                        npc_id: self.dialog.npc_id,
                    });
                    self.dialog.close();
                }
            }
        } // !menu_only

        if state == NpcDialogState::WaitingForMenu {
            let menu_events = self.build_menu_window(ui, dx);
            events.extend(menu_events);
        }

        if state == NpcDialogState::WaitingForNumberInput {
            if self.number_input_dialog.is_none() {
                let mut dialog = NumberInputDialog::new(
                    NumberInputConfig {
                        label: Some("Input number".to_string()),
                        show_cancel: false,
                        escape_cancels: false,
                        default_value: String::new(),
                        max_len: 10,
                    },
                    WidgetId(INPUT_ID.0),
                );
                dialog.init_container(&self.container);
                self.number_input_dialog = Some(dialog);
            }
            let dialog = self.number_input_dialog.as_mut().unwrap();
            dialog.init_container(&self.container);
            if let NumberInputResult::Submitted = dialog.build(ui) {
                let value: i32 = dialog.value_i32().unwrap_or(0);
                events.push(GameEvent::RequestNpcInputNumber {
                    npc_id: self.dialog.npc_id,
                    value,
                });
                self.number_input_dialog = None;
                self.dialog.close();
            }
        } else {
            self.number_input_dialog = None;
        }

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl NpcDialog {
    fn build_menu_window(&mut self, ui: &mut UiFrame, dx: f32) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let (btn_w, btn_h) = self.btn_size;
        let menu_w = MENU_W;
        let padding = PADDING;
        let menu_item_h = MENU_ITEM_HEIGHT;
        let text_area_w = menu_w - padding * 2.0;

        let total_items = self.dialog.menu_items.len();
        let visible_rows = total_items.min(MENU_VISIBLE_ROWS);
        let needs_scroll = total_items > MENU_VISIBLE_ROWS;
        let content_h = visible_rows as f32 * menu_item_h;

        let menu_y = (ui.ctx.screen_height / 2.0 + (76.0)).max(376.0).floor();
        let menu_h = (padding + content_h + padding + btn_h + padding).max(MENU_MIN_H);

        self.container.draw(
            &mut ui.draw_calls,
            dx,
            menu_y,
            menu_w,
            menu_h,
            [1.0, 1.0, 1.0, 0.95],
        );

        let text_color = self.container.text_color();
        let offset = self.dialog.menu_scroll_offset;
        let end_idx = (offset + MENU_VISIBLE_ROWS).min(total_items);
        let item_text_w = if needs_scroll {
            text_area_w - scrollbar::SCROLLBAR_W
        } else {
            text_area_w
        };

        for idx in offset..end_idx {
            let row = idx - offset;
            let item_y = menu_y + padding + row as f32 * menu_item_h;
            let item_rect = Rect::new(dx + padding, item_y, item_text_w, menu_item_h);
            let widget_id = WidgetId(MENU_BASE_ID + idx as u32);
            let response = ui.interact(widget_id, item_rect);
            if response.hovered() {
                ui.any_interactive_hovered = true;
            }

            let is_selected = idx == self.dialog.selected_menu_index;

            if is_selected {
                let highlight = [0.3, 0.3, 0.5, 0.5];
                let (v, i) = draw::quad_vertices(
                    item_rect.x,
                    item_rect.y,
                    item_rect.w,
                    item_rect.h,
                    highlight,
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }

            let label = format!("{}. {}", idx + 1, &self.dialog.menu_items[idx]);
            ui.colored_text(
                dx + padding + (4.0),
                item_y + ui.atlas.line_height - (4.0),
                &label,
                text_color,
            );

            if response.clicked() {
                self.dialog.selected_menu_index = idx;
            }
        }

        if needs_scroll {
            let max_scroll = total_items - MENU_VISIBLE_ROWS;
            let scroll_ids = ScrollbarIds {
                up: SCROLL_UP_ID,
                down: SCROLL_DOWN_ID,
                thumb: SCROLL_THUMB_ID,
            };
            let content_rect = Rect::new(dx + padding, menu_y + padding, text_area_w, content_h);
            let scroll_x = dx + menu_w - scrollbar::SCROLLBAR_W - padding;
            self.dialog.menu_scroll_offset = scrollbar::scrollbar(
                ui,
                scroll_ids,
                offset,
                MENU_VISIBLE_ROWS,
                max_scroll,
                content_rect,
                scroll_x,
                menu_y + padding,
                content_h,
            );
        }

        let menu_rect = Rect::new(dx, menu_y, menu_w, menu_h);
        let menu_btns = menu_rect.buttons_bottom_right(
            2,
            btn_w,
            btn_h,
            BTN_BOTTOM,
            BTN_FIRST_RIGHT,
            BTN_SPACING,
        );

        let cancel = ui.button(CANCEL_BTN_ID, menu_btns[0], &CANCEL_BTN, "Cancel");
        let ok = ui.button(MENU_OK_BTN_ID, menu_btns[1], &OK_BTN, "OK");

        if ok.clicked() {
            let choice = (self.dialog.selected_menu_index + 1) as u8;
            events.push(GameEvent::RequestNpcMenuSelect {
                npc_id: self.dialog.npc_id,
                choice,
            });
            self.dialog.close();
        }
        if cancel.clicked() {
            events.push(GameEvent::RequestNpcMenuSelect {
                npc_id: self.dialog.npc_id,
                choice: 255,
            });
            self.dialog.close();
        }

        events
    }

    fn build_deal_type_popup(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let (btn_w, btn_h) = self.btn_size;
        let (dialog_w, dialog_h) = self.win_size;

        let dx = ((ui.ctx.screen_width - dialog_w) / 2.0).floor();
        let dy = (ui.ctx.screen_height / 1.5).floor();

        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(dx, dy, dialog_w, dialog_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(WIN_TEXTURE.to_string()),
            });
        } else {
            let (v, i) = draw::quad_vertices(dx, dy, dialog_w, dialog_h, [0.2, 0.2, 0.28, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
            let border_color = [0.5, 0.5, 0.6, 1.0];
            for (bx, by, bw, bh) in [
                (dx, dy, dialog_w, 1.0),
                (dx, dy + dialog_h - 1.0, dialog_w, 1.0),
                (dx, dy, 1.0, dialog_h),
                (dx + dialog_w - 1.0, dy, 1.0, dialog_h),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
        }

        let container = Rect::new(dx, dy, dialog_w, dialog_h);
        let btns = container.buttons_bottom_right(
            3,
            btn_w,
            btn_h,
            BTN_BOTTOM,
            BTN_FIRST_RIGHT,
            BTN_SPACING,
        );

        let message = "Please select a Deal type";
        let (text_y, text_x) =
            container.text_dialog_alignment(PADDING, btns[0].y, ui.atlas.line_height);
        let text_color = if self.has_grf_textures {
            [0.0, 0.0, 0.0, 1.0]
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        ui.text(text_x, text_y, message, text_color);

        let cancel = ui.button(DEAL_CANCEL_BTN_ID, btns[0], &CANCEL_BTN, "Cancel");
        let sell = ui.button(SELL_BTN_ID, btns[1], &SELL_BTN, "Sell");
        let buy = ui.button(BUY_BTN_ID, btns[2], &BUY_BTN, "Buy");

        if buy.clicked() {
            events.push(GameEvent::RequestNpcDealType {
                npc_id: self.dialog.npc_id,
                deal_type: 0,
            });
            self.dialog.close();
        }
        if sell.clicked() {
            events.push(GameEvent::RequestNpcDealType {
                npc_id: self.dialog.npc_id,
                deal_type: 1,
            });
            self.dialog.close();
        }
        if cancel.clicked() {
            self.dialog.close();
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InGameWindow;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
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
    fn enter_triggers_next() {
        let mut npc = NpcDialog::new();
        npc.dialog.open_text(100, "Hello");
        npc.dialog.wait_for_next(100);

        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);

        let events = npc.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            GameEvent::RequestNpcNext { npc_id: 100 }
        ));
        assert_eq!(npc.dialog.state, NpcDialogState::DisplayingText);
    }

    #[test]
    fn escape_cancels_menu() {
        let mut npc = NpcDialog::new();
        npc.dialog.show_menu(100, vec!["Buy".into(), "Sell".into()]);

        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);

        let events = npc.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(events.len(), 1);
        match &events[0] {
            GameEvent::RequestNpcMenuSelect { npc_id, choice } => {
                assert_eq!(*npc_id, 100);
                assert_eq!(*choice, 255);
            }
            other => panic!("expected RequestNpcMenuSelect, got {other:?}"),
        }
        assert!(!npc.dialog.is_open());
    }

    #[test]
    fn escape_cancels_deal_type() {
        let mut npc = NpcDialog::new();
        npc.dialog.show_deal_type(100);

        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);

        let events = npc.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(events.is_empty());
        assert!(!npc.dialog.is_open());
    }

    #[test]
    fn menu_with_many_items_keeps_scroll_offset_bounded() {
        let mut npc = NpcDialog::new();
        let items: Vec<String> = (1..=10).map(|i| format!("Item {i}")).collect();
        npc.dialog.show_menu(100, items);

        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);

        npc.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(npc.dialog.menu_scroll_offset, 0);
        assert_eq!(npc.dialog.selected_menu_index, 0);

        let mut ctx2 = UiContext::new(800.0, 600.0);
        ctx2.key_down = true;
        let mut ui2 = make_frame(&ctx2, &mut state);

        for _ in 0..6 {
            npc.build(&mut ui2, &mut crate::BuildCtx::test(&mut character, &data));
        }
        assert_eq!(npc.dialog.selected_menu_index, 6);
        assert_eq!(npc.dialog.menu_scroll_offset, 2);
    }

    #[test]
    fn menu_mouse_wheel_scrolls_and_persists() {
        let mut npc = NpcDialog::new();
        let items: Vec<String> = (1..=10).map(|i| format!("Item {i}")).collect();
        npc.dialog.show_menu(100, items);

        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);
        npc.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(npc.dialog.menu_scroll_offset, 0);

        let mut ctx2 = UiContext::new(800.0, 600.0);
        ctx2.mouse_x = 300.0;
        ctx2.mouse_y = 400.0;
        ctx2.scroll_delta = -1.0; // scroll down
        let mut ui2 = make_frame(&ctx2, &mut state);
        npc.build(&mut ui2, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(
            npc.dialog.menu_scroll_offset, 1,
            "mouse wheel should scroll down"
        );

        let ctx3 = UiContext::new(800.0, 600.0);
        let mut ui3 = make_frame(&ctx3, &mut state);
        npc.build(&mut ui3, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(
            npc.dialog.menu_scroll_offset, 1,
            "scroll offset must persist across frames"
        );
    }
}
