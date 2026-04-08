use ragnarok_ui::draw::{self, DrawCall, TextureRef};

const SYSBOX_LU: &str = "data/texture/유저인터페이스/sysbox_lu.bmp";
const SYSBOX_MU: &str = "data/texture/유저인터페이스/sysbox_mu.bmp";
const SYSBOX_RU: &str = "data/texture/유저인터페이스/sysbox_ru.bmp";
const SYSBOX_LM: &str = "data/texture/유저인터페이스/sysbox_lm.bmp";
const SYSBOX_MM: &str = "data/texture/유저인터페이스/sysbox_bg.bmp";
const SYSBOX_RM: &str = "data/texture/유저인터페이스/sysbox_rm.bmp";
const SYSBOX_LD: &str = "data/texture/유저인터페이스/sysbox_ld.bmp";
const SYSBOX_MD: &str = "data/texture/유저인터페이스/sysbox_md.bmp";
const SYSBOX_RD: &str = "data/texture/유저인터페이스/sysbox_rd.bmp";

const SYSBOX_TEXTURES: [&str; 9] = [
    SYSBOX_LU, SYSBOX_MU, SYSBOX_RU,
    SYSBOX_LM, SYSBOX_MM, SYSBOX_RM,
    SYSBOX_LD, SYSBOX_MD, SYSBOX_RD,
];

#[derive(Clone, Copy)]
struct NineSliceSizes {
    left_w: f32,
    right_w: f32,
    top_h: f32,
    bottom_h: f32,
}

impl NineSliceSizes {
    fn from_texture_sizes(size_fn: &impl Fn(&str) -> Option<(u32, u32)>) -> Option<Self> {
        let (lw, th) = size_fn(SYSBOX_LU)?;
        let (rw, _) = size_fn(SYSBOX_RU)?;
        let (_, bh) = size_fn(SYSBOX_LD)?;
        Some(Self {
            left_w: lw as f32,
            right_w: rw as f32,
            top_h: th as f32,
            bottom_h: bh as f32,
        })
    }
}

pub struct DialogContainer {
    pub has_grf_textures: bool,
    sysbox_sizes: Option<NineSliceSizes>,
}

impl DialogContainer {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            sysbox_sizes: None,
        }
    }

    pub fn set_texture_sizes(&mut self, size_fn: &impl Fn(&str) -> Option<(u32, u32)>) {
        self.sysbox_sizes = NineSliceSizes::from_texture_sizes(size_fn);
    }

    pub fn copy_sizes_from(&mut self, other: &DialogContainer) {
        self.sysbox_sizes = other.sysbox_sizes;
    }

    pub fn text_color(&self) -> [f32; 4] {
        if self.has_grf_textures {
            [0.0, 0.0, 0.0, 1.0]
        } else {
            [1.0, 1.0, 1.0, 1.0]
        }
    }

    pub fn draw(&self, draw_calls: &mut Vec<DrawCall>, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        if self.has_grf_textures {
            if let Some(sizes) = &self.sysbox_sizes {
                draw_nine_slice(draw_calls, x, y, w, h, sizes, color);
                return;
            }
        }
        draw_fallback(draw_calls, x, y, w, h);
    }

    pub fn grf_texture_paths() -> Vec<&'static str> {
        SYSBOX_TEXTURES.to_vec()
    }
}

fn draw_nine_slice(
    draw_calls: &mut Vec<DrawCall>,
    x: f32, y: f32, w: f32, h: f32,
    sizes: &NineSliceSizes,
    color: [f32; 4],
) {
    let lw = sizes.left_w.floor();
    let rw = sizes.right_w.floor();
    let th = sizes.top_h.floor();
    let bh = sizes.bottom_h.floor();

    // Round to avoid sub-pixel gaps between tiles
    let x = x.floor();
    let y = y.floor();
    let w = w.floor();
    let h = h.floor();

    // Compute boundaries from both edges so adjacent pieces share exact coordinates
    let mid_x = x + lw;
    let right_x = x + w - rw;
    let mid_y = y + th;
    let bot_y = y + h - bh;
    let mw = right_x - mid_x;
    let mh = bot_y - mid_y;

    let right_edge = x + w;
    let bottom_edge = y + h;

    // Row 0: top
    push_bounds(draw_calls, SYSBOX_LU, x, y, mid_x, mid_y, color);
    if mw > 0.0 {
        push_bounds(draw_calls, SYSBOX_MU, mid_x, y, right_x, mid_y, color);
    }
    push_bounds(draw_calls, SYSBOX_RU, right_x, y, right_edge, mid_y, color);

    // Row 1: middle
    if mh > 0.0 {
        push_bounds(draw_calls, SYSBOX_LM, x, mid_y, mid_x, bot_y, color);
        if mw > 0.0 {
            push_bounds(draw_calls, SYSBOX_MM, mid_x, mid_y, right_x, bot_y, color);
        }
        push_bounds(draw_calls, SYSBOX_RM, right_x, mid_y, right_edge, bot_y, color);
    }

    // Row 2: bottom
    push_bounds(draw_calls, SYSBOX_LD, x, bot_y, mid_x, bottom_edge, color);
    if mw > 0.0 {
        push_bounds(draw_calls, SYSBOX_MD, mid_x, bot_y, right_x, bottom_edge, color);
    }
    push_bounds(draw_calls, SYSBOX_RD, right_x, bot_y, right_edge, bottom_edge, color);
}

fn push_bounds(draw_calls: &mut Vec<DrawCall>, texture: &str, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) {
    let (v, i) = draw::quad_from_bounds(x0, y0, x1, y1, color);
    draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::Named(texture.to_string()),
    });
}

