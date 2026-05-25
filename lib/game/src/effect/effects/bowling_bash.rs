//! EF_BOWLINGBASH — ground impact ring (partial implementation).
//!
//! Original game's full recipe spawns a cylinder swept-attack visual
//! plus a 3D-circle ground impact ring. This module ships **only the
//! ring portion** — the cylinder portion is blocked on the Cylinder
//! renderer (Batch CYL). Recipe parameters for the ring (from
//! (the increase-agility recipe):
//!
//! * texture `effect/ring_yellow.tga`, additive blend
//! * outer radius starts at 8.0 and grows by 0.7/frame
//! * deceleration `-(0.7 / 30) / 2 ≈ -0.0117 /frame²`
//! * peak alpha 45/255, fades after frame 35
//! * 50-frame visible lifetime
//!
//! `TOTAL_DURATION_MS` is left at the table's 2500 ms so that, once the
//! cylinder portion lands, both sub-primitives share the same parent
//! lifetime without changing the table.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "ring_yellow.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 2_500;
const TOTAL_DURATION_S: f32 = TOTAL_DURATION_MS as f32 / 1000.0;

const RING_LIFE_FRAMES: f32 = 50.0;
const RING_LIFE_S: f32 = RING_LIFE_FRAMES / FRAMES_PER_SECOND;

const INITIAL_RADIUS: f32 = 8.0;
const RADIUS_SPEED_PER_FRAME: f32 = 0.7;
const RADIUS_ACCEL_PER_FRAME2: f32 = -(RADIUS_SPEED_PER_FRAME / 30.0) / 2.0;

const PEAK_ALPHA: f32 = 45.0 / 255.0;
const FADE_OUT_AT_FRAME: f32 = 35.0;

const THICKNESS: f32 = 4.0;
const UV_REPEAT: f32 = 4.0;

pub struct BowlingBashEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl BowlingBashEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }
}

fn radius_at(frame: f32) -> f32 {
    INITIAL_RADIUS
        + RADIUS_SPEED_PER_FRAME * frame
        + RADIUS_ACCEL_PER_FRAME2 * frame * (frame + 1.0) / 2.0
}

fn alpha_at(frame: f32) -> f32 {
    if frame <= FADE_OUT_AT_FRAME {
        PEAK_ALPHA
    } else {
        let fade =
            ((frame - FADE_OUT_AT_FRAME) / (RING_LIFE_FRAMES - FADE_OUT_AT_FRAME)).clamp(0.0, 1.0);
        PEAK_ALPHA * (1.0 - fade)
    }
}

impl Effect for BowlingBashEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= TOTAL_DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.age >= RING_LIFE_S {
            return;
        }
        let frame = (self.age * FRAMES_PER_SECOND).clamp(0.0, RING_LIFE_FRAMES);
        let radius = radius_at(frame).max(0.0);
        if radius <= 0.0 {
            return;
        }
        let alpha = alpha_at(frame);
        out.push(EffectPrimitiveDraw::GroundDisc {
            center: self.world_pos,
            radius,
            thickness: THICKNESS,
            rotation: 0.0,
            arc_angle_deg: 360.0,
            uv_repeat: UV_REPEAT,
            texture: TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Additive,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(effect: &BowlingBashEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut BowlingBashEffect, dt: f32) -> EffectStatus {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
        })
    }

    #[test]
    fn emits_ring_then_expires_after_50_frames() {
        let mut eff = BowlingBashEffect::new([0.0, 0.0, 0.0]);
        step(&mut eff, 0.0);
        let prims = draws(&eff);
        assert_eq!(prims.len(), 1);
        match &prims[0] {
            EffectPrimitiveDraw::GroundDisc {
                radius,
                texture,
                blend,
                ..
            } => {
                assert!((*radius - INITIAL_RADIUS).abs() < 1e-3);
                assert_eq!(*texture, TEXTURE);
                assert_eq!(*blend, BlendKind::Additive);
            }
            _ => panic!("expected GroundDisc"),
        }
        step(&mut eff, RING_LIFE_S + 0.01);
        assert_eq!(draws(&eff).len(), 0, "ring expires at frame 50");
    }

    #[test]
    fn ring_grows_then_fade_begins_after_frame_35() {
        let mut eff = BowlingBashEffect::new([0.0; 3]);
        // Frame 10 — well before fade.
        step(&mut eff, 10.0 / FRAMES_PER_SECOND);
        let (r_early, a_early) = match &draws(&eff)[0] {
            EffectPrimitiveDraw::GroundDisc { radius, color, .. } => (*radius, color[3]),
            _ => unreachable!(),
        };
        assert!(r_early > INITIAL_RADIUS, "ring grows");
        assert!((a_early - PEAK_ALPHA).abs() < 1e-6, "still at peak alpha");

        // Frame 45 — deep into fade.
        step(&mut eff, 35.0 / FRAMES_PER_SECOND);
        let a_late = match &draws(&eff)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_late < PEAK_ALPHA, "fade-out engaged");
    }

    #[test]
    fn parent_dies_after_total_duration() {
        let mut eff = BowlingBashEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < TOTAL_DURATION_S * 1.5 {
            status = step(&mut eff, 1.0 / 60.0);
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
