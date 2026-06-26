//! `EF_JUMPBODY` (445) / `EF_LANDBODY` (446) — vertical "to the moon" jump.
//!
//! High-jump / land-jump body lights: translate the actor sprite along the Y
//! axis with an alpha fade — no primitive is drawn.
//!
//! * **Jumpbody** rises and vanishes from frame 35: `lift = BodyTime·20`,
//!   `alpha = 255 − BodyTime·20` (`BodyTime = age − 35`).
//! * **Landbody** drops in from above and fades in as it lands:
//!   `lift = (25 − BodyTime2)·20` (high → 0), `alpha = (BodyTime2 − 10)·17`
//!   (`BodyTime2 = age + 11`), plus a spin that rights the
//!   body as it touches down.
//!
//! In the original the lift scales with perspective distance; our anchor
//! is screen-space, so a fixed [`LIFT_SCALE`] converts lift units → pixels.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{BodyVertical, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;

/// Stand-in for the original's perspective-distance factor — how many
/// screen pixels one unit of lift becomes. Controls how high the jump
/// throws the sprite: it should shoot well up off-screen ("to the moon") before
/// it fades, and Landbody should drop in from high above. Tune on a real actor.
const LIFT_SCALE: f32 = 3.5;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// High jump — rise & vanish.
    High,
    /// Land jump (+ spin) — drop in, fade in, spin upright.
    Land,
}

pub struct JumpBodyEffect {
    kind: Kind,
    age_frames: f32,
}

impl JumpBodyEffect {
    /// `Jumpbody` (445).
    pub fn high() -> Self {
        Self {
            kind: Kind::High,
            age_frames: 0.0,
        }
    }

    /// `Landbody` (446).
    pub fn land() -> Self {
        Self {
            kind: Kind::Land,
            age_frames: 0.0,
        }
    }

    /// `BodyTime` for the high jump (`age − 35`); negative before the
    /// rise begins.
    fn high_body_time(&self) -> f32 {
        self.age_frames - 35.0
    }

    /// `BodyTime2` for the land jump (`age + 11`).
    fn land_body_time(&self) -> f32 {
        self.age_frames + 11.0
    }
}

impl Effect for JumpBodyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        let done = match self.kind {
            // Rise complete once it has fully faded (alpha 0 at BodyTime 12.75).
            Kind::High => self.high_body_time() >= 13.0,
            // Landed (add 0 at BodyTime2 25) and spun upright (BodyTime2 36).
            Kind::Land => self.land_body_time() >= 36.0,
        };
        if done {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_vertical(&self) -> Option<BodyVertical> {
        match self.kind {
            Kind::High => {
                let t = self.high_body_time();
                if t < 0.0 {
                    // Before frame 35 the actor stands normally.
                    return None;
                }
                Some(BodyVertical {
                    lift_px: (t * 20.0).max(0.0) * LIFT_SCALE,
                    alpha: (1.0 - t * 20.0 / 255.0).clamp(0.0, 1.0),
                    squeeze: 1.0,
                })
            }
            Kind::Land => {
                let t = self.land_body_time();
                Some(BodyVertical {
                    lift_px: ((25.0 - t) * 20.0).max(0.0) * LIFT_SCALE,
                    alpha: ((t - 10.0) * 17.0 / 255.0).clamp(0.0, 1.0),
                    squeeze: 1.0,
                })
            }
        }
    }

    fn body_angle(&self) -> Option<f32> {
        // Only Landbody spins: a full roll that lands upright.
        // `angle = 180 − sin(BodyTime2·5 + 90°)·180`, wrapped to [0, 360).
        if self.kind != Kind::Land {
            return None;
        }
        let t = self.land_body_time();
        let sin = ((t * 5.0 + 90.0).to_radians()).sin();
        let mut deg = 180.0 - sin * 180.0;
        if deg >= 360.0 {
            deg -= 360.0;
        }
        Some(deg.to_radians())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut JumpBodyEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn jumpbody_rises_and_fades_only_after_frame_35() {
        let mut e = JumpBodyEffect::high();
        step(&mut e, 10.0);
        assert!(
            e.body_vertical().is_none(),
            "stands normally before frame 35"
        );
        step(&mut e, 30.0); // ~frame 40, into the rise
        let v = e.body_vertical().expect("rising");
        assert!(v.lift_px > 0.0, "lifts off the ground");
        assert!(v.alpha < 1.0, "fading out");
        step(&mut e, 20.0); // past the fade
        assert_eq!(
            e.update(&EffectUpdateCtx {
                delta: 0.0,
                camera_target: None,
                caster_yaw: None
            }),
            EffectStatus::Dead
        );
    }

    #[test]
    fn landbody_drops_in_fades_in_and_spins() {
        let mut e = JumpBodyEffect::land();
        let v0 = e.body_vertical().expect("starts high");
        assert!(
            v0.lift_px > 0.0 && v0.alpha < 0.5,
            "high and faint at first"
        );
        assert!(e.body_angle().is_some(), "Landbody spins");
        step(&mut e, 15.0); // lands (~frame 15)
        let v1 = e.body_vertical().expect("near ground");
        assert!(v1.lift_px < v0.lift_px, "descends");
        assert!(v1.alpha > v0.alpha, "fades in");
    }
}
