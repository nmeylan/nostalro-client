//! `EF_ICEARROW` (id 26, Cold Bolt) and `EF_FIREARROW` (id 31, Fire Bolt) — a
//! rain of cross-texture bolts that fall near-vertically onto the target.
//!
//! Both share one primitive pattern in the original game, so they're one struct
//! with two
//! `const` parameter sets here.
//!
//!   * Anchored at the target. The count is the spell level; from frame 12 it is
//!     scaled and one cross-texture bolt is launched every 10 frames while
//!     the launched count is below it — so the bolt count equals the level (our
//!     `hit_count`).
//!   * Each bolt spawns at `target + (30±5, -60, 20±5)` (RO −Y is up, so ~60
//!     units above and slightly to one corner) and travels at `2.75`/frame along
//!     the fall direction onto the target — a steep, near-vertical streak. It
//!     lives 70 frames and fades from frame 50. A cross texture is two
//!     orthogonal planes sharing the fall axis; we emit two `WorldQuad`s.
//!   * Ice: `11.5 × 3.8`, single `icearrow.tga`, blue `ring_blue.tga` impact ring.
//!     Fire: `14 × 3.5`, six animated `불화살N.tga` frames, yellow `ring_yellow.tga`
//!     ring, plus a `particle4.spr` spark spray (every 4 frames, along the fall
//!     direction).
//!   * Impact ring (flat): inner 10, grows `1.2`/frame, 30-frame
//!     life, fades over its last 10 frames. One per bolt as it lands.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Effect literals are 1:1 with our world units, so the bolt renders at
/// the original game's size with `WORLD_SCALE = 1.0`. (Kept as a single knob;
/// impact timing is invariant under it.)
const WORLD_SCALE: f32 = 0.7;

/// One bolt every 10 frames from frame 12; count = `hit_count`.
const SPAWN_START_FRAME: f32 = 12.0;
const SPAWN_PERIOD_FRAMES: f32 = 10.0;
const MAX_BOLTS: usize = 10;

const BOLT_LIFE_FRAMES: f32 = 70.0;
const BOLT_FADE_START: f32 = 50.0;
const BOLT_SPEED_PER_FRAME: f32 = 2.75 * WORLD_SCALE;

/// Spawn offset above the target (`-Y` up), shared corner with per-bolt jitter.
const OFFSET_BASE: [f32; 3] = [30.0, -60.0, 20.0];
const OFFSET_JITTER: f32 = 5.0;

// Impact ring — a textured annulus whose `ring_*.tga` is a
// repeating spike/corona gradient (tips at the outer edge, tiled 4× around), so
// the band thickness is the spike length. The outer radius integrates a
// decelerating speed; the band is capped at `RING_INNER_SIZE`. At 1:1 the
// corona reaches ~20 units — too large for a bolt splash, so the whole ring is
// downscaled (the bolt stays full-size).
const RING_SCALE: f32 = 0.45;
const RING_LIFE_FRAMES: f32 = 30.0;
/// The band's max radial thickness (spike length).
const RING_INNER_SIZE: f32 = 10.0 * RING_SCALE;
/// Ring growth speed and its (negative) acceleration `-(speed/(dur+40))*2`.
const RING_SPEED0: f32 = 1.2;
const RING_ACCEL: f32 = -RING_SPEED0 / 35.0;
/// The ring texture tiles four times around the circle.
const RING_UV_REPEAT: f32 = 4.0;
const RING_FADE_IN: f32 = 10.0;
const RING_FADE_START: f32 = RING_LIFE_FRAMES - 10.0;
const RING_MAX_ALPHA: f32 = 1.0;
/// `-Y` is up: lift the flat ring a hair off the ground so the terrain doesn't
/// depth-occlude ("swallow") it at grazing camera angles.
const RING_GROUND_LIFT: f32 = -0.5;

