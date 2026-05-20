//! Bottom_Vertical family — vertical "curtain" strips anchored at two
//! ground points and rising to `max_height`.
//!
//! Original game dispatcher `Bottom_Vertical(texture, F1)` builds a
//! bottom-2 primitive with up to 4 cells. The bottom-2 render
//! emits each cell as a single quad whose corners are:
//!   * `vec1 = (vecB_now, -max_height)` (top, "now" side)
//!   * `vec2 = (vecB_now, 0)`           (bottom, "now")
//!   * `vec3 = (vecB_pre, 0)`           (bottom, "pre" side)
//!   * `vec4 = (vecB_pre, -max_height)` (top, "pre")
//!
//! Native RO `-Y` = up so `-max_height` is the top. The two ground
//! points define the strip's orientation in the XZ plane; height is
//! constant. Per `height[0]` value the bottom render picks a colour and
//! blend mode.
//!
//! F1 → cell layout (from `Bottom_Vertical` lines 17108-17182):
//!
//! | F1 | label              | cells | layout (XZ) | alpha base | height[0] |
//! |----|--------------------|-------|-------------|--------|-----------|
//! | 0  | Dissonance/Uglydance | 4   | `(rand±20, rand±20)` → `(0, 0)` | 15  | (unset = default white alpha) |
//! | 1  | Assassincross      | 4     | crossed cross-of-strips through origin | 50 | 1 → pink (255,155,155) additive |
//! | 2  | Dontforgetme       | 4     | crossed cross-of-strips through origin | 50 | 2 → green (155,255,155) additive |
//! | 3  | (Browserbased)     | 2     | one X-strip + one Z-strip through origin | 100 | 3 → yellow (255,255,100) alpha |
//! | 4  | Serviceforyou      | 2     | one X-strip + one Z-strip through origin | 100 | 4 → pink (255,150,150) alpha |
//!
//! `max_height` isn't set explicitly in the bottom-vertical song — the original
//! default for the bottom-2 primitive is whatever the primitive constructor
//! leaves it at. We hand-pick `STRIP_HEIGHT = 10.0` as a visible-but-
//! restrained default (about player-height); tune later if the gif
//! references disagree.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

#[derive(Clone, Copy, Debug)]
pub struct BottomVerticalParams {
    pub texture: &'static str,
    /// per-element base alpha (out of 255).
    pub base_alpha: f32,
    /// RGB tint selected per variant (0..1).
    pub tint_rgb: [f32; 3],
    /// the textured-rect render's blend arg — 0 = alpha blend, 1 = additive.
    pub blend: BlendKind,
    /// Cell layout pattern. Maps F1 from the original game to the
    /// per-strip placement formula in `Bottom_Vertical`.
    pub layout: StripLayout,
}

#[derive(Clone, Copy, Debug)]
pub enum StripLayout {
    /// F1=0: 4 strips, each from a per-strip random XZ offset (±20) to
    /// the origin. Forms radial lines converging on the actor.
    RadialConverge,
    /// F1=1/2: 4 strips arranged as two perpendicular pairs (forming
    /// a # cross), each spanning ±3 with a random endpoint jitter.
    CrossedQuartet,
    /// F1=3/4: 2 perpendicular strips through the origin — one X-axis,
    /// one Z-axis, both ±6 long.
    PerpendicularPair,
}

const FRAMES_PER_SECOND: f32 = 60.0;
const FADE_IN_FRAMES: f32 = 15.0;
const FADE_IN_SECS: f32 = FADE_IN_FRAMES / FRAMES_PER_SECOND;
/// Strip vertical extent in world units (native RO `-Y` = up, so the
/// strip top sits at `actor.y - STRIP_HEIGHT`). Default hand-picked
/// since the original game's `Bottom_Vertical` never assigns `max_height` — see module
/// docs.
const STRIP_HEIGHT: f32 = 10.0;

/// `EF_BOTTOM_DISSONANCE` → `Bottom_Vertical("ring_blue.tga")`.
/// F1=0 default-case → no `height[0]` set, the bottom2 render falls
/// through to the default white-alpha branch.
pub const DISSONANCE: BottomVerticalParams = BottomVerticalParams {
    texture: "ring_blue.tga",
    base_alpha: 15.0 / 255.0,
    tint_rgb: [1.0, 1.0, 1.0],
    blend: BlendKind::Alpha,
    layout: StripLayout::RadialConverge,
};

