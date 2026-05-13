use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const LAYER_COUNT: usize = 6;
const TOTAL_HEIGHT: f32 = 30.0;
const RADIUS: f32 = 6.5;
const SPIN_DEG_PER_SEC: f32 = 90.0;
const BASE_COLOR: [f32; 4] = [1.0, 0.95, 0.55, 0.45];

pub struct CylinderPillar {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl CylinderPillar {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for CylinderPillar {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let layer_height = TOTAL_HEIGHT / LAYER_COUNT as f32;
        for i in 0..LAYER_COUNT {
            let direction = if i % 2 == 0 { 1.0 } else { -1.0 };
            let theta = (self.age * SPIN_DEG_PER_SEC * direction).to_radians();
            let (sin_t, cos_t) = theta.sin_cos();
            let rotate = |u: f32, v: f32| -> [f32; 2] {
                let cu = u - 0.5;
                let cv = v - 0.5;
                [cu * cos_t - cv * sin_t + 0.5, cu * sin_t + cv * cos_t + 0.5]
            };
            let uv = [
                rotate(0.0, 0.0),
                rotate(1.0, 0.0),
                rotate(0.0, 1.0),
                rotate(1.0, 1.0),
            ];
            let color = [
                BASE_COLOR[0] * self.tint[0],
                BASE_COLOR[1] * self.tint[1],
                BASE_COLOR[2] * self.tint[2],
                BASE_COLOR[3] * self.tint[3],
            ];
            let y = self.world_pos[1] + (i as f32 + 0.5) * layer_height;
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [self.world_pos[0], y, self.world_pos[2]],
                size: [RADIUS * 2.0, layer_height * 1.2],
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
    fn stacks_layers_vertically() {
        let pillar = CylinderPillar::new(&CustomParams {
            world_pos: [5.0, 0.0, 5.0],
            ..Default::default()
        });
        let mut list = EffectDrawList::new();
        pillar.collect_draws(&mut list, &ctx());
        assert_eq!(list.len(), LAYER_COUNT);
        let ys: Vec<f32> = list
            .primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, .. } => pos[1],
                _ => unreachable!(),
            })
            .collect();
        for w in ys.windows(2) {
            assert!(w[1] > w[0]);
        }
    }
}
