use crate::Window;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_titlebar,
    text_color,
};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const ITEM_LIST_SELECTION_WINDOW_ID: WidgetId = WidgetId(2100);
const OK_ID: WidgetId = WidgetId(2101);
const CANCEL_ID: WidgetId = WidgetId(2102);
const OVERLAY_ID: WidgetId = WidgetId(2103);
const SCROLL_UP_ID: WidgetId = WidgetId(2104);
const SCROLL_DOWN_ID: WidgetId = WidgetId(2105);
const SCROLL_THUMB_ID: WidgetId = WidgetId(2106);
const RESIZE_ID: WidgetId = WidgetId(2107);
const ROW_BASE_ID: u32 = 2110;

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

const WIN_W: f32 = 200.0;
const TITLE_H: f32 = 17.0;
const ROW_H: f32 = 20.0;
const ICON_SIZE: f32 = 18.0;
const PAD: f32 = 6.0;
const FOOTER_H: f32 = 28.0;
const DEFAULT_VISIBLE_ROWS: usize = 4;
const MIN_VISIBLE_ROWS: usize = 2;
const MAX_VISIBLE_ROWS: usize = 16;
const GRIP_W: f32 = 40.0;
const GRIP_H: f32 = 6.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;
const SELECTED_COLOR: [f32; 4] = [0.3, 0.3, 0.5, 0.5];

#[derive(Clone, PartialEq)]
pub enum ListContext {
    MakingArrow,
    ElementalConverter,
    WeaponRefine,
    RepairWeapon { target_aid: u32 },
    Identify,
    AutoSpell,
    SelectPetEgg,
}

#[derive(Clone)]
pub struct ListRow {
    pub name: String,
    pub icon: Option<String>,
    pub index: i16,
    pub item_id: u16,
    pub refine: u8,
    pub cards: [u16; 4],
    pub skill_id: i32,
}

#[derive(Default)]
pub struct ItemListSelectionWindow {
    has_grf_textures: bool,
    open: bool,
    title: String,
    context: Option<ListContext>,
    rows: Vec<ListRow>,
    selected: usize,
    scroll_offset: usize,
    visible_rows: usize,
    resizing: bool,
    resize_start_mouse: f32,
    resize_start_rows: usize,
    btn_size: (f32, f32),
}

