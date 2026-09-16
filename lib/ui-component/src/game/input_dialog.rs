use crate::Window;
use crate::helper::dialog_container::DialogContainer;
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const OK_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_OK,
    hover: ragnarok_resources::ui::BTN_OK_A,
    pressed: ragnarok_resources::ui::BTN_OK_B,
};
pub const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::BTN_CANCEL,
    hover: ragnarok_resources::ui::BTN_CANCEL_A,
    pressed: ragnarok_resources::ui::BTN_CANCEL_B,
};

const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;
const DIALOG_W: f32 = 220.0;
const DIALOG_H: f32 = 55.0;
const PADDING: f32 = 4.0;
const PADDING_X: f32 = 12.0;
const BTN_SPACING: f32 = 3.0;

const COUNT_W: f32 = 182.0;
const COUNT_H: f32 = 46.0;
const COUNT_MARGIN: f32 = 15.0;
const COUNT_NAME_Y: f32 = 6.0;
const COUNT_INPUT_Y: f32 = 22.0;
const COUNT_INPUT_W: f32 = 80.0;
const COUNT_INPUT_H: f32 = 16.0;
const COUNT_BTN_RIGHT: f32 = 55.0;
const COUNT_BTN_BOTTOM: f32 = 32.0;
/// The frame border; everything else starts below it, so a drag never lands on
/// the input or the button.
const COUNT_DRAG_H: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDialogResult {
    None,
    Submitted,
    Cancel,
}

pub enum InputDialogLayout {
    /// The quantity prompt every item move goes through: a fixed frame with the
    /// item name above the field and an OK button as the only way out.
    ItemCount { item_name: String },
    Text {
        label: Option<String>,
        show_cancel: bool,
    },
}

pub struct InputDialogConfig {
    pub layout: InputDialogLayout,
    pub escape_cancels: bool,
    pub default_value: String,
    pub max_len: usize,
    pub numeric_only: bool,
    pub max_value: Option<i32>,
}

pub struct InputDialog {
    pub has_grf_textures: bool,
    input: TextInput,
    btn_size: (f32, f32),
    is_item_count: bool,
    show_cancel: bool,
    escape_cancels: bool,
    label: Option<String>,
    max_value: Option<i32>,
    base_id: WidgetId,
    container: DialogContainer,
}

const OFFSET_INPUT: u32 = 0;
const OFFSET_OK: u32 = 1;
const OFFSET_CANCEL: u32 = 2;
const OFFSET_WINDOW: u32 = 3;

impl InputDialog {
    pub fn new(config: InputDialogConfig, base_id: WidgetId) -> Self {
        let mut input =
            TextInput::new(config.max_len, false).with_numeric_only(config.numeric_only);
        input.text = config.default_value;
        input.select_all();
        let (is_item_count, label, show_cancel) = match config.layout {
            InputDialogLayout::ItemCount { item_name } => (true, Some(item_name), false),
            InputDialogLayout::Text { label, show_cancel } => (false, label, show_cancel),
        };
        Self {
            has_grf_textures: false,
            input,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            is_item_count,
            show_cancel,
            escape_cancels: config.escape_cancels,
            label,
            max_value: config.max_value,
            base_id,
            container: DialogContainer::new(),
        }
    }

    pub fn init_container(&mut self, source: &DialogContainer) {
        self.has_grf_textures = source.has_grf_textures;
        self.container.copy_sizes_from(source);
    }

    pub fn value_str(&self) -> &str {
        &self.input.text
    }

    pub fn set_input_text(&mut self, text: &str) {
        self.input.text = text.to_string();
        self.input.select_all();
    }

    pub fn clear_input(&mut self) {
        self.input.text.clear();
        self.input.cursor_pos = 0;
        self.input.clear_selection();
    }

    pub fn value_i16(&self) -> Option<i16> {
        self.input.text.parse().ok()
    }

    pub fn value_i32(&self) -> Option<i32> {
        self.input.text.parse().ok()
    }

