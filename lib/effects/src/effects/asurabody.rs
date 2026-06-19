//! `EF_ASURABODY` (424) — Asura Strike body bloom.
//!
//! 10 expanding **additive white** after-images blooming
//! off the body, each copy `i` larger (grows by `i·3`, distance-scaled)
//! and its alpha pumped by a body timer — building while the timer ≤ 130
//! (`alpha = time − 50 − i·5`), then draining (`alpha = 145 − time +
//! (10 − i)·10`), clamped 0..=200. A glowing halo, not a flat tint. No
//! primitive — the copies are emitted via [`Effect::body_copies`] and drawn by
//! the shared composer.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{BodyCopy, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const COPIES: usize = 10;
/// Per-layer growth — the original `i·3` (source units) condensed to a
/// fraction of body size per copy.
const GROWTH_PER_LAYER: f32 = 0.05;
/// The bloom has fully drained by body timer 145.
const END_FRAME: f32 = 145.0;

pub const TEXTURES: &[&str] = &[];

#[derive(Default)]
pub struct AsuraBodyEffect {
    age_frames: f32,
}

impl AsuraBodyEffect {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for AsuraBodyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= END_FRAME {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        let bt = self.age_frames;
        let mut copies = Vec::with_capacity(COPIES);
        for i in 1..=COPIES {
            let i_f = i as f32;
            let alpha_src = if bt <= 130.0 {
                bt - 50.0 - i_f * 5.0
            } else {
                145.0 - bt + (10.0 - i_f) * 10.0
            };
            let alpha = alpha_src.clamp(0.0, 200.0) / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            let scale = 1.0 + i_f * GROWTH_PER_LAYER;
            copies.push(BodyCopy {
                // Composer scales copies about the body centre, so no manual
                // recentre is needed — the bloom is concentric on all sides.
                offset_px: [0.0, 0.0],
                margin_px: 0.0,
                scale: [scale, scale],
                tint: [255, 255, 255],
                alpha,
                additive: true,
                behind: false,
            });
        }
        (!copies.is_empty()).then_some(copies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut AsuraBodyEffect, frames: f32) {
        e.update(&EffectUpdateCtx { delta: frames / FPS, camera_target: None, caster_yaw: None });
    }

    #[test]
    fn blooms_additive_white_copies_then_dies() {
        let mut e = AsuraBodyEffect::new();
        assert!(e.body_copies().is_none(), "no halo before BodyTime 50");
        step(&mut e, 100.0);
        let copies = e.body_copies().expect("blooming");
        assert!(copies.iter().all(|c| c.additive && c.tint == [255, 255, 255]));
        assert!(copies.iter().all(|c| c.scale[0] > 1.0), "copies expand");
        step(&mut e, 60.0);
        assert_eq!(
            e.update(&EffectUpdateCtx { delta: 0.0, camera_target: None, caster_yaw: None }),
            EffectStatus::Dead
        );
    }
}
