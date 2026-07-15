use crate::helper::dialog_container::DialogContainer;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};

const PAD: f32 = 4.0;
const ICON: f32 = 24.0;
/// Fixed board width; long labels are clipped by the caller.
pub const BOARD_W: f32 = 140.0;

/// Screen-space `[x0, y0, x1, y1]` of a board centered above a head anchor.
pub fn board_rect(anchor_x: f32, anchor_y: f32, head_offset: f32) -> [f32; 4] {
    let box_h = PAD + ICON + PAD;
    let box_x = anchor_x - BOARD_W / 2.0;
    let box_y = anchor_y - head_offset - 5.0 - box_h;
    [box_x, box_y, box_x + BOARD_W, box_y + box_h]
}

/// Pixel width available to the label between the icon and the right edge.
pub fn label_max_width() -> f32 {
    BOARD_W - ICON - 3.0 * PAD
}

/// Container art (frame) plus whatever icons a board type needs.
pub fn grf_texture_paths(icons: &[&'static str]) -> Vec<&'static str> {
    let mut paths = DialogContainer::grf_texture_paths();
    paths.extend_from_slice(icons);
    paths
}

/// Draw a labeled board above a head: frame + a left icon (only when
/// `container` has GRF art) + the `label` as-is (callers pre-clip/compose it).
pub fn draw_board(
    draw_calls: &mut Vec<DrawCall>,
    container: &DialogContainer,
    atlas: &FontAtlas,
    anchor_x: f32,
    anchor_y: f32,
    head_offset: f32,
    icon_tex: &str,
    label: &str,
) {
    let [box_x, box_y, box_x1, box_y1] = board_rect(anchor_x, anchor_y, head_offset);
    let box_w = box_x1 - box_x;
    let box_h = box_y1 - box_y;

    container.draw(draw_calls, box_x, box_y, box_w, box_h, [1.0; 4]);

    if container.has_grf_textures {
        let icon_x = box_x + PAD;
        let icon_y = box_y + (box_h - ICON) / 2.0;
        let (v, i) = draw::quad_vertices(icon_x, icon_y, ICON, ICON, [1.0; 4]);
        draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::Named(icon_tex.to_string()),
        });
    }

    let text_x = box_x + PAD + ICON + PAD;
    let text_y = box_y + (box_h + atlas.line_height) / 2.0;
    let (verts, indices) = draw::text_vertices(label, text_x, text_y, container.text_color(), atlas);
    if !verts.is_empty() {
        draw_calls.push(DrawCall {
            vertices: verts,
            indices,
            texture: TextureRef::FontAtlas,
        });
    }
}

/// Clip `text` to `max_w` pixels, appending `…` when it doesn't fit.
pub fn truncate_to_width(text: &str, max_w: f32, atlas: &FontAtlas) -> String {
    if atlas.measure_text(text) <= max_w {
        return text.to_string();
    }
    let ellipsis = "…";
    let ellipsis_w = atlas.measure_text(ellipsis);
    let mut out = String::new();
    for ch in text.chars() {
        let mut trial = out.clone();
        trial.push(ch);
        if atlas.measure_text(&trial) + ellipsis_w > max_w {
            break;
        }
        out.push(ch);
    }
    out.push_str(ellipsis);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_sits_centered_above_owner_head() {
        let [x0, y0, x1, y1] = board_rect(100.0, 200.0, 50.0);
        assert!(((x0 + x1) / 2.0 - 100.0).abs() < f32::EPSILON);
        assert!((x1 - x0 - BOARD_W).abs() < f32::EPSILON);
        let _ = y0;
        assert!(y1 < 200.0);
    }
}
