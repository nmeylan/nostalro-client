use std::cell::Cell;
use std::rc::Rc;

use super::confirm_dialog::{ConfirmDialog, ConfirmResult};
use crate::{InGameWindow, Window};
use ragnarok_game::character::Character;
use ragnarok_game::data_table::DataTable;
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

const BG_ID: WidgetId = WidgetId(510);
const RESUME_ID: WidgetId = WidgetId(500);
const OPTION_ID: WidgetId = WidgetId(501);
const CHARSELECT_ID: WidgetId = WidgetId(502);
const QUIT_ID: WidgetId = WidgetId(503);
const RESTART_ID: WidgetId = WidgetId(504);
const RESURRECT_ID: WidgetId = WidgetId(505);

const MENU_W: f32 = 140.0;
const FALLBACK_BTN_W: f32 = 120.0;
const FALLBACK_BTN_H: f32 = 24.0;
const BTN_SPACING: f32 = 4.0;
const PADDING_TOP: f32 = 12.0;
const PADDING_BOTTOM: f32 = 12.0;
const MENU_H: f32 = PADDING_TOP + 4.0 * FALLBACK_BTN_H + 3.0 * BTN_SPACING + PADDING_BOTTOM;

const WIN_TEXTURE: &str = "data/texture/유저인터페이스/basic_interface/titlebar_fix.bmp";

