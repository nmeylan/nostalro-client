use crate::draw::{self, DrawCall, TextureRef};
use crate::frame::{ButtonTextures, UiFrame, WidgetId};
use crate::rect::Rect;
use crate::text_input::TextInput;
use ragnarok_game::event::GameEvent;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoginFocus {
    Username,
    Password,
}

pub struct LoginWindow {
    pub username: TextInput,
    pub password: TextInput,
    pub error_message: Option<String>,
    pub focus: LoginFocus,
    pub has_grf_textures: bool,
    win_size: (f32, f32),
    input_size: (f32, f32),
    btn_size: (f32, f32),
}

// Fallback layout constants (used when no GRF textures)
const FALLBACK_WIN_W: f32 = 280.0;
const FALLBACK_WIN_H: f32 = 120.0;
const FALLBACK_FIELD_W: f32 = 150.0;
const FALLBACK_FIELD_H: f32 = 20.0;
const FALLBACK_BTN_W: f32 = 80.0;
const FALLBACK_BTN_H: f32 = 22.0;

// Position offsets within the window (skin-specific, match win_login.bmp design)
const LABEL_X: f32 = 10.0;
const FIELD_X: f32 = 85.0;
const USERNAME_Y: f32 = 15.0;
const PASSWORD_Y: f32 = 42.0;
const BTN_Y: f32 = 75.0;
const CONNECT_BTN_X: f32 = 45.0;
const EXIT_BTN_X: f32 = 155.0;

const INPUT_TEXTURE: &str = "data/texture/유저인터페이스/login_interface/name-edit.bmp";
const WIN_TEXTURE: &str = "data/texture/유저인터페이스/login_interface/win_login.bmp";

const USERNAME_ID: WidgetId = WidgetId(0);
const PASSWORD_ID: WidgetId = WidgetId(1);
const CONNECT_ID: WidgetId = WidgetId(2);
const EXIT_ID: WidgetId = WidgetId(3);

const CONNECT_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/login_interface/btn_connect.bmp",
    hover: "data/texture/유저인터페이스/login_interface/btn_connect_a.bmp",
    pressed: "data/texture/유저인터페이스/login_interface/btn_connect_b.bmp",
};

const EXIT_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/login_interface/btn_exit.bmp",
    hover: "data/texture/유저인터페이스/login_interface/btn_exit_a.bmp",
    pressed: "data/texture/유저인터페이스/login_interface/btn_exit_b.bmp",
};

