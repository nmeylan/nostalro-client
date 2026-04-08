use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, TextInputBg, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use ragnarok_ui::text_input::TextInput;

const OVERLAY_ID: WidgetId = WidgetId(420);
const INPUT_ID: WidgetId = WidgetId(421);
const OK_BTN_ID: WidgetId = WidgetId(422);
const CANCEL_BTN_ID: WidgetId = WidgetId(423);

const DIALOG_W: f32 = 220.0;
const DIALOG_H: f32 = 55.0;
const PADDING: f32 = 4.0;
const BTN_SPACING: f32 = 3.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

const WIN_TEXTURE: &str = "data/texture/유저인터페이스/win_msgbox.bmp";

const OK_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_ok.bmp",
    hover: "data/texture/유저인터페이스/btn_ok_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_ok_b.bmp",
};
const CANCEL_BTN: ButtonTextures = ButtonTextures {
    normal: "data/texture/유저인터페이스/btn_cancel.bmp",
    hover: "data/texture/유저인터페이스/btn_cancel_a.bmp",
    pressed: "data/texture/유저인터페이스/btn_cancel_b.bmp",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropQuantityResult {
    None,
    Ok(i16),
    Cancel,
}

pub struct DropQuantityDialog {
    pub item_index: u16,
    pub max_count: i16,
    pub has_grf_textures: bool,
    input: TextInput,
    btn_size: (f32, f32),
    win_size: (f32, f32),
}

impl DropQuantityDialog {
    pub fn new(item_index: u16, max_count: i16) -> Self {
        let mut input = TextInput::new(6, false);
        input.text = max_count.to_string();
        input.cursor_pos = input.text.chars().count();
        Self {
            item_index,
            max_count,
            has_grf_textures: false,
            input,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            win_size: (DIALOG_W, DIALOG_H),
        }
    }

    pub fn set_texture_sizes(&mut self, size_fn: impl Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(OK_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
        if let Some((w, h)) = size_fn(WIN_TEXTURE) {
            self.win_size = (w as f32, h as f32);
        }
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> DropQuantityResult {
        if ui.ctx.key_escape {
            return DropQuantityResult::Cancel;
        }

        // Full-screen overlay to block input behind
        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(OVERLAY_ID, screen);
        let (v, i) = draw::quad_vertices(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height, [0.0, 0.0, 0.0, 0.5]);
        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });

        // Dialog box centered on screen
        let (dialog_w, dialog_h) = self.win_size;
        let dx = ((ui.ctx.screen_width - dialog_w) / 2.0).floor();
        let dy = ((ui.ctx.screen_height - dialog_h) / 2.0).floor();

        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(dx, dy, dialog_w, dialog_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::Named(WIN_TEXTURE.to_string()) });
        } else {
            let (v, i) = draw::quad_vertices(dx, dy, dialog_w, dialog_h, [0.2, 0.2, 0.28, 1.0]);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
            let border_color = [0.5, 0.5, 0.6, 1.0];
            for (bx, by, bw, bh) in [
                (dx, dy, dialog_w, 1.0),
                (dx, dy + dialog_h - 1.0, dialog_w, 1.0),
                (dx, dy, 1.0, dialog_h),
                (dx + dialog_w - 1.0, dy, 1.0, dialog_h),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
                ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
            }
        }

        let text_color = if self.has_grf_textures { [0.0, 0.0, 0.0, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };

        // Label
        let label = format!("How many (max {})?", self.max_count);
        let label_w = ui.atlas.measure_text(&label);
        let label_x = dx + (dialog_w - label_w) / 2.0;
        let label_y = dy + PADDING + ui.atlas.line_height;
        ui.text(label_x, label_y, &label, text_color);

        // Input field
        let input_y = label_y + PADDING;
        let (btn_w, btn_h) = self.btn_size;
        let input_w = dialog_w - PADDING * 2.0 - btn_w * 2.0 - BTN_SPACING * 3.0;
        let input_rect = Rect::new(dx + PADDING, input_y, input_w, 16.0);

        if ui.focused() != Some(INPUT_ID) {
            ui.set_focus(INPUT_ID);
        }
        let input_bg = if self.has_grf_textures { TextInputBg::Transparent } else { TextInputBg::Default };
        ui.text_input(INPUT_ID, input_rect, &mut self.input, input_bg);

        // OK / Cancel buttons
        let btn_x = dx + PADDING + input_w + BTN_SPACING;
        let ok_rect = Rect::new(btn_x, input_y - 2.0, btn_w, btn_h);
        let cancel_rect = Rect::new(btn_x + btn_w + BTN_SPACING, input_y - 2.0, btn_w, btn_h);

        let ok = ui.button(OK_BTN_ID, ok_rect, &OK_BTN, "OK");
        let cancel = ui.button(CANCEL_BTN_ID, cancel_rect, &CANCEL_BTN, "Cancel");

        if ok.clicked() || ui.ctx.key_enter {
            let qty: i16 = self.input.text.parse().unwrap_or(0);
            if qty > 0 && qty <= self.max_count {
                return DropQuantityResult::Ok(qty);
            }
            return DropQuantityResult::Cancel;
        }
        if cancel.clicked() {
            return DropQuantityResult::Cancel;
        }

        DropQuantityResult::None
    }

    pub fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            WIN_TEXTURE,
            OK_BTN.normal, OK_BTN.hover, OK_BTN.pressed,
            CANCEL_BTN.normal, CANCEL_BTN.hover, CANCEL_BTN.pressed,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_renderer::font_atlas::FontAtlas;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn enter_key_confirms_with_valid_quantity() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        dialog.input.text = "5".to_string();
        dialog.input.cursor_pos = 1;
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::Ok(5));
    }

    #[test]
    fn enter_key_cancels_with_zero() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        dialog.input.text = "0".to_string();
        dialog.input.cursor_pos = 1;
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::Cancel);
    }

    #[test]
    fn enter_key_cancels_with_over_max() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        dialog.input.text = "11".to_string();
        dialog.input.cursor_pos = 2;
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::Cancel);
    }

    #[test]
    fn escape_key_cancels() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::Cancel);
    }

    #[test]
    fn no_input_returns_none() {
        let mut dialog = DropQuantityDialog::new(0, 10);
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), DropQuantityResult::None);
    }

    #[test]
    fn initial_text_is_max_count() {
        let dialog = DropQuantityDialog::new(5, 42);
        assert_eq!(dialog.input.text, "42");
        assert_eq!(dialog.input.cursor_pos, 2);
    }
}