/// `EF_BOTTOM_UGLYDANCE` → `Bottom_Vertical("ring_red.tga")`. Same F1=0
/// shape as Dissonance with a red texture instead of blue.
pub const UGLYDANCE: BottomVerticalParams = BottomVerticalParams {
    texture: "ring_red.tga",
    base_alpha: 15.0 / 255.0,
    tint_rgb: [1.0, 1.0, 1.0],
    blend: BlendKind::Alpha,
    layout: StripLayout::RadialConverge,
};

/// `EF_BOTTOM_ASSASSINCROSS` → `Bottom_Vertical("ring_red.tga", 1)`.
/// F1=1: cross of 4 strips, additive pink tint (255,155,155).
pub const ASSASSINCROSS: BottomVerticalParams = BottomVerticalParams {
    texture: "ring_red.tga",
    base_alpha: 50.0 / 255.0,
    tint_rgb: [255.0 / 255.0, 155.0 / 255.0, 155.0 / 255.0],
    blend: BlendKind::Additive,
    layout: StripLayout::CrossedQuartet,
};

/// `EF_BOTTOM_DONTFORGETME` → `Bottom_Vertical("magic_green.tga", 2)`.
/// F1=2: cross of 4 strips, additive green tint (155,255,155).
pub const DONTFORGETME: BottomVerticalParams = BottomVerticalParams {
    texture: "magic_green.tga",
    base_alpha: 50.0 / 255.0,
    tint_rgb: [155.0 / 255.0, 255.0 / 255.0, 155.0 / 255.0],
    blend: BlendKind::Additive,
    layout: StripLayout::CrossedQuartet,
};

/// `EF_BOTTOM_SERVICEFORYOU` → `Bottom_Vertical("safeline.bmp", 4)`.
/// F1=4: 2 perpendicular strips, alpha-blend pink (255,150,150).
pub const SERVICEFORYOU: BottomVerticalParams = BottomVerticalParams {
    texture: "safeline.bmp",
    base_alpha: 100.0 / 255.0,
    tint_rgb: [255.0 / 255.0, 150.0 / 255.0, 150.0 / 255.0],
    blend: BlendKind::Alpha,
    layout: StripLayout::PerpendicularPair,
};

pub const TEXTURES: &[&str] = &[
    "ring_blue.tga",
    "ring_red.tga",
    "magic_green.tga",
    "safeline.bmp",
];

pub struct BottomVerticalEffect {
    world_pos: [f32; 3],
    params: BottomVerticalParams,
    age: f32,
    /// Per-strip endpoint offsets, frozen at spawn. Up to 4 entries; cells
    /// beyond `strip_count` are unused. Each entry is `(pre_xz, now_xz)`
    /// — the two ground anchors of one strip.
    strips: [Strip; 4],
    strip_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct Strip {
    pre: [f32; 2],
    now: [f32; 2],
}

impl BottomVerticalEffect {
    pub fn new(attach: Attach, params: BottomVerticalParams) -> Self {
        let world_pos = match attach {
            Attach::WorldPos(p) => p,
            Attach::Entity(_) | Attach::Projectile { .. } | Attach::Trail { .. } => [0.0; 3],
        };
        let (strips, strip_count) = build_strips(params.layout, &world_pos);
        Self {
            world_pos,
            params,
            age: 0.0,
            strips,
            strip_count,
        }
    }
}

impl Effect for BottomVerticalEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let fade = (self.age / FADE_IN_SECS).clamp(0.0, 1.0);
        let alpha = self.params.base_alpha * fade;
        let [tr, tg, tb] = self.params.tint_rgb;
        // Strip corners (CCW from the +Y-up "front" face viewer
        // perspective): bottom-now, bottom-pre, top-pre, top-now.
        // Standard UV mapping fills the texture across the quad.
        let uv = [
            [1.0, 1.0],
            [0.0, 1.0],
            [0.0, 0.0],
            [1.0, 0.0],
        ];
        let base_y = self.world_pos[1];
        let top_y = base_y - STRIP_HEIGHT;
        for s in &self.strips[..self.strip_count] {
            let now_x = self.world_pos[0] + s.now[0];
            let now_z = self.world_pos[2] + s.now[1];
            let pre_x = self.world_pos[0] + s.pre[0];
            let pre_z = self.world_pos[2] + s.pre[1];
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners: [
                    [now_x, base_y, now_z],
                    [pre_x, base_y, pre_z],
                    [pre_x, top_y, pre_z],
                    [now_x, top_y, now_z],
                ],
                uv,
                texture: self.params.texture,
                color: [tr, tg, tb, alpha],
                blend: self.params.blend,
            });
        }
    }
}

