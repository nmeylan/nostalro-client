use crate::Window;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

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
pub enum ConfirmResult {
    Ok,
    Cancel,
}

pub struct ConfirmDialogState {
    pub message: String,
    pub show_cancel: bool,
    /// Informational box with no buttons (e.g. "Please wait..."), dismissed
    /// programmatically by clearing `state`.
    pub no_buttons: bool,
    onclose: Option<Box<dyn FnMut(ConfirmResult)>>,
    deliver_result: bool,
}

impl ConfirmDialogState {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            show_cancel: false,
            no_buttons: false,
            onclose: None,
            deliver_result: false,
        }
    }
}

pub struct ConfirmDialog {
    pub state: Option<ConfirmDialogState>,
    pub has_grf_textures: bool,
    result: Option<ConfirmResult>,
    btn_size: (f32, f32),
    win_size: (f32, f32),
}

impl Default for ConfirmDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self {
            state: None,
            has_grf_textures: false,
            result: None,
            btn_size: (FALLBACK_BTN_W, FALLBACK_BTN_H),
            win_size: (DIALOG_W, DIALOG_H),
        }
    }

    pub fn show<F>(&mut self, message: &str, show_cancel: bool, onclose: F)
    where
        F: FnMut(ConfirmResult) + 'static,
    {
        let mut state = ConfirmDialogState::new(message);
        state.show_cancel = show_cancel;
        state.onclose = Some(Box::new(onclose));
        self.state = Some(state);
    }

    pub fn show_confirm(&mut self, message: &str) {
        let mut state = ConfirmDialogState::new(message);
        state.show_cancel = true;
        state.deliver_result = true;
        self.result = None;
        self.state = Some(state);
    }

    pub fn take_result(&mut self) -> Option<ConfirmResult> {
        self.result.take()
    }

    /// Shows a buttonless informational box (e.g. "Please wait...") that stays
    /// until [`ConfirmDialog::dismiss`] or a new `show*` call replaces it.
    pub fn show_message(&mut self, message: &str) {
        let mut state = ConfirmDialogState::new(message);
        state.no_buttons = true;
        self.state = Some(state);
    }

    pub fn dismiss(&mut self) {
        self.state = None;
    }

    pub fn close(&mut self) {
        if let Some(ref mut state) = self.state {
            eprintln!(
                "close: show_cancel={}, has_callback={}",
                state.show_cancel,
                state.onclose.is_some()
            );
            if !state.show_cancel {
                if let Some(ref mut callback) = state.onclose.take() {
                    eprintln!("calling Ok callback");
                    callback(ConfirmResult::Ok);
                }
            } else {
                if let Some(ref mut callback) = state.onclose.take() {
                    eprintln!("calling Cancel callback");
                    callback(ConfirmResult::Cancel);
                }
            }
            self.state = None;
        }
    }

    pub fn build(&mut self, ui: &mut UiFrame) {
        let state = match &mut self.state {
            Some(s) => s,
            None => return,
        };

        let screen = Rect::new(0.0, 0.0, ui.ctx.screen_width, ui.ctx.screen_height);
        ui.interact(OVERLAY_ID, screen);
        let (v, i) = draw::quad_vertices(
            0.0,
            0.0,
            ui.ctx.screen_width,
            ui.ctx.screen_height,
            [0.0, 0.0, 0.0, 0.5],
        );
        ui.draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });

        let (dialog_w, dialog_h) = self.win_size;
        let dx = ((ui.ctx.screen_width - dialog_w) / 2.0).floor();
        let dy = ((ui.ctx.screen_height - dialog_h) / 2.0).floor();

        if self.has_grf_textures {
            let (v, i) = draw::quad_vertices(dx, dy, dialog_w, dialog_h, [1.0, 1.0, 1.0, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(WIN_TEXTURE.to_string()),
            });
        } else {
            let (v, i) = draw::quad_vertices(dx, dy, dialog_w, dialog_h, [0.2, 0.2, 0.28, 1.0]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::White,
            });
            let border_color = [0.5, 0.5, 0.6, 1.0];
            for (bx, by, bw, bh) in [
                (dx, dy, dialog_w, 1.0),
                (dx, dy + dialog_h - 1.0, dialog_w, 1.0),
                (dx, dy, 1.0, dialog_h),
                (dx + dialog_w - 1.0, dy, 1.0, dialog_h),
            ] {
                let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
                ui.draw_calls.push(DrawCall {
                    vertices: v.to_vec(),
                    indices: i.to_vec(),
                    texture: TextureRef::White,
                });
            }
        }

        let (btn_w, btn_h) = self.btn_size;
        let container = Rect::new(dx, dy, dialog_w, dialog_h);

        if state.no_buttons {
            let (text_y, text_x) = container.text_dialog_alignment(
                PADDING,
                dy + dialog_h - PADDING,
                ui.atlas.line_height,
            );
            let text_color = if self.has_grf_textures {
                [0.0, 0.0, 0.0, 1.0]
            } else {
                [1.0, 1.0, 1.0, 1.0]
            };
            ui.text(text_x, text_y, &state.message, text_color);
            return;
        }

        let num_buttons = if state.show_cancel { 2 } else { 1 };
        let btns = container.buttons_bottom_right(
            num_buttons,
            btn_w,
            btn_h,
            BTN_BOTTOM,
            BTN_FIRST_RIGHT,
            BTN_SPACING,
        );

        let (text_y, text_x) =
            container.text_dialog_alignment(PADDING, btns[0].y, ui.atlas.line_height);
        let text_color = if self.has_grf_textures {
            [0.0, 0.0, 0.0, 1.0]
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        ui.text(text_x, text_y, &state.message, text_color);

        let mut callback = state.onclose.take();
        let deliver_result = state.deliver_result;

        if state.show_cancel {
            let cancel = ui.button(CANCEL_BTN_ID, btns[0], &CANCEL_BTN, "Cancel");
            if cancel.clicked() {
                if deliver_result {
                    self.result = Some(ConfirmResult::Cancel);
                }
                if let Some(ref mut cb) = callback {
                    cb(ConfirmResult::Cancel);
                }
                self.state = None;
                return;
            }
        }

        let ok = ui.button(OK_BTN_ID, btns[num_buttons - 1], &OK_BTN, "OK");
        if ok.clicked() {
            if deliver_result {
                self.result = Some(ConfirmResult::Ok);
            }
            if let Some(ref mut cb) = callback {
                cb(ConfirmResult::Ok);
            }
            self.state = None;
            return;
        }

        state.onclose = callback;
    }
}

