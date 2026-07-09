use crate::helper::dialog_container::DialogContainer;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};

pub const VENDING_ICON_TEX: &str = "data/texture/유저인터페이스/shop.bmp";
const PAD: f32 = 4.0;
const ICON: f32 = 24.0;
/// Fixed board width, matching the chat-room board; long names are clipped.
pub const BOARD_W: f32 = 140.0;

/// Screen-space `[x0, y0, x1, y1]` of a shop board centered above a head anchor.
pub fn board_rect(anchor_x: f32, anchor_y: f32, head_offset: f32) -> [f32; 4] {
    let box_h = PAD + ICON + PAD;
    let box_x = anchor_x - BOARD_W / 2.0;
    let box_y = anchor_y - head_offset - 5.0 - box_h;
    [box_x, box_y, box_x + BOARD_W, box_y + box_h]
}

/// Textures needed to render the board with GRF art (frame + bag icon).
pub fn grf_texture_paths() -> Vec<&'static str> {
    let mut paths = DialogContainer::grf_texture_paths();
    paths.push(VENDING_ICON_TEX);
    paths
}

/// Draw the vendor's shop board (frame + bag icon + name) into `draw_calls`.
/// `container` supplies the frame art/sizing; when it has no GRF textures the
/// bag icon is omitted and the fallback frame is used.
pub fn draw_board(
    draw_calls: &mut Vec<DrawCall>,
    container: &DialogContainer,
    atlas: &FontAtlas,
    anchor_x: f32,
    anchor_y: f32,
    head_offset: f32,
    name: &str,
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
            texture: TextureRef::Named(VENDING_ICON_TEX.to_string()),
        });
    }

    let text_x = box_x + PAD + ICON + PAD;
    let text_y = box_y + (box_h + atlas.line_height) / 2.0;
    let avail_w = box_x1 - text_x - PAD;
    let label = truncate_to_width(name, avail_w, atlas);
    let (verts, indices) = draw::text_vertices(&label, text_x, text_y, container.text_color(), atlas);
    if !verts.is_empty() {
        draw_calls.push(DrawCall {
            vertices: verts,
            indices,
            texture: TextureRef::FontAtlas,
        });
    }
}

fn truncate_to_width(text: &str, max_w: f32, atlas: &FontAtlas) -> String {
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
