use crate::{BuildCtx, InGameWindow, Window};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::state::StateCache;
use ragnarok_ui::text_input::TextInput;

pub const CHAT_WINDOW_ID: WidgetId = WidgetId(300);
const INPUT_ID: WidgetId = WidgetId(301);
const WHISPER_INPUT_ID: WidgetId = WidgetId(317);
const HEIGHT_DRAG_ID: WidgetId = WidgetId(303);
const WIDTH_DRAG_ID: WidgetId = WidgetId(304);
const SCROLL_UP_ID: WidgetId = WidgetId(305);
const SCROLL_DOWN_ID: WidgetId = WidgetId(306);
const SCROLL_THUMB_ID: WidgetId = WidgetId(308);
const SIZE_BTN_ID: WidgetId = WidgetId(302);
const CHANNEL_BTN_ID: WidgetId = WidgetId(311);
const CHANNEL_MENU_ITEM_BASE: u32 = 330;
const WHISPER_MENU_BTN_ID: WidgetId = WidgetId(320);
const WHISPER_MENU_ITEM_BASE: u32 = 321;
const MAX_WHISPER_HISTORY: usize = 7;
const WHISPER_MENU_ITEM_H: f32 = 16.0;
const MENU_TEXT_BASELINE: f32 = 12.0;

const MAX_MESSAGES: usize = 100;
const MAX_HISTORY: usize = 32;
const MAX_INPUT_LEN: usize = 100;
const MAX_WHISPER_NAME_LEN: usize = 24;
const INPUT_GAP: f32 = 4.0;

const DEFAULT_CHAT_W: f32 = 550.0;
const MIN_CHAT_W: f32 = 550.0;
const MAX_CHAT_W: f32 = 2000.0;
const MIN_MSG_AREA_H: f32 = 0.0;
const INPUT_H: f32 = 22.0;
const PADDING: f32 = 4.0;
const LINE_H: f32 = 14.0;
const DRAG_HANDLE_VISUAL: f32 = 3.0;
const DRAG_HIT_AREA: f32 = 6.0;
const SCROLLBAR_W: f32 = 14.0;
const SCROLL_BTN_H: f32 = 14.0;
const BUBBLE_SIZE: f32 = 10.0;
const BUBBLE_GAP: f32 = 2.0;
const LIST_BTN_W: f32 = 8.0;
const CHANNEL_MENU_ITEM_H: f32 = 16.0;

// dialog_bg.bmp is 600px wide with fixed painted wells; the input row stretches
// it to chat_w, so each field sits at its native well coordinate scaled by chat_w/600.
const TEX_NATIVE_W: f32 = 600.0;
const WHISPER_NATIVE_X: f32 = 4.0;
const WHISPER_NATIVE_W: f32 = 90.0;
const LIST_NATIVE_X: f32 = 97.0;
const MSG_NATIVE_X: f32 = 108.0;
const MSG_NATIVE_RIGHT: f32 = 32.0;

const SIZE_STEP: f32 = LINE_H * 3.0;
const SIZE_CYCLE: [f32; 7] = [
    0.0,
    0.0,
    SIZE_STEP,
    SIZE_STEP * 2.0,
    SIZE_STEP * 3.0,
    SIZE_STEP * 4.0,
    SIZE_STEP * 5.0,
];
const DEFAULT_SIZE_INDEX: usize = 5;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const YELLOW: [f32; 4] = [1.0, 1.0, 0.4, 1.0];
const RED: [f32; 4] = [1.0, 0.1, 0.1, 1.0];
pub const PARTY_COLOR: [f32; 4] = [0.173, 0.576, 0.859, 1.0];
pub const GUILD_COLOR: [f32; 4] = [0.427, 0.996, 0.012, 1.0];
pub const WHISPER_IN_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
pub const WHISPER_OUT_COLOR: [f32; 4] = [1.0, 1.0, 0.0, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SendChannel {
    #[default]
    Public,
    Party,
    Guild,
    Whisper,
}

const SEND_CHANNELS: [SendChannel; 3] =
    [SendChannel::Public, SendChannel::Party, SendChannel::Guild];

impl SendChannel {
    fn color(self) -> [f32; 4] {
        match self {
            SendChannel::Public => WHITE,
            SendChannel::Party => PARTY_COLOR,
            SendChannel::Guild => GUILD_COLOR,
            SendChannel::Whisper => WHISPER_OUT_COLOR,
        }
    }

    fn label(self) -> &'static str {
        match self {
            SendChannel::Public => "Public",
            SendChannel::Party => "Party",
            SendChannel::Guild => "Guild",
            SendChannel::Whisper => "Whisper",
        }
    }
}

/// Lines that fit fully inside a message area of height `h`, accounting for the
/// top padding so the bottom line is never clipped by the input row drawn over it.
fn visible_line_count(h: f32) -> usize {
    ((h - PADDING) / LINE_H).max(0.0) as usize
}

fn route_message(message: String, send_channel: SendChannel) -> String {
    if message.starts_with(['/', '%', '$']) {
        return message;
    }
    match send_channel {
        SendChannel::Party => format!("%{message}"),
        SendChannel::Guild => format!("${message}"),
        _ => message,
    }
}

/// The channel a typed line will go to. Explicit prefix/modifier overrides win
/// first, then a filled whisper target, then the sticky selection.
fn effective_channel(
    msg: &str,
    sticky: SendChannel,
    ctrl: bool,
    alt: bool,
    whisper_active: bool,
) -> SendChannel {
    if msg.starts_with('%') || ctrl {
        SendChannel::Party
    } else if msg.starts_with('$') || msg.starts_with("/gc ") || alt {
        SendChannel::Guild
    } else if msg.starts_with('/') {
        SendChannel::Public
    } else if whisper_active {
        SendChannel::Whisper
    } else {
        sticky
    }
}

const DIALOG_BG: &str = ragnarok_resources::ui::basic::DIALOG_BG;
const SCROLL_UP: &str = ragnarok_resources::ui::basic::DIALSCR_UP;
const SCROLL_DOWN: &str = ragnarok_resources::ui::basic::DIALSCR_DOWN;
const LIST_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::basic::DIALOG_BTN0,
    hover: ragnarok_resources::ui::basic::DIALOG_BTN1,
    pressed: ragnarok_resources::ui::basic::DIALOG_BTN2,
};
const BUBBLE_TEX: &str = ragnarok_resources::ui::basic::SYS_BASE_OFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatChannel {
    System,
    Public,
    Party,
    Guild,
    Whisper,
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
    send_channel: SendChannel,
    channel_menu_open: bool,
    dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
    drag_start_msg_h: f32,
    drag_start_chat_w: f32,
    drag_start_scroll: f32,
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
    initial_size_index: Option<usize>,
    pending_focus: bool,
    whisper_history: Vec<String>,
    whisper_menu_open: bool,
}

