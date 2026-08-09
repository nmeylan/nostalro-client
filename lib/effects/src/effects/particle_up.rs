//! Rising-sparkle status effects — Hptime/Sptime, Hated/Hated2, SmaReady, Sprinklesand (ids 331, 332, 543, 572, 546, 310).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{BodyCopy, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const SPAWN_PERIOD: u32 = 4;
const FLASH_SWELL: f32 = 25.0;
const FLASH_FADE_START: f32 = 150.0;
const FLASH_END: f32 = 175.0;

fn rgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

#[derive(Clone, Copy)]
pub struct ParticleUpParams {
    pub texture: &'static str,
    pub tint_rgb: (u8, u8, u8),
    pub spawn_start: u32,
    pub spawn_end: u32,
    pub prims_per_spawn: usize,
    pub spread: f32,
    pub base_dist: f32,
    pub dist_rand: f32,
    pub rise_base: f32,
    pub rise_rand: f32,
    /// Spawn height above the anchor; −Y is up.
    pub y_offset: f32,
    pub stagger_start: bool,
    pub glow_scale: f32,
    /// `false` keeps motes axis-aligned; spinning smears the star texture.
    pub spin: bool,
    /// Additive body flash carried alongside the motes: it swells over the first
    /// 25 frames, holds, then fades out between frames 150 and 175.
    pub body_flash: Option<[u8; 3]>,
}

const fn p(texture: &'static str, tint_rgb: (u8, u8, u8)) -> ParticleUpParams {
    ParticleUpParams {
        texture,
        tint_rgb,
        spawn_start: 0,
        spawn_end: 20,
        prims_per_spawn: 1,
        spread: 3.0,
        base_dist: 2.5,
        dist_rand: 1.5,
        rise_base: 0.2,
        rise_rand: 0.2,
        y_offset: -6.0,
        stagger_start: false,
        glow_scale: 0.0,
        spin: true,
        body_flash: None,
    }
}

pub const HPTIME: ParticleUpParams = p("pok1.tga", (220, 250, 220));
pub const HEAL_MOTE: ParticleUpParams = ParticleUpParams {
    base_dist: 2.83,
    dist_rand: 1.41,
    spawn_end: 50,
    spread: 6.0,
    rise_base: 0.3,
    rise_rand: 0.4,
    y_offset: 0.0,
    spin: false,
    ..p("pok1.tga", (220, 250, 220))
};
pub const SPTIME: ParticleUpParams = p("pok1.tga", (150, 150, 250));
pub const HATED: ParticleUpParams = ParticleUpParams {
    body_flash: Some([5, 5, 255]),
    spawn_end: 80,
    prims_per_spawn: 2, // denser field
    spread: 5.0,
    base_dist: 1.2,
    dist_rand: 0.8,
    rise_base: 0.1,
    rise_rand: 0.3,
    ..p("pok1.tga", (150, 150, 250))
};
pub const HATED2: ParticleUpParams = ParticleUpParams {
    tint_rgb: (250, 100, 100),
    body_flash: None,
    ..HATED
};
pub const SMAREADY: ParticleUpParams = ParticleUpParams {
    spawn_start: 40,
    spawn_end: 120,
    body_flash: None,
    ..HATED
};
pub const SPRINKLESAND: ParticleUpParams = ParticleUpParams {
    spawn_end: 0,
    prims_per_spawn: 20,
    spread: 4.0,
    base_dist: 0.8,
    dist_rand: 1.2,
    rise_base: 0.2,
    rise_rand: 0.4,
    stagger_start: true,
    ..p("thunder_center.bmp", (250, 250, 150))
};

pub const SMA3: ParticleUpParams = ParticleUpParams {
    spawn_end: 0,
    spread: 4.0,
    base_dist: 1.2,
    dist_rand: 0.8,
    rise_base: 0.1,
    rise_rand: 0.2,
    ..p("thunder_ball_0002.bmp", (120, 120, 255))
};
pub const SMA3_TOTAL_DURATION_MS: u32 = 1100;

pub const TEXTURES: &[&str] = &[
    "pok1.tga",
    "pok3.tga",
    "thunder_center.bmp",
    "thunder_ball_0002.bmp",
];

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
    size: f32,
    rise: f32,
    rotation: f32,
    process: i32,
    alpha: f32,
}

pub struct ParticleUpEffect {
    params: ParticleUpParams,
    center: [f32; 3],
    rng: Rng,
    particles: Vec<Particle>,
    frame: u32,
    frame_accum: f32,
}

impl ParticleUpEffect {
    pub fn new(anchor: [f32; 3], params: ParticleUpParams) -> Self {
        let seed = anchor[0].to_bits() ^ anchor[2].to_bits() ^ 0x9A1C_0FF1;
        Self {
            params,
            center: anchor,
            rng: Rng(seed | 1),
            particles: Vec::new(),
            frame: 0,
            frame_accum: 0.0,
        }
    }

    fn spawn_burst(&mut self) {
        for _ in 0..self.params.prims_per_spawn * 4 {
            let process = if self.params.stagger_start {
                -10 + (self.rng.next_u32() % 10) as i32
            } else {
                0
            };
            self.particles.push(Particle {
                pos: [
                    self.center[0] + self.rng.range(-self.params.spread, self.params.spread),
                    self.center[1] + self.params.y_offset,
                    self.center[2] + self.rng.range(-self.params.spread, self.params.spread),
                ],
                size: self.params.base_dist + self.rng.range(0.0, self.params.dist_rand),
                rise: self.params.rise_base + self.rng.range(0.0, self.params.rise_rand),
                rotation: if self.params.spin {
                    self.rng.range(0.0, std::f32::consts::TAU)
                } else {
                    0.0
                },
                process,
                alpha: 0.0,
            });
        }
    }

