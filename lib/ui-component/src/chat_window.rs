use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

const CHAT_WINDOW_ID: WidgetId = WidgetId(300);
const INPUT_ID: WidgetId = WidgetId(301);
const WHISPER_INPUT_ID: WidgetId = WidgetId(317);
const HEIGHT_DRAG_ID: WidgetId = WidgetId(303);
const WIDTH_DRAG_ID: WidgetId = WidgetId(304);
const SCROLL_UP_ID: WidgetId = WidgetId(305);
const SCROLL_DOWN_ID: WidgetId = WidgetId(306);
const SCROLL_THUMB_ID: WidgetId = WidgetId(308);
const SIZE_BTN_ID: WidgetId = WidgetId(302);
const FILTER_BTN_ID: WidgetId = WidgetId(310);
const CHATMODE_BTN_ID: WidgetId = WidgetId(311);
const MINIMIZE_BTN_ID: WidgetId = WidgetId(315);
const LOCK_BTN_ID: WidgetId = WidgetId(316);

const MAX_MESSAGES: usize = 100;
const MAX_HISTORY: usize = 32;
const MAX_INPUT_LEN: usize = 100;
const MAX_WHISPER_NAME_LEN: usize = 24;
const WHISPER_INPUT_W: f32 = 90.0;
const INPUT_GAP: f32 = 4.0;

// DIALOG_BG texture layout (600px wide)
const DIALOG_BG_W: f32 = 600.0;
const DIALOG_BG_WHISPER_X: f32 = 3.0;
const DIALOG_BG_WHISPER_W: f32 = 90.0;
const DIALOG_BG_MSG_X: f32 = 110.0;
const DIALOG_BG_MSG_W: f32 = 460.0;

const DEFAULT_CHAT_W: f32 = 350.0;
const MIN_CHAT_W: f32 = 250.0;
const MAX_CHAT_W: f32 = 600.0;
const MIN_MSG_AREA_H: f32 = 0.0;
const INPUT_H: f32 = 22.0;
const PADDING: f32 = 4.0;
const LINE_H: f32 = 14.0;
const DRAG_HANDLE_VISUAL: f32 = 3.0;
const DRAG_HIT_AREA: f32 = 6.0;
const SCROLLBAR_W: f32 = 14.0;
const SCROLL_BTN_H: f32 = 14.0;
const TOOLBAR_BTN_SIZE: f32 = 11.0;
const TOOLBAR_BTN_GAP: f32 = 2.0;
const TOOLBAR_H: f32 = 17.0;

// 3 lines per step
const SIZE_STEP: f32 = LINE_H * 3.0;
// Index 0 = hidden, 1 = collapsed (input only), 2..6 = message area heights
const SIZE_CYCLE: [f32; 7] = [0.0, 0.0, SIZE_STEP, SIZE_STEP * 2.0, SIZE_STEP * 3.0, SIZE_STEP * 4.0, SIZE_STEP * 5.0];
const DEFAULT_SIZE_INDEX: usize = 5;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const YELLOW: [f32; 4] = [1.0, 1.0, 0.4, 1.0];

// GRF texture paths
const DIALOG_BG: &str = "data/texture/유저인터페이스/basic_interface/dialog_bg.bmp";
const SCROLL_UP: &str = "data/texture/유저인터페이스/basic_interface/dialscr_up.bmp";
const SCROLL_DOWN: &str = "data/texture/유저인터페이스/basic_interface/dialscr_down.bmp";
const CHANNEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/basic_interface/dialog_btn0.bmp",
    hover: "data/texture/유저인터페이스/basic_interface/dialog_btn1.bmp",
    pressed: "data/texture/유저인터페이스/basic_interface/dialog_btn2.bmp",
};
const SYS_BASE_OFF: &str = "data/texture/유저인터페이스/basic_interface/sys_base_off.bmp";
const CHATMODE_ON: &str = "data/texture/유저인터페이스/basic_interface/chatmode_on.bmp";
const CHATMODE_OFF: &str = "data/texture/유저인터페이스/basic_interface/chatmode_off.bmp";
const NEW_TAB_BTN: &str = "data/texture/유저인터페이스/basic_interface/battle_option_a.bmp";
const BATTLE_OPT_BTN: &str = "data/texture/유저인터페이스/basic_interface/battle_option2_a.bmp";
const STICKY_BTN: &str = "data/texture/유저인터페이스/basic_interface/stickoff.bmp";
const MINIMIZE_BTN: &str = "data/texture/유저인터페이스/basic_interface/wnd_mini_b.bmp";
const LOCK_DRAG_BTN: &str = "data/texture/유저인터페이스/basic_interface/lock_dragwnd.bmp";
const UNLOCK_DRAG_BTN: &str = "data/texture/유저인터페이스/basic_interface/unlock_dragwnd.bmp";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatChannel {
    System,
    Public,
    Party,
    Guild,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatFilter {
    All,
    Public,
    Party,
    Guild,
}

impl ChatFilter {
    fn next(self) -> Self {
        match self {
            ChatFilter::All => ChatFilter::Public,
            ChatFilter::Public => ChatFilter::Party,
            ChatFilter::Party => ChatFilter::Guild,
            ChatFilter::Guild => ChatFilter::All,
        }
    }

    fn passes(self, channel: ChatChannel) -> bool {
        match self {
            ChatFilter::All => true,
            ChatFilter::Public => matches!(channel, ChatChannel::System | ChatChannel::Public),
            ChatFilter::Party => matches!(channel, ChatChannel::System | ChatChannel::Party),
            ChatFilter::Guild => matches!(channel, ChatChannel::System | ChatChannel::Guild),
        }
    }

    fn label(self) -> &'static str {
        match self {
            ChatFilter::All => "All",
            ChatFilter::Public => "Pub",
            ChatFilter::Party => "Pty",
            ChatFilter::Guild => "Gld",
        }
    }
}

