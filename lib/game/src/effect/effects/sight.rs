//! `EF_SIGHT` / `EF_RUWACH` — orbit-spawn SpriteParticle effects.
//!
//! Both effects share the same shape: every N parent frames, emit two
//! particles on opposite sides of a radius-15 orbit around the master. The
//! "upper" particle hovers around y = -20 (Sight) / y = -10 (Ruwach) and uses
//! the skill's signature sprite; the "lower" particle sits on the ground
//! (`y = 0`) and uses `Shadow.spr` as a ground tag.
//!
//! Per-particle state: an initial offset, an optional Y acceleration, a
//! shrinking size via a per-frame size speed, and a linear alpha ramp via a
//! per-frame alpha speed plus an optional late-life fade-out cliff. Each
//! variant tunes these constants for its own look.
//!
//! Both ids are radius-15 orbits with a brief per-particle life; Sight orbits
//! slightly faster and a touch higher than Ruwach, and uses a different sprite
//! per the constants below.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SIGHT_SPRITE: &str = "data/sprite/이팩트/sight";
pub const SHADOW_SPRITE: &str = "data/sprite/shadow";
pub const PARTICLE2_SPRITE: &str = "data/sprite/이팩트/particle2";

pub const SPRITES: &[&str] = &[SIGHT_SPRITE, SHADOW_SPRITE, PARTICLE2_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const ORBIT_RADIUS: f32 = 15.0;
/// Particle motion advances every N ticks at 60 fps (matches the
/// renderer's `(1000/60) * anim_speed` cadence).
const PARTICLE_ANIM_TICKS: f32 = 4.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * PARTICLE_ANIM_TICKS;

/// Per-skill variant parameters. One `Params` constant per effect id.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Parent emitter lifetime in frames (60 fps).
    pub parent_duration_frames: f32,
    /// Frames between each two-particle spawn.
    pub spawn_period_frames: u32,
    /// Compass-rotation rate of the orbit angle in degrees/frame.
    /// Sight is plain `frame * -5°`; Ruwach scales the angle by `1/1.3`.
    pub angle_deg_per_frame: f32,
    pub angle_divisor: f32,
    pub upper: ParticleParams,
    pub lower: ParticleParams,
}

#[derive(Clone, Copy, Debug)]
pub struct ParticleParams {
    pub sprite: &'static str,
    pub duration_frames: f32,
    /// Y offset above ground at spawn (negative = up; native RO uses
    /// -Y = up).
    pub y_offset: f32,
    /// Per-frame Y acceleration on the offset — positive = drift down.
    pub y_accel_per_frame: f32,
    pub size: f32,
    /// Size change per frame, negative = shrinks.
    pub size_speed_per_frame: f32,
    /// Initial alpha (normalised to 0..1).
    pub alpha_init: f32,
    /// Alpha change per frame (negative = fades).
    pub alpha_speed_per_frame: f32,
    /// Frame at which the late-life fadeout kicks in.
    /// `f32::MAX` disables.
    pub fadeout_start_frame: f32,
}

pub const SIGHT: Params = Params {
    parent_duration_frames: 162.0,
    spawn_period_frames: 2,
    angle_deg_per_frame: -5.0,
    angle_divisor: 1.0,
    upper: ParticleParams {
        sprite: SIGHT_SPRITE,
        duration_frames: 20.0,
        // The source spawns the particle at y = -20 (above the master)
        // with a +0.1 Y accel pulling it back to ground over the
        // 20-frame lifetime. The user wants the orbit to read like
        // Ruwach (a constant-height arc rather than a settling drop),
        // so we hold the particle at the spawn height and remove the
        // downward drift (accel 0 below).
        y_offset: -20.0,
        y_accel_per_frame: 0.0,
        size: 2.5,
        size_speed_per_frame: -0.1,
        alpha_init: 150.0 / 255.0,
        alpha_speed_per_frame: -3.0 / 255.0,
        fadeout_start_frame: f32::MAX,
    },
    lower: ParticleParams {
        sprite: SHADOW_SPRITE,
        duration_frames: 20.0,
        y_offset: 0.0,
        y_accel_per_frame: 0.0,
        size: 1.5,
        size_speed_per_frame: -0.06,
        alpha_init: 120.0 / 255.0,
        alpha_speed_per_frame: -1.0 / 255.0,
        fadeout_start_frame: f32::MAX,
    },
};

