use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    FOOTER_TEX, ITEMWIN_MID_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_titlebar,
    text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

const OVERLAY_ID: WidgetId = WidgetId(1100);
pub const CARD_INSERT_WINDOW_ID: WidgetId = WidgetId(1101);
const OK_BTN_ID: WidgetId = WidgetId(1102);
const CANCEL_BTN_ID: WidgetId = WidgetId(1103);
const SCROLL_UP_ID: WidgetId = WidgetId(1104);
const SCROLL_DOWN_ID: WidgetId = WidgetId(1105);
const SCROLL_THUMB_ID: WidgetId = WidgetId(1106);
const ITEM_BASE_ID: u32 = 1110;

const WIN_W: f32 = 250.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 27.0;
const VISIBLE_ROWS: usize = 5;
const ITEM_ROW_H: f32 = 28.0;
const ICON_SIZE: f32 = 24.0;
const ICON_OFFSET_X: f32 = 4.0;
const ICON_OFFSET_Y: f32 = 2.0;
const PAD_LEFT: f32 = 4.0;
const PAD_Y: f32 = 2.0;
const BTN_BOTTOM: f32 = 4.0;
const BTN_FIRST_RIGHT: f32 = 5.0;
const BTN_SPACING: f32 = 3.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

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

pub struct EligibleItem {
    pub inventory_index: u16,
    pub display_name: String,
    pub icon_path: Option<String>,
}

pub struct CardInsertDialog {
    pub has_grf_textures: bool,
    card_index: u16,
    card_name: String,
    eligible_items: Vec<EligibleItem>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    open: bool,
    btn_size: (f32, f32),
}

impl Default for CardInsertDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl CardInsertDialog {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            card_index: 0,
            card_name: String::new(),
            eligible_items: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            open: false,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn window_size(&self) -> (f32, f32) {
        let visible = VISIBLE_ROWS.min(self.eligible_items.len()).max(1);
        let container_h = visible as f32 * ITEM_ROW_H + 2.0 * PAD_Y;
        (WIN_W, TITLE_H + container_h + FOOTER_H)
    }

    pub fn open(&mut self, card_index: u16, card_name: String, items: Vec<EligibleItem>) {
        self.card_index = card_index;
        self.card_name = card_name;
        self.eligible_items = items;
        self.selected_index = None;
        self.scroll_offset = 0;
        self.open = true;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn pending_texture_paths(&self) -> Vec<String> {
        self.eligible_items
            .iter()
            .filter_map(|item| item.icon_path.clone())
            .collect()
    }

    fn close(&mut self) {
        self.open = false;
        self.eligible_items.clear();
        self.selected_index = None;
        self.scroll_offset = 0;
    }
}

impl Window for CardInsertDialog {
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
            ITEMWIN_MID_TEX,
            FOOTER_TEX,
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

impl InGameWindow for CardInsertDialog {
    fn owns_keyboard(&self, _ctx: &BuildCtx) -> bool {
        self.open
    }

    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.open
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.close();
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        if !self.open {
            return vec![];
        }

        let mut events = Vec::new();
        let grf = self.has_grf_textures;

        if ui.ctx.key_enter && self.selected_index.is_some() {
            let idx = self.selected_index.unwrap();
            let equip_index = self.eligible_items[idx].inventory_index;
            let card_index = self.card_index;
            self.close();
            return vec![GameEvent::RequestCardInsert {
                card_index,
                equip_index,
            }];
        }

        let item_count = self.eligible_items.len();
        let visible = VISIBLE_ROWS.min(item_count).max(1);
        let container_h = visible as f32 * ITEM_ROW_H + 2.0 * PAD_Y;
        let win_h = TITLE_H + container_h + FOOTER_H;
        let win = ui.window(CARD_INSERT_WINDOW_ID, WIN_W, win_h, TITLE_H);

        draw_titlebar(ui, win.x, win.y, WIN_W, TITLE_H, grf);
        let title = format!("Card Insert - {}", self.card_name);
        let title_text_color = text_color(grf);
        ui.text(
            win.x + 20.0,
            win.y + TITLE_H - 4.0,
            &title,
            title_text_color,
        );

        let container_y = win.y + TITLE_H;
        draw_container(ui, win.x, container_y, WIN_W, container_h, grf);

        let has_scrollbar = item_count > visible;
        let scrollbar_w = if has_scrollbar { SCROLLBAR_W } else { 0.0 };
        let max_scroll = item_count.saturating_sub(visible);
        let content_rect = Rect::new(win.x, container_y, WIN_W, container_h);

        if has_scrollbar {
            let sb_x = win.x + WIN_W - SCROLLBAR_W - 1.0;
            self.scroll_offset = scrollbar::scrollbar(
                ui,
                ScrollbarIds {
                    up: SCROLL_UP_ID,
                    down: SCROLL_DOWN_ID,
                    thumb: SCROLL_THUMB_ID,
                },
                self.scroll_offset,
                visible,
                max_scroll,
                content_rect,
                sb_x,
                container_y,
                container_h,
            );
        }

