//! `EF_TAEREADY` (449) — Taekwon ready-stance blue body flash.
//!
//! Overlays the body with `RGB(5,5,255)`, alpha ramping
//! in (`≤10` → `×15`), holding (`≤20` → 160), then fading (`≤50` →
//! `155 − (t−20)·5`), rendered **twice additively** for a subtle blue flash.
//! No primitive — emitted via [`Effect::body_copies`].

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{BodyCopy, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const END_FRAME: f32 = 50.0;

pub const TEXTURES: &[&str] = &[];

#[derive(Default)]
pub struct TaeReadyEffect {
    age_frames: f32,
}

impl TaeReadyEffect {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for TaeReadyEffect {
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
        let t = self.age_frames;
        let alpha_src = if t <= 10.0 {
            t * 15.0
        } else if t <= 20.0 {
            160.0
        } else {
            155.0 - (t - 20.0) * 5.0
        };
        let alpha = (alpha_src / 255.0).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return None;
        }
        let flash = BodyCopy {
            offset_px: [0.0, 0.0],
            margin_px: 0.0,
            scale: [1.0, 1.0],
            tint: [5, 5, 255],
            alpha,
            additive: true,
            behind: false,
            body_layers_only: false,
        };
        // Rendered twice for the additive blue overlay.
        Some(vec![flash, flash])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut TaeReadyEffect, frames: f32) {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        });
    }

    #[test]
    fn blue_flash_ramps_holds_fades_as_two_additive_copies() {
        let mut e = TaeReadyEffect::new();
        step(&mut e, 5.0); // into the ramp
        let early = e.body_copies().expect("ramping");
        assert_eq!(early.len(), 2, "two additive copies");
        assert!(early.iter().all(|c| c.additive && c.tint == [5, 5, 255]));
        step(&mut e, 10.0); // hold window (~frame 15)
        let hold = e.body_copies().unwrap()[0].alpha;
        step(&mut e, 20.0); // into fade
        let fade = e.body_copies().map(|c| c[0].alpha).unwrap_or(0.0);
        assert!(fade < hold, "alpha fades after the hold");
    }
}
