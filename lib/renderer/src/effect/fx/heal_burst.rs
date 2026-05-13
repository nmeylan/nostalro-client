use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const SPARKLE_COUNT: usize = 8;
const RISE_HEIGHT: f32 = 9.0;
const SPARKLE_SIZE: f32 = 1.2;
const RADIUS: f32 = 3.0;
const STAGGER_SEC: f32 = 0.08;
const SPARKLE_DURATION: f32 = 0.7;
const BASE_COLOR: [f32; 3] = [0.85, 1.0, 0.8];

pub struct HealBurst {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl HealBurst {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for HealBurst {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        for i in 0..SPARKLE_COUNT {
            let local = self.age - i as f32 * STAGGER_SEC;
            if local < 0.0 || local > SPARKLE_DURATION {
                continue;
            }
            let progress = local / SPARKLE_DURATION;
            let theta = std::f32::consts::TAU * (i as f32 / SPARKLE_COUNT as f32);
            let (sin_t, cos_t) = theta.sin_cos();
            let alpha = 1.0 - progress;
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [
                    self.world_pos[0] + cos_t * RADIUS,
                    self.world_pos[1] + progress * RISE_HEIGHT,
                    self.world_pos[2] + sin_t * RADIUS,
                ],
                size: [SPARKLE_SIZE, SPARKLE_SIZE],
                uv,
                texture: self.texture,
                color: [
                    BASE_COLOR[0] * self.tint[0],
                    BASE_COLOR[1] * self.tint[1],
                    BASE_COLOR[2] * self.tint[2],
                    alpha * self.tint[3],
                ],
                blend: BlendKind::Additive,
            });
        }
    }
}