impl LoginWindow {
    pub fn new() -> Self {
        Self {
            username: TextInput::new(23, false),
            password: TextInput::new(23, true),
            error_message: None,
            focus: LoginFocus::Username,
            has_grf_textures: false,
            win_size: (FALLBACK_WIN_W, FALLBACK_WIN_H),
            input_size: (FALLBACK_FIELD_W, FALLBACK_FIELD_H),
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    /// Set actual BMP dimensions from texture cache. Call after preloading textures.
    /// `size_fn` maps a texture path to its (width, height) in pixels.
    pub fn set_texture_sizes(&mut self, size_fn: impl Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(WIN_TEXTURE) {
            self.win_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(INPUT_TEXTURE) {
            self.input_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(CONNECT_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let (win_w, win_h) = self.win_size;
        let (field_w, field_h) = self.input_size;
        let (btn_w, btn_h) = self.btn_size;
        let win = Rect::centered_in(ui.ctx.screen_width, ui.ctx.screen_height, win_w, win_h);

        // Tab cycles focus
        if ui.ctx.key_tab {
            self.focus = match self.focus {
                LoginFocus::Username => LoginFocus::Password,
                LoginFocus::Password => LoginFocus::Username,
            };
            let focus_id = match self.focus {
                LoginFocus::Username => USERNAME_ID,
                LoginFocus::Password => PASSWORD_ID,
            };
            ui.set_focus(focus_id);
        }

        // Window background
        if self.has_grf_textures {
            let (verts, indices) = draw::quad_vertices(win.x, win.y, win.w, win.h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: verts.to_vec(),
                indices: indices.to_vec(),
                texture: TextureRef::Named(WIN_TEXTURE.to_string()),
            });
        } else {
            let (verts, indices) = draw::quad_vertices(win.x, win.y, win.w, win.h, [0.08, 0.08, 0.12, 0.95]);
            ui.draw_calls.push(DrawCall {
                vertices: verts.to_vec(),
                indices: indices.to_vec(),
                texture: TextureRef::White,
            });

            let border_color = [0.4, 0.4, 0.5, 1.0];
            let b = 1.0;
            for (bx, by, bw, bh) in [
                (win.x, win.y, win.w, b),
                (win.x, win.y + win.h - b, win.w, b),
                (win.x, win.y, b, win.h),
                (win.x + win.w - b, win.y, b, win.h),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
        }

        // Labels
        let label_color = [0.9, 0.9, 0.9, 1.0];
        let label_y_offset = (field_h - ui.atlas.line_height) / 2.0;
        ui.text(win.x + LABEL_X, win.y + USERNAME_Y + label_y_offset, "Username", label_color);
        ui.text(win.x + LABEL_X, win.y + PASSWORD_Y + label_y_offset, "Password", label_color);

        // Text inputs
        let input_tex = if self.has_grf_textures { Some(INPUT_TEXTURE) } else { None };
        let username_rect = Rect::new(win.x + FIELD_X, win.y + USERNAME_Y, field_w, field_h);
        let password_rect = Rect::new(win.x + FIELD_X, win.y + PASSWORD_Y, field_w, field_h);
        ui.text_input(USERNAME_ID, username_rect, &mut self.username, input_tex);
        ui.text_input(PASSWORD_ID, password_rect, &mut self.password, input_tex);

        // Sync focus back from UiFrame (click-to-focus may have changed it)
        match ui.focused() {
            Some(id) if id == USERNAME_ID => self.focus = LoginFocus::Username,
            Some(id) if id == PASSWORD_ID => self.focus = LoginFocus::Password,
            _ => {}
        }

        // Buttons
        let connect_rect = Rect::new(win.x + CONNECT_BTN_X, win.y + BTN_Y, btn_w, btn_h);
        let exit_rect = Rect::new(win.x + EXIT_BTN_X, win.y + BTN_Y, btn_w, btn_h);
        let connect = ui.button(CONNECT_ID, connect_rect, &CONNECT_BTN, "Connect");
        let exit = ui.button(EXIT_ID, exit_rect, &EXIT_BTN, "Exit");

        // Enter or Connect button submits login
        let submit = ui.ctx.key_enter || connect.clicked();
        if submit && !self.username.text.is_empty() && !self.password.text.is_empty() {
            self.error_message = None;
            events.push(GameEvent::RequestLogin {
                username: self.username.text.clone(),
                password: self.password.text.clone(),
            });
        }

        if exit.clicked() {
            events.push(GameEvent::Disconnected("User exit".to_string()));
        }

        // Error message
        if let Some(msg) = &self.error_message {
            let error_y = win.y + win_h + 5.0;
            let error_w = ui.atlas.measure_text(msg);
            let error_x = win.x + (win_w - error_w) / 2.0;
            ui.text(error_x, error_y, msg, [1.0, 0.3, 0.3, 1.0]);
        }

        events
    }

    pub fn set_error(&mut self, msg: &str) {
        self.error_message = Some(msg.to_string());
    }

    pub fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            CONNECT_BTN.normal,
            CONNECT_BTN.hover,
            CONNECT_BTN.pressed,
            EXIT_BTN.normal,
            EXIT_BTN.hover,
            EXIT_BTN.pressed,
            INPUT_TEXTURE,
            WIN_TEXTURE,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::UiContext;
    use ragnarok_renderer::font_atlas::FontAtlas;

    fn make_ctx() -> UiContext {
        UiContext::new(800.0, 600.0)
    }

    fn make_frame(ctx: &UiContext) -> UiFrame<'_> {
        let atlas = FontAtlas::from_embedded(14.0);
        let atlas = Box::leak(Box::new(atlas));
        UiFrame::new(ctx, atlas, 0.0, false, Some(USERNAME_ID))
    }

    #[test]
    fn tab_cycles_focus() {
        let mut login = LoginWindow::new();
        assert_eq!(login.focus, LoginFocus::Username);

        let mut ctx = make_ctx();
        ctx.key_tab = true;
        let mut ui = make_frame(&ctx);
        login.build(&mut ui);
        assert_eq!(login.focus, LoginFocus::Password);

        let mut ui = make_frame(&ctx);
        login.build(&mut ui);
        assert_eq!(login.focus, LoginFocus::Username);
    }

    #[test]
    fn enter_with_credentials_emits_request_login() {
        let mut login = LoginWindow::new();
        login.username.text = "admin".to_string();
        login.password.text = "pass123".to_string();

        let mut ctx = make_ctx();
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx);
        let events = login.build(&mut ui);
        assert_eq!(events.len(), 1);
        match &events[0] {
            GameEvent::RequestLogin { username, password } => {
                assert_eq!(username, "admin");
                assert_eq!(password, "pass123");
            }
            _ => panic!("expected RequestLogin"),
        }
    }

    #[test]
    fn enter_with_empty_fields_does_nothing() {
        let mut login = LoginWindow::new();

        let mut ctx = make_ctx();
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx);
        let events = login.build(&mut ui);
        assert!(events.is_empty());
    }

    #[test]
    fn set_error_stores_message() {
        let mut login = LoginWindow::new();
        assert!(login.error_message.is_none());
        login.set_error("Invalid credentials");
        assert_eq!(login.error_message.as_deref(), Some("Invalid credentials"));
    }
}