// Fire-only spark spray.
const SPRAY_FIRST_FRAME: f32 = 4.0;
const SPRAY_INTERVAL: f32 = 4.0;
const SPRAY_MIN_DURATION: f32 = 6.0;
const SPRAY_MAX_DURATION: f32 = 30.0;
const SPRAY_MIN_SPEED: f32 = 0.6 * WORLD_SCALE;
const SPRAY_MAX_SPEED: f32 = 1.5 * WORLD_SCALE;
const SPRAY_MIN_SIZE: f32 = 0.2;
const SPRAY_MAX_SIZE: f32 = 0.5;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

const ICE_FRAMES: &[&str] = &["icearrow.tga"];
const FIRE_FRAMES: &[&str] = &[
    "불화살1.tga",
    "불화살2.tga",
    "불화살3.tga",
    "불화살4.tga",
    "불화살5.tga",
    "불화살6.tga",
];
const PARTICLE4_SPRITE: &str = "data/sprite/이팩트/particle4";

pub const ICE_TEXTURES: &[&str] = &["icearrow.tga", "ring_blue.tga"];
pub const FIRE_TEXTURES: &[&str] = &[
    "불화살1.tga",
    "불화살2.tga",
    "불화살3.tga",
    "불화살4.tga",
    "불화살5.tga",
    "불화살6.tga",
    "ring_yellow.tga",
];
pub const FIRE_SPRITES: &[&str] = &[PARTICLE4_SPRITE];

/// Worst-case wall clock (max bolt count) so the holder never cuts a high-level
/// cast short; a low-`hit_count` instance ends itself earlier via `update`.
const MAX_TOTAL_FRAMES: f32 =
    SPAWN_START_FRAME + SPAWN_PERIOD_FRAMES * (MAX_BOLTS as f32 - 1.0) + BOLT_LIFE_FRAMES;
pub const ICE_TOTAL_DURATION_MS: u32 = (MAX_TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;
pub const FIRE_TOTAL_DURATION_MS: u32 = ICE_TOTAL_DURATION_MS;

#[derive(Clone, Copy)]
pub struct BoltParams {
    /// One frame (ice) or the animation set (fire), cycled at 1 frame/tick.
    frames: &'static [&'static str],
    /// Half-extents: `len` along the fall axis, `wid` perpendicular (raw source
    /// literals, scaled by [`WORLD_SCALE`] at emit time).
    half_len: f32,
    half_wid: f32,
    bolt_color: [f32; 3],
    ring_texture: &'static str,
    ring_color: [f32; 3],
    spray: bool,
}

pub const ICE_ARROW: BoltParams = BoltParams {
    frames: ICE_FRAMES,
    half_len: 11.5,
    half_wid: 3.8,
    bolt_color: [0.7, 0.85, 1.0],
    ring_texture: "ring_blue.tga",
    ring_color: [0.6, 0.8, 1.0],
    spray: false,
};

pub const FIRE_ARROW: BoltParams = BoltParams {
    frames: FIRE_FRAMES,
    half_len: 14.0,
    half_wid: 3.5,
    bolt_color: [1.0, 0.85, 0.4],
    ring_texture: "ring_yellow.tga",
    ring_color: [1.0, 0.8, 0.3],
    spray: true,
};

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

struct Bolt {
    spawn_frame: f32,
    /// Start position relative to the target.
    offset: [f32; 3],
    fall_dir: [f32; 3],
    /// Cross-plane axes perpendicular to the fall direction.
    perp_a: [f32; 3],
    perp_b: [f32; 3],
    /// Frame (relative to spawn) at which the bolt reaches the target.
    impact_life: f32,
}

struct SprayParticle {
    spawn_frame: f32,
    dir: [f32; 3],
    speed: f32,
    accel: f32,
    init_size: f32,
    size_speed: f32,
    duration: f32,
}

pub struct MagicBoltEffect {
    target: [f32; 3],
    params: BoltParams,
    bolts: Vec<Bolt>,
    spray: Vec<SprayParticle>,
    total_frames: f32,
    age_frames: f32,
}

