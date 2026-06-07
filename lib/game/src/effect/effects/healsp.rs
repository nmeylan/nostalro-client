//! `EF_HEALSP` — Soul Drain / Sp Recovery glow.
//!
//!
//! Composite spawned at the master's feet, lifetime 100 frames at 60 fps:
//!
//!   * Three concentric translucent cylinders (all
//!     `alpha_down.tga`, all tinted cyan `(32, 176, 232)`), each one
//!     taller than the next. Innermost has max alpha `16/255`, middle
//!     `32/255`, outer `64/255` — so visually the layers stack from very
//!     faint inside to fully visible outside.
//!   * Every 2 frames — one orbiting `particle2.spr` sprite. Radius 3
//!     in the XZ plane, random longitude, 30-frame lifetime, rising at
//!     -1.2 wu/frame with positive gravity decel (arcs up
//!     then falls). Size `0.55`.
//!
//! All cylinders spin at 5°/frame and share the same
//! alpha envelope: linear fade-in over 30 frames, hold, fade out over
//! the final 20 frames.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const CYLINDER_TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[CYLINDER_TEXTURE];

pub const PARTICLE_SPRITE: &str = "data/sprite/이팩트/particle2";
pub const SPRITES: &[&str] = &[PARTICLE_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 100.0;
const FADE_IN_FRAMES: f32 = 30.0;
const FADE_OUT_AT: f32 = PARENT_DURATION_FRAMES - 20.0;
const SPIN_DEG_PER_FRAME: f32 = 5.0;
const TINT: [f32; 3] = [0x20 as f32 / 255.0, 0xb0 as f32 / 255.0, 0xe8 as f32 / 255.0];

// Three nested cylinders, launched innermost first. All three share
// roughly the same height — only the radii (and tint alpha) stack from
// inside out — matching the original game silhouette where the inner
// shell rises almost as high as the outer one. The inner shell is the
// faintest, the outer the most visible.
const CYLINDER_RADII: [f32; 3] = [2.5, 3.5, 4.5];
const CYLINDER_HEIGHTS: [f32; 3] = [32.0, 34.0, 35.0];
const CYLINDER_MAX_ALPHAS: [f32; 3] = [16.0 / 255.0, 32.0 / 255.0, 64.0 / 255.0];
const CYLINDER_SIDES: u32 = 24;

// Orbiting particles (one every 2 frames). Orbit radius sits a
// fraction outside the outermost cylinder so the sparkles read as
// "drifting around" the column rather than inside it.
const SPAWN_PERIOD_FRAMES: u32 = 2;
const PARTICLE_DURATION_FRAMES: f32 = 30.0;
const PARTICLE_ORBIT_RADIUS: f32 = 5.5;
const PARTICLE_SIZE: f32 = 0.55;
const PARTICLE_FADEOUT_AT: f32 =
    PARTICLE_DURATION_FRAMES - PARTICLE_DURATION_FRAMES / 3.0;
const PARTICLE_ANIM_TICKS: f32 = 4.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * PARTICLE_ANIM_TICKS;
const PARTICLE_INITIAL_Y_SPEED_PER_FRAME: f32 = -1.2;
const PARTICLE_Y_ACCEL_PER_FRAME: f32 =
    -(PARTICLE_INITIAL_Y_SPEED_PER_FRAME / PARTICLE_DURATION_FRAMES) / 1.5;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

fn cylinder_alpha(frame: f32, peak: f32) -> f32 {
    let rise = (frame / FADE_IN_FRAMES).clamp(0.0, 1.0);
    let fall = if frame < FADE_OUT_AT {
        1.0
    } else {
        let span = (PARENT_DURATION_FRAMES - FADE_OUT_AT).max(1e-3);
        (1.0 - (frame - FADE_OUT_AT) / span).clamp(0.0, 1.0)
    };
    peak * rise * fall
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

pub struct HealSpEffect {
    world_pos: [f32; 3],
    particles: Vec<Particle>,
    age_frames: f32,
    last_spawn_frame: i32,
    rng_state: u32,
}

impl HealSpEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
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
        self.particles.push(Particle {
            anchor: self.world_pos,
            offset: [PARTICLE_ORBIT_RADIUS * sn, 0.0, PARTICLE_ORBIT_RADIUS * cs],
            y_velocity_per_frame: PARTICLE_INITIAL_Y_SPEED_PER_FRAME,
            age_frames: 0.0,
        });
    }
}

