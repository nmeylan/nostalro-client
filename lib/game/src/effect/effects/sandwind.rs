//! `EF_SANDWIND` — Bard/Dancer Sandwind (id 46).
//!
//! Original game's SandWind is multiple 2D-textures with
//! Horizontal blow — particles blown horizontally in a wind. We approximate
//! with a periodic burst of camera-facing billboards that drift along a
//! constant wind vector and fade out.
//!
//! Lifetime: ~1800 ms (108 frames).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SAND_TEXTURE: &str = "sandwind.tga";
pub const TEXTURES: &[&str] = &[SAND_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 90.0;
const PARTICLE_DURATION_FRAMES: f32 = 50.0;

pub const TOTAL_DURATION_MS: u32 =
    ((PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

const SPAWN_INTERVAL_FRAMES: u32 = 3;
const PARTICLE_SIZE: f32 = 1.2;
/// Wind drift in world units per frame, XZ. Positive X = +east in our coords.
const WIND_DRIFT_PER_FRAME: [f32; 2] = [0.4, 0.0];

#[derive(Clone, Copy)]
struct Particle {
    age_frames: f32,
    spawn_xz_offset: [f32; 2],
    y_offset: f32,
}

impl Particle {
    fn alive(&self) -> bool {
        self.age_frames < PARTICLE_DURATION_FRAMES
    }

    fn position(&self, world: [f32; 3]) -> [f32; 3] {
        [
            world[0] + self.spawn_xz_offset[0] + WIND_DRIFT_PER_FRAME[0] * self.age_frames,
            world[1] + self.y_offset,
            world[2] + self.spawn_xz_offset[1] + WIND_DRIFT_PER_FRAME[1] * self.age_frames,
        ]
    }

    fn alpha(&self) -> f32 {
        let t = self.age_frames / PARTICLE_DURATION_FRAMES;
        if t < 0.2 {
            t / 0.2
        } else {
            1.0 - (t - 0.2) / 0.8
        }
        .clamp(0.0, 1.0)
    }
}

pub struct SandwindEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    particles: Vec<Particle>,
    last_spawn_frame: i32,
    rng_state: u32,
}

impl SandwindEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let rng_state = 0x5A5A_C0DE
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(3);
        Self {
            world_pos,
            age_frames: 0.0,
            particles: Vec::new(),
            last_spawn_frame: -1,
            rng_state,
        }
    }

    fn lcg(&mut self) -> f32 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.rng_state >> 8) as f32 / ((1u32 << 24) as f32)
    }
}

impl Effect for SandwindEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        let next_frame = self.last_spawn_frame + 1;
        for f in next_frame..=current_frame {
            if f >= 0
                && (f as f32) <= PARENT_DURATION_FRAMES
                && (f as u32) % SPAWN_INTERVAL_FRAMES == 0
            {
                let x = (self.lcg() - 0.5) * 4.0;
                let z = (self.lcg() - 0.5) * 4.0;
                let y = -1.5 - self.lcg() * 2.0;
                self.particles.push(Particle {
                    age_frames: 0.0,
                    spawn_xz_offset: [x, z],
                    y_offset: y,
                });
            }
        }
        self.last_spawn_frame = current_frame;

        for p in &mut self.particles {
            p.age_frames += dt_frames;
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
        for p in &self.particles {
            let alpha = p.alpha();
            if alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: p.position(self.world_pos),
                size: [PARTICLE_SIZE, PARTICLE_SIZE],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                rotation: 0.0,
                texture: SAND_TEXTURE,
                color: [0.9, 0.8, 0.6, alpha * 0.7],
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

    #[test]
    fn periodic_spawn_drifts_in_wind_direction() {
        let mut e = SandwindEffect::new([0.0; 3]);
        // Step through a handful of spawn intervals.
        for _ in 0..15 {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert!(!list.primitives.is_empty());

        // After more time the oldest particle has drifted along +X.
        let mut max_x = f32::MIN;
        for p in &list.primitives {
            if let EffectPrimitiveDraw::Billboard { pos, .. } = p {
                max_x = max_x.max(pos[0]);
            }
        }
        assert!(max_x > 0.0, "particles drift along wind");
    }

    #[test]
    fn dies_after_full_lifetime() {
        let mut e = SandwindEffect::new([0.0; 3]);
        let total = (PARENT_DURATION_FRAMES + PARTICLE_DURATION_FRAMES + 5.0) as i32;
        let mut status = EffectStatus::Running;
        for _ in 0..total {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
