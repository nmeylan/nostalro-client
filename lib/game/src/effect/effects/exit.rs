//! `EF_EXIT` — character despawn / portal-out effect.
//!
//! Composite spawned at the master's foot:
//!   * Frame 0 — a tall translucent cylinder (bottom radius = top radius =
//!     4.5, height 35) using
//!     `effect/alpha_down.tga`. Fades in over 30 frames, holds, then
//!     fades out over the final 20 frames of its 100-frame lifetime.
//!   * Every 9 frames — one orbiting sparkle sprite
//!     (`particle1.spr`) at a random orbit longitude on a radius-3 circle.
//!     Each particle has an initial upward velocity (1.2/frame)
//!     and a small downward acceleration so it arcs upward, slows, falls.
//!     50-frame particle lifetime, size 0.55.
//!
//! Despawn / portal-out sparkle, observed against the original game.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[RING_TEXTURE];

pub const PARTICLE_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SPRITES: &[&str] = &[PARTICLE_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 100.0;
const CYLINDER_RADIUS: f32 = 4.5;
const CYLINDER_HEIGHT: f32 = 35.0;
const CYLINDER_MAX_ALPHA: f32 = 90.0 / 255.0;
const CYLINDER_FADE_IN_FRAMES: f32 = 30.0;
const CYLINDER_FADE_OUT_AT: f32 = PARENT_DURATION_FRAMES - 20.0;
const CYLINDER_SIDES: u32 = 24;

const SPAWN_PERIOD_FRAMES: u32 = 9;
const PARTICLE_DURATION_FRAMES: f32 = 50.0;
const PARTICLE_ORBIT_RADIUS: f32 = 3.0;
const PARTICLE_SIZE: f32 = 0.55;
const PARTICLE_FADEOUT_AT: f32 = PARTICLE_DURATION_FRAMES - PARTICLE_DURATION_FRAMES / 3.0;
const PARTICLE_ANIM_TICKS: f32 = 4.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * PARTICLE_ANIM_TICKS;

// Initial Y velocity, per-frame. In native RO coords -Y is up so a
// particle with -1.2 / frame drifts up.
const PARTICLE_INITIAL_Y_SPEED_PER_FRAME: f32 = -1.2;
// Decel = -(initial_speed / duration) / 1.5 ≈ +0.016 per-frame
// (positive = decel, eventually falls back down).
const PARTICLE_Y_ACCEL_PER_FRAME: f32 =
    -(PARTICLE_INITIAL_Y_SPEED_PER_FRAME / PARTICLE_DURATION_FRAMES) / 1.5;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

fn cylinder_alpha(frame: f32) -> f32 {
    let alpha_speed = CYLINDER_MAX_ALPHA / CYLINDER_FADE_IN_FRAMES;
    let raised = (alpha_speed * frame).min(CYLINDER_MAX_ALPHA);
    if frame < CYLINDER_FADE_OUT_AT {
        raised
    } else {
        let span = (PARENT_DURATION_FRAMES - CYLINDER_FADE_OUT_AT).max(1e-3);
        let t = ((frame - CYLINDER_FADE_OUT_AT) / span).clamp(0.0, 1.0);
        raised * (1.0 - t)
    }
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    anchor: [f32; 3],
    offset: [f32; 3],
    y_velocity_per_frame: f32,
    age_frames: f32,
}

impl Particle {
    fn alive(&self) -> bool {
        self.age_frames < PARTICLE_DURATION_FRAMES
    }

    fn step(&mut self, dt_frames: f32) {
        self.y_velocity_per_frame += PARTICLE_Y_ACCEL_PER_FRAME * dt_frames;
        self.offset[1] += self.y_velocity_per_frame * dt_frames;
        self.age_frames += dt_frames;
    }

    fn alpha(&self) -> f32 {
        if self.age_frames < PARTICLE_FADEOUT_AT {
            1.0
        } else {
            let span = (PARTICLE_DURATION_FRAMES - PARTICLE_FADEOUT_AT).max(1e-3);
            (1.0 - (self.age_frames - PARTICLE_FADEOUT_AT) / span).clamp(0.0, 1.0)
        }
    }

    fn position(&self) -> [f32; 3] {
        [
            self.anchor[0] + self.offset[0],
            self.anchor[1] + self.offset[1],
            self.anchor[2] + self.offset[2],
        ]
    }
}

pub struct ExitEffect {
    world_pos: [f32; 3],
    particles: Vec<Particle>,
    age_frames: f32,
    last_spawn_frame: i32,
    rng_state: u32,
}

impl ExitEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        // Stable per (world_pos) so the same spawn yields the same orbit
        // angles in tests / replays.
        let rng_state = 0x9E37_79B9
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(13);
        Self {
            world_pos,
            particles: Vec::new(),
            age_frames: 0.0,
            last_spawn_frame: -1,
            rng_state,
        }
    }

    fn lcg(&mut self) -> u32 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.rng_state
    }

    fn lcg_float(&mut self) -> f32 {
        (self.lcg() >> 8) as f32 / ((1u32 << 24) as f32)
    }

    fn spawn_particle(&mut self) {
        let longitude_deg = self.lcg_float() * 360.0;
        let (sn, cs) = longitude_deg.to_radians().sin_cos();
        // Orbit offset = (0, 0, radius) rotated about Y by `long`,
        // which expands to (radius*sin, 0, radius*cos).
        self.particles.push(Particle {
            anchor: self.world_pos,
            offset: [PARTICLE_ORBIT_RADIUS * sn, 0.0, PARTICLE_ORBIT_RADIUS * cs],
            y_velocity_per_frame: PARTICLE_INITIAL_Y_SPEED_PER_FRAME,
            age_frames: 0.0,
        });
    }
}

