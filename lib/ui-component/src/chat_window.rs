use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

const INPUT_ID: WidgetId = WidgetId(200);
const MAX_MESSAGES: usize = 100;
const MAX_INPUT_LEN: usize = 100;

const CHAT_W: f32 = 350.0;
const MSG_AREA_H: f32 = 150.0;
const INPUT_H: f32 = 22.0;
const PADDING: f32 = 4.0;
const LINE_H: f32 = 16.0;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const GREEN: [f32; 4] = [0.6, 1.0, 0.6, 1.0];
const YELLOW: [f32; 4] = [1.0, 1.0, 0.4, 1.0];

pub struct ChatLine {
    pub text: String,
    pub color: [f32; 4],
}

pub struct ChatWindow {
    pub input: TextInput,
    pub messages: Vec<ChatLine>,
    pub active: bool,
}

impl ChatWindow {
    pub fn new() -> Self {
        Self {
            input: TextInput::new(MAX_INPUT_LEN, false),
            messages: Vec::new(),
            active: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn add_message(&mut self, text: String, color: [f32; 4]) {
        self.messages.push(ChatLine { text, color });
        if self.messages.len() > MAX_MESSAGES {
            self.messages.remove(0);
        }
    }

    pub fn add_chat(&mut self, message: String) {
        self.add_message(message, WHITE);
    }

    pub fn add_own_chat(&mut self, message: String) {
        self.add_message(message, GREEN);
    }

    pub fn add_system(&mut self, message: String) {
        self.add_message(message, YELLOW);
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let screen_h = ui.ctx.screen_height;

        let chat_x = PADDING;
        let chat_y = screen_h - MSG_AREA_H - INPUT_H - PADDING * 2.0;

        // Activate chat on Enter when inactive
        if ui.ctx.key_enter && !self.active {
            self.active = true;
            ui.set_focus(INPUT_ID);
            self.draw_messages(ui, chat_x, chat_y);
            self.draw_input(ui, chat_x, chat_y + MSG_AREA_H);
            return events;
        }

        if self.active {
            if ui.ctx.key_enter {
                if !self.input.text.is_empty() {
                    let message = self.input.text.clone();
                    self.input.text.clear();
                    self.input.cursor_pos = 0;
                    events.push(GameEvent::RequestSendChat { message });
                }
                self.active = false;
            } else if ui.ctx.key_escape {
                self.input.text.clear();
                self.input.cursor_pos = 0;
                self.active = false;
            }
        }

        self.draw_messages(ui, chat_x, chat_y);

        if self.active {
            self.draw_input(ui, chat_x, chat_y + MSG_AREA_H);
            ui.text_input(INPUT_ID, Rect::new(chat_x, chat_y + MSG_AREA_H, CHAT_W, INPUT_H), &mut self.input, None);
        }

        events
    }

    fn draw_messages(&self, ui: &mut UiFrame, x: f32, y: f32) {
        use ragnarok_ui::draw;

        // Semi-transparent background
        let bg_color = [0.0, 0.0, 0.0, 0.4];
        let (v, i) = draw::quad_vertices(x, y, CHAT_W, MSG_AREA_H, bg_color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });

        // Visible lines from bottom
        let max_lines = (MSG_AREA_H / LINE_H) as usize;
        let start = self.messages.len().saturating_sub(max_lines);
        let visible = &self.messages[start..];

        for (i, line) in visible.iter().enumerate() {
            let text_y = y + PADDING + (i as f32) * LINE_H + ui.atlas.line_height;
            ui.text(x + PADDING, text_y, &line.text, line.color);
        }
    }

    fn draw_input(&self, ui: &mut UiFrame, x: f32, y: f32) {
        use ragnarok_ui::draw;

        let bg_color = [0.0, 0.0, 0.0, 0.6];
        let (v, i) = draw::quad_vertices(x, y, CHAT_W, INPUT_H, bg_color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_message_stores_and_trims() {
        let mut chat = ChatWindow::new();
        for i in 0..150 {
            chat.add_chat(format!("msg {i}"));
        }
        assert_eq!(chat.messages.len(), MAX_MESSAGES);
        assert_eq!(chat.messages[0].text, "msg 50");
        assert_eq!(chat.messages[99].text, "msg 149");
    }

    #[test]
    fn is_active_tracks_state() {
        let mut chat = ChatWindow::new();
        assert!(!chat.is_active());
        chat.active = true;
        assert!(chat.is_active());
    }
}
