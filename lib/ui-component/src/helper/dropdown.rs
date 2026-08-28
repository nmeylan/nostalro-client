use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{ButtonTextures, UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const OPTION_H: f32 = 16.0;

const CARET: ButtonTextures = ButtonTextures {
    normal: ragnarok_resources::ui::basic::TXTBOX_BTN_A,
    hover: ragnarok_resources::ui::basic::TXTBOX_BTN_B,
    pressed: ragnarok_resources::ui::basic::TXTBOX_BTN_C,
};

const BG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BORDER: [f32; 4] = [0.761, 0.761, 0.761, 1.0];
const SELECTION: [f32; 4] = [0.451, 0.62, 0.937, 1.0];
const TEXT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

pub fn grf_texture_paths() -> Vec<&'static str> {
    vec![CARET.normal, CARET.hover, CARET.pressed]
}

#[derive(Default)]
pub struct Dropdown {
    pub open: bool,
    opened_this_frame: bool,
}

pub struct DropdownResponse {
    pub toggled: bool,
    pub overlay_rect: Option<Rect>,
}

impl Dropdown {
    pub fn begin_frame(&mut self) {
        self.opened_this_frame = false;
    }

    /// Draws the closed control (box + label + caret) and toggles `open` on click.
    /// When open, returns the option-list rect, flipped above the box when it would
    /// overflow `content_bounds`.
    pub fn show(
        &mut self,
        ui: &mut UiFrame,
        id: WidgetId,
        rect: Rect,
        label: &str,
        option_count: usize,
        content_bounds: Rect,
        blocked: bool,
    ) -> DropdownResponse {
        let resp = ui.interact(id, rect);
        if resp.hovered() {
            ui.any_interactive_hovered = true;
        }

        fill(ui, rect.x, rect.y, rect.w, rect.h, BG);
        border(ui, rect);
        ui.text(rect.x + 3.0, rect.y + 12.0, label, TEXT);

        let caret = Rect::new(rect.x + rect.w - rect.h, rect.y, rect.h, rect.h);
        if ui.has_grf_textures {
            let pressed = resp.hovered() && (ui.ctx.mouse_clicked || ui.ctx.mouse_down);
            let tex = if pressed {
                CARET.pressed
            } else if resp.hovered() {
                CARET.hover
            } else {
                CARET.normal
            };
            let (v, i) = draw::quad_vertices(caret.x, caret.y, caret.w, caret.h, BG);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(tex.to_string()),
            });
        } else {
            down_arrow(ui, caret.x + caret.w / 2.0, caret.y + 6.0, TEXT);
        }

        let mut toggled = false;
        if resp.clicked() && !blocked {
            self.open = !self.open;
            if self.open {
                self.opened_this_frame = true;
            }
            toggled = true;
        }

        let overlay_rect = self.open.then(|| {
            let list_h = option_count as f32 * OPTION_H;
            let down_y = rect.y + rect.h;
            let list_y = if down_y + list_h > content_bounds.y + content_bounds.h {
                (rect.y - list_h).max(content_bounds.y)
            } else {
                down_y
            };
            Rect::new(rect.x, list_y, rect.w, list_h)
        });

        DropdownResponse {
            toggled,
            overlay_rect,
        }
    }

    /// Draws the open option list on the popup layer and returns the clicked index.
    /// Closes the dropdown when an option is picked.
    pub fn show_overlay(
        &mut self,
        ui: &mut UiFrame,
        rect: Rect,
        option_base: u32,
        labels: &[&str],
    ) -> Option<usize> {
        ui.begin_popup_layer(rect);
        fill(ui, rect.x, rect.y, rect.w, rect.h, BG);
        border(ui, rect);
        let mut picked = None;
        for (i, label) in labels.iter().enumerate() {
            let item = Rect::new(rect.x, rect.y + i as f32 * OPTION_H, rect.w, OPTION_H);
            let r = ui.interact(WidgetId(option_base + i as u32), item);
            if r.hovered() {
                ui.any_interactive_hovered = true;
                fill(ui, item.x, item.y, item.w, item.h, SELECTION);
            }
            ui.text(item.x + 4.0, item.y + 12.0, label, TEXT);
            if r.clicked() && !self.opened_this_frame {
                picked = Some(i);
                self.open = false;
            }
        }
        ui.end_popup_layer();
        picked
    }
}

fn fill(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let (v, i) = draw::quad_vertices(x, y, w, h, color);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
}

fn border(ui: &mut UiFrame, r: Rect) {
    fill(ui, r.x, r.y, r.w, 1.0, BORDER);
    fill(ui, r.x, r.y + r.h - 1.0, r.w, 1.0, BORDER);
    fill(ui, r.x, r.y, 1.0, r.h, BORDER);
    fill(ui, r.x + r.w - 1.0, r.y, 1.0, r.h, BORDER);
}

fn down_arrow(ui: &mut UiFrame, cx: f32, top: f32, color: [f32; 4]) {
    for r in 0..4 {
        let w = (7 - r * 2) as f32;
        fill(ui, cx - w / 2.0, top + r as f32, w, 1.0, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;
    use ragnarok_ui::test_support::test_frame;

    #[test]
    fn opens_on_box_click_then_selects_option_next_frame() {
        let mut state = StateCache::new();
        let mut dd = Dropdown::default();
        let labels = ["Leader", "Officer", "Member"];
        let box_rect = Rect::new(100.0, 100.0, 62.0, 16.0);
        let bounds = Rect::new(0.0, 0.0, 800.0, 600.0);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 110.0;
        ctx.mouse_y = 105.0;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        dd.begin_frame();
        let resp = dd.show(
            &mut ui,
            WidgetId(1),
            box_rect,
            "Member",
            labels.len(),
            bounds,
            false,
        );
        assert!(resp.toggled);
        assert!(dd.open);
        let overlay = resp.overlay_rect.expect("overlay when open");
        assert_eq!(dd.show_overlay(&mut ui, overlay, 10, &labels), None);

        let mut ctx = UiContext::new(800.0, 600.0);
        ctx.mouse_x = 110.0;
        ctx.mouse_y = overlay.y + OPTION_H + 8.0;
        ctx.mouse_clicked = true;
        let mut ui = test_frame(&mut ctx, &mut state);
        dd.begin_frame();
        let resp = dd.show(
            &mut ui,
            WidgetId(1),
            box_rect,
            "Member",
            labels.len(),
            bounds,
            false,
        );
        assert!(!resp.toggled);
        let overlay = resp.overlay_rect.expect("still open");
        assert_eq!(dd.show_overlay(&mut ui, overlay, 10, &labels), Some(1));
        assert!(!dd.open);
    }
}
