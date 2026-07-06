//! `EF_TEIHIT1` / `EF_TEIHIT1X` / `EF_TEIHIT3` — radial streak-burst hit
//! effects (ids 262, 266, 276).
//!
//! The burst spawns `N` emitters, each with **four** sub-emitter
//! streaks.
//! So one effect = `N * 4` streaks. Each streak is a long thin
//! quad pointing in a random 3D direction (two random angles: a rise angle in
//! XZ, a display angle in XY), whose midpoint slides outward as
//! its distance grows by a fixed step per frame from a negative start. Alpha
//! ramps up over frames 11-20 then falls.
//!
//! Each streak is a real world-space quad (not a
//! camera-facing billboard), so we build it with `WorldQuad` — the burst
//! then reads correctly from any camera. The base distances are in units
//! ~6× the on-screen silhouette; [`WORLD_SCALE`] maps them to the gif.
//!
//! `EF_TEIHIT2` / `EF_BACKSTAP` use the *different* directional
//! spray and have no reference gif in the library; they are deferred.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Unit → world-unit factor. Unlike some effects, the streak's raw
/// distance (grows to ~140) is already near world scale: the burst fills
/// almost the whole screen in the original, so the streaks travel out tens of
/// world units. Tuned so the bright burst spans most of the camera frame.
const WORLD_SCALE: f32 = 0.6;

/// Burst centre sits above the actor's feet — lifted 9 (native
/// RO coordinates use `-Y = up`).
const CENTER_RISE: f32 = 9.0 * WORLD_SCALE;

/// Alpha ramps in units of 15/255 over frames 11-20, so it
/// peaks near `150/255`.
const ALPHA_RISE: f32 = 15.0 / 255.0;
const ALPHA_FALL: f32 = 5.0 / 255.0;

#[derive(Clone, Copy)]
pub struct TeihitParams {
    pub texture: &'static str,
    /// RGB tint applied to the (greyscale) streak texture under additive blend.
    pub tint: [f32; 3],
    /// Emitter count; streaks = `prim_count * 4`.
    pub prim_count: usize,
    /// Distance gained per frame once the streak is live.
    pub distance_speed: f32,
    /// Half the streak length along its axis.
    pub half_len: f32,
    /// Half the streak width.
    pub width: f32,
    /// Frame counter start = `-delay_base - random(delay_rand)`.
    pub delay_base: i32,
    pub delay_rand: u32,
}

pub const TEIHIT1: TeihitParams = TeihitParams {
    texture: "alpha_center.tga",
    tint: [1.0, 0.85, 0.35], // yellow
    prim_count: 20,
    distance_speed: 3.0,
    half_len: 0.7 * 24.0,
    width: 0.7,
    delay_base: 10,
    delay_rand: 25,
};
pub const TEIHIT1X: TeihitParams = TeihitParams {
    texture: "lens1.tga",
    tint: [1.0, 1.0, 1.0], // neutral
    prim_count: 24,
    distance_speed: 3.0,
    half_len: 0.7 * 24.0,
    width: 0.7,
    delay_base: 10,
    delay_rand: 25,
};
pub const TEIHIT3: TeihitParams = TeihitParams {
    texture: "lens2.tga",
    tint: [0.55, 0.75, 1.0], // a bit blue
    prim_count: 20,
    distance_speed: 2.0,
    half_len: 1.0 * 16.0,
    width: 1.0,
    delay_base: 20,
    delay_rand: 10,
};

/// The directional spray (`EF_TEIHIT2` / `EF_BACKSTAP`). Distinct from the
/// streaks above: a burst of camera-facing billboards
/// that erupt from the target and fly outward along the
/// caster→target heading (±30° jitter) with a random vertical velocity,
/// ramping alpha over 10 frames then fading. Only the emitter count
/// differs between the two ids (5 → 20 emitters, each = 4 streaks).
///
/// Each sub-emitter is a small camera-facing quad sized by
/// its distance (0.1–0.9), **alpha-blended**.
/// So the burst reads as a spray of small solid bubbles projected in the
/// facing direction — not glowing streaks.
///
/// The original game shows pale/neutral bubbles (not red), so we leave the
/// soft round `alpha_center.tga` untinted. Its texture `effect\thunder_red.bmp`
/// is **absent from the classic GRF** and there is no reference gif, so the
/// look is matched to the observed in-game behaviour
/// rather than a capture.
#[derive(Clone, Copy)]
pub struct TeiHit2Params {
    /// Emitter count; bubbles = `prim_count * 4`.
    pub prim_count: usize,
}

