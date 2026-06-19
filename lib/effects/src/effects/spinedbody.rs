//! `EF_SPINEDBODY` (414) / `EF_SPINEDBODY2` (466) — full barrel-roll of the
//! target sprite.
//!
//! Both share the same render formula:
//! `angle = 180 − sin(BodyTime·5 + 90°)·180` over `BodyTime` 0..=36 (a full
//! turn). They differ only in timing:
//!
//! * **Spinedbody** (414) — immediate, **2× speed**: `BodyTime = frame·2`
//!   while `frame < 18`. No sound. A quick roll.
//! * **Spinedbody2** (466) — **delayed** to frame 14, normal speed:
//!   `BodyTime = frame − 14` while `14 ≤ frame < 50`, `kicking.wav` at
//!   frame 5. The slower twin.
//!
//! No primitive is drawn; the spin is applied to the actor sprite by the shared
//! composer via [`Effect::body_angle`].
//!
//! The original also pivots body/head ±40 px and accessories ±10 px around the
//! angle; a pure quad rotation about the anchor approximates it — add the pivot
//! only if the roll looks off-center on a real actor.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
/// The render formula sweeps a full turn over `BodyTime` 0..=36.
const ROLL_BODY_TIME: f32 = 36.0;

#[derive(Clone, Copy)]
struct SpinConfig {
    /// First frame the roll applies.
    start_frame: f32,
    /// Last frame (exclusive) the roll applies.
    end_frame: f32,
    /// `BodyTime` advance per frame (414 rolls at 2×, 466 at 1×).
    speed: f32,
    /// Plays `kicking.wav` at frame 5 (466 only).
    sfx: bool,
}

/// 414: `BodyTime = frame·2`, window `frame < 18`, no sound.
const SPINEDBODY: SpinConfig =
    SpinConfig { start_frame: 0.0, end_frame: 18.0, speed: 2.0, sfx: false };
/// 466: `BodyTime = frame − 14`, window `14 ≤ frame < 50`, `kicking.wav`.
const SPINEDBODY2: SpinConfig =
    SpinConfig { start_frame: 14.0, end_frame: 14.0 + ROLL_BODY_TIME, speed: 1.0, sfx: true };

pub const TEXTURES: &[&str] = &[];

pub struct SpinedBodyEffect {
    cfg: SpinConfig,
    age_frames: f32,
    sfx_pending: bool,
}

impl SpinedBodyEffect {
    /// `Spinedbody` (414) — immediate 2× roll.
    pub fn spinedbody() -> Self {
        Self { cfg: SPINEDBODY, age_frames: 0.0, sfx_pending: false }
    }

    /// `Spinedbody2` (466) — delayed 1× roll with `kicking.wav`.
    pub fn spinedbody2() -> Self {
        Self { cfg: SPINEDBODY2, age_frames: 0.0, sfx_pending: false }
    }

    /// Roll-animation time for the current frame.
    fn body_time(&self) -> f32 {
        (self.age_frames - self.cfg.start_frame) * self.cfg.speed
    }
}

impl Effect for SpinedBodyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = self.age_frames;
        self.age_frames += ctx.delta * FPS;
        if self.cfg.sfx && before < 5.0 && self.age_frames >= 5.0 {
            self.sfx_pending = true;
        }
        if self.age_frames >= self.cfg.end_frame {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_angle(&self) -> Option<f32> {
        if self.age_frames < self.cfg.start_frame || self.age_frames >= self.cfg.end_frame {
            return None;
        }
        let t = self.body_time();
        if !(0.0..=ROLL_BODY_TIME).contains(&t) {
            return None;
        }
        let sin = ((t * 5.0 + 90.0).to_radians()).sin();
        let mut deg = 180.0 - sin * 180.0;
        if deg >= 360.0 {
            deg -= 360.0;
        }
        Some(deg.to_radians())
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        if self.sfx_pending {
            self.sfx_pending = false;
            Some("effect\\kicking.wav")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut SpinedBodyEffect, frames: f32) {
        e.update(&EffectUpdateCtx { delta: frames / FPS, camera_target: None, caster_yaw: None });
    }

    #[test]
    fn spinedbody_rolls_immediately_at_double_speed() {
        let mut e = SpinedBodyEffect::spinedbody();
        // Active from frame 0 (no delay), no sound.
        assert!(e.body_angle().is_some(), "rolls from the first frame");
        step(&mut e, 9.0); // BodyTime ≈ 18 → ~half a turn (upside down)
        let a = e.body_angle().expect("mid-roll").to_degrees();
        assert!((a - 180.0).abs() < 30.0, "near a half-turn at frame 9, got {a}");
        step(&mut e, 10.0); // past frame 18
        assert_eq!(
            e.update(&EffectUpdateCtx { delta: 0.0, camera_target: None, caster_yaw: None }),
            EffectStatus::Dead
        );
    }

    #[test]
    fn spinedbody2_is_delayed_and_slower() {
        let mut e = SpinedBodyEffect::spinedbody2();
        step(&mut e, 10.0);
        assert!(e.body_angle().is_none(), "no roll before frame 14");
        step(&mut e, 10.0); // ~frame 20, mid-roll
        assert!(e.body_angle().is_some(), "rolling inside the window");
    }
}