impl MagicBoltEffect {
    pub fn new(target: [f32; 3], hit_count: u8, params: BoltParams) -> Self {
        let n = (hit_count as usize).clamp(1, MAX_BOLTS);

        let mut state: u32 = 0x1CEA_8800 ^ (params.frames.len() as u32).wrapping_mul(0x9E37_79B9);
        let mut lcg = || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) as f32 / ((1u32 << 24) as f32)
        };

        let bolts = (0..n)
            .map(|i| {
                let offset = [
                    (OFFSET_BASE[0] + (lcg() * 2.0 - 1.0) * OFFSET_JITTER) * WORLD_SCALE,
                    OFFSET_BASE[1] * WORLD_SCALE,
                    (OFFSET_BASE[2] + (lcg() * 2.0 - 1.0) * OFFSET_JITTER) * WORLD_SCALE,
                ];
                let dist = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2])
                    .sqrt();
                let fall_dir = [-offset[0] / dist, -offset[1] / dist, -offset[2] / dist];
                let mut perp_a = cross(fall_dir, [0.0, -1.0, 0.0]);
                if perp_a[0].abs() + perp_a[1].abs() + perp_a[2].abs() < 1e-4 {
                    perp_a = cross(fall_dir, [1.0, 0.0, 0.0]);
                }
                let perp_a = norm(perp_a);
                let perp_b = norm(cross(fall_dir, perp_a));
                Bolt {
                    spawn_frame: SPAWN_START_FRAME + SPAWN_PERIOD_FRAMES * i as f32,
                    offset,
                    fall_dir,
                    perp_a,
                    perp_b,
                    impact_life: dist / BOLT_SPEED_PER_FRAME,
                }
            })
            .collect();

        let last_spawn = SPAWN_START_FRAME + SPAWN_PERIOD_FRAMES * (n as f32 - 1.0);
        let total_frames = last_spawn + BOLT_LIFE_FRAMES;

        let spray = if params.spray {
            let global_fall = norm([-OFFSET_BASE[0], -OFFSET_BASE[1], -OFFSET_BASE[2]]);
            let mut v = Vec::new();
            let mut frame = SPRAY_FIRST_FRAME;
            // Sparks accompany the falling bolts; stop once the last one has landed.
            while frame <= last_spawn + BOLT_LIFE_FRAMES * 0.5 {
                // Jitter the fall direction by a small cone.
                let jx = (lcg() * 2.0 - 1.0) * 0.35;
                let jy = (lcg() * 2.0 - 1.0) * 0.2;
                let jz = (lcg() * 2.0 - 1.0) * 0.35;
                let dir = norm([
                    global_fall[0] + jx,
                    global_fall[1] + jy,
                    global_fall[2] + jz,
                ]);
                let duration =
                    SPRAY_MIN_DURATION + lcg() * (SPRAY_MAX_DURATION - SPRAY_MIN_DURATION);
                let speed = SPRAY_MIN_SPEED + lcg() * (SPRAY_MAX_SPEED - SPRAY_MIN_SPEED);
                let init_size = SPRAY_MIN_SIZE + lcg() * (SPRAY_MAX_SIZE - SPRAY_MIN_SIZE);
                v.push(SprayParticle {
                    spawn_frame: frame,
                    dir,
                    speed,
                    accel: -(speed / duration) / 1.5,
                    init_size,
                    size_speed: -(init_size / duration),
                    duration,
                });
                frame += SPRAY_INTERVAL;
            }
            v
        } else {
            Vec::new()
        };

        Self {
            target,
            params,
            bolts,
            spray,
            total_frames,
            age_frames: 0.0,
        }
    }

    fn push_plane(&self, out: &mut EffectDrawList, bolt: &Bolt, center: [f32; 3], perp: [f32; 3], alpha: f32, texture: &'static str) {
        let hl = self.params.half_len * WORLD_SCALE;
        let hw = self.params.half_wid * WORLD_SCALE;
        let l = [bolt.fall_dir[0] * hl, bolt.fall_dir[1] * hl, bolt.fall_dir[2] * hl];
        let w = [perp[0] * hw, perp[1] * hw, perp[2] * hw];
        let corners = [
            [center[0] - l[0] - w[0], center[1] - l[1] - w[1], center[2] - l[2] - w[2]],
            [center[0] + l[0] - w[0], center[1] + l[1] - w[1], center[2] + l[2] - w[2]],
            [center[0] + l[0] + w[0], center[1] + l[1] + w[1], center[2] + l[2] + w[2]],
            [center[0] - l[0] + w[0], center[1] - l[1] + w[1], center[2] - l[2] + w[2]],
        ];
        out.push(EffectPrimitiveDraw::WorldQuad {
            corners,
            uv: UNIT_UV,
            texture,
            color: [self.params.bolt_color[0], self.params.bolt_color[1], self.params.bolt_color[2], alpha],
            blend: BlendKind::Additive,
            no_depth: false,
        });
    }

    fn push_ring(&self, out: &mut EffectDrawList, ring_age: f32) {
        let alpha = if ring_age < RING_FADE_IN {
            (ring_age / RING_FADE_IN) * RING_MAX_ALPHA
        } else if ring_age < RING_FADE_START {
            RING_MAX_ALPHA
        } else {
            (RING_MAX_ALPHA
                * (1.0 - (ring_age - RING_FADE_START) / (RING_LIFE_FRAMES - RING_FADE_START)))
                .max(0.0)
        };
        if alpha <= 0.0 {
            return;
        }
        // Outer radius integrates a decelerating speed; the band thickness is
        // `min(radius, innerSize)` so it fills as a disc
        // until `innerSize`, then holds that width as the ring widens.
        let outer =
            (RING_SPEED0 * ring_age + 0.5 * RING_ACCEL * ring_age * ring_age) * RING_SCALE;
        if outer <= 0.0 {
            return;
        }
        let thickness = outer.min(RING_INNER_SIZE);
        let center = [self.target[0], self.target[1] + RING_GROUND_LIFT, self.target[2]];
        out.push(EffectPrimitiveDraw::GroundDisc {
            center,
            radius: outer,
            thickness,
            rotation: 0.0,
            arc_angle_deg: 360.0,
            uv_repeat: RING_UV_REPEAT,
            texture: self.params.ring_texture,
            color: [self.params.ring_color[0], self.params.ring_color[1], self.params.ring_color[2], alpha],
            blend: BlendKind::Additive,
        });
    }
}

