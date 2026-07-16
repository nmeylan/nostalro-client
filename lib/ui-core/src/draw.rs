use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::ui_renderer::UiVertex;
pub use ragnarok_renderer::texture::EMOTION_ICON_PREFIX;
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
    let corners = [(-half, -half), (half, -half), (-half, half), (half, half)];
    let verts = [
        UiVertex {
            position: [
                cx + corners[0].0 * cos - corners[0].1 * sin,
                cy + corners[0].0 * sin + corners[0].1 * cos,
            ],
            tex_coord: [0.0, 0.0],
            color,
        },
        UiVertex {
            position: [
                cx + corners[1].0 * cos - corners[1].1 * sin,
                cy + corners[1].0 * sin + corners[1].1 * cos,
            ],
            tex_coord: [1.0, 0.0],
            color,
        },
        UiVertex {
            position: [
                cx + corners[2].0 * cos - corners[2].1 * sin,
                cy + corners[2].0 * sin + corners[2].1 * cos,
            ],
            tex_coord: [0.0, 1.0],
            color,
        },
        UiVertex {
            position: [
                cx + corners[3].0 * cos - corners[3].1 * sin,
                cy + corners[3].0 * sin + corners[3].1 * cos,
            ],
            tex_coord: [1.0, 1.0],
            color,
        },
    ];
    let indices = [0, 1, 2, 2, 1, 3];
    (verts, indices)
}

pub fn quad_vertices_vgrad(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    top: [f32; 4],
    bot: [f32; 4],
) -> ([UiVertex; 4], [u32; 6]) {
    let verts = [
        UiVertex {
            position: [x, y],
            tex_coord: [0.0, 0.0],
            color: top,
        },
        UiVertex {
            position: [x + w, y],
            tex_coord: [1.0, 0.0],
            color: top,
        },
        UiVertex {
            position: [x, y + h],
            tex_coord: [0.0, 1.0],
            color: bot,
        },
        UiVertex {
            position: [x + w, y + h],
            tex_coord: [1.0, 1.0],
            color: bot,
        },
    ];
    (verts, [0, 1, 2, 2, 1, 3])
}

pub fn rounded_rect(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: [f32; 4],
) -> (Vec<UiVertex>, Vec<u32>) {
    rounded_rect_vgrad(x, y, w, h, radius, color, color)
}

/// Rounded rectangle as a center-fan triangle mesh, vertex color lerped from
/// `top` to `bot` by vertical position. Corners are absent geometry (background
/// shows through), matching the color-keyed rounding of the real interface art.
pub fn rounded_rect_vgrad(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    top: [f32; 4],
    bot: [f32; 4],
) -> (Vec<UiVertex>, Vec<u32>) {
    rounded_rect_corners_vgrad(x, y, w, h, [radius; 4], top, bot)
}

/// Like `rounded_rect_vgrad` but with independent corner radii, ordered
/// `[top-left, top-right, bottom-right, bottom-left]`. Lets stacked window
/// chrome round only its outward corners (title = top pair, footer = bottom pair).
pub fn rounded_rect_corners_vgrad(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radii: [f32; 4],
    top: [f32; 4],
    bot: [f32; 4],
) -> (Vec<UiVertex>, Vec<u32>) {
    use std::f32::consts::{FRAC_PI_2, PI};
    const SEG: usize = 3;
    let cap = w.min(h) * 0.5;
    let r = radii.map(|v| v.max(0.0).min(cap));

    let corners = [
        (x + r[0], y + r[0], PI, r[0]),
        (x + w - r[1], y + r[1], PI + FRAC_PI_2, r[1]),
        (x + w - r[2], y + h - r[2], 2.0 * PI, r[2]),
        (x + r[3], y + h - r[3], 2.0 * PI + FRAC_PI_2, r[3]),
    ];
    let mut perim: Vec<[f32; 2]> = Vec::with_capacity(4 * (SEG + 1));
    for (ccx, ccy, start, r) in corners {
        for s in 0..=SEG {
            let a = start + FRAC_PI_2 * (s as f32 / SEG as f32);
            let p = [ccx + r * a.cos(), ccy + r * a.sin()];
            let dup = perim
                .last()
                .is_some_and(|l| (l[0] - p[0]).abs() <= 1e-4 && (l[1] - p[1]).abs() <= 1e-4);
            if !dup {
                perim.push(p);
            }
        }
    }
    if perim.len() > 1 {
        let (f, l) = (perim[0], *perim.last().unwrap());
        if (f[0] - l[0]).abs() <= 1e-4 && (f[1] - l[1]).abs() <= 1e-4 {
            perim.pop();
        }
    }

    let col_at = |py: f32| {
        let t = if h > 0.0 {
            ((py - y) / h).clamp(0.0, 1.0)
        } else {
            0.0
        };
        [
            top[0] + (bot[0] - top[0]) * t,
            top[1] + (bot[1] - top[1]) * t,
            top[2] + (bot[2] - top[2]) * t,
            top[3] + (bot[3] - top[3]) * t,
        ]
    };

    let (cx, cy) = (x + w * 0.5, y + h * 0.5);
    let mut verts = Vec::with_capacity(perim.len() + 1);
    verts.push(UiVertex {
        position: [cx, cy],
        tex_coord: [0.5, 0.5],
        color: col_at(cy),
    });
    for p in &perim {
        verts.push(UiVertex {
            position: *p,
            tex_coord: [0.0, 0.0],
            color: col_at(p[1]),
        });
    }
    let n = perim.len() as u32;
    let mut indices = Vec::with_capacity(perim.len() * 3);
    for i in 0..n {
        indices.push(0);
        indices.push(1 + i);
        indices.push(1 + (i + 1) % n);
    }
    (verts, indices)
}

