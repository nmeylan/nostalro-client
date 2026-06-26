//! `EF_BLITZBEAT` — Falcon Blitz Beat strike (id 115).
//!
//! Falcon Blitz Beat — a volley of translating cross-textured needles.
//!
//! Observed in-game: the needle direction is **world-fixed** (NE → SW),
//! independent of the caster's facing or the camera. The yaw comes from the
//! master's facing angle, but for Blitzbeat that angle is
//! a fixed compass heading; rotating the camera changes how the needles
//! appear on screen but not their world-axis direction.
//!
//! Forward axis = NE = `(+√2/2, 0, +√2/2)` → `yaw = π/4`. The
//! motion sets `speed = -1.2 + accel*frame` and steps `pos += speed * forward`
//! each frame, which moves the needles in the *opposite* of forward — i.e.
//! SW. So needles start in the NE arc and translate SW.
//!
//! At spawn it emits 10 cross-textured needles. Each needle:
//!   * scatter offset (world XZ): `(3.5*sinθ, -7, -3.5*cosθ)` for a random θ
//!     — `-7` is "7 above ground" (native RO `-Y = up`).
//!   * initial position offset along caster's forward: `+15` units.
//!   * per-frame motion along caster's forward axis with `speed += accel`
//!     each tick, starting `speed = -1.2`, `accel = -0.1`. So at frame `f`
//!     the cumulative forward offset is `15 - 1.2*f - 0.05*f*(f+1)`. The
//!     needle starts 15 ahead, slows, reverses, and ends ~30 units behind.
//!   * width random `2..6`, height `0.2` (thin needle). `width_speed = 0.1`,
//!     `width_accel = -0.01` — width grows slightly then plateaus over 20
//!     frames.
//!   * alpha: instant ramp to 1.0 in ~2 frames, hold, hard fade in last 2.
//!
//! Each needle is drawn as a cross-texture: the same texture on one
//! horizontal quad and one vertical quad, both with the
//! needle's long axis along the caster's forward direction. This keeps the
//! needle silhouette readable from any camera angle.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const BLITZBEAT_TEXTURE: &str = "ac_center2.tga";
pub const TEXTURES: &[&str] = &[BLITZBEAT_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const NEEDLE_COUNT: u32 = 10;
const DURATION_FRAMES: f32 = 20.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const SCATTER_RADIUS: f32 = 3.5;
const Y_OFFSET: f32 = -7.0;
const FORWARD_INIT: f32 = 15.0;
const SPEED_INIT: f32 = -1.2;
const SPEED_ACCEL: f32 = -0.1;

const HALF_HEIGHT: f32 = 0.1;
const HALF_WIDTH_MIN: f32 = 1.0;
const HALF_WIDTH_MAX: f32 = 3.0;
const WIDTH_SPEED_INIT: f32 = 0.1;
const WIDTH_ACCEL: f32 = -0.01;

const FADE_IN_FRAMES: f32 = 2.0;
const FADE_OUT_START: f32 = DURATION_FRAMES - 2.0;

#[derive(Clone, Copy)]
struct Needle {
    scatter: [f32; 3],
    base_half_width: f32,
}

pub struct BlitzbeatEffect {
    caster_pos: [f32; 3],
    yaw: f32,
    age: f32,
    needles: [Needle; NEEDLE_COUNT as usize],
}

/// World-fixed yaw for the needle volley: forward axis = NE, so the
/// per-frame negative-speed motion translates the needles SW.
const FIXED_YAW: f32 = std::f32::consts::FRAC_PI_4;

impl BlitzbeatEffect {
    /// Spawn at `caster_pos`. The needle direction is world-fixed (NE→SW);
    /// caster facing and camera angle are ignored.
    pub fn new(caster_pos: [f32; 3]) -> Self {
        Self::with_yaw(caster_pos, FIXED_YAW)
    }

    /// Override the world-fixed yaw — used by tests; production callers
    /// should use [`Self::new`].
    pub fn with_yaw(caster_pos: [f32; 3], yaw: f32) -> Self {
        let seed = position_hash(&caster_pos);
        let mut needles = [Needle {
            scatter: [0.0; 3],
            base_half_width: 0.0,
        }; NEEDLE_COUNT as usize];
        for i in 0..NEEDLE_COUNT as usize {
            let salt = (i as u64) * 4;
            let theta = rand_in_range(seed, salt + 1, 0.0, std::f32::consts::TAU);
            let half_width = rand_in_range(seed, salt + 2, HALF_WIDTH_MIN, HALF_WIDTH_MAX);
            let (sn, cs) = theta.sin_cos();
            needles[i] = Needle {
                scatter: [SCATTER_RADIUS * sn, Y_OFFSET, -SCATTER_RADIUS * cs],
                base_half_width: half_width,
            };
        }
        Self {
            caster_pos,
            yaw,
            age: 0.0,
            needles,
        }
    }

    fn forward(&self) -> [f32; 3] {
        let (s, c) = self.yaw.sin_cos();
        [c, 0.0, s]
    }

    fn forward_offset_at(&self, frame: f32) -> f32 {
        // Closed form for the per-frame `speed += accel; delta_pos
        // += speed * forward`. At frame f (real, fractional):
        //   speed_i = SPEED_INIT + i * SPEED_ACCEL  for i ∈ 1..f
        //   delta = FORWARD_INIT + Σ speed_i = FORWARD_INIT + SPEED_INIT*f
        //           + SPEED_ACCEL * f*(f+1)/2
        FORWARD_INIT + SPEED_INIT * frame + SPEED_ACCEL * frame * (frame + 1.0) * 0.5
    }

    fn half_width_at(&self, base: f32, frame: f32) -> f32 {
        // width(f) = base + WIDTH_SPEED_INIT*f + WIDTH_ACCEL * f*(f+1)/2
        let w = base + WIDTH_SPEED_INIT * frame + WIDTH_ACCEL * frame * (frame + 1.0) * 0.5;
        w.max(0.05)
    }

    fn alpha_at(&self, frame: f32) -> f32 {
        if frame < FADE_IN_FRAMES {
            (frame / FADE_IN_FRAMES).clamp(0.0, 1.0)
        } else if frame < FADE_OUT_START {
            1.0
        } else {
            (1.0 - (frame - FADE_OUT_START) / (DURATION_FRAMES - FADE_OUT_START)).clamp(0.0, 1.0)
        }
    }
}

impl Effect for BlitzbeatEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age * FRAMES_PER_SECOND >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        let alpha = self.alpha_at(frame);
        if alpha <= 0.0 {
            return;
        }
        let forward = self.forward();
        let forward_offset = self.forward_offset_at(frame);
        let color = [1.0, 1.0, 1.0, alpha];
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for needle in &self.needles {
            let half_width = self.half_width_at(needle.base_half_width, frame);
            let center = [
                self.caster_pos[0] + needle.scatter[0] + forward[0] * forward_offset,
                self.caster_pos[1] + needle.scatter[1] + forward[1] * forward_offset,
                self.caster_pos[2] + needle.scatter[2] + forward[2] * forward_offset,
            ];
            for plane in [
                QuadPlane::HorizontalYaw(self.yaw),
                QuadPlane::VerticalYaw(self.yaw),
            ] {
                out.push(EffectPrimitiveDraw::Texture3D {
                    center,
                    size: [half_width, HALF_HEIGHT],
                    plane,
                    uv,
                    texture: BLITZBEAT_TEXTURE,
                    color,
                    blend: BlendKind::Additive,
                });
            }
        }
    }
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

    fn step_n(e: &mut BlitzbeatEffect, n: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..n {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
            if s == EffectStatus::Dead {
                break;
            }
        }
        s
    }

    fn draws(e: &BlitzbeatEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn ten_parallel_cross_textured_needles_translate_along_forward() {
        let mut e = BlitzbeatEffect::with_yaw([0.0, 0.0, 0.0], 0.0);

        // After a couple of frames the needles are visible; cross-texture =
        // two quads (horizontal + vertical) per needle = 20 total.
        step_n(&mut e, 3);
        let prims = draws(&e);
        assert_eq!(prims.len(), 20, "10 needles × 2 quads (cross-texture)");

        // All planes are HorizontalYaw / VerticalYaw with the *same* yaw —
        // the needles are parallel, not crossed.
        for p in &prims {
            let yaw = match p {
                EffectPrimitiveDraw::Texture3D {
                    plane: QuadPlane::HorizontalYaw(y) | QuadPlane::VerticalYaw(y),
                    ..
                } => *y,
                _ => panic!("expected Texture3D needle plane"),
            };
            assert_eq!(yaw, 0.0, "all needles share caster yaw");
        }

        // Forward translation: at frame 0 the needles sit at +15 forward
        // (along +X for yaw=0); a few frames later they've moved less far
        // along forward (speed is negative, decelerating). Average X across
        // needles should drop.
        let avg_x = |prims: &[EffectPrimitiveDraw]| -> f32 {
            let xs: Vec<f32> = prims
                .iter()
                .map(|p| match p {
                    EffectPrimitiveDraw::Texture3D { center, .. } => center[0],
                    _ => panic!(),
                })
                .collect();
            xs.iter().sum::<f32>() / xs.len() as f32
        };
        let x_early = avg_x(&prims);
        step_n(&mut e, 8);
        let x_later = avg_x(&draws(&e));
        assert!(
            x_later < x_early,
            "needles translate backward along forward axis: {x_early} -> {x_later}"
        );
    }

    #[test]
    fn dies_after_20_frames() {
        let mut e = BlitzbeatEffect::new([0.0, 0.0, 0.0]);
        let status = step_n(&mut e, 25);
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn deterministic_scatter_per_position() {
        let a = BlitzbeatEffect::new([10.0, 0.0, 20.0]);
        let b = BlitzbeatEffect::new([10.0, 0.0, 20.0]);
        for i in 0..NEEDLE_COUNT as usize {
            assert_eq!(a.needles[i].scatter, b.needles[i].scatter);
            assert_eq!(a.needles[i].base_half_width, b.needles[i].base_half_width);
        }
    }
}
