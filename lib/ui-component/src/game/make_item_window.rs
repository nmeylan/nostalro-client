use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    ITEMWIN_MID_TEX, SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer,
    draw_titlebar, text_color,
};
use crate::game::inventory_window::INV_WINDOW_ID;
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const MAKE_ITEM_WINDOW_ID: WidgetId = WidgetId(2200);
const MAKE_ID: WidgetId = WidgetId(2201);
const CANCEL_ID: WidgetId = WidgetId(2202);
const OK_ID: WidgetId = WidgetId(2203);
const SCROLL_UP_ID: WidgetId = WidgetId(2204);
const SCROLL_DOWN_ID: WidgetId = WidgetId(2205);
const SCROLL_THUMB_ID: WidgetId = WidgetId(2206);
const SLOT_BASE_ID: u32 = 2220;
const ROW_BASE_ID: u32 = 2230;

const MAKE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_make.bmp",
    hover: "data/texture/유저인터페이스/btn_make_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_make_b.bmp",
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
const SLOT_BG_TEX: &str = ITEMWIN_MID_TEX;

const LIST_TITLE: &str = "Item list you can craft";
const WIN_W: f32 = 240.0;
const TITLE_H: f32 = 17.0;
const ROW_H: f32 = 20.0;
const ICON_SIZE: f32 = 18.0;
const HEADER_ICON: f32 = 24.0;
const HEADER_H: f32 = 24.0;
const PAD: f32 = 6.0;
const SLOT_SIZE: f32 = 28.0;
const FOOTER_H: f32 = 28.0;
const MAX_VISIBLE_ROWS: usize = 7;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;
const SELECTED_COLOR: [f32; 4] = [0.3, 0.3, 0.5, 0.5];