pub const TEIHIT2: TeiHit2Params = TeiHit2Params { prim_count: 5 };
pub const BACKSTAP: TeiHit2Params = TeiHit2Params { prim_count: 20 };

/// Round soft fallback for the absent `thunder_red.bmp`.
const TEIHIT2_TEXTURE: &str = "alpha_center.tga";
/// Pale/neutral bubbles (the in-game effect is not red).
const TEIHIT2_TINT: [f32; 3] = [1.0, 1.0, 1.0];
/// Each bubble's quad spans radius `distance` (0.1–0.9); the billboard size
/// is the diameter. Kept close to the source so the bubbles stay small.
const TEIHIT2_BUBBLE_SCALE: f32 = 1.2;

pub const TEXTURES: &[&str] = &[
    TEIHIT1.texture,
    TEIHIT1X.texture,
    TEIHIT3.texture,
    TEIHIT2_TEXTURE,
];

pub const TOTAL_DURATION_MS: u32 = 3000;

/// The directional dart-burst erupts at the target (a short-range hit), so it
/// has no caster→target flight delay — the hit keeps the attack-motion timing.
pub const PROJECTILE_FLIGHT: crate::effect_queue::ProjectileFlight =
    crate::effect_queue::ProjectileFlight::AtTarget;

struct Rng(u32);
impl Rng {
    fn from_seed(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9).wrapping_add(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

struct Streak {
    /// XY-plane display angle, radians.
    angle_xy: f32,
    /// XZ-plane rise angle, radians.
    angle_xz: f32,
    process: i32,
    distance: f32,
    alpha: f32,
}

pub struct TeihitEffect {
    params: TeihitParams,
    center: [f32; 3],
    streaks: Vec<Streak>,
    /// Fractional-frame accumulator so the state machine can advance slower
    /// than real time (see [`TIME_SCALE`]).
    frame_accum: f32,
}

impl TeihitEffect {
    pub fn new(anchor: [f32; 3], params: TeihitParams) -> Self {
        let seed = anchor[0].to_bits() ^ anchor[2].to_bits() ^ 0x7E1_4117;
        let mut rng = Rng::from_seed(seed);
        let streaks = (0..params.prim_count * 4)
            .map(|_| Streak {
                angle_xy: rng.range(0.0, std::f32::consts::TAU),
                angle_xz: rng.range(0.0, std::f32::consts::TAU),
                process: -params.delay_base - rng.next_u32().rem_euclid(params.delay_rand) as i32,
                distance: -10.0,
                alpha: 0.0,
            })
            .collect();
        Self {
            params,
            center: [anchor[0], anchor[1] - CENTER_RISE, anchor[2]],
            streaks,
            frame_accum: 0.0,
        }
    }

    /// One frame of streak state evolution.
    fn step_frame(&mut self) {
        for s in &mut self.streaks {
            s.process += 1;
            if s.process > 0 {
                s.distance += self.params.distance_speed;
            }
            if s.process > 10 && s.process <= 20 {
                s.alpha = (s.alpha + ALPHA_RISE).min(150.0 / 255.0);
            } else if s.process > 20 {
                s.alpha = (s.alpha - ALPHA_FALL).max(0.0);
            }
        }
    }

    /// World-space corners of one streak: a local
    /// `(Rx along axis, Ry across)` quad rotated by `angle_xy` then `angle_xz`.
    fn corners(&self, s: &Streak) -> [[f32; 3]; 4] {
        let d = s.distance * WORLD_SCALE;
        let l = self.params.half_len * WORLD_SCALE;
        let w = self.params.width * WORLD_SCALE;
        let (sin_xy, cos_xy) = s.angle_xy.sin_cos();
        let (sin_xz, cos_xz) = s.angle_xz.sin_cos();
        let place = |rx: f32, ry: f32| {
            let x1 = cos_xy * rx - sin_xy * ry;
            let y1 = sin_xy * rx + cos_xy * ry;
            [
                self.center[0] + cos_xz * x1,
                self.center[1] + y1,
                self.center[2] + sin_xz * x1,
            ]
        };
        [
            place(d + l, w),
            place(d - l, w),
            place(d - l, -w),
            place(d + l, -w),
        ]
    }
}

impl Effect for TeihitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        // Done once every streak has finished its fade-out.
        let alive = self
            .streaks
            .iter()
            .any(|s| s.process <= 20 || s.alpha > 0.0);
        if alive {
            EffectStatus::Running
        } else {
            EffectStatus::Dead
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.streaks {
            if s.alpha <= 0.0 {
                continue;
            }
            let [r, g, b] = self.params.tint;
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners: self.corners(s),
                uv: [[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                texture: self.params.texture,
                color: [r, g, b, s.alpha],
                blend: BlendKind::Additive,
                no_depth: false,
            });
        }
    }
}

/// The directional billboard-spray darts. Each dart
/// starts at the target, picks the caster→target heading ±30° at spawn, then
/// flies outward at a per-dart speed while drifting vertically; alpha ramps
/// `+25/frame` for 10 frames then fades `−3/frame`.
struct Dart {
    pos: [f32; 3],
    sin_a: f32,
    cos_a: f32,
    speed: f32,
    y_vel: f32,
    /// Bubble diameter in world units (quad radius = `distance`).
    size: f32,
    process: i32,
    alpha: f32,
    rotation: f32,
}

pub struct TeiHit2Effect {
    darts: Vec<Dart>,
    frame_accum: f32,
}

impl TeiHit2Effect {
    pub fn new(from: [f32; 3], to: [f32; 3], params: TeiHit2Params) -> Self {
        let seed = to[0].to_bits() ^ to[2].to_bits() ^ 0xBACC_57A8;
        let mut rng = Rng::from_seed(seed);
        // Caster→target heading.
        let base_angle = (to[0] - from[0]).atan2(to[2] - from[2]);
        let origin = [to[0], to[1] - 8.0 * WORLD_SCALE, to[2]];
        let darts = (0..params.prim_count * 4)
            .map(|_| {
                let jitter = (rng.range(0.0, 60.0) - 30.0).to_radians();
                let (sin_a, cos_a) = (base_angle + jitter).sin_cos();
                // `distance = random(16)*0.05 + 0.1` → 0.1..0.9 quad radius.
                let distance = rng.range(0.1, 0.9);
                Dart {
                    pos: origin,
                    sin_a,
                    cos_a,
                    speed: rng.range(0.75, 1.25),
                    y_vel: rng.range(-1.0, 1.0),
                    size: distance * TEIHIT2_BUBBLE_SCALE,
                    process: -40 + rng.next_u32().rem_euclid(10) as i32,
                    alpha: 0.0,
                    rotation: 0.0,
                }
            })
            .collect();
        Self {
            darts,
            frame_accum: 0.0,
        }
    }