        let list_y = container_y + PAD_Y;
        let row_content_w = WIN_W - PAD_LEFT - scrollbar_w;
        let name_x = win.x + PAD_LEFT + ICON_OFFSET_X + ICON_SIZE + 4.0;

        for i in 0..visible {
            let list_idx = self.scroll_offset + i;
            if list_idx >= item_count {
                break;
            }
            let item = &self.eligible_items[list_idx];
            let ry = list_y + i as f32 * ITEM_ROW_H;
            let row_rect = Rect::new(win.x + PAD_LEFT, ry, row_content_w, ITEM_ROW_H);
            let response = ui.interact(WidgetId(ITEM_BASE_ID + list_idx as u32), row_rect);

            if let Some(icon_path) = &item.icon_path {
                let ix = win.x + PAD_LEFT + ICON_OFFSET_X;
                let iy = ry + ICON_OFFSET_Y;
                let (v, idx) =
                    draw::quad_vertices(ix, iy, ICON_SIZE, ICON_SIZE, [1.0, 1.0, 1.0, 1.0]);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::Named(icon_path.clone()),
                });
            }

            let is_selected = self.selected_index == Some(list_idx);
            if is_selected {
                let (v, idx) = draw::quad_vertices(
                    row_rect.x,
                    ry,
                    row_content_w,
                    ITEM_ROW_H,
                    [0.3, 0.5, 0.8, 0.3],
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
            } else if response.hovered() {
                let (v, idx) = draw::quad_vertices(
                    row_rect.x,
                    ry,
                    row_content_w,
                    ITEM_ROW_H,
                    [0.4, 0.4, 0.6, 0.2],
                );
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: idx.to_vec(),
                    texture: TextureRef::White,
                });
            }

            let text_y = ry + ITEM_ROW_H - 8.0;
            ui.text(name_x, text_y, &item.display_name, text_color(grf));

            if response.clicked() {
                self.selected_index = Some(list_idx);
            }
            if response.double_clicked() {
                let equip_index = item.inventory_index;
                let card_index = self.card_index;
                self.close();
                return vec![GameEvent::RequestCardInsert {
                    card_index,
                    equip_index,
                }];
            }
        }

        let footer_y = container_y + container_h;
        draw_footer(ui, win.x, footer_y, WIN_W, FOOTER_H, grf);

        let (btn_w, btn_h) = self.btn_size;
        let footer_rect = Rect::new(win.x, footer_y, WIN_W, FOOTER_H);
        let btns = footer_rect.buttons_bottom_right(
            2,
            btn_w,
            btn_h,
            BTN_BOTTOM,
            BTN_FIRST_RIGHT,
            BTN_SPACING,
        );

        let cancel = ui.button(CANCEL_BTN_ID, btns[0], &CANCEL_BTN, "Cancel");
        let ok = ui.button(OK_BTN_ID, btns[1], &OK_BTN, "OK");

        if cancel.clicked() {
            self.close();
            events.push(GameEvent::DialogClosed);
        }
        if ok.clicked()
            && let Some(idx) = self.selected_index
        {
            let equip_index = self.eligible_items[idx].inventory_index;
            let card_index = self.card_index;
            self.close();
            events.push(GameEvent::RequestCardInsert {
                card_index,
                equip_index,
            });
        }

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
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    fn test_items() -> Vec<EligibleItem> {
        vec![
            EligibleItem {
                inventory_index: 1,
                display_name: "Sword [3]".into(),
                icon_path: None,
            },
            EligibleItem {
                inventory_index: 2,
                display_name: "Bow [2]".into(),
                icon_path: None,
            },
        ]
    }

    #[test]
    fn escape_closes_dialog() {
        let mut dialog = CardInsertDialog::new();
        dialog.open(10, "Poring Card".into(), test_items());
        assert!(dialog.is_open());

        let mut character = Character::new();
        let data = DataTable::new();
        let mut ctx = crate::BuildCtx::test(&mut character, &data);
        assert!(dialog.wants_escape(&ctx));
        dialog.on_escape(&mut ctx);

        assert!(!dialog.is_open());
    }

    #[test]
    fn enter_confirms_when_selected() {
        let mut dialog = CardInsertDialog::new();
        dialog.open(10, "Poring Card".into(), test_items());
        dialog.selected_index = Some(0);

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut character = Character::new();
        let data = DataTable::new();
        let mut ui = make_frame(&ctx, &mut state);
        let events = dialog.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        assert!(!dialog.is_open());
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestCardInsert {
                card_index: 10,
                equip_index: 1
            }
        )));
    }

    #[test]
    fn enter_does_nothing_when_no_selection() {
        let mut dialog = CardInsertDialog::new();
        dialog.open(10, "Poring Card".into(), test_items());

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut character = Character::new();
        let data = DataTable::new();
        let mut ui = make_frame(&ctx, &mut state);
        let events = dialog.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        assert!(dialog.is_open());
        assert!(events.is_empty());
    }

    #[test]
    fn closed_dialog_returns_no_events() {
        let mut dialog = CardInsertDialog::new();
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut character = Character::new();
        let data = DataTable::new();
        let mut ui = make_frame(&ctx, &mut state);
        let events = dialog.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(events.is_empty());
    }
}