fn draw_fallback(draw_calls: &mut Vec<DrawCall>, x: f32, y: f32, w: f32, h: f32) {
    let (v, i) = draw::quad_vertices(x, y, w, h, [0.15, 0.15, 0.22, 0.95]);
    draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
    let border_color = [0.5, 0.5, 0.6, 1.0];
    for (bx, by, bw, bh) in [
        (x, y, w, 1.0),
        (x, y + h - 1.0, w, 1.0),
        (x, y, 1.0, h),
        (x + w - 1.0, y, 1.0, h),
    ] {
        let (v, i) = draw::quad_vertices(bx, by, bw, bh, border_color);
        draw_calls.push(DrawCall {
            vertices: v.to_vec(),
            indices: i.to_vec(),
            texture: TextureRef::White,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_sizes() -> NineSliceSizes {
        NineSliceSizes { left_w: 10.0, right_w: 12.0, top_h: 8.0, bottom_h: 6.0 }
    }

    #[test]
    fn nine_slice_generates_nine_draw_calls() {
        let mut calls = Vec::new();
        draw_nine_slice(&mut calls, 0.0, 0.0, 100.0, 80.0, &test_sizes(), [1.0; 4]);
        assert_eq!(calls.len(), 9);
    }

    #[test]
    fn nine_slice_correct_textures() {
        let mut calls = Vec::new();
        draw_nine_slice(&mut calls, 0.0, 0.0, 100.0, 80.0, &test_sizes(), [1.0; 4]);
        let expected = [
            SYSBOX_LU, SYSBOX_MU, SYSBOX_RU,
            SYSBOX_LM, SYSBOX_MM, SYSBOX_RM,
            SYSBOX_LD, SYSBOX_MD, SYSBOX_RD,
        ];
        for (i, call) in calls.iter().enumerate() {
            match &call.texture {
                TextureRef::Named(name) => assert_eq!(name, expected[i], "piece {i}"),
                _ => panic!("expected Named texture for piece {i}"),
            }
        }
    }

    #[test]
    fn nine_slice_center_geometry() {
        let sizes = test_sizes();
        let mut calls = Vec::new();
        draw_nine_slice(&mut calls, 50.0, 100.0, 200.0, 150.0, &sizes, [1.0; 4]);
        // MM is index 4 (row1, col1)
        let center = &calls[4];
        let min_x = center.vertices[0].position[0];
        let min_y = center.vertices[0].position[1];
        let max_x = center.vertices[3].position[0];
        let max_y = center.vertices[3].position[1];
        assert!((min_x - 60.0).abs() < 0.01); // 50 + 10
        assert!((min_y - 108.0).abs() < 0.01); // 100 + 8
        assert!((max_x - 238.0).abs() < 0.01); // 60 + (200-10-12)
        assert!((max_y - 244.0).abs() < 0.01); // 108 + (150-8-6)
    }

    #[test]
    fn nine_slice_skips_middle_when_too_small() {
        let sizes = NineSliceSizes { left_w: 50.0, right_w: 50.0, top_h: 40.0, bottom_h: 40.0 };
        let mut calls = Vec::new();
        draw_nine_slice(&mut calls, 0.0, 0.0, 100.0, 80.0, &sizes, [1.0; 4]);
        // Middle width = 0 and middle height = 0, so only corners: LU, RU, LD, RD
        assert_eq!(calls.len(), 4);
    }

    #[test]
    fn fallback_draws_background_and_border() {
        let mut calls = Vec::new();
        draw_fallback(&mut calls, 0.0, 0.0, 100.0, 80.0);
        // 1 background + 4 border edges
        assert_eq!(calls.len(), 5);
        for call in &calls {
            assert!(matches!(call.texture, TextureRef::White));
        }
    }

    #[test]
    fn container_uses_fallback_when_no_grf() {
        let container = DialogContainer::new();
        let mut calls = Vec::new();
        container.draw(&mut calls, 0.0, 0.0, 100.0, 80.0, [1.0; 4]);
        assert_eq!(calls.len(), 5); // fallback
    }

    #[test]
    fn container_uses_nine_slice_when_grf() {
        let mut container = DialogContainer::new();
        container.has_grf_textures = true;
        container.sysbox_sizes = Some(test_sizes());
        let mut calls = Vec::new();
        container.draw(&mut calls, 0.0, 0.0, 100.0, 80.0, [1.0; 4]);
        assert_eq!(calls.len(), 9);
    }

    #[test]
    fn from_texture_sizes_returns_none_on_missing() {
        let sizes = NineSliceSizes::from_texture_sizes(&|_| None);
        assert!(sizes.is_none());
    }

    #[test]
    fn from_texture_sizes_extracts_dimensions() {
        let sizes = NineSliceSizes::from_texture_sizes(&|name| {
            match name {
                SYSBOX_LU => Some((12, 10)),
                SYSBOX_RU => Some((14, 10)),
                SYSBOX_LD => Some((12, 8)),
                _ => Some((5, 5)),
            }
        }).unwrap();
        assert_eq!(sizes.left_w, 12.0);
        assert_eq!(sizes.right_w, 14.0);
        assert_eq!(sizes.top_h, 10.0);
        assert_eq!(sizes.bottom_h, 8.0);
    }

    #[test]
    fn text_color_depends_on_grf() {
        let mut container = DialogContainer::new();
        assert_eq!(container.text_color(), [1.0, 1.0, 1.0, 1.0]);
        container.has_grf_textures = true;
        assert_eq!(container.text_color(), [0.0, 0.0, 0.0, 1.0]);
    }
}
