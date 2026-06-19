//! `EF_DAMAGE1` (652) / `EF_DAMAGE1_2` (653) / `EF_DAMAGE1_3` (654) — floating
//! recoloured "1" numbers.
//!
//! These are **not** primitives: the original game spawns a recoloured "1"
//! on the actor — the recovery/regen rising-number animation
//! recoloured. It rises the same way the HP-recovery number does,
//! so these reuse the damage-number path's rising-number curve.
//!
//!
//! * **Damage1** (652) — red `(255,0,0)` (also plays `hit2.wav`; sound is out of
//!   scope).
//! * **Damage12** (653) — red `(255,0,0)`.
//! * **Damage13** (654) — purple `(255,100,255)`.
//!
//! No primitive and no body channel: the effect emits a one-shot
//! [`NumberRequest`] on its first frame, then idles until the holder despawns
//! it. The holder drains the request keyed by the attached entity and the
//! client spawns the floating number on that actor.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx, NumberRequest};

const FPS: f32 = 60.0;
/// Matches the effect's duration table entry (500 ms); the number itself
/// outlives this via the damage-number manager's own lifetime.
const DURATION_FRAMES: f32 = 0.5 * FPS;

#[derive(Clone, Copy)]
pub struct NumberParams {
    pub color: [f32; 3],
}

/// 652 / 653 — red `(255,0,0)`.
pub const DAMAGE1: NumberParams = NumberParams { color: [1.0, 0.0, 0.0] };
/// 654 — purple `(255,100,255)`.
pub const DAMAGE1_3: NumberParams = NumberParams { color: [1.0, 100.0 / 255.0, 1.0] };

/// The displayed number is always `1`.
const NUMBER_VALUE: i32 = 1;

pub struct DamageNumberEffect {
    color: [f32; 3],
    age_frames: f32,
    request_pending: bool,
}

impl DamageNumberEffect {
    pub fn new(params: NumberParams) -> Self {
        Self { color: params.color, age_frames: 0.0, request_pending: true }
    }
}

impl Effect for DamageNumberEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn take_number_request(&mut self) -> Option<NumberRequest> {
        if self.request_pending {
            self.request_pending = false;
            Some(NumberRequest { value: NUMBER_VALUE, color: self.color })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut DamageNumberEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: frames / FPS, camera_target: None, caster_yaw: None })
    }

    #[test]
    fn emits_one_number_request_then_none() {
        let mut e = DamageNumberEffect::new(DAMAGE1);
        let req = e.take_number_request().expect("first call yields a request");
        assert_eq!(req.value, 1);
        assert_eq!(req.color, [1.0, 0.0, 0.0]);
        assert!(e.take_number_request().is_none(), "request is one-shot");
    }

    #[test]
    fn purple_variant_carries_its_colour() {
        let mut e = DamageNumberEffect::new(DAMAGE1_3);
        let req = e.take_number_request().unwrap();
        assert_eq!(req.color, [1.0, 100.0 / 255.0, 1.0]);
    }

    #[test]
    fn dies_after_duration() {
        let mut e = DamageNumberEffect::new(DAMAGE1);
        assert_eq!(step(&mut e, 1.0), EffectStatus::Running);
        assert_eq!(step(&mut e, DURATION_FRAMES), EffectStatus::Dead);
    }
}
