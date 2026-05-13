use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::ui_renderer::UiVertex;
pub use ragnarok_renderer::{UiDrawCall as DrawCall, UiTextureRef as TextureRef};

pub fn quad_vertices(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> ([UiVertex; 4], [u32; 6]) {
    quad_vertices_uv(x, y, w, h, [0.0, 0.0], [1.0, 1.0], color)
}

/// Quad from explicit corner coordinates - avoids float drift from `x + w` recomputation
pub fn quad_from_bounds(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: [f32; 4],
) -> ([UiVertex; 4], [u32; 6]) {
    let uv_min = [0.0, 0.0];
    let uv_max = [1.0, 1.0];
    let verts = [
        UiVertex {
            position: [x0, y0],
            tex_coord: [uv_min[0], uv_min[1]],
            color,
        },
        UiVertex {
            position: [x1, y0],
            tex_coord: [uv_max[0], uv_min[1]],
            color,
        },
        UiVertex {
            position: [x0, y1],
            tex_coord: [uv_min[0], uv_max[1]],
            color,
        },
        UiVertex {
            position: [x1, y1],
            tex_coord: [uv_max[0], uv_max[1]],
            color,
        },
    ];
    let indices = [0, 1, 2, 2, 1, 3];
    (verts, indices)
}

pub fn quad_vertices_uv(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: [f32; 4],
) -> ([UiVertex; 4], [u32; 6]) {
    let verts = [
        UiVertex {
            position: [x, y],
            tex_coord: [uv_min[0], uv_min[1]],
            color,
        },
        UiVertex {
            position: [x + w, y],
            tex_coord: [uv_max[0], uv_min[1]],
            color,
        },
        UiVertex {
            position: [x, y + h],
            tex_coord: [uv_min[0], uv_max[1]],
            color,
        },
        UiVertex {
            position: [x + w, y + h],
            tex_coord: [uv_max[0], uv_max[1]],
            color,
        },
    ];
    let indices = [0, 1, 2, 2, 1, 3];
    (verts, indices)
}

pub fn quad_vertices_rotated(
    cx: f32,
    cy: f32,
    size: f32,
    angle_rad: f32,
    color: [f32; 4],
) -> ([UiVertex; 4], [u32; 6]) {
    let half = size / 2.0;
    let cos = angle_rad.cos();
    let sin = angle_rad.sin();
    // Corners relative to center: TL, TR, BL, BR
    let corners = [
        (-half, -half),
        (half, -half),
        (-half, half),
        (half, half),
    ];
    let verts = [
        UiVertex {
            position: [cx + corners[0].0 * cos - corners[0].1 * sin,
                        cy + corners[0].0 * sin + corners[0].1 * cos],
            tex_coord: [0.0, 0.0],
            color,
        },
        UiVertex {
            position: [cx + corners[1].0 * cos - corners[1].1 * sin,
                        cy + corners[1].0 * sin + corners[1].1 * cos],
            tex_coord: [1.0, 0.0],
            color,
        },
        UiVertex {
            position: [cx + corners[2].0 * cos - corners[2].1 * sin,
                        cy + corners[2].0 * sin + corners[2].1 * cos],
            tex_coord: [0.0, 1.0],
            color,
        },
        UiVertex {
            position: [cx + corners[3].0 * cos - corners[3].1 * sin,
                        cy + corners[3].0 * sin + corners[3].1 * cos],
            tex_coord: [1.0, 1.0],
            color,
        },
    ];
    let indices = [0, 1, 2, 2, 1, 3];
    (verts, indices)
}

pub struct ColoredSpan<'a> {
    pub text: &'a str,
    pub color: [f32; 4],
}

pub fn parse_color_codes<'a>(text: &'a str, default_color: [f32; 4]) -> Vec<ColoredSpan<'a>> {
    let mut spans = Vec::new();
    let mut current_color = default_color;
    let bytes = text.as_bytes();
    let mut seg_start = 0;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'^' && i + 6 < bytes.len() {
            let hex = &text[i + 1..i + 7];
            if hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                if i > seg_start {
                    spans.push(ColoredSpan {
                        text: &text[seg_start..i],
                        color: current_color,
                    });
                }
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
                current_color = [
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    default_color[3],
                ];
                i += 7;
                seg_start = i;
                continue;
            }
        }
        i += 1;
    }

    if seg_start < bytes.len() {
        spans.push(ColoredSpan {
            text: &text[seg_start..],
            color: current_color,
        });
    }

    spans
}

pub fn strip_color_codes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'^' && i + 6 < bytes.len() {
            let hex = &text[i + 1..i + 7];
            if hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                i += 7;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }

    result
}

pub fn colored_text_vertices(
    text: &str,
    x: f32,
    y: f32,
    default_color: [f32; 4],
    atlas: &FontAtlas,
) -> (Vec<UiVertex>, Vec<u32>) {
    let spans = parse_color_codes(text, default_color);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut cursor_x = x;

    for span in &spans {
        for ch in span.text.chars() {
            let glyph = atlas.glyph(ch);

            if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
                let gx = atlas.snap_to_physical(cursor_x + glyph.offset[0]);
                let gy = atlas.snap_to_physical(y + glyph.offset[1]);

                let base = vertices.len() as u32;
                let (verts, idxs) = quad_vertices_uv(
                    gx,
                    gy,
                    glyph.size[0],
                    glyph.size[1],
                    glyph.uv_min,
                    glyph.uv_max,
                    span.color,
                );
                vertices.extend_from_slice(&verts);
                indices.extend(idxs.iter().map(|i| i + base));
            }

            cursor_x += glyph.advance;
        }
    }

    (vertices, indices)
}

