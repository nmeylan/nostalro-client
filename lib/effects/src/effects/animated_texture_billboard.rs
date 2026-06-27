//! Texture-cycling / static camera-facing billboard anchored to the master entity.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAME_MS_60FPS: f32 = 1000.0 / 60.0;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub textures: &'static [&'static str],
    /// Game ticks per texture step.
    pub tcount: u32,
    /// Half-diagonal of the rendered square in world units. Quad side = `distance * √2`.
    pub distance: f32,
    /// World-Y offset relative to the attach position.
    pub delta_y: f32,
    pub alpha: f32,
    /// When true, renders depth-free so the full quad is visible even when the lower
    /// half would be clipped by the ground plane.
    pub no_depth: bool,
}

const TCOUNT_TORCH: u32 = 6;
const TCOUNT_DUST: u32 = 8;
const DEFAULT_DISTANCE: f32 = 20.0;
const DEFAULT_DELTA_Y: f32 = -10.0;
const TORCH_ALPHA: f32 = 130.0 / 255.0;
const DUST_ALPHA: f32 = 100.0 / 255.0;
const GLOW_DISTANCE: f32 = 30.0;
const GLOW_ALPHA: f32 = 50.0 / 255.0;
const GLOW_TCOUNT: u32 = 1;

pub const TORCH_RED_TEXTURES: &[&str] = &[
    "torch_red01.bmp",
    "torch_red02.bmp",
    "torch_red03.bmp",
    "torch_red04.bmp",
    "torch_red05.bmp",
    "torch_red06.bmp",
    "torch_red07.bmp",
    "torch_red08.bmp",
    "torch_red09.bmp",
    "torch_red10.bmp",
    "torch_red11.bmp",
    "torch_red12.bmp",
    "torch_red13.bmp",
];

pub const TORCH_GREEN_TEXTURES: &[&str] = &[
    "torch_green01.bmp",
    "torch_green02.bmp",
    "torch_green03.bmp",
    "torch_green04.bmp",
    "torch_green05.bmp",
    "torch_green06.bmp",
    "torch_green07.bmp",
    "torch_green08.bmp",
    "torch_green09.bmp",
    "torch_green10.bmp",
    "torch_green11.bmp",
    "torch_green12.bmp",
    "torch_green13.bmp",
];

pub const TORCH_VIOLET_TEXTURES: &[&str] = &[
    "torch_violet01.bmp",
    "torch_violet02.bmp",
    "torch_violet03.bmp",
    "torch_violet04.bmp",
    "torch_violet05.bmp",
    "torch_violet06.bmp",
    "torch_violet07.bmp",
    "torch_violet08.bmp",
    "torch_violet09.bmp",
    "torch_violet10.bmp",
    "torch_violet11.bmp",
    "torch_violet12.bmp",
    "torch_violet13.bmp",
];

pub const DUST_TEXTURES: &[&str] = &[
    "dust01.bmp",
    "dust02.bmp",
    "dust03.bmp",
    "dust04.bmp",
    "dust05.bmp",
    "dust06.bmp",
    "dust07.bmp",
    "dust08.bmp",
    "dust09.bmp",
];

pub const TORCH_RED: Params = Params {
    textures: TORCH_RED_TEXTURES,
    tcount: TCOUNT_TORCH,
    distance: DEFAULT_DISTANCE,
    delta_y: DEFAULT_DELTA_Y,
    alpha: TORCH_ALPHA,
    no_depth: false,
};

pub const TORCH_GREEN: Params = Params {
    textures: TORCH_GREEN_TEXTURES,
    tcount: TCOUNT_TORCH,
    distance: DEFAULT_DISTANCE,
    delta_y: DEFAULT_DELTA_Y,
    alpha: TORCH_ALPHA,
    no_depth: false,
};

pub const TORCH_PURPLE: Params = Params {
    textures: TORCH_VIOLET_TEXTURES,
    tcount: TCOUNT_TORCH,
    distance: DEFAULT_DISTANCE,
    delta_y: DEFAULT_DELTA_Y,
    alpha: TORCH_ALPHA,
    no_depth: false,
};

