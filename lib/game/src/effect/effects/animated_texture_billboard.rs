//! Animated effect-texture — camera-facing billboard whose texture cycles
//! through a fixed list every N game ticks. Drives the Torch recolour
//! family (`TorchRed`, `TorchGreen`, `TorchPurple`) and would also fit any
//! similar `EffectTextureSet_Animation(F1)` variant.
//!
//! Cadence and geometry:
//!
//!
//! - 13 `.bmp` textures held in the list.
//! - The current texture index advances every `tcount` game ticks.
//!   `tcount` is 8 for the
//!   Dust variant and 6 for the three Torch variants.
//! - The quad is anchored 10 units below the master entity's Y —
//!   slightly below the master entity.
//! - `distance = 20` — radius from which the four corner vertices
//!   are projected (90° spacing → square quad with side
//!   `distance * sqrt(2) ≈ 28.3`).
//! - `alpha = 130` for the Torch variants (≈ 0.51 in RGBA).
//! - No sin-table radius oscillation, so the
//!   distance is constant.
//! - Initial 135° roll around the view
//!   axis. The renderer's billboard is axis-aligned, so this roll isn't
//!   reproduced; the visual still reads correctly for square-aspect
//!   torches because the textures are roughly symmetric.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

const FRAME_MS_60FPS: f32 = 1000.0 / 60.0;

/// Per-id texture-ani recipe.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Bare GRF filenames (no `data/texture/effect/` prefix). The renderer
    /// prepends that prefix in its `texture_lookup`. Length matters only
    /// for the modulo at frame-cycle time; 13 matches every animated
    /// torch/dust variant.
    pub textures: &'static [&'static str],
    /// Game ticks per texture step (tcount in the original game). 6 → 100 ms/step at
    /// 60 fps.
    pub tcount: u32,
    /// Half-diagonal of the rendered square in world
    /// units. The quad side ends up `distance * √2`.
    pub distance: f32,
    /// World-Y offset relative to the attach position.
    /// Negative values move the quad down (native RO coords).
    pub delta_y: f32,
    /// Quad opacity (base alpha / 255) mapped to 0..1.
    pub alpha: f32,
}

const TCOUNT_TORCH: u32 = 6;
const TCOUNT_DUST: u32 = 8;
const DEFAULT_DISTANCE: f32 = 20.0;
const DEFAULT_DELTA_Y: f32 = -10.0;
const TORCH_ALPHA: f32 = 130.0 / 255.0;
const DUST_ALPHA: f32 = 100.0 / 255.0;

/// 13 frames of the red torch flame.
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

/// 9 frames of the ambient dust mote — `EffectTextureSet_Animation(0)` in
/// the original game. tcount differs (8 vs 6) because the prim's update reads
/// `flag1[4] == 0` to pick the slower cadence.
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
};

pub const TORCH_GREEN: Params = Params {
    textures: TORCH_GREEN_TEXTURES,
    tcount: TCOUNT_TORCH,
    distance: DEFAULT_DISTANCE,
    delta_y: DEFAULT_DELTA_Y,
    alpha: TORCH_ALPHA,
};

pub const TORCH_PURPLE: Params = Params {
    textures: TORCH_VIOLET_TEXTURES,
    tcount: TCOUNT_TORCH,
    distance: DEFAULT_DISTANCE,
    delta_y: DEFAULT_DELTA_Y,
    alpha: TORCH_ALPHA,
};

pub const DUST: Params = Params {
    textures: DUST_TEXTURES,
    tcount: TCOUNT_DUST,
    distance: DEFAULT_DISTANCE,
    delta_y: DEFAULT_DELTA_Y,
    alpha: DUST_ALPHA,
};

/// Concatenated texture list for `effect::effect_texture_paths` preload.
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
];

pub struct AnimatedTextureBillboardEffect {
    world_pos: [f32; 3],
    params: Params,
    age: f32,
}

impl AnimatedTextureBillboardEffect {
    pub fn new(attach: Attach, params: Params) -> Self {
        let world_pos = match attach {
            Attach::WorldPos(p) => p,
            Attach::Entity(_) | Attach::Projectile { .. } => [0.0; 3],
        };
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
        // Ambient: never self-terminates; the holder kills it when
        // duration_ms (infinite for the Torch family) elapses.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let texture = self.params.textures[self.texture_index()];
        // Quad side = distance * √2 (corners spaced 90° on a circle of
        // radius `distance`).
        let side = self.params.distance * std::f32::consts::SQRT_2;
        out.push(EffectPrimitiveDraw::Billboard {
            pos: [
                self.world_pos[0],
                self.world_pos[1] + self.params.delta_y,
                self.world_pos[2],
            ],
            size: [side, side],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            texture,
            color: [1.0, 1.0, 1.0, self.params.alpha],
            blend: BlendKind::Alpha,
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
        // Sociable test: drive an effect for one full cycle and confirm
        // the emitted Billboard's texture name cycles through every entry
        // in the texture list at the configured cadence (tcount=6 ticks =
        // 100 ms per step).
        let mut e = AnimatedTextureBillboardEffect::new(Attach::WorldPos([0.0; 3]), TORCH_RED);
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
            // Advance one full step (100 ms + a sliver to avoid landing
            // exactly on a step boundary).
            e.update(&ctx(0.1 + 1e-4));
        }
        assert_eq!(seen, TORCH_RED_TEXTURES);
    }

    #[test]
    fn quad_anchors_below_master_with_distance_sized_side() {
        // Spawn at a known world position; one render should put the
        // billboard at world_y - 10 with side √2 × distance ≈ 28.28.
        let e =
            AnimatedTextureBillboardEffect::new(Attach::WorldPos([5.0, 100.0, 7.0]), TORCH_GREEN);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let EffectPrimitiveDraw::Billboard { pos, size, color, .. } = list.primitives[0]
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
        // Concatenated preload list = sum of all variants.
        assert_eq!(TEXTURES.len(), 13 * 3 + 9);
    }
}
