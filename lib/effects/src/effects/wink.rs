//! Wink (`misc\wink.spr`) and Fvoice (`misc\fvoice.spr`): a wink/voice emote
//! that flies off as a four-clip zoomed trail. Both share the exact same
//! handler; only the sprite differs.
//!
//! The emote is drawn at the emote position: the target for a targeted cast,
//! or the caster itself when cast on self. It does not travel: the
//! screen-space fly-off is baked
//! into the `.act` clip offsets.
//!
//! Each `.act` holds four actions, one per diagonal fly-off direction
//! (↙ ↖ ↗ ↘). The action is picked from the caster→target angle plus the
//! camera longitude so the trail always shoots the right way on
//! screen, then plays the
//! six motions of that action once (anim speed 6, no repeat,
//! full size, source vertex colour). Cast on self there is no direction, so the
//! choice reduces to the camera longitude — the emote keeps a consistent screen
//! direction as the camera orbits.
//!
//! The primitive is depth-check free, so it draws over the terrain
//! instead of being depth-culled by the ground it stands on (`no_depth`).
//!
//! Implemented as a `Custom` effect (rather than a `spr_def` one-shot) because
//! the action must be chosen from the live camera, which only `collect_draws`
//! sees. Each frame it emits one `SpriteParticle` with the chosen
//! `action_index` and the animated `motion_index`.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};

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
    /// Caster position.
    caster: [f32; 3],
    /// Where the emote is drawn. For a targeted cast this is the target;
    /// cast on self it is the caster, so the emote sits on the caster
    /// rather than on any separate target.
    target: [f32; 3],
    sprite: &'static str,
    age: f32,
}

impl WinkEffect {
    pub fn new(caster: [f32; 3], target: [f32; 3], sprite: &'static str) -> Self {
        Self { caster, target, sprite, age: 0.0 }
    }

    fn motion_index(&self) -> usize {
        let raw = (self.age * FRAMES_PER_SECOND / ANIM_SPEED) as usize;
        raw.min(MOTION_COUNT - 1)
    }

    /// Pick the fly-off action from the caster→target relationship, the way the
    /// original game chooses it from the caster-to-emote angle.
    ///
    /// The `.act` lays its four actions out as a 2×2 screen grid (clip offsets
    /// are screen pixels — `+x` right, `+y` down): action 0 = left/low,
    /// 1 = left/high, 2 = right/high, 3 = right/low. So we only need which
    /// screen half the caster sits in relative to the emote, which is robust
    /// against any world angle convention: project the caster-to-emote
    /// vector onto the camera's screen basis (the way the renderer aims
    /// sprites) and read off the horizontal/vertical sign. Cast on self there
    /// is no direction — fall back to the camera-longitude choice so the emote
    /// stays put as the camera orbits.
    fn action_index(&self, camera: &CameraView) -> usize {
        let dir = sub(self.caster, self.target);
        if dir[0] == 0.0 && dir[1] == 0.0 && dir[2] == 0.0 {
            return action_for_angle(camera_longitude_deg(camera));
        }
        let forward = normalize(sub(camera.target, camera.eye));
        let right = normalize(cross(forward, camera.up));
        // `cross(right, forward)` points toward the top of the screen (native
        // RO `-Y` = up), so a positive projection means higher on screen.
        let screen_up = cross(right, forward);
        let on_right = dot(dir, right) >= 0.0;
        let on_top = dot(dir, screen_up) >= 0.0;
        match (on_right, on_top) {
            (false, false) => 0,
            (false, true) => 1,
            (true, true) => 2,
            (true, false) => 3,
        }
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len <= 1e-6 { v } else { [v[0] / len, v[1] / len, v[2] / len] }
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
            position: self.target,
            action_index: self.action_index(&ctx.camera),
            motion_index: self.motion_index(),
            size_scale: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            blend: BlendKind::Alpha,
            aim_target: None,
            no_depth: true,
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

    fn render_ctx(camera: CameraView) -> EffectRenderCtx {
        EffectRenderCtx { camera, screen_w: 256.0, screen_h: 256.0, elapsed: 0.0 }
    }

    fn particle(e: &WinkEffect, camera: CameraView) -> ([f32; 3], usize) {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx(camera));
        match l.primitives.first() {
            Some(EffectPrimitiveDraw::SpriteParticle { position, action_index, no_depth, .. }) => {
                assert!(*no_depth, "CHIMTO is RF_NODEPTHCHECK so it isn't swallowed by the floor");
                (*position, *action_index)
            }
            _ => panic!("expected a SpriteParticle"),
        }
    }

    #[test]
    fn renders_at_the_target_and_action_tracks_the_caster_side() {
        // The emote draws at the emote position (the target), and the fly-off
        // action is chosen from the caster relative to it. The `.act`
        // grid is 0=left/low, 1=left/high, 2=right/high, 3=right/low.
        let cam = cam_at(0.0);
        // Self-cast sits on the caster and uses the camera-only fallback action.
        let (self_pos, self_act) = particle(&WinkEffect::new([2.0, 0.0, 3.0], [2.0, 0.0, 3.0], FVOICE), cam);
        assert_eq!(self_pos, [2.0, 0.0, 3.0], "self-cast draws on the caster");
        assert_eq!(self_act, action_for_angle(camera_longitude_deg(&cam)));
        // Targeted: the emote is drawn at the target, not the caster.
        let target = [10.0, 0.0, 0.0];
        let (pos, _) = particle(&WinkEffect::new([0.0; 3], target, FVOICE), cam);
        assert_eq!(pos, target, "emote renders at the target (m_deltaPos)");
        // Casters on opposite sides of the target pick the left vs right action.
        let east = particle(&WinkEffect::new([20.0, 0.0, 0.0], target, FVOICE), cam).1;
        let west = particle(&WinkEffect::new([-20.0, 0.0, 0.0], target, FVOICE), cam).1;
        assert_ne!(east, west, "caster side of the target changes the fly-off action");
    }

    #[test]
    fn motion_plays_once_then_holds_and_effect_dies() {
        let mut e = WinkEffect::new([0.0; 3], [0.0; 3], WINK);
        let m0 = e.motion_index();
        for _ in 0..6 {
            e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None });
        }
        let m_mid = e.motion_index();
        assert!(m_mid > m0, "motion advances: {m0} -> {m_mid}");

        let mut status = EffectStatus::Running;
        for _ in 0..200 {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None });
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead, "self-terminates");
        assert_eq!(e.motion_index(), MOTION_COUNT - 1, "holds the final motion");
    }
}