/// Build the per-strip anchor table for a given layout. Random values
/// in the original game's bottom-vertical song are reproduced via position-hashed pseudo-
/// random numbers so the same spawn looks identical across rerenders.
fn build_strips(layout: StripLayout, world_pos: &[f32; 3]) -> ([Strip; 4], usize) {
    let seed = position_hash(world_pos);
    let mut strips = [Strip {
        pre: [0.0; 2],
        now: [0.0; 2],
    }; 4];
    let count = match layout {
        // F1=0: `vecB_pre = (random(40)-20, random(40)-20)`,
        // `vecB_now = (0, 0)`.
        StripLayout::RadialConverge => {
            for (i, s) in strips.iter_mut().enumerate() {
                let rx = rand_in_range(seed, i as u64 * 4, -20.0, 20.0);
                let rz = rand_in_range(seed, i as u64 * 4 + 1, -20.0, 20.0);
                s.pre = [rx, rz];
                s.now = [0.0, 0.0];
            }
            4
        }
        // F1=1/2: 4 strips making a `#` cross.
        // ec<2: `pre = (random(7)-3, -3)`, `now = (-pre.x, 3)`.
        // ec≥2: `pre = (-3, random(7)-3)`, `now = (3, -pre.z)`.
        StripLayout::CrossedQuartet => {
            for (i, s) in strips.iter_mut().enumerate() {
                if i < 2 {
                    let rx = rand_in_range(seed, i as u64 * 4, -3.0, 4.0);
                    s.pre = [rx, -3.0];
                    s.now = [-rx, 3.0];
                } else {
                    let rz = rand_in_range(seed, i as u64 * 4, -3.0, 4.0);
                    s.pre = [-3.0, rz];
                    s.now = [3.0, -rz];
                }
            }
            4
        }
        // F1=3/4: 2 perpendicular strips through origin.
        // ec=0: pre = (6, 0), now = (-6, 0). ec=1: pre = (0, -6), now = (0, 6).
        StripLayout::PerpendicularPair => {
            strips[0].pre = [6.0, 0.0];
            strips[0].now = [-6.0, 0.0];
            strips[1].pre = [0.0, -6.0];
            strips[1].now = [0.0, 6.0];
            2
        }
    };
    (strips, count)
}

fn position_hash(pos: &[f32; 3]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    pos[0].to_bits().hash(&mut h);
    pos[1].to_bits().hash(&mut h);
    pos[2].to_bits().hash(&mut h);
    h.finish()
}

