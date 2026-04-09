use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::UiFrame;
use crate::Window;
use crate::helper::dialog_container::DialogContainer;

const DISPLAY_DURATION: f32 = 3.0;
const FADE_OUT_DURATION: f32 = 1.0;
const PADDING: f32 = 4.0;
const ICON_SIZE: f32 = 24.0;
const TOP_Y: f32 = 50.0;

struct PickupEntry {
    item_name: String,
    count: u16,
    icon_texture: Option<String>,
    start_time: Option<f32>,
}

pub struct ItemPickupNotification {
    pub has_grf_textures: bool,
    pub container: DialogContainer,
    entry: Option<PickupEntry>,
}

impl ItemPickupNotification {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            container: DialogContainer::new(),
            entry: None,
        }
    }

    pub fn show(&mut self, item_name: String, count: u16, icon_texture: Option<String>) {
        self.entry = Some(PickupEntry {
            item_name,
            count,
            icon_texture,
            start_time: None,
        });
    }

    pub fn build(&mut self, ui: &mut UiFrame) {
        let Some(entry) = &mut self.entry else { return };

        let start = *entry.start_time.get_or_insert(ui.elapsed_secs);
        let age = ui.elapsed_secs - start;
        let total = DISPLAY_DURATION + FADE_OUT_DURATION;

        if age > total {
            self.entry = None;
            return;
        }

        let alpha = if age > DISPLAY_DURATION {
            1.0 - (age - DISPLAY_DURATION) / FADE_OUT_DURATION
        } else {
            1.0
        };

        let text = format!("{} - {} obtained.", entry.item_name, entry.count);
        let text_w = ui.atlas.measure_text(&text);
        let has_icon = entry.icon_texture.is_some();
        let icon_space = if has_icon { ICON_SIZE + PADDING } else { 0.0 };
        let bar_w = PADDING + icon_space + text_w + PADDING;
        let bar_h = PADDING + ICON_SIZE + PADDING;

        let x = ((ui.ctx.screen_width - bar_w) / 2.0).floor();
        let y = TOP_Y;

        self.container.draw(&mut ui.draw_calls, x, y, bar_w, bar_h, [1.0, 1.0, 1.0, alpha]);

        if let Some(icon_path) = &entry.icon_texture {
            let ix = x + PADDING;
            let iy = y + PADDING;
            let (v, i) = draw::quad_vertices(ix, iy, ICON_SIZE, ICON_SIZE, [1.0, 1.0, 1.0, alpha]);
            ui.draw_calls.push(DrawCall {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: TextureRef::Named(icon_path.clone()),
            });
        }

        let tx = x + PADDING + icon_space;
        let ty = y + PADDING + ui.atlas.line_height + (ICON_SIZE - ui.atlas.line_height) / 2.0;
        let mut color = self.container.text_color();
        color[3] = alpha;
        ui.text(tx, ty, &text, color);
    }

    pub fn is_empty(&self) -> bool {
        self.entry.is_none()
    }

}

impl Window for ItemPickupNotification {
    fn has_grf_textures(&self) -> bool { self.has_grf_textures }
    fn set_has_grf_textures(&mut self, value: bool) { self.has_grf_textures = value; }

    fn set_texture_sizes(&mut self, size_fn: &dyn Fn(&str) -> Option<(u32, u32)>) {
        self.container.set_texture_sizes(size_fn);
    }

    fn grf_texture_paths() -> Vec<&'static str> {
        DialogContainer::grf_texture_paths()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_renderer::font_atlas::FontAtlas;
    use ragnarok_ui::context::UiContext;
    use ragnarok_ui::state::StateCache;

    fn make_frame_with_elapsed<'a>(
        ctx: &'a UiContext, state: &'a mut StateCache, elapsed: f32,
    ) -> UiFrame<'a> {
        let atlas = FontAtlas::from_embedded(14.0, 1.0);
        let atlas = Box::leak(Box::new(atlas));
        let positions: &'static std::collections::HashMap<u32, [f32; 2]> = Box::leak(Box::default());
        UiFrame::new(ctx, atlas, state, elapsed, false, None, positions)
    }

    #[test]
    fn empty_notification_no_draws() {
        let mut notif = ItemPickupNotification::new();
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame_with_elapsed(&ctx, &mut state, 0.0);
        notif.build(&mut ui);
        assert!(ui.draw_calls.is_empty());
    }

    #[test]
    fn visible_after_show() {
        let mut notif = ItemPickupNotification::new();
        notif.show("Red Potion".to_string(), 5, None);
        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);
        let mut ui = make_frame_with_elapsed(&ctx, &mut state, 1.0);
        notif.build(&mut ui);
        assert!(!ui.draw_calls.is_empty());
        assert!(!notif.is_empty());
    }

    #[test]
    fn expires_after_duration() {
        let mut notif = ItemPickupNotification::new();
        notif.show("Red Potion".to_string(), 5, None);

        let mut state = StateCache::new();
        let ctx = UiContext::new(800.0, 600.0);

        // First build sets start_time to 0.0
        let mut ui = make_frame_with_elapsed(&ctx, &mut state, 0.0);
        notif.build(&mut ui);
        assert!(!notif.is_empty());

        // Build after total duration
        let mut ui = make_frame_with_elapsed(&ctx, &mut state, 5.0);
        notif.build(&mut ui);
        assert!(notif.is_empty());
    }
}
