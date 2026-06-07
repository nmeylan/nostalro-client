//! EF_PIERCE — repeated horizontal cylinder rings sliding along the strike
//! axis, plus radial particle bursts.
//!
//! Reference: `ro-effects/effects/imgs/50-100/81.gif`.
//!
//! The original fires a burst every
//! 20 parent frames while the count is below `level * 20`. Each burst emits:
//!
//! * One horizontal cylinder ring, laid on its side and yawed by
//!   `180 − target_heading`.
//!   Its velocity is `(0, −speed, 0)` rotated by that orientation, so the
//!   cylinder slides along the strike heading at 0.7 → 0 over 15 frames.
//!   Outer radius `11 + 0.1·frame`; inner radius `6 + 0.1·frame`
//!   (each successive burst is fractionally wider). Height 2.5.
//!   Texture `ring_yellow.tga`.
//! * On every burst **after** the first: 4× particles (forward cone
//!   spread, no gravity) and 4× particles (rear cone, falls).
//!   Both use the `particle1` sprite.
//!
//! The target's facing drives the strike heading in the original game.
//! We reconstruct the same heading from the caster→target vector
//! (`atan2(to.x − from.x, to.z − from.z)`); single-point anchors collapse
//! to heading 0.
//!
//! Skill level isn't yet carried through the spawn pipeline as its own
//! field, but the `hit_count` parameter on `SpawnRequest` (used by
//! multi-bolt skills like Soul Strike) is plumbed end-to-end and maps
//! naturally to Pierce's level. Level 1 fires one burst; level N
//! fires N bursts spaced 20 frames apart.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_yellow.tga";
pub const PARTICLE_SPRITE: &str = "data/sprite/\u{c774}\u{d31d}\u{d2b8}/particle1";
pub const TEXTURES: &[&str] = &[RING_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const SIDES: u32 = 10;

const BURST_INTERVAL_FRAMES: f32 = 20.0;
const CYLINDER_LIFETIME_FRAMES: f32 = 15.0;
const CYLINDER_INITIAL_SPEED: f32 = 0.7;
const CYLINDER_INITIAL_OUTER: f32 = 11.0;
const CYLINDER_INITIAL_INNER: f32 = 6.0;
const CYLINDER_GROWTH_PER_BURST: f32 = 0.1 * BURST_INTERVAL_FRAMES;
const CYLINDER_HEIGHT: f32 = 2.5;
/// The cylinder centre sits 10 units above the target in native RO
/// coords — a pure Y offset applied before the orientation is built.
const CYLINDER_PIVOT_Y_OFFSET: f32 = -10.0;
const RING_ALPHA: f32 = 1.0;

const PARTICLES_PER_DIR: usize = 4;
const PARTICLE_LIFETIME_MIN_FRAMES: f32 = 6.0;
const PARTICLE_LIFETIME_MAX_FRAMES: f32 = 30.0;
const PARTICLE_SIZE_MIN: f32 = 0.6;
const PARTICLE_SIZE_MAX: f32 = 1.6;
const PARTICLE_SPEED_MIN: f32 = 0.6;
const PARTICLE_SPEED_MAX: f32 = 1.5;
/// Outer cone half-angle (latitudes 40-140°).
const PARTICLE_CONE_LAT_MIN_DEG: f32 = 40.0;
const PARTICLE_CONE_LAT_MAX_DEG: f32 = 140.0;
/// Heading spread around the strike axis (`angle ± 40°`).
const PARTICLE_HEADING_SPREAD_DEG: f32 = 40.0;
/// Gravity ≈ −0.3..−1.2 per frame with deceleration in the original game. We
/// approximate the average integrated drop with a constant downward accel
/// applied each frame.
const PARTICLE_GRAVITY_PER_FRAME: f32 = -0.012;
const PARTICLE_MAX_LIFE_FRAMES: f32 = PARTICLE_LIFETIME_MAX_FRAMES;

const DEFAULT_LEVEL: u8 = 1;
const MAX_LEVEL: u8 = 10;

/// Parent emitter holds until the last burst's particles finish. With
/// level N: last burst spawns at (N-1) × 20 frames; particles live up to
/// PARTICLE_MAX_LIFE_FRAMES after that.
pub const TOTAL_DURATION_MS: u32 = ((MAX_LEVEL as f32 - 1.0) * BURST_INTERVAL_FRAMES
    + CYLINDER_LIFETIME_FRAMES
    + PARTICLE_MAX_LIFE_FRAMES) as u32
    * 1000
    / FRAMES_PER_SECOND as u32;

/// Deterministic PRNG so unit tests reproduce frames.
#[derive(Clone, Copy)]
struct Rng(u32);

impl Rng {
    fn from_seed(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9).wrapping_add(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let r = self.next_u32() as f32 / u32::MAX as f32;
        lo + (hi - lo) * r
    }
}

#[derive(Clone, Copy, Debug)]
struct Cylinder {
    spawn_frame: f32,
    outer0: f32,
    inner0: f32,
    pivot: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    spawn_frame: f32,
    origin: [f32; 3],
    velocity: [f32; 3],
    gravity: bool,
    lifetime_frames: f32,
    size: f32,
}

impl Particle {
    fn alive_at(&self, frame: f32) -> Option<f32> {
        let local = frame - self.spawn_frame;
        if local < 0.0 || local >= self.lifetime_frames {
            None
        } else {
            Some(local)
        }
    }
    fn position(&self, local: f32) -> [f32; 3] {
        let gravity_dy = if self.gravity {
            PARTICLE_GRAVITY_PER_FRAME * local * (local + 1.0) * 0.5
        } else {
            0.0
        };
        [
            self.origin[0] + self.velocity[0] * local,
            self.origin[1] + self.velocity[1] * local + gravity_dy,
            self.origin[2] + self.velocity[2] * local,
        ]
    }
    fn alpha(&self, local: f32) -> f32 {
        let ramp_in = self.lifetime_frames * 0.1;
        if local < ramp_in {
            local / ramp_in
        } else {
            (1.0 - (local - ramp_in) / (self.lifetime_frames - ramp_in)).max(0.0)
        }
    }
    fn size_at(&self, local: f32) -> f32 {
        (self.size * (1.0 - local / self.lifetime_frames)).max(0.0)
    }
}

pub struct PierceEffect {
    target_pos: [f32; 3],
    heading_rad: f32,
    level: u8,
    age: f32,
    /// Next burst index that should still be emitted (`0..level`).
    next_burst_idx: u8,
    cylinders: Vec<Cylinder>,
    particles: Vec<Particle>,
    rng: Rng,
}

impl PierceEffect {
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        Self::new_with_level(from, to, DEFAULT_LEVEL)
    }

    pub fn new_with_level(from: [f32; 3], to: [f32; 3], level: u8) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let heading_rad = if dx.abs() < 1e-4 && dz.abs() < 1e-4 {
            0.0
        } else {
            dx.atan2(dz)
        };
        let level = level.clamp(1, MAX_LEVEL);
        let seed = (to[0].to_bits() ^ to[2].to_bits()) ^ heading_rad.to_bits();
        Self {
            target_pos: to,
            heading_rad,
            level,
            age: 0.0,
            next_burst_idx: 0,
            cylinders: Vec::with_capacity(level as usize),
            particles: Vec::new(),
            rng: Rng::from_seed(seed),
        }
    }

    /// Pivot point — `target + (0, -10, 0)` in native RO coords (`-Y` is
    /// up, so this lifts the ring 10 units above the target). The
    /// offset is applied to the target position *before*
    /// the rotation is built, so it stays a pure Y offset and
    /// does **not** rotate with `heading`.
    fn cylinder_pivot(&self) -> [f32; 3] {
        [
            self.target_pos[0],
            self.target_pos[1] + CYLINDER_PIVOT_Y_OFFSET,
            self.target_pos[2],
        ]
    }

    fn spawn_burst(&mut self, burst_idx: u8, frame: f32) {
        let pivot = self.cylinder_pivot();
        let grow = burst_idx as f32 * CYLINDER_GROWTH_PER_BURST;
        self.cylinders.push(Cylinder {
            spawn_frame: frame,
            outer0: CYLINDER_INITIAL_OUTER + grow,
            inner0: CYLINDER_INITIAL_INNER + grow,
            pivot,
        });

        // Bursts after the first add the radial particle storm.
        if burst_idx == 0 {
            return;
        }
        let strike_heading_deg = self.heading_rad.to_degrees();
        for forward in [true, false] {
            let centre_deg = if forward {
                strike_heading_deg
            } else {
                strike_heading_deg + 180.0
            };
            for _ in 0..PARTICLES_PER_DIR {
                let yaw_deg = centre_deg
                    + self
                        .rng
                        .range_f32(-PARTICLE_HEADING_SPREAD_DEG, PARTICLE_HEADING_SPREAD_DEG);
                let lat_deg =
                    self.rng.range_f32(PARTICLE_CONE_LAT_MIN_DEG, PARTICLE_CONE_LAT_MAX_DEG);
                let yaw_rad = yaw_deg.to_radians();
                let lat_rad = lat_deg.to_radians();
                let speed = self.rng.range_f32(PARTICLE_SPEED_MIN, PARTICLE_SPEED_MAX);
                let (sin_y, cos_y) = yaw_rad.sin_cos();
                let dir = [
                    sin_y * lat_rad.sin(),
                    -lat_rad.cos(),
                    cos_y * lat_rad.sin(),
                ];
                let lifetime = self
                    .rng
                    .range_f32(PARTICLE_LIFETIME_MIN_FRAMES, PARTICLE_LIFETIME_MAX_FRAMES);
                let size = self.rng.range_f32(PARTICLE_SIZE_MIN, PARTICLE_SIZE_MAX);
                self.particles.push(Particle {
                    spawn_frame: frame,
                    origin: pivot,
                    velocity: [dir[0] * speed, dir[1] * speed, dir[2] * speed],
                    gravity: !forward,
                    lifetime_frames: lifetime,
                    size,
                });
            }
        }
    }
}

