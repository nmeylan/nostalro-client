//! Wink (`misc\wink.spr`) and Fvoice (`misc\fvoice.spr`): a wink/voice emote
//! that flies off as a four-clip zoomed trail. Both share the exact same
//! handler; only the sprite differs.
//!
//! Each `.act` holds four actions, one per diagonal fly-off direction
//! (↙ ↖ ↗ ↘). The original game picks the action from the caster→target angle
//! plus the camera longitude so the trail always shoots the right way on
//! screen, then plays the six motions of that action once (anim speed 6,
//! no anim repeat, size = 1.0, uses original vertex color). Cast on self there
//! is no target, so the angle reduces to the camera longitude — the emote keeps
//! a consistent screen direction as the camera orbits.
//!
//! Implemented as a `Custom` effect (rather than a `spr_def` one-shot) because
//! the action must be chosen from the live camera, which only `collect_draws`
//! sees. Each frame it emits one `SpriteParticle` with the chosen
//! `action_index` and the animated `motion_index`.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const ANIM_SPEED: f32 = 6.0;
/// `wink.act` has six motions per action; one-shot, so the index clamps to the
/// last and holds it for the rest of the lifetime.
const MOTION_COUNT: usize = 6;
/// Matches `default_duration_ms(EffectId::Wink)` — 100 frames at 60 fps. The
/// six-motion animation finishes well before this; the
/// held final motion lingers until it elapses.
const TOTAL_DURATION_S: f32 = 1667.0 / 1000.0;

/// Wink emote (`misc\wink.spr`).
pub const WINK: &str = "data/sprite/이팩트/wink";
/// Voice emote (`misc\fvoice.spr`), same handler as Wink.
pub const FVOICE: &str = "data/sprite/이팩트/fvoice";

pub const SPRITES: &[&str] = &[WINK, FVOICE];

pub struct WinkEffect {
    anchor: [f32; 3],
    sprite: &'static str,
    age: f32,
}

impl WinkEffect {
    pub fn new(anchor: [f32; 3], sprite: &'static str) -> Self {
        Self { anchor, sprite, age: 0.0 }
    }

    fn motion_index(&self) -> usize {
        let raw = (self.age * FRAMES_PER_SECOND / ANIM_SPEED) as usize;
        raw.min(MOTION_COUNT - 1)
    }
}

/// Camera azimuth in degrees `[0, 360)` — the direction from the look target
/// back to the eye around the world Y axis.
fn camera_longitude_deg(camera: &CameraView) -> f32 {
    let dx = camera.eye[0] - camera.target[0];
    let dz = camera.eye[2] - camera.target[2];
    dx.atan2(dz).to_degrees().rem_euclid(360.0)
}

/// Pick the wink's fly-off action from the camera-relative angle, matching the
/// original game's quadrant switch.
fn action_for_angle(angle_deg: f32) -> usize {
    let a = angle_deg.rem_euclid(360.0);
    if (180.0..270.0).contains(&a) {
        0
    } else if (90.0..180.0).contains(&a) {
        1
    } else if a < 90.0 {
        2
    } else {
        3
    }
}

impl Effect for WinkEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= TOTAL_DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        out.push(EffectPrimitiveDraw::SpriteParticle {
            sprite_path: self.sprite,
            position: self.anchor,
            action_index: action_for_angle(camera_longitude_deg(&ctx.camera)),
            motion_index: self.motion_index(),
            size_scale: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            blend: BlendKind::Alpha,
            aim_target: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam_at(deg: f32) -> CameraView {
        let r = deg.to_radians();
        // Place the eye on a circle around the origin target at azimuth `deg`.
        CameraView {
            eye: [r.sin() * 10.0, 5.0, r.cos() * 10.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, -1.0, 0.0],
        }
    }

    #[test]
    fn action_follows_camera_quadrant() {
        // Each 90° camera quadrant selects a different fly-off action, so the
        // wink keeps a consistent screen direction as the camera orbits.
        assert_eq!(action_for_angle(45.0), 2);
        assert_eq!(action_for_angle(135.0), 1);
        assert_eq!(action_for_angle(225.0), 0);
        assert_eq!(action_for_angle(315.0), 3);
        // Derived from a real camera azimuth and wrapped past 360°.
        assert_eq!(action_for_angle(camera_longitude_deg(&cam_at(45.0))), 2);
        assert_eq!(action_for_angle(720.0 + 200.0), 0);
    }

    #[test]
    fn motion_plays_once_then_holds_and_effect_dies() {
        let mut e = WinkEffect::new([0.0; 3], WINK);
        let m0 = e.motion_index();
        for _ in 0..6 {
            e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None });
        }
        let m_mid = e.motion_index();
        assert!(m_mid > m0, "motion advances: {m0} -> {m_mid}");

        let mut status = EffectStatus::Running;
        for _ in 0..200 {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None });
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead, "self-terminates");
        assert_eq!(e.motion_index(), MOTION_COUNT - 1, "holds the final motion");
    }
}