pub const RUWACH: Params = Params {
    parent_duration_frames: 120.0,
    spawn_period_frames: 3,
    angle_deg_per_frame: -5.0,
    angle_divisor: 1.3,
    upper: ParticleParams {
        sprite: PARTICLE2_SPRITE,
        duration_frames: 25.0,
        y_offset: -10.0,
        y_accel_per_frame: 0.0,
        size: 3.0,
        size_speed_per_frame: -0.1,
        alpha_init: 250.0 / 255.0,
        alpha_speed_per_frame: -3.0 / 255.0,
        fadeout_start_frame: 19.0,
    },
    lower: ParticleParams {
        sprite: SHADOW_SPRITE,
        duration_frames: 25.0,
        y_offset: 0.0,
        y_accel_per_frame: 0.0,
        size: 1.5,
        size_speed_per_frame: -0.06,
        alpha_init: 150.0 / 255.0,
        alpha_speed_per_frame: -2.5 / 255.0,
        fadeout_start_frame: 19.0,
    },
};

/// Total visible duration: parent lifetime + the longer particle lifetime.
/// The holder uses this for despawn; `update` keeps the effect alive while
/// any particle is still rendering.
pub const fn total_duration_ms(p: &Params) -> u32 {
    let upper = p.upper.duration_frames;
    let lower = p.lower.duration_frames;
    let particle_max = if upper > lower { upper } else { lower };
    ((p.parent_duration_frames + particle_max) / FRAMES_PER_SECOND * 1000.0) as u32
}

/// One live particle: integrates per-tick velocity, size, alpha.
#[derive(Clone, Copy, Debug)]
struct Particle {
    sprite: &'static str,
    /// World-space anchor (master pos at spawn).
    anchor: [f32; 3],
    /// Offset from anchor; Y integrates `y_accel`.
    offset: [f32; 3],
    /// Current Y velocity in frame units (units per parent-frame).
    y_velocity_per_frame: f32,
    y_accel_per_frame: f32,
    size: f32,
    size_speed_per_frame: f32,
    alpha: f32,
    alpha_speed_per_frame: f32,
    fadeout_start_frame: f32,
    age_frames: f32,
    lifetime_frames: f32,
}

impl Particle {
    fn alive(&self) -> bool {
        self.age_frames < self.lifetime_frames && self.alpha > 0.0 && self.size > 0.0
    }

    /// Step by `dt_frames` parent-frames (60 fps).
    fn step(&mut self, dt_frames: f32) {
        self.y_velocity_per_frame += self.y_accel_per_frame * dt_frames;
        self.offset[1] += self.y_velocity_per_frame * dt_frames;
        self.size = (self.size + self.size_speed_per_frame * dt_frames).max(0.0);
        self.alpha = (self.alpha + self.alpha_speed_per_frame * dt_frames).clamp(0.0, 1.0);
        if self.age_frames >= self.fadeout_start_frame {
            let cliff_span = (self.lifetime_frames - self.fadeout_start_frame).max(1e-3);
            let t = ((self.age_frames - self.fadeout_start_frame) / cliff_span).clamp(0.0, 1.0);
            self.alpha *= 1.0 - t;
        }
        self.age_frames += dt_frames;
    }

    fn position(&self) -> [f32; 3] {
        [
            self.anchor[0] + self.offset[0],
            self.anchor[1] + self.offset[1],
            self.anchor[2] + self.offset[2],
        ]
    }
}

pub struct OrbitEffect {
    world_pos: [f32; 3],
    params: Params,
    particles: Vec<Particle>,
    age_frames: f32,
    total_duration_s: f32,
    last_spawn_frame: i32,
}

impl OrbitEffect {
    pub fn new(world_pos: [f32; 3], params: Params) -> Self {
        Self {
            world_pos,
            params,
            particles: Vec::new(),
            age_frames: 0.0,
            total_duration_s: total_duration_ms(&params) as f32 / 1000.0,
            last_spawn_frame: -1,
        }
    }

    /// Compute orbit (sn, cs) at a given parent frame.
    fn orbit_at(&self, frame: f32) -> (f32, f32) {
        let degree = (frame / self.params.angle_divisor) * self.params.angle_deg_per_frame;
        let rad = degree.to_radians();
        (rad.sin(), rad.cos())
    }

    fn spawn_pair(&mut self, frame: i32) {
        let (sn, cs) = self.orbit_at(frame as f32);
        let upper_offset = [
            ORBIT_RADIUS * sn,
            self.params.upper.y_offset,
            -ORBIT_RADIUS * cs,
        ];
        let lower_offset = [
            ORBIT_RADIUS * sn,
            self.params.lower.y_offset,
            -ORBIT_RADIUS * cs,
        ];
        self.particles.push(Particle {
            sprite: self.params.upper.sprite,
            anchor: self.world_pos,
            offset: upper_offset,
            y_velocity_per_frame: 0.0,
            y_accel_per_frame: self.params.upper.y_accel_per_frame,
            size: self.params.upper.size,
            size_speed_per_frame: self.params.upper.size_speed_per_frame,
            alpha: self.params.upper.alpha_init,
            alpha_speed_per_frame: self.params.upper.alpha_speed_per_frame,
            fadeout_start_frame: self.params.upper.fadeout_start_frame,
            age_frames: 0.0,
            lifetime_frames: self.params.upper.duration_frames,
        });
        self.particles.push(Particle {
            sprite: self.params.lower.sprite,
            anchor: self.world_pos,
            offset: lower_offset,
            y_velocity_per_frame: 0.0,
            y_accel_per_frame: self.params.lower.y_accel_per_frame,
            size: self.params.lower.size,
            size_speed_per_frame: self.params.lower.size_speed_per_frame,
            alpha: self.params.lower.alpha_init,
            alpha_speed_per_frame: self.params.lower.alpha_speed_per_frame,
            fadeout_start_frame: self.params.lower.fadeout_start_frame,
            age_frames: 0.0,
            lifetime_frames: self.params.lower.duration_frames,
        });
    }
}

