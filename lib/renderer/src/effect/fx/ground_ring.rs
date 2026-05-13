use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const DEFAULT_RADIUS: f32 = 14.0;
const DEFAULT_COLOR: [f32; 4] = [0.6, 0.85, 1.0, 0.55];
const ROTATION_DEG_PER_SEC: f32 = 30.0;

pub struct GroundRing {
    world_pos: [f32; 3],
    tint: [f32; 4],
    radius: f32,
    age: f32,
    texture: &'static str,
}

impl GroundRing {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            radius: DEFAULT_RADIUS,
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for GroundRing {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let theta = (self.age * ROTATION_DEG_PER_SEC).to_radians();
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
            DEFAULT_COLOR[0] * self.tint[0],
            DEFAULT_COLOR[1] * self.tint[1],
            DEFAULT_COLOR[2] * self.tint[2],
            DEFAULT_COLOR[3] * self.tint[3],
        ];
        out.push(EffectPrimitiveDraw::Billboard {
            pos: self.world_pos,
            size: [self.radius * 2.0, self.radius * 2.0],
            uv,
            texture: self.texture,
            color,
            blend: BlendKind::Additive,
        });
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
    fn emits_one_billboard_at_spawn_position() {
        let params = CustomParams {
            world_pos: [4.0, 0.0, -7.0],
            target_pos: None,
            texture: None,
            tint: None,
        };
        let ring = GroundRing::new(&params);
        let mut list = EffectDrawList::new();
        ring.collect_draws(&mut list, &ctx());
        assert_eq!(list.len(), 1);
        match &list.primitives[0] {
            EffectPrimitiveDraw::Billboard { pos, blend, .. } => {
                assert!((pos[0] - 4.0).abs() < 0.01);
                assert!((pos[2] - (-7.0)).abs() < 0.01);
                assert_eq!(*blend, BlendKind::Additive);
            }
            _ => panic!("GroundRing should emit a Billboard"),
        }
    }

    #[test]
    fn rotation_changes_uvs_over_time() {
        let mut ring = GroundRing::new(&CustomParams::default());
        let mut list_a = EffectDrawList::new();
        ring.collect_draws(&mut list_a, &ctx());
        ring.update(&EffectUpdateCtx { dt: 1.0 });
        let mut list_b = EffectDrawList::new();
        ring.collect_draws(&mut list_b, &ctx());
        let uv_a = match &list_a.primitives[0] {
            EffectPrimitiveDraw::Billboard { uv, .. } => *uv,
            _ => unreachable!(),
        };
        let uv_b = match &list_b.primitives[0] {
            EffectPrimitiveDraw::Billboard { uv, .. } => *uv,
            _ => unreachable!(),
        };
        assert_ne!(uv_a, uv_b);
    }
}
