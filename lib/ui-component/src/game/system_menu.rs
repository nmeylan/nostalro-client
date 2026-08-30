use std::cell::Cell;
use std::rc::Rc;

use super::confirm_dialog::{ConfirmDialog, ConfirmResult};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId, WindowOrder};
use ragnarok_ui::rect::Rect;

const BG_ID: WidgetId = WidgetId(510);
const WINDOW_ID: WidgetId = WidgetId(511);
const RESUME_ID: WidgetId = WidgetId(500);
const OPTION_ID: WidgetId = WidgetId(501);
const CHARSELECT_ID: WidgetId = WidgetId(502);
const QUIT_ID: WidgetId = WidgetId(503);
const RESTART_ID: WidgetId = WidgetId(504);
const RESURRECT_ID: WidgetId = WidgetId(505);
const GRAPHICS_ID: WidgetId = WidgetId(506);
const SHORTCUT_ID: WidgetId = WidgetId(507);
const HOTKEY_ID: WidgetId = WidgetId(508);

const MENU_W: f32 = 140.0;
const FALLBACK_BTN_W: f32 = 120.0;
const FALLBACK_BTN_H: f32 = 24.0;
const BTN_SPACING: f32 = 4.0;
const PADDING_TOP: f32 = 12.0;
const PADDING_BOTTOM: f32 = 12.0;
const MENU_H: f32 = PADDING_TOP + 4.0 * FALLBACK_BTN_H + 3.0 * BTN_SPACING + PADDING_BOTTOM;

const WIN_TEXTURE: &str = ragnarok_resources::ui::basic::TITLEBAR_FIX;

const RESUME_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::ESC_02A,
    hover: ragnarok_resources::ui::ESC_02B,
    pressed: ragnarok_resources::ui::ESC_02C,
};
const CHARSELECT_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::ESC_01A,
    hover: ragnarok_resources::ui::ESC_01B,
    pressed: ragnarok_resources::ui::ESC_01C,
};
const QUIT_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::ESC_03A,
    hover: ragnarok_resources::ui::ESC_03B,
    pressed: ragnarok_resources::ui::ESC_03C,
};
const RESTART_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::ESC_04A,
    hover: ragnarok_resources::ui::ESC_04B,
    pressed: ragnarok_resources::ui::ESC_04C,
};
const RESURRECT_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::ESC_05A,
    hover: ragnarok_resources::ui::ESC_05B,
    pressed: ragnarok_resources::ui::ESC_05C,
};
const GRAPHICS_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::ESC_06A,
    hover: ragnarok_resources::ui::ESC_06B,
    pressed: ragnarok_resources::ui::ESC_06C,
};
const SOUND_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::ESC_07A,
    hover: ragnarok_resources::ui::ESC_07B,
    pressed: ragnarok_resources::ui::ESC_07C,
};
const SHORTCUT_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::ESC_08A,
    hover: ragnarok_resources::ui::ESC_08B,
    pressed: ragnarok_resources::ui::ESC_08C,
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
            GRAPHICS_BTN.normal,
            GRAPHICS_BTN.hover,
            GRAPHICS_BTN.pressed,
            SOUND_BTN.normal,
            SOUND_BTN.hover,
            SOUND_BTN.pressed,
            SHORTCUT_BTN.normal,
            SHORTCUT_BTN.hover,
            SHORTCUT_BTN.pressed,
        ];
        paths.extend(ConfirmDialog::grf_texture_paths());
        paths
    }
}

impl InGameWindow for SystemMenu {
    fn owns_keyboard(&self, _ctx: &BuildCtx) -> bool {
        self.open
    }

    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.open && !self.dead
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        if self.pending_confirm != PendingConfirm::None {
            self.pending_confirm = PendingConfirm::None;
        } else {
            self.open = false;
        }
        Vec::new()
    }

    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        let mut events = Vec::new();

        if !self.open {
            return events;
        }

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
    Graphics,
    Sound,
    Shortcut,
    Hotkeys,
    CharSelect,
    Quit,
    Restart,
    Resurrect,
}

