//! `EF_STORMKICK4` / `EF_STORMKICK5` — rising-sparkle fountain (ids 462, 463).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &["whitelight.tga"];

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 2000;
const PEONG_TEXTURE: &str = "whitelight.tga";
const PARTICLE_COUNT: usize = 30;
const TINT_RGB: [u8; 3] = [105, 105, 225];

#[derive(Clone, Copy)]
pub struct PeongUpParams {
    pub dist_base: f32,
    pub dist_rand: f32,
    pub xz_speed: f32,
    pub rise_base: f32,
    pub rise_rand: f32,
}

pub const PEONGUP: PeongUpParams = PeongUpParams {
    dist_base: 2.0,
    dist_rand: 0.55,
    xz_speed: 3.5,
    rise_base: 0.08,
    rise_rand: 0.055,
};

struct Rng(u32);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

struct Particle {
    pos: [f32; 3],
    vel_xz: [f32; 2],
    rise: f32,
    size: f32,
    process: i32,
    alpha: f32,
}

pub struct PeongUpEffect {
    particles: Vec<Particle>,
    frame_accum: f32,
}

impl PeongUpEffect {
    pub fn new(anchor: [f32; 3], params: PeongUpParams) -> Self {
        let seed = anchor[0].to_bits() ^ anchor[2].to_bits() ^ 0x5111_0FF1;
        let mut rng = Rng(seed | 1);
        let mut particles = Vec::with_capacity(PARTICLE_COUNT);
        for _ in 0..PARTICLE_COUNT {
            let angle = rng.range(0.0, std::f32::consts::TAU);
            let dist = params.dist_base + rng.range(0.0, params.dist_rand);
            let process = -((rng.next_u32() % 26) as i32);
            particles.push(Particle {
                pos: [
                    anchor[0] + angle.cos() * dist,
                    anchor[1],
                    anchor[2] + angle.sin() * dist,
                ],
                vel_xz: [angle.cos() * params.xz_speed, angle.sin() * params.xz_speed],
                rise: params.rise_base + rng.range(0.0, params.rise_rand),
                size: 2.0 + rng.range(0.0, 1.0),
                process,
                alpha: 0.0,
            });
        }
        Self {
            particles,
            frame_accum: 0.0,
        }
    }

    fn step_frame(&mut self) {
        for pt in &mut self.particles {
            pt.process += 1;
            if pt.process > 0 {
                pt.pos[0] += pt.vel_xz[0] * 0.01;
                pt.pos[2] += pt.vel_xz[1] * 0.01;
                pt.pos[1] -= pt.rise; // −Y is up.
                if pt.process < 30 {
                    pt.alpha = (pt.alpha + 5.0 / 255.0).min(1.0);
                } else {
                    pt.alpha -= 2.0 / 255.0;
                }
            }
        }
        self.particles
            .retain(|pt| !(pt.process >= 30 && pt.alpha <= 0.0));
    }
}

impl Effect for PeongUpEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        if self.particles.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let tint = [
            TINT_RGB[0] as f32 / 255.0,
            TINT_RGB[1] as f32 / 255.0,
            TINT_RGB[2] as f32 / 255.0,
        ];
        for pt in &self.particles {
            if pt.alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: pt.pos,
                size: [pt.size, pt.size],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                rotation: 0.0,
                texture: PEONG_TEXTURE,
                color: [tint[0], tint[1], tint[2], pt.alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(e: &mut PeongUpEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        st
    }

    fn billboards(e: &PeongUpEffect) -> Vec<([f32; 3], [f32; 4])> {
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 256.0,
                screen_h: 256.0,
                elapsed: 0.0,
            },
        );
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard {
                    pos,
                    color,
                    blend: BlendKind::Additive,
                    ..
                } => (*pos, *color),
                _ => panic!("expected additive Billboard sparkles"),
            })
            .collect()
    }

    #[test]
    fn spawns_blue_sparkle_field() {
        let mut e = PeongUpEffect::new([0.0; 3], PEONGUP);
        tick(&mut e, 40);
        let bb = billboards(&e);
        assert!(!bb.is_empty(), "sparkles visible");
        let (_, c) = bb[0];
        assert!(c[2] > c[0] && c[2] > c[1], "sparkles are blue: {c:?}");
    }

    #[test]
    fn particles_rise_and_effect_dies() {
        let mut e = PeongUpEffect::new([0.0, 0.0, 0.0], PEONGUP);
        tick(&mut e, 32);
        let bb = billboards(&e);
        let y_early = bb.iter().map(|(p, _)| p[1]).sum::<f32>() / bb.len().max(1) as f32;
        tick(&mut e, 10);
        let bb = billboards(&e);
        if !bb.is_empty() {
            let y_late = bb.iter().map(|(p, _)| p[1]).sum::<f32>() / bb.len() as f32;
            assert!(
                y_late < y_early,
                "particles drift up (native −Y): {y_early} -> {y_late}"
            );
        }
        assert_eq!(tick(&mut e, 400), EffectStatus::Dead);
    }
}