impl Default for ChatWindow {
    fn default() -> Self {
        Self::new()
    }
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
            initial_size_index: None,
            pending_focus: false,
            whisper_history: Vec::new(),
            whisper_menu_open: false,
        }
    }

    /// Remembers a recent whisper partner (most recent first, capped), so the
    /// Whisper tab's history dropdown can re-target them without retyping.
    pub fn remember_whisper(&mut self, name: String) {
        if name.is_empty() {
            return;
        }
        self.whisper_history.retain(|n| n != &name);
        self.whisper_history.insert(0, name);
        self.whisper_history.truncate(MAX_WHISPER_HISTORY);
    }

    pub fn start_whisper(&mut self, name: String) {
        self.whisper_target.text = name;
        self.whisper_target.cursor_pos = self.whisper_target.text.chars().count();
        self.active = true;
        self.pending_focus = true;
    }

    pub fn set_initial_size_index(&mut self, index: usize) {
        self.initial_size_index = Some(index);
    }

    pub fn get_size_index(&self, state: &StateCache) -> usize {
        state
            .get::<ChatWindowState>(CHAT_WINDOW_ID)
            .map(|s| s.size_index)
            .unwrap_or(DEFAULT_SIZE_INDEX)
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Drops the input focus and the half-typed line without sending it.
    pub fn cancel_input(&mut self) {
        self.input.text.clear();
        self.input.cursor_pos = 0;
        self.history_index = None;
        self.draft.clear();
        self.active = false;
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        self.bounding_rect
            .as_ref()
            .is_some_and(|r| r.contains(x, y))
    }

    pub fn add_message(&mut self, text: String, color: [f32; 4], channel: ChatChannel) {
        self.messages.push(ChatLine {
            text,
            color,
            channel,
        });
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

    pub fn add_chat_colored(&mut self, message: String, color: [f32; 4]) {
        self.add_message(message, color, ChatChannel::Public);
    }

    pub fn add_notice(&mut self, message: String) {
        self.add_message(message, WHITE, ChatChannel::System);
    }

    pub fn add_system(&mut self, message: String) {
        self.add_message(message, YELLOW, ChatChannel::System);
    }

    pub fn add_party(&mut self, message: String) {
        self.add_message(message, PARTY_COLOR, ChatChannel::Party);
    }

    pub fn add_guild(&mut self, message: String) {
        self.add_message(message, GUILD_COLOR, ChatChannel::Guild);
    }

    pub fn add_whisper_in(&mut self, sender: String, message: String) {
        self.add_message(
            format!("(From {sender}) : {message}"),
            WHISPER_IN_COLOR,
            ChatChannel::Whisper,
        );
    }

    pub fn add_whisper_out(&mut self, name: String, message: String) {
        self.add_message(
            format!("(To {name}) : {message}"),
            WHISPER_OUT_COLOR,
            ChatChannel::Whisper,
        );
    }

    pub fn add_error(&mut self, message: String) {
        self.add_message(message, RED, ChatChannel::System);
    }

    fn draw_visual_lines(
        &self,
        ui: &mut UiFrame,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        scroll_offset: usize,
        visual_lines: &[(String, [f32; 4])],
    ) {
        use ragnarok_ui::draw;
        let line_h = LINE_H;
        let padding = PADDING;

        let bg_color = [0.0, 0.0, 0.0, 0.8];
        let (v, i) = draw::quad_vertices(x, y, w, h, bg_color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });

        let max_lines = visible_line_count(h);
        let end = visual_lines.len().saturating_sub(scroll_offset);
        let start = end.saturating_sub(max_lines);
        let visible = &visual_lines[start..end];

        for (i, (text, color)) in visible.iter().enumerate() {
            let text_y = y + padding + (i as f32) * line_h + ui.atlas.line_height;
            ui.text(x + padding, text_y, text, *color);
        }
    }

    fn draw_scrollbar_filtered(
        &self,
        ui: &mut UiFrame,
        x: f32,
        y: f32,
        h: f32,
        scroll_offset: usize,
        total: usize,
    ) {
        use ragnarok_ui::draw;
        let scrollbar_w = SCROLLBAR_W;
        let scroll_btn_h = SCROLL_BTN_H;

        let max_lines = visible_line_count(h);
        let max_scroll = total.saturating_sub(max_lines);

        let track_color = [0.0, 0.0, 0.0, 0.3];
        let (v, i) = draw::quad_vertices(x, y, scrollbar_w, h, track_color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });

        let up_rect = Rect::new(x, y, scrollbar_w, scroll_btn_h);
        let up_response = ui.interact(SCROLL_UP_ID, up_rect);
        if up_response.hovered() {
            ui.any_interactive_hovered = true;
        }
        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(x, y, scrollbar_w, scroll_btn_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::Named(SCROLL_UP.to_string()),
            });
        } else {
            let color = if up_response.hovered() {
                [0.5, 0.5, 0.6, 1.0]
            } else {
                [0.3, 0.3, 0.4, 1.0]
            };
            let (v, i) = draw::quad_vertices(x, y, scrollbar_w, scroll_btn_h, color);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
            ui.text(
                x + (3.0),
                y + ui.atlas.line_height,
                "\u{25B2}",
                [0.8, 0.8, 0.8, 1.0],
            );
        }

        let down_y = y + h - scroll_btn_h;
        let down_rect = Rect::new(x, down_y, scrollbar_w, scroll_btn_h);
        let down_response = ui.interact(SCROLL_DOWN_ID, down_rect);
        if down_response.hovered() {
            ui.any_interactive_hovered = true;
        }
        if self.has_grf_textures {
            let (v, i) =
                draw::quad_vertices(x, down_y, scrollbar_w, scroll_btn_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::Named(SCROLL_DOWN.to_string()),
            });
        } else {
            let color = if down_response.hovered() {
                [0.5, 0.5, 0.6, 1.0]
            } else {
                [0.3, 0.3, 0.4, 1.0]
            };
            let (v, i) = draw::quad_vertices(x, down_y, scrollbar_w, scroll_btn_h, color);
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
            ui.text(
                x + (3.0),
                down_y + ui.atlas.line_height,
                "\u{25BC}",
                [0.8, 0.8, 0.8, 1.0],
            );
        }

        if up_response.clicked() || down_response.clicked() {
            let delta: isize = if up_response.clicked() { 1 } else { -1 };
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            let new_offset = (state.scroll_offset as isize + delta).clamp(0, max_scroll as isize);
            state.scroll_offset = new_offset as usize;
        }

        if total > max_lines && max_scroll > 0 {
            let track_h = h - scroll_btn_h * 2.0;
            let track_top = y + scroll_btn_h;
            let thumb_ratio = max_lines as f32 / total as f32;
            let thumb_h = (track_h * thumb_ratio).max(10.0);
            let scroll_ratio = scroll_offset as f32 / max_scroll as f32;
            // scroll_offset 0 = at bottom, max = at top, so thumb starts at bottom when offset=0
            let thumb_y = track_top + (track_h - thumb_h) * (1.0 - scroll_ratio);

            let thumb_rect = Rect::new(x, thumb_y, scrollbar_w, thumb_h);
            let t = ui.drag_handle(SCROLL_THUMB_ID, thumb_rect, true);
            if t.started {
                ui.state
                    .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                    .drag_start_scroll = scroll_offset as f32;
            }
            if t.dragging {
                let start = ui
                    .state
                    .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                    .drag_start_scroll;
                let delta_scroll = -(t.delta_y / track_h) * max_scroll as f32;
                let offset = (start + delta_scroll).round().clamp(0.0, max_scroll as f32);
                ui.state
                    .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                    .scroll_offset = offset as usize;
            }

            let thumb_active = t.dragging || t.hovered;
            let thumb_color = if thumb_active {
                [0.6, 0.6, 0.7, 0.9]
            } else {
                [0.5, 0.5, 0.6, 0.8]
            };
            let (v, i) = draw::quad_vertices(
                x + (2.0),
                thumb_y,
                scrollbar_w - (4.0),
                thumb_h,
                thumb_color,
            );
            ui.draw_calls.push(draw::DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: draw::TextureRef::White,
            });
        }
    }

    fn draw_bubble(&self, ui: &mut UiFrame, id: WidgetId, rect: Rect, tint: [f32; 4]) -> bool {
        use ragnarok_ui::draw;
        let resp = ui.interact(id, rect);
        if resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        let color = if resp.hovered() {
            [
                (tint[0] + 0.25).min(1.0),
                (tint[1] + 0.25).min(1.0),
                (tint[2] + 0.25).min(1.0),
                tint[3],
            ]
        } else {
            tint
        };
        let tex = if self.has_grf_textures {
            draw::TextureRef::Named(BUBBLE_TEX.to_string())
        } else {
            draw::TextureRef::White
        };
        let (v, i) = draw::quad_vertices(rect.x, rect.y, rect.w, rect.h, color);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: tex,
        });
        resp.clicked()
    }

    fn draw_channel_menu(&mut self, ui: &mut UiFrame, anchor: Rect) {
        let open = ui
            .state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .channel_menu_open;
        if !open {
            return;
        }
        use ragnarok_ui::draw;
        let item_h = CHANNEL_MENU_ITEM_H;
        let w = SEND_CHANNELS
            .iter()
            .map(|c| ui.atlas.measure_text(c.label()))
            .fold(0.0_f32, f32::max)
            + 8.0;
        let w = w.max(72.0);
        let h = SEND_CHANNELS.len() as f32 * item_h;
        let list = Rect::new(anchor.x + anchor.w - w, anchor.y - h, w, h);
        ui.begin_popup_layer(list);
        let (v, i) = draw::quad_vertices(list.x, list.y, list.w, list.h, [0.1, 0.1, 0.13, 0.97]);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });
        let mut picked = None;
        for (idx, ch) in SEND_CHANNELS.iter().enumerate() {
            let item = Rect::new(list.x, list.y + idx as f32 * item_h, w, item_h);
            let r = ui.interact(WidgetId(CHANNEL_MENU_ITEM_BASE + idx as u32), item);
            if r.hovered() {
                ui.any_interactive_hovered = true;
                let (v, i) =
                    draw::quad_vertices(item.x, item.y, item.w, item.h, [0.28, 0.28, 0.36, 1.0]);
                ui.draw_calls.push(draw::DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: draw::TextureRef::White,
                });
            }
            ui.text(
                item.x + 4.0,
                item.y + MENU_TEXT_BASELINE,
                ch.label(),
                ch.color(),
            );
            if r.clicked() {
                picked = Some(*ch);
            }
        }
        ui.end_popup_layer();

        let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
        if let Some(ch) = picked {
            state.send_channel = ch;
            state.channel_menu_open = false;
            self.whisper_target.text.clear();
            self.whisper_target.cursor_pos = 0;
        } else if ui.ctx.mouse_clicked
            && !anchor.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
            && !list.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
        {
            state.channel_menu_open = false;
        }
    }

    fn draw_whisper_list_button(&mut self, ui: &mut UiFrame, btn_rect: Rect, popup_anchor: Rect) {
        use ragnarok_ui::draw;
        let has_history = !self.whisper_history.is_empty();
        let clicked = ui
            .button(WHISPER_MENU_BTN_ID, btn_rect, &LIST_BTN, "\u{25BC}")
            .clicked();
        if clicked && has_history {
            self.whisper_menu_open = !self.whisper_menu_open;
        }

        if !self.whisper_menu_open || !has_history {
            return;
        }

        let list_h = self.whisper_history.len() as f32 * WHISPER_MENU_ITEM_H;
        let list_w = self
            .whisper_history
            .iter()
            .map(|n| ui.atlas.measure_text(n))
            .fold(0.0_f32, f32::max)
            .max(popup_anchor.w - 6.0)
            + 6.0;
        let list = Rect::new(popup_anchor.x, popup_anchor.y - list_h, list_w, list_h);
        ui.begin_popup_layer(list);
        let (v, i) = draw::quad_vertices(list.x, list.y, list.w, list.h, [0.1, 0.1, 0.13, 0.97]);
        ui.draw_calls.push(draw::DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: draw::TextureRef::White,
        });
        let mut picked = None;
        for (idx, name) in self.whisper_history.iter().enumerate() {
            let item = Rect::new(
                list.x,
                list.y + idx as f32 * WHISPER_MENU_ITEM_H,
                list.w,
                WHISPER_MENU_ITEM_H,
            );
            let r = ui.interact(WidgetId(WHISPER_MENU_ITEM_BASE + idx as u32), item);
            if r.hovered() {
                ui.any_interactive_hovered = true;
                let (v, i) =
                    draw::quad_vertices(item.x, item.y, item.w, item.h, [0.28, 0.28, 0.36, 1.0]);
                ui.draw_calls.push(draw::DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: draw::TextureRef::White,
                });
            }
            ui.text(
                item.x + 3.0,
                item.y + MENU_TEXT_BASELINE,
                name,
                [0.9, 0.9, 0.9, 1.0],
            );
            if r.clicked() {
                picked = Some(idx);
            }
        }
        ui.end_popup_layer();

        if let Some(idx) = picked {
            self.whisper_target.text = self.whisper_history[idx].clone();
            self.whisper_target.cursor_pos = self.whisper_target.text.chars().count();
            self.whisper_menu_open = false;
            self.focused_input = WHISPER_INPUT_ID;
        } else if ui.ctx.mouse_clicked
            && !btn_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
            && !list.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
        {
            self.whisper_menu_open = false;
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

impl Window for ChatWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            DIALOG_BG,
            SCROLL_UP,
            SCROLL_DOWN,
            LIST_BTN.normal,
            LIST_BTN.hover,
            LIST_BTN.pressed,
            BUBBLE_TEX,
        ]
    }
}

