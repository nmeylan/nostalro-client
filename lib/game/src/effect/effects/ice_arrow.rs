//! `EF_ICEARROW` (id 26) — a stream of icy cross-texture shards flying from
//! the caster toward the target, finished by an expanding ring on impact.
//!
//! The original game's IceArrow effect has two parts:
//!   * 3D-cross-texture shards spawned on a 10-frame cadence from frame
//!     12. Each lives 70 frames, travels at a constant speed along a fixed
//!     heading, is `11.5 × 3.8` in size, fades from frame 50, uses
//!     `icearrow.tga` (additive). A cross texture is two orthogonal textured
//!     planes sharing the motion axis — we emit two `WorldQuad`s per shard.
//!   * 3D-circle impact ring (`ring_blue.tga`): a flat ring at the target
//!     that grows `1.2`/frame (slightly decelerating), inner size 10, alpha
//!     ramps over 10 frames and fades over the last 20 of its 30-frame life.
//!
//! The original computes the shard heading from a fixed spawn offset (it has
//! no explicit target); we reinterpret it as a caster→target trail so the
//! shards stream at whatever the skill targeted. The original fires the ring
//! off a caller hit-flag we don't have, so we trigger it when the first
//! shard's travel time elapses (`|to − from| / speed`).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Shard size (`11.5 × 3.8`), travel speed (`2.75`/frame) and ring radii
/// (inner 10, `1.2`/frame growth) are large source literals; downscale so the
/// shards are arrow-sized and the ring is a character-wide splash.
const WORLD_SCALE: f32 = 0.15;

const SHARD_TEXTURE: &str = "icearrow.tga";
const RING_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[SHARD_TEXTURE, RING_TEXTURE];

/// Cadence: one shard every 10 frames from frame 12.
const SPAWN_START_FRAME: f32 = 12.0;
const SPAWN_PERIOD_FRAMES: f32 = 10.0;
const SHARD_COUNT: usize = 8;

const SHARD_LIFE_FRAMES: f32 = 70.0;
const SHARD_FADE_START: f32 = 50.0;
const SHARD_SPEED_PER_FRAME: f32 = 2.75 * WORLD_SCALE;
/// width (along motion) / height (perpendicular), half-extents.
const SHARD_HALF_LEN: f32 = 11.5 * WORLD_SCALE;
const SHARD_HALF_WID: f32 = 3.8 * WORLD_SCALE;
/// Spawn slightly above the caster so the shards read as falling arrows.
const SPAWN_LIFT: f32 = 9.0 * WORLD_SCALE;
const SHARD_LATERAL_JITTER: f32 = 5.0 * WORLD_SCALE;

const RING_LIFE_FRAMES: f32 = 30.0;
const RING_INNER: f32 = 10.0 * WORLD_SCALE;
const RING_GROWTH_PER_FRAME: f32 = 1.2 * WORLD_SCALE;
const RING_THICKNESS: f32 = 4.0 * WORLD_SCALE;
const RING_FADE_START: f32 = RING_LIFE_FRAMES - 20.0;
const RING_FADE_IN: f32 = 10.0;
const RING_MAX_ALPHA: f32 = 1.0;

const SHARD_COLOR: [f32; 3] = [0.7, 0.85, 1.0];
const RING_COLOR: [f32; 3] = [0.6, 0.8, 1.0];

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

/// Last shard spawn + its life, plus the ring; the wall-clock end is the
/// longer of the two so neither is cut off.
const LAST_SPAWN_FRAME: f32 = SPAWN_START_FRAME + SPAWN_PERIOD_FRAMES * (SHARD_COUNT as f32 - 1.0);
const TOTAL_FRAMES: f32 = LAST_SPAWN_FRAME + SHARD_LIFE_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

