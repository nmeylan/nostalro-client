//! `EF_NAPALMBEAT` — Wizard Napalm Beat (id 32).
//!
//! Original game spawns 15 explosion sprites (one per frame,
//! frames 0–14), each at a random screen-space offset from the target. Each
//! particle cycles through 8 `폭발{1-8}.tga` frames with anim speed 2, fades
//! in over 10 frames, fades out from frame 20, and dies at frame 30.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::spike_burst::seed_from_world;

const EXPLOSION_TEXTURES: &[&str] = &[
    "폭발1.tga",
    "폭발2.tga",
    "폭발3.tga",
    "폭발4.tga",
    "폭발5.tga",
    "폭발6.tga",
    "폭발7.tga",
    "폭발8.tga",
];
pub const TEXTURES: &[&str] = EXPLOSION_TEXTURES;

const FRAMES_PER_SECOND: f32 = 60.0;
const PARTICLE_COUNT: usize = 15;
const PARTICLE_DURATION: f32 = 30.0;
const TEXTURE_COUNT: usize = 8;
const ANIM_SPEED: f32 = 2.0;
const FADE_IN_FRAMES: f32 = 10.0;
const FADE_OUT_START: f32 = 20.0;

const LAST_SPAWN_FRAME: f32 = (PARTICLE_COUNT - 1) as f32;
const TOTAL_FRAMES: f32 = LAST_SPAWN_FRAME + PARTICLE_DURATION;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const OFFSET_MIN: f32 = 1.0;
const OFFSET_MAX: f32 = 5.0;
const SIZE_MIN: f32 = 2.0;
const SIZE_MAX: f32 = 5.0;
const Y_OFFSET: f32 = -2.0;

struct Particle {
    spawn_frame: f32,
    offset_x: f32,
    offset_z: f32,
    size: f32,
}

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
}

fn make_particles(seed: u32) -> Vec<Particle> {
    let mut rng = seed;
    (0..PARTICLE_COUNT)
        .map(|i| {
            let angle = lcg_float(&mut rng) * std::f32::consts::TAU;
            let length = OFFSET_MIN + lcg_float(&mut rng) * (OFFSET_MAX - OFFSET_MIN);
            let size = SIZE_MIN + lcg_float(&mut rng) * (SIZE_MAX - SIZE_MIN);
            Particle {
                spawn_frame: i as f32,
                offset_x: length * angle.sin(),
                offset_z: length * angle.cos(),
                size,
            }
        })
        .collect()
}

pub struct NapalmBeatEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    particles: Vec<Particle>,
}

impl NapalmBeatEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let particles = make_particles(seed_from_world(world_pos));
        Self {
            world_pos,
            age_frames: 0.0,
            particles,
        }
    }
}

impl Effect for NapalmBeatEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.particles {
            let local_age = self.age_frames - p.spawn_frame;
            if local_age < 0.0 || local_age >= PARTICLE_DURATION {
                continue;
            }

            let tex_step = (local_age * ANIM_SPEED * TEXTURE_COUNT as f32 / PARTICLE_DURATION)
                as usize;
            let tex_idx = tex_step.min(TEXTURE_COUNT - 1);

            let alpha = if local_age < FADE_IN_FRAMES {
                local_age / FADE_IN_FRAMES
            } else if local_age < FADE_OUT_START {
                1.0
            } else {
                1.0 - (local_age - FADE_OUT_START) / (PARTICLE_DURATION - FADE_OUT_START)
            };

            out.push(EffectPrimitiveDraw::Billboard {
                pos: [
                    self.world_pos[0] + p.offset_x,
                    self.world_pos[1] + Y_OFFSET,
                    self.world_pos[2] + p.offset_z,
                ],
                size: [p.size, p.size],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: 0.0,
                texture: EXPLOSION_TEXTURES[tex_idx],
                color: [1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0)],
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

    fn advance(e: &mut NapalmBeatEffect, frames: f32) {
        e.update(&ctx(frames / FRAMES_PER_SECOND));
    }

    fn draw_count(e: &NapalmBeatEffect) -> usize {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives.len()
    }

    fn first_texture(e: &NapalmBeatEffect) -> String {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        match &list.primitives[0] {
            EffectPrimitiveDraw::Billboard { texture, .. } => texture.to_string(),
            _ => unreachable!(),
        }
    }

    fn first_alpha(e: &NapalmBeatEffect) -> f32 {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        match list.primitives[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        }
    }

    #[test]
    fn spawns_one_particle_per_frame_up_to_fifteen() {
        let mut e = NapalmBeatEffect::new([10.0, 0.0, 20.0]);

        advance(&mut e, 0.5);
        assert_eq!(draw_count(&e), 1, "1 particle at frame 0.5");

        advance(&mut e, 5.0);
        assert_eq!(draw_count(&e), 6, "6 particles at frame 5.5");

        advance(&mut e, 14.5);
        assert_eq!(draw_count(&e), 15, "all 15 alive at frame 20");
    }

    #[test]
    fn earlier_particles_expire_while_later_ones_live() {
        let mut e = NapalmBeatEffect::new([0.0; 3]);
        advance(&mut e, 35.0);
        let count = draw_count(&e);
        assert!(count < 15, "some early particles should have died by frame 35, got {count}");
        assert!(count > 0, "late particles should still be alive at frame 35");
    }

    #[test]
    fn textures_cycle_through_explosion_frames() {
        let mut e = NapalmBeatEffect::new([0.0; 3]);
        advance(&mut e, 0.5);
        let first = first_texture(&e);
        assert!(first.starts_with("폭발"), "texture should be an explosion frame");

        advance(&mut e, 14.0);
        let mid = first_texture(&e);
        assert_ne!(first, mid, "texture should advance over time");
    }

    #[test]
    fn alpha_fades_in_and_out() {
        let mut e = NapalmBeatEffect::new([0.0; 3]);

        advance(&mut e, 0.5);
        let alpha_early = first_alpha(&e);
        assert!(alpha_early < 0.2, "near-zero alpha at start (fade in), got {alpha_early}");

        advance(&mut e, 14.5);
        let alpha_mid = first_alpha(&e);
        assert!((alpha_mid - 1.0).abs() < 0.01, "full alpha in middle, got {alpha_mid}");

        advance(&mut e, 13.0);
        let alpha_late = first_alpha(&e);
        assert!(alpha_late < 0.3, "fading out near end, got {alpha_late}");
    }

    #[test]
    fn dies_after_total_duration() {
        let mut e = NapalmBeatEffect::new([0.0; 3]);
        let status = e.update(&ctx(TOTAL_FRAMES / FRAMES_PER_SECOND + 0.01));
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn deterministic_offsets_from_same_position() {
        let a = NapalmBeatEffect::new([5.0, 0.0, 10.0]);
        let b = NapalmBeatEffect::new([5.0, 0.0, 10.0]);
        for i in 0..PARTICLE_COUNT {
            assert_eq!(a.particles[i].offset_x, b.particles[i].offset_x);
            assert_eq!(a.particles[i].offset_z, b.particles[i].offset_z);
            assert_eq!(a.particles[i].size, b.particles[i].size);
        }
    }
}
