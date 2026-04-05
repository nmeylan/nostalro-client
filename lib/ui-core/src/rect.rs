#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    pub fn centered_in(screen_w: f32, screen_h: f32, w: f32, h: f32) -> Self {
        Self {
            x: ((screen_w - w) / 2.0).floor(),
            y: ((screen_h - h) / 2.0).floor(),
            w,
            h,
        }
    }

    /// Compute button rects right-aligned at the bottom of this rect.
    /// Returns rects from right to left: index 0 is the rightmost button.
    /// Matches the RO client layout: BTN_BOTTOM=4, first_right=5, spacing=3.
    pub fn buttons_bottom_right(&self, count: usize, btn_w: f32, btn_h: f32,
                                 btn_bottom: f32, first_right: f32, spacing: f32) -> Vec<Rect> {
        let y = self.y + self.h - btn_bottom - btn_h;
        (0..count).map(|i| {
            let right_offset = first_right + i as f32 * (btn_w + spacing);
            Rect::new(self.x + self.w - right_offset - btn_w, y, btn_w, btn_h)
        }).collect()
    }

    /// Y position for a single line of text vertically centered between
    /// `self.y + top_pad` and `bottom_y` (typically the button row Y).
    pub fn text_dialog_alignment(&self, padding: f32, bottom_y: f32, line_height: f32) -> (f32, f32) {
        let top = self.y + padding;
        let y = top + (bottom_y - top - line_height) / 2.0;
        (y, self.x + padding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_inside() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(r.contains(50.0, 40.0));
    }

    #[test]
    fn contains_boundary() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(r.contains(10.0, 20.0));
        assert!(r.contains(110.0, 70.0));
    }

    #[test]
    fn contains_outside() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(!r.contains(9.0, 40.0));
        assert!(!r.contains(111.0, 40.0));
        assert!(!r.contains(50.0, 19.0));
        assert!(!r.contains(50.0, 71.0));
    }

    #[test]
    fn centered_in_calculation() {
        let r = Rect::centered_in(800.0, 600.0, 200.0, 100.0);
        assert_eq!(r.x, 300.0);
        assert_eq!(r.y, 250.0);
        assert_eq!(r.w, 200.0);
        assert_eq!(r.h, 100.0);
    }
}
