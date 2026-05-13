use ragnarok_game::effect::{EffectBlend, GroundRingParams};

use crate::effect::custom_effect::{CustomEffect, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

pub struct GroundRing {
    world_pos: [f32; 3],
    params: GroundRingParams,
    age: f32,
}

impl GroundRing {
    pub fn new(world_pos: [f32; 3], params: &GroundRingParams) -> Self {
        Self {
            world_pos,
            params: *params,
            age: 0.0,
        }
    }

    fn fade_alpha(&self) -> f32 {
        let fade_in_s = self.params.fade_in_ms as f32 / 1000.0;
        if fade_in_s > 0.0 && self.age < fade_in_s {
            return (self.age / fade_in_s).clamp(0.0, 1.0);
        }
        1.0
    }
}

impl CustomEffect for GroundRing {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = self.fade_alpha();
        let rotation = (self.age * self.params.rotation_deg_per_sec).to_radians();
        let mut color = self.params.color;
        color[3] *= alpha;
        out.push(EffectPrimitiveDraw::Ring {
            center: self.world_pos,
            radius: self.params.radius,
            thickness: self.params.thickness,
            rotation,
            texture: self.params.texture,
            color,
            blend: blend_from(self.params.blend),
        });
    }
}

fn blend_from(b: EffectBlend) -> BlendKind {
    match b {
        EffectBlend::Alpha => BlendKind::Alpha,
        EffectBlend::Additive => BlendKind::Additive,
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
    fn reads_params_into_emitted_ring() {
        let params = GroundRingParams {
            texture: "magic_target.tga",
            radius: 18.0,
            thickness: 18.0,
            rotation_deg_per_sec: 90.0,
            color: [0.5, 0.6, 0.7, 0.8],
            blend: EffectBlend::Additive,
            fade_in_ms: 0,
            fade_out_ms: 0,
        };
        let mut ring = GroundRing::new([4.0, 0.0, -7.0], &params);
        // Advance 1s so rotation is non-zero (params.rotation_deg_per_sec * 1s = 90deg).
        ring.update(&EffectUpdateCtx { dt: 1.0 });
        let mut list = EffectDrawList::new();
        ring.collect_draws(&mut list, &ctx());
        assert_eq!(list.len(), 1);
        match &list.primitives[0] {
            EffectPrimitiveDraw::Ring {
                center,
                radius,
                texture,
                rotation,
                color,
                blend,
                ..
            } => {
                assert!((center[0] - 4.0).abs() < 0.01);
                assert!((center[2] - (-7.0)).abs() < 0.01);
                assert!((radius - 18.0).abs() < 0.01);
                assert_eq!(*texture, "magic_target.tga");
                let half_pi = std::f32::consts::FRAC_PI_2;
                assert!((rotation - half_pi).abs() < 0.01);
                assert!((color[0] - 0.5).abs() < 0.01);
                assert_eq!(*blend, BlendKind::Additive);
            }
            _ => panic!("GroundRing should emit a Ring"),
        }
    }

    #[test]
    fn fade_in_ramps_alpha_from_zero() {
        let params = GroundRingParams {
            fade_in_ms: 200,
            color: [1.0, 1.0, 1.0, 1.0],
            ..GroundRingParams::DEFAULT
        };
        let mut ring = GroundRing::new([0.0; 3], &params);
        ring.update(&EffectUpdateCtx { dt: 0.1 });
        let mut list = EffectDrawList::new();
        ring.collect_draws(&mut list, &ctx());
        let alpha = match &list.primitives[0] {
            EffectPrimitiveDraw::Ring { color, .. } => color[3],
            _ => unreachable!(),
        };
        // After 100ms of a 200ms fade-in, alpha should be ~0.5.
        assert!((alpha - 0.5).abs() < 0.05);
    }
}
