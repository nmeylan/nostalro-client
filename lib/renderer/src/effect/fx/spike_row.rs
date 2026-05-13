use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const SHARD_COUNT: usize = 6;
const SHARD_STAGGER_SEC: f32 = 0.07;
const SHARD_RISE_SEC: f32 = 0.5;
const SHARD_FADE_SEC: f32 = 0.3;
const SHARD_PEAK_HEIGHT: f32 = 6.0;
const SHARD_WIDTH: f32 = 1.4;
const SHARD_HEIGHT: f32 = 5.0;
const SHARD_COLOR: [f32; 3] = [0.75, 0.65, 0.5];

pub struct SpikeRow {
    world_pos: [f32; 3],
    target_pos: [f32; 3],
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl SpikeRow {
    pub fn new(params: &CustomParams) -> Self {
        let target_pos = params.target_pos.unwrap_or(params.world_pos);
        Self {
            world_pos: params.world_pos,
            target_pos,
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for SpikeRow {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        for i in 0..SHARD_COUNT {
            let t_local = self.age - i as f32 * SHARD_STAGGER_SEC;
            if t_local < 0.0 || t_local > SHARD_RISE_SEC + SHARD_FADE_SEC {
                continue;
            }
            let lerp = (i as f32 + 0.5) / SHARD_COUNT as f32;
            let pos_x = self.world_pos[0] * (1.0 - lerp) + self.target_pos[0] * lerp;
            let pos_z = self.world_pos[2] * (1.0 - lerp) + self.target_pos[2] * lerp;
            let height = if t_local < SHARD_RISE_SEC {
                SHARD_PEAK_HEIGHT * (t_local / SHARD_RISE_SEC)
            } else {
                SHARD_PEAK_HEIGHT
            };
            let alpha = if t_local < SHARD_RISE_SEC {
                1.0
            } else {
                1.0 - (t_local - SHARD_RISE_SEC) / SHARD_FADE_SEC
            };
            let color = [
                SHARD_COLOR[0] * self.tint[0],
                SHARD_COLOR[1] * self.tint[1],
                SHARD_COLOR[2] * self.tint[2],
                alpha * self.tint[3],
            ];
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [pos_x, self.world_pos[1] + height * 0.5, pos_z],
                size: [SHARD_WIDTH, SHARD_HEIGHT],
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
    fn shards_appear_in_sequence() {
        let mut row = SpikeRow::new(&CustomParams {
            world_pos: [0.0, 0.0, 0.0],
            target_pos: Some([10.0, 0.0, 0.0]),
            ..Default::default()
        });
        let mut early = EffectDrawList::new();
        row.collect_draws(&mut early, &ctx());
        assert_eq!(early.len(), 1, "only the first shard at t=0");

        row.update(&EffectUpdateCtx {
            dt: SHARD_STAGGER_SEC * (SHARD_COUNT as f32 - 0.5),
        });
        let mut late = EffectDrawList::new();
        row.collect_draws(&mut late, &ctx());
        assert_eq!(late.len(), SHARD_COUNT, "all shards visible after stagger");
    }

    #[test]
    fn shards_lie_between_world_and_target() {
        let row = SpikeRow::new(&CustomParams {
            world_pos: [0.0, 0.0, 0.0],
            target_pos: Some([12.0, 0.0, 0.0]),
            ..Default::default()
        });
        let mut list = EffectDrawList::new();
        row.collect_draws(&mut list, &ctx());
        for prim in &list.primitives {
            let EffectPrimitiveDraw::Billboard { pos, .. } = prim else {
                unreachable!();
            };
            assert!(pos[0] >= 0.0 && pos[0] <= 12.0);
        }
    }
}