impl Effect for PierceEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        let frame = self.age * FRAMES_PER_SECOND;

        while self.next_burst_idx < self.level {
            let spawn_at = self.next_burst_idx as f32 * BURST_INTERVAL_FRAMES;
            if frame < spawn_at {
                break;
            }
            self.spawn_burst(self.next_burst_idx, spawn_at);
            self.next_burst_idx += 1;
        }

        self.cylinders
            .retain(|c| frame - c.spawn_frame < CYLINDER_LIFETIME_FRAMES);
        self.particles.retain(|p| p.alive_at(frame).is_some());

        let last_spawn = (self.level as f32 - 1.0) * BURST_INTERVAL_FRAMES;
        if frame >= last_spawn + CYLINDER_LIFETIME_FRAMES + PARTICLE_MAX_LIFE_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;

        for c in &self.cylinders {
            let local = frame - c.spawn_frame;
            if local < 0.0 || local >= CYLINDER_LIFETIME_FRAMES {
                continue;
            }
            // Speed decays each frame by `-(speed / duration) / 2.0`,
            // giving a velocity that ramps down smoothly. Closed-form
            // displacement is
            // `v0 * (t - t²/(4·T))` (since accel is half the linear
            // ramp rate). Match the integration with discrete `t · (t+1)/2`.
            let t = local;
            let cap = CYLINDER_LIFETIME_FRAMES;
            let s = CYLINDER_INITIAL_SPEED
                * (t - t * (t + 1.0) / (4.0 * cap));
            let (sin_h, cos_h) = self.heading_rad.sin_cos();
            let centre = [
                c.pivot[0] + s * sin_h,
                c.pivot[1],
                c.pivot[2] + s * cos_h,
            ];

            let fade_start = CYLINDER_LIFETIME_FRAMES - CYLINDER_LIFETIME_FRAMES / 2.0;
            let alpha = if local < fade_start {
                RING_ALPHA
            } else {
                let span = CYLINDER_LIFETIME_FRAMES - fade_start;
                RING_ALPHA * (1.0 - (local - fade_start) / span).max(0.0)
            };

            // Tilt/yaw conventions: the renderer's `transform_local`
            // uses opposite signs to the "rotate -90° about X,
            // then yaw" ordering. Compensate at the call site (rather than
            // changing the shared renderer) so the other effects that
            // already work with `tilt_x_rad = 0` keep their behaviour.
            // Net effect: top-ring offset = `(h·sin(heading), 0,
            // h·cos(heading))`, as the original game shows.
            out.push(EffectPrimitiveDraw::Cylinder {
                base: centre,
                bottom_size: c.inner0,
                top_size: c.outer0,
                height: CYLINDER_HEIGHT,
                sides: SIDES,
                rotation: 0.0,
                tilt_x_rad: std::f32::consts::FRAC_PI_2,
                rotation_y_rad: -self.heading_rad,
                uv_scroll: [0.0, 0.0],
                texture: RING_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
            });
        }

        for p in &self.particles {
            let Some(local) = p.alive_at(frame) else {
                continue;
            };
            let alpha = p.alpha(local);
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: PARTICLE_SPRITE,
                position: p.position(local),
                action_index: 0,
                motion_index: local as usize,
                size_scale: p.size_at(local),
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
                aim_target: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_and_collect(e: &mut PierceEffect, dt: f32) -> Vec<EffectPrimitiveDraw> {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None, caster_yaw: None,
        });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &ctx());
        list.primitives
    }

    #[test]
    fn cylinder_aims_along_caster_to_target_direction() {
        // Caster at origin, target 10 units along +X. Strike heading is
        // atan2(10, 0) = π/2. Tilt is always -π/2 (axis horizontal);
        // pivot is target + (0, -10, 0) in native RO coords (no
        // horizontal offset — the Y lift is applied pre-rotation).
        let mut e = PierceEffect::new([0.0; 3], [10.0, 0.0, 0.0]);
        let prims = step_and_collect(&mut e, 1.0 / FRAMES_PER_SECOND);
        let (tilt, yaw, base) = prims
            .iter()
            .find_map(|p| match p {
                EffectPrimitiveDraw::Cylinder {
                    tilt_x_rad,
                    rotation_y_rad,
                    base,
                    ..
                } => Some((*tilt_x_rad, *rotation_y_rad, *base)),
                _ => None,
            })
            .expect("pierce emits a cylinder");
        // Tilt = +π/2 (axis horizontal); yaw = -heading to compensate
        // for the renderer's flipped sign conventions.
        assert!((tilt - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!((yaw + std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        // Bottom-ring centre = pivot + a small slide along heading at
        // frame 1. Pivot = (target.x, target.y - 10, target.z) =
        // (10, -10, 0); base.x stays at 10 (slide is along +X but small);
        // base.y is the elevated -10.
        assert!((base[1] - (-CYLINDER_PIVOT_Y_OFFSET.abs())).abs() < 0.1);
        assert!((base[0] - 10.0).abs() < 1.0);
    }

    #[test]
    fn level_n_emits_n_cylinders_with_first_skipping_particles() {
        // Sociable test: covers burst scheduling, particle skip on the
        // first burst, and particle emission on subsequent bursts.
        let mut e = PierceEffect::new_with_level([0.0; 3], [10.0, 0.0, 0.0], 3);

        // Frame ~1 → first burst only, no particles yet.
        let p1 = step_and_collect(&mut e, 1.0 / FRAMES_PER_SECOND);
        let cyl1 = p1
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Cylinder { .. }))
            .count();
        let part1 = p1
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        assert_eq!(cyl1, 1);
        assert_eq!(part1, 0, "first burst has no particles");

        // Frame ~21 → second burst landed: now 2 cylinders + 8 particles
        // (4 forward + 4 gravity). The first cylinder is still in its
        // 15-frame window only at frame 20 boundary; conservatively just
        // check the new particle count.
        let p21 = step_and_collect(&mut e, 20.0 / FRAMES_PER_SECOND);
        let part21 = p21
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        assert_eq!(part21, 2 * PARTICLES_PER_DIR, "burst #2 spawned 8 particles");
    }

    #[test]
    fn dies_after_last_burst_finishes() {
        let mut e = PierceEffect::new([0.0; 3], [10.0, 0.0, 0.0]);
        let total_s = TOTAL_DURATION_MS as f32 / 1000.0;
        let s = e.update(&EffectUpdateCtx {
            delta: total_s + 0.5,
            camera_target: None, caster_yaw: None,
        });
        assert!(matches!(s, EffectStatus::Dead));
    }
}
