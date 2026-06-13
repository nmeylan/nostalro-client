//! `Giantbody` (422) / `Giantbody2` (423) — giant body enlarge.
//!
//! Original game: SetCharacterPixelRatio *= sn with
//! `sn = 1.5 - (sin(BodySin4·4 + 90°) + 1)·0.25`. `BodySin4` rises with the
//! state counter, clamped to 45, so `sn` eases from `1.0` (at `BodySin4 = 0`)
//! up to `1.5` (at `BodySin4 = 45`). `Giantbody` ramps in over those ~45
//! frames; `Giantbody2` pins `BodySin4 = 45` immediately for an instant 1.5×.
//!
//! It attaches to the actor and only enlarges its sprite — no primitive is
//! drawn (the effect-viewer shows nothing; validate on a real actor). The
//! enlarge persists until the buff is removed, so the effect stays alive and
//! the holder despawns it via the duration table.

use crate::effect::draw::{EffectDrawList, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const RAMP_FRAMES: f32 = 45.0;
const TARGET_RATIO: f32 = 1.5;

pub struct GiantBodyEffect {
    /// `BodySin4` — the ramp position, 0..=45.
    body_sin4: f32,
    ramp_per_frame: f32,
}

impl GiantBodyEffect {
    /// `Giantbody` (422) — eases from 1.0 to 1.5 over ~45 frames.
    pub fn ramped() -> Self {
        Self {
            body_sin4: 0.0,
            ramp_per_frame: 1.0,
        }
    }

    /// `Giantbody2` (423) — instant 1.5× (`BodySin4 = 45` from the first frame).
    pub fn instant() -> Self {
        Self {
            body_sin4: RAMP_FRAMES,
            ramp_per_frame: 0.0,
        }
    }

    fn ratio(&self) -> f32 {
        // sn = 1.5 - (sin(BodySin4*4 + 90) + 1) * 0.25
        let phase = (self.body_sin4 * 4.0 + 90.0).to_radians();
        TARGET_RATIO - (phase.sin() + 1.0) * 0.25
    }
}

impl Effect for GiantBodyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.body_sin4 = (self.body_sin4 + ctx.delta * FPS * self.ramp_per_frame).min(RAMP_FRAMES);
        // Persistent body enlarge — the holder despawns it via the duration
        // table when the buff ends.
        EffectStatus::Running
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_scale(&self) -> Option<f32> {
        Some(self.ratio())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut GiantBodyEffect, frames: f32) {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        });
    }

    #[test]
    fn ramped_eases_from_one_to_one_and_a_half() {
        let mut e = GiantBodyEffect::ramped();
        let start = e.body_scale().unwrap();
        assert!((start - 1.0).abs() < 1e-4, "starts at 1.0, got {start}");
        step(&mut e, 22.0);
        let mid = e.body_scale().unwrap();
        assert!(mid > 1.0 && mid < TARGET_RATIO, "eases through the middle, got {mid}");
        step(&mut e, 30.0); // past the 45-frame ramp
        let full = e.body_scale().unwrap();
        assert!((full - TARGET_RATIO).abs() < 1e-3, "reaches 1.5, got {full}");
    }

    #[test]
    fn instant_is_one_and_a_half_immediately_and_draws_nothing() {
        let mut e = GiantBodyEffect::instant();
        assert!((e.body_scale().unwrap() - TARGET_RATIO).abs() < 1e-3);
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 800.0,
                screen_h: 600.0,
                elapsed: 0.0,
            },
        );
        assert!(list.primitives.is_empty(), "body scale emits no primitives");
        assert_eq!(
            e.update(&EffectUpdateCtx { delta: 1.0, camera_target: None, caster_yaw: None }),
            EffectStatus::Running,
            "the enlarge persists",
        );
    }
}