pub fn text_vertices(
    text: &str,
    x: f32,
    y: f32,
    color: [f32; 4],
    atlas: &FontAtlas,
) -> (Vec<UiVertex>, Vec<u32>) {
    text_vertices_clipped(text, x, y, color, atlas, f32::NEG_INFINITY, f32::INFINITY)
}

pub fn text_vertices_scaled(
    text: &str,
    x: f32,
    y: f32,
    color: [f32; 4],
    atlas: &FontAtlas,
    scale: f32,
) -> (Vec<UiVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut cursor_x = x;

    for ch in text.chars() {
        let glyph = atlas.glyph(ch);

        if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
            let gx = atlas.snap_to_physical(cursor_x + glyph.offset[0] * scale);
            let gy = atlas.snap_to_physical(y + glyph.offset[1] * scale);
            let gw = glyph.size[0] * scale;
            let gh = glyph.size[1] * scale;

            let base = vertices.len() as u32;
            let (verts, idxs) = quad_vertices_uv(gx, gy, gw, gh, glyph.uv_min, glyph.uv_max, color);
            vertices.extend_from_slice(&verts);
            indices.extend(idxs.iter().map(|i| i + base));
        }

        cursor_x += glyph.advance * scale;
    }

    (vertices, indices)
}

pub fn text_vertices_clipped(
    text: &str,
    x: f32,
    y: f32,
    color: [f32; 4],
    atlas: &FontAtlas,
    clip_left: f32,
    clip_right: f32,
) -> (Vec<UiVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut cursor_x = x;

    for ch in text.chars() {
        let glyph = atlas.glyph(ch);

        if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
            let gx = atlas.snap_to_physical(cursor_x + glyph.offset[0]);
            let gy = atlas.snap_to_physical(y + glyph.offset[1]);
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
                    draw_x,
                    gy,
                    draw_w,
                    glyph.size[1],
                    [uv_left, glyph.uv_min[1]],
                    [uv_right, glyph.uv_max[1]],
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

pub fn word_wrap(
    text: &str,
    max_width: f32,
    measure: impl Fn(&str) -> f32,
    truncate: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        if truncate {
            let mut current_line = String::new();
            for ch in paragraph.chars() {
                current_line.push(ch);
                if measure(&current_line) > max_width {
                    current_line.pop();
                    if !current_line.is_empty() {
                        lines.push(current_line);
                    }
                    current_line = ch.to_string();
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        } else {
            let words: Vec<&str> = paragraph.split(' ').collect();
            let mut current_line = String::new();
            for word in words {
                if current_line.is_empty() {
                    current_line = word.to_string();
                } else {
                    let candidate = format!("{current_line} {word}");
                    if measure(&candidate) <= max_width {
                        current_line = candidate;
                    } else {
                        lines.push(current_line);
                        current_line = word.to_string();
                    }
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    #[test]
    fn parse_color_codes_single_color() {
        let spans = parse_color_codes("^FF0000Red text", WHITE);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Red text");
        assert!((spans[0].color[0] - 1.0).abs() < 0.01);
        assert!(spans[0].color[1].abs() < 0.01);
    }

    #[test]
    fn parse_color_codes_multiple_colors() {
        let spans = parse_color_codes("Hello ^FF0000Red ^00FF00Green", WHITE);
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "Hello ");
        assert_eq!(spans[0].color, WHITE);
        assert_eq!(spans[1].text, "Red ");
        assert_eq!(spans[2].text, "Green");
        assert!((spans[2].color[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn parse_color_codes_default_prefix() {
        let spans = parse_color_codes("No color codes here", WHITE);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "No color codes here");
        assert_eq!(spans[0].color, WHITE);
    }

    #[test]
    fn parse_color_codes_incomplete_hex_treated_as_literal() {
        let spans = parse_color_codes("^FF00 not enough", WHITE);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "^FF00 not enough");
    }

    #[test]
    fn strip_color_codes_removes_markers() {
        assert_eq!(strip_color_codes("^FF0000Red ^000000Black"), "Red Black");
        assert_eq!(strip_color_codes("No codes"), "No codes");
        assert_eq!(strip_color_codes("^FF00short"), "^FF00short");
    }

    fn char_count_measure(s: &str) -> f32 {
        s.chars().count() as f32
    }

    #[test]
    fn word_wrap_truncate_breaks_long_word() {
        // max_width=5, measure = char count; "abcdefgh" (8 chars) should be split
        let lines = word_wrap("abcdefgh", 5.0, char_count_measure, true);
        assert_eq!(lines, vec!["abcde", "fgh"]);
    }

    #[test]
    fn word_wrap_truncate_mixed_short_and_long() {
        // Truncate treats spaces as regular characters
        let lines = word_wrap("hi abcdefgh ok", 5.0, char_count_measure, true);
        assert_eq!(lines, vec!["hi ab", "cdefg", "h ok"]);
    }

    #[test]
    fn word_wrap_no_truncate_keeps_long_word_intact() {
        let lines = word_wrap("abcdefgh", 5.0, char_count_measure, false);
        assert_eq!(lines, vec!["abcdefgh"]);
    }
}
