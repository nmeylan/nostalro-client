//! `EF_SANDWIND` — Bard/Dancer Sandwind (id 46).
//!
//! A drifting screen-space blow of sand particles. Particles spawn scattered
//! across the screen and drift diagonally in a fixed wind direction, then
//! fade out near the end of their lives.
//!
//! Behaviour:
//!   * Parent duration 180 frames; first 60 frames spawn one particle
//!     every 2 frames (so up to 30 particles).
//!   * Each particle: 30 px square (→ 3 wu via the `10 px / wu` mapping
//!     shared with [`super::endure`]).
//!   * Wind heading 130..145°, speed 2.6..5.6 px/frame → diagonal
//!     down-right drift, ~0.26..0.56 wu/frame.
//!   * Spawn scatter `x ∈ [-500, 140]` px → particles seeded across ~64 wu
//!     of horizontal screen width; `y = -240..-190` px → spawn roughly one
//!     character above the camera focus.
//!   * Alpha 128/255, fade-out begins at `duration - duration/3` → ~50%
//!     opacity then linear fade in the final third.
//!   * Lifetime `duration - random(100)` → 80..180 frames per particle.
//!   * Texture `sand1.bmp`.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SAND_TEXTURE: &str = "sand1.bmp";
pub const TEXTURES: &[&str] = &[SAND_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const PARENT_DURATION_FRAMES: f32 = 180.0;
const SPAWN_WINDOW_FRAMES: f32 = 60.0;
const SPAWN_INTERVAL_FRAMES: u32 = 2;
const PARTICLE_DURATION_MIN_FRAMES: f32 = 80.0;
const PARTICLE_DURATION_MAX_FRAMES: f32 = 180.0;

pub const TOTAL_DURATION_MS: u32 =
    ((SPAWN_WINDOW_FRAMES + PARTICLE_DURATION_MAX_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

const PARTICLE_SIZE: f32 = 3.0;
const PARTICLE_ALPHA: f32 = 128.0 / 255.0;

/// Random speed 2.6..5.6 px/frame, mapping to 0.26..0.56 wu/frame via the
/// shared 10 px/wu screen-space scale.
const WIND_SPEED_MIN_PER_FRAME: f32 = 0.26;
const WIND_SPEED_MAX_PER_FRAME: f32 = 0.56;

/// Spawn scatter — `x ∈ [-500, 140]` px and `y ∈ [-240, -190]` px in screen
/// space. In our world-space approximation we spread across the XZ plane
/// with a slight Y lift so the particles ride above the ground rather than
/// sit on it.
const SPAWN_X_HALF_RANGE: f32 = 32.0;
const SPAWN_X_OFFSET: f32 = -18.0;
const SPAWN_Z_HALF_RANGE: f32 = 6.0;
const SPAWN_Y_MIN: f32 = -24.0;
const SPAWN_Y_MAX: f32 = -19.0;

#[derive(Clone, Copy)]
struct Particle {
    age_frames: f32,
    spawn_xz_offset: [f32; 2],
    y_offset: f32,
    /// Per-particle wind direction × speed, XZ world units / frame.
    wind_per_frame: [f32; 2],
    /// Per-particle lifetime (frames) — `duration - random(100)`.
    duration_frames: f32,
}

impl Particle {
    fn alive(&self) -> bool {
        self.age_frames < self.duration_frames
    }

    fn position(&self, world: [f32; 3]) -> [f32; 3] {
        [
            world[0] + self.spawn_xz_offset[0] + self.wind_per_frame[0] * self.age_frames,
            world[1] + self.y_offset,
            world[2] + self.spawn_xz_offset[1] + self.wind_per_frame[1] * self.age_frames,
        ]
    }

    fn alpha(&self) -> f32 {
        // Fade-out begins at `duration - duration/3` → hold for the first
        // 2/3 of life, then fade linearly to 0.
        let fade_out_at = self.duration_frames * 2.0 / 3.0;
        if self.age_frames < fade_out_at {
            PARTICLE_ALPHA
        } else {
            let span = (self.duration_frames - fade_out_at).max(1e-3);
            let t = ((self.age_frames - fade_out_at) / span).clamp(0.0, 1.0);
            PARTICLE_ALPHA * (1.0 - t)
        }
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
                && (f as f32) < SPAWN_WINDOW_FRAMES
                && (f as u32) % SPAWN_INTERVAL_FRAMES == 0
            {
                // Per-particle wind: heading 130..145° in the 2D screen
                // convention `(sin(lon), -cos(lon))`. Map screen y to
                // world +Z so the drift reads as a diagonal across the
                // XZ plane.
                let longitude_deg = 130.0 + self.lcg() * 15.0;
                let (sn, cs) = longitude_deg.to_radians().sin_cos();
                let speed = WIND_SPEED_MIN_PER_FRAME
                    + self.lcg() * (WIND_SPEED_MAX_PER_FRAME - WIND_SPEED_MIN_PER_FRAME);
                let wind = [sn * speed, -cs * speed];

                let x = SPAWN_X_OFFSET + (self.lcg() - 0.5) * 2.0 * SPAWN_X_HALF_RANGE;
                let z = (self.lcg() - 0.5) * 2.0 * SPAWN_Z_HALF_RANGE;
                let y = SPAWN_Y_MIN + self.lcg() * (SPAWN_Y_MAX - SPAWN_Y_MIN);
                let duration = PARTICLE_DURATION_MIN_FRAMES
                    + self.lcg()
                        * (PARTICLE_DURATION_MAX_FRAMES - PARTICLE_DURATION_MIN_FRAMES);
                self.particles.push(Particle {
                    age_frames: 0.0,
                    spawn_xz_offset: [x, z],
                    y_offset: y,
                    wind_per_frame: wind,
                    duration_frames: duration,
                });
            }
        }
        self.last_spawn_frame = current_frame;

        for p in &mut self.particles {
            p.age_frames += dt_frames;
        }
        self.particles.retain(|p| p.alive());

        if self.age_frames >= PARENT_DURATION_FRAMES && self.particles.is_empty() {
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
                color: [1.0, 1.0, 1.0, alpha],
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

    fn drift(p: &Particle) -> (f32, f32) {
        (
            p.wind_per_frame[0] * p.age_frames,
            p.wind_per_frame[1] * p.age_frames,
        )
    }

    #[test]
    fn particles_spawn_and_drift_diagonally() {
        // Sociable: by 20 frames the spawn schedule has fired several
        // times, particles exist, and at least one is moving in both X
        // and Z because `longitude ∈ [130°, 145°]` yields non-zero
        // components on both axes.
        let mut e = SandwindEffect::new([0.0; 3]);
        for _ in 0..20 {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        assert!(!e.particles.is_empty(), "spawn schedule fired");

        let any_diagonal = e.particles.iter().any(|p| {
            let (dx, dz) = drift(p);
            dx.abs() > 0.0 && dz.abs() > 0.0
        });
        assert!(any_diagonal, "particles drift diagonally in XZ");
    }

    #[test]
    fn dies_after_full_lifetime() {
        let mut e = SandwindEffect::new([0.0; 3]);
        let total =
            (PARENT_DURATION_FRAMES + PARTICLE_DURATION_MAX_FRAMES + 5.0) as i32;
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
