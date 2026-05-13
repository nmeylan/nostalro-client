use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const FLIGHT_DURATION: f32 = 0.6;
const ARC_HEIGHT: f32 = 12.0;
const TRAIL_COUNT: usize = 6;
const HEAD_SIZE: f32 = 2.4;
const TRAIL_FADE: f32 = 0.4;
const BASE_COLOR: [f32; 3] = [1.0, 0.85, 0.4];

pub struct SplineProjectile {
    world_pos: [f32; 3],
    target_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl SplineProjectile {
    pub fn new(params: &CustomParams) -> Self {
        let target = params.target_pos.unwrap_or([
            params.world_pos[0] + 10.0,
            params.world_pos[1],
            params.world_pos[2],
        ]);
        Self {
            world_pos: params.world_pos,
            target_pos: target,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }

    fn position_at(&self, t: f32) -> [f32; 3] {
        let one_minus = 1.0 - t;
        let ctrl = [
            (self.world_pos[0] + self.target_pos[0]) * 0.5,
            (self.world_pos[1] + self.target_pos[1]) * 0.5 + ARC_HEIGHT,
            (self.world_pos[2] + self.target_pos[2]) * 0.5,
        ];
        [
            one_minus * one_minus * self.world_pos[0]
                + 2.0 * one_minus * t * ctrl[0]
                + t * t * self.target_pos[0],
            one_minus * one_minus * self.world_pos[1]
                + 2.0 * one_minus * t * ctrl[1]
                + t * t * self.target_pos[1],
            one_minus * one_minus * self.world_pos[2]
                + 2.0 * one_minus * t * ctrl[2]
                + t * t * self.target_pos[2],
        ]
    }
}

impl CustomEffect for SplineProjectile {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let progress = (self.age / FLIGHT_DURATION).clamp(0.0, 1.0);
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        for i in 0..TRAIL_COUNT {
            let trail_t = (progress - (i as f32) * 0.05).max(0.0);
            let alpha_factor = 1.0 - (i as f32 / TRAIL_COUNT as f32) * (1.0 - TRAIL_FADE);
            let pos = self.position_at(trail_t);
            let color = [
                BASE_COLOR[0] * self.tint[0],
                BASE_COLOR[1] * self.tint[1],
                BASE_COLOR[2] * self.tint[2],
                alpha_factor * self.tint[3],
            ];
            let size = HEAD_SIZE * alpha_factor;
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [size, size],
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
    fn projectile_traces_arc_from_source_to_target() {
        let mut proj = SplineProjectile::new(&CustomParams {
            world_pos: [0.0, 0.0, 0.0],
            target_pos: Some([10.0, 0.0, 0.0]),
            ..Default::default()
        });
        proj.update(&EffectUpdateCtx {
            dt: FLIGHT_DURATION * 0.5,
        });
        let mut list = EffectDrawList::new();
        proj.collect_draws(&mut list, &ctx());
        let head_pos = match &list.primitives[0] {
            EffectPrimitiveDraw::Billboard { pos, .. } => *pos,
            _ => unreachable!(),
        };
        assert!(head_pos[0] > 0.0 && head_pos[0] < 10.0);
        assert!(head_pos[1] > 0.0, "midpoint should arc upward");
    }
}