fn norm(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-5 { [0.0, 0.0, 1.0] } else { [v[0] / len, v[1] / len, v[2] / len] }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[derive(Clone, Copy)]
struct Shard {
    spawn_frame: f32,
    lateral: [f32; 3],
}

pub struct IceArrowEffect {
    from: [f32; 3],
    to: [f32; 3],
    dir: [f32; 3],
    /// Perpendicular axes for the two cross planes.
    perp_a: [f32; 3],
    perp_b: [f32; 3],
    shards: [Shard; SHARD_COUNT],
    /// Frame at which the impact ring fires (first shard's travel time).
    ring_frame: f32,
    age_frames: f32,
}

impl IceArrowEffect {
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        let dir = norm([to[0] - from[0], to[1] - from[1], to[2] - from[2]]);
        // Two perpendiculars to the motion axis → the cross planes.
        let mut perp_a = cross(dir, [0.0, -1.0, 0.0]);
        if perp_a[0].abs() + perp_a[1].abs() + perp_a[2].abs() < 1e-4 {
            perp_a = cross(dir, [1.0, 0.0, 0.0]);
        }
        let perp_a = norm(perp_a);
        let perp_b = norm(cross(dir, perp_a));

        let mut state: u32 = 0x1CEA_8800;
        let mut lcg = || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((state >> 8) as f32 / ((1u32 << 24) as f32)) * 2.0 - 1.0
        };
        let shards = std::array::from_fn(|i| {
            let j1 = lcg() * SHARD_LATERAL_JITTER;
            let j2 = lcg() * SHARD_LATERAL_JITTER;
            Shard {
                spawn_frame: SPAWN_START_FRAME + SPAWN_PERIOD_FRAMES * i as f32,
                lateral: [
                    perp_a[0] * j1 + perp_b[0] * j2,
                    perp_a[1] * j1 + perp_b[1] * j2,
                    perp_a[2] * j1 + perp_b[2] * j2,
                ],
            }
        });

        let dist = {
            let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        };
        let travel_frames = if SHARD_SPEED_PER_FRAME > 0.0 {
            dist / SHARD_SPEED_PER_FRAME
        } else {
            SHARD_LIFE_FRAMES
        };
        Self {
            from,
            to,
            dir,
            perp_a,
            perp_b,
            shards,
            ring_frame: SPAWN_START_FRAME + travel_frames,
            age_frames: 0.0,
        }
    }

    /// Centre of a shard at the current age, or `None` if it isn't alive.
    fn shard_center(&self, shard: &Shard) -> Option<([f32; 3], f32)> {
        let life = self.age_frames - shard.spawn_frame;
        if life < 0.0 || life > SHARD_LIFE_FRAMES {
            return None;
        }
        let travelled = SHARD_SPEED_PER_FRAME * life;
        let center = [
            self.from[0] + shard.lateral[0] + self.dir[0] * travelled,
            self.from[1] + shard.lateral[1] - SPAWN_LIFT + self.dir[1] * travelled,
            self.from[2] + shard.lateral[2] + self.dir[2] * travelled,
        ];
        let alpha = if life < SHARD_FADE_START {
            1.0
        } else {
            (1.0 - (life - SHARD_FADE_START) / (SHARD_LIFE_FRAMES - SHARD_FADE_START)).clamp(0.0, 1.0)
        };
        Some((center, alpha))
    }

    fn push_plane(&self, out: &mut EffectDrawList, center: [f32; 3], perp: [f32; 3], alpha: f32) {
        let lx = self.dir[0] * SHARD_HALF_LEN;
        let ly = self.dir[1] * SHARD_HALF_LEN;
        let lz = self.dir[2] * SHARD_HALF_LEN;
        let wx = perp[0] * SHARD_HALF_WID;
        let wy = perp[1] * SHARD_HALF_WID;
        let wz = perp[2] * SHARD_HALF_WID;
        let corners = [
            [center[0] - lx - wx, center[1] - ly - wy, center[2] - lz - wz],
            [center[0] + lx - wx, center[1] + ly - wy, center[2] + lz - wz],
            [center[0] + lx + wx, center[1] + ly + wy, center[2] + lz + wz],
            [center[0] - lx + wx, center[1] - ly + wy, center[2] - lz + wz],
        ];
        out.push(EffectPrimitiveDraw::WorldQuad {
            corners,
            uv: UNIT_UV,
            texture: SHARD_TEXTURE,
            color: [SHARD_COLOR[0], SHARD_COLOR[1], SHARD_COLOR[2], alpha],
            blend: BlendKind::Additive,
            no_depth: false,
        });
    }
}