    fn step_frame(&mut self) {
        if self.frame >= self.params.spawn_start
            && self.frame <= self.params.spawn_end
            && (self.frame - self.params.spawn_start) % SPAWN_PERIOD == 0
        {
            self.spawn_burst();
        }
        for pt in &mut self.particles {
            pt.process += 1;
            if pt.process > 0 {
                pt.pos[1] -= pt.rise; // native -Y = up
                if self.params.spin {
                    pt.rotation -= 5.0_f32.to_radians();
                }
                if pt.process <= 10 {
                    pt.alpha = (pt.alpha + 15.0 / 255.0).min(150.0 / 255.0);
                } else {
                    pt.alpha -= 3.0 / 255.0;
                }
            }
        }
        self.particles
            .retain(|pt| !(pt.process > 10 && pt.alpha <= 0.0));
        self.frame += 1;
    }
}

impl Effect for ParticleUpEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        let flash_over = self.params.body_flash.is_none() || self.frame as f32 > FLASH_END;
        if flash_over && self.frame > self.params.spawn_end && self.particles.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        let tint = self.params.body_flash?;
        let f = self.frame as f32;
        let swell = if f <= FLASH_SWELL {
            f
        } else if f <= FLASH_FADE_START {
            FLASH_SWELL
        } else {
            FLASH_END - f
        };
        let alpha_255 = if swell <= 10.0 {
            swell * 15.0
        } else if swell <= 20.0 {
            160.0
        } else {
            155.0 - (swell - 20.0) * 5.0
        };
        if alpha_255 <= 0.0 {
            return None;
        }
        let copy = BodyCopy {
            offset_px: [0.0, 0.0],
            margin_px: 0.0,
            scale: [1.0, 1.0],
            tint,
            alpha: (alpha_255 / 255.0).clamp(0.0, 1.0),
            additive: true,
            behind: false,
            body_layers_only: false,
        };
        Some(vec![copy, copy])
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        let delta = [
            pos[0] - self.center[0],
            pos[1] - self.center[1],
            pos[2] - self.center[2],
        ];
        self.center = pos;
        for p in &mut self.particles {
            p.pos[0] += delta[0];
            p.pos[1] += delta[1];
            p.pos[2] += delta[2];
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let (r, g, b) = self.params.tint_rgb;
        let tint = rgb(r, g, b);
        for pt in &self.particles {
            if pt.alpha <= 0.0 {
                continue;
            }
            if self.params.glow_scale > 0.0 {
                let glow = pt.size * self.params.glow_scale;
                out.push(EffectPrimitiveDraw::BillboardSpriteDisc {
                    pos: pt.pos,
                    size: [glow, glow],
                    segments: 20,
                    rotation: pt.rotation,
                    texture: self.params.texture,
                    color: [tint[0], tint[1], tint[2], pt.alpha * 0.4],
                    blend: BlendKind::Additive,
                });
            }
            out.push(EffectPrimitiveDraw::BillboardSpriteDisc {
                pos: pt.pos,
                size: [pt.size, pt.size],
                segments: 20,
                rotation: pt.rotation,
                texture: self.params.texture,
                color: [tint[0], tint[1], tint[2], pt.alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(e: &mut ParticleUpEffect, frames: u32) -> EffectStatus {
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

    fn billboards(e: &ParticleUpEffect) -> Vec<([f32; 3], [f32; 4])> {
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
                EffectPrimitiveDraw::BillboardSpriteDisc {
                    pos,
                    color,
                    blend: BlendKind::Additive,
                    ..
                } => (*pos, *color),
                _ => panic!("expected additive disc sparkles"),
            })
            .collect()
    }

    #[test]
    fn spawns_bursts_and_tints_each_variant() {
        let mut e = ParticleUpEffect::new([0.0; 3], HPTIME);
        tick(&mut e, 12); // past first burst's fade-in
        let bb = billboards(&e);
        assert!(!bb.is_empty(), "sparkles spawned");
        // green channel dominates blue for the HP variant.
        let (_, c) = bb[0];
        assert!(c[1] > c[2], "HP sparkles are greenish: {c:?}");
    }

    #[test]
    fn hated_pairs_its_motes_with_a_blue_body_flash_that_swells_then_fades() {
        let mut e = ParticleUpEffect::new([0.0; 3], HATED);
        tick(&mut e, 20);
        let swelling = e.body_copies().expect("flashing");
        assert_eq!(swelling.len(), 2);
        assert!(swelling.iter().all(|c| c.additive && c.tint == [5, 5, 255]));
        tick(&mut e, 150);
        let fading = e.body_copies().expect("still fading");
        assert!(fading[0].alpha < swelling[0].alpha);
        assert_eq!(tick(&mut e, 20), EffectStatus::Dead);

        let hated2 = ParticleUpEffect::new([0.0; 3], HATED2);
        assert!(hated2.body_copies().is_none(), "only Hated flashes");
    }

    #[test]
    fn particles_rise_and_effect_eventually_dies() {
        let mut e = ParticleUpEffect::new([0.0, 0.0, 0.0], SPTIME);
        tick(&mut e, 6);
        let y_early = billboards(&e).iter().map(|(p, _)| p[1]).sum::<f32>()
            / billboards(&e).len().max(1) as f32;
        tick(&mut e, 8);
        let bb = billboards(&e);
        if !bb.is_empty() {
            let y_late = bb.iter().map(|(p, _)| p[1]).sum::<f32>() / bb.len() as f32;
            assert!(
                y_late < y_early,
                "particles drift up (native -Y): {y_early} -> {y_late}"
            );
        }
        assert_eq!(tick(&mut e, 400), EffectStatus::Dead);
    }
}
