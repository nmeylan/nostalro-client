//! `EF_LIGHTSPHERE` / `EF_LIGHTSPHERE2` — orbiting light-point sphere
//! (enum ids 348 / 381).
//!
//! Original game's dispatch calls `LightSphere("effect\\white02.bmp")` ~80
//! times, and each light sphere carries four particles — roughly 320
//! bluish light points orbiting on a slowly-rotating sphere shell whose
//! radius creeps outward (`distance *= 1.02` up to a cap). The reference
//! reads as a compact, bright, faintly-blue ball of sparks.
//!
//! Modeled as a fixed cloud of points spread deterministically over a
//! sphere (a per-index hash, so it's reproducible — no RNG), rotated about
//! the vertical axis each frame, each point a small additive `white02.bmp`
//! billboard. `Lightsphere` (348) fades in/holds/fades out; `Lightsphere2`
//! (381) is persistent (the holder reaps it at its sentinel duration).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const WHITE02_TEXTURE: &str = "white02.bmp";
pub const TEXTURES: &[&str] = &[WHITE02_TEXTURE];

const FPS: f32 = 60.0;
const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

/// Light points spread over the shell. Fewer than the original's ~320 (each
/// is a small additive billboard; this many already reads as a dense ball).
const POINTS: u32 = 140;
const POINT_SIZE: f32 = 0.9;
/// Bluish tint (original per-vertex RGB ≈ (105,105,255)).
const TINT: [f32; 3] = [105.0 / 255.0, 105.0 / 255.0, 1.0];

const RADIUS_START: f32 = 0.5;
const RADIUS_CAP: f32 = 3.5;
/// Frames to reach the cap (original creeps via `distance *= 1.02`).
const RADIUS_GROW_FRAMES: f32 = 45.0;
const ROT_DEG_PER_FRAME: f32 = 1.5;
const PEAK_ALPHA: f32 = 0.7;
const FADE_FRAMES: f32 = 40.0;

#[derive(Clone, Copy)]
pub struct Params {
    pub persistent: bool,
    /// Wall-clock lifetime in frames (ignored when `persistent`).
    pub total_frames: f32,
}

/// `Lightsphere` (348): ~10 s then fades out.
pub const LIGHTSPHERE: Params = Params {
    persistent: false,
    total_frames: 600.0,
};
/// `Lightsphere2` (379): persistent ambient sphere.
pub const LIGHTSPHERE2: Params = Params {
    persistent: true,
    total_frames: f32::MAX,
};

pub const fn total_duration_ms(p: &Params) -> u32 {
    if p.persistent {
        u32::MAX
    } else {
        (p.total_frames / FPS * 1000.0) as u32
    }
}

fn hash01(i: u32, salt: u32) -> f32 {
    let x = i
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(40_503))
        .wrapping_add(0x9E37_79B9);
    let x = x ^ (x >> 15);
    (x % 100_000) as f32 / 100_000.0
}

pub struct LightSphereEffect {
    center: [f32; 3],
    params: Params,
    age_frames: f32,
}

impl LightSphereEffect {
    pub fn new(world_pos: [f32; 3], params: Params) -> Self {
        Self {
            center: world_pos,
            params,
            age_frames: 0.0,
        }
    }

    fn radius(&self) -> f32 {
        let t = (self.age_frames / RADIUS_GROW_FRAMES).clamp(0.0, 1.0);
        RADIUS_START + (RADIUS_CAP - RADIUS_START) * t
    }

    fn alpha(&self) -> f32 {
        if self.params.persistent {
            return PEAK_ALPHA;
        }
        if self.age_frames < FADE_FRAMES {
            PEAK_ALPHA * (self.age_frames / FADE_FRAMES)
        } else if self.age_frames < self.params.total_frames - FADE_FRAMES {
            PEAK_ALPHA
        } else {
            PEAK_ALPHA
                * (1.0 - (self.age_frames - (self.params.total_frames - FADE_FRAMES)) / FADE_FRAMES)
                    .clamp(0.0, 1.0)
        }
    }
}

impl Effect for LightSphereEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if !self.params.persistent && self.age_frames >= self.params.total_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return;
        }
        let radius = self.radius();
        let spin = (self.age_frames * ROT_DEG_PER_FRAME).to_radians();
        for i in 0..POINTS {
            // Deterministic direction on the unit sphere.
            let theta = hash01(i, 1) * std::f32::consts::TAU + spin;
            let phi = (2.0 * hash01(i, 2) - 1.0).acos();
            let (st, ct) = theta.sin_cos();
            let (sp, cp) = phi.sin_cos();
            let pos = [
                self.center[0] + radius * sp * ct,
                self.center[1] + radius * cp,
                self.center[2] + radius * sp * st,
            ];
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [POINT_SIZE, POINT_SIZE],
                uv: UNIT_UV,
                rotation: 0.0,
                texture: WHITE02_TEXTURE,
                color: [TINT[0], TINT[1], TINT[2], alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut LightSphereEffect, frames: f32) -> EffectStatus {
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

    fn draws(e: &LightSphereEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_a_blue_additive_point_cloud_on_a_growing_shell() {
        let mut e = LightSphereEffect::new([0.0; 3], LIGHTSPHERE);
        step(&mut e, 5.0);
        let r_early = e.radius();
        let prims = draws(&e);
        assert_eq!(prims.len() as u32, POINTS);
        match &prims[0] {
            EffectPrimitiveDraw::Billboard { color, blend, .. } => {
                assert_eq!(*blend, BlendKind::Additive);
                assert!(color[2] > color[0], "bluish");
            }
            other => panic!("expected Billboard, got {other:?}"),
        }
        step(&mut e, RADIUS_GROW_FRAMES);
        assert!(e.radius() > r_early, "shell radius grows then caps");
        assert!((e.radius() - RADIUS_CAP).abs() < 1e-3);
    }

    #[test]
    fn lightsphere_fades_and_dies_but_lightsphere2_persists() {
        let mut a = LightSphereEffect::new([0.0; 3], LIGHTSPHERE);
        assert_eq!(step(&mut a, LIGHTSPHERE.total_frames + 1.0), EffectStatus::Dead);

        let mut b = LightSphereEffect::new([0.0; 3], LIGHTSPHERE2);
        assert_eq!(step(&mut b, 100_000.0), EffectStatus::Running);
        assert!((b.alpha() - PEAK_ALPHA).abs() < 1e-6, "persistent holds alpha");
    }

    #[test]
    fn cloud_rotates_over_time() {
        let mut e = LightSphereEffect::new([0.0; 3], LIGHTSPHERE2);
        step(&mut e, 30.0);
        let p0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { pos, .. } => *pos,
            _ => unreachable!(),
        };
        step(&mut e, 30.0);
        let p1 = match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { pos, .. } => *pos,
            _ => unreachable!(),
        };
        assert!(p0 != p1, "point orbits as the sphere spins");
    }
}
