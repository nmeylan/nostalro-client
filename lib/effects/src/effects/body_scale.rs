//! Body-scale effects — Giantbody (422/423) and
//! Babybody (538/539/540). Both ease the actor's pixel ratio via
//! `sin(ease·4 + 90°)` over an ease position 0..=45; they differ only in the target
//! ratio and ramp direction:
//!
//! * **Giant** grows 1.0 → **1.5**: `sn = 1.5 − (sin + 1)·0.25`.
//! * **Baby** shrinks 1.0 → **0.5**: `sn = (sin + 1)·0.25 + 0.5` (clamped to
//!   0.5 past ease 45).
//!
//! Variants:
//! `Giantbody` / `Babybody` ramp in over ~45 frames; `Giantbody2` / `Babybody2`
//! pin the ease at 45 for the instant scale; `BabybodyBack` reverses (ease =
//! 45 − frame·3, i.e. 45→0 over 15 frames) to grow the actor back to normal.
//!
//! These attach to the actor and only scale its sprite — no primitive is drawn
//! (the effect-viewer shows nothing; validate on a real actor). The grow/shrink
//! buffs persist until removed; `BabybodyBack` self-terminates once it restores.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const RAMP_FRAMES: f32 = 45.0;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Grow to 1.5×.
    Giant,
    /// Shrink to 0.5×.
    Baby,
}

pub struct BodyScaleEffect {
    kind: Kind,
    /// The ease position, 0..=45.
    body_sin: f32,
    /// Per-frame ramp (signed); 0 holds the instant scale.
    ramp_per_frame: f32,
    /// `BabybodyBack` ends once it has restored (`body_sin` back to 0); the
    /// grow/shrink buffs persist (the holder despawns them via the table).
    transient: bool,
}

impl BodyScaleEffect {
    /// `Giantbody` (422) — eases 1.0 → 1.5 over ~45 frames.
    pub fn giant_ramped() -> Self {
        Self { kind: Kind::Giant, body_sin: 0.0, ramp_per_frame: 1.0, transient: false }
    }

    /// `Giantbody2` (423) — instant 1.5×.
    pub fn giant_instant() -> Self {
        Self { kind: Kind::Giant, body_sin: RAMP_FRAMES, ramp_per_frame: 0.0, transient: false }
    }

    /// `Babybody` (538) — eases 1.0 → 0.5 over ~45 frames.
    pub fn baby_ramped() -> Self {
        Self { kind: Kind::Baby, body_sin: 0.0, ramp_per_frame: 1.0, transient: false }
    }

    /// `Babybody2` (539) — instant 0.5×.
    pub fn baby_instant() -> Self {
        Self { kind: Kind::Baby, body_sin: RAMP_FRAMES, ramp_per_frame: 0.0, transient: false }
    }

    /// `BabybodyBack` (540) — reverses 0.5 → 1.0 over ~15 frames (`−3`/frame).
    pub fn baby_back() -> Self {
        Self { kind: Kind::Baby, body_sin: RAMP_FRAMES, ramp_per_frame: -3.0, transient: true }
    }

    fn ratio(&self) -> f32 {
        let sinangle = self.body_sin * 4.0 + 90.0;
        match self.kind {
            // sn = 1.5 - (sin(ease*4 + 90) + 1) * 0.25
            Kind::Giant => 1.5 - (sinangle.to_radians().sin() + 1.0) * 0.25,
            // sn = (sin + 1) * 0.25 + 0.5, pinned to 0.5 past ease 45.
            Kind::Baby => {
                let a = sinangle.clamp(90.0, 270.0);
                if a >= 270.0 {
                    0.5
                } else {
                    (a.to_radians().sin() + 1.0) * 0.25 + 0.5
                }
            }
        }
    }
}

impl Effect for BodyScaleEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.body_sin =
            (self.body_sin + ctx.delta * FPS * self.ramp_per_frame).clamp(0.0, RAMP_FRAMES);
        if self.transient && self.body_sin <= 0.0 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_scale(&self) -> Option<f32> {
        Some(self.ratio())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut BodyScaleEffect, frames: f32) {
        e.update(&EffectUpdateCtx { delta: frames / FPS, camera_target: None, caster_yaw: None });
    }

    #[test]
    fn giant_eases_from_one_to_one_and_a_half() {
        let mut e = BodyScaleEffect::giant_ramped();
        assert!((e.body_scale().unwrap() - 1.0).abs() < 1e-4, "starts at 1.0");
        step(&mut e, 22.0);
        let mid = e.body_scale().unwrap();
        assert!(mid > 1.0 && mid < 1.5, "eases through the middle, got {mid}");
        step(&mut e, 30.0); // past the ramp
        assert!((e.body_scale().unwrap() - 1.5).abs() < 1e-3, "reaches 1.5");
    }

    #[test]
    fn baby_shrinks_from_one_to_one_half_and_instant_is_half() {
        let mut e = BodyScaleEffect::baby_ramped();
        assert!((e.body_scale().unwrap() - 1.0).abs() < 1e-4, "starts at 1.0");
        step(&mut e, 22.0);
        let mid = e.body_scale().unwrap();
        assert!(mid < 1.0 && mid > 0.5, "eases through the middle, got {mid}");
        step(&mut e, 30.0);
        assert!((e.body_scale().unwrap() - 0.5).abs() < 1e-3, "reaches 0.5");

        let inst = BodyScaleEffect::baby_instant();
        assert!((inst.body_scale().unwrap() - 0.5).abs() < 1e-3, "instant baby is 0.5");
    }

    #[test]
    fn baby_back_restores_to_one_then_dies() {
        let mut e = BodyScaleEffect::baby_back();
        assert!((e.body_scale().unwrap() - 0.5).abs() < 1e-3, "starts shrunk");
        // ~15 frames at -3/frame returns the ease 45 → 0 (ratio 1.0), then ends.
        for _ in 0..20 {
            step(&mut e, 1.0);
        }
        assert!((e.body_scale().unwrap() - 1.0).abs() < 1e-3, "restored to normal");
        assert_eq!(
            e.update(&EffectUpdateCtx { delta: 0.0, camera_target: None, caster_yaw: None }),
            EffectStatus::Dead,
            "the restore is transient",
        );
    }
}