/// The elemental stones, metals and Star Crumb are made without a material
/// slot; every other producible (weapons, potions) shows the three slots.
fn is_weapon_target_item(item_id: u16) -> bool {
    !matches!(item_id, 994..=1000)
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Phase {
    #[default]
    List,
    Process,
}

#[derive(Clone)]
struct ProducibleRow {
    item_id: u16,
    name: String,
    icon: Option<String>,
}

#[derive(Clone)]
struct MaterialSlot {
    item_id: u16,
    icon: Option<String>,
}

#[derive(Default)]
pub struct MakeItemWindow {
    has_grf_textures: bool,
    open: bool,
    phase: Phase,
    rows: Vec<ProducibleRow>,
    selected: usize,
    scroll_offset: usize,
    slots: [Option<MaterialSlot>; 3],
    btn_size: (f32, f32),
}

impl MakeItemWindow {
    pub fn new() -> Self {
        Self {
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            ..Default::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, rows: Vec<(u16, String, Option<String>)>) {
        self.rows = rows
            .into_iter()
            .map(|(item_id, name, icon)| ProducibleRow {
                item_id,
                name,
                icon,
            })
            .collect();
        self.selected = 0;
        self.scroll_offset = 0;
        self.slots = [None, None, None];
        self.phase = Phase::List;
        self.open = !self.rows.is_empty();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.phase = Phase::List;
        self.rows.clear();
        self.slots = [None, None, None];
    }

    /// Both phases share this height so the list's OK button and the process
    /// window's Make button sit at the same screen position (spam-click friendly).
    /// Anchored to whichever phase needs more room so neither is clipped: the
    /// process phase's header+slot block is a fixed floor the list must respect.
    fn window_height(&self) -> f32 {
        let visible_rows = self.rows.len().min(MAX_VISIBLE_ROWS);
        let list_h = TITLE_H + PAD + visible_rows as f32 * ROW_H + PAD + FOOTER_H;
        let process_h = TITLE_H + PAD + HEADER_H + PAD + SLOT_SIZE + PAD + FOOTER_H;
        list_h.max(process_h)
    }

    fn build_list(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let (btn_w, btn_h) = self.btn_size;

        let visible_rows = self.rows.len().min(MAX_VISIBLE_ROWS);
        let max_scroll = self.rows.len().saturating_sub(visible_rows);
        let list_h = visible_rows as f32 * ROW_H;
        let win_h = self.window_height();

        let win = ui.window_at(MAKE_ITEM_WINDOW_ID, WIN_W, win_h, TITLE_H, 260.0, 140.0);
        let (dx, dy) = (win.x, win.y);
        ui.interact(MAKE_ITEM_WINDOW_ID, Rect::new(dx, dy, WIN_W, win_h));

        draw_titlebar(ui, dx, dy, WIN_W, TITLE_H, grf);
        ui.text(dx + 17.0, dy + TITLE_H - 3.0, LIST_TITLE, tc);

        let body_y = dy + TITLE_H;
        let body_h = win_h - TITLE_H - FOOTER_H;
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
            ui.text(text_x, row_y + ROW_H - 5.0, &row.name, tc);
        }
        if let Some(idx) = clicked {
            self.selected = idx;
        }

        let win_rect = Rect::new(dx, dy, WIN_W, win_h);
        let btns = win_rect.buttons_bottom_right(2, btn_w, btn_h, 5.0, 5.0, 3.0);
        let cancel = ui.button(CANCEL_ID, btns[0], &CANCEL_BTN, "Cancel");
        let ok = ui.button(OK_ID, btns[1], &OK_BTN, "OK");
        if ok.clicked() {
            if self.rows.get(self.selected).is_some() {
                self.phase = Phase::Process;
                self.slots = [None, None, None];
            }
        } else if cancel.clicked() {
            events.push(GameEvent::RequestMakingItem {
                item_id: 0,
                materials: [0; 3],
            });
            self.close();
        }
    }

    fn build_process(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        events: &mut Vec<GameEvent>,
    ) {
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let (btn_w, btn_h) = self.btn_size;

        let Some(target) = self.rows.get(self.selected).cloned() else {
            self.close();
            return;
        };
        let show_slots = is_weapon_target_item(target.item_id);
        let win_h = self.window_height();
        let body_h = win_h - TITLE_H - FOOTER_H;

        let win = ui.window_at(MAKE_ITEM_WINDOW_ID, WIN_W, win_h, TITLE_H, 260.0, 140.0);
        let (dx, dy) = (win.x, win.y);
        ui.interact(MAKE_ITEM_WINDOW_ID, Rect::new(dx, dy, WIN_W, win_h));

        draw_titlebar(ui, dx, dy, WIN_W, TITLE_H, grf);
        ui.text(
            dx + 17.0,
            dy + TITLE_H - 3.0,
            &format!("{} Create", target.name),
            tc,
        );

        let body_y = dy + TITLE_H;
        draw_container(ui, dx, body_y, WIN_W, body_h, grf);
        draw_footer(ui, dx, dy + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        let hx = dx + PAD;
        let hy = body_y + PAD;
        let mut text_x = hx;
        if let Some(icon) = &target.icon {
            let iy = hy + (HEADER_H - HEADER_ICON) / 2.0;
            let (v, i) = draw::quad_vertices(hx, iy, HEADER_ICON, HEADER_ICON, [1.0; 4]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(icon.clone()),
            });
            text_x += HEADER_ICON + 6.0;
        }
        ui.text(text_x, hy + HEADER_H - 6.0, &target.name, tc);

        if show_slots {
            let slots_y = hy + HEADER_H + PAD;
            for s in 0..3 {
                let slot_x = hx + s as f32 * (SLOT_SIZE + PAD);
                let slot_rect = Rect::new(slot_x, slots_y, SLOT_SIZE, SLOT_SIZE);
                if grf {
                    let (v, i) =
                        draw::quad_vertices(slot_x, slots_y, SLOT_SIZE, SLOT_SIZE, [1.0; 4]);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: i.to_vec(),
                        texture: TextureRef::Named(SLOT_BG_TEX.to_string()),
                    });
                } else {
                    let (v, i) = draw::quad_vertices(
                        slot_x,
                        slots_y,
                        SLOT_SIZE,
                        SLOT_SIZE,
                        [0.1, 0.1, 0.15, 0.9],
                    );
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: i.to_vec(),
                        texture: TextureRef::White,
                    });
                }
                let resp = ui.interact(WidgetId(SLOT_BASE_ID + s as u32), slot_rect);
                if resp.hovered() {
                    ui.any_interactive_hovered = true;
                }
                if resp.clicked() && self.slots[s].is_some() {
                    self.slots[s] = None;
                }
                if let Some((source_id, inv_index)) = ui.drop_zone(slot_rect)
                    && source_id == INV_WINDOW_ID
                    && let Some(item) = character.inventory.get_item(inv_index as u16)
                {
                    self.slots[s] = Some(MaterialSlot {
                        item_id: item.item_id,
                        icon: item.icon_path(),
                    });
                }
                if let Some(slot) = &self.slots[s]
                    && let Some(icon) = &slot.icon
                {
                    let pad = (SLOT_SIZE - ICON_SIZE) / 2.0;
                    let (v, i) =
                        draw::quad_vertices(slot_x + pad, slots_y + pad, ICON_SIZE, ICON_SIZE, [1.0; 4]);
                    ui.draw_calls.push(DrawCall {
                        vertices: v.to_vec(),
                        indices: i.to_vec(),
                        texture: TextureRef::Named(icon.clone()),
                    });
                }
            }
        }

        let win_rect = Rect::new(dx, dy, WIN_W, win_h);
        let btns = win_rect.buttons_bottom_right(2, btn_w, btn_h, 5.0, 5.0, 3.0);
        let cancel = ui.button(CANCEL_ID, btns[0], &CANCEL_BTN, "Cancel");
        let make = ui.button(MAKE_ID, btns[1], &MAKE_BTN, "Make");
        if make.clicked() {
            let materials = [
                self.slots[0].as_ref().map(|s| s.item_id).unwrap_or(0),
                self.slots[1].as_ref().map(|s| s.item_id).unwrap_or(0),
                self.slots[2].as_ref().map(|s| s.item_id).unwrap_or(0),
            ];
            events.push(GameEvent::RequestMakingItem {
                item_id: target.item_id,
                materials,
            });
            self.close();
        } else if cancel.clicked() {
            // The server keeps the character locked (menuskill) until it gets a
            // making-item reply; answer with itemId 0 so its cancel path clears it.
            events.push(GameEvent::RequestMakingItem {
                item_id: 0,
                materials: [0; 3],
            });
            self.close();
        }
    }
}

impl Window for MakeItemWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(MAKE_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            SLOT_BG_TEX,
            MAKE_BTN.normal,
            MAKE_BTN.hover,
            MAKE_BTN.pressed,
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

impl InGameWindow for MakeItemWindow {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        character: &mut Character,
        _data: &DataTable,
    ) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;

        match self.phase {
            Phase::List => self.build_list(ui, &mut events),
            Phase::Process => self.build_process(ui, character, &mut events),
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

    #[test]
    fn builds_both_phases_without_panicking() {
        let mut win = MakeItemWindow::new();
        win.open(vec![
            (501, "Red Potion".into(), None),
            (503, "Yellow Potion".into(), None),
        ]);
        assert!(win.is_open());

        let mut character = Character::new();
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);

        let mut ui = make_frame(&ctx, &mut state);
        assert!(win.build(&mut ui, &mut character, &DataTable::new()).is_empty());

        win.phase = Phase::Process;
        let mut ui = make_frame(&ctx, &mut state);
        assert!(win.build(&mut ui, &mut character, &DataTable::new()).is_empty());

        win.close();
        assert!(!win.is_open());
    }
}
