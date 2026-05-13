use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const RING_COUNT: usize = 3;
const RING_RADII: [f32; RING_COUNT] = [4.5, 5.0, 5.5];
const RING_Y_OFFSETS: [f32; RING_COUNT] = [-0.5, 0.0, 0.5];
const RING_BASE_ROT_DEG: [f32; RING_COUNT] = [0.0, 90.0, 180.0];
const RING_SPIN_DEG_PER_SEC: [f32; RING_COUNT] = [60.0, -45.0, 35.0];
const RING_ALPHA: [f32; RING_COUNT] = [0.55, 0.55, 0.55];
const BASE_COLOR: [f32; 3] = [0.55, 0.75, 1.0];

pub struct CastCircle {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl CastCircle {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for CastCircle {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for i in 0..RING_COUNT {
            let theta_deg = RING_BASE_ROT_DEG[i] + RING_SPIN_DEG_PER_SEC[i] * self.age;
            let theta = theta_deg.to_radians();
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
                RING_ALPHA[i] * self.tint[3],
            ];
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [
                    self.world_pos[0],
                    self.world_pos[1] + RING_Y_OFFSETS[i],
                    self.world_pos[2],
                ],
                size: [RING_RADII[i] * 2.0, RING_RADII[i] * 2.0],
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
    fn emits_three_rings_at_spawn_position() {
        let circle = CastCircle::new(&CustomParams {
            world_pos: [1.0, 2.0, 3.0],
            ..Default::default()
        });
        let mut list = EffectDrawList::new();
        circle.collect_draws(&mut list, &ctx());
        assert_eq!(list.len(), RING_COUNT);
        for prim in &list.primitives {
            let EffectPrimitiveDraw::Billboard { pos, blend, .. } = prim else {
                panic!("CastCircle should emit billboards");
            };
            assert!((pos[0] - 1.0).abs() < 0.01);
            assert!((pos[2] - 3.0).abs() < 0.01);
            assert_eq!(*blend, BlendKind::Additive);
        }
    }
}
