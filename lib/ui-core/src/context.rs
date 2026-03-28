use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};

pub struct UiContext {
    pub screen_width: f32,
    pub screen_height: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_clicked: bool,
    pub mouse_down: bool,
    pub typed_chars: Vec<char>,
    pub key_backspace: bool,
    pub key_enter: bool,
    pub key_tab: bool,
    pub key_escape: bool,
    pub key_left: bool,
    pub key_right: bool,
    pub key_delete: bool,
    pub key_up: bool,
    pub key_down: bool,
    pub key_f10: bool,
    pub scroll_delta: f32,
}

impl UiContext {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_clicked: false,
            mouse_down: false,
            typed_chars: Vec::new(),
            key_backspace: false,
            key_enter: false,
            key_tab: false,
            key_escape: false,
            key_left: false,
            key_right: false,
            key_delete: false,
            key_up: false,
            key_down: false,
            key_f10: false,
            scroll_delta: 0.0,
        }
    }

    pub fn begin_frame(&mut self) {
        self.mouse_clicked = false;
        self.typed_chars.clear();
        self.key_backspace = false;
        self.key_enter = false;
        self.key_tab = false;
        self.key_escape = false;
        self.key_left = false;
        self.key_right = false;
        self.key_delete = false;
        self.key_up = false;
        self.key_down = false;
        self.key_f10 = false;
        self.scroll_delta = 0.0;
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as f32;
                self.mouse_y = position.y as f32;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            self.mouse_clicked = true;
                            self.mouse_down = true;
                        }
                        ElementState::Released => {
                            self.mouse_down = false;
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match &event.logical_key {
                        Key::Named(NamedKey::Backspace) => self.key_backspace = true,
                        Key::Named(NamedKey::Enter) => self.key_enter = true,
                        Key::Named(NamedKey::Tab) => self.key_tab = true,
                        Key::Named(NamedKey::Escape) => self.key_escape = true,
                        Key::Named(NamedKey::ArrowLeft) => self.key_left = true,
                        Key::Named(NamedKey::ArrowRight) => self.key_right = true,
                        Key::Named(NamedKey::Delete) => self.key_delete = true,
                        Key::Named(NamedKey::ArrowUp) => self.key_up = true,
                        Key::Named(NamedKey::ArrowDown) => self.key_down = true,
                        Key::Named(NamedKey::F10) => self.key_f10 = true,
                        Key::Character(_) => {
                            if let Some(text) = &event.text {
                                for ch in text.chars() {
                                    if !ch.is_control() {
                                        self.typed_chars.push(ch);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.scroll_delta += match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
                };
            }
            WindowEvent::Resized(size) => {
                self.screen_width = size.width as f32;
                self.screen_height = size.height as f32;
            }
            _ => {}
        }
    }
}
