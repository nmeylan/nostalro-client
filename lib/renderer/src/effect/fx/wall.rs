use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const PILLAR_COUNT: usize = 5;
const PILLAR_WIDTH: f32 = 2.5;
const PILLAR_HEIGHT: f32 = 16.0;
const PILLAR_SPACING: f32 = 4.0;
const ICE_COLOR: [f32; 4] = [0.7, 0.85, 1.0, 0.75];

pub struct Wall {
    world_pos: [f32; 3],
    target_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl Wall {
    pub fn new(params: &CustomParams) -> Self {
        let target = params.target_pos.unwrap_or([
            params.world_pos[0],
            params.world_pos[1],
            params.world_pos[2] + PILLAR_SPACING * (PILLAR_COUNT as f32 - 1.0),
        ]);
        Self {
            world_pos: params.world_pos,
            target_pos: target,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for Wall {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        for i in 0..PILLAR_COUNT {
            let lerp = if PILLAR_COUNT > 1 {
                i as f32 / (PILLAR_COUNT as f32 - 1.0)
            } else {
                0.5
            };
            let x = self.world_pos[0] * (1.0 - lerp) + self.target_pos[0] * lerp;
            let z = self.world_pos[2] * (1.0 - lerp) + self.target_pos[2] * lerp;
            let color = [
                ICE_COLOR[0] * self.tint[0],
                ICE_COLOR[1] * self.tint[1],
                ICE_COLOR[2] * self.tint[2],
                ICE_COLOR[3] * self.tint[3],
            ];
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [x, self.world_pos[1] + PILLAR_HEIGHT * 0.5, z],
                size: [PILLAR_WIDTH, PILLAR_HEIGHT],
                uv,
                texture: self.texture,
                color,
                blend: BlendKind::Alpha,
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
    fn emits_pillars_along_line() {
        let wall = Wall::new(&CustomParams {
            world_pos: [0.0, 0.0, 0.0],
            target_pos: Some([16.0, 0.0, 0.0]),
            ..Default::default()
        });
        let mut list = EffectDrawList::new();
        wall.collect_draws(&mut list, &ctx());
        assert_eq!(list.len(), PILLAR_COUNT);
        let xs: Vec<f32> = list
            .primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, .. } => pos[0],
                _ => unreachable!(),
            })
            .collect();
        for w in xs.windows(2) {
            assert!(w[1] > w[0], "pillars must be ordered along the line");
        }
    }
}
