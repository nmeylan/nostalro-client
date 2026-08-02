//! EF_PIERCE — horizontal cylinder rings sliding along the strike axis, plus radial particle bursts.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_yellow.tga";
pub const PARTICLE_SPRITE: &str = ragnarok_resources::sprite::effect::PARTICLE1;
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
const CYLINDER_PIVOT_Y_OFFSET: f32 = -10.0; // −Y is up.
const RING_ALPHA: f32 = 1.0;

const PARTICLES_PER_DIR: usize = 4;
const PARTICLE_LIFETIME_MIN_FRAMES: f32 = 6.0;
const PARTICLE_LIFETIME_MAX_FRAMES: f32 = 30.0;
const PARTICLE_SIZE_MIN: f32 = 0.6;
const PARTICLE_SIZE_MAX: f32 = 1.6;
const PARTICLE_SPEED_MIN: f32 = 0.6;
const PARTICLE_SPEED_MAX: f32 = 1.5;
const PARTICLE_CONE_LAT_MIN_DEG: f32 = 40.0;
const PARTICLE_CONE_LAT_MAX_DEG: f32 = 140.0;
const PARTICLE_HEADING_SPREAD_DEG: f32 = 40.0;
const PARTICLE_GRAVITY_PER_FRAME: f32 = -0.012;
const PARTICLE_MAX_LIFE_FRAMES: f32 = PARTICLE_LIFETIME_MAX_FRAMES;

const DEFAULT_LEVEL: u8 = 1;
const MAX_LEVEL: u8 = 10;

pub const TOTAL_DURATION_MS: u32 = ((MAX_LEVEL as f32 - 1.0) * BURST_INTERVAL_FRAMES
    + CYLINDER_LIFETIME_FRAMES
    + PARTICLE_MAX_LIFE_FRAMES) as u32
    * 1000
    / FRAMES_PER_SECOND as u32;

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
                let lat_deg = self
                    .rng
                    .range_f32(PARTICLE_CONE_LAT_MIN_DEG, PARTICLE_CONE_LAT_MAX_DEG);
                let yaw_rad = yaw_deg.to_radians();
                let lat_rad = lat_deg.to_radians();
                let speed = self.rng.range_f32(PARTICLE_SPEED_MIN, PARTICLE_SPEED_MAX);
                let (sin_y, cos_y) = yaw_rad.sin_cos();
                let dir = [sin_y * lat_rad.sin(), -lat_rad.cos(), cos_y * lat_rad.sin()];
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
            let t = local;
            let cap = CYLINDER_LIFETIME_FRAMES;
            let s = CYLINDER_INITIAL_SPEED * (t - t * (t + 1.0) / (4.0 * cap));
            let (sin_h, cos_h) = self.heading_rad.sin_cos();
            let centre = [c.pivot[0] + s * sin_h, c.pivot[1], c.pivot[2] + s * cos_h];

            let fade_start = CYLINDER_LIFETIME_FRAMES - CYLINDER_LIFETIME_FRAMES / 2.0;
            let alpha = if local < fade_start {
                RING_ALPHA
            } else {
                let span = CYLINDER_LIFETIME_FRAMES - fade_start;
                RING_ALPHA * (1.0 - (local - fade_start) / span).max(0.0)
            };

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
                alpha_bottom: alpha,
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
                no_depth: false,
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
            camera_target: None,
            caster_yaw: None,
        });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &ctx());
        list.primitives
    }

    #[test]
    fn cylinder_aims_along_caster_to_target_direction() {
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
        assert!((tilt - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!((yaw + std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        assert!((base[1] - (-CYLINDER_PIVOT_Y_OFFSET.abs())).abs() < 0.1);
        assert!((base[0] - 10.0).abs() < 1.0);
    }

    #[test]
    fn level_n_emits_n_cylinders_with_first_skipping_particles() {
        let mut e = PierceEffect::new_with_level([0.0; 3], [10.0, 0.0, 0.0], 3);

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
        let p21 = step_and_collect(&mut e, 20.0 / FRAMES_PER_SECOND);
        let part21 = p21
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        assert_eq!(
            part21,
            2 * PARTICLES_PER_DIR,
            "burst #2 spawned 8 particles"
        );
    }

    #[test]
    fn dies_after_last_burst_finishes() {
        let mut e = PierceEffect::new([0.0; 3], [10.0, 0.0, 0.0]);
        let total_s = TOTAL_DURATION_MS as f32 / 1000.0;
        let s = e.update(&EffectUpdateCtx {
            delta: total_s + 0.5,
            camera_target: None,
            caster_yaw: None,
        });
        assert!(matches!(s, EffectStatus::Dead));
    }
}
