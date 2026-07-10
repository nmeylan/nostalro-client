//! `EF_BLITZBEAT` (id 115) — Falcon Blitz Beat cross-textured needle volley.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const BLITZBEAT_TEXTURE: &str = "ac_center2.tga";
pub const TEXTURES: &[&str] = &[BLITZBEAT_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const NEEDLE_COUNT: u32 = 10;
const DURATION_FRAMES: f32 = 20.0;
const LIFETIME_FRAMES: f32 = 32.0;
pub const TOTAL_DURATION_MS: u32 = (LIFETIME_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const SCATTER_RADIUS: f32 = 3.5;
const Y_OFFSET: f32 = -7.0;
const FORWARD_INIT: f32 = 15.0;
const SPEED_INIT: f32 = -1.2;
const SPEED_ACCEL: f32 = -0.1;

const HALF_HEIGHT: f32 = 0.1;
const HALF_WIDTH_MIN: f32 = 1.0;
const HALF_WIDTH_MAX: f32 = 3.0;
const WIDTH_SPEED_INIT: f32 = 0.1;
const WIDTH_ACCEL: f32 = -0.01;

const FADE_IN_FRAMES: f32 = 2.0;
const FADE_OUT_START: f32 = DURATION_FRAMES - 2.0;

#[derive(Clone, Copy)]
struct Needle {
    scatter: [f32; 3],
    base_half_width: f32,
}

pub struct BlitzbeatEffect {
    caster_pos: [f32; 3],
    yaw: f32,
    age: f32,
    needles: [Needle; NEEDLE_COUNT as usize],
}

const FIXED_YAW: f32 = std::f32::consts::FRAC_PI_4;

impl BlitzbeatEffect {
    pub fn new(caster_pos: [f32; 3]) -> Self {
        Self::with_yaw(caster_pos, FIXED_YAW)
    }

    pub fn with_yaw(caster_pos: [f32; 3], yaw: f32) -> Self {
        let seed = position_hash(&caster_pos);
        let mut needles = [Needle {
            scatter: [0.0; 3],
            base_half_width: 0.0,
        }; NEEDLE_COUNT as usize];
        for i in 0..NEEDLE_COUNT as usize {
            let salt = (i as u64) * 4;
            let theta = rand_in_range(seed, salt + 1, 0.0, std::f32::consts::TAU);
            let half_width = rand_in_range(seed, salt + 2, HALF_WIDTH_MIN, HALF_WIDTH_MAX);
            let (sn, cs) = theta.sin_cos();
            needles[i] = Needle {
                scatter: [SCATTER_RADIUS * sn, Y_OFFSET, -SCATTER_RADIUS * cs],
                base_half_width: half_width,
            };
        }
        Self {
            caster_pos,
            yaw,
            age: 0.0,
            needles,
        }
    }

    fn forward(&self) -> [f32; 3] {
        let (s, c) = self.yaw.sin_cos();
        [c, 0.0, s]
    }

    fn forward_offset_at(&self, frame: f32) -> f32 {
        FORWARD_INIT + SPEED_INIT * frame + SPEED_ACCEL * frame * (frame + 1.0) * 0.5
    }

    fn half_width_at(&self, base: f32, frame: f32) -> f32 {
        let w = base + WIDTH_SPEED_INIT * frame + WIDTH_ACCEL * frame * (frame + 1.0) * 0.5;
        w.max(0.05)
    }

    fn alpha_at(&self, frame: f32) -> f32 {
        if frame < FADE_IN_FRAMES {
            (frame / FADE_IN_FRAMES).clamp(0.0, 1.0)
        } else if frame < FADE_OUT_START {
            1.0
        } else {
            (1.0 - (frame - FADE_OUT_START) / (DURATION_FRAMES - FADE_OUT_START)).clamp(0.0, 1.0)
        }
    }
}

impl Effect for BlitzbeatEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age * FRAMES_PER_SECOND >= LIFETIME_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        let alpha = self.alpha_at(frame);
        if alpha <= 0.0 {
            return;
        }
        let forward = self.forward();
        let forward_offset = self.forward_offset_at(frame);
        let color = [1.0, 1.0, 1.0, alpha];
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for needle in &self.needles {
            let half_width = self.half_width_at(needle.base_half_width, frame);
            let center = [
                self.caster_pos[0] + needle.scatter[0] + forward[0] * forward_offset,
                self.caster_pos[1] + needle.scatter[1] + forward[1] * forward_offset,
                self.caster_pos[2] + needle.scatter[2] + forward[2] * forward_offset,
            ];
            for plane in [
                QuadPlane::HorizontalYaw(self.yaw),
                QuadPlane::VerticalYaw(self.yaw),
            ] {
                out.push(EffectPrimitiveDraw::Texture3D {
                    center,
                    size: [half_width, HALF_HEIGHT],
                    plane,
                    uv,
                    texture: BLITZBEAT_TEXTURE,
                    color,
                    blend: BlendKind::Additive,
                });
            }
        }
    }
}

fn position_hash(pos: &[f32; 3]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    pos[0].to_bits().hash(&mut h);
    pos[1].to_bits().hash(&mut h);
    pos[2].to_bits().hash(&mut h);
    h.finish()
}

fn rand_in_range(seed: u64, salt: u64, lo: f32, hi: f32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(salt);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    let t = ((x >> 40) as f32) / ((1u64 << 24) as f32);
    lo + t * (hi - lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_n(e: &mut BlitzbeatEffect, n: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..n {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
            if s == EffectStatus::Dead {
                break;
            }
        }
        s
    }

    fn draws(e: &BlitzbeatEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn ten_parallel_cross_textured_needles_translate_along_forward() {
        let mut e = BlitzbeatEffect::with_yaw([0.0, 0.0, 0.0], 0.0);

        step_n(&mut e, 3);
        let prims = draws(&e);
        assert_eq!(prims.len(), 20);

        for p in &prims {
            let yaw = match p {
                EffectPrimitiveDraw::Texture3D {
                    plane: QuadPlane::HorizontalYaw(y) | QuadPlane::VerticalYaw(y),
                    ..
                } => *y,
                _ => panic!("expected Texture3D needle plane"),
            };
            assert_eq!(yaw, 0.0);
        }

        let avg_x = |prims: &[EffectPrimitiveDraw]| -> f32 {
            let xs: Vec<f32> = prims
                .iter()
                .map(|p| match p {
                    EffectPrimitiveDraw::Texture3D { center, .. } => center[0],
                    _ => panic!(),
                })
                .collect();
            xs.iter().sum::<f32>() / xs.len() as f32
        };
        let x_early = avg_x(&prims);
        step_n(&mut e, 8);
        let x_later = avg_x(&draws(&e));
        assert!(x_later < x_early, "{x_early} -> {x_later}");
    }

    #[test]
    fn outlives_needles_so_frame_30_wave_can_fire() {
        let mut e = BlitzbeatEffect::new([0.0, 0.0, 0.0]);
        assert_eq!(step_n(&mut e, 25), EffectStatus::Running);
        assert_eq!(step_n(&mut e, 10), EffectStatus::Dead);
    }

    #[test]
    fn deterministic_scatter_per_position() {
        let a = BlitzbeatEffect::new([10.0, 0.0, 20.0]);
        let b = BlitzbeatEffect::new([10.0, 0.0, 20.0]);
        for i in 0..NEEDLE_COUNT as usize {
            assert_eq!(a.needles[i].scatter, b.needles[i].scatter);
            assert_eq!(a.needles[i].base_half_width, b.needles[i].base_half_width);
        }
    }
}
