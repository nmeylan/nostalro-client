use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_game::event::{CharacterInfo, GameEvent};
use ragnarok_game::job_class::job_class_name;
use crate::Window;

const FALLBACK_WIN_W: f32 = 360.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

const HEADER_H: f32 = 22.0;
const LIST_X: f32 = 12.0;
const LIST_BOTTOM: f32 = 32.0;
const ROW_H: f32 = 17.0;
const ROW_PAD_LEFT: f32 = 5.0;
const ROW_PAD_TOP: f32 = 2.0;
const OK_BTN_RIGHT: f32 = 50.0;
const CANCEL_BTN_RIGHT: f32 = 5.0;
const BTN_BOTTOM: f32 = 4.0;

const WINDOW_ID: WidgetId = WidgetId(210);
const FALLBACK_TITLE_BAR_H: f32 = 30.0;

const OK_ID: WidgetId = WidgetId(200);
const CANCEL_ID: WidgetId = WidgetId(201);

const WIN_TEXTURE: &str = "data/texture/유저인터페이스/login_interface/win_select.bmp";

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

const SELECTED_COLOR: [f32; 4] = [0.804, 0.878, 1.0, 1.0];
const LIST_BG_COLOR: [f32; 4] = [0.969, 0.969, 0.969, 1.0];
const LIST_BORDER_COLOR: [f32; 4] = [0.78, 0.78, 0.78, 1.0];

pub struct CharSelectWindow {
    pub characters: Vec<CharacterInfo>,
    pub selected_index: Option<usize>,
    pub has_grf_textures: bool,
    win_size: (f32, f32),
    btn_size: (f32, f32),
}

