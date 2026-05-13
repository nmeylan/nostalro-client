use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const QUAD_SIZE: f32 = 4.0;
const BASE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.85];

pub struct FlatQuad {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl FlatQuad {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for FlatQuad {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        out.push(EffectPrimitiveDraw::Billboard {
            pos: self.world_pos,
            size: [QUAD_SIZE, QUAD_SIZE],
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
