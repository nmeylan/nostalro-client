//! Bottom_Vertical family — vertical "curtain" strips anchored at two
//! ground points and rising to an animated `max_height`.
//!
//! Each variant builds up to 4 cells. Each cell is rendered as
//! one quad (a `WorldQuad` strip) whose corners are
//! the two ground anchor points lifted to the strip's animated height:
//!   * `(now, -max_height)` (top, "now" side)
//!   * `(now, 0)`           (bottom, "now")
//!   * `(pre, 0)`           (bottom, "pre" side)
//!   * `(pre, -max_height)` (top, "pre")
//!
//! Native RO `-Y` = up so `-max_height` is the top. The two ground points
//! define the strip's orientation in XZ; the phase is set to
//! `random(360)` at spawn (a per-strip phase).
//!
//! **Animation.** Every frame the phase advances
//! `+4` for the F1=1 variant (Assassincross) or `+2` otherwise, then
//! `max_height = 20 + 10 * sin(phase)`. So every strip's height
//! pulses between 10 and 30, each starting from its own random phase — the
//! curtains breathe out of sync rather than standing static.
//!
//! **Blend + tint (by variant).** Some variants draw **additive** (the
//! texture adds onto the scene), some draw **alpha** (composited over it):
//!   * Dissonance/Uglydance → additive, white
//!   * Assassincross        → alpha, pink (255,155,155)
//!   * Dontforgetme         → alpha, green (155,255,155)
//!   * Serviceforyou        → additive, pink (255,150,150)
//!
//! F1 → cell layout: F1=0 = 4 radial strips `(rand±20)→(0,0)`, base alpha 15;
//! F1=1/2 = 4 crossed strips, base alpha 50; F1=4 = 2 perpendicular strips,
//! base alpha 100.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

#[derive(Clone, Copy, Debug)]
pub struct BottomVerticalParams {
    pub texture: &'static str,
    /// Per-cell base alpha (out of 255).
    pub base_alpha: f32,
    /// RGB tint selected per variant (0..1).
    pub tint_rgb: [f32; 3],
    /// Blend mode: additive or alpha, per variant.
    pub blend: BlendKind,
    /// Phase increment per frame: 4 for
    /// the F1=1 variant (Assassincross), 2 otherwise.
    pub phase_speed_deg: f32,
    /// Cell layout pattern (F1 → per-strip placement).
    pub layout: StripLayout,
}

#[derive(Clone, Copy, Debug)]
pub enum StripLayout {
    /// F1=0: 4 strips, each from a per-strip random XZ offset (±20) to the
    /// origin. Radial lines converging on the actor.
    RadialConverge,
    /// F1=1/2: 4 strips arranged as two perpendicular pairs (a `#` cross).
    CrossedQuartet,
    /// F1=4: 2 perpendicular strips through the origin (one X, one Z).
    PerpendicularPair,
}

const FRAMES_PER_SECOND: f32 = 60.0;
const FADE_IN_FRAMES: f32 = 15.0;
const FADE_IN_SECS: f32 = FADE_IN_FRAMES / FRAMES_PER_SECOND;
/// `max_height = MAX_HEIGHT_BASE + MAX_HEIGHT_AMP * sin(phase)`
/// → pulses between 10 and 30 world units (native RO `-Y` = up).
const MAX_HEIGHT_BASE: f32 = 20.0;
const MAX_HEIGHT_AMP: f32 = 10.0;

/// `EF_BOTTOM_DISSONANCE` → `ring_blue.tga`, F1=0 →
/// additive white; phase +2/frame.
pub const DISSONANCE: BottomVerticalParams = BottomVerticalParams {
    texture: "ring_blue.tga",
    base_alpha: 15.0 / 255.0,
    tint_rgb: [1.0, 1.0, 1.0],
    blend: BlendKind::Additive,
    phase_speed_deg: 2.0,
    layout: StripLayout::RadialConverge,
};

/// `EF_BOTTOM_UGLYDANCE` → `ring_red.tga`. Same F1=0 shape
/// as Dissonance with a red texture.
pub const UGLYDANCE: BottomVerticalParams = BottomVerticalParams {
    texture: "ring_red.tga",
    base_alpha: 15.0 / 255.0,
    tint_rgb: [1.0, 1.0, 1.0],
    blend: BlendKind::Additive,
    phase_speed_deg: 2.0,
    layout: StripLayout::RadialConverge,
};