impl CharSelectWindow {
    pub fn new(characters: Vec<CharacterInfo>) -> Self {
        let selected_index = if characters.is_empty() { None } else { Some(0) };
        Self {
            characters,
            selected_index,
            has_grf_textures: false,
            win_size: (FALLBACK_WIN_W, FALLBACK_WIN_W),
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if ui.ctx.key_down
            && let Some(idx) = self.selected_index
                && idx + 1 < self.characters.len() {
                    self.selected_index = Some(idx + 1);
                }
        if ui.ctx.key_up
            && let Some(idx) = self.selected_index
                && idx > 0 {
                    self.selected_index = Some(idx - 1);
                }

        if self.has_grf_textures {
            self.build_grf(ui, &mut events);
        } else {
            self.build_fallback(ui, &mut events);
        }

        if ui.ctx.key_enter
            && let Some(idx) = self.selected_index
                && let Some(ch) = self.characters.get(idx) {
                    events.push(GameEvent::RequestSelectCharacter { slot: ch.slot as u8 });
                }
        if ui.ctx.key_escape {
            events.push(GameEvent::BackToServerSelect);
        }

        events
    }

    fn build_grf(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let (win_w, win_h) = self.win_size;
        let (btn_w, btn_h) = self.btn_size;
        let win = ui.window(WINDOW_ID, win_w, win_h, HEADER_H );

        let (v, i) = draw::quad_vertices(win.x, win.y, win_w, win_h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(), indices: i.to_vec(),
            texture: TextureRef::Named(WIN_TEXTURE.to_string()),
        });

        let list_x = LIST_X ;
        let header_h = HEADER_H ;
        let list_w = win_w - list_x * 2.0;
        let list_h = win_h - header_h - (LIST_BOTTOM);
        let list_rect = Rect::new(win.x + list_x, win.y + header_h, list_w, list_h);
        let (v, i) = draw::quad_vertices(list_rect.x, list_rect.y, list_rect.w, list_rect.h, LIST_BG_COLOR);
        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });

        let b = 1.0;
        for (bx, by, bw, bh) in [
            (list_rect.x, list_rect.y, list_rect.w, b),
            (list_rect.x, list_rect.y + list_rect.h - b, list_rect.w, b),
            (list_rect.x, list_rect.y, b, list_rect.h),
            (list_rect.x + list_rect.w - b, list_rect.y, b, list_rect.h),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, LIST_BORDER_COLOR);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
        }

        let row_h = ROW_H ;
        let text_color = [0.0, 0.0, 0.0, 1.0];
        for (idx, ch) in self.characters.iter().enumerate() {
            let row_y = list_rect.y + (ROW_PAD_TOP) + idx as f32 * row_h;
            if row_y + row_h > list_rect.y + list_rect.h {
                break;
            }
            let row_rect = Rect::new(list_rect.x + 1.0, row_y, list_w - 2.0, row_h);
            let row = ui.interact(WidgetId(WINDOW_ID.0 + 10 + idx as u32), row_rect);
            if row.hovered() { ui.any_interactive_hovered = true; }
            if row.clicked() {
                self.selected_index = Some(idx);
            }

            if self.selected_index == Some(idx) {
                let (v, i) = draw::quad_vertices(row_rect.x, row_rect.y, row_rect.w, row_rect.h, SELECTED_COLOR);
                ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
            }

            let text_y = row_y + row_h - (3.0);
            let label = format!("{}  {}  Lv.{}", ch.name, job_class_name(ch.class), ch.base_level);
            ui.text(list_rect.x + (ROW_PAD_LEFT), text_y, &label, text_color);
        }

        let btn_y = win.y + win_h - (BTN_BOTTOM) - btn_h;
        let ok_rect = Rect::new(win.x + win_w - (OK_BTN_RIGHT) - btn_w, btn_y, btn_w, btn_h);
        let cancel_rect = Rect::new(win.x + win_w - (CANCEL_BTN_RIGHT) - btn_w, btn_y, btn_w, btn_h);
        let ok = ui.button(OK_ID, ok_rect, &OK_BTN, "OK");
        let cancel = ui.button(CANCEL_ID, cancel_rect, &CANCEL_BTN, "Cancel");

        if ok.clicked()
            && let Some(idx) = self.selected_index
                && let Some(ch) = self.characters.get(idx) {
                    events.push(GameEvent::RequestSelectCharacter { slot: ch.slot as u8 });
                }
        if cancel.clicked() {
            events.push(GameEvent::BackToServerSelect);
        }
    }

    fn build_fallback(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let row_h = ROW_H ;
        let list_h = self.characters.len().max(1) as f32 * row_h;
        let padding = 8.0 ;
        let title_h = 30.0 ;
        let detail_h = if self.selected_index.is_some() { 100.0  } else { 0.0 };
        let btn_h = FALLBACK_BTN_H ;
        let btn_w = FALLBACK_BTN_W ;
        let win_w = FALLBACK_WIN_W ;
        let win_h = title_h + list_h + padding + detail_h + padding + btn_h + padding;
        let win = ui.window(WINDOW_ID, win_w, win_h, FALLBACK_TITLE_BAR_H );

        // Window background
        let (v, i) = draw::quad_vertices(win.x, win.y, win.w, win_h, [0.08, 0.08, 0.12, 0.95]);
        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });

        // Border
        let border_color = [0.4, 0.4, 0.5, 1.0];
        let b = 1.0;
        for (bx, by, bw, bh) in [
            (win.x, win.y, win.w, b),
            (win.x, win.y + win_h - b, win.w, b),
            (win.x, win.y, b, win_h),
            (win.x + win.w - b, win.y, b, win_h),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
        }

        // Title
        let title = "Select Character";
        let title_w = ui.atlas.measure_text(title);
        let title_x = win.x + (win.w - title_w) / 2.0;
        let title_y = win.y + padding + ui.atlas.line_height;
        ui.text(title_x, title_y, title, [1.0, 1.0, 1.0, 1.0]);

        // Character list
        let list_y = win.y + title_h;
        let text_color = [1.0, 1.0, 1.0, 1.0];
        let dim_color = [0.7, 0.7, 0.7, 1.0];
        for (idx, ch) in self.characters.iter().enumerate() {
            let row_y = list_y + idx as f32 * row_h;
            let row_rect = Rect::new(win.x + padding, row_y, win.w - padding * 2.0, row_h);
            let row = ui.interact(WidgetId(WINDOW_ID.0 + 10 + idx as u32), row_rect);
            if row.hovered() { ui.any_interactive_hovered = true; }
            if row.clicked() {
                self.selected_index = Some(idx);
            }

            let bg_color = if self.selected_index == Some(idx) {
                [0.2, 0.2, 0.4, 1.0]
            } else if row.hovered() {
                [0.15, 0.15, 0.25, 1.0]
            } else {
                [0.0, 0.0, 0.0, 0.0]
            };
            if bg_color[3] > 0.0 {
                let (v, i) = draw::quad_vertices(row_rect.x, row_rect.y, row_rect.w, row_rect.h, bg_color);
                ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
            }

            let text_y = row_y + (row_h + ui.atlas.line_height) / 2.0 - (2.0);
            let label = format!("{}  {}  Lv.{}", ch.name, job_class_name(ch.class), ch.base_level);
            ui.text(win.x + padding + (4.0), text_y, &label, text_color);
        }

        // Detail panel for selected character
        let mut detail_bottom = list_y + list_h;
        if let Some(idx) = self.selected_index
            && let Some(ch) = self.characters.get(idx) {
                let detail_y = list_y + list_h + padding;
                detail_bottom = detail_y + detail_h;
                let line_h = ui.atlas.line_height + (2.0);
                let col1_x = win.x + padding + (4.0);
                let col2_x = win.x + win.w / 2.0 + (4.0);

                let mut y = detail_y + line_h;
                ui.text(col1_x, y, &format!("Job Lv.{}", ch.job_level), dim_color);
                ui.text(col2_x, y, &format!("Map: {}", ch.map), dim_color);

                y += line_h;
                ui.text(col1_x, y, &format!("HP: {}/{}", ch.hp, ch.max_hp), dim_color);
                ui.text(col2_x, y, &format!("SP: {}/{}", ch.sp, ch.max_sp), dim_color);

                y += line_h;
                ui.text(col1_x, y, &format!("STR {} AGI {} VIT {}", ch.str, ch.agi, ch.vit), dim_color);

                y += line_h;
                ui.text(col1_x, y, &format!("INT {} DEX {} LUK {}", ch.int, ch.dex, ch.luk), dim_color);
            }

        // OK / Cancel buttons
        let btn_y = detail_bottom + padding;
        let btn_spacing = 8.0 ;
        let total_btn_w = btn_w * 2.0 + btn_spacing;
        let btn_start_x = win.x + (win.w - total_btn_w) / 2.0;

        let ok_rect = Rect::new(btn_start_x, btn_y, btn_w, btn_h);
        let cancel_rect = Rect::new(btn_start_x + btn_w + btn_spacing, btn_y, btn_w, btn_h);
        let ok = ui.button(OK_ID, ok_rect, &OK_BTN, "OK");
        let cancel = ui.button(CANCEL_ID, cancel_rect, &CANCEL_BTN, "Cancel");

        if ok.clicked()
            && let Some(idx) = self.selected_index
                && let Some(ch) = self.characters.get(idx) {
                    events.push(GameEvent::RequestSelectCharacter { slot: ch.slot as u8 });
                }
        if cancel.clicked() {
            events.push(GameEvent::BackToServerSelect);
        }
    }

}

