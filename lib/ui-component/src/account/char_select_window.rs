use crate::Window;
use ragnarok_game::event::{CharacterInfo, GameEvent};
use ragnarok_game::job_class::job_class_name;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

const WIN_W: f32 = 576.0;
const WIN_H: f32 = 342.0;
const TITLE_BAR_H: f32 = 22.0;

const SLOTS_PER_PAGE: usize = 3;
const DEFAULT_MAX_SLOTS: usize = 9;

const SLOT_W: f32 = 126.0;
const SLOT_H: f32 = 132.0;
const SLOT_TOP: f32 = 44.0;
const SLOT_LEFTS: [f32; 3] = [60.0, 224.0, 386.0];
const SPRITE_ANCHOR_X: f32 = 63.0;
const SPRITE_ANCHOR_Y: f32 = 110.0;

const BOX_W: f32 = 139.0;
const BOX_H: f32 = 144.0;
const BOX_TOP: f32 = 40.0;
const BOX_DX: f32 = -5.0;

const ARROW_W: f32 = 13.0;
const ARROW_H: f32 = 13.0;
const ARROW_TOP: f32 = 105.0;
const ARROW_MARGIN: f32 = 40.0;

const BTN_W: f32 = 42.0;
const BTN_H: f32 = 20.0;
const BTN_BOTTOM: f32 = 4.0;
const OK_RIGHT: f32 = 50.0;
const CANCEL_RIGHT: f32 = 4.0;
const DEL_LEFT: f32 = 4.0;

const PANEL_Y: f32 = 204.0;
const VAL_COL1_X: f32 = 68.0;
const VAL_COL2_X: f32 = 216.0;
const ROW_YS: [f32; 6] = [2.0, 18.0, 34.0, 50.0, 66.0, 82.0];

const WINDOW_ID: WidgetId = WidgetId(210);
const OK_ID: WidgetId = WidgetId(200);
const CANCEL_ID: WidgetId = WidgetId(201);
const MAKE_ID: WidgetId = WidgetId(202);
const DEL_ID: WidgetId = WidgetId(203);
const ARROW_L_ID: WidgetId = WidgetId(204);
const ARROW_R_ID: WidgetId = WidgetId(205);
const DEL_CONFIRM_ID: WidgetId = WidgetId(206);
const DEL_CANCEL_ID: WidgetId = WidgetId(207);
const DEL_INPUT_ID: WidgetId = WidgetId(208);

const DIALOG_W: f32 = 300.0;
const DIALOG_H: f32 = 140.0;
const BIRTHDATE_MAX_LEN: usize = 8;

const WIN_TEXTURE: &str = "data/texture/유저인터페이스/login_interface/win_select.bmp";
const BOX_TEXTURE: &str = "data/texture/유저인터페이스/login_interface/box_select.bmp";
const ARROW_L_TEXTURE: &str = "data/texture/유저인터페이스/scroll1left.bmp";
const ARROW_R_TEXTURE: &str = "data/texture/유저인터페이스/scroll1right.bmp";

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
const MAKE_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_make.bmp",
    hover: "data/texture/유저인터페이스/btn_make_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_make_b.bmp",
};
const DEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_del.bmp",
    hover: "data/texture/유저인터페이스/btn_del_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_del_b.bmp",
};
const ARROW_L_BTN: ButtonTextures = ButtonTextures {
    normal: ARROW_L_TEXTURE,
    hover: ARROW_L_TEXTURE,
    pressed: ARROW_L_TEXTURE,
};
const ARROW_R_BTN: ButtonTextures = ButtonTextures {
    normal: ARROW_R_TEXTURE,
    hover: ARROW_R_TEXTURE,
    pressed: ARROW_R_TEXTURE,
};

const TEXT_COLOR: [f32; 4] = [0.15, 0.16, 0.32, 1.0];
const SLOT_BORDER_COLOR: [f32; 4] = [0.78, 0.80, 0.86, 1.0];

/// One occupied, on-screen slot the client uses to place the animated sprite.
pub struct CharSlotView {
    pub char_index: usize,
    pub anchor: [f32; 2],
    pub selected: bool,
}

/// Open only while a character is queued for deletion and awaiting the birthdate
/// confirmation (the reserved-deletion flow).
struct DeleteDialog {
    gid: u32,
    birthdate: TextInput,
    error: Option<String>,
}

pub struct CharSelectWindow {
    pub characters: Vec<CharacterInfo>,
    pub max_slots: usize,
    page: usize,
    selected_col: usize,
    pub has_grf_textures: bool,
    win_origin: (f32, f32),
    delete_dialog: Option<DeleteDialog>,
    delete_status: Option<String>,
}