impl Effect for MagicBoltEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= self.total_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for bolt in &self.bolts {
            let life = self.age_frames - bolt.spawn_frame;
            if life < 0.0 || life > BOLT_LIFE_FRAMES {
                continue;
            }
            let travelled = BOLT_SPEED_PER_FRAME * life;
            let center = [
                self.target[0] + bolt.offset[0] + bolt.fall_dir[0] * travelled,
                self.target[1] + bolt.offset[1] + bolt.fall_dir[1] * travelled,
                self.target[2] + bolt.offset[2] + bolt.fall_dir[2] * travelled,
            ];
            let alpha = if life < BOLT_FADE_START {
                1.0
            } else {
                (1.0 - (life - BOLT_FADE_START) / (BOLT_LIFE_FRAMES - BOLT_FADE_START)).max(0.0)
            };
            if alpha > 0.0 {
                let frame = self.params.frames[(life as usize) % self.params.frames.len()];
                // Two orthogonal planes sharing the fall axis = a cross texture.
                self.push_plane(out, bolt, center, bolt.perp_a, alpha, frame);
                self.push_plane(out, bolt, center, bolt.perp_b, alpha, frame);
            }

            // Impact ring once the bolt reaches the target.
            let ring_age = life - bolt.impact_life;
            if ring_age >= 0.0 && ring_age <= RING_LIFE_FRAMES {
                self.push_ring(out, ring_age);
            }
        }

        for p in &self.spray {
            let life = self.age_frames - p.spawn_frame;
            if life < 0.0 || life >= p.duration {
                continue;
            }
            let dist = p.speed * life + 0.5 * p.accel * life * life;
            let pos = [
                self.target[0] + p.dir[0] * dist,
                self.target[1] + p.dir[1] * dist,
                self.target[2] + p.dir[2] * dist,
            ];
            let size = (p.init_size + p.size_speed * life).max(0.0);
            let alpha = (1.0 - life / p.duration).clamp(0.0, 1.0);
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: PARTICLE4_SPRITE,
                position: pos,
                action_index: 0,
                motion_index: (life * 2.0) as usize,
                size_scale: size,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
                aim_target: None,
                no_depth: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut MagicBoltEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        s
    }

    fn quads(e: &MagicBoltEffect) -> Vec<[[f32; 3]; 4]> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::WorldQuad { corners, .. } => Some(*corners),
                _ => None,
            })
            .collect()
    }

    fn quad_center(q: &[[f32; 3]; 4]) -> [f32; 3] {
        [
            q.iter().map(|c| c[0]).sum::<f32>() / 4.0,
            q.iter().map(|c| c[1]).sum::<f32>() / 4.0,
            q.iter().map(|c| c[2]).sum::<f32>() / 4.0,
        ]
    }

    #[test]
    fn bolt_count_tracks_hit_count_with_two_planes_each() {
        for count in 1..=5u8 {
            // Count distinct live bolts at the moment the last one has spawned.
            let mut e = MagicBoltEffect::new([0.0, 0.0, 0.0], count, ICE_ARROW);
            let last_spawn = SPAWN_START_FRAME + SPAWN_PERIOD_FRAMES * (count as f32 - 1.0);
            step(&mut e, last_spawn as u32 + 2);
            let q = quads(&e);
            // Two planes per still-alive bolt; all bolts are within their 70-frame
            // life this early, so every spawned bolt is present.
            assert_eq!(q.len(), count as usize * 2, "hit_count={count} → {count} bolts × 2 planes");
        }
    }

    #[test]
    fn bolts_fall_from_above_onto_the_target() {
        let target = [0.0, 0.0, 0.0];
        let mut e = MagicBoltEffect::new(target, 1, ICE_ARROW);
        let spawn = SPAWN_START_FRAME as u32 + 1;
        step(&mut e, spawn);
        let early = quad_center(&quads(&e)[0]);
        // -Y is up: a fresh bolt starts above the target (negative y).
        assert!(early[1] < target[1], "bolt starts above the target: y={}", early[1]);
        // Advance to the frame the bolt reaches the target.
        let impact_frame = (SPAWN_START_FRAME + e.bolts[0].impact_life).round() as u32;
        step(&mut e, impact_frame - spawn);
        let landed = quad_center(&quads(&e)[0]);
        assert!(landed[1] > early[1], "bolt descends toward the ground: {} -> {}", early[1], landed[1]);
        let dxz = (landed[0] * landed[0] + landed[2] * landed[2]).sqrt();
        assert!(dxz < 1.5, "bolt converges on the target in XZ: {dxz}");
    }

    #[test]
    fn fire_sprays_particles_and_cycles_frames_ice_does_not() {
        let mut fire = MagicBoltEffect::new([0.0; 3], 1, FIRE_ARROW);
        let mut ice = MagicBoltEffect::new([0.0; 3], 1, ICE_ARROW);
        step(&mut fire, 20);
        step(&mut ice, 20);

        let mut fl = EffectDrawList::new();
        fire.collect_draws(&mut fl, &render_ctx());
        let sprites = fl.primitives.iter().filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. })).count();
        let fire_tex: Vec<&str> = fl.primitives.iter().filter_map(|p| match p {
            EffectPrimitiveDraw::WorldQuad { texture, .. } => Some(*texture),
            _ => None,
        }).collect();
        assert!(sprites > 0, "fire emits a spark spray");
        assert!(fire_tex.iter().all(|t| FIRE_FRAMES.contains(t)), "fire bolt uses the animated flame frames");

        let mut il = EffectDrawList::new();
        ice.collect_draws(&mut il, &render_ctx());
        let ice_sprites = il.primitives.iter().filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. })).count();
        assert_eq!(ice_sprites, 0, "ice has no spray");
    }

    #[test]
    fn terminates_after_its_bolts_die() {
        let mut e = MagicBoltEffect::new([0.0; 3], 3, FIRE_ARROW);
        let total = e.total_frames as u32 + 2;
        let status = step(&mut e, total);
        assert_eq!(status, EffectStatus::Dead);
    }
}