impl Effect for ExitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        let next_frame = self.last_spawn_frame + 1;
        for f in next_frame..=current_frame {
            if f >= 0
                && (f as f32) <= PARENT_DURATION_FRAMES
                && (f as u32) % SPAWN_PERIOD_FRAMES == 0
            {
                self.spawn_particle();
            }
        }
        self.last_spawn_frame = current_frame;

        for p in &mut self.particles {
            p.step(dt_frames);
        }
        self.particles.retain(|p| p.alive());

        if self.age_frames
            >= PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES
            && self.particles.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        // Cylinder runs the first 100 frames.
        if self.age_frames < PARENT_DURATION_FRAMES {
            let alpha = cylinder_alpha(self.age_frames);
            if alpha > 0.0 {
                out.push(EffectPrimitiveDraw::Frustum {
                    base: self.world_pos,
                    bottom_size: CYLINDER_RADIUS,
                    top_size: CYLINDER_RADIUS,
                    height: CYLINDER_HEIGHT,
                    sides: CYLINDER_SIDES,
                    rotation: 0.0,
                    uv_repeat: 1.0,
                    uv_scroll: [0.0, 0.0],
                    wave_amplitude: 0.0,
                    wave_frequency: 0.0,
                    wave_phase: 0.0,
                    wave_mode: FrustumWaveMode::Sine,
                    tilt_x_rad: 0.0,
                    rotation_y_rad: 0.0,
                    cull_back: false,
                    texture: RING_TEXTURE,
                    color: [1.0, 1.0, 1.0, alpha],
                    blend: BlendKind::Alpha,
                });
            }
        }

        for p in &self.particles {
            let alpha = p.alpha();
            if alpha <= 0.0 {
                continue;
            }
            let motion = (p.age_frames * (1000.0 / FRAMES_PER_SECOND) / PARTICLE_FRAME_MS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: PARTICLE_SPRITE,
                position: p.position(),
                motion_index: motion,
                size_scale: PARTICLE_SIZE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
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

    fn step_frames(e: &mut ExitEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn cylinder_renders_then_orbit_particles_spawn_every_nine_frames() {
        // Sociable test: covers the cylinder-only frame, the cylinder
        // fade-in alpha ramp, the particle spawn cadence (every 9
        // frames → 0, 9, 18, …), and the particle orbit radius.
        let mut e = ExitEffect::new([10.0, 0.0, 20.0]);
        // First tick lands on frame 1 — frame 0's spawn already fired.
        step_frames(&mut e, 1);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let cylinders: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. }))
            .count();
        assert_eq!(cylinders, 1, "one Frustum per frame while parent alive");

        // Frame 1: particle from spawn at frame 0 lives.
        let p_count = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        assert_eq!(p_count, 1, "one particle after frame 0 spawn");

        // Each spawned particle sits on a radius-3 circle in XZ.
        for prim in &list.primitives {
            if let EffectPrimitiveDraw::SpriteParticle { position, .. } = prim {
                let dx = position[0] - 10.0;
                let dz = position[2] - 20.0;
                let r = (dx * dx + dz * dz).sqrt();
                assert!(
                    (r - PARTICLE_ORBIT_RADIUS).abs() < 0.1,
                    "particle on radius-3 circle: r={r}",
                );
            }
        }

        // Step to frame 10 — frames 0 and 9 spawned, so 2 alive particles.
        step_frames(&mut e, 9);
        let mut list2 = EffectDrawList::new();
        e.collect_draws(&mut list2, &render_ctx());
        let p2: usize = list2
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        assert_eq!(p2, 2, "two particles after frames 0 and 9 spawn");
    }

    #[test]
    fn cylinder_alpha_fades_in_then_holds_then_fades_out() {
        // Sociable check on the alpha envelope: at frame 0 alpha is 0,
        // at frame 30 it's at max, mid-life it holds, late-life drops.
        let mut e = ExitEffect::new([0.0; 3]);
        e.update(&ctx(0.0));
        let a_start = cylinder_alpha(0.0);
        let a_max = cylinder_alpha(CYLINDER_FADE_IN_FRAMES);
        let a_mid = cylinder_alpha(60.0);
        let a_late = cylinder_alpha(PARENT_DURATION_FRAMES - 2.0);
        assert!(a_start <= 1e-3, "frame 0 alpha is 0");
        assert!((a_max - CYLINDER_MAX_ALPHA).abs() < 1e-3);
        assert!((a_mid - CYLINDER_MAX_ALPHA).abs() < 1e-3, "holds at max");
        assert!(a_late < a_max, "fades out by end");
        let _ = e;
    }

    #[test]
    fn dies_after_parent_plus_particle_lifetime() {
        let mut e = ExitEffect::new([0.0; 3]);
        let total = PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES + 5.0;
        let mut status = EffectStatus::Running;
        for _ in 0..(total as i32) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
