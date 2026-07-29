use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{BodyAction, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const KICK_ACTION: usize = 12;
const KICK_START_FRAME: usize = 5;
const ARM_FRAME: f32 = 5.0;
const REVERT_FRAME: f32 = 35.0;
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
        if before < ARM_FRAME && self.age_frames >= ARM_FRAME {
            self.action_pending = true;
            self.sfx_pending = true;
        }
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
            Some("effect\\t_날라차기.wav")
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