#[derive(Default)]
struct ChatWindowState {
    size_index: usize,
    msg_area_h: f32,
    chat_w: f32,
    pos_x: f32,
    pos_y: f32,
    scroll_offset: usize,
    initialized: bool,
    chat_mode_on: bool,
    locked: bool,
    dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
    filter: u8, // ChatFilter encoded as u8 for Default
    drag_start_msg_h: f32,
    drag_start_chat_w: f32,
    drag_start_scroll: f32,
}

impl ChatWindowState {
    fn chat_filter(&self) -> ChatFilter {
        match self.filter {
            1 => ChatFilter::Public,
            2 => ChatFilter::Party,
            3 => ChatFilter::Guild,
            _ => ChatFilter::All,
        }
    }

    fn set_chat_filter(&mut self, f: ChatFilter) {
        self.filter = match f {
            ChatFilter::All => 0,
            ChatFilter::Public => 1,
            ChatFilter::Party => 2,
            ChatFilter::Guild => 3,
        };
    }
}

pub struct ChatLine {
    pub text: String,
    pub color: [f32; 4],
    pub channel: ChatChannel,
}

pub struct ChatWindow {
    pub input: TextInput,
    pub whisper_target: TextInput,
    pub messages: Vec<ChatLine>,
    pub active: bool,
    pub has_grf_textures: bool,
    pub focused_input: WidgetId,
    bounding_rect: Option<Rect>,
    sent_history: Vec<String>,
    history_index: Option<usize>,
    draft: String,
}