impl MenuButton {
    fn id(self) -> WidgetId {
        match self {
            MenuButton::Resume => RESUME_ID,
            MenuButton::Graphics => GRAPHICS_ID,
            MenuButton::Sound => OPTION_ID,
            MenuButton::Shortcut => SHORTCUT_ID,
            MenuButton::Hotkeys => HOTKEY_ID,
            MenuButton::CharSelect => CHARSELECT_ID,
            MenuButton::Quit => QUIT_ID,
            MenuButton::Restart => RESTART_ID,
            MenuButton::Resurrect => RESURRECT_ID,
        }
    }

    fn textures(self) -> &'static ButtonTextures {
        match self {
            MenuButton::Resume => &RESUME_BTN,
            MenuButton::Graphics => &GRAPHICS_BTN,
            MenuButton::Sound => &SOUND_BTN,
            MenuButton::Shortcut => &SHORTCUT_BTN,
            MenuButton::Hotkeys => &DUMMY_BTN,
            MenuButton::CharSelect => &CHARSELECT_BTN,
            MenuButton::Quit => &QUIT_BTN,
            MenuButton::Restart => &RESTART_BTN,
            MenuButton::Resurrect => &RESURRECT_BTN,
        }
    }

    fn label(self) -> &'static str {
        match self {
            MenuButton::Resume => "Resume",
            MenuButton::Graphics => "Graphics",
            MenuButton::Sound => "Sound",
            MenuButton::Shortcut => "Shortcut",
            MenuButton::Hotkeys => "Hotkeys",
            MenuButton::CharSelect => "Character Select",
            MenuButton::Quit => "Quit Game",
            MenuButton::Restart => "Restart",
            MenuButton::Resurrect => "Resurrect",
        }
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
                MenuButton::Graphics,
                MenuButton::Sound,
                MenuButton::Shortcut,
                MenuButton::Hotkeys,
                MenuButton::Quit,
                MenuButton::Resume,
            ]
        } else {
            vec![
                MenuButton::Resume,
                MenuButton::Graphics,
                MenuButton::Sound,
                MenuButton::Shortcut,
                MenuButton::Hotkeys,
                MenuButton::CharSelect,
                MenuButton::Quit,
            ]
        }
    }

    fn on_button_click(&mut self, button: MenuButton, events: &mut Vec<GameEvent>) {
        match button {
            MenuButton::Resume => self.open = false,
            MenuButton::Graphics => {
                events.push(GameEvent::ToggleGraphicOptions);
                self.open = false;
            }
            MenuButton::Sound => {
                events.push(GameEvent::ToggleSoundOptions);
                self.open = false;
            }
            MenuButton::Shortcut => {
                events.push(GameEvent::ToggleShortcutList);
                self.open = false;
            }
            MenuButton::Hotkeys => {
                events.push(GameEvent::ToggleHotkeyConfig);
                self.open = false;
            }
            MenuButton::CharSelect => self.pending_confirm = PendingConfirm::CharacterSelect,
            MenuButton::Quit => self.pending_confirm = PendingConfirm::QuitGame,
            MenuButton::Restart => events.push(GameEvent::ReturnToSavePoint),
            MenuButton::Resurrect => events.push(GameEvent::RequestStandingResurrection),
        }
    }

    fn claim_pointer(&self, ui: &mut UiFrame, rect: Rect) {
        ui.ensure_in_z_order_with(WINDOW_ID, WindowOrder::Foreground);
        ui.enter_window(WINDOW_ID, rect);
        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(BG_ID, screen);
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

        self.claim_pointer(ui, Rect::new(mx, my, menu_w, menu_h));

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
            let clicked = if button == MenuButton::Hotkeys {
                ui.text_button(button.id(), rect, button.label()).clicked()
            } else {
                ui.button(button.id(), rect, button.textures(), button.label())
                    .clicked()
            };
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

        self.claim_pointer(ui, Rect::new(mx, my, MENU_W, menu_h));

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
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;

    use crate::game::chat_window::CHAT_WINDOW_ID;
    use crate::game::minimap_window::MINIMAP_WINDOW_ID;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    #[test]
    fn escape_closes_an_open_menu() {
        let mut menu = SystemMenu::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut ctx = crate::BuildCtx::test(&mut character, &data);

        assert!(!menu.wants_escape(&ctx));
        menu.open = true;
        assert!(menu.wants_escape(&ctx));
        menu.on_escape(&mut ctx);
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
        let (mx, my) = button_center(600.0, 7, 0);
        ctx.mouse_x = mx;
        ctx.mouse_y = my;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(!menu.open);
    }

    #[test]
    fn graphics_button_emits_toggle_and_closes() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        let (mx, my) = button_center(600.0, 7, 1);
        ctx.mouse_x = mx;
        ctx.mouse_y = my;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::ToggleGraphicOptions))
        );
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
        let (mx, my) = button_center(600.0, 7, 5);
        ctx.mouse_x = mx;
        ctx.mouse_y = my;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
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
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
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
        let (mx, my) = button_center(600.0, 7, 6);
        ctx.mouse_x = mx;
        ctx.mouse_y = my;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
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
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
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
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(events.is_empty());
        assert_eq!(menu.pending_confirm, PendingConfirm::None);
        assert!(menu.open);
    }

    fn button_center(screen_h: f32, button_count: usize, idx: usize) -> (f32, f32) {
        let n = button_count as f32;
        let menu_h = PADDING_TOP + n * FALLBACK_BTN_H + (n - 1.0) * BTN_SPACING + PADDING_BOTTOM;
        let btn_x = ((800.0 - MENU_W) / 2.0).floor() + (MENU_W - FALLBACK_BTN_W) / 2.0;
        let my = ((screen_h - menu_h) / 2.0).floor();
        let btn_y = my + PADDING_TOP + idx as f32 * (FALLBACK_BTN_H + BTN_SPACING);
        (btn_x + FALLBACK_BTN_W / 2.0, btn_y + FALLBACK_BTN_H / 2.0)
    }

    #[test]
    fn button_wins_over_a_window_drawn_behind_the_menu() {
        let mut menu = SystemMenu::new();
        menu.open = true;
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let (mx, my) = button_center(600.0, 7, 0);

        let mut frame = |clicked: bool| {
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.mouse_x = mx;
            ctx.mouse_y = my;
            ctx.mouse_clicked = clicked;
            let mut ui = test_frame(&mut ctx, &mut state);
            let z = ui.get_z_order();
            ui.compute_hovered_window(&z);
            ui.window_fixed(CHAT_WINDOW_ID, 800.0, 600.0, 0.0, 0.0);
            ui.window_fixed(MINIMAP_WINDOW_ID, 100.0, 100.0, 700.0, 0.0);
            menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        };

        frame(false);
        frame(true);
        assert!(!menu.open);
    }

    #[test]
    fn dead_restart_emits_return_to_savepoint() {
        let mut menu = SystemMenu::new();
        menu.open_dead();
        let mut state = StateCache::new();
        let mut character = Character::new();
        let data = DataTable::new();

        let (mx, my) = button_center(600.0, 3, 0);
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = mx;
        ctx.mouse_y = my;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
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

        let (mx, my) = button_center(600.0, 4, 1);
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = mx;
        ctx.mouse_y = my;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        let events = menu.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
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
        let mut character = Character::new();
        let data = DataTable::new();
        let ctx = crate::BuildCtx::test(&mut character, &data);

        assert!(!menu.wants_escape(&ctx));
        assert!(menu.open);
        assert!(menu.dead);
    }
}