impl ItemListSelectionWindow {
    pub fn new() -> Self {
        Self {
            visible_rows: DEFAULT_VISIBLE_ROWS,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            ..Default::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, title: impl Into<String>, context: ListContext, rows: Vec<ListRow>) {
        self.title = title.into();
        self.context = Some(context);
        self.open = !rows.is_empty();
        self.rows = rows;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    fn close(&mut self) {
        self.open = false;
        self.rows.clear();
        self.context = None;
    }

    /// The server locks the character (menuskill) while the list is open and only
    /// clears it on a reply. Closing must still answer with an invalid sentinel so
    /// the server's cancel path runs without performing the action.
    pub fn cancel(&mut self, events: &mut Vec<GameEvent>) {
        if let Some(context) = self.context.clone() {
            let event = match context {
                ListContext::MakingArrow | ListContext::ElementalConverter => {
                    GameEvent::RequestMakingArrow { item_id: 0 }
                }
                ListContext::WeaponRefine => GameEvent::RequestWeaponRefine { index: 0 },
                ListContext::RepairWeapon { .. } => GameEvent::RequestRepairItem {
                    index: -1,
                    item_id: 0,
                    refine: 0,
                    cards: [0; 4],
                },
                ListContext::Identify => GameEvent::RequestIdentifyItem { index: -1 },
                ListContext::AutoSpell => GameEvent::RequestSelectAutoSpell { skill_id: 0 },
                // Hatching is an item-use, not a menuskill; cancelling needs no reply.
                ListContext::SelectPetEgg => {
                    self.close();
                    return;
                }
            };
            events.push(event);
        }
        self.close();
    }

    fn confirm(&mut self, events: &mut Vec<GameEvent>) {
        let (row, context) = match (self.rows.get(self.selected), self.context.clone()) {
            (Some(row), Some(context)) => (row, context),
            _ => {
                self.close();
                return;
            }
        };
        let event = match context {
            ListContext::MakingArrow | ListContext::ElementalConverter => {
                GameEvent::RequestMakingArrow {
                    item_id: row.item_id,
                }
            }
            ListContext::WeaponRefine => GameEvent::RequestWeaponRefine {
                index: row.index as i32,
            },
            ListContext::RepairWeapon { .. } => GameEvent::RequestRepairItem {
                index: row.index,
                item_id: row.item_id,
                refine: row.refine,
                cards: row.cards,
            },
            ListContext::Identify => GameEvent::RequestIdentifyItem { index: row.index },
            ListContext::AutoSpell => GameEvent::RequestSelectAutoSpell {
                skill_id: row.skill_id,
            },
            ListContext::SelectPetEgg => GameEvent::RequestSelectPetEgg {
                index: row.index as u16,
            },
        };
        events.push(event);
        self.close();
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();

        if ui.ctx.key_up && self.selected > 0 {
            self.selected -= 1;
        }
        if ui.ctx.key_down && self.selected + 1 < self.rows.len() {
            self.selected += 1;
        }
        if ui.ctx.key_enter {
            self.confirm(&mut events);
            return events;
        }
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let (btn_w, btn_h) = self.btn_size;

        let visible_rows = self.visible_rows.clamp(MIN_VISIBLE_ROWS, MAX_VISIBLE_ROWS);
        let max_scroll = self.rows.len().saturating_sub(visible_rows);
        let list_h = visible_rows as f32 * ROW_H;
        let win_h = TITLE_H + PAD + list_h + PAD + FOOTER_H;
        let dx = ((ui.ctx.screen_width - WIN_W) / 2.0).max(0.0).floor();
        let dy = ((ui.ctx.screen_height - win_h) / 2.0).max(0.0).floor();

        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(OVERLAY_ID, screen);
        ui.interact(
            ITEM_LIST_SELECTION_WINDOW_ID,
            Rect::new(dx, dy, WIN_W, win_h),
        );

        draw_titlebar(ui, dx, dy, WIN_W, TITLE_H, grf);
        ui.text(dx + 17.0, dy + TITLE_H - 3.0, &self.title, tc);

        let body_y = dy + TITLE_H;
        let body_h = PAD + list_h + PAD;
        draw_container(ui, dx, body_y, WIN_W, body_h, grf);
        draw_footer(ui, dx, dy + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        let has_scroll = max_scroll > 0;
        let list_w = WIN_W - PAD * 2.0 - if has_scroll { SCROLLBAR_W } else { 0.0 };
        let list_x = dx + PAD;
        let list_y = body_y + PAD;

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
        let mut confirm_now = false;
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
            if resp.double_clicked() {
                clicked = Some(idx);
                confirm_now = true;
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
            ui.text(text_x, row_y + ROW_H - 5.0, &row.name, tc);
        }
        if let Some(idx) = clicked {
            self.selected = idx;
        }
        if confirm_now {
            self.confirm(&mut events);
            ui.has_grf_textures = prev_grf;
            return events;
        }

        let win_rect = Rect::new(dx, dy, WIN_W, win_h);
        let btns = win_rect.buttons_bottom_right(2, btn_w, btn_h, 5.0, 5.0, 3.0);
        let cancel = ui.button(CANCEL_ID, btns[0], &CANCEL_BTN, "Cancel");
        let ok = ui.button(OK_ID, btns[1], &OK_BTN, "OK");
        if ok.clicked() {
            self.confirm(&mut events);
        } else if cancel.clicked() {
            self.cancel(&mut events);
        }

        let grip_rect = Rect::new(dx + PAD, dy + win_h - GRIP_H, GRIP_W, GRIP_H);
        ui.interact(RESIZE_ID, grip_rect);
        let grip_hovered = grip_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y);
        if grip_hovered {
            ui.any_interactive_hovered = true;
        }
        if grip_hovered && ui.ctx.mouse_clicked {
            self.resizing = true;
            self.resize_start_mouse = ui.ctx.mouse_y;
            self.resize_start_rows = visible_rows;
        }
        if !ui.ctx.mouse_down {
            self.resizing = false;
        }
        if self.resizing {
            let delta = ((ui.ctx.mouse_y - self.resize_start_mouse) / ROW_H).round() as i32;
            self.visible_rows = (self.resize_start_rows as i32 + delta)
                .clamp(MIN_VISIBLE_ROWS as i32, MAX_VISIBLE_ROWS as i32)
                as usize;
        }
        let grip_color = if grip_hovered || self.resizing {
            [0.6, 0.6, 0.7, 0.9]
        } else {
            [0.5, 0.5, 0.6, 0.8]
        };
        let (gv, gi) = draw::quad_vertices(grip_rect.x, grip_rect.y + 2.0, GRIP_W, 2.0, grip_color);
        ui.draw_calls.push(DrawCall {
            vertices: gv.to_vec(),
            indices: gi.to_vec(),
            texture: TextureRef::White,
        });

        ui.has_grf_textures = prev_grf;
        events
    }
}

impl Window for ItemListSelectionWindow {
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

    fn row(item_id: u16, index: i16) -> ListRow {
        ListRow {
            name: format!("Item {item_id}"),
            icon: None,
            index,
            item_id,
            refine: 0,
            cards: [0; 4],
            skill_id: 0,
        }
    }

    #[test]
    fn enter_confirms_selected_row_by_context() {
        let mut win = ItemListSelectionWindow::new();
        win.open(
            "Identify",
            ListContext::Identify,
            vec![row(0, 3), row(0, 7)],
        );
        win.selected = 1;

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(!win.is_open());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestIdentifyItem { index: 7 }))
        );
    }

    #[test]
    fn escape_sends_cancel_reply_to_unlock_menuskill() {
        let mut win = ItemListSelectionWindow::new();
        win.open("Make Arrow", ListContext::MakingArrow, vec![row(1750, 0)]);

        let mut events = Vec::new();
        win.cancel(&mut events);

        assert!(!win.is_open());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestMakingArrow { item_id: 0 }))
        );
    }

    #[test]
    fn arrow_context_emits_making_arrow_with_item_id() {
        let mut win = ItemListSelectionWindow::new();
        win.open("Make Arrow", ListContext::MakingArrow, vec![row(1750, 0)]);

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestMakingArrow { item_id: 1750 }))
        );
    }
}