impl CharSelectWindow {
    pub fn new(characters: Vec<CharacterInfo>) -> Self {
        let first_slot = characters.iter().map(|c| c.slot.max(0) as usize).min();
        let (page, selected_col) = match first_slot {
            Some(slot) => (slot / SLOTS_PER_PAGE, slot % SLOTS_PER_PAGE),
            None => (0, 0),
        };
        Self {
            characters,
            max_slots: DEFAULT_MAX_SLOTS,
            page,
            selected_col,
            has_grf_textures: false,
            win_origin: (0.0, 0.0),
            delete_dialog: None,
            delete_status: None,
        }
    }

    /// Restore the previously selected slot (from persisted config), placing the
    /// cursor on its page. Ignored if the slot is out of range.
    pub fn preselect_slot(&mut self, slot: Option<u8>) {
        if let Some(slot) = slot {
            let slot = slot as usize;
            if slot < self.max_slots {
                self.page = slot / SLOTS_PER_PAGE;
                self.selected_col = slot % SLOTS_PER_PAGE;
            }
        }
    }

    pub fn open_delete_dialog(&mut self, gid: u32, _delete_reserved_date: i32) {
        // Idempotent: a server reserve-ack arriving after the user opened the dialog
        // and started typing must not reset the birthdate field.
        if self.delete_dialog.as_ref().map(|d| d.gid) == Some(gid) {
            return;
        }
        self.delete_status = None;
        self.delete_dialog = Some(DeleteDialog {
            gid,
            birthdate: TextInput::new(BIRTHDATE_MAX_LEN, false),
            error: None,
        });
    }

    pub fn close_delete_dialog(&mut self) {
        self.delete_dialog = None;
    }

    pub fn set_delete_dialog_error(&mut self, msg: String) {
        if let Some(dialog) = &mut self.delete_dialog {
            dialog.error = Some(msg);
        }
    }

    pub fn set_delete_status(&mut self, msg: String) {
        self.delete_status = Some(msg);
    }

    pub fn remove_character(&mut self, gid: u32) {
        self.characters.retain(|c| c.gid != gid);
    }

    fn pages(&self) -> usize {
        self.max_slots.div_ceil(SLOTS_PER_PAGE).max(1)
    }

    fn selected_slot(&self) -> usize {
        self.page * SLOTS_PER_PAGE + self.selected_col
    }

    fn character_at(&self, slot: usize) -> Option<&CharacterInfo> {
        self.characters.iter().find(|c| c.slot.max(0) as usize == slot)
    }

    fn move_selection(&mut self, delta: i32) {
        let count = self.max_slots as i32;
        if count == 0 {
            return;
        }
        let cur = self.selected_slot() as i32;
        let next = (cur + delta).rem_euclid(count) as usize;
        self.page = next / SLOTS_PER_PAGE;
        self.selected_col = next % SLOTS_PER_PAGE;
    }

    /// Occupied slots on the current page, in screen space, for the sprite pass.
    pub fn visible_slot_views(&self) -> Vec<CharSlotView> {
        let mut out = Vec::new();
        for col in 0..SLOTS_PER_PAGE {
            let slot = self.page * SLOTS_PER_PAGE + col;
            if let Some(idx) = self
                .characters
                .iter()
                .position(|c| c.slot.max(0) as usize == slot)
            {
                out.push(CharSlotView {
                    char_index: idx,
                    anchor: [
                        self.win_origin.0 + SLOT_LEFTS[col] + SPRITE_ANCHOR_X,
                        self.win_origin.1 + SLOT_TOP + SPRITE_ANCHOR_Y,
                    ],
                    selected: col == self.selected_col,
                });
            }
        }
        out
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let modal = self.delete_dialog.is_some();

        if !modal {
            if ui.ctx.key_left {
                self.move_selection(-1);
            }
            if ui.ctx.key_right {
                self.move_selection(1);
            }
        }

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        if self.has_grf_textures {
            self.build_grf(ui, &mut events);
        } else {
            self.build_fallback(ui, &mut events);
        }
        ui.has_grf_textures = prev_grf;

        if modal {
            self.build_delete_dialog(ui, &mut events);
            return events;
        }

        let slot = self.selected_slot();
        let occupied = self.character_at(slot).is_some();
        if ui.ctx.key_enter {
            events.push(if occupied {
                GameEvent::RequestSelectCharacter { slot: slot as u8 }
            } else {
                GameEvent::RequestCreateCharacter { slot: slot as u8 }
            });
        }
        if ui.ctx.key_escape {
            events.push(GameEvent::BackToServerSelect);
        }

        events
    }

