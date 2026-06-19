//! `EF_JUMPKICK` (457) — Taekwon flying-kick forced pose.
//!
//! Not a particle effect: in the original game it only forces
//! the caster's animation at frame 5 (snaps to the skill pose —
//! action group `96 / 8 = 12`, the Skill action,
//! held on frame 5), plays `flyingkick.wav`, and reverts to the normal attack
//! at frame 35. So this effect draws nothing; it arms a one-shot [`BodyAction`]
//! that the game-update step plays on the attached actor (the effect viewer
//! holds the actor static, so it snaps the pose in; validate the animated kick
//! on a real actor).

use crate::effect::draw::{EffectDrawList, EffectStatus};
use crate::effect::effect_trait::{BodyAction, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
/// Forced animation: action group `96 / 8 = 12` (the Skill pose).
const KICK_ACTION: usize = 12;
/// The held start frame.
const KICK_START_FRAME: usize = 5;
/// Frame 5 arms the pose; it reverts at frame 35.
const ARM_FRAME: f32 = 5.0;
const REVERT_FRAME: f32 = 35.0;
/// The pose holds from frame 5 to 35 (~0.5 s at 60 fps).
const KICK_DURATION_MS: f32 = (REVERT_FRAME - ARM_FRAME) / FPS * 1000.0;

pub const TEXTURES: &[&str] = &[];

#[derive(Default)]
pub struct JumpkickEffect {
    age_frames: f32,
    action_pending: bool,
    sfx_pending: bool,
}

impl JumpkickEffect {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for JumpkickEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = self.age_frames;
        self.age_frames += ctx.delta * FPS;
        // Cross frame 5 once → arm the forced pose + kick sound.
        if before < ARM_FRAME && self.age_frames >= ARM_FRAME {
            self.action_pending = true;
            self.sfx_pending = true;
        }
        // The holder despawns it at the table duration; keep running so the
        // pose is armed before then.
        EffectStatus::Running
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn take_body_action(&mut self) -> Option<BodyAction> {
        if self.action_pending {
            self.action_pending = false;
            Some(BodyAction {
                action_index: KICK_ACTION,
                start_frame: KICK_START_FRAME,
                duration_ms: KICK_DURATION_MS,
            })
        } else {
            None
        }
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        if self.sfx_pending {
            self.sfx_pending = false;
            Some("effect\\flyingkick.wav")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut JumpkickEffect, frames: f32) {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        });
    }

    #[test]
    fn arms_the_kick_pose_once_at_frame_five() {
        let mut e = JumpkickEffect::new();
        step(&mut e, 4.0);
        assert!(e.take_body_action().is_none(), "not armed before frame 5");
        step(&mut e, 2.0); // cross frame 5
        let action = e.take_body_action().expect("armed at frame 5");
        assert_eq!(action.action_index, 12, "plays the Skill action group");
        assert_eq!(action.start_frame, 5);
        assert!(e.take_body_action().is_none(), "one-shot — fires only once");
    }
}
