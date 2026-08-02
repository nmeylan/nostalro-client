use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{CameraView, Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const ANIM_SPEED: f32 = 2.0;

pub const TOTAL_DURATION_MS: u32 = 40000;
const TOTAL_DURATION_S: f32 = TOTAL_DURATION_MS as f32 / 1000.0;

const GHOST_SPRITE: &str = ragnarok_resources::sprite::effect::GHOST;
const BAT_SPRITE: &str = ragnarok_resources::sprite::effect::BAT;
pub const SPRITES: &[&str] = &[GHOST_SPRITE, BAT_SPRITE];

const WORLD_SCALE: f32 = 0.35;
const RADIUS: f32 = 16.0 * WORLD_SCALE;
const BOB_AMP: f32 = 4.0 * WORLD_SCALE;
const SIZE_SCALE: f32 = 1.5;
const RISE_DEG_PER_FRAME: f32 = 3.0;

const TRAIL: usize = 10;
const FADE_IN_FRAMES: f32 = 30.0;
const FADE_OUT_S: f32 = 1.0;

#[derive(Clone, Copy)]
pub struct GhostParams {
    pub sprite: &'static str,
    pub orbit_speed_deg: f32,
    pub lissajous: bool,
    pub ghost_actions: bool,
    pub motion_count: usize,
}

pub const GHOST: GhostParams = GhostParams {
    sprite: GHOST_SPRITE,
    orbit_speed_deg: 1.0,
    lissajous: false,
    ghost_actions: true,
    motion_count: 8,
};
pub const BAT: GhostParams = GhostParams {
    sprite: BAT_SPRITE,
    orbit_speed_deg: 1.0,
    lissajous: false,
    ghost_actions: false,
    motion_count: 5,
};
pub const BAT2: GhostParams = GhostParams {
    sprite: BAT_SPRITE,
    orbit_speed_deg: 2.0,
    lissajous: true,
    ghost_actions: false,
    motion_count: 5,
};

pub struct GhostEffect {
    anchor: [f32; 3],
    params: GhostParams,
    age: f32,
}

fn camera_longitude_deg(camera: &CameraView) -> f32 {
    let dx = camera.eye[0] - camera.target[0];
    let dz = camera.eye[2] - camera.target[2];
    dx.atan2(dz).to_degrees().rem_euclid(360.0)
}

fn ghost_action_for_angle(angle_deg: f32) -> usize {
    if (180.0..360.0).contains(&angle_deg.rem_euclid(360.0)) {
        6
    } else {
        0
    }
}

impl GhostEffect {
    pub fn new(anchor: [f32; 3], params: GhostParams) -> Self {
        Self {
            anchor,
            params,
            age: 0.0,
        }
    }

    fn motion_index(&self, local_frame: f32) -> usize {
        ((local_frame / ANIM_SPEED).max(0.0) as usize) % self.params.motion_count
    }

    fn envelope(&self, age_frames: f32) -> f32 {
        let fade_in = (age_frames / FADE_IN_FRAMES).clamp(0.0, 1.0);
        let fade_out = ((TOTAL_DURATION_S - self.age) / FADE_OUT_S).clamp(0.0, 1.0);
        fade_in * fade_out
    }
}

impl Effect for GhostEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= TOTAL_DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        let age_frames = self.age * FRAMES_PER_SECOND;
        let cam_long = camera_longitude_deg(&ctx.camera);
        let env = self.envelope(age_frames);
        if env <= 0.0 {
            return;
        }
        for i in 0..TRAIL {
            let local_frame = age_frames - i as f32;
            if local_frame < 0.0 {
                continue;
            }
            let rot_deg = local_frame * self.params.orbit_speed_deg;
            let theta = rot_deg.to_radians();
            let x = self.anchor[0] + theta.cos() * RADIUS;
            let z_unit = if self.params.lissajous {
                (2.0 * theta).sin()
            } else {
                theta.sin()
            };
            let z = self.anchor[2] + z_unit * RADIUS;
            let bob = BOB_AMP * (local_frame * RISE_DEG_PER_FRAME).to_radians().sin();
            let y = self.anchor[1] + bob;

            let trail_fade = 1.0 - (i as f32 / TRAIL as f32) * 0.7;
            let alpha = env * trail_fade;

            // 180° offset: camera_longitude_deg measures target→eye; the ghost facing needs the opposite sense.
            let action_index = if self.params.ghost_actions {
                ghost_action_for_angle(rot_deg + cam_long + 180.0)
            } else {
                0
            };

            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: self.params.sprite,
                position: [x, y, z],
                action_index,
                motion_index: self.motion_index(local_frame),
                size_scale: SIZE_SCALE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Alpha,
                aim_target: None,
                no_depth: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_cam(deg: f32) -> EffectRenderCtx {
        let r = deg.to_radians();
        EffectRenderCtx {
            camera: CameraView {
                eye: [r.sin() * 10.0, 5.0, r.cos() * 10.0],
                target: [0.0, 0.0, 0.0],
                up: [0.0, -1.0, 0.0],
            },
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut GhostEffect, frames: u32) {
        for _ in 0..frames {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn draws(e: &GhostEffect, cam_deg: f32) -> Vec<EffectPrimitiveDraw> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &ctx_cam(cam_deg));
        l.primitives
    }

    fn sprite_positions(prims: &[EffectPrimitiveDraw]) -> Vec<[f32; 3]> {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { position, .. } => Some(*position),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn orbiters_circle_the_anchor_and_bob_vertically() {
        let anchor = [3.0, 1.0, -2.0];
        let mut e = GhostEffect::new(anchor, GHOST);
        step(&mut e, 40);
        let pos = sprite_positions(&draws(&e, 0.0));
        assert!(!pos.is_empty(), "swarm visible after fade-in");
        for p in &pos {
            let dx = p[0] - anchor[0];
            let dz = p[2] - anchor[2];
            let r = (dx * dx + dz * dz).sqrt();
            assert!(
                (r - RADIUS).abs() < 1e-3,
                "ghost orbits on a circle of RADIUS: {r}"
            );
        }
        let y0 = pos[0][1];
        step(&mut e, 10);
        let y1 = sprite_positions(&draws(&e, 0.0))[0][1];
        assert!((y1 - y0).abs() > 1e-4, "lead orbiter bobs in Y");
    }

    #[test]
    fn ghost_action_faces_camera_bat_holds_zero() {
        let mut g = GhostEffect::new([0.0; 3], GHOST);
        step(&mut g, 5);
        let actions: std::collections::BTreeSet<usize> = (0..360)
            .step_by(30)
            .flat_map(|d| {
                draws(&g, d as f32).into_iter().filter_map(|p| match p {
                    EffectPrimitiveDraw::SpriteParticle { action_index, .. } => Some(action_index),
                    _ => None,
                })
            })
            .collect();
        assert_eq!(actions, [0usize, 6].into_iter().collect());

        let mut b = GhostEffect::new([0.0; 3], BAT);
        step(&mut b, 5);
        assert!(
            draws(&b, 123.0).iter().all(|p| matches!(
                p,
                EffectPrimitiveDraw::SpriteParticle {
                    action_index: 0,
                    ..
                }
            )),
            "bat holds action 0"
        );
    }

    #[test]
    fn bat2_orbit_is_a_figure_eight_and_effect_self_terminates() {
        let mut e = GhostEffect::new([0.0; 3], BAT2);
        step(&mut e, 60);
        let radii: Vec<f32> = sprite_positions(&draws(&e, 0.0))
            .iter()
            .map(|p| (p[0] * p[0] + p[2] * p[2]).sqrt())
            .collect();
        let max = radii.iter().cloned().fold(0.0_f32, f32::max);
        let min = radii.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            max - min > 1e-2,
            "bat2 path is not a constant-radius circle"
        );

        let mut status = EffectStatus::Running;
        for _ in 0..(TOTAL_DURATION_MS / 16 + 100) {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
