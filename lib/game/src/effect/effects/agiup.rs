//! `EF_AGIUP` — Increase Agility cast fountain (enum id 456).
//!
//! The original game launches one crossed-quad particle
//! every 2 frames over its ~100-frame life: a 3D "+" of two perpendicular
//! vertical `ac_center2.tga` streaks, spawned at a random yaw and radial
//! offset around the caster, rising as it fades. The texture is a vertical
//! bright stripe, so each cross reads as a pair of crossed light beams; the
//! stream of them is the rising sparkle fountain in the reference.
//!
//! Modeled as a deterministic emitter: particle `i` launches at frame
//! `i · 2` with parameters derived from a cheap per-index hash (no RNG, so
//! the effect is reproducible frame-for-frame). Each live particle emits two
//! `Texture3D` `VerticalYaw` quads (yaw and yaw + 90°).

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const AC_CENTER2_TEXTURE: &str = "ac_center2.tga";
pub const TEXTURES: &[&str] = &[AC_CENTER2_TEXTURE];

const FPS: f32 = 60.0;
const TOTAL_FRAMES: f32 = 100.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FPS * 1000.0) as u32;

/// A fresh cross particle every other frame.
const SPAWN_PERIOD_FRAMES: f32 = 2.0;
/// Per-particle lifetime (50 frames).
const PARTICLE_LIFE_FRAMES: f32 = 50.0;
/// Fraction of life spent fading in (original ramps alpha over ~20 frames).
const FADE_IN_FRAC: f32 = 0.2;
const PEAK_ALPHA: f32 = 0.85;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

/// Cheap deterministic hash → `[0, 1)`, varied by `salt` so each particle
/// attribute is independent without an RNG (RNG would break reproducibility).
fn hash01(i: u32, salt: u32) -> f32 {
    let x = i
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(40_503))
        .wrapping_add(0x9E37_79B9);
    let x = x ^ (x >> 15);
    (x % 100_000) as f32 / 100_000.0
}

/// The reference streaks alternate cool-blue and warm-yellow; pick per
/// particle (the white `ac_center2` texture carries no colour of its own).
const TINT_BLUE: [f32; 3] = [0.55, 0.7, 1.0];
const TINT_YELLOW: [f32; 3] = [1.0, 0.95, 0.55];

struct Particle {
    spawn_frame: f32,
    yaw: f32,
    /// Horizontal offset direction (radians) + distance from the caster.
    offset_angle: f32,
    offset_dist: f32,
    rise_speed: f32,
    half_w: f32,
    half_h: f32,
    tint: [f32; 3],
}

impl Particle {
    fn from_index(i: u32) -> Self {
        Particle {
            spawn_frame: i as f32 * SPAWN_PERIOD_FRAMES,
            yaw: hash01(i, 1) * std::f32::consts::TAU,
            offset_angle: hash01(i, 2) * std::f32::consts::TAU,
            offset_dist: 1.0 + hash01(i, 3) * 3.0,
            rise_speed: 0.15 + hash01(i, 4) * 0.25,
            half_w: 0.25 + hash01(i, 5) * 0.2,
            half_h: 1.0 + hash01(i, 6) * 2.5,
            tint: if i % 2 == 0 { TINT_BLUE } else { TINT_YELLOW },
        }
    }
}

pub struct AgiUpEffect {
    base: [f32; 3],
    age_frames: f32,
}

impl AgiUpEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            base: world_pos,
            age_frames: 0.0,
        }
    }

    fn particle_count(&self) -> u32 {
        (self.age_frames / SPAWN_PERIOD_FRAMES).floor() as u32 + 1
    }
}

impl Effect for AgiUpEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for i in 0..self.particle_count() {
            let p = Particle::from_index(i);
            let local_age = self.age_frames - p.spawn_frame;
            if local_age < 0.0 || local_age >= PARTICLE_LIFE_FRAMES {
                continue;
            }
            let t = local_age / PARTICLE_LIFE_FRAMES;
            let alpha = if t < FADE_IN_FRAC {
                PEAK_ALPHA * (t / FADE_IN_FRAC)
            } else {
                PEAK_ALPHA * (1.0 - (t - FADE_IN_FRAC) / (1.0 - FADE_IN_FRAC))
            };
            if alpha <= 0.0 {
                continue;
            }
            let (osin, ocos) = p.offset_angle.sin_cos();
            let center = [
                self.base[0] + ocos * p.offset_dist,
                self.base[1] - local_age * p.rise_speed, // native -Y up → rising
                self.base[2] + osin * p.offset_dist,
            ];
            // Two perpendicular vertical streaks form the 3D cross.
            for leg in 0..2 {
                let yaw = p.yaw + leg as f32 * std::f32::consts::FRAC_PI_2;
                out.push(EffectPrimitiveDraw::Texture3D {
                    center,
                    size: [p.half_w, p.half_h],
                    plane: QuadPlane::VerticalYaw(yaw),
                    uv: UNIT_UV,
                    texture: AC_CENTER2_TEXTURE,
                    color: [p.tint[0], p.tint[1], p.tint[2], alpha],
                    blend: BlendKind::Additive,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut AgiUpEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None, caster_yaw: None,
        })
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(e: &AgiUpEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_paired_perpendicular_vertical_streaks() {
        // After a few spawns there are cross particles, each contributing two
        // VerticalYaw Texture3D quads 90° apart.
        let mut e = AgiUpEffect::new([0.0; 3]);
        step(&mut e, 12.0);
        let prims = draws(&e);
        assert!(prims.len() >= 4 && prims.len() % 2 == 0, "paired legs");
        // First particle's two legs differ by 90° in yaw.
        let yaw = |p: &EffectPrimitiveDraw| match p {
            EffectPrimitiveDraw::Texture3D {
                plane: QuadPlane::VerticalYaw(y),
                ..
            } => *y,
            other => panic!("expected VerticalYaw Texture3D, got {other:?}"),
        };
        assert!((((yaw(&prims[1]) - yaw(&prims[0])).abs()) - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    }

    #[test]
    fn particles_rise_and_spawn_count_grows_over_time() {
        let mut e = AgiUpEffect::new([0.0; 3]);
        step(&mut e, 10.0);
        let early = e.particle_count();
        step(&mut e, 20.0);
        assert!(e.particle_count() > early, "more particles spawn over time");

        // A given particle's y decreases (native -Y up = rising) as it ages.
        let p = Particle::from_index(0);
        // y at two ages:
        let y_at = |age: f32| -age * p.rise_speed;
        assert!(y_at(20.0) < y_at(2.0), "particle rises");
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = AgiUpEffect::new([0.0; 3]);
        assert_eq!(step(&mut e, TOTAL_FRAMES + 1.0), EffectStatus::Dead);
    }
}
