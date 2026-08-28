use crate::helper::window_chrome::{
    SYS_BASE_OFF_TEX, SYS_BASE_ON_TEX, TITLEBAR_TEX, draw_container, draw_footer, draw_sys_button,
    draw_titlebar, text_color,
};
use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const STORAGE_PASSWORD_WINDOW_ID: WidgetId = WidgetId(5000);
const CLOSE_BTN_ID: WidgetId = WidgetId(5001);
const PASSWORD_INPUT_ID: WidgetId = WidgetId(5002);
const NEW_PASSWORD_INPUT_ID: WidgetId = WidgetId(5003);
const CONFIRM_INPUT_ID: WidgetId = WidgetId(5004);
const OK_ID: WidgetId = WidgetId(5005);
const CANCEL_ID: WidgetId = WidgetId(5006);

const CLOSE_OFF_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_OFF;
const CLOSE_ON_TEX: &str = ragnarok_resources::ui::basic::SYS_CLOSE_ON;
const OK_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_OK,
    hover: ragnarok_resources::ui::BTN_OK_A,
    pressed: ragnarok_resources::ui::BTN_OK_B,
};
const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_CANCEL,
    hover: ragnarok_resources::ui::BTN_CANCEL_A,
    pressed: ragnarok_resources::ui::BTN_CANCEL_B,
};

const WIN_W: f32 = 240.0;
const TITLE_H: f32 = 17.0;
const FOOTER_H: f32 = 30.0;
const PAD: f32 = 8.0;
const ROW_H: f32 = 22.0;
const LABEL_W: f32 = 96.0;
const FIELD_H: f32 = 16.0;
const MSG_H: f32 = 18.0;
const CLOSE_BTN_SIZE: f32 = 11.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

const MIN_LEN: usize = 4;
const MAX_LEN: usize = 8;

const MSG_TOO_SHORT: &str = "Use 4 to 8 characters.";
const MSG_MISMATCH: &str = "The passwords do not match.";
const MSG_EMPTY: &str = "Enter your password.";

const ERROR_COLOR: [f32; 4] = [1.0, 0.45, 0.45, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoragePasswordMode {
    Enter,
    SetNew,
}

pub struct StoragePasswordWindow {
    pub has_grf_textures: bool,
    open: bool,
    mode: StoragePasswordMode,
    password: TextInput,
    new_password: TextInput,
    confirm: TextInput,
    message: Option<String>,
    focus_next: bool,
    btn_size: (f32, f32),
}

impl Default for StoragePasswordWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl StoragePasswordWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            open: false,
            mode: StoragePasswordMode::Enter,
            password: TextInput::new(MAX_LEN, true),
            new_password: TextInput::new(MAX_LEN, true),
            confirm: TextInput::new(MAX_LEN, true),
            message: None,
            focus_next: false,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn mode(&self) -> StoragePasswordMode {
        self.mode
    }

    pub fn open_with(&mut self, mode: StoragePasswordMode) {
        self.mode = mode;
        self.message = None;
        self.clear_inputs();
        self.focus_next = true;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.message = None;
        self.clear_inputs();
    }

    pub fn set_message(&mut self, message: String) {
        self.message = Some(message);
    }

    fn clear_inputs(&mut self) {
        for input in [
            &mut self.password,
            &mut self.new_password,
            &mut self.confirm,
        ] {
            input.text.clear();
            input.cursor_pos = 0;
        }
    }

    fn rows(&self) -> f32 {
        match self.mode {
            StoragePasswordMode::Enter => 1.0,
            StoragePasswordMode::SetNew => 2.0,
        }
    }

    fn body_h(&self) -> f32 {
        PAD + ROW_H * self.rows() + MSG_H + PAD
    }

    fn field_ids(&self) -> &'static [WidgetId] {
        match self.mode {
            StoragePasswordMode::Enter => &[PASSWORD_INPUT_ID],
            StoragePasswordMode::SetNew => &[NEW_PASSWORD_INPUT_ID, CONFIRM_INPUT_ID],
        }
    }

    fn cycle_focus(&mut self, ui: &mut UiFrame) {
        let ids = self.field_ids();
        let current = ui
            .focused()
            .and_then(|id| ids.iter().position(|f| *f == id));
        let next = match current {
            Some(i) => ids[(i + 1) % ids.len()],
            None => ids[0],
        };
        ui.set_focus(next);
    }

    /// The submission, or the reason it was rejected.
    fn validated(&self) -> Result<GameEvent, &'static str> {
        match self.mode {
            StoragePasswordMode::Enter => {
                let password = self.password.text.clone();
                if password.is_empty() {
                    return Err(MSG_EMPTY);
                }
                if password.chars().count() < MIN_LEN {
                    return Err(MSG_TOO_SHORT);
                }
                Ok(GameEvent::RequestStoragePassword {
                    change: false,
                    password,
                    new_password: String::new(),
                })
            }
            StoragePasswordMode::SetNew => {
                let new_password = self.new_password.text.clone();
                if new_password.is_empty() {
                    return Err(MSG_EMPTY);
                }
                if new_password.chars().count() < MIN_LEN {
                    return Err(MSG_TOO_SHORT);
                }
                if new_password != self.confirm.text {
                    return Err(MSG_MISMATCH);
                }
                Ok(GameEvent::RequestStoragePassword {
                    change: true,
                    password: String::new(),
                    new_password,
                })
            }
        }
    }

    fn submit(&mut self, events: &mut Vec<GameEvent>) {
        match self.validated() {
            Ok(event) => {
                events.push(event);
                self.clear_inputs();
                self.message = None;
            }
            Err(message) => {
                self.message = Some(message.to_string());
            }
        }
    }
}