    fn step_frame(&mut self) {
        for d in &mut self.darts {
            d.process += 1;
            if d.process <= 0 {
                continue;
            }
            d.rotation -= 5.0_f32.to_radians();
            if d.process <= 10 {
                d.alpha = (d.alpha + 25.0 / 255.0).min(1.0);
            } else {
                d.alpha = (d.alpha - 3.0 / 255.0).max(0.0);
            }
            // `dx.atan2(dz)` heading (+Z = 0): direction is (sin, cos) on (x, z).
            d.pos[0] += d.speed * WORLD_SCALE * d.sin_a;
            d.pos[2] += d.speed * WORLD_SCALE * d.cos_a;
            d.pos[1] += d.y_vel * WORLD_SCALE;
        }
    }
}

impl Effect for TeiHit2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        // The spray runs at native 60 fps (no gif to stretch against, unlike
        // the streaks which borrow `TIME_SCALE`).
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        let alive = self.darts.iter().any(|d| d.process <= 10 || d.alpha > 0.0);
        if alive {
            EffectStatus::Running
        } else {
            EffectStatus::Dead
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = TEIHIT2_TINT;
        for d in &self.darts {
            if d.alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: d.pos,
                size: [d.size, d.size],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: d.rotation,
                texture: TEIHIT2_TEXTURE,
                color: [r, g, b, d.alpha],
                // Alpha-blended: solid bubbles, not glow.
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(e: &mut TeihitEffect, source_frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..source_frames {
            st = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        st
    }

    fn quads(e: &TeihitEffect) -> Vec<[[f32; 3]; 4]> {
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 256.0,
                screen_h: 256.0,
                elapsed: 0.0,
            },
        );
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::WorldQuad {
                    corners,
                    blend: BlendKind::Additive,
                    ..
                } => *corners,
                _ => panic!("expected additive WorldQuad streaks"),
            })
            .collect()
    }

    fn radius(c: &[[f32; 3]; 4], center: [f32; 3]) -> f32 {
        let mid = [
            (c[0][0] + c[2][0]) / 2.0 - center[0],
            (c[0][1] + c[2][1]) / 2.0 - center[1],
            (c[0][2] + c[2][2]) / 2.0 - center[2],
        ];
        (mid[0] * mid[0] + mid[1] * mid[1] + mid[2] * mid[2]).sqrt()
    }

    fn billboards(e: &TeiHit2Effect) -> Vec<[f32; 3]> {
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 256.0,
                screen_h: 256.0,
                elapsed: 0.0,
            },
        );
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard {
                    pos,
                    blend: BlendKind::Alpha,
                    ..
                } => *pos,
                _ => panic!("expected alpha-blended Billboard bubbles"),
            })
            .collect()
    }

    #[test]
    fn teihit2_count_scales_with_prim_count_and_darts_fly_then_die() {
        // 5 prims × 4 = 20 darts (Teihit2); Backstap = 20 × 4 = 80.
        assert_eq!(
            TeiHit2Effect::new([0.0; 3], [0.0, 0.0, 30.0], TEIHIT2)
                .darts
                .len(),
            20,
        );
        let mut e = TeiHit2Effect::new([0.0; 3], [0.0, 0.0, 30.0], BACKSTAP);
        assert_eq!(e.darts.len(), 80);

        // Past the staggered start + fade-in, darts are visible and have flown
        // outward from the target origin toward +Z (the demo heading).
        for _ in 0..(40 + 12) {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        let visible = billboards(&e);
        assert!(!visible.is_empty(), "darts visible after fade-in");
        let mean_z: f32 = visible.iter().map(|p| p[2]).sum::<f32>() / visible.len() as f32;
        assert!(
            mean_z > 30.0,
            "darts erupt past the target along the heading: {mean_z}"
        );

        // Every dart eventually fades out and the burst dies.
        let mut st = EffectStatus::Running;
        for _ in 0..2000 {
            st = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
            if st == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(st, EffectStatus::Dead);
    }

    #[test]
    fn spawns_four_streaks_per_prim_once_visible() {
        let mut e = TeihitEffect::new([0.0; 3], TEIHIT1);
        assert!(quads(&e).is_empty(), "nothing visible before fade-in");
        // Advance past the longest start delay + fade-in.
        tick(
            &mut e,
            (TEIHIT1.delay_base as u32 + TEIHIT1.delay_rand) + 18,
        );
        let n = quads(&e).len();
        assert!(
            n > 0 && n <= TEIHIT1.prim_count * 4,
            "some of the {} streaks show",
            TEIHIT1.prim_count * 4
        );
    }

    #[test]
    fn streaks_travel_outward_then_burst_dies() {
        let mut e = TeihitEffect::new([0.0; 3], TEIHIT3);
        let center = e.center;
        tick(
            &mut e,
            (TEIHIT3.delay_base as u32 + TEIHIT3.delay_rand) + 15,
        );
        let early: f32 = quads(&e).iter().map(|c| radius(c, center)).sum::<f32>()
            / quads(&e).len().max(1) as f32;
        tick(&mut e, 12);
        let late_qs = quads(&e);
        if !late_qs.is_empty() {
            let late =
                late_qs.iter().map(|c| radius(c, center)).sum::<f32>() / late_qs.len() as f32;
            assert!(late > early, "streaks slide outward: {early} -> {late}");
        }
        // Eventually every streak fades out and the effect dies.
        assert_eq!(tick(&mut e, 200), EffectStatus::Dead);
    }
}
