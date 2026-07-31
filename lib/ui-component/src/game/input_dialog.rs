use crate::Window;
use crate::helper::dialog_container::DialogContainer;
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

pub const OK_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_ok.bmp",
    hover: "data/texture/유저인터페이스/btn_ok_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_ok_b.bmp",
};
pub const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_cancel.bmp",
    hover: "data/texture/유저인터페이스/btn_cancel_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_cancel_b.bmp",
};

const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;
const DIALOG_W: f32 = 220.0;
const DIALOG_H: f32 = 55.0;
const PADDING: f32 = 4.0;
const PADDING_X: f32 = 12.0;
const BTN_SPACING: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDialogResult {
    None,
    Submitted,
    Cancel,
}

pub struct InputDialogConfig {
    pub label: Option<String>,
    pub show_cancel: bool,
    pub escape_cancels: bool,
    pub default_value: String,
    pub max_len: usize,
    pub numeric_only: bool,
}

pub struct InputDialog {
    pub has_grf_textures: bool,
    input: TextInput,
    btn_size: (f32, f32),
    show_cancel: bool,
    escape_cancels: bool,
    label: Option<String>,
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
        input.cursor_pos = input.text.chars().count();
        Self {
            has_grf_textures: false,
            input,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            show_cancel: config.show_cancel,
            escape_cancels: config.escape_cancels,
            label: config.label,
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
        self.input.cursor_pos = text.chars().count();
    }

    pub fn clear_input(&mut self) {
        self.input.text.clear();
        self.input.cursor_pos = 0;
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

    pub fn build(&mut self, ui: &mut UiFrame) -> InputDialogResult {
        if self.escape_cancels && ui.take_escape() {
            return InputDialogResult::Cancel;
        }

        let dw = DIALOG_W;
        let label_h = if self.label.is_some() {
            ui.atlas.line_height + PADDING
        } else {
            0.0
        };
        let dh = DIALOG_H + label_h;
        let title_bar_h = PADDING * 2.0 + ui.atlas.line_height;
        let win = ui.window(self.win_id(), dw, dh, title_bar_h);
        let dx = win.x;
        let dy = win.y;

        self.container.has_grf_textures = self.has_grf_textures;
        self.container
            .draw(&mut ui.draw_calls, dx, dy, dw, dh, [1.0, 1.0, 1.0, 1.0]);

        let text_color = self.container.text_color();

        let mut content_y = dy + PADDING + ui.atlas.line_height;
        if let Some(label) = &self.label {
            ui.text(dx + PADDING_X, content_y, label, text_color);
            content_y += label_h;
        }

        let (btn_w, btn_h) = self.btn_size;
        let cancel_space = if self.show_cancel {
            btn_w + BTN_SPACING
        } else {
            0.0
        };
        let input_w = dw - PADDING_X * 2.0 - btn_w - cancel_space - BTN_SPACING * 2.0;
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

        let input_rect = Rect::new(dx + PADDING_X, content_y, input_w, 16.0);
        ui.text_input(input_id, input_rect, &mut self.input, input_bg);

        let btn_x = PADDING_X + dx + input_w + BTN_SPACING * 2.0;
        let ok_rect = Rect::new(btn_x, content_y - 2.0, btn_w, btn_h);
        let ok = ui.button(ok_id, ok_rect, &OK_BTN, "OK");

        if self.show_cancel {
            let cancel_rect = Rect::new(btn_x + btn_w + BTN_SPACING, content_y - 2.0, btn_w, btn_h);
            let cancel = ui.button(self.cancel_id(), cancel_rect, &CANCEL_BTN, "Cancel");
            if cancel.clicked() {
                return InputDialogResult::Cancel;
            }
        }

        if ok.clicked() || ui.ctx.key_enter {
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

    fn make_dialog(default_value: &str, show_cancel: bool) -> InputDialog {
        InputDialog::new(
            InputDialogConfig {
                label: Some("How many?".to_string()),
                show_cancel,
                escape_cancels: true,
                default_value: default_value.to_string(),
                max_len: 6,
                numeric_only: true,
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
        let mut ui = make_frame(&ctx, &mut state);
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
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), InputDialogResult::Cancel);
    }

    #[test]
    fn no_input_returns_none() {
        let mut dialog = make_dialog("10", true);
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);
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
    fn invalid_parse_returns_none() {
        let dialog = make_dialog("abc", false);
        assert_eq!(dialog.value_i16(), None);
        assert_eq!(dialog.value_i32(), None);
    }
}
