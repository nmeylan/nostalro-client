use std::f32::consts::{SQRT_2, TAU};

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

const WORLD_SCALE: f32 = 0.5;
const SPARKLE_SIZE_SCALE: f32 = 0.7;

const SPARKLE_TEXTURE: &str = "whitelight.tga";
pub const TEXTURES: &[&str] = &[SPARKLE_TEXTURE];

const SPAWN_FRAME: f32 = 30.0;
const SPARKLE_COUNT: usize = 48;

const ALPHA_INIT_255: f32 = 230.0;
const ALPHA_FALL_PER_FRAME: f32 = 6.0;
const DECAY_START_FRAME: f32 = 25.0;
const SPEED_DECAY: f32 = 0.9;

const LIFE_FRAMES: f32 = DECAY_START_FRAME + ALPHA_INIT_255 / ALPHA_FALL_PER_FRAME;
const TOTAL_FRAMES: f32 = SPAWN_FRAME + LIFE_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

const TINTS: [[f32; 3]; 3] = [
    [1.0, 200.0 / 255.0, 200.0 / 255.0],
    [1.0, 160.0 / 255.0, 160.0 / 255.0],
    [1.0, 120.0 / 255.0, 120.0 / 255.0],
];

#[derive(Clone, Copy)]
struct Sparkle {
    dir: [f32; 3],
    size: f32,
    tint: [f32; 3],
}

pub struct CartterEffect {
    anchor: [f32; 3],
    sparkles: [Sparkle; SPARKLE_COUNT],
    age_frames: f32,
    cum_height: f32,
    max_height: f32,
    stepped: i32,
}

impl CartterEffect {
    pub fn new(anchor: [f32; 3]) -> Self {
        let mut state = (anchor[0].to_bits() ^ anchor[2].to_bits().rotate_left(13)) ^ 0xCA77_7E12;
        let mut lcg = || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) as f32 / ((1u32 << 24) as f32)
        };
        let sparkles = std::array::from_fn(|_| {
            let azim = lcg() * TAU;
            let elev = lcg() * std::f32::consts::PI;
            let (se, ce) = elev.sin_cos();
            let (sa, ca) = azim.sin_cos();
            let distance = 2.5 + lcg() * 0.6;
            let tint = TINTS[(lcg() * 3.0) as usize % 3];
            Sparkle {
                dir: [ce * sa, -se, ce * ca],
                size: distance * SQRT_2 * SPARKLE_SIZE_SCALE,
                tint,
            }
        });
        Self {
            anchor,
            sparkles,
            age_frames: 0.0,
            cum_height: 0.0,
            max_height: 1.0,
            stepped: 0,
        }
    }

    fn process(&self) -> f32 {
        self.age_frames - SPAWN_FRAME
    }

    fn alpha(&self) -> f32 {
        let p = self.process();
        if p <= DECAY_START_FRAME {
            ALPHA_INIT_255 / 255.0
        } else {
            (ALPHA_INIT_255 - ALPHA_FALL_PER_FRAME * (p - DECAY_START_FRAME)).max(0.0) / 255.0
        }
    }
}

impl Effect for CartterEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = self.process().floor() as i32;
        while self.stepped < target {
            self.cum_height += self.max_height;
            if (self.stepped as f32) >= DECAY_START_FRAME {
                self.max_height *= SPEED_DECAY;
            }
            self.stepped += 1;
        }
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.process() <= 0.0 {
            return;
        }
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return;
        }
        let drift = self.cum_height * WORLD_SCALE;
        for s in &self.sparkles {
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [
                    self.anchor[0] + s.dir[0] * drift,
                    self.anchor[1] + s.dir[1] * drift,
                    self.anchor[2] + s.dir[2] * drift,
                ],
                size: [s.size, s.size],
                uv: UNIT_UV,
                rotation: 0.0,
                texture: SPARKLE_TEXTURE,
                color: [s.tint[0], s.tint[1], s.tint[2], alpha],
                blend: BlendKind::Additive,
            });
        }
    }
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

    fn step(e: &mut CartterEffect, frames: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        s
    }

    fn sparkles(e: &CartterEffect) -> Vec<([f32; 3], f32)> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, color, .. } => Some((*pos, color[3])),
                _ => None,
            })
            .collect()
    }

    fn spread(s: &[([f32; 3], f32)]) -> f32 {
        s.iter()
            .map(|(p, _)| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
            .sum::<f32>()
            / s.len() as f32
    }

    #[test]
    fn dormant_until_frame_30_then_48_sparkles() {
        let mut e = CartterEffect::new([0.0; 3]);
        step(&mut e, 20);
        assert!(sparkles(&e).is_empty(), "no sparkles before frame 30");
        step(&mut e, 12);
        assert_eq!(
            sparkles(&e).len(),
            SPARKLE_COUNT,
            "all 48 sparkles after frame 30"
        );
    }

    #[test]
    fn sparkles_drift_outward_then_slow() {
        let mut e = CartterEffect::new([0.0; 3]);
        step(&mut e, 35);
        let early = spread(&sparkles(&e));
        step(&mut e, 20);
        let mid = spread(&sparkles(&e));
        assert!(mid > early, "cloud expands: {early} -> {mid}");
        let g1 = mid - early;
        step(&mut e, 20);
        let late = spread(&sparkles(&e));
        let g2 = late - mid;
        assert!(
            g2 < g1,
            "displacement per frame shrinks after decay: {g1} -> {g2}"
        );
    }

    #[test]
    fn alpha_decays_to_zero_and_terminates() {
        let mut e = CartterEffect::new([0.0; 3]);
        step(&mut e, (SPAWN_FRAME + DECAY_START_FRAME) as u32 + 1);
        let peak = sparkles(&e).first().map(|s| s.1).unwrap_or(0.0);
        let status = step(&mut e, (ALPHA_INIT_255 / ALPHA_FALL_PER_FRAME) as u32 + 5);
        assert!(peak > 0.5, "full alpha during the hold: {peak}");
        assert_eq!(status, EffectStatus::Dead);
    }
}
