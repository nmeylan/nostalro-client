use ab_glyph::{Font, FontRef, ScaleFont};
use std::collections::HashMap;

const FALLBACK_FONT: &[u8] = include_bytes!("fonts/NotoSans-Regular.ttf");
const CJK_FONT_PATH: &str = "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc";

#[derive(Debug, Clone)]
pub struct GlyphInfo {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub offset: [f32; 2],
    pub size: [f32; 2],
    pub advance: f32,
}

pub struct FontAtlas {
    pub image: image::RgbaImage,
    pub glyphs: HashMap<char, GlyphInfo>,
    pub line_height: f32,
    pub ascent: f32,
    pub dpi_scale: f32,
}

impl FontAtlas {
    pub fn from_embedded(px_height: f32, dpi_scale: f32) -> Self {
        Self::build(FALLBACK_FONT, px_height, dpi_scale)
    }

    /// Build a font atlas from the system NotoSansCJK font, falling back to the
    /// embedded font if the CJK font is not available. Includes ASCII plus the
    /// given extra characters (typically collected from GRF file paths).
    pub fn from_system_cjk(px_height: f32, dpi_scale: f32, extra_chars: &[char]) -> Self {
        match std::fs::read(CJK_FONT_PATH) {
            Ok(data) => Self::build_with_extra_chars(&data, px_height, dpi_scale, extra_chars),
            Err(_) => {
                tracing::warn!(
                    "CJK font not found at {CJK_FONT_PATH}, Korean text will not render"
                );
                Self::build(FALLBACK_FONT, px_height, dpi_scale)
            }
        }
    }

    /// Rasterize at physical resolution (`px_height * dpi_scale`) for crispness,
    /// but store all glyph metrics in logical pixels (divided by `dpi_scale`).
    pub fn build(font_data: &[u8], px_height: f32, dpi_scale: f32) -> Self {
        Self::build_with_extra_chars(font_data, px_height, dpi_scale, &[])
    }

