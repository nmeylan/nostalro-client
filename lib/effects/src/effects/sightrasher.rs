//! `EF_SIGHTRASHER` (#62) — Sightrasher's radial reveal burst.
//!
//! In the original game Sightrasher fires four staggered waves (at
//! frames 0/5/10/15) of eight rays each, one ray every 45°.
//! Each wave is two layers on the same eight directions:
//!
//! * **Sight layer** (the `sight` sprite) — the eye sprite, lifted to head
//!   height, sliding outward along its ray and fading.
//! * **Shadow layer** (the `shadow` sprite) — a flat ground underlayer on the same
//!   directions, smaller.
//!
//! The wave's sprite size shrinks with the spawn frame
//! (`2.5 - frame/10` → 2.5/2.0/1.5/1.0) and the initial radial
//! offset grows (`5 + frame/5`). A single expanding `ring_yellow.tga`
//! ground ring punches out at frame 10.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SIGHT_SPRITE: &str = "data/sprite/이팩트/sight";
pub const SHADOW_SPRITE: &str = "data/sprite/shadow";
pub const SPRITES: &[&str] = &[SIGHT_SPRITE, SHADOW_SPRITE];
pub const TEXTURES: &[&str] = &["ring_yellow.tga"];

const FRAMES_PER_SECOND: f32 = 60.0;

/// Uniform down-scale from the original world units. Maps the
/// radial offset (`~5-8`) to ~2-3 wu and the `-20` lift to head height (~7 wu)
/// in one factor, preserving the original ratio (skill: scale uniformly).
const WORLD_SCALE: f32 = 0.55;
/// The size literals map onto our larger sprite footprint.
const SIZE_RENDER_SCALE: f32 = 0.5;

const NUM_DIRS: usize = 8;
/// Frames the four waves spawn on (`frame % 5 == 0 && < 20`).
const WAVE_FRAMES: [f32; 4] = [0.0, 5.0, 10.0, 15.0];
/// Particle life = `85 - spawn_frame`, so every wave dies by frame 85.
const BASE_PARTICLE_LIFE: f32 = 85.0;
/// Outward slide speed of a ray over its life (magnitude 1.0).
const OUTWARD_SPEED: f32 = 1.0;

// --- frame-10 ground ring ---
const RING_START_FRAME: f32 = 10.0;
const RING_GROWTH_PER_FRAME: f32 = 2.5;
const RING_LIFE: f32 = 40.0;
const RING_FADE_START: f32 = 20.0;
const RING_MAX_ALPHA: f32 = 150.0 / 255.0;
const RING_THICKNESS: f32 = 1.0;

/// Self-terminate once both the rays (≤85f) and the ring (≤50f) are gone.
const TOTAL_FRAMES: f32 = 90.0;

pub struct SightrasherEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl SightrasherEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age: 0.0 }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }

    /// Emit one wave's eight rays on `sprite`, lifted by `lift` world units.
    fn collect_wave(
        &self,
        out: &mut EffectDrawList,
        frame: f32,
        wave_frame: f32,
        sprite: &'static str,
        size_orig: f32,
        lift: f32,
    ) {
        let local = frame - wave_frame;
        let life = BASE_PARTICLE_LIFE - wave_frame;
        if local < 0.0 || local > life {
            return;
        }
        let size = (size_orig * SIZE_RENDER_SCALE).max(0.0);
        if size <= 0.0 {
            return;
        }
        // Alpha fades linearly to 0 over the particle's life
        // (`150 - frame*2`, gone near frame 75 → use the wave life).
        let alpha = (1.0 - local / life).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let radius0 = 5.0 + wave_frame / 5.0;
        let dist = (radius0 + OUTWARD_SPEED * local) * WORLD_SCALE;
        for a in 0..NUM_DIRS {
            let deg = (a as f32) * -45.0;
            let (s, c) = deg.to_radians().sin_cos();
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: sprite,
                position: [
                    self.world_pos[0] + s * dist,
                    self.world_pos[1] - lift,
                    self.world_pos[2] - c * dist,
                ],
                action_index: 0,
                motion_index: (local / 4.0) as usize,
                size_scale: size,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
                aim_target: None,
                no_depth: false,
            });
        }
    }

    fn collect_ring(&self, out: &mut EffectDrawList, frame: f32) {
        let local = frame - RING_START_FRAME;
        if local < 0.0 || local > RING_LIFE {
            return;
        }
        let alpha = if local < RING_FADE_START {
            RING_MAX_ALPHA
        } else {
            RING_MAX_ALPHA * (1.0 - (local - RING_FADE_START) / (RING_LIFE - RING_FADE_START))
        };
        if alpha <= 0.0 {
            return;
        }
        let radius = (RING_GROWTH_PER_FRAME * local) * WORLD_SCALE;
        if radius <= 0.0 {
            return;
        }
        out.push(EffectPrimitiveDraw::GroundDisc {
            center: self.world_pos,
            radius,
            thickness: RING_THICKNESS,
            rotation: 0.0,
            arc_angle_deg: 360.0,
            uv_repeat: 1.0,
            texture: "ring_yellow.tga",
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Additive,
        });
    }
}

impl Effect for SightrasherEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.frame() > TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();
        for &wf in &WAVE_FRAMES {
            let size = 2.5 - wf / 10.0;
            // Shadow underlayer on the ground, then the lifted eye sprite.
            self.collect_wave(out, frame, wf, SHADOW_SPRITE, 1.0 - wf / 30.0, 0.0);
            self.collect_wave(out, frame, wf, SIGHT_SPRITE, size, 20.0 * WORLD_SCALE);
        }
        self.collect_ring(out, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn run_to(c: &mut SightrasherEffect, target_frame: f32) {
        let delta = (target_frame - c.frame()) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx { delta, ..Default::default() });
        }
    }

    fn draws(c: &SightrasherEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn first_wave_emits_eight_rays_per_layer() {
        let mut c = SightrasherEffect::new([0.0; 3]);
        run_to(&mut c, 1.0);
        let sprites = draws(&c)
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        // Wave 0 only, two layers × 8 directions.
        assert_eq!(sprites, 2 * NUM_DIRS);
    }

    #[test]
    fn ring_punches_out_at_frame_ten_and_grows() {
        let mut c = SightrasherEffect::new([0.0; 3]);
        run_to(&mut c, 5.0);
        let early = draws(&c).iter().any(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. }));
        assert!(!early, "no ring before frame 10");
        run_to(&mut c, 15.0);
        let r1 = ring_radius(&c);
        run_to(&mut c, 25.0);
        let r2 = ring_radius(&c);
        assert!(r2 > r1 && r1 > 0.0, "ring grows ({r1} → {r2})");
    }

    fn ring_radius(c: &SightrasherEffect) -> f32 {
        draws(c).into_iter().find_map(|p| match p {
            EffectPrimitiveDraw::GroundDisc { radius, .. } => Some(radius),
            _ => None,
        }).unwrap_or(0.0)
    }

    #[test]
    fn dies_after_all_particles_gone() {
        let mut c = SightrasherEffect::new([0.0; 3]);
        run_to(&mut c, TOTAL_FRAMES - 5.0);
        assert_eq!(c.update(&EffectUpdateCtx::default()), EffectStatus::Running);
        run_to(&mut c, TOTAL_FRAMES + 5.0);
        assert_eq!(c.update(&EffectUpdateCtx::default()), EffectStatus::Dead);
    }
}
