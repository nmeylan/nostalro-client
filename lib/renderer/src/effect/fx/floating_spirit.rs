use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const SPIRIT_SIZE: f32 = 2.5;
const FLOAT_AMPLITUDE: f32 = 1.5;
const BASE_HEIGHT: f32 = 5.0;
const FLOAT_HZ: f32 = 0.6;
const BASE_COLOR: [f32; 4] = [0.9, 0.95, 1.0, 0.7];

pub struct FloatingSpirit {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl FloatingSpirit {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for FloatingSpirit {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let bob = (self.age * FLOAT_HZ * std::f32::consts::TAU).sin();
        out.push(EffectPrimitiveDraw::Billboard {
            pos: [
                self.world_pos[0],
                self.world_pos[1] + BASE_HEIGHT + bob * FLOAT_AMPLITUDE,
                self.world_pos[2],
            ],
            size: [SPIRIT_SIZE, SPIRIT_SIZE],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            texture: self.texture,
            color: [
                BASE_COLOR[0] * self.tint[0],
                BASE_COLOR[1] * self.tint[1],
                BASE_COLOR[2] * self.tint[2],
                BASE_COLOR[3] * self.tint[3],
            ],
            blend: BlendKind::Alpha,
        });
    }
}