    pub fn build_with_extra_chars(
        font_data: &[u8],
        px_height: f32,
        dpi_scale: f32,
        extra_chars: &[char],
    ) -> Self {
        let physical_height = px_height * dpi_scale;
        let font = FontRef::try_from_slice(font_data).expect("invalid font data");
        let scaled = font.as_scaled(physical_height);

        let line_height = (scaled.height() + scaled.line_gap()) / dpi_scale;
        let ascent = scaled.ascent() / dpi_scale;

        let mut chars: Vec<char> = (32u8..127).map(|b| b as char).collect();
        for &ch in extra_chars {
            if !chars.contains(&ch) && !ch.is_control() {
                chars.push(ch);
            }
        }
        let mut glyph_renders: Vec<(
            char,
            ab_glyph::GlyphId,
            Option<ab_glyph::OutlinedGlyph>,
            f32,
        )> = Vec::new();

        for &ch in &chars {
            let glyph_id = font.glyph_id(ch);
            let glyph =
                glyph_id.with_scale_and_position(physical_height, ab_glyph::point(0.0, 0.0));
            let outlined = font.outline_glyph(glyph);
            let advance = scaled.h_advance(glyph_id);
            glyph_renders.push((ch, glyph_id, outlined, advance));
        }

        // Shelf-pack: estimate atlas size
        let padding = 1;
        let max_glyph_width = glyph_renders
            .iter()
            .filter_map(|(_, _, og, _)| og.as_ref().map(|g| g.px_bounds().width() as u32 + padding))
            .max()
            .unwrap_or(1);
        let total_area: u32 = glyph_renders
            .iter()
            .filter_map(|(_, _, og, _)| {
                og.as_ref().map(|g| {
                    let b = g.px_bounds();
                    (b.width() as u32 + padding) * (b.height() as u32 + padding)
                })
            })
            .sum();

        // 1.5x margin accounts for shelf-packing row waste
        let mut atlas_size = ((total_area as f64 * 1.5).sqrt().ceil() as u32)
            .next_power_of_two()
            .max(64);
        if atlas_size < max_glyph_width + padding {
            atlas_size = (max_glyph_width + padding).next_power_of_two();
        }

        let mut image = image::RgbaImage::new(atlas_size, atlas_size);
        let mut glyphs_map = HashMap::new();

        // Shelf packing
        let mut cursor_x = 0u32;
        let mut cursor_y = 0u32;
        let mut row_height = 0u32;

        for (ch, _glyph_id, outlined, advance) in &glyph_renders {
            if let Some(og) = outlined {
                let bounds = og.px_bounds();
                let gw = bounds.width() as u32;
                let gh = bounds.height() as u32;

                if cursor_x + gw + padding > atlas_size {
                    cursor_x = 0;
                    cursor_y += row_height + padding;
                    row_height = 0;
                }

                if cursor_y + gh + padding > atlas_size {
                    // Atlas too small - this shouldn't happen with our estimate
                    break;
                }

                let ox = cursor_x;
                let oy = cursor_y;

                og.draw(|x, y, c| {
                    let px = ox + x;
                    let py = oy + y;
                    if px < atlas_size && py < atlas_size {
                        let c = c.clamp(0.0, 1.0);
                        let alpha = 2.0 * c - c * c;
                        let v = (alpha * 255.0) as u8;
                        image.put_pixel(px, py, image::Rgba([255, 255, 255, v]));
                    }
                });

                let inv = 1.0 / atlas_size as f32;
                glyphs_map.insert(
                    *ch,
                    GlyphInfo {
                        uv_min: [ox as f32 * inv, oy as f32 * inv],
                        uv_max: [(ox + gw) as f32 * inv, (oy + gh) as f32 * inv],
                        offset: [bounds.min.x / dpi_scale, bounds.min.y / dpi_scale],
                        size: [gw as f32 / dpi_scale, gh as f32 / dpi_scale],
                        advance: *advance / dpi_scale,
                    },
                );

                cursor_x += gw + padding;
                row_height = row_height.max(gh);
            } else {
                // Whitespace glyph
                glyphs_map.insert(
                    *ch,
                    GlyphInfo {
                        uv_min: [0.0, 0.0],
                        uv_max: [0.0, 0.0],
                        offset: [0.0, 0.0],
                        size: [0.0, 0.0],
                        advance: *advance / dpi_scale,
                    },
                );
            }
        }

        Self {
            image,
            glyphs: glyphs_map,
            line_height,
            ascent,
            dpi_scale,
        }
    }

    #[inline]
    pub fn snap_to_physical(&self, logical: f32) -> f32 {
        (logical * self.dpi_scale).round() / self.dpi_scale
    }

    pub fn glyph(&self, ch: char) -> &GlyphInfo {
        self.glyphs
            .get(&ch)
            .unwrap_or_else(|| self.glyphs.get(&'?').unwrap())
    }

    pub fn measure_text(&self, text: &str) -> f32 {
        text.chars().map(|ch| self.glyph(ch).advance).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas() -> FontAtlas {
        FontAtlas::from_embedded(20.0, 1.0)
    }

    #[test]
    fn atlas_contains_all_ascii_printable() {
        let a = atlas();
        for b in 32u8..127 {
            assert!(
                a.glyphs.contains_key(&(b as char)),
                "missing char {}",
                b as char
            );
        }
    }

    #[test]
    fn uvs_within_bounds() {
        let a = atlas();
        for info in a.glyphs.values() {
            assert!(info.uv_min[0] >= 0.0 && info.uv_min[0] <= 1.0);
            assert!(info.uv_min[1] >= 0.0 && info.uv_min[1] <= 1.0);
            assert!(info.uv_max[0] >= 0.0 && info.uv_max[0] <= 1.0);
            assert!(info.uv_max[1] >= 0.0 && info.uv_max[1] <= 1.0);
        }
    }

    #[test]
    fn measure_text_consistency() {
        let a = atlas();
        let w1 = a.measure_text("A");
        let w2 = a.measure_text("AA");
        assert!((w2 - w1 * 2.0).abs() < 0.01);
    }

    #[test]
    fn unknown_char_falls_back_to_question_mark() {
        let a = atlas();
        let g = a.glyph('\u{4e00}');
        let q = a.glyph('?');
        assert_eq!(g.advance, q.advance);
    }
}
