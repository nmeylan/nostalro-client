//! `EF_YUFITELHIT` — Jupiter Thunder impact splash (id 94).
//!
//! Original game's Yufitelhit → repeated ground splashes via
//! 3D-texture with roll `90`. A new horizontal quad is spawned every
//! 20 frames; each lives 30 frames, expanding from half-size 1.5 to 4.5 and
//! fading alpha 1.0 → 0.0 linearly. Spawns alternate `thunder_pang.bmp` /
//! `pokjuk_d.bmp`. Total visible window covers 150 frames (~2500 ms).

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const THUNDER_PANG: &str = "thunder_pang.bmp";
pub const POKJUK_D: &str = "pokjuk_d.bmp";
pub const TEXTURES: &[&str] = &[THUNDER_PANG, POKJUK_D];

const FRAMES_PER_SECOND: f32 = 60.0;
const SPAWN_PERIOD_FRAMES: f32 = 20.0;
const SPAWN_COUNT: u32 = 6;
const QUAD_LIFE_FRAMES: f32 = 30.0;
const START_HALF: f32 = 1.5;
const END_HALF: f32 = 4.5;
const LAST_SPAWN_FRAME: f32 = (SPAWN_COUNT as f32 - 1.0) * SPAWN_PERIOD_FRAMES;
const TOTAL_FRAMES: f32 = LAST_SPAWN_FRAME + QUAD_LIFE_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub struct YufitelhitEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl YufitelhitEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }
}

impl Effect for YufitelhitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age * FRAMES_PER_SECOND >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        for i in 0..SPAWN_COUNT {
            let spawn_frame = i as f32 * SPAWN_PERIOD_FRAMES;
            let local = frame - spawn_frame;
            if local < 0.0 || local >= QUAD_LIFE_FRAMES {
                continue;
            }
            let t = local / QUAD_LIFE_FRAMES;
            let half = START_HALF + (END_HALF - START_HALF) * t;
            let alpha = (1.0 - t).clamp(0.0, 1.0);
            let texture = if i % 2 == 0 { THUNDER_PANG } else { POKJUK_D };
            out.push(EffectPrimitiveDraw::Texture3D {
                center: self.world_pos,
                size: [half, half],
                plane: QuadPlane::Horizontal,
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                texture,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_n(e: &mut YufitelhitEffect, n: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..n {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
            });
            if s == EffectStatus::Dead {
                break;
            }
        }
        s
    }

    fn count_draws(e: &YufitelhitEffect) -> usize {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives.len()
    }

    #[test]
    fn spawns_stagger_and_overlap_then_die() {
        let mut e = YufitelhitEffect::new([0.0, 0.0, 0.0]);
        assert_eq!(count_draws(&e), 1, "first quad live at frame 0");

        step_n(&mut e, 25);
        assert_eq!(count_draws(&e), 2, "second quad spawned by frame 25");

        step_n(&mut e, (LAST_SPAWN_FRAME as u32) - 25);
        assert!(count_draws(&e) >= 1);

        let status = step_n(&mut e, 200);
        assert_eq!(status, EffectStatus::Dead);
        assert_eq!(count_draws(&e), 0);
    }
}