/// `EF_BOTTOM_ASSASSINCROSS` → `ring_red.tga`, F1=1.
/// Alpha pink (255,155,155); phase +4/frame.
pub const ASSASSINCROSS: BottomVerticalParams = BottomVerticalParams {
    texture: "ring_red.tga",
    base_alpha: 50.0 / 255.0,
    tint_rgb: [255.0 / 255.0, 155.0 / 255.0, 155.0 / 255.0],
    blend: BlendKind::Alpha,
    phase_speed_deg: 4.0,
    layout: StripLayout::CrossedQuartet,
};

/// `EF_BOTTOM_DONTFORGETME` → `magic_green.tga`, F1=2.
/// Alpha green (155,255,155); phase +2/frame.
pub const DONTFORGETME: BottomVerticalParams = BottomVerticalParams {
    texture: "magic_green.tga",
    base_alpha: 50.0 / 255.0,
    tint_rgb: [155.0 / 255.0, 255.0 / 255.0, 155.0 / 255.0],
    blend: BlendKind::Alpha,
    phase_speed_deg: 2.0,
    layout: StripLayout::CrossedQuartet,
};

/// `EF_BOTTOM_SERVICEFORYOU` → `safeline.bmp`, F1=4.
/// Additive pink (255,150,150); phase +2/frame.
pub const SERVICEFORYOU: BottomVerticalParams = BottomVerticalParams {
    texture: "safeline.bmp",
    base_alpha: 100.0 / 255.0,
    tint_rgb: [255.0 / 255.0, 150.0 / 255.0, 150.0 / 255.0],
    blend: BlendKind::Additive,
    phase_speed_deg: 2.0,
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
    /// Per-strip endpoint offsets + animation phase, frozen at spawn.
    strips: [Strip; 4],
    strip_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct Strip {
    pre: [f32; 2],
    now: [f32; 2],
    /// Phase `= random(360)` at spawn (degrees) — the per-strip
    /// height-pulse phase offset.
    phase0_deg: f32,
}

impl BottomVerticalEffect {
    pub fn new(world_pos: [f32; 3], params: BottomVerticalParams) -> Self {
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
        let frames = self.age * FRAMES_PER_SECOND;
        // Strip corners (CCW): bottom-now, bottom-pre, top-pre, top-now.
        let uv = [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
        let base_y = self.world_pos[1];
        for s in &self.strips[..self.strip_count] {
            let phase = (s.phase0_deg + self.params.phase_speed_deg * frames).to_radians();
            let max_height = MAX_HEIGHT_BASE + MAX_HEIGHT_AMP * phase.sin();
            let top_y = base_y - max_height;
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
                no_depth: false,
            });
        }
    }
}

