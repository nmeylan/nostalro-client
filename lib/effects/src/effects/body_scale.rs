//! Body-scale effects — Giantbody (422/423) and Babybody (538/539/540).

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const RAMP_FRAMES: f32 = 45.0;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Giant,
    Baby,
}

pub struct BodyScaleEffect {
    kind: Kind,
    body_sin: f32,
    ramp_per_frame: f32,
    transient: bool,
}

impl BodyScaleEffect {
    pub fn giant_ramped() -> Self {
        Self {
            kind: Kind::Giant,
            body_sin: 0.0,
            ramp_per_frame: 1.0,
            transient: false,
        }
    }

    pub fn giant_instant() -> Self {
        Self {
            kind: Kind::Giant,
            body_sin: RAMP_FRAMES,
            ramp_per_frame: 0.0,
            transient: false,
        }
    }

    pub fn baby_ramped() -> Self {
        Self {
            kind: Kind::Baby,
            body_sin: 0.0,
            ramp_per_frame: 1.0,
            transient: false,
        }
    }

    pub fn baby_instant() -> Self {
        Self {
            kind: Kind::Baby,
            body_sin: RAMP_FRAMES,
            ramp_per_frame: 0.0,
            transient: false,
        }
    }

    pub fn baby_back() -> Self {
        Self {
            kind: Kind::Baby,
            body_sin: RAMP_FRAMES,
            ramp_per_frame: -3.0,
            transient: true,
        }
    }

    fn ratio(&self) -> f32 {
        let sinangle = self.body_sin * 4.0 + 90.0;
        match self.kind {
            Kind::Giant => 1.5 - (sinangle.to_radians().sin() + 1.0) * 0.25,
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
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        });
    }

    #[test]
    fn giant_eases_from_one_to_one_and_a_half() {
        let mut e = BodyScaleEffect::giant_ramped();
        assert!((e.body_scale().unwrap() - 1.0).abs() < 1e-4);
        step(&mut e, 22.0);
        let mid = e.body_scale().unwrap();
        assert!(mid > 1.0 && mid < 1.5, "got {mid}");
        step(&mut e, 30.0);
        assert!((e.body_scale().unwrap() - 1.5).abs() < 1e-3);
    }

    #[test]
    fn baby_shrinks_from_one_to_one_half_and_instant_is_half() {
        let mut e = BodyScaleEffect::baby_ramped();
        assert!((e.body_scale().unwrap() - 1.0).abs() < 1e-4);
        step(&mut e, 22.0);
        let mid = e.body_scale().unwrap();
        assert!(mid < 1.0 && mid > 0.5, "got {mid}");
        step(&mut e, 30.0);
        assert!((e.body_scale().unwrap() - 0.5).abs() < 1e-3);

        let inst = BodyScaleEffect::baby_instant();
        assert!((inst.body_scale().unwrap() - 0.5).abs() < 1e-3);
    }

    #[test]
    fn baby_back_restores_to_one_then_dies() {
        let mut e = BodyScaleEffect::baby_back();
        assert!((e.body_scale().unwrap() - 0.5).abs() < 1e-3);
        for _ in 0..20 {
            step(&mut e, 1.0);
        }
        assert!((e.body_scale().unwrap() - 1.0).abs() < 1e-3);
        assert_eq!(
            e.update(&EffectUpdateCtx {
                delta: 0.0,
                camera_target: None,
                caster_yaw: None
            }),
            EffectStatus::Dead,
        );
    }
}
