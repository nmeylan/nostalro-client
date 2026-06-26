//! `EF_DETECTING` — Improve Concentration cast ring on the ground (id 119).
//!
//! A single horizontal `fashasha.tga` quad on the ground that expands from
//! half-size 1.5 outward over ~57 frames; alpha holds at 1.0 until frame 40,
//! then fades linearly to 0 at frame 57. Additive blend.
//!

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const DETECTING_TEXTURE: &str = "fashasha.tga";
pub const TEXTURES: &[&str] = &[DETECTING_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: f32 = 57.0;
const FADE_START_FRAME: f32 = 40.0;
const START_HALF_SIZE: f32 = 3.0;
const END_HALF_SIZE: f32 = 15.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub struct DetectingEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl DetectingEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }
}

impl Effect for DetectingEffect {
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
        let t = (frame / TOTAL_FRAMES).clamp(0.0, 1.0);
        let size = START_HALF_SIZE + (END_HALF_SIZE - START_HALF_SIZE) * t;
        let alpha = if frame < FADE_START_FRAME {
            1.0
        } else {
            (1.0 - (frame - FADE_START_FRAME) / (TOTAL_FRAMES - FADE_START_FRAME)).clamp(0.0, 1.0)
        };
        out.push(EffectPrimitiveDraw::Texture3D {
            center: self.world_pos,
            size: [size, size],
            plane: QuadPlane::Horizontal,
            uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            texture: DETECTING_TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Additive,
        });
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

    fn step_n(e: &mut DetectingEffect, n: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..n {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
            if s == EffectStatus::Dead {
                break;
            }
        }
        s
    }

    fn first_quad(e: &DetectingEffect) -> ([f32; 2], f32) {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        match &list.primitives[0] {
            EffectPrimitiveDraw::Texture3D { size, color, .. } => (*size, color[3]),
            _ => panic!("expected Texture3D"),
        }
    }

    #[test]
    fn ring_expands_then_fades_then_dies() {
        let mut e = DetectingEffect::new([0.0, 0.0, 0.0]);

        let (s0, a0) = first_quad(&e);
        assert!((s0[0] - START_HALF_SIZE).abs() < 1e-3);
        assert!((a0 - 1.0).abs() < 1e-3);

        step_n(&mut e, 20);
        let (s1, a1) = first_quad(&e);
        assert!(s1[0] > s0[0], "size grew {} -> {}", s0[0], s1[0]);
        assert!((a1 - 1.0).abs() < 1e-3, "alpha still 1.0 before fade");

        step_n(&mut e, 25);
        let (_s2, a2) = first_quad(&e);
        assert!(a2 < 1.0 && a2 > 0.0, "alpha mid-fade: {a2}");

        let status = step_n(&mut e, 50);
        assert_eq!(status, EffectStatus::Dead);
    }
}
