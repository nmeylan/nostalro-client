//! `EF_PEONG` (id 411) — flower-pop burst used by the wedding / bloom effect.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const START_FRAME: f32 = 0.0;
const WORLD_SCALE: f32 = 1.6;

const RISING_COUNT: usize = 20;
const BURST_SECTORS: usize = 4;
const BURST_PER_SECTOR: usize = 16;

const PEONG_TEXTURES: [&str; 4] = ["peong1.tga", "peong2.tga", "peong3.tga", "peong2.tga"];

pub const TEXTURES: &[&str] = &["peong1.tga", "peong2.tga", "peong3.tga"];

pub const SPARKLE_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SPRITES: &[&str] = &[SPARKLE_SPRITE];
const SPARKLE_ANIM_TICKS: f32 = 3.0;
pub const TOTAL_DURATION_MS: u32 = 2600;

struct Rng(u32);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
    fn int(&mut self, n: u32) -> i32 {
        (self.next_u32() % n) as i32
    }
}

struct Mote {
    pos: [f32; 3],
    size: f32,
    alpha: f32,
    process: i32,
    texture: &'static str,
    kind: Kind,
}

enum Kind {
    Sparkle {
        rise: f32,
        amp: f32,
        wx_phase: f32,
        wx_speed: f32,
        wz_phase: f32,
        wz_speed: f32,
        grow: f32,
    },
    Burst {
        base_y: f32,
        drift: [f32; 2],
    },
}

impl Mote {
    fn step(&mut self) {
        self.process += 1;
        if self.process <= 0 {
            return;
        }
        match &mut self.kind {
            Kind::Sparkle {
                rise,
                amp,
                wx_phase,
                wx_speed,
                wz_phase,
                wz_speed,
                grow,
            } => {
                *wx_phase += *wx_speed;
                *wz_phase += *wz_speed;
                self.pos[0] += *amp * wx_phase.sin();
                self.pos[2] += *amp * wz_phase.sin();
                self.pos[1] -= *rise; // −Y is up.
                self.size += *grow;
                if self.process < 30 {
                    self.alpha = (self.alpha + 5.0 / 255.0).min(1.0);
                } else {
                    self.alpha -= 2.0 / 255.0;
                }
            }
            Kind::Burst { base_y, drift } => {
                if self.process <= 90 {
                    self.pos[0] += drift[0];
                    self.pos[2] += drift[1];
                    let arc = (self.process as f32 * 2.0).to_radians().sin();
                    self.pos[1] = *base_y - arc * 1.5 * WORLD_SCALE;
                }
                if self.process <= 25 {
                    self.alpha = (self.alpha + 2.0 / 255.0).min(1.0);
                }
                if self.process > 70 {
                    self.alpha -= 1.0 / 255.0;
                }
            }
        }
    }

    fn dead(&self) -> bool {
        self.process > 25 && self.alpha <= 0.0
    }
}

pub struct PeongEffect {
    anchor: [f32; 3],
    motes: Vec<Mote>,
    age_frames: f32,
    step_accumulator: f32,
    spawned: bool,
}

impl PeongEffect {
    pub fn new(anchor: [f32; 3]) -> Self {
        Self {
            anchor,
            motes: Vec::new(),
            age_frames: 0.0,
            step_accumulator: 0.0,
            spawned: false,
        }
    }

    fn spawn(&mut self) {
        let [ax, ay, az] = self.anchor;
        let seed = ax.to_bits() ^ az.to_bits() ^ 0x9E37_79B9;
        let mut rng = Rng(seed | 1);

        for _ in 0..RISING_COUNT {
            let angle = rng.range(0.0, std::f32::consts::TAU);
            let radius = rng.range(0.0, 0.8) * WORLD_SCALE;
            let up = rng.range(0.0, 3.0) * WORLD_SCALE;
            let size = (0.2 + rng.range(0.0, 0.2)) * WORLD_SCALE;
            let grow = 0.012 * WORLD_SCALE;
            let rise = (0.10 + rng.range(0.0, 0.08)) * WORLD_SCALE;
            let amp = 0.08 * WORLD_SCALE;
            self.motes.push(Mote {
                pos: [
                    ax + angle.cos() * radius,
                    ay - up,
                    az + angle.sin() * radius,
                ],
                size,
                alpha: 0.0,
                process: -rng.int(26),
                texture: SPARKLE_SPRITE,
                kind: Kind::Sparkle {
                    rise,
                    amp,
                    wx_phase: rng.range(0.0, std::f32::consts::TAU),
                    wx_speed: rng.range(0.03, 0.09),
                    wz_phase: rng.range(0.0, std::f32::consts::TAU),
                    wz_speed: rng.range(0.03, 0.09),
                    grow,
                },
            });
        }

        for sector in 0..BURST_SECTORS {
            let base = sector as f32 * 90.0;
            for k in 0..BURST_PER_SECTOR {
                let angle = (base + rng.range(0.0, 90.0)).to_radians();
                let offset = (2.0 + rng.range(0.0, 0.8)) * WORLD_SCALE;
                let size = (2.5 + rng.range(0.0, 2.0)) * WORLD_SCALE;
                let base_y = ay - 4.0 * WORLD_SCALE; // −Y is up.
                let drift = [
                    angle.cos() * 0.05 * WORLD_SCALE,
                    angle.sin() * 0.05 * WORLD_SCALE,
                ];
                self.motes.push(Mote {
                    pos: [ax + angle.cos() * offset, base_y, az + angle.sin() * offset],
                    size,
                    alpha: 0.0,
                    process: -rng.int(11),
                    texture: PEONG_TEXTURES[k % 4],
                    kind: Kind::Burst { base_y, drift },
                });
            }
        }
        self.spawned = true;
    }
}