impl Effect for OrbitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        // Spawn at every parent frame divisible by the period, as long as
        // the parent is still alive. Catch up across larger dt by walking
        // each frame boundary between last_spawn_frame and current_frame.
        let current_frame = self.age_frames.floor() as i32;
        if (self.age_frames as f32) <= self.params.parent_duration_frames {
            let next_frame = self.last_spawn_frame + 1;
            for f in next_frame..=current_frame {
                if f >= 0 && f as f32 <= self.params.parent_duration_frames
                    && (f as u32) % self.params.spawn_period_frames == 0
                {
                    self.spawn_pair(f);
                }
            }
            self.last_spawn_frame = current_frame;
        }

        for p in &mut self.particles {
            p.step(dt_frames);
        }
        self.particles.retain(|p| p.alive());

        if self.age_frames / FRAMES_PER_SECOND >= self.total_duration_s
            && self.particles.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.particles {
            if p.alpha <= 0.0 || p.size <= 0.0 {
                continue;
            }
            let motion = (p.age_frames * (1000.0 / FRAMES_PER_SECOND) / PARTICLE_FRAME_MS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: p.sprite,
                position: p.position(),
                motion_index: motion,
                size_scale: p.size,
                color: [1.0, 1.0, 1.0, p.alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_one_frame(e: &mut OrbitEffect) {
        e.update(&ctx(1.0 / FRAMES_PER_SECOND));
    }

    #[test]
    fn sight_emits_one_pair_per_period_frame_in_orbit() {
        // Sociable test: after stepping the first spawn boundary the
        // effect emits two SpriteParticles, one with the SIGHT sprite at
        // y_offset=-20 (upper) and one with SHADOW at y=0 (lower).
        // Their XZ offset is on a radius-15 circle around the anchor.
        let mut e = OrbitEffect::new([10.0, 5.0, 20.0], SIGHT);
        // First tick lands on frame 0 → spawn fires immediately.
        step_one_frame(&mut e);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 2, "exactly two particles per spawn");
        let sprites: Vec<&str> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } => Some(*sprite_path),
                _ => None,
            })
            .collect();
        assert!(sprites.contains(&SIGHT_SPRITE), "upper uses sight sprite");
        assert!(sprites.contains(&SHADOW_SPRITE), "lower uses shadow sprite");

        for prim in &list.primitives {
            let EffectPrimitiveDraw::SpriteParticle { position, .. } = prim else { unreachable!() };
            let dx = position[0] - 10.0;
            let dz = position[2] - 20.0;
            let r = (dx * dx + dz * dz).sqrt();
            assert!(
                (r - ORBIT_RADIUS).abs() < 0.5,
                "particle on radius-15 orbit: r={r}",
            );
        }
    }

    #[test]
    fn ruwach_uses_particle2_sprite_and_period_3() {
        // Ruwach should spawn on frames 0, 3, 6, … so after stepping 3
        // frames we get two pairs (frame 0 and frame 3).
        let mut e = OrbitEffect::new([0.0; 3], RUWACH);
        for _ in 0..4 {
            step_one_frame(&mut e);
        }
        // Frames 0 and 3 are spawn points → 2 pairs = 4 particles
        // (we may have lost none yet at age=4 frames).
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert!(
            list.primitives.len() >= 4,
            "two pairs spawned across frames 0 and 3, got {}",
            list.primitives.len(),
        );
        // Ruwach upper uses particle2, not sight.
        let used: std::collections::HashSet<&str> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } => Some(*sprite_path),
                _ => None,
            })
            .collect();
        assert!(used.contains(PARTICLE2_SPRITE));
        assert!(used.contains(SHADOW_SPRITE));
        assert!(!used.contains(SIGHT_SPRITE));
    }

    #[test]
    fn particles_fade_and_die_then_effect_ends() {
        // After parent_duration + particle_lifetime, all particles must
        // be reaped and the effect reports Dead.
        let mut e = OrbitEffect::new([0.0; 3], SIGHT);
        let total_frames = SIGHT.parent_duration_frames + SIGHT.upper.duration_frames + 5.0;
        let mut status = EffectStatus::Running;
        for _ in 0..(total_frames as i32) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