    pub fn win_id(&self) -> WidgetId {
        WidgetId(self.base_id.0 + OFFSET_WINDOW)
    }
    pub fn window_size(&self) -> (f32, f32) {
        if self.is_item_count {
            return (COUNT_W, COUNT_H);
        }
        let label_h = if self.label.is_some() { 18.0 } else { 0.0 };
        (DIALOG_W, DIALOG_H + label_h)
    }
    fn input_id(&self) -> WidgetId {
        WidgetId(self.base_id.0 + OFFSET_INPUT)
    }
    fn ok_id(&self) -> WidgetId {
        WidgetId(self.base_id.0 + OFFSET_OK)
    }
    fn cancel_id(&self) -> WidgetId {
        WidgetId(self.base_id.0 + OFFSET_CANCEL)
    }

    fn clamp_to_max(&mut self) {
        let Some(max) = self.max_value else {
            return;
        };
        if self
            .input
            .text
            .parse::<i32>()
            .is_ok_and(|value| value > max)
        {
            self.set_input_text(&max.to_string());
        }
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> InputDialogResult {
        if self.escape_cancels && ui.take_escape() {
            return InputDialogResult::Cancel;
        }

        let label_h = if self.label.is_some() {
            ui.atlas.line_height + PADDING
        } else {
            0.0
        };
        let (dw, dh) = if self.is_item_count {
            (COUNT_W, COUNT_H)
        } else {
            (DIALOG_W, DIALOG_H + label_h)
        };
        let title_bar_h = if self.is_item_count {
            COUNT_DRAG_H
        } else {
            PADDING * 2.0 + ui.atlas.line_height
        };
        let win = ui.window(self.win_id(), dw, dh, title_bar_h);
        ui.interact(self.win_id(), win);
        let dx = win.x;
        let dy = win.y;

        self.container.has_grf_textures = self.has_grf_textures;
        self.container
            .draw(&mut ui.draw_calls, dx, dy, dw, dh, [1.0, 1.0, 1.0, 1.0]);

        let text_color = self.container.text_color();
        let (btn_w, btn_h) = self.btn_size;

        let (input_rect, ok_rect, cancel_rect) = if self.is_item_count {
            if let Some(name) = &self.label {
                ui.text(
                    dx + COUNT_MARGIN,
                    dy + COUNT_NAME_Y + ui.atlas.line_height,
                    name,
                    text_color,
                );
            }
            (
                Rect::new(
                    dx + COUNT_MARGIN,
                    dy + COUNT_INPUT_Y,
                    COUNT_INPUT_W,
                    COUNT_INPUT_H,
                ),
                Rect::new(
                    dx + dw - COUNT_BTN_RIGHT,
                    dy + dh - COUNT_BTN_BOTTOM,
                    btn_w,
                    btn_h,
                ),
                None,
            )
        } else {
            let mut content_y = dy + PADDING + ui.atlas.line_height;
            if let Some(label) = &self.label {
                ui.text(dx + PADDING_X, content_y, label, text_color);
                content_y += label_h;
            }
            let cancel_space = if self.show_cancel {
                btn_w + BTN_SPACING
            } else {
                0.0
            };
            let input_w = dw - PADDING_X * 2.0 - btn_w - cancel_space - BTN_SPACING * 2.0;
            let btn_x = PADDING_X + dx + input_w + BTN_SPACING * 2.0;
            (
                Rect::new(dx + PADDING_X, content_y, input_w, 16.0),
                Rect::new(btn_x, content_y - 2.0, btn_w, btn_h),
                self.show_cancel
                    .then(|| Rect::new(btn_x + btn_w + BTN_SPACING, content_y - 2.0, btn_w, btn_h)),
            )
        };

        let input_bg = if self.has_grf_textures {
            TextInputBg::Gray
        } else {
            TextInputBg::Default
        };

        let input_id = self.input_id();
        let ok_id = self.ok_id();

        if ui.focused() != Some(input_id) {
            ui.set_focus(input_id);
        }

        ui.text_input(input_id, input_rect, &mut self.input, input_bg);
        let ok = ui.button(ok_id, ok_rect, &OK_BTN, "OK");

        if let Some(cancel_rect) = cancel_rect {
            let cancel = ui.button(self.cancel_id(), cancel_rect, &CANCEL_BTN, "Cancel");
            if cancel.clicked() {
                return InputDialogResult::Cancel;
            }
        }

        if ok.clicked() || ui.ctx.key_enter {
            self.clamp_to_max();
            return InputDialogResult::Submitted;
        }

        InputDialogResult::None
    }
}

impl Window for InputDialog {
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
        self.container.set_texture_sizes(size_fn);
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = DialogContainer::grf_texture_paths();
        paths.extend_from_slice(&[
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ]);
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    fn make_dialog(default_value: &str, show_cancel: bool) -> InputDialog {
        InputDialog::new(
            InputDialogConfig {
                layout: InputDialogLayout::Text {
                    label: Some("How many?".to_string()),
                    show_cancel,
                },
                escape_cancels: true,
                default_value: default_value.to_string(),
                max_len: 6,
                numeric_only: true,
                max_value: Some(50),
            },
            WidgetId(900),
        )
    }