pub fn square_wedge_vertices(
    cx: f32,
    cy: f32,
    half: f32,
    start_rad: f32,
    sweep_rad: f32,
    color: [f32; 4],
) -> (Vec<UiVertex>, Vec<u32>) {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, TAU};
    let end_rad = start_rad + sweep_rad;
    // Sample angles: the endpoints plus any square-corner direction strictly
    // between them. The square boundary is straight between corners, so the
    // projection of any in-between angle is collinear — corners are all we need.
    let mut angles = vec![start_rad, end_rad];
    for k in 0..4 {
        let base = FRAC_PI_4 + k as f32 * FRAC_PI_2;
        for n in -1..=1 {
            let a = base + n as f32 * TAU;
            if a > start_rad && a < end_rad {
                angles.push(a);
            }
        }
    }
    angles.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut verts = Vec::with_capacity(angles.len() + 1);
    let mut indices = Vec::with_capacity(angles.len() * 3);
    verts.push(UiVertex {
        position: [cx, cy],
        tex_coord: [0.5, 0.5],
        color,
    });
    for a in &angles {
        let (c, s) = (a.cos(), a.sin());
        let m = c.abs().max(s.abs()).max(f32::EPSILON);
        verts.push(UiVertex {
            position: [cx + half * c / m, cy + half * s / m],
            tex_coord: [0.0, 0.0],
            color,
        });
    }
    for i in 1..verts.len() as u32 - 1 {
        indices.push(0);
        indices.push(i);
        indices.push(i + 1);
    }
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

            if gx_right > clip_left && gx < clip_right {
                let mut draw_x = gx;
                let mut draw_w = glyph.size[0];
                let mut uv_left = glyph.uv_min[0];
                let mut uv_right = glyph.uv_max[0];
                let uv_span = uv_right - uv_left;
                let px_span = glyph.size[0];

                if draw_x < clip_left {
                    let clipped = clip_left - draw_x;
                    uv_left += uv_span * (clipped / px_span);
                    draw_w -= clipped;
                    draw_x = clip_left;
                }
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

    #[test]
    fn quad_vgrad_carries_top_and_bottom_colors() {
        let top = [1.0, 0.0, 0.0, 1.0];
        let bot = [0.0, 0.0, 1.0, 1.0];
        let (v, _) = quad_vertices_vgrad(0.0, 0.0, 10.0, 10.0, top, bot);
        assert_eq!([v[0].color, v[1].color], [top, top]);
        assert_eq!([v[2].color, v[3].color], [bot, bot]);
    }

    #[test]
    fn rounded_rect_is_valid_triangle_fan_with_clamped_radius() {
        let (v, i) = rounded_rect(0.0, 0.0, 20.0, 10.0, 100.0, WHITE);
        assert!(!i.is_empty() && i.len() % 3 == 0);
        assert!(i.iter().all(|&idx| (idx as usize) < v.len()));
        for vert in &v {
            assert!((-0.01..=20.01).contains(&vert.position[0]));
            assert!((-0.01..=10.01).contains(&vert.position[1]));
        }
    }

    #[test]
    fn per_corner_radii_keep_sharp_corners_and_round_others() {
        // footer style: sharp top (TL,TR = 0), rounded bottom (BR,BL = 4)
        let (v, _) = rounded_rect_corners_vgrad(0.0, 0.0, 20.0, 10.0, [0.0, 0.0, 4.0, 4.0], WHITE, WHITE);
        let has = |x: f32, y: f32| {
            v.iter()
                .any(|vt| (vt.position[0] - x).abs() < 1e-3 && (vt.position[1] - y).abs() < 1e-3)
        };
        assert!(has(0.0, 0.0) && has(20.0, 0.0), "top corners stay sharp");
        assert!(!has(0.0, 10.0) && !has(20.0, 10.0), "bottom corners are rounded away");
    }

    fn char_count_measure(s: &str) -> f32 {
        s.chars().count() as f32
    }

    #[test]
    fn word_wrap_truncate_breaks_long_word() {
        let lines = word_wrap("abcdefgh", 5.0, char_count_measure, true);
        assert_eq!(lines, vec!["abcde", "fgh"]);
    }

    #[test]
    fn word_wrap_truncate_mixed_short_and_long() {
        let lines = word_wrap("hi abcdefgh ok", 5.0, char_count_measure, true);
        assert_eq!(lines, vec!["hi ab", "cdefg", "h ok"]);
    }

    #[test]
    fn word_wrap_no_truncate_keeps_long_word_intact() {
        let lines = word_wrap("abcdefgh", 5.0, char_count_measure, false);
        assert_eq!(lines, vec!["abcdefgh"]);
    }
}
