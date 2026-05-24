//! `EF_HASTEUP` (id 98) — Adrenaline Rush self-buff cast effect.
//!
//! Three layers all launched at frame 0:
//!
//!
//!   * 20 radial flash spikes (`alpha_center.tga`) — same shape
//!     as `Bash`/`Flasher` but tuned slower and longer. Length starts at 0,
//!     growing ≈ 3.85..5.31/frame for the first 40 frames, then the
//!     speed swaps to ≈ 1.5..2.4/frame for the second 40.
//!     80-frame lifetime.
//!   * 4 orbiting sparkles (`particle1.spr`) at compass
//!     headings (0°/90°/180°/270°) on a radius-7 orbit, accelerating
//!     rotation (longitude speed = 0.3 + frame·0.2/frame²)
//!     and drifting upward
//!     (initial -0.1/frame, accel -0.002/frame). 100-frame lifetime.
//!   * Parent emitter lives 300 frames for the audio cue (plays
//!     at frame 30) but emits no further primitives.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::spike_burst::{
    self, ChangeGrowth, SpikeBurst, SpikeBurstParams, seed_from_world,
};

pub const TEXTURES: &[&str] = &[spike_burst::SPIKE_TEXTURE];
pub const PARTICLE_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SPRITES: &[&str] = &[PARTICLE_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const SPIKE_DURATION_FRAMES: f32 = 80.0;
const PARENT_DURATION_FRAMES: f32 = 300.0;
const PARTICLE_DURATION_FRAMES: f32 = 100.0;

pub const SPIKES: SpikeBurstParams = SpikeBurstParams {
    count: 20,
    duration_frames: SPIKE_DURATION_FRAMES,
    angular_speed_deg_range: (1.0, 7.0),
    length_init_range: (0.0, 0.0),
    growth_range: (3.85 / 6.0, 5.31 / 6.0),
    change_growth: Some(ChangeGrowth {
        at_frame: 40.0,
        growth_range: (1.5 / 6.0, 2.4 / 6.0),
    }),
    thickness: 0.35,
    // The reference gif's spike rays are clearly a subtle background layer
    // — the orbit sparkles dominate, so the spike alpha stays low.
    // (The original game leaves the burst near-opaque, but visually
    // it reads as secondary.) Pin alpha low so the burst stays
    // a secondary visual, with `particle1.spr` reading as the primary.
    max_alpha: 90.0 / 255.0,
    fade_in_frames: 10.0,
    fade_out_start_frame: SpikeBurstParams::default_fade_out_start(SPIKE_DURATION_FRAMES),
    height_offset: -5.0,
    texture: spike_burst::SPIKE_TEXTURE,
    color_tint: [1.0, 1.0, 1.0],
    blend: BlendKind::Alpha,
};

// Orbiting-sparkle per-particle constants for the
// HasteUp recipe.
const ORBIT_RADIUS: f32 = 7.0;
// The gif shows the 4
// orbit sparkles as the primary visual layer, each with a detailed ray
// pattern baked into `particle1.spr`. A literal size renders
// them as small dots in our viewer, drowned by the spike burst, so we
// scale up so the sparkles read as the dominant burst element.
const PARTICLE_SIZE: f32 = 1.2;
// Spawn at the same chest-height offset the spike burst uses. The original
// attaches sparkles to the character's
// body centre, so spikes and orbit particles read at the same height
// from frame 0; our `world_pos` is the raw spawn anchor (ground level),
// so we apply the offset explicitly to keep both layers co-located.
const PARTICLE_INITIAL_Y_OFFSET: f32 = -5.0;
// Initial Y velocity (negative Y = up in native RO).
const PARTICLE_Y_SPEED_PER_FRAME: f32 = -0.1;
// Continues drifting up faster each frame.
const PARTICLE_Y_ACCEL_PER_FRAME: f32 = -0.002;
// Longitude is in degrees, integrated each frame as
// `longitude += speed; speed
// += accel`.
const PARTICLE_LONG_SPEED_INIT_DEG: f32 = 0.3;
const PARTICLE_LONG_ACCEL_DEG_PER_FRAME: f32 = 0.2;
// Fade-out window — defaults to `duration -
// duration/10` ≈ frame 90 of 100.
const PARTICLE_FADEOUT_AT: f32 = PARTICLE_DURATION_FRAMES - PARTICLE_DURATION_FRAMES / 10.0;

const PARTICLE_ANIM_TICKS: f32 = 4.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * PARTICLE_ANIM_TICKS;

pub const TOTAL_DURATION_MS: u32 = ((PARENT_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

#[derive(Clone, Copy, Debug)]
struct OrbitParticle {
    /// Initial compass heading in degrees (0/90/180/270).
    initial_longitude_deg: f32,
    age_frames: f32,
    y_offset: f32,
    y_velocity_per_frame: f32,
}

impl OrbitParticle {
    fn step(&mut self, dt_frames: f32) {
        self.y_velocity_per_frame += PARTICLE_Y_ACCEL_PER_FRAME * dt_frames;
        self.y_offset += self.y_velocity_per_frame * dt_frames;
        self.age_frames += dt_frames;
    }

    /// Closed-form integration of `longitude(N) = N*v0 + accel*N*(N+1)/2`
    /// matching the per-frame `longitude += speed;
    /// speed += accel`.
    fn longitude_deg(&self) -> f32 {
        let n = self.age_frames;
        self.initial_longitude_deg
            + n * PARTICLE_LONG_SPEED_INIT_DEG
            + PARTICLE_LONG_ACCEL_DEG_PER_FRAME * n * (n + 1.0) / 2.0
    }

    fn alpha(&self) -> f32 {
        if self.age_frames < PARTICLE_FADEOUT_AT {
            1.0
        } else {
            let span = (PARTICLE_DURATION_FRAMES - PARTICLE_FADEOUT_AT).max(1e-3);
            (1.0 - (self.age_frames - PARTICLE_FADEOUT_AT) / span).clamp(0.0, 1.0)
        }
    }

    fn alive(&self) -> bool {
        self.age_frames < PARTICLE_DURATION_FRAMES
    }

    fn position(&self, anchor: [f32; 3]) -> [f32; 3] {
        let rad = self.longitude_deg().to_radians();
        let (sn, cs) = rad.sin_cos();
        [
            anchor[0] + ORBIT_RADIUS * sn,
            anchor[1] + self.y_offset,
            anchor[2] + ORBIT_RADIUS * cs,
        ]
    }
}

pub struct HasteUpEffect {
    world_pos: [f32; 3],
    spikes: SpikeBurst,
    particles: Vec<OrbitParticle>,
    age_frames: f32,
}

impl HasteUpEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let particles = (0..4)
            .map(|i| OrbitParticle {
                initial_longitude_deg: i as f32 * 90.0,
                age_frames: 0.0,
                y_offset: PARTICLE_INITIAL_Y_OFFSET,
                y_velocity_per_frame: PARTICLE_Y_SPEED_PER_FRAME,
            })
            .collect();
        Self {
            world_pos,
            spikes: SpikeBurst::new(SPIKES, seed_from_world(world_pos)),
            particles,
            age_frames: 0.0,
        }
    }
}

impl Effect for HasteUpEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;
        self.spikes.tick(ctx.delta);
        for p in &mut self.particles {
            p.step(dt_frames);
        }
        self.particles.retain(|p| p.alive());

        if self.age_frames >= PARENT_DURATION_FRAMES
            && !self.spikes.alive()
            && self.particles.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        self.spikes.collect_draws(out, self.world_pos);
        for p in &self.particles {
            let a = p.alpha();
            if a <= 0.0 {
                continue;
            }
            let motion = (p.age_frames * (1000.0 / FRAMES_PER_SECOND) / PARTICLE_FRAME_MS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: PARTICLE_SPRITE,
                position: p.position(self.world_pos),
                motion_index: motion,
                size_scale: PARTICLE_SIZE,
                color: [1.0, 1.0, 1.0, a],
                // Additive so the particles pop on top of the
                // alpha-blended spike rays rather than getting washed
                // out by them during the burst's bright frames.
                blend: BlendKind::Additive,
                aim_target: None,
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

    fn step_frames(e: &mut HasteUpEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn emits_twenty_spikes_plus_four_orbit_particles_at_spawn() {
        let mut e = HasteUpEffect::new([0.0; 3]);
        step_frames(&mut e, 5);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let spikes = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { texture, .. } if *texture == spike_burst::SPIKE_TEXTURE))
            .count();
        let particles = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == PARTICLE_SPRITE))
            .count();
        assert_eq!(spikes, 20);
        assert_eq!(particles, 4);
    }

    #[test]
    fn orbit_particles_rotate_and_drift_upward() {
        // Sociable: confirm the orbit advances over time and the upward
        // drift (negative Y in native RO) accumulates.
        let mut e = HasteUpEffect::new([0.0; 3]);
        step_frames(&mut e, 10);
        let early_lon = e.particles[0].longitude_deg();
        let early_y = e.particles[0].y_offset;
        step_frames(&mut e, 30);
        let late_lon = e.particles[0].longitude_deg();
        let late_y = e.particles[0].y_offset;
        assert!(late_lon > early_lon, "longitude advances {early_lon} → {late_lon}");
        assert!(late_y < early_y, "drifts up (Y decreases) {early_y} → {late_y}");
    }

    #[test]
    fn dies_after_parent_duration() {
        let mut e = HasteUpEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..(PARENT_DURATION_FRAMES as i32 + 5) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