impl Effect for IceArrowEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for shard in &self.shards {
            if let Some((center, alpha)) = self.shard_center(shard) {
                if alpha <= 0.0 {
                    continue;
                }
                // Two orthogonal planes sharing the motion axis = a cross texture.
                self.push_plane(out, center, self.perp_a, alpha);
                self.push_plane(out, center, self.perp_b, alpha);
            }
        }

        let ring_age = self.age_frames - self.ring_frame;
        if ring_age >= 0.0 && ring_age <= RING_LIFE_FRAMES {
            let alpha = if ring_age < RING_FADE_IN {
                (ring_age / RING_FADE_IN) * RING_MAX_ALPHA
            } else if ring_age < RING_FADE_START {
                RING_MAX_ALPHA
            } else {
                (RING_MAX_ALPHA
                    * (1.0 - (ring_age - RING_FADE_START) / (RING_LIFE_FRAMES - RING_FADE_START)))
                    .max(0.0)
            };
            if alpha > 0.0 {
                let radius = RING_INNER + RING_GROWTH_PER_FRAME * ring_age + RING_THICKNESS;
                out.push(EffectPrimitiveDraw::GroundDisc {
                    center: self.to,
                    radius,
                    thickness: RING_THICKNESS,
                    rotation: 0.0,
                    arc_angle_deg: 360.0,
                    uv_repeat: 1.0,
                    texture: RING_TEXTURE,
                    color: [RING_COLOR[0], RING_COLOR[1], RING_COLOR[2], alpha],
                    blend: BlendKind::Additive,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut IceArrowEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        }
        s
    }

    fn shards(e: &IceArrowEffect) -> Vec<[f32; 3]> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        // Group the two planes per shard by averaging consecutive WorldQuad centres.
        let centers: Vec<[f32; 3]> = l.primitives.iter().filter_map(|p| match p {
            EffectPrimitiveDraw::WorldQuad { corners, .. } => Some([
                corners.iter().map(|c| c[0]).sum::<f32>() / 4.0,
                corners.iter().map(|c| c[1]).sum::<f32>() / 4.0,
                corners.iter().map(|c| c[2]).sum::<f32>() / 4.0,
            ]),
            _ => None,
        }).collect();
        centers
    }

    fn rings(e: &IceArrowEffect) -> Vec<f32> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives.iter().filter_map(|p| match p {
            EffectPrimitiveDraw::GroundDisc { radius, .. } => Some(*radius),
            _ => None,
        }).collect()
    }

    #[test]
    fn shards_emit_two_planes_each_and_travel_toward_target() {
        // Trail heading along +Z.
        let mut e = IceArrowEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 22.0]);
        step(&mut e, SPAWN_START_FRAME as u32 + 5);
        let centers = shards(&e);
        assert!(!centers.is_empty(), "first shard alive after frame 12");
        // Two WorldQuads per live shard.
        assert_eq!(centers.len() % 2, 0, "shards come in plane pairs");
        let early_z = centers[0][2];
        step(&mut e, 20);
        let late_z = shards(&e)[0][2];
        assert!(late_z > early_z, "shard travels toward +Z target: {early_z} -> {late_z}");
    }

    #[test]
    fn ring_appears_only_after_arrival_and_grows() {
        let mut e = IceArrowEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 22.0]);
        // Before the first shard could reach the target.
        step(&mut e, SPAWN_START_FRAME as u32 + 2);
        assert!(rings(&e).is_empty(), "no ring before arrival");
        // Past the arrival frame.
        let ring_frame = e.ring_frame.ceil() as u32;
        step(&mut e, ring_frame.saturating_sub(SPAWN_START_FRAME as u32 + 2) + 3);
        let r_early = rings(&e);
        assert_eq!(r_early.len(), 1, "exactly one impact ring");
        let early = r_early[0];
        step(&mut e, 8);
        let late = rings(&e)[0];
        assert!(late > early, "impact ring expands: {early} -> {late}");
    }

    #[test]
    fn terminates_after_the_last_shard_dies() {
        let mut e = IceArrowEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 22.0]);
        let status = step(&mut e, TOTAL_FRAMES as u32 + 2);
        assert_eq!(status, EffectStatus::Dead);
    }
}
