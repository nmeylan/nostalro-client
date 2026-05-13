use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const SHARD_COUNT: usize = 8;
const TRAVEL_DISTANCE: f32 = 7.0;
const SHARD_DURATION: f32 = 0.35;
const SHARD_SIZE: f32 = 2.2;
const BASE_COLOR: [f32; 3] = [1.0, 0.95, 0.6];

pub struct RadialBurst {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl RadialBurst {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for RadialBurst {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.age > SHARD_DURATION {
            return;
        }
        let progress = self.age / SHARD_DURATION;
        let radius = TRAVEL_DISTANCE * progress;
        let alpha = 1.0 - progress;
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        for i in 0..SHARD_COUNT {
            let theta = std::f32::consts::TAU * (i as f32 / SHARD_COUNT as f32);
            let (sin_t, cos_t) = theta.sin_cos();
            let pos = [
                self.world_pos[0] + cos_t * radius,
                self.world_pos[1] + 2.0,
                self.world_pos[2] + sin_t * radius,
            ];
            let color = [
                BASE_COLOR[0] * self.tint[0],
                BASE_COLOR[1] * self.tint[1],
                BASE_COLOR[2] * self.tint[2],
                alpha * self.tint[3],
            ];
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [SHARD_SIZE, SHARD_SIZE],
                uv,
                texture: self.texture,
                color,
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectRenderCtx<'static> {
        static CAM: std::sync::OnceLock<crate::camera::Camera> = std::sync::OnceLock::new();
        let cam = CAM.get_or_init(crate::camera::Camera::default);
        EffectRenderCtx {
            camera: cam,
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn shards_expand_outward_and_fade() {
        let mut burst = RadialBurst::new(&CustomParams::default());
        let mut early = EffectDrawList::new();
        burst.update(&EffectUpdateCtx { dt: 0.05 });
        burst.collect_draws(&mut early, &ctx());

        let mut late = EffectDrawList::new();
        burst.update(&EffectUpdateCtx { dt: 0.2 });
        burst.collect_draws(&mut late, &ctx());

        let early_alpha = match &early.primitives[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        };
        let late_alpha = match &late.primitives[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(late_alpha < early_alpha);

        burst.update(&EffectUpdateCtx { dt: 1.0 });
        let mut done = EffectDrawList::new();
        burst.collect_draws(&mut done, &ctx());
        assert!(done.is_empty());
    }
}
