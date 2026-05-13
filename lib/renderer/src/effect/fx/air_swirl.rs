use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const PUFFS: usize = 6;
const RADIUS: f32 = 4.0;
const PUFF_SIZE: f32 = 3.0;
const RISE: f32 = 6.0;
const SPIN_DEG_PER_SEC: f32 = 120.0;
const BASE_COLOR: [f32; 4] = [0.85, 0.9, 1.0, 0.45];

pub struct AirSwirl {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl AirSwirl {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for AirSwirl {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        for i in 0..PUFFS {
            let base = std::f32::consts::TAU * (i as f32 / PUFFS as f32);
            let theta = base + (self.age * SPIN_DEG_PER_SEC).to_radians();
            let (sin_t, cos_t) = theta.sin_cos();
            let bob = ((self.age * 2.5) + i as f32 * 0.7).sin() * 0.5 + 0.5;
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [
                    self.world_pos[0] + cos_t * RADIUS,
                    self.world_pos[1] + bob * RISE,
                    self.world_pos[2] + sin_t * RADIUS,
                ],
                size: [PUFF_SIZE, PUFF_SIZE],
                uv,
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
}
