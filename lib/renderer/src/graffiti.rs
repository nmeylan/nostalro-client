//! Composes a Graffiti message into a single ground-decal texture by blitting
//! glyphs out of `effect/alpabet.bmp`, an 8x8 grid of 32x32 letters.

use image::RgbaImage;

pub const ALPHABET_TEXTURE: &str = ragnarok_resources::texture::effect::ALPHABET;

/// Side of the composed decal, in pixels.
pub const CANVAS: u32 = 384;
const GLYPH: u32 = 32;
const GRID_COLS: u32 = 8;
const ROW_ADVANCE: u32 = 36;
const LINE_WRAP: u32 = 512;
const WIDE_ADVANCE: u32 = 32;
const NARROW_ADVANCE: u32 = 16;

pub fn texture_key(aid: u32) -> String {
    format!("graffiti/{aid}")
}

/// The glyph slot a character occupies, and how far the pen advances past it.
/// `None` draws nothing but still advances.
fn glyph(c: char) -> (Option<u32>, u32) {
    match c {
        'a'..='z' => (Some(26 + c as u32 - 'a' as u32), WIDE_ADVANCE),
        'A'..='Z' => (Some(c as u32 - 'A' as u32), WIDE_ADVANCE),
        '0'..='9' => (Some(54 + c as u32 - '0' as u32), WIDE_ADVANCE),
        '?' => (Some(53), NARROW_ADVANCE),
        '!' => (Some(54), NARROW_ADVANCE),
        _ => (None, NARROW_ADVANCE),
    }
}

/// Width the next word occupies, used to wrap at a space rather than mid-word.
fn word_width(rest: &[char]) -> u32 {
    rest.iter().take_while(|c| **c != ' ').count() as u32 * WIDE_ADVANCE
}

/// Short messages start further down the decal, so text stays roughly centred
/// whatever its length.
fn first_row_y(char_count: usize) -> u32 {
    (144u32.saturating_sub(char_count as u32) / 24) * ROW_ADVANCE
}

pub fn compose(alphabet: &RgbaImage, message: &str) -> RgbaImage {
    let mut canvas = RgbaImage::new(CANVAS, CANVAS);
    let chars: Vec<char> = message.chars().collect();
    let mut pen_x = 0u32;
    let mut pen_y = first_row_y(chars.len());

    for (i, &c) in chars.iter().enumerate() {
        if c == ' ' {
            if pen_x + word_width(&chars[i + 1..]) >= LINE_WRAP {
                pen_x = 0;
                pen_y += ROW_ADVANCE;
            }
            pen_x += NARROW_ADVANCE;
            continue;
        }
        let (slot, advance) = glyph(c);
        if pen_x + advance >= LINE_WRAP {
            pen_x = 0;
            pen_y += ROW_ADVANCE;
        }
        if let Some(slot) = slot {
            blit(
                alphabet,
                &mut canvas,
                (slot % GRID_COLS) * GLYPH,
                (slot / GRID_COLS) * GLYPH,
                pen_x,
                pen_y,
                advance,
            );
        }
        pen_x += advance;
    }
    canvas
}

/// Blits one 32x32 glyph, horizontally squeezed into `dest_w`. The atlas must
/// already be colour-keyed: transparent source pixels are skipped.
fn blit(
    src: &RgbaImage,
    dst: &mut RgbaImage,
    src_x: u32,
    src_y: u32,
    dst_x: u32,
    dst_y: u32,
    dest_w: u32,
) {
    for row in 0..GLYPH {
        let sy = src_y + row;
        let dy = dst_y + row;
        if sy >= src.height() || dy >= dst.height() {
            continue;
        }
        for col in 0..dest_w {
            let sx = src_x + col * GLYPH / dest_w;
            let dx = dst_x + col;
            if sx >= src.width() || dx >= dst.width() {
                continue;
            }
            let px = src.get_pixel(sx, sy);
            if px[3] == 0 {
                continue;
            }
            dst.put_pixel(dx, dy, *px);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors the real atlas: opaque cells over a magenta background, colour-keyed
    /// by the caller before composing. Every glyph cell is filled with its own slot
    /// index as a colour, so a blit can be traced back to the character it came from.
    fn atlas() -> RgbaImage {
        let mut img = RgbaImage::new(GLYPH * GRID_COLS, GLYPH * GRID_COLS);
        for slot in 0..64u32 {
            let (ox, oy) = ((slot % GRID_COLS) * GLYPH, (slot / GRID_COLS) * GLYPH);
            for y in 0..GLYPH {
                for x in 0..GLYPH {
                    let color = match y {
                        0 => image::Rgba([255, 0, 255, 255]),
                        _ => image::Rgba([slot as u8 + 1, 9, 9, 255]),
                    };
                    img.put_pixel(ox + x, oy + y, color);
                }
            }
        }
        ragnarok_formats::apply_magenta_transparency(img.as_mut());
        img
    }

    #[test]
    fn letters_land_side_by_side_and_unknown_chars_leave_a_gap() {
        let out = compose(&atlas(), "Ab.");
        let y = first_row_y(3) + 1;

        // 'A' is slot 0, 'b' is slot 27; '.' draws nothing.
        assert_eq!(out.get_pixel(0, y)[0], 1);
        assert_eq!(out.get_pixel(WIDE_ADVANCE, y)[0], 28);
        assert_eq!(out.get_pixel(2 * WIDE_ADVANCE, y)[3], 0);
        assert_eq!(
            out.get_pixel(0, y - 1)[3],
            0,
            "keyed-out atlas pixels stay transparent"
        );
    }

    #[test]
    fn a_long_word_wraps_onto_the_next_row() {
        let long = "x".repeat(20);
        let out = compose(&atlas(), &long);
        let y = first_row_y(long.len()) + 1;

        let first_row = (0..CANVAS).any(|x| out.get_pixel(x, y)[3] != 0);
        let second_row = (0..CANVAS).any(|x| out.get_pixel(x, y + ROW_ADVANCE)[3] != 0);
        assert!(first_row && second_row);
    }
}
