use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};

const DOUBLE_CLICK_THRESHOLD_MS: u128 = 400;
const DOUBLE_CLICK_DISTANCE: f32 = 5.0;

pub struct UiContext {
    pub screen_width: f32,
    pub screen_height: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_clicked: bool,
    pub mouse_double_clicked: bool,
    pub mouse_down: bool,
    pub mouse_right_clicked: bool,
    pub typed_chars: Vec<char>,
    last_click_time: std::time::Instant,
    last_click_pos: (f32, f32),
    pub key_backspace: bool,
    pub key_enter: bool,
    pub key_tab: bool,
    pub key_escape: bool,
    pub key_left: bool,
    pub key_right: bool,
    pub key_delete: bool,
    pub key_up: bool,
    pub key_down: bool,
    pub key_f1: bool,
    pub key_f2: bool,
    pub key_f3: bool,
    pub key_f4: bool,
    pub key_f5: bool,
    pub key_f6: bool,
    pub key_f7: bool,
    pub key_f8: bool,
    pub key_f9: bool,
    pub key_f10: bool,
    pub key_f12: bool,
    pub scroll_delta: f32,
    pub dpi_scale: f32,
}

impl UiContext {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_clicked: false,
            mouse_double_clicked: false,
            mouse_down: false,
            mouse_right_clicked: false,
            typed_chars: Vec::new(),
            last_click_time: std::time::Instant::now(),
            last_click_pos: (0.0, 0.0),
            key_backspace: false,
            key_enter: false,
            key_tab: false,
            key_escape: false,
            key_left: false,
            key_right: false,
            key_delete: false,
            key_up: false,
            key_down: false,
            key_f1: false,
            key_f2: false,
            key_f3: false,
            key_f4: false,
            key_f5: false,
            key_f6: false,
            key_f7: false,
            key_f8: false,
            key_f9: false,
            key_f10: false,
            key_f12: false,
            scroll_delta: 0.0,
            dpi_scale: 1.0,
        }
    }

    pub fn begin_frame(&mut self) {
        self.mouse_clicked = false;
        self.mouse_double_clicked = false;
        self.mouse_right_clicked = false;
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
        self.key_f1 = false;
        self.key_f2 = false;
        self.key_f3 = false;
        self.key_f4 = false;
        self.key_f5 = false;
        self.key_f6 = false;
        self.key_f7 = false;
        self.key_f8 = false;
        self.key_f9 = false;
        self.key_f10 = false;
        self.key_f12 = false;
        self.scroll_delta = 0.0;
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as f32 / self.dpi_scale;
                self.mouse_y = position.y as f32 / self.dpi_scale;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            let now = std::time::Instant::now();
                            let elapsed = now.duration_since(self.last_click_time).as_millis();
                            let dx = self.mouse_x - self.last_click_pos.0;
                            let dy = self.mouse_y - self.last_click_pos.1;
                            let dist = (dx * dx + dy * dy).sqrt();

                            self.mouse_clicked = true;
                            self.mouse_down = true;

                            if elapsed < DOUBLE_CLICK_THRESHOLD_MS && dist < DOUBLE_CLICK_DISTANCE {
                                self.mouse_double_clicked = true;
                                // Reset so a third click doesn't count as another double
                                self.last_click_time = now - std::time::Duration::from_secs(1);
                            } else {
                                self.last_click_time = now;
                            }
                            self.last_click_pos = (self.mouse_x, self.mouse_y);
                        }
                        ElementState::Released => {
                            self.mouse_down = false;
                        }
                    }
                }
                if *button == MouseButton::Right && *state == ElementState::Pressed {
                    self.mouse_right_clicked = true;
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
                        Key::Named(NamedKey::F1) => self.key_f1 = true,
                        Key::Named(NamedKey::F2) => self.key_f2 = true,
                        Key::Named(NamedKey::F3) => self.key_f3 = true,
                        Key::Named(NamedKey::F4) => self.key_f4 = true,
                        Key::Named(NamedKey::F5) => self.key_f5 = true,
                        Key::Named(NamedKey::F6) => self.key_f6 = true,
                        Key::Named(NamedKey::F7) => self.key_f7 = true,
                        Key::Named(NamedKey::F8) => self.key_f8 = true,
                        Key::Named(NamedKey::F9) => self.key_f9 = true,
                        Key::Named(NamedKey::F10) => self.key_f10 = true,
                        Key::Named(NamedKey::F12) => self.key_f12 = true,
                        Key::Named(NamedKey::Space) => self.typed_chars.push(' '),
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
                self.screen_width = size.width as f32 / self.dpi_scale;
                self.screen_height = size.height as f32 / self.dpi_scale;
            }
            _ => {}
        }
    }
}