impl Window for ConfirmDialog {
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
        if let Some((w, h)) = size_fn(WIN_TEXTURE) {
            self.win_size = (w as f32, h as f32);
        }
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        vec![
            WIN_TEXTURE,
            OK_BTN.normal,
            OK_BTN.hover,
            OK_BTN.pressed,
            CANCEL_BTN.normal,
            CANCEL_BTN.hover,
            CANCEL_BTN.pressed,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn make_frame<'a>(ctx: &'a UiContext, state: &'a mut StateCache) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> =
            Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, 0.0, false, None, positions)
    }

    #[test]
    fn close_without_cancel_calls_ok() {
        let mut dialog = ConfirmDialog::new();
        let callback_result = Rc::new(RefCell::new(None));
        let result_clone = Rc::clone(&callback_result);
        dialog.show("Message", false, move |result| {
            *result_clone.borrow_mut() = Some(result);
        });

        dialog.close();
        assert_eq!(*callback_result.borrow(), Some(ConfirmResult::Ok));
        assert!(dialog.state.is_none());
    }

    #[test]
    fn close_with_cancel_calls_cancel() {
        let mut dialog = ConfirmDialog::new();
        let callback_result = Rc::new(RefCell::new(None));
        let result_clone = Rc::clone(&callback_result);
        dialog.show("Message", true, move |result| {
            *result_clone.borrow_mut() = Some(result);
        });

        dialog.close();
        assert_eq!(*callback_result.borrow(), Some(ConfirmResult::Cancel));
        assert!(dialog.state.is_none());
    }

    #[test]
    fn build_with_no_state_returns_early() {
        let mut dialog = ConfirmDialog::new();
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame(&ctx, &mut state);

        dialog.build(&mut ui);
    }
}