const RESUME_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/esc_02a.bmp",
    hover: "data/texture/유저인터페이스/esc_02b.bmp",
    pressed: "data/texture/유저인터페이스/esc_02c.bmp",
};
const CHARSELECT_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/esc_01a.bmp",
    hover: "data/texture/유저인터페이스/esc_01b.bmp",
    pressed: "data/texture/유저인터페이스/esc_01c.bmp",
};
const QUIT_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/esc_03a.bmp",
    hover: "data/texture/유저인터페이스/esc_03b.bmp",
    pressed: "data/texture/유저인터페이스/esc_03c.bmp",
};
const RESTART_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/esc_04a.bmp",
    hover: "data/texture/유저인터페이스/esc_04b.bmp",
    pressed: "data/texture/유저인터페이스/esc_04c.bmp",
};
const RESURRECT_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/esc_05a.bmp",
    hover: "data/texture/유저인터페이스/esc_05b.bmp",
    pressed: "data/texture/유저인터페이스/esc_05c.bmp",
};
const DUMMY_BTN: ButtonTextures = ButtonTextures {
    normal: "",
    hover: "",
    pressed: "",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingConfirm {
    None,
    CharacterSelect,
    QuitGame,
}

pub struct SystemMenu {
    pub open: bool,
    pub dead: bool,
    pub can_resurrect: bool,
    pub has_grf_textures: bool,
    pub allow_escape_toggle: bool,
    pending_confirm: PendingConfirm,
    confirm_dialog: ConfirmDialog,
    confirm_dialog_out_param: Rc<Cell<Option<ConfirmResult>>>,
    win_size: (f32, f32),
    btn_size: (f32, f32),
}

impl Default for SystemMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMenu {
    pub fn new() -> Self {
        Self {
            open: false,
            dead: false,
            can_resurrect: false,
            has_grf_textures: false,
            allow_escape_toggle: false,
            pending_confirm: PendingConfirm::None,
            confirm_dialog: ConfirmDialog::new(),
            confirm_dialog_out_param: Rc::new(Cell::new(None)),
            win_size: (MENU_W, MENU_H),
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }
}

impl Window for SystemMenu {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(RESUME_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(WIN_TEXTURE) {
            self.win_size = (w as f32, h as f32);
        }
        self.confirm_dialog.set_texture_sizes(size_fn);
        self.confirm_dialog.has_grf_textures = true;
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![
            WIN_TEXTURE,
            RESUME_BTN.normal,
            RESUME_BTN.hover,
            RESUME_BTN.pressed,
            CHARSELECT_BTN.normal,
            CHARSELECT_BTN.hover,
            CHARSELECT_BTN.pressed,
            QUIT_BTN.normal,
            QUIT_BTN.hover,
            QUIT_BTN.pressed,
            RESTART_BTN.normal,
            RESTART_BTN.hover,
            RESTART_BTN.pressed,
            RESURRECT_BTN.normal,
            RESURRECT_BTN.hover,
            RESURRECT_BTN.pressed,
        ];
        paths.extend(ConfirmDialog::grf_texture_paths());
        paths
    }
}

impl InGameWindow for SystemMenu {
    fn build(
        &mut self,
        ui: &mut UiFrame,
        _character: &mut Character,
        _data: &DataTable,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if self.allow_escape_toggle
            && ui.ctx.key_escape
            && !self.dead
            && self.pending_confirm == PendingConfirm::None
        {
            self.open = !self.open;
            if !self.open {
                return events;
            }
        }

        if !self.open {
            return events;
        }

        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(BG_ID, screen);

        if self.pending_confirm != PendingConfirm::None {
            let pending = self.pending_confirm;
            let out_param = Rc::clone(&self.confirm_dialog_out_param);
            self.confirm_dialog_out_param.set(None);
            self.confirm_dialog
                .show("Are you sure?", true, move |result| {
                    out_param.set(Some(result));
                });
            self.confirm_dialog.build(ui);
            if let Some(result) = self.confirm_dialog_out_param.get() {
                match result {
                    ConfirmResult::Ok => {
                        match pending {
                            PendingConfirm::CharacterSelect => {
                                events.push(GameEvent::BackToCharacterSelect)
                            }
                            PendingConfirm::QuitGame => events.push(GameEvent::QuitGame),
                            PendingConfirm::None => {}
                        }
                        self.pending_confirm = PendingConfirm::None;
                        self.open = false;
                    }
                    ConfirmResult::Cancel => {
                        self.pending_confirm = PendingConfirm::None;
                    }
                }
            } else if ui.ctx.key_escape {
                self.pending_confirm = PendingConfirm::None;
            }
            return events;
        }

        if self.has_grf_textures {
            self.build_grf(ui, &mut events);
        } else {
            self.build_fallback(ui, &mut events);
        }

        events
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuButton {
    Resume,
    Sound,
    CharSelect,
    Quit,
    Restart,
    Resurrect,
}

impl MenuButton {
    fn id(self) -> WidgetId {
        match self {
            MenuButton::Resume => RESUME_ID,
            MenuButton::Sound => OPTION_ID,
            MenuButton::CharSelect => CHARSELECT_ID,
            MenuButton::Quit => QUIT_ID,
            MenuButton::Restart => RESTART_ID,
            MenuButton::Resurrect => RESURRECT_ID,
        }
    }

    fn textures(self) -> &'static ButtonTextures {
        match self {
            MenuButton::Resume => &RESUME_BTN,
            MenuButton::CharSelect => &CHARSELECT_BTN,
            MenuButton::Quit => &QUIT_BTN,
            MenuButton::Restart => &RESTART_BTN,
            MenuButton::Resurrect => &RESURRECT_BTN,
            MenuButton::Sound => &DUMMY_BTN,
        }
    }

    fn label(self) -> &'static str {
        match self {
            MenuButton::Resume => "Resume",
            MenuButton::Sound => "Sound",
            MenuButton::CharSelect => "Character Select",
            MenuButton::Quit => "Quit Game",
            MenuButton::Restart => "Restart",
            MenuButton::Resurrect => "Resurrect",
        }
    }

    /// Buttons without dedicated GRF art render text-styled like the Sound button.
    fn force_fallback(self) -> bool {
        matches!(self, MenuButton::Sound)
    }
}

impl SystemMenu {
    pub fn open_dead(&mut self) {
        self.open = true;
        self.dead = true;
    }

    pub fn close_dead(&mut self) {
        self.dead = false;
        self.open = false;
    }

    fn buttons(&self) -> Vec<MenuButton> {
        if self.dead {
            let mut buttons = vec![MenuButton::Restart];
            if self.can_resurrect {
                buttons.push(MenuButton::Resurrect);
            }
            buttons.push(MenuButton::CharSelect);
            buttons.push(MenuButton::Quit);
            buttons
        } else if self.has_grf_textures {
            vec![
                MenuButton::CharSelect,
                MenuButton::Sound,
                MenuButton::Quit,
                MenuButton::Resume,
            ]
        } else {
            vec![
                MenuButton::Resume,
                MenuButton::Sound,
                MenuButton::CharSelect,
                MenuButton::Quit,
            ]
        }
    }

    fn on_button_click(&mut self, button: MenuButton, events: &mut Vec<GameEvent>) {
        match button {
            MenuButton::Resume => self.open = false,
            MenuButton::Sound => {
                events.push(GameEvent::ToggleSoundOptions);
                self.open = false;
            }
            MenuButton::CharSelect => self.pending_confirm = PendingConfirm::CharacterSelect,
            MenuButton::Quit => self.pending_confirm = PendingConfirm::QuitGame,
            MenuButton::Restart => events.push(GameEvent::ReturnToSavePoint),
            MenuButton::Resurrect => events.push(GameEvent::RequestStandingResurrection),
        }
    }

    fn build_grf(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let buttons = self.buttons();
        let n = buttons.len() as f32;
        let (btn_w, btn_h) = self.btn_size;
        let (titlebar_w, titlebar_h) = self.win_size;
        let grf_btn_spacing = 3.0;
        let body_padding_top = 6.0;
        let body_padding_bottom = 6.0;
        let menu_w = btn_w + 60.0;
        let body_h =
            body_padding_top + n * btn_h + (n - 1.0) * grf_btn_spacing + body_padding_bottom;
        let menu_h = titlebar_h + body_h;

        let mx = ((ui.ctx.screen_width - menu_w) / 2.0).floor();
        let my = ((ui.ctx.screen_height - menu_h) / 2.0).floor() + 80.0;

        let titlebar_x = mx + (menu_w - titlebar_w) / 2.0;
        let (v, i) =
            draw::quad_vertices(titlebar_x, my, titlebar_w, titlebar_h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(WIN_TEXTURE.to_string()),
        });

        let body_y = my + titlebar_h;
        let (v, i) = draw::quad_vertices(mx, body_y, menu_w, body_h, [1.0, 1.0, 1.0, 1.0]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });

        let btn_x = mx + (menu_w - btn_w) / 2.0;
        let btn_y = |idx: usize| body_y + body_padding_top + idx as f32 * (btn_h + grf_btn_spacing);

        for (idx, &button) in buttons.iter().enumerate() {
            let rect = Rect::new(btn_x, btn_y(idx), btn_w, btn_h);
            let prev_grf = ui.has_grf_textures;
            if button.force_fallback() {
                ui.has_grf_textures = false;
            }
            let clicked = ui
                .button(button.id(), rect, button.textures(), button.label())
                .clicked();
            ui.has_grf_textures = prev_grf;
            if clicked {
                self.on_button_click(button, events);
            }
        }
    }

    fn build_fallback(&mut self, ui: &mut UiFrame, events: &mut Vec<GameEvent>) {
        let buttons = self.buttons();
        let n = buttons.len() as f32;
        let menu_h = PADDING_TOP + n * FALLBACK_BTN_H + (n - 1.0) * BTN_SPACING + PADDING_BOTTOM;
        let mx = ((ui.ctx.screen_width - MENU_W) / 2.0).floor();
        let my = ((ui.ctx.screen_height - menu_h) / 2.0).floor();

        let (v, i) = draw::quad_vertices(mx, my, MENU_W, menu_h, [0.2, 0.2, 0.28, 0.95]);
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
        let border_color = [0.5, 0.5, 0.6, 1.0];
        for (bx, by, bw, bh) in [
            (mx, my, MENU_W, 1.0),
            (mx, my + menu_h - 1.0, MENU_W, 1.0),
            (mx, my, 1.0, menu_h),
            (mx + MENU_W - 1.0, my, 1.0, menu_h),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
        }

        let btn_x = mx + (MENU_W - FALLBACK_BTN_W) / 2.0;
        let btn_y = |idx: usize| my + PADDING_TOP + idx as f32 * (FALLBACK_BTN_H + BTN_SPACING);

        for (idx, &button) in buttons.iter().enumerate() {
            let rect = Rect::new(btn_x, btn_y(idx), FALLBACK_BTN_W, FALLBACK_BTN_H);
            if ui
                .button(button.id(), rect, &DUMMY_BTN, button.label())
                .clicked()
            {
                self.on_button_click(button, events);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InGameWindow;
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
    fn escape_toggles_menu() {
        let mut menu = SystemMenu::new();
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();
        assert!(!menu.open);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.allow_escape_toggle = true;
        menu.build(&mut ui, &mut character, &data);
        assert!(menu.open);

        let mut ui = make_frame(&ctx, &mut state);
        menu.allow_escape_toggle = true;
        menu.build(&mut ui, &mut character, &data);
        assert!(!menu.open);
    }

    #[test]
    fn escape_blocked_when_not_allowed() {
        let mut menu = SystemMenu::new();
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.allow_escape_toggle = false;
        menu.build(&mut ui, &mut character, &data);
        assert!(!menu.open);
    }

    #[test]
    fn resume_closes_menu() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        let btn_x = ((800.0 - MENU_W) / 2.0).floor() + (MENU_W - FALLBACK_BTN_W) / 2.0;
        let btn_y = ((600.0 - MENU_H) / 2.0).floor() + PADDING_TOP;
        ctx.mouse_x = btn_x + FALLBACK_BTN_W / 2.0;
        ctx.mouse_y = btn_y + FALLBACK_BTN_H / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.allow_escape_toggle = true;
        menu.build(&mut ui, &mut character, &data);
        assert!(!menu.open);
    }

    #[test]
    fn charselect_opens_confirm_then_emits_event() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        let btn_x = ((800.0 - MENU_W) / 2.0).floor() + (MENU_W - FALLBACK_BTN_W) / 2.0;
        let btn_y =
            ((600.0 - MENU_H) / 2.0).floor() + PADDING_TOP + 2.0 * (FALLBACK_BTN_H + BTN_SPACING);
        ctx.mouse_x = btn_x + FALLBACK_BTN_W / 2.0;
        ctx.mouse_y = btn_y + FALLBACK_BTN_H / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.allow_escape_toggle = true;
        let events = menu.build(&mut ui, &mut character, &data);
        assert!(events.is_empty());
        assert_eq!(menu.pending_confirm, PendingConfirm::CharacterSelect);

        let mut ctx = UiContext::new(800.0, 600.0);
        let dialog_w: f32 = 220.0;
        let dialog_h: f32 = 40.0;
        let dx = ((800.0 - dialog_w) / 2.0).floor();
        let dy = ((600.0 - dialog_h) / 2.0).floor();
        let btn_w = 42.0;
        let btn_h = 20.0;
        let btn_x = dx + dialog_w - 5.0 - btn_w * 2.0 - 3.0;
        let btn_y = dy + dialog_h - 4.0 - btn_h;
        ctx.mouse_x = btn_x + btn_w / 2.0;
        ctx.mouse_y = btn_y + btn_h / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.allow_escape_toggle = true;
        let events = menu.build(&mut ui, &mut character, &data);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::BackToCharacterSelect))
        );
        assert!(!menu.open);
    }

    #[test]
    fn quit_opens_confirm_then_emits_event() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        let btn_x = ((800.0 - MENU_W) / 2.0).floor() + (MENU_W - FALLBACK_BTN_W) / 2.0;
        let btn_y =
            ((600.0 - MENU_H) / 2.0).floor() + PADDING_TOP + 3.0 * (FALLBACK_BTN_H + BTN_SPACING);
        ctx.mouse_x = btn_x + FALLBACK_BTN_W / 2.0;
        ctx.mouse_y = btn_y + FALLBACK_BTN_H / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.allow_escape_toggle = true;
        let events = menu.build(&mut ui, &mut character, &data);
        assert!(events.is_empty());
        assert_eq!(menu.pending_confirm, PendingConfirm::QuitGame);

        let mut ctx = UiContext::new(800.0, 600.0);
        let dialog_w: f32 = 220.0;
        let dialog_h: f32 = 40.0;
        let dx = ((800.0 - dialog_w) / 2.0).floor();
        let dy = ((600.0 - dialog_h) / 2.0).floor();
        let btn_w = 42.0;
        let btn_h = 20.0;
        let btn_x = dx + dialog_w - 5.0 - btn_w * 2.0 - 3.0;
        let btn_y = dy + dialog_h - 4.0 - btn_h;
        ctx.mouse_x = btn_x + btn_w / 2.0;
        ctx.mouse_y = btn_y + btn_h / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.allow_escape_toggle = true;
        let events = menu.build(&mut ui, &mut character, &data);
        assert!(events.iter().any(|e| matches!(e, GameEvent::QuitGame)));
        assert!(!menu.open);
    }

    #[test]
    fn confirm_cancel_returns_to_menu() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        menu.pending_confirm = PendingConfirm::CharacterSelect;
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        let dialog_w: f32 = 220.0;
        let dialog_h: f32 = 40.0;
        let dx = ((800.0 - dialog_w) / 2.0).floor();
        let dy = ((600.0 - dialog_h) / 2.0).floor();
        let btn_w = 42.0;
        let btn_h = 20.0;
        let cancel_btn_x = dx + dialog_w - 5.0 - btn_w;
        let cancel_btn_y = dy + dialog_h - 4.0 - btn_h;
        ctx.mouse_x = cancel_btn_x + btn_w / 2.0;
        ctx.mouse_y = cancel_btn_y + btn_h / 2.0;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.allow_escape_toggle = true;
        let events = menu.build(&mut ui, &mut character, &data);
        assert!(events.is_empty());
        assert_eq!(menu.pending_confirm, PendingConfirm::None);
        assert!(menu.open);
    }

    fn dead_button_center(screen_h: f32, button_count: usize, idx: usize) -> (f32, f32) {
        let n = button_count as f32;
        let menu_h = PADDING_TOP + n * FALLBACK_BTN_H + (n - 1.0) * BTN_SPACING + PADDING_BOTTOM;
        let btn_x = ((800.0 - MENU_W) / 2.0).floor() + (MENU_W - FALLBACK_BTN_W) / 2.0;
        let my = ((screen_h - menu_h) / 2.0).floor();
        let btn_y = my + PADDING_TOP + idx as f32 * (FALLBACK_BTN_H + BTN_SPACING);
        (btn_x + FALLBACK_BTN_W / 2.0, btn_y + FALLBACK_BTN_H / 2.0)
    }

    #[test]
    fn dead_restart_emits_return_to_savepoint() {
        let mut menu = SystemMenu::new();
        menu.open_dead();
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let (mx, my) = dead_button_center(600.0, 3, 0);
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = mx;
        ctx.mouse_y = my;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = menu.build(&mut ui, &mut character, &data);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::ReturnToSavePoint))
        );
    }

    #[test]
    fn resurrect_button_present_only_with_token() {
        let mut menu = SystemMenu::new();
        menu.open_dead();
        menu.can_resurrect = true;
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let (mx, my) = dead_button_center(600.0, 4, 1);
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = mx;
        ctx.mouse_y = my;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &mut state);
        let events = menu.build(&mut ui, &mut character, &data);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::RequestStandingResurrection))
        );
    }

    #[test]
    fn escape_cannot_close_death_menu() {
        let mut menu = SystemMenu::new();
        menu.open_dead();
        menu.allow_escape_toggle = true;
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        menu.build(&mut ui, &mut character, &data);
        assert!(menu.open);
        assert!(menu.dead);
    }
}