impl Window for StoragePasswordWindow {
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
    fn window_size(&self) -> (f32, f32) {
        (WIN_W, TITLE_H + self.body_h() + FOOTER_H)
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            TITLEBAR_TEX,
            SYS_BASE_OFF_TEX,
            SYS_BASE_ON_TEX,
            CLOSE_OFF_TEX,
            CLOSE_ON_TEX,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ]
    }
}

impl InGameWindow for StoragePasswordWindow {
    fn wants_escape(&self, _ctx: &BuildCtx) -> bool {
        self.is_open()
    }

    fn on_escape(&mut self, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        self.close();
        Vec::new()
    }

    fn owns_keyboard(&self, _ctx: &BuildCtx) -> bool {
        self.is_open()
    }

    fn build(&mut self, ui: &mut UiFrame, _ctx: &mut BuildCtx) -> Vec<GameEvent> {
        if !self.open {
            return Vec::new();
        }
        let mut events = Vec::new();

        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let tc = text_color(grf);
        let bg = if grf {
            TextInputBg::Gray
        } else {
            TextInputBg::Default
        };

        let body_h = self.body_h();
        let win_h = TITLE_H + body_h + FOOTER_H;
        let win = ui.window_at(
            STORAGE_PASSWORD_WINDOW_ID,
            WIN_W,
            win_h,
            TITLE_H,
            260.0,
            180.0,
        );
        let (x, y) = (win.x, win.y);
        ui.interact(STORAGE_PASSWORD_WINDOW_ID, Rect::new(x, y, WIN_W, win_h));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 16.0, y + TITLE_H - 3.0, "Storage Password", tc);

        let close_rect = Rect::new(
            x + WIN_W - CLOSE_BTN_SIZE - 3.0,
            y + (TITLE_H - CLOSE_BTN_SIZE) / 2.0,
            CLOSE_BTN_SIZE,
            CLOSE_BTN_SIZE,
        );
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        draw_sys_button(
            ui,
            close_rect,
            (CLOSE_BTN_SIZE, CLOSE_BTN_SIZE),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );
        if close_resp.clicked() {
            self.close();
            ui.has_grf_textures = prev_grf;
            return events;
        }

        let body_y = y + TITLE_H;
        draw_container(ui, x, body_y, WIN_W, body_h, grf);
        draw_footer(ui, x, y + win_h - FOOTER_H, WIN_W, FOOTER_H, grf);

        if self.focus_next {
            self.focus_next = false;
            ui.set_focus(self.field_ids()[0]);
        } else if ui.ctx.key_tab {
            self.cycle_focus(ui);
        }