impl ChatWindow {
    pub fn new() -> Self {
        Self {
            input: TextInput::new(MAX_INPUT_LEN, false),
            whisper_target: TextInput::new(MAX_WHISPER_NAME_LEN, false),
            messages: Vec::new(),
            active: false,
            has_grf_textures: false,
            focused_input: INPUT_ID,
            bounding_rect: None,
            sent_history: Vec::new(),
            history_index: None,
            draft: String::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        self.bounding_rect.as_ref().is_some_and(|r| r.contains(x, y))
    }

    pub fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            DIALOG_BG,
            SCROLL_UP,
            SCROLL_DOWN,
            CHANNEL_BTN.normal,
            CHANNEL_BTN.hover,
            CHANNEL_BTN.pressed,
            SYS_BASE_OFF,
            CHATMODE_ON,
            CHATMODE_OFF,
            NEW_TAB_BTN,
            BATTLE_OPT_BTN,
            STICKY_BTN,
            MINIMIZE_BTN,
            LOCK_DRAG_BTN,
            UNLOCK_DRAG_BTN,
        ]
    }

    pub fn add_message(&mut self, text: String, color: [f32; 4], channel: ChatChannel) {
        self.messages.push(ChatLine { text, color, channel });
        if self.messages.len() > MAX_MESSAGES {
            self.messages.remove(0);
        }
    }

    pub fn add_chat(&mut self, message: String) {
        self.add_message(message, WHITE, ChatChannel::Public);
    }

    pub fn add_own_chat(&mut self, message: String) {
        self.add_message(message, GREEN, ChatChannel::Public);
    }

    pub fn add_system(&mut self, message: String) {
        self.add_message(message, YELLOW, ChatChannel::System);
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let screen_h = ui.ctx.screen_height;
        let input_h = INPUT_H ;
        let toolbar_h = TOOLBAR_H ;
        let padding = PADDING ;
        let line_h = LINE_H ;
        let scrollbar_w = SCROLLBAR_W ;
        let max_msg_h = screen_h - input_h - (50.0);

        // Initialize or read persistent state
        let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
        if !state.initialized {
            state.size_index = DEFAULT_SIZE_INDEX;
            state.msg_area_h = SIZE_CYCLE[DEFAULT_SIZE_INDEX];
            state.chat_w = DEFAULT_CHAT_W ;
            state.pos_x = padding;
            let default_h = SIZE_CYCLE[DEFAULT_SIZE_INDEX] + toolbar_h + input_h;
            state.pos_y = screen_h - default_h - padding;
            state.initialized = true;
        }

        // F10 cycles through predefined sizes
        if ui.ctx.key_f10 {
            state.size_index = (state.size_index + 1) % SIZE_CYCLE.len();
            state.msg_area_h = SIZE_CYCLE[state.size_index];
        }

        let size_index = state.size_index;
        let mut msg_area_h = state.msg_area_h;
        let mut chat_w = state.chat_w;

        // Index 0 = fully hidden
        if size_index == 0 {
            self.bounding_rect = None;
            return events;
        }

        let show_messages = size_index >= 2 && msg_area_h > 0.0;
        let total_h = if show_messages { msg_area_h + toolbar_h + input_h } else { toolbar_h + input_h };
        let chat_x = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).pos_x;
        let chat_y = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).pos_y;

        // Read lock state for drag gating
        let drag_locked = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).locked;

        // Height drag handle (between message area and input)
        if show_messages {
            let handle_center_y = chat_y + msg_area_h;
            let handle_rect = Rect::new(chat_x, handle_center_y - DRAG_HIT_AREA / 2.0, chat_w, DRAG_HIT_AREA);
            let h = ui.drag_handle(HEIGHT_DRAG_ID, handle_rect, !drag_locked);
            if h.started {
                ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).drag_start_msg_h = msg_area_h;
            }
            if h.dragging {
                let start = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).drag_start_msg_h;
                msg_area_h = (start - h.delta_y).clamp(MIN_MSG_AREA_H, max_msg_h);
            }
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            if h.dragging || msg_area_h != state.msg_area_h {
                state.msg_area_h = msg_area_h;
            }
        }

        // Width drag handle (right edge)
        {
            let right_edge_x = chat_x + chat_w;
            let handle_rect = Rect::new(right_edge_x - DRAG_HIT_AREA / 2.0, chat_y, DRAG_HIT_AREA, total_h);
            let w = ui.drag_handle(WIDTH_DRAG_ID, handle_rect, !drag_locked);
            if w.started {
                ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).drag_start_chat_w = chat_w;
            }
            if w.dragging {
                let start = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).drag_start_chat_w;
                chat_w = (start + w.delta_x).clamp(MIN_CHAT_W, MAX_CHAT_W);
            }
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            if w.dragging || chat_w != state.chat_w {
                state.chat_w = chat_w;
            }
        }

        // Recalculate layout after resize drag
        let show_messages = size_index >= 2 && msg_area_h > 0.0;
        let total_h = if show_messages { msg_area_h + toolbar_h + input_h } else { toolbar_h + input_h };
        let chat_x = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).pos_x;
        let chat_y = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).pos_y;

        self.bounding_rect = Some(Rect::new(chat_x, chat_y, chat_w, total_h));

        ui.ensure_in_z_order(CHAT_WINDOW_ID);
        ui.enter_window(CHAT_WINDOW_ID, self.bounding_rect.unwrap());
        if !ui.is_current_window_occluded() && ui.ctx.mouse_clicked && self.bounding_rect.unwrap().contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
            ui.bring_to_front(CHAT_WINDOW_ID);
        }

        // Activate chat on Enter when inactive
        if ui.ctx.key_enter && !self.active {
            self.active = true;
            ui.set_focus(INPUT_ID);
            // Draw will happen on next frame via main path
            return events;
        }

        if self.active {
            if ui.ctx.key_enter {
                if !self.input.text.is_empty() {
                    let message = self.input.text.clone();
                    if self.sent_history.last() != Some(&message) {
                        self.sent_history.push(message.clone());
                        if self.sent_history.len() > MAX_HISTORY {
                            self.sent_history.remove(0);
                        }
                    }
                    self.history_index = None;
                    self.draft.clear();
                    self.input.text.clear();
                    self.input.cursor_pos = 0;
                    events.push(GameEvent::RequestSendChat { message });
                }
                self.active = false;
            } else if ui.ctx.key_escape {
                self.input.text.clear();
                self.input.cursor_pos = 0;
                self.history_index = None;
                self.draft.clear();
                self.active = false;
            } else {
                if self.history_index.is_some() && !ui.ctx.typed_chars.is_empty() {
                    self.history_index = None;
                }

                if ui.ctx.key_up && !self.sent_history.is_empty() {
                    match self.history_index {
                        None => {
                            self.draft = self.input.text.clone();
                            self.history_index = Some(0);
                        }
                        Some(i) if i + 1 < self.sent_history.len() => {
                            self.history_index = Some(i + 1);
                        }
                        _ => {}
                    }
                    if let Some(i) = self.history_index {
                        let msg = &self.sent_history[self.sent_history.len() - 1 - i];
                        self.input.text = msg.clone();
                        self.input.cursor_pos = self.input.text.chars().count();
                    }
                }

                if ui.ctx.key_down {
                    match self.history_index {
                        Some(0) => {
                            self.history_index = None;
                            self.input.text = self.draft.clone();
                            self.input.cursor_pos = self.input.text.chars().count();
                        }
                        Some(i) => {
                            self.history_index = Some(i - 1);
                            let msg = &self.sent_history[self.sent_history.len() - 1 - (i - 1)];
                            self.input.text = msg.clone();
                            self.input.cursor_pos = self.input.text.chars().count();
                        }
                        None => {}
                    }
                }
            }
        }

        // Read button states
        let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
        let filter = state.chat_filter();
        let locked = state.locked;

        // Filter messages for display
        let filtered: Vec<&ChatLine> = self.messages.iter()
            .filter(|line| filter.passes(line.channel))
            .collect();

        if show_messages {
            // Wrap messages into visual lines
            let text_area_w = chat_w - padding * 2.0;
            let atlas = &*ui.atlas;
            let visual_lines: Vec<(String, [f32; 4])> = filtered.iter()
                .flat_map(|line| {
                    let wrapped = ragnarok_ui::draw::word_wrap(&line.text, text_area_w, |t| atlas.measure_text(t));
                    let color = line.color;
                    wrapped.into_iter().map(move |w| (w, color))
                })
                .collect();
            let total_visual = visual_lines.len();

            // Mouse wheel scroll when hovering message area
            let msg_rect = Rect::new(chat_x, chat_y, chat_w, msg_area_h);
            if msg_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) && ui.ctx.scroll_delta != 0.0 {
                let max_lines = (msg_area_h / line_h) as usize;
                let max_scroll = total_visual.saturating_sub(max_lines);
                let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
                let delta = ui.ctx.scroll_delta.round() as isize;
                let new_offset = (state.scroll_offset as isize + delta).clamp(0, max_scroll as isize);
                state.scroll_offset = new_offset as usize;
            }

            let scroll_offset = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).scroll_offset;
            self.draw_visual_lines(ui, chat_x, chat_y, chat_w, msg_area_h, scroll_offset, &visual_lines);
            self.draw_scrollbar_filtered(ui, chat_x + chat_w - scrollbar_w, chat_y, msg_area_h, scroll_offset, total_visual);
            self.draw_height_handle(ui, chat_x, chat_y + msg_area_h, chat_w);
        }

        let toolbar_y = chat_y + if show_messages { msg_area_h } else { 0.0 };
        let input_y = toolbar_y + toolbar_h;

        // Draw toolbar between messages and input
        self.draw_toolbar(ui, chat_x, toolbar_y, chat_w, filter, locked);

        // Toolbar drag to move window (when unlocked)
        if !drag_locked {
            let toolbar_rect = Rect::new(chat_x, toolbar_y, chat_w, toolbar_h);
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            if toolbar_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) && ui.ctx.mouse_clicked && !state.dragging {
                state.dragging = true;
                state.drag_offset_x = ui.ctx.mouse_x - state.pos_x;
                state.drag_offset_y = ui.ctx.mouse_y - state.pos_y;
            }
            if state.dragging {
                if ui.ctx.mouse_down {
                    state.pos_x = (ui.ctx.mouse_x - state.drag_offset_x)
                        .clamp(0.0, ui.ctx.screen_width - chat_w);
                    state.pos_y = (ui.ctx.mouse_y - state.drag_offset_y)
                        .clamp(0.0, ui.ctx.screen_height - total_h);
                } else {
                    state.dragging = false;
                }
            }
        }

        if self.active {
            // Tab switches focus between whisper target and message input
            if ui.ctx.key_tab {
                if self.focused_input == INPUT_ID {
                    self.focused_input = WHISPER_INPUT_ID;
                } else {
                    self.focused_input = INPUT_ID;
                }
                ui.set_focus(self.focused_input);
            }

            // Compute input rects — scale to match DIALOG_BG texture layout when available
            let (whisper_rect, msg_rect, input_bg) = if self.has_grf_textures {
                let scale = chat_w / (DIALOG_BG_W);
                let wr = Rect::new(chat_x + (DIALOG_BG_WHISPER_X) * scale, input_y, (DIALOG_BG_WHISPER_W) * scale, input_h);
                let mr = Rect::new(chat_x + (DIALOG_BG_MSG_X) * scale, input_y, (DIALOG_BG_MSG_W) * scale, input_h);

                let input_row = Rect::new(chat_x, input_y, chat_w, input_h);
                let (v, i) = ragnarok_ui::draw::quad_vertices(input_row.x, input_row.y, input_row.w, input_row.h, [1.0; 4]);
                ui.draw_calls.push(ragnarok_ui::draw::DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: ragnarok_ui::draw::TextureRef::Named(DIALOG_BG.to_string()),
                });
                (wr, mr, TextInputBg::Transparent)
            } else {
                let wr = Rect::new(chat_x, input_y, WHISPER_INPUT_W , input_h);
                let msg_x = chat_x + (WHISPER_INPUT_W) + (INPUT_GAP);
                let msg_w = chat_w - (WHISPER_INPUT_W) - (INPUT_GAP);
                let mr = Rect::new(msg_x, input_y, msg_w, input_h);
                (wr, mr, TextInputBg::Default)
            };

            // Track click-to-focus
            if ui.ctx.mouse_clicked {
                if whisper_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
                    self.focused_input = WHISPER_INPUT_ID;
                } else if msg_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
                    self.focused_input = INPUT_ID;
                }
            }

            // Whisper target input (left)
            ui.text_input(WHISPER_INPUT_ID, whisper_rect, &mut self.whisper_target, input_bg);

            // Message input (right)
            ui.text_input(INPUT_ID, msg_rect, &mut self.input, input_bg);
        }

        self.draw_width_handle(ui, chat_x + chat_w, chat_y, total_h);

        events
    }

    fn draw_visual_lines(&self, ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, scroll_offset: usize, visual_lines: &[(String, [f32; 4])]) {
        use ragnarok_ui::draw;
        let line_h = LINE_H ;
        let padding = PADDING ;

        // Semi-transparent background
        let bg_color = [0.0, 0.0, 0.0, 0.8];
        let (v, i) = draw::quad_vertices(x, y, w, h, bg_color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });

        let max_lines = (h / line_h) as usize;
        let end = visual_lines.len().saturating_sub(scroll_offset);
        let start = end.saturating_sub(max_lines);
        let visible = &visual_lines[start..end];

        for (i, (text, color)) in visible.iter().enumerate() {
            let text_y = y + padding + (i as f32) * line_h + ui.atlas.line_height;
            ui.text(x + padding, text_y, text, *color);
        }
    }

    fn draw_scrollbar_filtered(&self, ui: &mut UiFrame, x: f32, y: f32, h: f32, scroll_offset: usize, total: usize) {
        use ragnarok_ui::draw;
        let line_h = LINE_H ;
        let scrollbar_w = SCROLLBAR_W ;
        let scroll_btn_h = SCROLL_BTN_H ;

        let max_lines = (h / line_h) as usize;
        let max_scroll = total.saturating_sub(max_lines);

        // Track background
        let track_color = [0.0, 0.0, 0.0, 0.3];
        let (v, i) = draw::quad_vertices(x, y, scrollbar_w, h, track_color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });

        // Up button
        let up_rect = Rect::new(x, y, scrollbar_w, scroll_btn_h);
        let up_response = ui.interact(SCROLL_UP_ID, up_rect);
        if up_response.hovered() { ui.any_interactive_hovered = true; }
        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(x, y, scrollbar_w, scroll_btn_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::Named(SCROLL_UP.to_string()),
            });
        } else {
            let color = if up_response.hovered() { [0.5, 0.5, 0.6, 1.0] } else { [0.3, 0.3, 0.4, 1.0] };
            let (v, i) = draw::quad_vertices(x, y, scrollbar_w, scroll_btn_h, color);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
            ui.text(x + (3.0), y + ui.atlas.line_height, "\u{25B2}", [0.8, 0.8, 0.8, 1.0]);
        }

        // Down button
        let down_y = y + h - scroll_btn_h;
        let down_rect = Rect::new(x, down_y, scrollbar_w, scroll_btn_h);
        let down_response = ui.interact(SCROLL_DOWN_ID, down_rect);
        if down_response.hovered() { ui.any_interactive_hovered = true; }
        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(x, down_y, scrollbar_w, scroll_btn_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::Named(SCROLL_DOWN.to_string()),
            });
        } else {
            let color = if down_response.hovered() { [0.5, 0.5, 0.6, 1.0] } else { [0.3, 0.3, 0.4, 1.0] };
            let (v, i) = draw::quad_vertices(x, down_y, scrollbar_w, scroll_btn_h, color);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
            ui.text(x + (3.0), down_y + ui.atlas.line_height, "\u{25BC}", [0.8, 0.8, 0.8, 1.0]);
        }

        // Handle scroll button clicks
        if up_response.clicked() || down_response.clicked() {
            let delta: isize = if up_response.clicked() { 1 } else { -1 };
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            let new_offset = (state.scroll_offset as isize + delta).clamp(0, max_scroll as isize);
            state.scroll_offset = new_offset as usize;
        }

        // Thumb (proportional indicator + drag)
        if total > max_lines && max_scroll > 0 {
            let track_h = h - scroll_btn_h * 2.0;
            let track_top = y + scroll_btn_h;
            let thumb_ratio = max_lines as f32 / total as f32;
            let thumb_h = (track_h * thumb_ratio).max(10.0);
            let scroll_ratio = scroll_offset as f32 / max_scroll as f32;
            // scroll_offset 0 = at bottom, max = at top, so thumb starts at bottom when offset=0
            let thumb_y = track_top + (track_h - thumb_h) * (1.0 - scroll_ratio);

            // Thumb drag interaction
            let thumb_rect = Rect::new(x, thumb_y, scrollbar_w, thumb_h);
            let t = ui.drag_handle(SCROLL_THUMB_ID, thumb_rect, true);
            if t.started {
                ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).drag_start_scroll = scroll_offset as f32;
            }
            if t.dragging {
                let start = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).drag_start_scroll;
                let delta_scroll = -(t.delta_y / track_h) * max_scroll as f32;
                let offset = (start + delta_scroll).round().clamp(0.0, max_scroll as f32);
                ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).scroll_offset = offset as usize;
            }

            let thumb_active = t.dragging || t.hovered;
            let thumb_color = if thumb_active { [0.6, 0.6, 0.7, 0.9] } else { [0.5, 0.5, 0.6, 0.8] };
            let (v, i) = draw::quad_vertices(x + (2.0), thumb_y, scrollbar_w - (4.0), thumb_h, thumb_color);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
        }
    }

    fn draw_toolbar(&self, ui: &mut UiFrame, x: f32, y: f32, w: f32, filter: ChatFilter, locked: bool) {
        use ragnarok_ui::draw;
        let toolbar_h = TOOLBAR_H ;
        let btn_size = TOOLBAR_BTN_SIZE ;
        let btn_gap = TOOLBAR_BTN_GAP ;

        // Toolbar background
        let bg_color = [0.0, 0.0, 0.0, 0.3];
        let (v, i) = draw::quad_vertices(x, y, w, toolbar_h, bg_color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });

        // Buttons laid out from right to left
        // Visual position centered vertically, but hit area uses full toolbar height
        let btn_visual_y = y + (toolbar_h - btn_size) / 2.0;
        let mut btn_x = x + w - btn_gap;

        // Lock button
        btn_x -= btn_size;
        let lock_rect = Rect::new(btn_x, y, btn_size, toolbar_h);
        let lock_resp = ui.interact(LOCK_BTN_ID, lock_rect);
        if lock_resp.hovered() { ui.any_interactive_hovered = true; }
        if self.has_grf_textures {
            let tex = if locked { LOCK_DRAG_BTN } else { UNLOCK_DRAG_BTN };
            let (v, i) = draw::quad_vertices(btn_x, btn_visual_y, btn_size, btn_size, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(), indices: i.to_vec(),
                texture: draw::TextureRef::Named(tex.to_string()),
            });
        } else {
            let color = if locked { [0.6, 0.3, 0.3, 1.0] } else if lock_resp.hovered() { [0.5, 0.5, 0.6, 1.0] } else { [0.3, 0.3, 0.4, 1.0] };
            let (v, i) = draw::quad_vertices(btn_x, btn_visual_y, btn_size, btn_size, color);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(), indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
        }
        btn_x -= btn_gap;

        // Minimize button
        btn_x -= btn_size;
        let min_rect = Rect::new(btn_x, y, btn_size, toolbar_h);
        let min_resp = ui.interact(MINIMIZE_BTN_ID, min_rect);
        if min_resp.hovered() { ui.any_interactive_hovered = true; }
        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(btn_x, btn_visual_y, btn_size, btn_size, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(), indices: i.to_vec(),
                texture: draw::TextureRef::Named(MINIMIZE_BTN.to_string()),
            });
        } else {
            let color = if min_resp.hovered() { [0.5, 0.5, 0.6, 1.0] } else { [0.3, 0.3, 0.4, 1.0] };
            let (v, i) = draw::quad_vertices(btn_x, btn_visual_y, btn_size, btn_size, color);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(), indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
        }
        btn_x -= btn_gap;

        // Chat mode toggle
        let mode_w = btn_size + (4.0);
        btn_x -= mode_w;
        let mode_rect = Rect::new(btn_x, y, mode_w, toolbar_h);
        let mode_resp = ui.interact(CHATMODE_BTN_ID, mode_rect);
        if mode_resp.hovered() { ui.any_interactive_hovered = true; }
        let chat_mode_on = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).chat_mode_on;
        if self.has_grf_textures {
            let tex = if chat_mode_on { CHATMODE_ON } else { CHATMODE_OFF };
            let (v, i) = draw::quad_vertices(btn_x, btn_visual_y, mode_w, btn_size, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(), indices: i.to_vec(),
                texture: draw::TextureRef::Named(tex.to_string()),
            });
        } else {
            let color = if chat_mode_on { [0.3, 0.6, 0.3, 1.0] } else if mode_resp.hovered() { [0.5, 0.5, 0.6, 1.0] } else { [0.3, 0.3, 0.4, 1.0] };
            let (v, i) = draw::quad_vertices(btn_x, btn_visual_y, mode_w, btn_size, color);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(), indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
        }
        btn_x -= btn_gap;

        // Size button
        btn_x -= btn_size;
        let size_rect = Rect::new(btn_x, y, btn_size, toolbar_h);
        let size_resp = ui.interact(SIZE_BTN_ID, size_rect);
        if size_resp.hovered() { ui.any_interactive_hovered = true; }
        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(btn_x, btn_visual_y, btn_size, btn_size, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(), indices: i.to_vec(),
                texture: draw::TextureRef::Named(SYS_BASE_OFF.to_string()),
            });
        } else {
            let color = if size_resp.hovered() { [0.5, 0.5, 0.6, 1.0] } else { [0.3, 0.3, 0.4, 1.0] };
            let (v, i) = draw::quad_vertices(btn_x, btn_visual_y, btn_size, btn_size, color);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(), indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
        }
        btn_x -= btn_gap;

        // Filter button with label
        let filter_w = 24.0 ;
        btn_x -= filter_w;
        let filter_rect = Rect::new(btn_x, y, filter_w, toolbar_h);
        let filter_resp = ui.interact(FILTER_BTN_ID, filter_rect);
        if filter_resp.hovered() { ui.any_interactive_hovered = true; }
        let color = if filter_resp.hovered() { [0.5, 0.5, 0.6, 1.0] } else { [0.3, 0.3, 0.4, 1.0] };
        let (v, i) = draw::quad_vertices(btn_x, btn_visual_y, filter_w, btn_size, color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(), indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });
        ui.text(btn_x + (2.0), btn_visual_y + ui.atlas.line_height, filter.label(), [0.8, 0.8, 0.8, 1.0]);

        // Handle button clicks
        if lock_resp.clicked() {
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            state.locked = !state.locked;
        }
        if min_resp.clicked() {
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            state.size_index = 1; // collapsed
            state.msg_area_h = SIZE_CYCLE[1];
        }
        if mode_resp.clicked() {
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            state.chat_mode_on = !state.chat_mode_on;
        }
        if size_resp.clicked() {
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            state.size_index = (state.size_index + 1) % SIZE_CYCLE.len();
            state.msg_area_h = SIZE_CYCLE[state.size_index];
        }
        if filter_resp.clicked() {
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            let next = state.chat_filter().next();
            state.set_chat_filter(next);
        }
    }

    fn draw_height_handle(&self, ui: &mut UiFrame, x: f32, y: f32, w: f32) {
        use ragnarok_ui::draw;
        let color = [0.4, 0.4, 0.5, 0.8];
        let visual_y = y - DRAG_HANDLE_VISUAL / 2.0;
        let (v, i) = draw::quad_vertices(x, visual_y, w, DRAG_HANDLE_VISUAL, color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });
    }

    fn draw_width_handle(&self, ui: &mut UiFrame, x: f32, y: f32, h: f32) {
        use ragnarok_ui::draw;
        let color = [0.4, 0.4, 0.5, 0.8];
        let visual_x = x - DRAG_HANDLE_VISUAL / 2.0;
        let (v, i) = draw::quad_vertices(visual_x, y, DRAG_HANDLE_VISUAL, h, color);
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
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_renderer::font_atlas::FontAtlas;

    fn make_frame<'a>(ctx: &'a UiContext, atlas: &'a FontAtlas, state: &'a mut StateCache) -> UiFrame<'a> {
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

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

    #[test]
    fn f10_cycles_through_all_sizes() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut state = StateCache::new();

        // First build initializes to DEFAULT_SIZE_INDEX (5)
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        assert_eq!(ws.size_index, DEFAULT_SIZE_INDEX);

        // Each F10 press cycles forward
        for expected_index in [6, 0, 1, 2, 3, 4, 5] {
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_f10 = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui);
            let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
            assert_eq!(ws.size_index, expected_index);
        }
    }

    #[test]
    fn hidden_mode_produces_no_draw_calls() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut state = StateCache::new();

        // Initialize
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        // Set to hidden (index 0): cycle from 5 -> 6 -> 0
        for _ in 0..2 {
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_f10 = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui);
        }

        let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        assert_eq!(ws.size_index, 0);

        // Build in hidden mode should produce no draw calls
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert!(ui.draw_calls.is_empty());
        assert!(chat.bounding_rect.is_none());
    }

    #[test]
    fn collapsed_mode_shows_input_only_when_active() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut state = StateCache::new();

        // Initialize
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        // Cycle to index 1 (collapsed): 5 -> 6 -> 0 -> 1
        for _ in 0..3 {
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_f10 = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui);
        }

        let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        assert_eq!(ws.size_index, 1);

        // When active in collapsed mode, should draw input bg only (no message area)
        chat.active = true;
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        // 1 draw call for input bg + text_input draw calls
        assert!(!ui.draw_calls.is_empty());
        assert!(chat.bounding_rect.is_some());
        let rect = chat.bounding_rect.unwrap();
        assert_eq!(rect.h, TOOLBAR_H + INPUT_H);
    }

    #[test]
    fn contains_point_checks_bounding_rect() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut state = StateCache::new();

        // Build to establish bounding rect
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.active = true;
        chat.build(&mut ui);

        let rect = chat.bounding_rect.unwrap();
        assert!(chat.contains_point(rect.x + 1.0, rect.y + 1.0));
        assert!(!chat.contains_point(0.0, 0.0));
    }

    #[test]
    fn height_drag_changes_msg_area_h() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut state = StateCache::new();

        // Initialize
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        let initial_h = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().msg_area_h;

        // Click on the height drag handle (at the boundary between msg area and toolbar)
        let handle_y = 600.0 - TOOLBAR_H - INPUT_H - PADDING;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 100.0;
        ctx.mouse_y = handle_y;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        // Drag upward by 50px
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 100.0;
        ctx.mouse_y = handle_y - 50.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        let new_h = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().msg_area_h;
        assert!(new_h > initial_h, "Height should increase when dragging up: {} > {}", new_h, initial_h);
    }

    #[test]
    fn width_drag_changes_chat_w() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut state = StateCache::new();

        // Initialize
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        let initial_w = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().chat_w;

        // Click on the right edge
        let rect = chat.bounding_rect.unwrap();
        let edge_x = rect.x + rect.w;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        // Drag right by 80px
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x + 80.0;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        let new_w = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().chat_w;
        assert!(new_w > initial_w, "Width should increase when dragging right: {} > {}", new_w, initial_w);
    }

    #[test]
    fn drag_respects_min_max_constraints() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut state = StateCache::new();

        // Initialize
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        // Try to drag width way beyond max
        let rect = chat.bounding_rect.unwrap();
        let edge_x = rect.x + rect.w;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x + 1000.0;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        let w = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().chat_w;
        assert!(w <= MAX_CHAT_W, "Width should be clamped to max: {} <= {}", w, MAX_CHAT_W);
    }

    #[test]
    fn scroll_offset_clamps_to_valid_range() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut state = StateCache::new();

        // Add some messages
        for i in 0..50 {
            chat.add_chat(format!("msg {i}"));
        }

        // Initialize
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        // Manually set scroll_offset to a huge value
        state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).scroll_offset = 500;

        // Scroll up with mouse wheel while over chat - should clamp
        let rect = chat.bounding_rect.unwrap();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = rect.x + 10.0;
        ctx.mouse_y = rect.y + 10.0;
        ctx.scroll_delta = 1.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        let offset = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().scroll_offset;
        let max_lines = (SIZE_CYCLE[DEFAULT_SIZE_INDEX] / LINE_H) as usize;
        let max_scroll = 50_usize.saturating_sub(max_lines);
        assert!(offset <= max_scroll, "Scroll offset should be clamped: {} <= {}", offset, max_scroll);
    }

    #[test]
    fn mouse_wheel_scrolls_when_over_chat() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut state = StateCache::new();

        for i in 0..50 {
            chat.add_chat(format!("msg {i}"));
        }

        // Initialize
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        assert_eq!(state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().scroll_offset, 0);

        // Scroll up (positive delta) while hovering over message area
        let rect = chat.bounding_rect.unwrap();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = rect.x + 10.0;
        ctx.mouse_y = rect.y + 10.0;
        ctx.scroll_delta = 3.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        let offset = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().scroll_offset;
        assert_eq!(offset, 3, "Scroll offset should increase by 3");
    }

    #[test]
    fn filter_cycles_and_filters_messages() {
        let mut chat = ChatWindow::new();
        chat.add_chat("public msg".to_string());
        chat.add_message("party msg".to_string(), [0.5, 0.5, 1.0, 1.0], ChatChannel::Party);
        chat.add_message("guild msg".to_string(), [0.5, 1.0, 0.5, 1.0], ChatChannel::Guild);
        chat.add_system("system msg".to_string());

        let filter_all = ChatFilter::All;
        assert_eq!(chat.messages.iter().filter(|m| filter_all.passes(m.channel)).count(), 4);

        let filter_pub = ChatFilter::Public;
        let pub_msgs: Vec<_> = chat.messages.iter().filter(|m| filter_pub.passes(m.channel)).collect();
        assert_eq!(pub_msgs.len(), 2); // public + system
        assert_eq!(pub_msgs[0].text, "public msg");
        assert_eq!(pub_msgs[1].text, "system msg");

        let filter_party = ChatFilter::Party;
        let party_msgs: Vec<_> = chat.messages.iter().filter(|m| filter_party.passes(m.channel)).collect();
        assert_eq!(party_msgs.len(), 2); // party + system

        let filter_guild = ChatFilter::Guild;
        let guild_msgs: Vec<_> = chat.messages.iter().filter(|m| filter_guild.passes(m.channel)).collect();
        assert_eq!(guild_msgs.len(), 2); // guild + system
    }

    #[test]
    fn filter_cycles_through_all_modes() {
        let mut f = ChatFilter::All;
        f = f.next(); assert_eq!(f, ChatFilter::Public);
        f = f.next(); assert_eq!(f, ChatFilter::Party);
        f = f.next(); assert_eq!(f, ChatFilter::Guild);
        f = f.next(); assert_eq!(f, ChatFilter::All);
    }

    #[test]
    fn lock_button_prevents_drag() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut state = StateCache::new();

        // Initialize
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        // Set locked
        state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID).locked = true;
        let initial_w = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().chat_w;

        // Try to drag width
        let rect = chat.bounding_rect.unwrap();
        let edge_x = rect.x + rect.w;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x + 100.0;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);

        let w = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().chat_w;
        assert_eq!(w, initial_w, "Width should not change when locked");
    }

    #[test]
    fn chat_input_history_navigation() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut state = StateCache::new();

        // Initialize and activate
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert!(chat.active);

        // Send 3 messages
        let messages = ["hello", "world", "test"];
        for msg in &messages {
            chat.active = true;
            chat.input.text = msg.to_string();
            chat.input.cursor_pos = msg.len();
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_enter = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui);
        }
        assert_eq!(chat.sent_history, vec!["hello", "world", "test"]);

        // Activate chat again
        chat.active = true;

        // Up -> most recent ("test")
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_up = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert_eq!(chat.input.text, "test");
        assert_eq!(chat.history_index, Some(0));

        // Up -> second most recent ("world")
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_up = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert_eq!(chat.input.text, "world");
        assert_eq!(chat.history_index, Some(1));

        // Down -> back to most recent ("test")
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert_eq!(chat.input.text, "test");
        assert_eq!(chat.history_index, Some(0));

        // Down -> back to draft (empty)
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert_eq!(chat.input.text, "");
        assert!(chat.history_index.is_none());

        // Duplicate suppression: send "test" again
        chat.active = true;
        chat.input.text = "test".to_string();
        chat.input.cursor_pos = 4;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert_eq!(chat.sent_history, vec!["hello", "world", "test"]);

        // Max history cap
        chat.sent_history.clear();
        for i in 0..MAX_HISTORY + 5 {
            chat.active = true;
            chat.input.text = format!("msg{i}");
            chat.input.cursor_pos = chat.input.text.len();
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_enter = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui);
        }
        assert_eq!(chat.sent_history.len(), MAX_HISTORY);
        assert_eq!(chat.sent_history[0], "msg5");

        // Draft preservation: type something, then browse history
        chat.active = true;
        chat.input.text = "draft text".to_string();
        chat.input.cursor_pos = 10;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_up = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert_eq!(chat.draft, "draft text");
        assert_ne!(chat.input.text, "draft text");

        // Down to restore draft
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert_eq!(chat.input.text, "draft text");
    }

    #[test]
    fn tab_switches_between_inputs() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut state = StateCache::new();

        // Initialize and activate chat
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui);
        assert!(chat.active);

        // Default focus should be on message input (INPUT_ID = 301)
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        // Set initial focus to message input
        ui.set_focus(INPUT_ID);
        chat.build(&mut ui);
        assert_eq!(ui.focused(), Some(INPUT_ID));

        // Press Tab - should switch to whisper input
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_tab = true;
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        let mut ui = UiFrame::new(&ctx, &atlas, &mut state, 0.0, false, Some(INPUT_ID), positions);
        chat.build(&mut ui);
        assert_eq!(ui.focused(), Some(WHISPER_INPUT_ID));

        // Press Tab again - should switch back to message input
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_tab = true;
        let mut ui = UiFrame::new(&ctx, &atlas, &mut state, 0.0, false, Some(WHISPER_INPUT_ID), positions);
        chat.build(&mut ui);
        assert_eq!(ui.focused(), Some(INPUT_ID));
    }
}
