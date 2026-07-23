pub mod char_create_window;
pub mod char_select_window;
pub mod login_server_list_window;
pub mod login_window;
pub mod server_list_window;

use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::UiFrame;

/// Draws a full-screen stretched background behind an account-flow screen. When
/// `texture` is `None` nothing is drawn and the renderer's clear color shows.
pub fn draw_background(ui: &mut UiFrame, texture: Option<&str>) {
    let Some(name) = texture else {
        return;
    };
    let (verts, indices) = draw::quad_vertices(
        0.0,
        0.0,
        ui.ctx.screen_width,
        ui.ctx.screen_height,
        [1.0, 1.0, 1.0, 1.0],
    );
    ui.draw_calls.push(DrawCall {
        vertices: verts.to_vec(),
        indices: indices.to_vec(),
        texture: TextureRef::Named(name.to_string()),
    });
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

    #[test]
    fn draw_background_pushes_fullscreen_quad_or_nothing() {
        let ctx = UiContext::new(1024.0, 768.0);
        let mut state = StateCache::new();

        let mut ui = make_frame(&ctx, &mut state);
        draw_background(&mut ui, None);
        assert!(ui.draw_calls.is_empty());

        let mut ui = make_frame(&ctx, &mut state);
        draw_background(&mut ui, Some("bg.bmp"));
        assert_eq!(ui.draw_calls.len(), 1);
        match &ui.draw_calls[0].texture {
            TextureRef::Named(n) => assert_eq!(n, "bg.bmp"),
            _ => panic!("expected named texture"),
        }
        let xs: Vec<f32> = ui.draw_calls[0].vertices.iter().map(|v| v.position[0]).collect();
        assert!(xs.iter().cloned().fold(f32::MIN, f32::max) >= 1024.0);
    }
}