impl Effect for PeongEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if !self.spawned {
            if self.age_frames < START_FRAME {
                return EffectStatus::Running;
            }
            self.spawn();
        }
        self.step_accumulator += ctx.delta * FRAMES_PER_SECOND;
        while self.step_accumulator >= 1.0 {
            for m in &mut self.motes {
                m.step();
            }
            self.step_accumulator -= 1.0;
        }
        self.motes.retain(|m| !m.dead());
        if self.spawned && self.motes.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for m in &self.motes {
            if m.alpha <= 0.0 {
                continue;
            }
            match m.kind {
                Kind::Sparkle { .. } => {
                    let cell = (m.process.max(0) as f32 / SPARKLE_ANIM_TICKS) as usize;
                    out.push(EffectPrimitiveDraw::SpriteParticle {
                        sprite_path: SPARKLE_SPRITE,
                        position: m.pos,
                        action_index: 0,
                        motion_index: cell,
                        size_scale: m.size,
                        color: [1.0, 1.0, 1.0, m.alpha],
                        blend: BlendKind::Additive,
                        aim_target: None,
                        no_depth: false,
                    });
                }
                Kind::Burst { .. } => {
                    out.push(EffectPrimitiveDraw::Billboard {
                        pos: m.pos,
                        size: [m.size, m.size],
                        uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                        rotation: 0.0,
                        texture: m.texture,
                        color: [1.0, 1.0, 1.0, m.alpha],
                        blend: BlendKind::Alpha,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(e: &mut PeongEffect, frames: u32) -> EffectStatus {
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

    fn draws(e: &PeongEffect) -> Vec<EffectPrimitiveDraw> {
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
    }

    fn sparkle_y_sum(prims: &[EffectPrimitiveDraw]) -> f32 {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { position, .. } => Some(position[1]),
                _ => None,
            })
            .sum()
    }

    #[test]
    fn blooms_in_promptly_after_spawn() {
        let mut e = PeongEffect::new([0.0; 3]);
        tick(&mut e, 30);
        assert!(!draws(&e).is_empty(), "cluster has bloomed in");
    }

    #[test]
    fn sparkles_surround_the_swirl_flower_at_peak() {
        let mut e = PeongEffect::new([0.0; 3]);
        tick(&mut e, 60);
        let prims = draws(&e);
        assert!(
            prims.iter().any(|p| matches!(
                p,
                EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == SPARKLE_SPRITE
            )),
            "twinkling sparkle sprites present"
        );
        assert!(
            prims.iter().any(|p| matches!(
                p,
                EffectPrimitiveDraw::Billboard { texture, .. } if texture.starts_with("peong")
            )),
            "swirl-flower billboards present"
        );
    }

    #[test]
    fn sparkles_drift_upward() {
        let mut e = PeongEffect::new([0.0; 3]);
        tick(&mut e, 40);
        let y_early = sparkle_y_sum(&draws(&e));
        tick(&mut e, 8);
        let y_late = sparkle_y_sum(&draws(&e));
        assert!(
            y_late < y_early,
            "sparkles drift up (native −Y): {y_early} -> {y_late}"
        );
    }

    #[test]
    fn effect_dies_after_the_bloom() {
        let mut e = PeongEffect::new([0.0; 3]);
        assert_eq!(tick(&mut e, 300), EffectStatus::Dead);
    }

    #[test]
    fn mote_advance_is_frame_rate_independent() {
        let half_second = |dt: f32, steps: u32| {
            let mut e = PeongEffect::new([0.0; 3]);
            for _ in 0..steps {
                e.update(&EffectUpdateCtx {
                    delta: dt,
                    camera_target: None,
                    caster_yaw: None,
                });
            }
            e.motes.iter().map(|m| m.process).sum::<i32>()
        };
        let fine = half_second(1.0 / 60.0, 30);
        let coarse = half_second(2.0 / 60.0, 15);
        assert_eq!(fine, coarse, "process advances by wall-clock, not per call");
    }
}
