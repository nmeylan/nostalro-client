use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const PANEL_COUNT: usize = 4;
const PANEL_WIDTH: f32 = 6.0;
const PANEL_HEIGHT: f32 = 5.0;
const SCROLL_HZ: f32 = 1.5;
const BASE_COLOR: [f32; 4] = [0.7, 0.9, 1.0, 0.6];

pub struct Waterfall {
    world_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl Waterfall {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            world_pos: params.world_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for Waterfall {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let scroll = (self.age * SCROLL_HZ).fract();
        for i in 0..PANEL_COUNT {
            let v_offset = scroll + i as f32 / PANEL_COUNT as f32;
            let y = self.world_pos[1] + PANEL_HEIGHT * 0.5 + i as f32 * PANEL_HEIGHT * 0.3;
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [self.world_pos[0], y, self.world_pos[2]],
                size: [PANEL_WIDTH, PANEL_HEIGHT],
                uv: [
                    [0.0, v_offset],
                    [1.0, v_offset],
                    [0.0, v_offset + 1.0],
                    [1.0, v_offset + 1.0],
                ],
                texture: self.texture,
                color: [
                    BASE_COLOR[0] * self.tint[0],
                    BASE_COLOR[1] * self.tint[1],
                    BASE_COLOR[2] * self.tint[2],
                    BASE_COLOR[3] * self.tint[3],
                ],
                blend: BlendKind::Alpha,
            });
        }
    }
}