pub const DUST: Params = Params {
    textures: DUST_TEXTURES,
    tcount: TCOUNT_DUST,
    distance: DEFAULT_DISTANCE,
    delta_y: DEFAULT_DELTA_Y,
    alpha: DUST_ALPHA,
    no_depth: false,
};

pub const GLOW_01_TEXTURES: &[&str] = &["glow01.bmp"];
pub const GLOW_02_TEXTURES: &[&str] = &["glow02.bmp"];
pub const GLOW_11_TEXTURES: &[&str] = &["glow11.bmp"];
pub const GLOW_12_TEXTURES: &[&str] = &["glow12.bmp"];

pub const GLOW_01: Params = Params {
    textures: GLOW_01_TEXTURES,
    tcount: GLOW_TCOUNT,
    distance: GLOW_DISTANCE,
    delta_y: 0.0,
    alpha: GLOW_ALPHA,
    no_depth: true,
};

pub const GLOW_02: Params = Params {
    textures: GLOW_02_TEXTURES,
    tcount: GLOW_TCOUNT,
    distance: GLOW_DISTANCE,
    delta_y: 0.0,
    alpha: GLOW_ALPHA,
    no_depth: true,
};

pub const GLOW_11: Params = Params {
    textures: GLOW_11_TEXTURES,
    tcount: GLOW_TCOUNT,
    distance: GLOW_DISTANCE,
    delta_y: 0.0,
    alpha: GLOW_ALPHA,
    no_depth: true,
};

pub const GLOW_12: Params = Params {
    textures: GLOW_12_TEXTURES,
    tcount: GLOW_TCOUNT,
    distance: GLOW_DISTANCE,
    delta_y: 0.0,
    alpha: GLOW_ALPHA,
    no_depth: true,
};

pub const TEXTURES: &[&str] = &[
    "torch_red01.bmp",
    "torch_red02.bmp",
    "torch_red03.bmp",
    "torch_red04.bmp",
    "torch_red05.bmp",
    "torch_red06.bmp",
    "torch_red07.bmp",
    "torch_red08.bmp",
    "torch_red09.bmp",
    "torch_red10.bmp",
    "torch_red11.bmp",
    "torch_red12.bmp",
    "torch_red13.bmp",
    "torch_green01.bmp",
    "torch_green02.bmp",
    "torch_green03.bmp",
    "torch_green04.bmp",
    "torch_green05.bmp",
    "torch_green06.bmp",
    "torch_green07.bmp",
    "torch_green08.bmp",
    "torch_green09.bmp",
    "torch_green10.bmp",
    "torch_green11.bmp",
    "torch_green12.bmp",
    "torch_green13.bmp",
    "torch_violet01.bmp",
    "torch_violet02.bmp",
    "torch_violet03.bmp",
    "torch_violet04.bmp",
    "torch_violet05.bmp",
    "torch_violet06.bmp",
    "torch_violet07.bmp",
    "torch_violet08.bmp",
    "torch_violet09.bmp",
    "torch_violet10.bmp",
    "torch_violet11.bmp",
    "torch_violet12.bmp",
    "torch_violet13.bmp",
    "dust01.bmp",
    "dust02.bmp",
    "dust03.bmp",
    "dust04.bmp",
    "dust05.bmp",
    "dust06.bmp",
    "dust07.bmp",
    "dust08.bmp",
    "dust09.bmp",
    "glow01.bmp",
    "glow02.bmp",
    "glow11.bmp",
    "glow12.bmp",
];

pub struct AnimatedTextureBillboardEffect {
    world_pos: [f32; 3],
    params: Params,
    age: f32,
}

impl AnimatedTextureBillboardEffect {
    pub fn new(world_pos: [f32; 3], params: Params) -> Self {
        Self {
            world_pos,
            params,
            age: 0.0,
        }
    }

