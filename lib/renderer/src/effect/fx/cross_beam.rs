use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const BEAM_LONG: f32 = 28.0;
const BEAM_SHORT: f32 = 3.0;
const BEAM_COLOR: [f32; 4] = [1.0, 0.95, 0.7, 0.85];

pub struct CrossBeam {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl CrossBeam {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for CrossBeam {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let color = [
            BEAM_COLOR[0] * self.tint[0],
            BEAM_COLOR[1] * self.tint[1],
            BEAM_COLOR[2] * self.tint[2],
            BEAM_COLOR[3] * self.tint[3],
        ];
        out.push(EffectPrimitiveDraw::Billboard {
            pos: [
                self.world_pos[0],
                self.world_pos[1] + BEAM_LONG * 0.5,
                self.world_pos[2],
            ],
            size: [BEAM_SHORT, BEAM_LONG],
            uv,
            texture: self.texture,
            color,
            blend: BlendKind::Additive,
        });
        out.push(EffectPrimitiveDraw::Billboard {
            pos: [
                self.world_pos[0],
                self.world_pos[1] + BEAM_LONG * 0.5,
                self.world_pos[2],
            ],
            size: [BEAM_LONG, BEAM_SHORT],
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
    fn emits_a_vertical_and_horizontal_beam() {
        let cross = CrossBeam::new(&CustomParams::default());
        let mut list = EffectDrawList::new();
        cross.collect_draws(&mut list, &ctx());
        assert_eq!(list.len(), 2);
        let sizes: Vec<[f32; 2]> = list
            .primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { size, .. } => *size,
                _ => unreachable!(),
            })
            .collect();
        assert!(sizes[0][1] > sizes[0][0], "first beam is vertical");
        assert!(sizes[1][0] > sizes[1][1], "second beam is horizontal");
    }
}