    fn build_grf(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let win = ui.window(WINDOW_ID, WIN_W, WIN_H, TITLE_BAR_H);
        self.win_origin = (win.x, win.y);

        push_quad(ui, win.x, win.y, WIN_W, WIN_H, TextureRef::Named(WIN_TEXTURE.to_string()));

        self.build_slots(ui, win.x, win.y, true);
        self.build_arrows(ui, win.x, win.y);
        self.build_info_panel(ui, win.x, win.y);
        self.build_buttons(ui, win.x, win.y, events);
    }

    fn build_fallback(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let win = ui.window(WINDOW_ID, WIN_W, WIN_H, TITLE_BAR_H);
        self.win_origin = (win.x, win.y);

        push_color_quad(ui, win.x, win.y, WIN_W, WIN_H, [0.08, 0.08, 0.12, 0.95]);
        let title = "Character Select";
        let tw = ui.atlas.measure_text(title);
        ui.text(
            win.x + (WIN_W - tw) / 2.0,
            win.y + ui.atlas.line_height,
            title,
            [1.0, 1.0, 1.0, 1.0],
        );

        self.build_slots(ui, win.x, win.y, false);
        self.build_arrows(ui, win.x, win.y);
        self.build_info_panel(ui, win.x, win.y);
        self.build_buttons(ui, win.x, win.y, events);
    }

