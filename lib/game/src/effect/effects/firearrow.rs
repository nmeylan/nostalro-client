//! `EF_FIREARROW` — Archer Fire Arrow (id 31).
//!
//! Original game's FireArrow emits one cross-texture — two
//! perpendicular textured quads — and cycles through a 6-frame flame
//! animation. No STR file ships in the classic GRF for this id, so we
//! render the cross via two camera-facing `Billboard`s, animating the UV
//! to walk through the 6 texture frames packed horizontally.
//!
//! Lifetime: ~1600 ms (96 frames at 60 fps). One-shot.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const FLAME_TEXTURE: &str = "firearrow.tga";
pub const TEXTURES: &[&str] = &[FLAME_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 96.0;
const FLAME_FRAMES: u32 = 6;
const FRAME_TICKS: f32 = DURATION_FRAMES / FLAME_FRAMES as f32;

pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const SIZE: f32 = 2.0;

pub struct FireArrowEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl FireArrowEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age_frames: 0.0 }
    }
}

impl Effect for FireArrowEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let t = (self.age_frames / DURATION_FRAMES).clamp(0.0, 1.0);
        let alpha = if t < 0.85 { 1.0 } else { 1.0 - (t - 0.85) / 0.15 };
        let flame_idx = ((self.age_frames / FRAME_TICKS) as u32).min(FLAME_FRAMES - 1);
        let u0 = (flame_idx as f32) / (FLAME_FRAMES as f32);
        let u1 = u0 + 1.0 / (FLAME_FRAMES as f32);
        let uv = [[u0, 0.0], [u1, 0.0], [u1, 1.0], [u0, 1.0]];
        let pos = [self.world_pos[0], self.world_pos[1] - 1.5, self.world_pos[2]];

        for rotation in [0.0_f32, std::f32::consts::FRAC_PI_2] {
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [SIZE, SIZE],
                uv,
                rotation,
                texture: FLAME_TEXTURE,
                color: [1.0, 1.0, 1.0, alpha.max(0.0)],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn cross_advances_through_six_frames() {
        let mut e = FireArrowEffect::new([0.0; 3]);
        e.update(&ctx(0.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let bbs: Vec<_> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Billboard { uv, .. } => Some(uv[0][0]),
                _ => None,
            })
            .collect();
        assert_eq!(bbs.len(), 2);
        // Mid-life: frame index advances → u0 > 0.
        e.update(&ctx(DURATION_FRAMES / FRAMES_PER_SECOND * 0.5));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let later_u0 = match &list.primitives[0] {
            EffectPrimitiveDraw::Billboard { uv, .. } => uv[0][0],
            _ => unreachable!(),
        };
        assert!(later_u0 > bbs[0]);
    }
}
