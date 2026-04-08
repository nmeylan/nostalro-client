use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;
use crate::dialog_container::DialogContainer;

const OVERLAY_ID: WidgetId = WidgetId(410);
const OK_BTN_ID: WidgetId = WidgetId(400);
const CANCEL_BTN_ID: WidgetId = WidgetId(401);

const DIALOG_W: f32 = 220.0;
const DIALOG_H: f32 = 40.0;
const PADDING: f32 = 4.0;
const BTN_BOTTOM: f32 = 4.0;
const BTN_FIRST_RIGHT: f32 = 5.0;
const BTN_SPACING: f32 = 3.0;
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
    pub has_grf_textures: bool,
    btn_size: (f32, f32),
    container: DialogContainer,
}

impl ConfirmDialog {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            has_grf_textures: false,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            container: DialogContainer::new(),
        }
    }

    pub fn set_texture_sizes(&mut self, size_fn: impl Fn(&str) -> Option<(u32, u32)>) {
        if let Some((w, h)) = size_fn(OK_BTN.normal) {
            self.btn_size = (w as f32, h as f32);
        }
        self.container.set_texture_sizes(&size_fn);
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
        let dialog_w = DIALOG_W;
        let dialog_h = DIALOG_H;
        let dx = ((ui.ctx.screen_width - dialog_w) / 2.0).floor();
        let dy = ((ui.ctx.screen_height - dialog_h) / 2.0).floor();

        self.container.has_grf_textures = self.has_grf_textures;
        self.container.draw(&mut ui.draw_calls, dx, dy, dialog_w, dialog_h, [1.0, 1.0, 1.0, 1.0]);

        // OK / Cancel buttons right-aligned at bottom
        let (btn_w, btn_h) = self.btn_size;
        let container = Rect::new(dx, dy, dialog_w, dialog_h);
        let btns = container.buttons_bottom_right(2, btn_w, btn_h, BTN_BOTTOM, BTN_FIRST_RIGHT, BTN_SPACING);

        // Message text centered
        let (text_y, text_x) =
            container.text_dialog_alignment(PADDING , btns[0].y, ui.atlas.line_height);
        ui.text(text_x, text_y, &self.message, self.container.text_color());

        let cancel = ui.button(CANCEL_BTN_ID, btns[0], &CANCEL_BTN, "Cancel");
        let ok = ui.button(OK_BTN_ID, btns[1], &OK_BTN, "OK");

        if ok.clicked() {
            return ConfirmResult::Ok;
        }
        if cancel.clicked() {
            return ConfirmResult::Cancel;
        }

        ConfirmResult::None
    }

    pub fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = DialogContainer::grf_texture_paths();
        paths.extend_from_slice(&[
            OK_BTN.normal, OK_BTN.hover, OK_BTN.pressed,
            CANCEL_BTN.normal, CANCEL_BTN.hover, CANCEL_BTN.pressed,
        ]);
        paths
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
