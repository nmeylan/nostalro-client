//! `EF_PRESSEDBODY` (413) / `EF_KICKEDBODY` (415) — vertical body deforms of
//! the target sprite.
//!
//! * **Pressedbody** — the body is squashed flat toward the feet.
//!   Active frame 17..=50 (`BodyTime = frame − 17`): `size` eases
//!   `1.0 → 0.4` over `BodyTime` 0..=3 then holds `≈0.4`; the top is pushed
//!   down by `(btm−top)·(1−size)`, i.e. a vertical scale of `size` about the
//!   feet — our [`BodyVertical::squeeze`]. `takingoff.wav` at frame 5.
//! * **Kickedbody** — the body is punted up then settles. Active
//!   `frame ≥ 30` (`BodyTime = frame − 30`): `size = sin(BodyTime·5°)·0.3`
//!   (peaks `BodyTime 18`), holds to 20, falls to 0 by 23; both top
//!   and bottom shift up by `(btm−top)·size`, i.e. a pure lift. `EF_hit2.wav` at 17,
//!   `EF_hit4.wav` at 25.
//!
//! No primitive — the deform is applied to the actor sprite by the shared
//! composer via [`Effect::body_vertical`].

use crate::effect::draw::{EffectDrawList, EffectStatus};
use crate::effect::effect_trait::{BodyVertical, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
/// Body height in screen pixels that one unit of `size` lifts Kickedbody —
/// the original scales by the on-screen `(btm−top)`; a fixed value approximates
/// it (peak lift `0.3·KICK_LIFT_PX`). Tune on a real actor.
const KICK_LIFT_PX: f32 = 80.0;

#[derive(Clone, Copy)]
enum Kind {
    Pressed,
    Kicked,
}

#[derive(Clone, Copy)]
struct Config {
    kind: Kind,
    /// First active frame.
    start_frame: f32,
    total_frames: f32,
    sfx: &'static [(f32, &'static str)],
}

const PRESSED: Config = Config {
    kind: Kind::Pressed,
    start_frame: 17.0,
    total_frames: 50.0,
    sfx: &[(5.0, "effect\\takingoff.wav")],
};

const KICKED: Config = Config {
    kind: Kind::Kicked,
    start_frame: 30.0,
    total_frames: 54.0,
    sfx: &[(17.0, "effect\\EF_hit2.wav"), (25.0, "effect\\EF_hit4.wav")],
};

pub const TEXTURES: &[&str] = &[];

pub fn pressed_total_duration_ms() -> u32 {
    (PRESSED.total_frames / FPS * 1000.0) as u32
}
pub fn kicked_total_duration_ms() -> u32 {
    (KICKED.total_frames / FPS * 1000.0) as u32
}

pub struct SquareBodyEffect {
    cfg: Config,
    process: f32,
    next_sfx: usize,
    pending_sfx: Option<&'static str>,
}

impl SquareBodyEffect {
    pub fn pressed() -> Self {
        Self::new(PRESSED)
    }
    pub fn kicked() -> Self {
        Self::new(KICKED)
    }

    fn new(cfg: Config) -> Self {
        Self { cfg, process: 0.0, next_sfx: 0, pending_sfx: None }
    }

    /// Deform-animation time (negative before the deform starts).
    fn body_time(&self) -> f32 {
        self.process - self.cfg.start_frame
    }
}

impl Effect for SquareBodyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.process += ctx.delta * FPS;
        while self.next_sfx < self.cfg.sfx.len() && self.process >= self.cfg.sfx[self.next_sfx].0 {
            self.pending_sfx = Some(self.cfg.sfx[self.next_sfx].1);
            self.next_sfx += 1;
        }
        if self.process >= self.cfg.total_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_vertical(&self) -> Option<BodyVertical> {
        let t = self.body_time();
        if t < 0.0 {
            return None;
        }
        match self.cfg.kind {
            Kind::Pressed => {
                let size = if t <= 3.0 {
                    (100.0 - t * 20.0) * 0.01
                } else if t <= 33.0 {
                    0.4
                } else {
                    return None;
                };
                Some(BodyVertical { lift_px: 0.0, alpha: 1.0, squeeze: size })
            }
            Kind::Kicked => {
                let size = if t <= 18.0 {
                    (t * 5.0).to_radians().sin() * 0.3
                } else if t <= 20.0 {
                    0.3
                } else if t <= 23.0 {
                    (23.0 - t) * 0.1
                } else {
                    return None;
                };
                Some(BodyVertical { lift_px: size * KICK_LIFT_PX, alpha: 1.0, squeeze: 1.0 })
            }
        }
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        self.pending_sfx.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut SquareBodyEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: frames / FPS, camera_target: None, caster_yaw: None })
    }

    #[test]
    fn pressedbody_squeezes_toward_the_feet_inside_its_window() {
        let mut e = SquareBodyEffect::pressed();
        assert!(e.body_vertical().is_none(), "stands normally before frame 17");
        step(&mut e, 30.0); // BodyTime ~13 → held squash
        let v = e.body_vertical().expect("squashing");
        assert!(v.squeeze < 1.0 && v.lift_px == 0.0, "vertical squeeze, no lift");
        assert_eq!(step(&mut e, 25.0), EffectStatus::Dead);
    }

    #[test]
    fn kickedbody_lifts_then_settles_with_two_hit_sounds() {
        let mut e = SquareBodyEffect::kicked();
        step(&mut e, 26.0); // past both sound frames (17, 25), before the lift (30)
        assert_eq!(e.take_sfx_request(), Some("effect\\EF_hit4.wav"), "latest queued sound");
        assert!(e.body_vertical().is_none(), "no lift before frame 30");
        step(&mut e, 18.0); // BodyTime ~14, rising
        let v = e.body_vertical().expect("airborne");
        assert!(v.lift_px > 0.0 && v.squeeze == 1.0, "pure lift, no squeeze");
    }
}