/// Deterministic pseudo-random float in `[lo, hi)` from `(seed, salt)`.
/// Uses a splitmix64-style finalizer so even small salt deltas
/// (`0/4/8/12`) scramble the top bits — a plain `seed*K + salt` shift
/// would collide because the salt sits in the low bits that get
/// shifted out.
fn rand_in_range(seed: u64, salt: u64, lo: f32, hi: f32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(salt);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    let t = ((x >> 40) as f32) / ((1u64 << 24) as f32);
    lo + t * (hi - lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(effect: &mut BottomVerticalEffect, dt: f32) {
        effect.update(&EffectUpdateCtx { delta: dt, camera_target: None });
    }

    fn draws(effect: &BottomVerticalEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn dissonance_emits_four_world_quad_strips_converging_on_master() {
        // Sociable test: F1=0 → RadialConverge → 4 strips, each ending
        // at the master XZ (the `now` end), starting at a randomised
        // outer offset (the `pre` end). Confirms strip count + that
        // the `now` corners line up on the actor's column.
        let mut e = BottomVerticalEffect::new(
            Attach::WorldPos([5.0, 0.0, 7.0]),
            DISSONANCE,
        );
        step(&mut e, FADE_IN_SECS);
        let prims = draws(&e);
        assert_eq!(prims.len(), 4, "F1=0 = RadialConverge spawns 4 strips");
        for p in &prims {
            let EffectPrimitiveDraw::WorldQuad { corners, blend, .. } = p else {
                panic!("expected WorldQuad, got {p:?}");
            };
            assert_eq!(*blend, BlendKind::Alpha);
            // corners[0] = bottom-now, corners[3] = top-now → both
            // anchored to (master.x, _, master.z).
            assert!((corners[0][0] - 5.0).abs() < 1e-4);
            assert!((corners[0][2] - 7.0).abs() < 1e-4);
            assert!((corners[3][0] - 5.0).abs() < 1e-4);
            assert!((corners[3][2] - 7.0).abs() < 1e-4);
            // top corner sits STRIP_HEIGHT above the bottom (native -Y up).
            assert!((corners[3][1] - (corners[0][1] - STRIP_HEIGHT)).abs() < 1e-4);
        }
    }

    #[test]
    fn dontforgetme_emits_four_additive_green_strips() {
        // F1=2 → CrossedQuartet → 4 strips with green tint, additive.
        let mut e = BottomVerticalEffect::new(
            Attach::WorldPos([0.0, 0.0, 0.0]),
            DONTFORGETME,
        );
        step(&mut e, FADE_IN_SECS);
        let prims = draws(&e);
        assert_eq!(prims.len(), 4);
        match &prims[0] {
            EffectPrimitiveDraw::WorldQuad { color, blend, texture, .. } => {
                assert_eq!(*blend, BlendKind::Additive);
                assert_eq!(*texture, "magic_green.tga");
                // Green-dominant tint: G channel ~ 1.0, R/B ~ 155/255.
                assert!(color[1] > color[0]);
                assert!(color[1] > color[2]);
            }
            other => panic!("expected WorldQuad, got {other:?}"),
        }
    }

    #[test]
    fn crossed_quartet_strips_have_distinct_random_endpoints() {
        // Regression for a hash-collision bug where the per-strip salt
        // (0/4/8/12) was shifted out of `rand_in_range`'s mixed value,
        // making all 4 strips identical. The visible result was "one
        // strip" instead of a `#` cross because ec=0 and ec=1 sat on top
        // of each other (and same for ec=2/3).
        let mut e = BottomVerticalEffect::new(
            Attach::WorldPos([12.0, 0.0, 34.0]),
            ASSASSINCROSS,
        );
        step(&mut e, FADE_IN_SECS);
        // Strips ec=0 and ec=1 both have form (rx, -3)→(-rx, 3); their
        // rx values must differ so the two strips occupy different lines.
        let s0 = e.strips[0];
        let s1 = e.strips[1];
        assert!(
            (s0.pre[0] - s1.pre[0]).abs() > 0.05,
            "ec=0/1 share the same rx ({} vs {}) — salt is being truncated",
            s0.pre[0],
            s1.pre[0],
        );
        // Same for ec=2/3 which share the (-3, rz)→(3, -rz) form.
        let s2 = e.strips[2];
        let s3 = e.strips[3];
        assert!(
            (s2.pre[1] - s3.pre[1]).abs() > 0.05,
            "ec=2/3 share the same rz ({} vs {}) — salt is being truncated",
            s2.pre[1],
            s3.pre[1],
        );
    }

    #[test]
    fn serviceforyou_emits_two_perpendicular_strips() {
        // F1=4 → PerpendicularPair → exactly 2 strips, one along X
        // (z=0 for both corners) and one along Z (x=0).
        let mut e = BottomVerticalEffect::new(
            Attach::WorldPos([0.0, 0.0, 0.0]),
            SERVICEFORYOU,
        );
        step(&mut e, FADE_IN_SECS);
        let prims = draws(&e);
        assert_eq!(prims.len(), 2, "F1=4 = PerpendicularPair spawns 2 strips");

        let mut saw_x_strip = false;
        let mut saw_z_strip = false;
        for p in &prims {
            if let EffectPrimitiveDraw::WorldQuad { corners, .. } = p {
                let along_x = corners[0][2].abs() < 1e-4 && corners[1][2].abs() < 1e-4;
                let along_z = corners[0][0].abs() < 1e-4 && corners[1][0].abs() < 1e-4;
                saw_x_strip |= along_x;
                saw_z_strip |= along_z;
            }
        }
        assert!(saw_x_strip, "expected an X-aligned strip");
        assert!(saw_z_strip, "expected a Z-aligned strip");
    }
}
