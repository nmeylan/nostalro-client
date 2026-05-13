use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const ORB_SIZE: f32 = 1.6;
const ORBIT_RADIUS: f32 = 2.0;
const ORBIT_DEG_PER_SEC: f32 = 180.0;
const ORB_HEIGHT: f32 = 6.0;
const BASE_COLOR: [f32; 4] = [0.9, 0.7, 1.0, 0.85];

pub struct StatusOrb {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl StatusOrb {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for StatusOrb {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let theta = (self.age * ORBIT_DEG_PER_SEC).to_radians();
        let (sin_t, cos_t) = theta.sin_cos();
        out.push(EffectPrimitiveDraw::Billboard {
            pos: [
                self.world_pos[0] + cos_t * ORBIT_RADIUS,
                self.world_pos[1] + ORB_HEIGHT,
                self.world_pos[2] + sin_t * ORBIT_RADIUS,
            ],
            size: [ORB_SIZE, ORB_SIZE],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            texture: self.texture,
            color: [
                BASE_COLOR[0] * self.tint[0],
                BASE_COLOR[1] * self.tint[1],
                BASE_COLOR[2] * self.tint[2],
                BASE_COLOR[3] * self.tint[3],
            ],
            blend: BlendKind::Additive,
        });
    }
}
