use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const SLASH_SIZE_W: f32 = 6.0;
const SLASH_SIZE_H: f32 = 2.5;
const SLASH_DURATION: f32 = 0.18;
const BASE_COLOR: [f32; 3] = [1.0, 0.95, 0.7];

pub struct MeleeImpact {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl MeleeImpact {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for MeleeImpact {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        if self.age > SLASH_DURATION {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.age > SLASH_DURATION {
            return;
        }
        let progress = self.age / SLASH_DURATION;
        let alpha = 1.0 - progress;
        let scale = 1.0 + progress * 0.4;
        out.push(EffectPrimitiveDraw::Billboard {
            pos: [
                self.world_pos[0],
                self.world_pos[1] + SLASH_SIZE_H * 0.5,
                self.world_pos[2],
            ],
            size: [SLASH_SIZE_W * scale, SLASH_SIZE_H * scale],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
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
