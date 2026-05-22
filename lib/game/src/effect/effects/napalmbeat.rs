//! `EF_NAPALMBEAT` — Wizard Napalm Beat (id 32).
//!
//! Original game's NapalmBeat emits a 2D-texture 8-frame
//! explosion animation in screen space. We approximate with a single
//! camera-facing `Billboard` cycling through the 8 frames.
//!
//! Lifetime: ~1000 ms (60 frames). One-shot.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "napalmbeat.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 60.0;
const FRAME_COUNT: u32 = 8;
const TICKS_PER_FRAME: f32 = DURATION_FRAMES / FRAME_COUNT as f32;

pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const SIZE: f32 = 4.0;

pub struct NapalmBeatEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl NapalmBeatEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age_frames: 0.0 }
    }
}

impl Effect for NapalmBeatEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame_idx = ((self.age_frames / TICKS_PER_FRAME) as u32).min(FRAME_COUNT - 1);
        let u0 = (frame_idx as f32) / (FRAME_COUNT as f32);
        let u1 = u0 + 1.0 / (FRAME_COUNT as f32);
        let uv = [[u0, 0.0], [u1, 0.0], [u1, 1.0], [u0, 1.0]];
        // Scale grows slightly then fades at the tail.
        let t = self.age_frames / DURATION_FRAMES;
        let scale = (1.0 + t * 0.3).min(1.4);
        let alpha = if t < 0.85 { 1.0 } else { 1.0 - (t - 0.85) / 0.15 };
        let pos = [self.world_pos[0], self.world_pos[1] - 2.0, self.world_pos[2]];
        out.push(EffectPrimitiveDraw::Billboard {
            pos,
            size: [SIZE * scale, SIZE * scale],
            uv,
            rotation: 0.0,
            texture: TEXTURE,
            color: [1.0, 1.0, 1.0, alpha.max(0.0)],
            blend: BlendKind::Additive,
        });
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
    fn cycles_through_eight_frames_then_dies() {
        let mut e = NapalmBeatEffect::new([0.0; 3]);
        e.update(&ctx(0.0));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let u0_a = match list.primitives[0] {
            EffectPrimitiveDraw::Billboard { uv, .. } => uv[0][0],
            _ => unreachable!(),
        };
        e.update(&ctx(DURATION_FRAMES / FRAMES_PER_SECOND * 0.6));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let u0_b = match list.primitives[0] {
            EffectPrimitiveDraw::Billboard { uv, .. } => uv[0][0],
            _ => unreachable!(),
        };
        assert!(u0_b > u0_a, "uv advances through frames");

        let mut status = EffectStatus::Running;
        for _ in 0..120 {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