    #[test]
    fn enter_key_submits() {
        let mut dialog = make_dialog("5", true);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), InputDialogResult::Submitted);
        assert_eq!(dialog.value_str(), "5");
        assert_eq!(dialog.value_i16(), Some(5));
    }

    #[test]
    fn escape_key_cancels() {
        let mut dialog = make_dialog("10", true);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), InputDialogResult::Cancel);
    }

    #[test]
    fn no_input_returns_none() {
        let mut dialog = make_dialog("10", true);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        let mut ui = test_frame(&mut ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), InputDialogResult::None);
    }

    #[test]
    fn default_value_is_set() {
        let dialog = make_dialog("42", false);
        assert_eq!(dialog.value_str(), "42");
        assert_eq!(dialog.value_i16(), Some(42));
        assert_eq!(dialog.value_i32(), Some(42));
    }

    #[test]
    fn typing_replaces_prefilled_value() {
        let mut dialog = make_dialog("42", true);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        {
            let mut ui = test_frame(&mut ctx, &mut state);
            dialog.build(&mut ui);
        }
        ctx.typed_chars = vec!['7'];
        let mut ui = test_frame(&mut ctx, &mut state);
        dialog.build(&mut ui);
        assert_eq!(dialog.value_str(), "7");
    }

    #[test]
    fn submitting_over_max_clamps_to_max() {
        let mut dialog = make_dialog("999", true);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), InputDialogResult::Submitted);
        assert_eq!(dialog.value_i16(), Some(50));
    }

    #[test]
    fn item_count_layout_has_one_button_at_the_official_spot() {
        let mut dialog = InputDialog::new(
            InputDialogConfig {
                layout: InputDialogLayout::ItemCount {
                    item_name: "Red Potion".to_string(),
                },
                escape_cancels: true,
                default_value: "10".to_string(),
                max_len: 6,
                numeric_only: true,
                max_value: Some(10),
            },
            WidgetId(910),
        );
        assert_eq!(dialog.window_size(), (COUNT_W, COUNT_H));

        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        let win_x = ((800.0 - COUNT_W) / 2.0).floor();
        let win_y = ((600.0 - COUNT_H) / 2.0).floor();

        ctx.mouse_clicked = true;
        ctx.mouse_x = win_x + COUNT_W - COUNT_BTN_RIGHT + 1.0;
        ctx.mouse_y = win_y + COUNT_H - COUNT_BTN_BOTTOM + 1.0;
        {
            let mut ui = test_frame(&mut ctx, &mut state);
            assert_eq!(dialog.build(&mut ui), InputDialogResult::Submitted);
        }

        ctx.mouse_x = win_x + COUNT_W - COUNT_BTN_RIGHT + FALLBACK_BTN_W + BTN_SPACING + 1.0;
        let mut ui = test_frame(&mut ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), InputDialogResult::None);
    }

    #[test]
    fn invalid_parse_returns_none() {
        let dialog = make_dialog("abc", false);
        assert_eq!(dialog.value_i16(), None);
        assert_eq!(dialog.value_i32(), None);
    }
}
