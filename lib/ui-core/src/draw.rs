use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::ui_renderer::UiVertex;
pub use ragnarok_renderer::{UiDrawCall as DrawCall, UiTextureRef as TextureRef};

pub fn quad_vertices(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> ([UiVertex; 4], [u32; 6]) {
    quad_vertices_uv(x, y, w, h, [0.0, 0.0], [1.0, 1.0], color)
}

pub fn quad_vertices_uv(
    x: f32, y: f32, w: f32, h: f32,
    uv_min: [f32; 2], uv_max: [f32; 2],
    color: [f32; 4],
) -> ([UiVertex; 4], [u32; 6]) {
    let verts = [
        UiVertex { position: [x, y], tex_coord: [uv_min[0], uv_min[1]], color },
        UiVertex { position: [x + w, y], tex_coord: [uv_max[0], uv_min[1]], color },
        UiVertex { position: [x, y + h], tex_coord: [uv_min[0], uv_max[1]], color },
        UiVertex { position: [x + w, y + h], tex_coord: [uv_max[0], uv_max[1]], color },
    ];
    let indices = [0, 1, 2, 2, 1, 3];
    (verts, indices)
}

pub fn text_vertices(
    text: &str, x: f32, y: f32, color: [f32; 4], atlas: &FontAtlas,
) -> (Vec<UiVertex>, Vec<u32>) {
    text_vertices_clipped(text, x, y, color, atlas, f32::NEG_INFINITY, f32::INFINITY)
}

pub fn text_vertices_clipped(
    text: &str, x: f32, y: f32, color: [f32; 4], atlas: &FontAtlas,
    clip_left: f32, clip_right: f32,
) -> (Vec<UiVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut cursor_x = x;

    for ch in text.chars() {
        let glyph = atlas.glyph(ch);

        if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
            let gx = (cursor_x + glyph.offset[0]).round();
            let gy = (y + glyph.offset[1]).round();
            let gx_right = gx + glyph.size[0];

            // Skip fully outside glyphs
            if gx_right > clip_left && gx < clip_right {
                let mut draw_x = gx;
                let mut draw_w = glyph.size[0];
                let mut uv_left = glyph.uv_min[0];
                let mut uv_right = glyph.uv_max[0];
                let uv_span = uv_right - uv_left;
                let px_span = glyph.size[0];

                // Clip left edge
                if draw_x < clip_left {
                    let clipped = clip_left - draw_x;
                    uv_left += uv_span * (clipped / px_span);
                    draw_w -= clipped;
                    draw_x = clip_left;
                }
                // Clip right edge
                if draw_x + draw_w > clip_right {
                    let clipped = (draw_x + draw_w) - clip_right;
                    uv_right -= uv_span * (clipped / px_span);
                    draw_w -= clipped;
                }

                let base = vertices.len() as u32;
                let (verts, idxs) = quad_vertices_uv(
                    draw_x, gy, draw_w, glyph.size[1],
                    [uv_left, glyph.uv_min[1]], [uv_right, glyph.uv_max[1]],
                    color,
                );
                vertices.extend_from_slice(&verts);
                indices.extend(idxs.iter().map(|i| i + base));
            }
        }

        cursor_x += glyph.advance;
    }

    (vertices, indices)
}
