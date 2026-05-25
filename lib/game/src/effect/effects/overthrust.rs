//! `EF_OVERTHRUST` — Blacksmith Over Thrust impact ring.
//!
//! A single camera-facing circle
//! primitive launched at frame 0:
//!
//!   * 40-frame life, 10° arc step, inner radius 10.
//!   * Screen-space anchor lifted 20 units up (above the
//!     caster's head).
//!   * Radius speed 7, radius accel `-(speed/(duration+40))*2 =
//!     -0.175` per frame — outward expansion that decelerates over the
//!     lifetime.
//!   * No fade-out delay → alpha linearly fades from full to zero across
//!     the whole 40 frames.
//!   * Texture: `alpha_center.tga`. The circle uses its own alpha write
//!     path, so the blend is
//!     [`BlendKind::Alpha`].
//!
//! Rendered as a [`BillboardRing`] (camera-facing screen-space annulus
//! with V across the radial thickness): matches the original game's
//! UV layout, so `alpha_center.tga` shows as a
//! thin glowing band of constant radial thickness.
//!
//! [`BillboardRing`]: crate::effect::draw::EffectPrimitiveDraw::BillboardRing

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "alpha_center.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 40.0;
pub const TOTAL_DURATION_MS: u32 =
    (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

// Lifts the screen-space anchor above the caster's centre — matches the
// original game's 20-unit lift (native RO Y is up = -Y).
const HEIGHT_OFFSET: f32 = -5.0;

// Original game's per-frame radius integration in pixel space:
//   speed = 7, accel = -0.175, init = 0.
// Scaled to world units so peak disc radius sits around the bash halo
// silhouette (~20 wu — the reference gif shows the ring filling most of
// the frame at peak).
const RADIUS_INIT: f32 = 0.0;
const RADIUS_SPEED: f32 = 2.3;
const RADIUS_ACCEL: f32 = -0.063;

// Original game's inner radius 10 in pixel space; ~1.5 wu at the
// flasher-derived ~8 px/wu conversion. Constant across the lifetime.
const RING_THICKNESS: f32 = 1.5;

const PEAK_ALPHA: f32 = 1.0;
const FADE_IN_FRAMES: f32 = 4.0;

pub struct OverthrustEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl OverthrustEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
        }
    }
}

fn radius_at(frame: f32) -> f32 {
    // r(N) = init + speed*N + accel*N*(N+1)/2
    let n = frame.max(0.0);
    let r = RADIUS_INIT + RADIUS_SPEED * n + RADIUS_ACCEL * n * (n + 1.0) * 0.5;
    r.max(0.0)
}

fn alpha_at(frame: f32) -> f32 {
    // Quick fade-in to mask the single-pixel pop, then linear fade-out
    // across the full lifetime (no fade-out delay).
    let n = frame.clamp(0.0, DURATION_FRAMES);
    let fade_in = (n / FADE_IN_FRAMES).clamp(0.0, 1.0);
    let fade_out = (1.0 - n / DURATION_FRAMES).clamp(0.0, 1.0);
    PEAK_ALPHA * fade_in * fade_out
}

impl Effect for OverthrustEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let radius = radius_at(self.age_frames);
        let alpha = alpha_at(self.age_frames);
        if alpha <= 0.0 || radius <= 0.0 {
            return;
        }
        let pos = [
            self.world_pos[0],
            self.world_pos[1] + HEIGHT_OFFSET,
            self.world_pos[2],
        ];
        out.push(EffectPrimitiveDraw::BillboardRing {
            pos,
            radius,
            thickness: RING_THICKNESS,
            segments: 36,
            uv_repeat: 4.0,
            texture: TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Alpha,
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

    fn draws(effect: &OverthrustEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut OverthrustEffect, frames: f32) -> EffectStatus {
        effect.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
        })
    }

    fn ring_radius(prims: &[EffectPrimitiveDraw]) -> f32 {
        match &prims[0] {
            EffectPrimitiveDraw::BillboardRing { radius, .. } => *radius,
            _ => panic!("expected BillboardRing"),
        }
    }

    #[test]
    fn emits_one_billboard_ring_above_caster() {
        let mut eff = OverthrustEffect::new([10.0, 0.5, -3.0]);
        step(&mut eff, 6.0);
        let prims = draws(&eff);
        assert_eq!(prims.len(), 1);
        match &prims[0] {
            EffectPrimitiveDraw::BillboardRing {
                pos,
                thickness,
                texture,
                blend,
                ..
            } => {
                assert_eq!(*pos, [10.0, 0.5 + HEIGHT_OFFSET, -3.0]);
                assert_eq!(*thickness, RING_THICKNESS);
                assert_eq!(*texture, TEXTURE);
                assert_eq!(*blend, BlendKind::Alpha);
            }
            _ => panic!("expected BillboardRing"),
        }
    }

    #[test]
    fn radius_grows_and_alpha_fades_across_lifetime() {
        let mut eff = OverthrustEffect::new([0.0; 3]);
        step(&mut eff, 6.0);
        let r_early = ring_radius(&draws(&eff));
        let a_early = match &draws(&eff)[0] {
            EffectPrimitiveDraw::BillboardRing { color, .. } => color[3],
            _ => unreachable!(),
        };

        step(&mut eff, 15.0);
        let r_mid = ring_radius(&draws(&eff));

        step(&mut eff, 15.0);
        let prims_late = draws(&eff);
        let a_late = match &prims_late[0] {
            EffectPrimitiveDraw::BillboardRing { color, .. } => color[3],
            _ => unreachable!(),
        };

        assert!(r_mid > r_early, "radius expands ({r_early} -> {r_mid})");
        assert!(a_late < a_early, "alpha fades ({a_early} -> {a_late})");
    }

    #[test]
    fn dies_after_forty_frames() {
        let mut eff = OverthrustEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..(DURATION_FRAMES as i32 + 2) {
            status = step(&mut eff, 1.0);
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
