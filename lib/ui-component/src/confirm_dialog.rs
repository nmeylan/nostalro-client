use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

const OVERLAY_ID: WidgetId = WidgetId(410);
const OK_BTN_ID: WidgetId = WidgetId(400);
const CANCEL_BTN_ID: WidgetId = WidgetId(401);

const DIALOG_W: f32 = 220.0;
const DIALOG_H: f32 = 80.0;
const PADDING: f32 = 12.0;
const BTN_SPACING: f32 = 8.0;
const FALLBACK_BTN_W: f32 = 42.0;
const FALLBACK_BTN_H: f32 = 20.0;

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
pub enum ConfirmResult {
    None,
    Ok,
    Cancel,
}

pub struct ConfirmDialog {
    pub message: String,
    btn_size: (f32, f32),
}

impl ConfirmDialog {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
        }
    }

    pub fn set_texture_sizes(&mut self, size_fn: impl Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(OK_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
    }

    pub fn build(&mut self, ui: &mut UiFrame) -> ConfirmResult {
        if ui.ctx.key_escape {
            return ConfirmResult::Cancel;
        }
        if ui.ctx.key_enter {
            return ConfirmResult::Ok;
        }

        // Full-screen overlay to block input behind
        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(OVERLAY_ID, screen);
        let (v, i) = draw::quad_vertices(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height, [0.0, 0.0, 0.0, 0.5]);
        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });

        // Dialog box centered on screen
        let dx = ((ui.ctx.screen_width - DIALOG_W) / 2.0).floor();
        let dy = ((ui.ctx.screen_height - DIALOG_H) / 2.0).floor();

        let (v, i) = draw::quad_vertices(dx, dy, DIALOG_W, DIALOG_H, [0.2, 0.2, 0.28, 1.0]);
        ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
        let border_color = [0.5, 0.5, 0.6, 1.0];
        for (bx, by, bw, bh) in [
            (dx, dy, DIALOG_W, 1.0),
            (dx, dy + DIALOG_H - 1.0, DIALOG_W, 1.0),
            (dx, dy, 1.0, DIALOG_H),
            (dx + DIALOG_W - 1.0, dy, 1.0, DIALOG_H),
        ] {
            let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
            ui.draw_calls.push(DrawCall { vertices: v.to_vec(), indices: i.to_vec(), texture: TextureRef::White });
        }

        // Message text
        let text_w = ui.atlas.measure_text(&self.message);
        let text_x = dx + (DIALOG_W - text_w) / 2.0;
        let text_y = dy + PADDING;
        ui.text(text_x, text_y, &self.message, [1.0, 1.0, 1.0, 1.0]);

        // OK / Cancel buttons centered
        let (btn_w, btn_h) = self.btn_size;
        let total_btn_w = btn_w * 2.0 + BTN_SPACING;
        let btn_x = dx + (DIALOG_W - total_btn_w) / 2.0;
        let btn_y = dy + DIALOG_H - PADDING - btn_h;

        let ok_rect = Rect::new(btn_x, btn_y, btn_w, btn_h);
        let cancel_rect = Rect::new(btn_x + btn_w + BTN_SPACING, btn_y, btn_w, btn_h);

        let ok = ui.button(OK_BTN_ID, ok_rect, &OK_BTN, "OK");
        let cancel = ui.button(CANCEL_BTN_ID, cancel_rect, &CANCEL_BTN, "Cancel");

        if ok.clicked() {
            return ConfirmResult::Ok;
        }
        if cancel.clicked() {
            return ConfirmResult::Cancel;
        }

        ConfirmResult::None
    }

    pub fn grf_texture_paths() -> Vec<&'static str> {
        vec![
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
        let atlas = FontAtlas::from_embedded(14.0);
        let atlas = Box::leak(Box::new(atlas));
        UiFrame::new(ctx, atlas, state, 0.0, false, None)
    }

    #[test]
    fn enter_key_confirms() {
        let mut dialog = ConfirmDialog::new("Are you sure?");
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_enter = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), ConfirmResult::Ok);
    }

    #[test]
    fn escape_key_cancels() {
        let mut dialog = ConfirmDialog::new("Are you sure?");
        let mut state = StateCache::new();
        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.key_escape = true;
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), ConfirmResult::Cancel);
    }

    #[test]
    fn no_input_returns_none() {
        let mut dialog = ConfirmDialog::new("Are you sure?");
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);
        assert_eq!(dialog.build(&mut ui), ConfirmResult::None);
    }
}