impl InGameWindow for ChatWindow {
    fn build(&mut self, ui: &mut UiFrame, ctx: &mut BuildCtx) -> Vec<GameEvent> {
        let _character = &mut *ctx.character;
        let _data = ctx.data;
        let mut events = Vec::new();
        let screen_h = ui.ctx.screen_height;
        let input_h = INPUT_H;
        let padding = PADDING;
        let scrollbar_w = SCROLLBAR_W;
        let max_msg_h = (screen_h - input_h - (50.0)).max(MIN_MSG_AREA_H);

        let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
        if !state.initialized {
            let idx = self.initial_size_index.take().unwrap_or(DEFAULT_SIZE_INDEX);
            state.size_index = idx;
            state.msg_area_h = SIZE_CYCLE[idx];
            state.chat_w = DEFAULT_CHAT_W;
            state.pos_x = padding;
            let default_h = SIZE_CYCLE[idx] + input_h;
            state.pos_y = screen_h - default_h - padding;
            state.initialized = true;
        }

        if ui.ctx.key_f10 {
            let next = state.size_index + 1;
            state.size_index = if next >= SIZE_CYCLE.len() { 0 } else { next };
            state.msg_area_h = SIZE_CYCLE[state.size_index];
        }

        let size_index = state.size_index;
        let mut msg_area_h = state.msg_area_h;
        let mut chat_w = state.chat_w;

        if size_index == 0 {
            self.bounding_rect = None;
            return events;
        }

        let show_messages = size_index >= 2 && msg_area_h > 0.0;
        let chat_x = ui
            .state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .pos_x;
        let chat_y = ui
            .state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .pos_y;

        if show_messages {
            let handle_center_y = chat_y + msg_area_h;
            let handle_rect = Rect::new(
                chat_x,
                handle_center_y - DRAG_HIT_AREA / 2.0,
                chat_w,
                DRAG_HIT_AREA,
            );
            let h = ui.drag_handle(HEIGHT_DRAG_ID, handle_rect, true);
            if h.started {
                ui.state
                    .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                    .drag_start_msg_h = msg_area_h;
            }
            if h.dragging {
                let start = ui
                    .state
                    .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                    .drag_start_msg_h;
                msg_area_h = (start - h.delta_y).clamp(MIN_MSG_AREA_H, max_msg_h);
            }
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            if h.dragging || msg_area_h != state.msg_area_h {
                state.msg_area_h = msg_area_h;
            }
        }

        let total_h = if show_messages {
            msg_area_h + input_h
        } else {
            input_h
        };

        {
            let right_edge_x = chat_x + chat_w;
            let handle_rect = Rect::new(
                right_edge_x - DRAG_HIT_AREA / 2.0,
                chat_y,
                DRAG_HIT_AREA,
                total_h,
            );
            let w = ui.drag_handle(WIDTH_DRAG_ID, handle_rect, true);
            if w.started {
                ui.state
                    .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                    .drag_start_chat_w = chat_w;
            }
            if w.dragging {
                let start = ui
                    .state
                    .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                    .drag_start_chat_w;
                chat_w = (start + w.delta_x).clamp(MIN_CHAT_W, MAX_CHAT_W);
            }
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            if w.dragging || chat_w != state.chat_w {
                state.chat_w = chat_w;
            }
        }
        chat_w = chat_w.min(ui.ctx.screen_width.max(MIN_CHAT_W));
        msg_area_h = msg_area_h.min(max_msg_h);
        {
            let fitted_h = if size_index >= 2 && msg_area_h > 0.0 {
                msg_area_h + input_h
            } else {
                input_h
            };
            let (screen_w, screen_h) = (ui.ctx.screen_width, ui.ctx.screen_height);
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            state.chat_w = chat_w;
            state.msg_area_h = msg_area_h;
            state.pos_x = state.pos_x.clamp(0.0, (screen_w - chat_w).max(0.0));
            state.pos_y = state.pos_y.clamp(0.0, (screen_h - fitted_h).max(0.0));
        }

        let tex_scale = chat_w / TEX_NATIVE_W;

        let show_messages = size_index >= 2 && msg_area_h > 0.0;
        let total_h = if show_messages {
            msg_area_h + input_h
        } else {
            input_h
        };
        let chat_x = ui
            .state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .pos_x;
        let chat_y = ui
            .state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .pos_y;

        self.bounding_rect = Some(Rect::new(chat_x, chat_y, chat_w, total_h));

        ui.ensure_in_z_order(CHAT_WINDOW_ID);
        ui.enter_window(CHAT_WINDOW_ID, self.bounding_rect.unwrap());
        if !ui.is_current_window_occluded()
            && ui.ctx.mouse_clicked
            && self
                .bounding_rect
                .unwrap()
                .contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
        {
            ui.bring_to_front(CHAT_WINDOW_ID);
        }

        if ui.enter_pressed() && !self.active {
            self.active = true;
            let forced = if ui.ctx.ctrl_pressed {
                Some(SendChannel::Party)
            } else if ui.ctx.alt_pressed {
                Some(SendChannel::Guild)
            } else {
                None
            };
            if let Some(channel) = forced {
                ui.state
                    .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                    .send_channel = channel;
                self.whisper_target.text.clear();
                self.whisper_target.cursor_pos = 0;
            }
            ui.set_focus(INPUT_ID);
            return events;
        }

        if self.active {
            if ui.enter_pressed() {
                if !self.input.text.trim().is_empty() {
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
                    let sticky = ui
                        .state
                        .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                        .send_channel;
                    let whisper = self.whisper_target.text.trim().to_string();
                    let channel = effective_channel(
                        &message,
                        sticky,
                        ui.ctx.ctrl_pressed,
                        ui.ctx.alt_pressed,
                        !whisper.is_empty(),
                    );
                    if matches!(channel, SendChannel::Party | SendChannel::Guild) {
                        ui.state
                            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                            .send_channel = channel;
                        self.whisper_target.text.clear();
                        self.whisper_target.cursor_pos = 0;
                    }
                    if channel == SendChannel::Whisper {
                        self.remember_whisper(whisper.clone());
                        events.push(GameEvent::RequestSendWhisper {
                            name: whisper,
                            message,
                        });
                    } else {
                        events.push(GameEvent::RequestSendChat {
                            message: route_message(message, channel),
                        });
                    }
                }
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

        if show_messages {
            let text_area_w = chat_w - padding * 2.0;
            let atlas = ui.atlas;
            let visual_lines: Vec<(String, [f32; 4])> = self
                .messages
                .iter()
                .flat_map(|line| {
                    let wrapped = ragnarok_ui::draw::word_wrap(
                        &line.text,
                        text_area_w,
                        |t| atlas.measure_text(t),
                        false,
                    );
                    let color = line.color;
                    wrapped.into_iter().map(move |w| (w, color))
                })
                .collect();
            let total_visual = visual_lines.len();

            let msg_rect = Rect::new(chat_x, chat_y, chat_w, msg_area_h);
            let hovered = msg_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y);
            if hovered {
                ui.any_hovered = true;
            }
            if hovered && ui.ctx.scroll_delta != 0.0 {
                let max_lines = visible_line_count(msg_area_h);
                let max_scroll = total_visual.saturating_sub(max_lines);
                let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
                let delta = ui.ctx.scroll_delta.round() as isize;
                let new_offset =
                    (state.scroll_offset as isize + delta).clamp(0, max_scroll as isize);
                state.scroll_offset = new_offset as usize;
            }

            let scroll_offset = ui
                .state
                .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
                .scroll_offset;
            self.draw_visual_lines(
                ui,
                chat_x,
                chat_y,
                chat_w,
                msg_area_h,
                scroll_offset,
                &visual_lines,
            );
            self.draw_scrollbar_filtered(
                ui,
                chat_x + chat_w - scrollbar_w,
                chat_y,
                msg_area_h,
                scroll_offset,
                total_visual,
            );
            self.draw_height_handle(ui, chat_x, chat_y + msg_area_h, chat_w);
        }

        let input_y = chat_y + if show_messages { msg_area_h } else { 0.0 };
        let bubble_y = input_y + (input_h - BUBBLE_SIZE) / 2.0;
        let height_bubble = Rect::new(
            chat_x + chat_w - padding * tex_scale - BUBBLE_SIZE,
            bubble_y,
            BUBBLE_SIZE,
            BUBBLE_SIZE,
        );
        let channel_bubble = Rect::new(
            height_bubble.x - BUBBLE_GAP - BUBBLE_SIZE,
            bubble_y,
            BUBBLE_SIZE,
            BUBBLE_SIZE,
        );
        let whisper_rect = Rect::new(
            chat_x + WHISPER_NATIVE_X * tex_scale,
            input_y,
            WHISPER_NATIVE_W * tex_scale,
            input_h,
        );
        let list_rect = Rect::new(
            chat_x + LIST_NATIVE_X * tex_scale,
            input_y,
            LIST_BTN_W * tex_scale,
            input_h,
        );
        let msg_x = chat_x + MSG_NATIVE_X * tex_scale;
        let msg_right = (chat_x + (TEX_NATIVE_W - MSG_NATIVE_RIGHT) * tex_scale)
            .min(channel_bubble.x - INPUT_GAP);
        let msg_rect = Rect::new(msg_x, input_y, (msg_right - msg_x).max(20.0), input_h);

        if self.active {
            let input_bg = if self.has_grf_textures {
                let (v, i) =
                    ragnarok_ui::draw::quad_vertices(chat_x, input_y, chat_w, input_h, [1.0; 4]);
                ui.draw_calls.push(ragnarok_ui::draw::DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: ragnarok_ui::draw::TextureRef::Named(DIALOG_BG.to_string()),
                });
                TextInputBg::Transparent
            } else {
                let (v, i) = ragnarok_ui::draw::quad_vertices(
                    chat_x,
                    input_y,
                    chat_w,
                    input_h,
                    [0.0, 0.0, 0.0, 0.6],
                );
                ui.draw_calls.push(ragnarok_ui::draw::DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: ragnarok_ui::draw::TextureRef::White,
                });
                TextInputBg::Gray
            };

            if self.pending_focus {
                self.pending_focus = false;
                self.focused_input = INPUT_ID;
                ui.set_focus(INPUT_ID);
            }
            if ui.ctx.key_tab {
                self.focused_input = if self.focused_input == INPUT_ID {
                    WHISPER_INPUT_ID
                } else {
                    INPUT_ID
                };
                ui.set_focus(self.focused_input);
            }
            if ui.ctx.mouse_clicked {
                if whisper_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
                    self.focused_input = WHISPER_INPUT_ID;
                } else if msg_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) {
                    self.focused_input = INPUT_ID;
                }
            }

            ui.text_input(
                WHISPER_INPUT_ID,
                whisper_rect,
                &mut self.whisper_target,
                input_bg,
            );
            let popup_anchor = Rect::new(
                whisper_rect.x,
                input_y,
                list_rect.x + list_rect.w - whisper_rect.x,
                input_h,
            );
            self.draw_whisper_list_button(ui, list_rect, popup_anchor);
            ui.text_input(INPUT_ID, msg_rect, &mut self.input, input_bg);
        } else {
            self.whisper_menu_open = false;
        }

        let sticky = ui
            .state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .send_channel;
        if self.draw_bubble(ui, CHANNEL_BTN_ID, channel_bubble, sticky.color()) {
            let st = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            st.channel_menu_open = !st.channel_menu_open;
        }
        if self.draw_bubble(ui, SIZE_BTN_ID, height_bubble, [0.55, 0.55, 0.6, 1.0]) {
            let st = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            let next = st.size_index + 1;
            st.size_index = if next >= SIZE_CYCLE.len() { 1 } else { next };
            st.msg_area_h = SIZE_CYCLE[st.size_index];
        }
        self.draw_channel_menu(ui, channel_bubble);

        {
            let input_row = Rect::new(chat_x, input_y, chat_w, input_h);
            let on_widget = channel_bubble.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
                || height_bubble.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
                || (self.active
                    && (whisper_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
                        || list_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
                        || msg_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)));
            let in_input = input_row.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) && !on_widget;
            let in_msg_area = show_messages && {
                let drag_area = Rect::new(
                    chat_x,
                    chat_y,
                    chat_w - scrollbar_w,
                    (msg_area_h - DRAG_HIT_AREA / 2.0).max(0.0),
                );
                drag_area.contains(ui.ctx.mouse_x, ui.ctx.mouse_y)
            };
            let state = ui.state.get_or_default::<ChatWindowState>(CHAT_WINDOW_ID);
            if (in_input || in_msg_area) && ui.ctx.mouse_clicked && !state.dragging {
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

        self.draw_width_handle(ui, chat_x + chat_w, chat_y, total_h);

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InGameWindow;
    use ragnarok_game::character::Character;
    use ragnarok_game::data_table::DataTable;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame<'a>(
        ctx: &'a UiContext,
        atlas: &'a FontAtlas,
        state: &'a mut StateCache,
    ) -> UiFrame<'a> {
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
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
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        assert_eq!(ws.size_index, DEFAULT_SIZE_INDEX);

        for expected_index in [6, 0, 1, 2, 3, 4, 5] {
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_f10 = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
            let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
            assert_eq!(ws.size_index, expected_index);
        }
    }

    #[test]
    fn shrinking_screen_refits_window_into_view() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        assert!(ws.pos_y > 300.0, "default layout sits near the bottom");

        let ctx = UiContext::new(480.0, 300.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        assert_eq!(ws.pos_x, 0.0);
        assert_eq!(ws.pos_y + ws.msg_area_h + INPUT_H, 300.0);
    }

    #[test]
    fn hidden_mode_produces_no_draw_calls() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        for _ in 0..2 {
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_f10 = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        }

        let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        assert_eq!(ws.size_index, 0);

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(ui.draw_calls.is_empty());
        assert!(chat.bounding_rect.is_none());
    }

    #[test]
    fn collapsed_mode_shows_input_only_when_active() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        for _ in 0..3 {
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_f10 = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        }

        let ws = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        assert_eq!(ws.size_index, 1);

        chat.active = true;
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(!ui.draw_calls.is_empty());
        assert!(chat.bounding_rect.is_some());
        let rect = chat.bounding_rect.unwrap();
        assert_eq!(rect.h, INPUT_H);
    }

    #[test]
    fn contains_point_checks_bounding_rect() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.active = true;
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let rect = chat.bounding_rect.unwrap();
        assert!(chat.contains_point(rect.x + 1.0, rect.y + 1.0));
        assert!(!chat.contains_point(0.0, 0.0));
    }

    #[test]
    fn height_drag_changes_msg_area_h() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        let initial_h = state
            .get::<ChatWindowState>(CHAT_WINDOW_ID)
            .unwrap()
            .msg_area_h;

        let handle_y = 600.0 - INPUT_H - PADDING;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 100.0;
        ctx.mouse_y = handle_y;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 100.0;
        ctx.mouse_y = handle_y - 50.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let new_h = state
            .get::<ChatWindowState>(CHAT_WINDOW_ID)
            .unwrap()
            .msg_area_h;
        assert!(
            new_h > initial_h,
            "Height should increase when dragging up: {} > {}",
            new_h,
            initial_h
        );
    }

    #[test]
    fn width_drag_changes_chat_w() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        let initial_w = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().chat_w;

        let rect = chat.bounding_rect.unwrap();
        let edge_x = rect.x + rect.w;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x + 80.0;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let new_w = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().chat_w;
        assert!(
            new_w > initial_w,
            "Width should increase when dragging right: {} > {}",
            new_w,
            initial_w
        );
    }

    #[test]
    fn visible_line_count_never_overflows_area() {
        for &h in &SIZE_CYCLE[2..] {
            let n = visible_line_count(h);
            assert!(
                PADDING + n as f32 * LINE_H <= h,
                "{n} lines overflow area of height {h}",
            );
            assert!(
                PADDING + (n + 1) as f32 * LINE_H > h,
                "area of height {h} could fit more than {n} lines",
            );
        }
    }

    #[test]
    fn message_area_drag_moves_window() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        let st = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        let (start_x, start_y) = (st.pos_x, st.pos_y);
        let grab_x = start_x + 40.0;
        let grab_y = start_y + 20.0;

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = grab_x;
        ctx.mouse_y = grab_y;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = grab_x + 60.0;
        ctx.mouse_y = grab_y - 30.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let st = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        assert_eq!(st.pos_x, start_x + 60.0);
        assert_eq!(st.pos_y, start_y - 30.0);
    }

    #[test]
    fn drag_respects_min_max_constraints() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let rect = chat.bounding_rect.unwrap();
        let edge_x = rect.x + rect.w;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_clicked = true;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = edge_x + 1000.0;
        ctx.mouse_y = rect.y + rect.h / 2.0;
        ctx.mouse_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let w = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap().chat_w;
        assert!(
            w <= MAX_CHAT_W,
            "Width should be clamped to max: {} <= {}",
            w,
            MAX_CHAT_W
        );
    }

    #[test]
    fn scroll_offset_clamps_to_valid_range() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        for i in 0..50 {
            chat.add_chat(format!("msg {i}"));
        }

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .scroll_offset = 500;

        let rect = chat.bounding_rect.unwrap();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = rect.x + 10.0;
        ctx.mouse_y = rect.y + 10.0;
        ctx.scroll_delta = 1.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let offset = state
            .get::<ChatWindowState>(CHAT_WINDOW_ID)
            .unwrap()
            .scroll_offset;
        let max_lines = visible_line_count(SIZE_CYCLE[DEFAULT_SIZE_INDEX]);
        let max_scroll = 50_usize.saturating_sub(max_lines);
        assert!(
            offset <= max_scroll,
            "Scroll offset should be clamped: {} <= {}",
            offset,
            max_scroll
        );
    }

    #[test]
    fn mouse_wheel_scrolls_when_over_chat() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        chat.active = true;
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        for i in 0..50 {
            chat.add_chat(format!("msg {i}"));
        }

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        assert_eq!(
            state
                .get::<ChatWindowState>(CHAT_WINDOW_ID)
                .unwrap()
                .scroll_offset,
            0
        );

        let rect = chat.bounding_rect.unwrap();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = rect.x + 10.0;
        ctx.mouse_y = rect.y + 10.0;
        ctx.scroll_delta = 3.0;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let offset = state
            .get::<ChatWindowState>(CHAT_WINDOW_ID)
            .unwrap()
            .scroll_offset;
        assert_eq!(offset, 3, "Scroll offset should increase by 3");
    }

    fn bubble_centers(state: &mut StateCache) -> (f32, f32, f32) {
        let st = state.get::<ChatWindowState>(CHAT_WINDOW_ID).unwrap();
        let input_y = st.pos_y + st.msg_area_h;
        let bubble_cy = input_y + (INPUT_H - BUBBLE_SIZE) / 2.0 + BUBBLE_SIZE / 2.0;
        let height_cx = st.pos_x + st.chat_w - PADDING - BUBBLE_SIZE / 2.0;
        let channel_cx = height_cx - BUBBLE_SIZE - BUBBLE_GAP;
        (channel_cx, height_cx, bubble_cy)
    }

    #[test]
    fn channel_bubble_sets_send_channel() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        chat.whisper_target.text = "Bob".to_string();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));

        let (channel_cx, _, bubble_cy) = bubble_centers(&mut state);
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = channel_cx;
        ctx.mouse_y = bubble_cy;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(
            state
                .get::<ChatWindowState>(CHAT_WINDOW_ID)
                .unwrap()
                .channel_menu_open
        );

        let party_idx = SEND_CHANNELS
            .iter()
            .position(|c| *c == SendChannel::Party)
            .unwrap();
        let item_y =
            bubble_cy - BUBBLE_SIZE / 2.0 - SEND_CHANNELS.len() as f32 * CHANNEL_MENU_ITEM_H
                + (party_idx as f32 + 0.5) * CHANNEL_MENU_ITEM_H;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = channel_cx - 20.0;
        ctx.mouse_y = item_y;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(
            state
                .get::<ChatWindowState>(CHAT_WINDOW_ID)
                .unwrap()
                .send_channel,
            SendChannel::Party
        );
        assert!(
            chat.whisper_target.text.is_empty(),
            "changing channel clears whisper"
        );
    }

    #[test]
    fn height_bubble_cycles_size() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        let before = state
            .get::<ChatWindowState>(CHAT_WINDOW_ID)
            .unwrap()
            .size_index;

        let (_, height_cx, bubble_cy) = bubble_centers(&mut state);
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = height_cx;
        ctx.mouse_y = bubble_cy;
        ctx.mouse_clicked = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        let after = state
            .get::<ChatWindowState>(CHAT_WINDOW_ID)
            .unwrap()
            .size_index;
        assert_ne!(after, before, "height bubble should cycle size_index");
    }

    #[test]
    fn blank_input_sends_nothing() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        for blank in ["", "   ", "\t "] {
            chat.active = true;
            chat.input.text = blank.to_string();
            chat.input.cursor_pos = chat.input.text.chars().count();
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_enter = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            let events = chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
            assert!(
                !events.iter().any(|e| matches!(
                    e,
                    GameEvent::RequestSendChat { .. } | GameEvent::RequestSendWhisper { .. }
                )),
                "blank {blank:?} must not send"
            );
        }
    }

    #[test]
    fn opening_chat_with_modifier_sets_channel() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();
        chat.whisper_target.text = "Bob".to_string();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        ctx.ctrl_pressed = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(chat.is_active());
        assert_eq!(
            state
                .get::<ChatWindowState>(CHAT_WINDOW_ID)
                .unwrap()
                .send_channel,
            SendChannel::Party
        );
        assert!(chat.whisper_target.text.is_empty());

        chat.active = false;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        ctx.alt_pressed = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(
            state
                .get::<ChatWindowState>(CHAT_WINDOW_ID)
                .unwrap()
                .send_channel,
            SendChannel::Guild
        );
    }

    #[test]
    fn enter_routes_by_channel_and_override() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        chat.active = true;
        chat.whisper_target.text = "Bob".to_string();
        chat.input.text = "psst".to_string();
        state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .send_channel = SendChannel::Public;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let events = chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestSendWhisper { name, message } if name == "Bob" && message == "psst"
        )));

        chat.active = true;
        chat.whisper_target.text = "Bob".to_string();
        chat.input.text = "%to party".to_string();
        state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .send_channel = SendChannel::Public;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let events = chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestSendChat { message } if message == "%to party"
        )));
        assert_eq!(
            state
                .get::<ChatWindowState>(CHAT_WINDOW_ID)
                .unwrap()
                .send_channel,
            SendChannel::Party,
            "override switches the sticky channel"
        );
        assert!(
            chat.whisper_target.text.is_empty(),
            "override clears whisper"
        );

        chat.active = true;
        chat.whisper_target.text.clear();
        chat.input.text = "team up".to_string();
        state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .send_channel = SendChannel::Party;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let events = chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestSendChat { message } if message == "%team up"
        )));

        chat.active = true;
        chat.input.text = "hi".to_string();
        state
            .get_or_default::<ChatWindowState>(CHAT_WINDOW_ID)
            .send_channel = SendChannel::Public;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        ctx.ctrl_pressed = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        let events = chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(events.iter().any(|e| matches!(
            e,
            GameEvent::RequestSendChat { message } if message == "%hi"
        )));
        assert_eq!(
            state
                .get::<ChatWindowState>(CHAT_WINDOW_ID)
                .unwrap()
                .send_channel,
            SendChannel::Party,
            "ctrl+enter switches the sticky channel to party"
        );
    }

    #[test]
    fn effective_channel_overrides_sticky() {
        assert_eq!(
            effective_channel("hi", SendChannel::Public, false, false, false),
            SendChannel::Public
        );
        assert_eq!(
            effective_channel("hi", SendChannel::Guild, false, false, false),
            SendChannel::Guild
        );
        assert_eq!(
            effective_channel("%hi", SendChannel::Public, false, false, false),
            SendChannel::Party
        );
        assert_eq!(
            effective_channel("$hi", SendChannel::Public, false, false, false),
            SendChannel::Guild
        );
        assert_eq!(
            effective_channel("hi", SendChannel::Public, true, false, false),
            SendChannel::Party
        );
        assert_eq!(
            effective_channel("hi", SendChannel::Public, false, true, false),
            SendChannel::Guild
        );
        // filled whisper target overrides the sticky channel...
        assert_eq!(
            effective_channel("hi", SendChannel::Public, false, false, true),
            SendChannel::Whisper
        );
        // ...but explicit prefix/modifier still wins over whisper
        assert_eq!(
            effective_channel("%hi", SendChannel::Public, false, false, true),
            SendChannel::Party
        );
        assert_eq!(
            effective_channel("$hi", SendChannel::Public, false, false, true),
            SendChannel::Guild
        );
        assert_eq!(
            effective_channel("hi", SendChannel::Public, true, false, true),
            SendChannel::Party
        );
        assert_eq!(
            effective_channel("hi", SendChannel::Public, false, true, true),
            SendChannel::Guild
        );
        // a client command is never chat, whatever the target or sticky channel
        assert_eq!(
            effective_channel("/sit", SendChannel::Whisper, false, false, true),
            SendChannel::Public
        );
        assert_eq!(
            effective_channel("/sit", SendChannel::Party, false, false, false),
            SendChannel::Public
        );
        assert_eq!(
            effective_channel("/gc hello", SendChannel::Public, false, false, true),
            SendChannel::Guild
        );
    }

    #[test]
    fn whisper_history_dedups_caps_and_orders_recent_first() {
        let mut chat = ChatWindow::new();
        chat.remember_whisper("".to_string());
        assert!(chat.whisper_history.is_empty());

        for name in ["Alice", "Bob", "Alice", "Carol"] {
            chat.remember_whisper(name.to_string());
        }
        assert_eq!(chat.whisper_history, vec!["Carol", "Alice", "Bob"]);

        for i in 0..MAX_WHISPER_HISTORY + 3 {
            chat.remember_whisper(format!("P{i}"));
        }
        assert_eq!(chat.whisper_history.len(), MAX_WHISPER_HISTORY);
        assert_eq!(
            chat.whisper_history[0],
            format!("P{}", MAX_WHISPER_HISTORY + 2)
        );
    }

    #[test]
    fn route_message_prefixes_party_and_guild_only() {
        assert_eq!(route_message("hi".to_string(), SendChannel::Public), "hi");
        assert_eq!(route_message("hi".to_string(), SendChannel::Party), "%hi");
        assert_eq!(route_message("hi".to_string(), SendChannel::Guild), "$hi");
        assert_eq!(
            route_message("/sit".to_string(), SendChannel::Party),
            "/sit"
        );
        assert_eq!(route_message("%x".to_string(), SendChannel::Guild), "%x");
    }

    #[test]
    fn chat_input_history_navigation() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(chat.active);

        let messages = ["hello", "world", "test"];
        for msg in &messages {
            chat.active = true;
            chat.input.text = msg.to_string();
            chat.input.cursor_pos = msg.len();
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_enter = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        }
        assert_eq!(chat.sent_history, vec!["hello", "world", "test"]);

        chat.active = true;

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_up = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(chat.input.text, "test");
        assert_eq!(chat.history_index, Some(0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_up = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(chat.input.text, "world");
        assert_eq!(chat.history_index, Some(1));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(chat.input.text, "test");
        assert_eq!(chat.history_index, Some(0));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(chat.input.text, "");
        assert!(chat.history_index.is_none());

        chat.active = true;
        chat.input.text = "test".to_string();
        chat.input.cursor_pos = 4;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(chat.sent_history, vec!["hello", "world", "test"]);

        chat.sent_history.clear();
        for i in 0..MAX_HISTORY + 5 {
            chat.active = true;
            chat.input.text = format!("msg{i}");
            chat.input.cursor_pos = chat.input.text.len();
            let mut ctx = UiContext::new(800.0, 600.0);
            ctx.key_enter = true;
            let mut ui = make_frame(&ctx, &atlas, &mut state);
            chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        }
        assert_eq!(chat.sent_history.len(), MAX_HISTORY);
        assert_eq!(chat.sent_history[0], "msg5");

        chat.active = true;
        chat.input.text = "draft text".to_string();
        chat.input.cursor_pos = 10;
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_up = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(chat.draft, "draft text");
        assert_ne!(chat.input.text, "draft text");

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_down = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(chat.input.text, "draft text");
    }

    #[test]
    fn tab_switches_between_inputs() {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let mut chat = ChatWindow::new();
        let mut character = Character::new();
        let data = DataTable::new();
        let mut state = StateCache::new();

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert!(chat.active);

        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &atlas, &mut state);
        ui.set_focus(INPUT_ID);
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(ui.focused(), Some(INPUT_ID));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_tab = true;
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        let mut ui = UiFrame::new(
            &ctx,
            &atlas,
            &mut state,
            0.0,
            false,
            Some(INPUT_ID),
            positions,
        );
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(ui.focused(), Some(WHISPER_INPUT_ID));

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_tab = true;
        let mut ui = UiFrame::new(
            &ctx,
            &atlas,
            &mut state,
            0.0,
            false,
            Some(WHISPER_INPUT_ID),
            positions,
        );
        chat.build(&mut ui, &mut crate::BuildCtx::test(&mut character, &data));
        assert_eq!(ui.focused(), Some(INPUT_ID));
    }
}