        let field_x = x + PAD + LABEL_W;
        let field_w = WIN_W - PAD * 2.0 - LABEL_W;
        let mut row_y = body_y + PAD;

        match self.mode {
            StoragePasswordMode::Enter => {
                ui.text(x + PAD, row_y + 12.0, "Password :", tc);
                ui.text_input(
                    PASSWORD_INPUT_ID,
                    Rect::new(field_x, row_y, field_w, FIELD_H),
                    &mut self.password,
                    bg,
                );
                row_y += ROW_H;
            }
            StoragePasswordMode::SetNew => {
                ui.text(x + PAD, row_y + 12.0, "New Password :", tc);
                ui.text_input(
                    NEW_PASSWORD_INPUT_ID,
                    Rect::new(field_x, row_y, field_w, FIELD_H),
                    &mut self.new_password,
                    bg,
                );
                row_y += ROW_H;
                ui.text(x + PAD, row_y + 12.0, "Confirm :", tc);
                ui.text_input(
                    CONFIRM_INPUT_ID,
                    Rect::new(field_x, row_y, field_w, FIELD_H),
                    &mut self.confirm,
                    bg,
                );
                row_y += ROW_H;
            }
        }

        if let Some(message) = &self.message {
            ui.text(x + PAD, row_y + 12.0, message, ERROR_COLOR);
        }

        let (btn_w, btn_h) = self.btn_size;
        let footer_y = y + win_h - FOOTER_H;
        let btn_y = footer_y + (FOOTER_H - btn_h) / 2.0;
        let mut bx = x + WIN_W - PAD - btn_w;
        let cancel = ui
            .button(
                CANCEL_ID,
                Rect::new(bx, btn_y, btn_w, btn_h),
                &CANCEL_BTN,
                "cancel",
            )
            .clicked();
        bx -= btn_w + 4.0;
        let ok = ui
            .button(OK_ID, Rect::new(bx, btn_y, btn_w, btn_h), &OK_BTN, "OK")
            .clicked();

        if cancel {
            self.close();
        } else if ok || ui.ctx.key_enter {
            self.submit(&mut events);
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

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    fn frame(win: &mut StoragePasswordWindow, state: &mut StateCache) -> Vec<GameEvent> {
        let mut character = Character::new();
        let data = DataTable::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = test_frame(&mut ctx, state);
        win.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data))
    }

    #[test]
    fn enter_mode_submits_typed_password() {
        let mut win = StoragePasswordWindow::new();
        win.open_with(StoragePasswordMode::Enter);
        win.password.text = "1234".to_string();
        let mut state = StateCache::new();

        let events = frame(&mut win, &mut state);
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestStoragePassword {
                change: false,
                password,
                new_password,
            }] if password == "1234" && new_password.is_empty()
        ));
        assert!(win.is_open());
        assert!(win.password.text.is_empty());
    }

    #[test]
    fn set_new_mode_rejects_mismatch_then_submits_match() {
        let mut win = StoragePasswordWindow::new();
        win.open_with(StoragePasswordMode::SetNew);
        win.new_password.text = "12345".to_string();
        win.confirm.text = "54321".to_string();
        let mut state = StateCache::new();

        assert!(frame(&mut win, &mut state).is_empty());
        assert_eq!(win.message.as_deref(), Some(MSG_MISMATCH));

        win.confirm.text = "12345".to_string();
        let events = frame(&mut win, &mut state);
        assert!(matches!(
            events.as_slice(),
            [GameEvent::RequestStoragePassword {
                change: true,
                password,
                new_password,
            }] if password.is_empty() && new_password == "12345"
        ));
    }

    #[test]
    fn short_password_is_rejected() {
        let mut win = StoragePasswordWindow::new();
        win.open_with(StoragePasswordMode::Enter);
        win.password.text = "12".to_string();
        let mut state = StateCache::new();

        assert!(frame(&mut win, &mut state).is_empty());
        assert_eq!(win.message.as_deref(), Some(MSG_TOO_SHORT));
        assert_eq!(win.password.text, "12");
    }
}
