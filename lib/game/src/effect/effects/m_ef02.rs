//! `EF_M02` — a directional monster effect (`m_ef02.spr`,
//! "Full Buster"). `m_ef02.act` holds four actions, one per screen quadrant; the
//! original game picks the action from the caster→target angle plus the camera
//! longitude so the sprite always faces the right way on screen, then plays that
//! action's eight motions once (anim speed 4, no repeat).
//!
//! Like Wink it can't be a `spr_def` one-shot: the action is chosen from the
//! live camera, which only `collect_draws` sees. Attached to its caster there is
//! no separate target, so the angle reduces to the camera longitude (the
//! caster→target angle degenerates to 0) — the sprite keeps a
//! consistent screen direction as the camera orbits.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const ANIM_SPEED: f32 = 4.0;
/// `m_ef02.act` has eight motions per action; one-shot, so the index clamps to
/// the last and holds it for the rest of the lifetime.
const MOTION_COUNT: usize = 8;
/// 100 frames at 60 fps. The eight-motion animation finishes
/// before this; the held final motion lingers until it elapses.
pub const TOTAL_DURATION_MS: u32 = 1667;

pub const SPRITE: &str = "data/sprite/이팩트/m_ef02";

/// Preload set for the SpriteParticle path (see
/// [`crate::effect::custom_effect_sprite_paths`]).
pub const SPRITES: &[&str] = &[SPRITE];

pub struct MEf02Effect {
    anchor: [f32; 3],
    age: f32,
}

impl MEf02Effect {
    pub fn new(anchor: [f32; 3]) -> Self {
        Self { anchor, age: 0.0 }
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

/// Pick the fly-off action from the camera-relative angle, matching the
/// original game's quadrant switch (distinct from Wink's map).
fn action_for_angle(angle_deg: f32) -> usize {
    let a = angle_deg.rem_euclid(360.0);
    if (180.0..270.0).contains(&a) {
        2
    } else if (90.0..180.0).contains(&a) {
        3
    } else if a < 90.0 {
        0
    } else {
        1
    }
}

impl Effect for MEf02Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= TOTAL_DURATION_MS as f32 / 1000.0 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        out.push(EffectPrimitiveDraw::SpriteParticle {
            sprite_path: SPRITE,
            position: self.anchor,
            action_index: action_for_angle(camera_longitude_deg(&ctx.camera)),
            motion_index: self.motion_index(),
            size_scale: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            blend: BlendKind::Alpha,
            aim_target: None,
            no_depth: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam_at(deg: f32) -> CameraView {
        let r = deg.to_radians();
        CameraView {
            eye: [r.sin() * 10.0, 5.0, r.cos() * 10.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, -1.0, 0.0],
        }
    }

    #[test]
    fn action_follows_camera_quadrant() {
        // Each 90° camera quadrant selects a different action — the M02 map,
        // not Wink's. Keeps a consistent screen direction as the camera orbits.
        assert_eq!(action_for_angle(45.0), 0);
        assert_eq!(action_for_angle(135.0), 3);
        assert_eq!(action_for_angle(225.0), 2);
        assert_eq!(action_for_angle(315.0), 1);
        assert_eq!(action_for_angle(camera_longitude_deg(&cam_at(225.0))), 2);
    }

    #[test]
    fn motion_plays_once_then_holds_and_effect_dies() {
        let mut e = MEf02Effect::new([0.0; 3]);
        let m0 = e.motion_index();
        for _ in 0..8 {
            e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None });
        }
        assert!(e.motion_index() > m0, "motion advances");

        let mut status = EffectStatus::Running;
        for _ in 0..200 {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None });
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead, "self-terminates");
        assert_eq!(e.motion_index(), MOTION_COUNT - 1, "holds final motion");
    }
}