/// Build the per-strip anchor + phase table for a given layout. The original
/// game's random values are reproduced via position-hashed
/// pseudo-random numbers so the same spawn looks identical across rerenders.
fn build_strips(layout: StripLayout, world_pos: &[f32; 3]) -> ([Strip; 4], usize) {
    let seed = position_hash(world_pos);
    let mut strips = [Strip {
        pre: [0.0; 2],
        now: [0.0; 2],
        phase0_deg: 0.0,
    }; 4];
    // Every layout seeds each strip's height-pulse phase from a distinct salt.
    for (i, s) in strips.iter_mut().enumerate() {
        s.phase0_deg = rand_in_range(seed, i as u64 * 4 + 2, 0.0, 360.0);
    }
    let count = match layout {
        // F1=0: `pre = (random(40)-20, random(40)-20)`, `now = (0,0)`.
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
        // F1=4: 2 perpendicular strips through origin.
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
        effect.update(&EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None });
    }

    fn draws(effect: &BottomVerticalEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn strip_height(p: &EffectPrimitiveDraw) -> f32 {
        let EffectPrimitiveDraw::WorldQuad { corners, .. } = p else {
            panic!("expected WorldQuad");
        };
        // bottom-now y minus top-now y (native -Y up → positive height).
        corners[0][1] - corners[3][1]
    }

    #[test]
    fn dissonance_emits_four_additive_strips_converging_on_master() {
        // Sociable test: F1=0 → RadialConverge → 4 additive strips, each
        // ending at the actor XZ (`now`), starting at a randomised outer
        // offset (`pre`). F1=0 → additive (was wrongly alpha).
        let mut e = BottomVerticalEffect::new([5.0, 0.0, 7.0], DISSONANCE);
        step(&mut e, FADE_IN_SECS);
        let prims = draws(&e);
        assert_eq!(prims.len(), 4, "F1=0 spawns 4 strips");
        for p in &prims {
            let EffectPrimitiveDraw::WorldQuad { corners, blend, .. } = p else {
                panic!("expected WorldQuad, got {p:?}");
            };
            assert_eq!(*blend, BlendKind::Additive);
            assert!((corners[0][0] - 5.0).abs() < 1e-4);
            assert!((corners[0][2] - 7.0).abs() < 1e-4);
            assert!((corners[3][0] - 5.0).abs() < 1e-4);
            assert!((corners[3][2] - 7.0).abs() < 1e-4);
        }
    }

    #[test]
    fn dontforgetme_emits_four_alpha_green_strips() {
        // F1=2 → CrossedQuartet → 4 strips, green, alpha.
        let mut e = BottomVerticalEffect::new([0.0, 0.0, 0.0], DONTFORGETME);
        step(&mut e, FADE_IN_SECS);
        let prims = draws(&e);
        assert_eq!(prims.len(), 4);
        match &prims[0] {
            EffectPrimitiveDraw::WorldQuad { color, blend, texture, .. } => {
                assert_eq!(*blend, BlendKind::Alpha);
                assert_eq!(*texture, "magic_green.tga");
                assert!(color[1] > color[0]);
                assert!(color[1] > color[2]);
            }
            other => panic!("expected WorldQuad, got {other:?}"),
        }
    }

    #[test]
    fn strip_height_pulses_between_10_and_30_over_time() {
        // Regression: strips used a fixed height and read as static/too short.
        // The animation drives `max_height = 20 + 10*sin(phase)`; sampling a
        // strip across its phase sweep must reach near both bounds.
        let mut e = BottomVerticalEffect::new([0.0, 0.0, 0.0], DISSONANCE);
        let mut min_h = f32::INFINITY;
        let mut max_h = f32::NEG_INFINITY;
        // +2°/frame → a full 360° sweep takes 180 frames.
        for _ in 0..180 {
            step(&mut e, 1.0 / 60.0);
            let h = strip_height(&draws(&e)[0]);
            min_h = min_h.min(h);
            max_h = max_h.max(h);
        }
        assert!(min_h < 12.0, "trough near 10: {min_h}");
        assert!(max_h > 28.0, "peak near 30: {max_h}");
    }

    #[test]
    fn crossed_quartet_strips_have_distinct_random_endpoints() {
        // Regression for a hash-collision bug where the per-strip salt was
        // shifted out of `rand_in_range`, making all 4 strips identical.
        let mut e = BottomVerticalEffect::new([12.0, 0.0, 34.0], ASSASSINCROSS);
        step(&mut e, FADE_IN_SECS);
        let s0 = e.strips[0];
        let s1 = e.strips[1];
        assert!((s0.pre[0] - s1.pre[0]).abs() > 0.05, "ec=0/1 rx must differ");
        let s2 = e.strips[2];
        let s3 = e.strips[3];
        assert!((s2.pre[1] - s3.pre[1]).abs() > 0.05, "ec=2/3 rz must differ");
    }

    #[test]
    fn serviceforyou_emits_two_perpendicular_additive_strips() {
        // F1=4 → PerpendicularPair → 2 strips (one X, one Z), additive pink
        // (F1=4 → additive).
        let mut e = BottomVerticalEffect::new([0.0, 0.0, 0.0], SERVICEFORYOU);
        step(&mut e, FADE_IN_SECS);
        let prims = draws(&e);
        assert_eq!(prims.len(), 2, "F1=4 spawns 2 strips");
        let mut saw_x_strip = false;
        let mut saw_z_strip = false;
        for p in &prims {
            if let EffectPrimitiveDraw::WorldQuad { corners, blend, .. } = p {
                assert_eq!(*blend, BlendKind::Additive);
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
