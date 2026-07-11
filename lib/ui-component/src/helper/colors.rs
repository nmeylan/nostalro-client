use ragnarok_ui::frame::UiFrame;

pub fn rgb(hex: u32) -> [f32; 4] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        1.0,
    ]
}

pub const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
pub const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
pub const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
pub const CYAN: [f32; 4] = [0.0, 1.0, 1.0, 1.0];
pub const MAGENTA: [f32; 4] = [0.807_843_1, 0.0, 0.807_843_1, 1.0];
pub const YELLOW: [f32; 4] = [1.0, 1.0, 0.0, 1.0];

/// Per-digit-count `(text_color, shadow_color)`. The shadow is drawn 1px offset so
/// e.g. the 7-digit price reads as black with a green edge rather than solid green.
pub fn price_style(price: i64) -> ([f32; 4], Option<[f32; 4]>) {
    let digits = price.max(0).to_string().len();
    match digits {
        1 => (rgb(0x000000), Some(CYAN)),
        2 => (BLUE, Some(MAGENTA)),
        3 => (BLUE, Some(CYAN)),
        4 => (RED, Some(YELLOW)),
        5 => (rgb(0xff18ff), None),
        6 => (BLUE, None),
        7 => (rgb(0x000000), Some(GREEN)),
        8 => (RED, None),
        9 => (rgb(0x000000), Some(rgb(0xcece63))),
        _ => (RED, Some(rgb(0xff007b))),
    }
}

pub fn draw_price_right(ui: &mut UiFrame, right_x: f32, y: f32, text: &str, price: i64) {
    let (color, shadow) = price_style(price);
    let x = right_x - ui.atlas.measure_text(text);
    if let Some(sh) = shadow {
        ui.text(x + 1.0, y, text, sh);
    }
    ui.text(x, y, text, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::format::format_thousands;

    #[test]
    fn price_style_matches_digit_buckets() {
        assert_eq!(price_style(800_000), ([0.0, 0.0, 1.0, 1.0], None));
        assert_eq!(
            price_style(2_000_000),
            ([0.0, 0.0, 0.0, 1.0], Some([0.0, 1.0, 0.0, 1.0]))
        );
        assert_eq!(price_style(28_000_000), ([1.0, 0.0, 0.0, 1.0], None));
        assert_eq!(format_thousands(28_000_000), "28,000,000");
    }
}