impl Effect for HealSpEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        if self.age_frames <= PARENT_DURATION_FRAMES {
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
        }

        for p in &mut self.particles {
            p.step(dt_frames);
        }
        self.particles.retain(|p| p.alive());

        if self.age_frames >= PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES
            && self.particles.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.age_frames <= PARENT_DURATION_FRAMES {
            let rotation = (self.age_frames * SPIN_DEG_PER_FRAME).to_radians();
            for i in 0..3 {
                let alpha = cylinder_alpha(self.age_frames, CYLINDER_MAX_ALPHAS[i]);
                if alpha <= 0.0 {
                    continue;
                }
                out.push(EffectPrimitiveDraw::Cylinder {
                    base: self.world_pos,
                    bottom_size: CYLINDER_RADII[i],
                    top_size: CYLINDER_RADII[i],
                    height: CYLINDER_HEIGHTS[i],
                    sides: CYLINDER_SIDES,
                    rotation,
                    tilt_x_rad: 0.0,
                    rotation_y_rad: 0.0,
                    uv_scroll: [0.0, 0.0],
                    texture: CYLINDER_TEXTURE,
                    color: [TINT[0], TINT[1], TINT[2], alpha],
                    blend: BlendKind::Additive,
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
                action_index: 0,
                motion_index: motion,
                size_scale: PARTICLE_SIZE,
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

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_frames(e: &mut HealSpEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn three_cylinders_plus_orbit_particles_on_schedule() {
        // Sociable: 3 nested Frustum cylinders + particles every 2
        // frames around a radius-3 ring.
        let mut e = HealSpEffect::new([5.0, 0.0, 7.0]);
        step_frames(&mut e, 11);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let cylinders: Vec<_> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Cylinder {
                    bottom_size,
                    top_size,
                    height,
                    color,
                    ..
                } => Some((*bottom_size, *top_size, *height, *color)),
                _ => None,
            })
            .collect();
        assert_eq!(cylinders.len(), 3);
        for (b, t, _, _) in &cylinders {
            assert!(
                (b - t).abs() < 1e-3,
                "cylinder (bottom == top)",
            );
        }
        // Heights stack: 15 < 25 < 35.
        let heights: Vec<f32> = cylinders.iter().map(|(_, _, h, _)| *h).collect();
        assert!(heights[0] < heights[1] && heights[1] < heights[2]);

        let particles: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        assert!(particles >= 5, "frames 0, 2, 4, 6, 8, 10 each spawn a particle");

        // Each particle sits on the radius-3 orbit.
        for prim in &list.primitives {
            if let EffectPrimitiveDraw::SpriteParticle { position, .. } = prim {
                let dx = position[0] - 5.0;
                let dz = position[2] - 7.0;
                let r = (dx * dx + dz * dz).sqrt();
                assert!(
                    (r - PARTICLE_ORBIT_RADIUS).abs() < 0.1,
                    "particle on the orbit ring: r={r}",
                );
            }
        }
    }

    #[test]
    fn cylinder_max_alphas_stack_from_inside_out() {
        // Sociable: peak alphas grow 16 → 32 → 64. Confirms the three
        // constants are wired in the right order so the visible
        // silhouette is brightest at the outer shell.
        let a0 = cylinder_alpha(FADE_IN_FRAMES, CYLINDER_MAX_ALPHAS[0]);
        let a1 = cylinder_alpha(FADE_IN_FRAMES, CYLINDER_MAX_ALPHAS[1]);
        let a2 = cylinder_alpha(FADE_IN_FRAMES, CYLINDER_MAX_ALPHAS[2]);
        assert!(a0 < a1 && a1 < a2);
    }

    #[test]
    fn dies_after_parent_plus_particle_lifetime() {
        let mut e = HealSpEffect::new([0.0; 3]);
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