    fn build_slots(&mut self, ui: &mut UiFrame, ox: f32, oy: f32, grf: bool) {
        for col in 0..SLOTS_PER_PAGE {
            let slot_rect = Rect::new(ox + SLOT_LEFTS[col], oy + SLOT_TOP, SLOT_W, SLOT_H);

            if !grf {
                push_border(ui, slot_rect, SLOT_BORDER_COLOR);
            }

            let resp = ui.interact(WidgetId(WINDOW_ID.0 + 10 + col as u32), slot_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() && self.delete_dialog.is_none() {
                self.selected_col = col;
            }

            if col == self.selected_col {
                if grf {
                    push_quad(
                        ui,
                        ox + SLOT_LEFTS[col] + BOX_DX,
                        oy + BOX_TOP,
                        BOX_W,
                        BOX_H,
                        TextureRef::Named(BOX_TEXTURE.to_string()),
                    );
                } else {
                    push_border(ui, slot_rect, [0.5, 0.62, 0.9, 1.0]);
                }
            }
        }
    }

    fn build_arrows(&mut self, ui: &mut UiFrame, ox: f32, oy: f32) {
        if self.pages() <= 1 || self.delete_dialog.is_some() {
            return;
        }
        let left = Rect::new(ox + ARROW_MARGIN, oy + ARROW_TOP, ARROW_W, ARROW_H);
        let right = Rect::new(
            ox + WIN_W - ARROW_MARGIN - ARROW_W,
            oy + ARROW_TOP,
            ARROW_W,
            ARROW_H,
        );
        if ui.button(ARROW_L_ID, left, &ARROW_L_BTN, "<").clicked() {
            self.move_selection(-(SLOTS_PER_PAGE as i32));
        }
        if ui.button(ARROW_R_ID, right, &ARROW_R_BTN, ">").clicked() {
            self.move_selection(SLOTS_PER_PAGE as i32);
        }
    }

    fn build_info_panel(&mut self, ui: &mut UiFrame, ox: f32, oy: f32) {
        let Some(ch) = self.character_at(self.selected_slot()) else {
            return;
        };
        let left = [
            ch.name.clone(),
            job_class_name(ch.class),
            ch.base_level.to_string(),
            ch.base_exp.to_string(),
            format!("{} / {}", ch.hp, ch.max_hp),
            format!("{} / {}", ch.sp, ch.max_sp),
        ];
        let right = [
            ch.str.to_string(),
            ch.agi.to_string(),
            ch.vit.to_string(),
            ch.int.to_string(),
            ch.dex.to_string(),
            ch.luk.to_string(),
        ];
        for (i, y) in ROW_YS.iter().enumerate() {
            let ty = oy + PANEL_Y + y + ui.atlas.line_height / 1.5;
            ui.text(ox + VAL_COL1_X, ty, &left[i], TEXT_COLOR);
            ui.text(ox + VAL_COL2_X, ty, &right[i], TEXT_COLOR);
        }
    }

    fn build_buttons(&mut self, ui: &mut UiFrame, ox: f32, oy: f32, events: &mut Vec<GameEvent>) {
        let btn_y = oy + WIN_H - BTN_BOTTOM - BTN_H;
        let modal = self.delete_dialog.is_some();
        let slot = self.selected_slot();
        let selected = self.character_at(slot);
        let occupied = selected.is_some();
        let selected_gid = selected.map(|c| c.gid);

        let cancel = Rect::new(ox + WIN_W - CANCEL_RIGHT - BTN_W, btn_y, BTN_W, BTN_H);
        if ui.button(CANCEL_ID, cancel, &CANCEL_BTN, "Cancel").clicked() && !modal {
            events.push(GameEvent::BackToServerSelect);
        }

        let action = Rect::new(ox + WIN_W - OK_RIGHT - BTN_W, btn_y, BTN_W, BTN_H);
        if occupied {
            if ui.button(OK_ID, action, &OK_BTN, "OK").clicked() && !modal {
                events.push(GameEvent::RequestSelectCharacter { slot: slot as u8 });
            }
            let del = Rect::new(ox + DEL_LEFT, btn_y, BTN_W, BTN_H);
            if ui.button(DEL_ID, del, &DEL_BTN, "del").clicked() && !modal {
                if let Some(gid) = selected_gid {
                    self.open_delete_dialog(gid, 0);
                    events.push(GameEvent::RequestDeleteCharacterReserve { gid });
                }
            }
        } else if ui.button(MAKE_ID, action, &MAKE_BTN, "Make").clicked() && !modal {
            events.push(GameEvent::RequestCreateCharacter { slot: slot as u8 });
        }

        if let Some(msg) = &self.delete_status {
            ui.text(ox + DEL_LEFT, btn_y - ui.atlas.line_height, msg, [0.85, 0.3, 0.3, 1.0]);
        }
    }

    fn build_delete_dialog(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let (ox, oy) = self.win_origin;
        let dx = ox + (WIN_W - DIALOG_W) / 2.0;
        let dy = oy + (WIN_H - DIALOG_H) / 2.0;

        push_color_quad(ui, ox, oy, WIN_W, WIN_H, [0.0, 0.0, 0.0, 0.55]);
        push_color_quad(ui, dx, dy, DIALOG_W, DIALOG_H, [0.11, 0.12, 0.18, 1.0]);
        push_border(ui, Rect::new(dx, dy, DIALOG_W, DIALOG_H), [0.5, 0.62, 0.9, 1.0]);

        let lh = ui.atlas.line_height;
        let name = self
            .delete_dialog
            .as_ref()
            .and_then(|d| self.characters.iter().find(|c| c.gid == d.gid))
            .map(|c| c.name.clone())
            .unwrap_or_default();
        ui.text(dx + 12.0, dy + 8.0 + lh, &format!("Delete {name}?"), [1.0; 4]);
        ui.text(
            dx + 12.0,
            dy + 8.0 + lh * 2.5,
            "Enter birthdate (YYMMDD) to confirm:",
            [0.85, 0.86, 0.92, 1.0],
        );

        let dialog = self.delete_dialog.as_mut().unwrap();
        let input_rect = Rect::new(dx + 12.0, dy + 8.0 + lh * 3.5, DIALOG_W - 24.0, 18.0);
        ui.text_input(DEL_INPUT_ID, input_rect, &mut dialog.birthdate, TextInputBg::Default);

        if let Some(err) = &dialog.error {
            ui.text(dx + 12.0, dy + 8.0 + lh * 5.0, err, [0.85, 0.3, 0.3, 1.0]);
        }

        let gid = dialog.gid;
        let birthdate = dialog.birthdate.text.clone();
        let btn_y = dy + DIALOG_H - BTN_BOTTOM - BTN_H;
        let confirm = Rect::new(dx + DIALOG_W - OK_RIGHT - BTN_W, btn_y, BTN_W, BTN_H);
        let cancel = Rect::new(dx + DIALOG_W - CANCEL_RIGHT - BTN_W, btn_y, BTN_W, BTN_H);

        let submit = ui.button(DEL_CONFIRM_ID, confirm, &OK_BTN, "OK").clicked() || ui.ctx.key_enter;
        if submit {
            events.push(GameEvent::RequestDeleteCharacterConfirm { gid, birthdate });
        }
        if ui.button(DEL_CANCEL_ID, cancel, &CANCEL_BTN, "Cancel").clicked() || ui.ctx.key_escape {
            events.push(GameEvent::RequestDeleteCharacterCancel { gid });
        }
    }
}

fn push_quad(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, texture: TextureRef) {
    let (v, i) = draw::quad_vertices(x, y, w, h, [1.0, 1.0, 1.0, 1.0]);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture,
    });
}

