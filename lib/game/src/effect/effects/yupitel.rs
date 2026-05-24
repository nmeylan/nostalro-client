//! `EF_YUFITEL` — Jupiter Thunder ball (id 93).
//!
//! Two co-traveling additive billboards: a static center glow
//! (`thunder_center.bmp`, 3.5×3.5, alpha 170/255) and an animated 6-frame
//! thunder ball (`thunder_ball_a-f.bmp`, 4.5×4.5, cycling at 1 tex/tick).
//! Both travel from caster toward target over 20 frames, matching the
//! original game's look.
//!
//! When spawned without trail data (`from == to`), falls back to a single
//! expanding billboard at the spawn point.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const CENTER_TEXTURE: &str = "thunder_center.bmp";
const BALL_TEXTURES: &[&str] = &[
    "thunder_ball_a.bmp",
    "thunder_ball_b.bmp",
    "thunder_ball_c.bmp",
    "thunder_ball_d.bmp",
    "thunder_ball_e.bmp",
    "thunder_ball_f.bmp",
];
pub const TEXTURES: &[&str] = &[
    CENTER_TEXTURE,
    "thunder_ball_a.bmp",
    "thunder_ball_b.bmp",
    "thunder_ball_c.bmp",
    "thunder_ball_d.bmp",
    "thunder_ball_e.bmp",
    "thunder_ball_f.bmp",
];

const FPS: f32 = 60.0;
const DURATION_FRAMES: f32 = 20.0;
const DURATION_S: f32 = DURATION_FRAMES / FPS;
pub const TOTAL_DURATION_MS: u32 = (DURATION_S * 1000.0) as u32;

const CENTER_SIZE: [f32; 2] = [3.5, 3.5];
const BALL_SIZE: [f32; 2] = [4.5, 4.5];
const CENTER_ALPHA: f32 = 170.0 / 255.0;
const Y_OFFSET: f32 = -5.0;
const TARGET_KILL_DISTANCE: f32 = 3.0;

const BALL_FRAME_S: f32 = 1.0 / FPS;

const STATIC_DURATION_FRAMES: f32 = 60.0;
const STATIC_DURATION_S: f32 = STATIC_DURATION_FRAMES / FPS;
const STATIC_BASE_SIZE: f32 = 3.0;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

pub struct YupitelEffect {
    from: [f32; 3],
    to: [f32; 3],
    age: f32,
    velocity: [f32; 3],
    is_trail: bool,
}

impl YupitelEffect {
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let dist = (dx * dx + dz * dz).sqrt();
        let is_trail = dist > TARGET_KILL_DISTANCE;

        let velocity = if dist > 0.001 {
            let speed_per_frame = dist / DURATION_FRAMES;
            let speed_per_s = speed_per_frame * FPS;
            let ux = dx / dist;
            let uz = dz / dist;
            [ux * speed_per_s, 0.0, uz * speed_per_s]
        } else {
            [0.0, 0.0, 0.0]
        };

        Self {
            from,
            to,
            age: 0.0,
            velocity,
            is_trail,
        }
    }

    fn current_pos(&self) -> [f32; 3] {
        [
            self.from[0] + self.velocity[0] * self.age,
            self.from[1] + Y_OFFSET,
            self.from[2] + self.velocity[2] * self.age,
        ]
    }

    fn reached_target(&self) -> bool {
        let pos = self.current_pos();
        let dx = pos[0] - self.to[0];
        let dz = pos[2] - self.to[2];
        (dx * dx + dz * dz).sqrt() <= TARGET_KILL_DISTANCE
    }
}

impl Effect for YupitelEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;

        if self.is_trail {
            if self.age >= DURATION_S || self.reached_target() {
                EffectStatus::Dead
            } else {
                EffectStatus::Running
            }
        } else if self.age >= STATIC_DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.is_trail {
            let pos = self.current_pos();
            let ball_idx = (self.age / BALL_FRAME_S) as usize % BALL_TEXTURES.len();

            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: CENTER_SIZE,
                uv: UNIT_UV,
                rotation: 0.0,
                texture: CENTER_TEXTURE,
                color: [1.0, 1.0, 1.0, CENTER_ALPHA],
                blend: BlendKind::Additive,
            });

            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: BALL_SIZE,
                uv: UNIT_UV,
                rotation: 0.0,
                texture: BALL_TEXTURES[ball_idx],
                color: [1.0, 1.0, 1.0, 1.0],
                blend: BlendKind::Additive,
            });
        } else {
            let t = (self.age / STATIC_DURATION_S).clamp(0.0, 1.0);
            let alpha = if t < 0.7 {
                1.0
            } else {
                (1.0 - (t - 0.7) / 0.3).clamp(0.0, 1.0)
            };
            let scale = STATIC_BASE_SIZE * (1.0 + t * 0.6);
            let ball_idx = (self.age / BALL_FRAME_S) as usize % BALL_TEXTURES.len();
            let pos = [self.from[0], self.from[1] + Y_OFFSET, self.from[2]];

            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [scale, scale],
                uv: UNIT_UV,
                rotation: 0.0,
                texture: BALL_TEXTURES[ball_idx],
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut YupitelEffect, dt: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: dt, camera_target: None })
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(e: &YupitelEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn trail_emits_two_billboards_moving_toward_target() {
        let mut e = YupitelEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 60.0]);
        step(&mut e, 1.0 / FPS);

        let d = draws(&e);
        assert_eq!(d.len(), 2, "center glow + animated ball");

        let z1 = match &d[0] {
            EffectPrimitiveDraw::Billboard { pos, .. } => pos[2],
            other => panic!("expected Billboard, got {other:?}"),
        };

        for _ in 0..5 {
            step(&mut e, 1.0 / FPS);
        }

        let z2 = match &draws(&e)[0] {
            EffectPrimitiveDraw::Billboard { pos, .. } => pos[2],
            other => panic!("expected Billboard, got {other:?}"),
        };
        assert!(z2 > z1, "billboard moved toward +Z target: {z1} -> {z2}");
    }

    #[test]
    fn center_has_reduced_alpha_ball_has_full() {
        let mut e = YupitelEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 60.0]);
        step(&mut e, 1.0 / FPS);

        let d = draws(&e);
        let center_alpha = match &d[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            other => panic!("expected Billboard, got {other:?}"),
        };
        let ball_alpha = match &d[1] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            other => panic!("expected Billboard, got {other:?}"),
        };

        assert!((center_alpha - CENTER_ALPHA).abs() < 0.01);
        assert!((ball_alpha - 1.0).abs() < 0.01);
    }

    #[test]
    fn trail_dies_after_duration() {
        let mut e = YupitelEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 60.0]);
        let mut status = EffectStatus::Running;
        for _ in 0..200 {
            status = step(&mut e, 1.0 / FPS);
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn static_fallback_emits_single_billboard_then_dies() {
        let mut e = YupitelEffect::new([5.0, 0.0, 7.0], [5.0, 0.0, 7.0]);
        step(&mut e, 0.0);
        assert_eq!(draws(&e).len(), 1);

        let mut status = EffectStatus::Running;
        for _ in 0..120 {
            status = step(&mut e, 1.0 / FPS);
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn both_use_additive_blend() {
        let mut e = YupitelEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 60.0]);
        step(&mut e, 1.0 / FPS);

        for prim in draws(&e) {
            match prim {
                EffectPrimitiveDraw::Billboard { blend, .. } => {
                    assert_eq!(blend, BlendKind::Additive);
                }
                other => panic!("expected Billboard, got {other:?}"),
            }
        }
    }
}