impl Window for CharSelectWindow {
    fn has_grf_textures(&self) -> bool { self.has_grf_textures }
    fn set_has_grf_textures(&mut self, value: bool) { self.has_grf_textures = value; }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(WIN_TEXTURE) {
            self.win_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(OK_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
            WIN_TEXTURE,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_renderer::font_atlas::FontAtlas;

    fn make_characters() -> Vec<CharacterInfo> {
        vec![
            CharacterInfo {
                gid: 1, name: "Knight".into(), class: 7, base_level: 50, job_level: 42,
                map: "prontera".into(), slot: 0, head: 1, hair_color: 0, weapon: 2,
                head_top: 0, head_mid: 0, head_bottom: 0, shield: 0, sex: 1,
                hp: 3000, max_hp: 3500, sp: 100, max_sp: 150,
                str: 50, agi: 30, vit: 40, int: 10, dex: 20, luk: 10,
            },
            CharacterInfo {
                gid: 2, name: "Wizard".into(), class: 9, base_level: 45, job_level: 38,
                map: "geffen".into(), slot: 1, head: 2, hair_color: 1, weapon: 10,
                head_top: 0, head_mid: 0, head_bottom: 0, shield: 0, sex: 0,
                hp: 1500, max_hp: 1800, sp: 400, max_sp: 500,
                str: 10, agi: 15, vit: 15, int: 60, dex: 40, luk: 5,
            },
            CharacterInfo {
                gid: 3, name: "Hunter".into(), class: 11, base_level: 60, job_level: 50,
                map: "payon".into(), slot: 2, head: 3, hair_color: 2, weapon: 11,
                head_top: 0, head_mid: 0, head_bottom: 0, shield: 0, sex: 1,
                hp: 2200, max_hp: 2500, sp: 150, max_sp: 200,
                str: 20, agi: 60, vit: 20, int: 15, dex: 55, luk: 30,
            },
        ]
    }

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn arrow_keys_navigate_selection() {
        let mut win = CharSelectWindow::new(make_characters());
        let mut state = StateCache::new();
        assert_eq!(win.selected_index, Some(0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_down = true;
        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(1));

        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(2));

        // Down at end stays at last
        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(2));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_up = true;
        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_index, Some(1));
    }

    #[test]
    fn enter_emits_request_select_character_with_correct_slot() {
        let mut win = CharSelectWindow::new(make_characters());
        let mut state = StateCache::new();
        win.selected_index = Some(1);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        // slot 1 corresponds to the second character (Wizard, slot=1)
        assert!(events.iter().any(|e| matches!(e, GameEvent::RequestSelectCharacter { slot: 1 })));
    }

    #[test]
    fn escape_emits_back_to_server_select() {
        let mut win = CharSelectWindow::new(make_characters());
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(events.iter().any(|e| matches!(e, GameEvent::BackToServerSelect)));
    }

    #[test]
    fn empty_list_has_no_selection() {
        let win = CharSelectWindow::new(vec![]);
        assert_eq!(win.selected_index, None);
    }
}