fn push_color_quad(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let (v, i) = draw::quad_vertices(x, y, w, h, color);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
}

fn push_border(ui: &mut UiFrame, rect: Rect, color: [f32; 4]) {
    let b = 1.0;
    for (bx, by, bw, bh) in [
        (rect.x, rect.y, rect.w, b),
        (rect.x, rect.y + rect.h - b, rect.w, b),
        (rect.x, rect.y, b, rect.h),
        (rect.x + rect.w - b, rect.y, b, rect.h),
    ] {
        push_color_quad(ui, bx, by, bw, bh, color);
    }
}

impl Window for CharSelectWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            WIN_TEXTURE,
            BOX_TEXTURE,
            ARROW_L_TEXTURE,
            ARROW_R_TEXTURE,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
            MAKE_BTN.normal,
            MAKE_BTN.hover,
            MAKE_BTN.pressed,
            DEL_BTN.normal,
            DEL_BTN.hover,
            DEL_BTN.pressed,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn character(slot: i8, name: &str) -> CharacterInfo {
        CharacterInfo {
            gid: slot as u32 + 1,
            name: name.into(),
            class: 7,
            base_level: 50,
            base_exp: 12345,
            job_level: 42,
            map: "prontera".into(),
            slot,
            head: 1,
            hair_color: 0,
            weapon: 2,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            shield: 0,
            sex: 1,
            hp: 3000,
            max_hp: 3500,
            sp: 100,
            max_sp: 150,
            str: 50,
            agi: 30,
            vit: 40,
            int: 10,
            dex: 20,
            luk: 10,
            effect_state: 0,
            zeny: 0,
        }
    }

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn arrow_keys_move_selection_across_slots() {
        let mut win = CharSelectWindow::new(vec![character(0, "Knight"), character(2, "Hunter")]);
        let mut state = StateCache::new();
        assert_eq!(win.selected_slot(), 0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_right = true;
        let mut ui = make_frame(&ctx, &mut state);
        win.build(&mut ui);
        assert_eq!(win.selected_slot(), 1);
    }

    #[test]
    fn enter_on_occupied_slot_selects_that_slot() {
        let mut win = CharSelectWindow::new(vec![character(2, "Hunter")]);
        let mut state = StateCache::new();
        assert_eq!(win.selected_slot(), 2);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestSelectCharacter { slot: 2 }))
        );
    }

    #[test]
    fn enter_on_empty_slot_requests_create() {
        let mut win = CharSelectWindow::new(vec![character(0, "Knight")]);
        let mut state = StateCache::new();
        win.move_selection(1); // slot 1 is empty
        assert_eq!(win.selected_slot(), 1);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestCreateCharacter { slot: 1 }))
        );
    }

    #[test]
    fn escape_returns_to_server_select() {
        let mut win = CharSelectWindow::new(vec![character(0, "Knight")]);
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);
        assert!(events.iter().any(|e| matches!(e, GameEvent::BackToServerSelect)));
    }

    #[test]
    fn preselect_slot_restores_page_and_column() {
        let mut win = CharSelectWindow::new(vec![character(0, "Knight")]);
        assert_eq!(win.selected_slot(), 0);

        win.preselect_slot(Some(7)); // page 2, column 1
        assert_eq!(win.page, 2);
        assert_eq!(win.selected_col, 1);
        assert_eq!(win.selected_slot(), 7);

        win.preselect_slot(Some(99)); // out of range: ignored
        assert_eq!(win.selected_slot(), 7);
    }

    #[test]
    fn open_delete_dialog_confirms_and_blocks_selection() {
        let mut win = CharSelectWindow::new(vec![character(0, "Knight")]);
        let gid = win.characters[0].gid;
        win.open_delete_dialog(gid, 0);
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestDeleteCharacterConfirm { gid: g, .. } if *g == gid
        )));
        // While the dialog is open, Enter must not also fire a character selection.
        assert!(!events.iter().any(|e| matches!(e, GameEvent::RequestSelectCharacter { .. })));
    }

    #[test]
    fn delete_dialog_escape_cancels_without_leaving_screen() {
        let mut win = CharSelectWindow::new(vec![character(0, "Knight")]);
        let gid = win.characters[0].gid;
        win.open_delete_dialog(gid, 0);
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = win.build(&mut ui);

        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestDeleteCharacterCancel { gid: g } if *g == gid
        )));
        assert!(!events.iter().any(|e| matches!(e, GameEvent::BackToServerSelect)));
    }
}