    fn texture_index(&self) -> usize {
        let step_ms = self.params.tcount as f32 * FRAME_MS_60FPS;
        let step = (self.age * 1000.0 / step_ms) as usize;
        step % self.params.textures.len()
    }
}

impl Effect for AnimatedTextureBillboardEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let texture = self.params.textures[self.texture_index()];
        let side = self.params.distance * std::f32::consts::SQRT_2;
        let pos = [
            self.world_pos[0],
            self.world_pos[1] + self.params.delta_y,
            self.world_pos[2],
        ];
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let blend = BlendKind::Additive;
        out.push(if self.params.no_depth {
            EffectPrimitiveDraw::BillboardFlash {
                pos,
                size: [side, side],
                uv,
                rotation: 0.0,
                texture,
                color: [1.0, 1.0, 1.0, self.params.alpha],
                blend,
            }
        } else {
            EffectPrimitiveDraw::Billboard {
                pos,
                size: [side, side],
                uv,
                rotation: 0.0,
                texture,
                color: [1.0, 1.0, 1.0, self.params.alpha],
                blend,
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn cycles_through_all_thirteen_textures_in_step_increments() {
        let mut e = AnimatedTextureBillboardEffect::new([0.0; 3], TORCH_RED);
        let mut seen = Vec::new();
        for _ in 0..TORCH_RED_TEXTURES.len() {
            let mut list = EffectDrawList::new();
            e.collect_draws(&mut list, &render_ctx());
            match list.primitives.first() {
                Some(EffectPrimitiveDraw::Billboard { texture, .. }) => {
                    seen.push(*texture);
                }
                other => panic!("expected Billboard, got {:?}", other),
            }
            e.update(&ctx(0.1 + 1e-4));
        }
        assert_eq!(seen, TORCH_RED_TEXTURES);
    }

    #[test]
    fn quad_anchors_below_master_with_distance_sized_side() {
        let e = AnimatedTextureBillboardEffect::new([5.0, 100.0, 7.0], TORCH_GREEN);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let EffectPrimitiveDraw::Billboard {
            pos, size, color, ..
        } = list.primitives[0]
        else {
            panic!("expected Billboard");
        };
        assert_eq!(pos, [5.0, 90.0, 7.0]);
        assert!((size[0] - 20.0 * std::f32::consts::SQRT_2).abs() < 1e-4);
        assert!((color[3] - 130.0 / 255.0).abs() < 1e-4);
    }

    #[test]
    fn texture_lists_have_expected_frame_counts() {
        assert_eq!(TORCH_RED_TEXTURES.len(), 13);
        assert_eq!(TORCH_GREEN_TEXTURES.len(), 13);
        assert_eq!(TORCH_VIOLET_TEXTURES.len(), 13);
        assert_eq!(DUST_TEXTURES.len(), 9);
        assert_eq!(GLOW_01_TEXTURES.len(), 1);
        assert_eq!(GLOW_02_TEXTURES.len(), 1);
        assert_eq!(GLOW_11_TEXTURES.len(), 1);
        assert_eq!(GLOW_12_TEXTURES.len(), 1);
        assert_eq!(TEXTURES.len(), 13 * 3 + 9 + 4);
    }

    #[test]
    fn glow_static_holds_single_texture_across_ticks_with_unit_quad_alpha() {
        let mut e = AnimatedTextureBillboardEffect::new([4.0, 50.0, 9.0], GLOW_01);
        for _ in 0..20 {
            let mut list = EffectDrawList::new();
            e.collect_draws(&mut list, &render_ctx());
            let EffectPrimitiveDraw::BillboardFlash {
                pos,
                size,
                color,
                texture,
                ..
            } = list.primitives[0]
            else {
                panic!("expected BillboardFlash");
            };
            assert_eq!(texture, "glow01.bmp");
            assert_eq!(pos, [4.0, 50.0, 9.0]);
            assert!((size[0] - 30.0 * std::f32::consts::SQRT_2).abs() < 1e-4);
            assert!((color[3] - 50.0 / 255.0).abs() < 1e-4);
            e.update(&ctx(0.05));
        }
    }
}
