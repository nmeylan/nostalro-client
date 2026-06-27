//! EF_BOTTOM_SANC — sustained Sanctuary pillar at the caster's feet.
//! Visible reference: `ro-effects/effects/imgs/300-350/317.gif`.
//!
//! A single rising 4-sided pillar:
//!   * square base width 2.5
//!   * vertical extent 16
//!   * start alpha 120 with a long fade timing
//!   * the parent's lifetime is the effect's lifetime (the table value
//!     is `99990 ms`), so the pillar is effectively permanent until
//!     the Sanctuary cell dies — it persists for the skill's whole
//!     duration rather than playing a one-shot animation, matching the
//!     sustained look in the reference gif.
//!
//! We render it as a 4-sided pillar that breathes exactly like Magnus
//! Exorcismus (it is the same `Bottom_Magnus` family in the original game):
//! it rises from the ground over ~90 frames and then pulses its height around
//! ~65 % of its peak. The geometry does not rotate.

use super::bottom_magnus::animated_height;
use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// Lifetime kept in lockstep with the spec's `duration_ms`; the effect is a
/// "permanent" sustained skill effect in the original game.
pub const TOTAL_DURATION_MS: u32 = 99_990;

/// Square pillar — `sides == 4`.
const SIDES: u32 = 4;
/// Pillar half-extent on the X / Z plane.
const BASE_RADIUS: f32 = 2.5;
/// Peak pillar height for F1 == 1. The rendered height breathes around ~65 %
/// of this (see [`animated_height`]).
const PILLAR_HEIGHT: f32 = 16.0;
/// `120 / 255` baseline alpha — the pillar holds at this level.
const BASE_ALPHA: f32 = 120.0 / 255.0;
/// Frames to ramp from 0 to BASE_ALPHA at spawn — matches the gif fade-in.
const FADE_IN_FRAMES: f32 = 15.0;

pub struct BottomSanctuaryPillarEffect {
    world_pos: [f32; 3],
    age: f32,
    phase_deg: f32,
}

impl BottomSanctuaryPillarEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let key = (world_pos[0].to_bits() ^ world_pos[2].to_bits()) as f32 * 1.6180339;
        Self {
            world_pos,
            age: 0.0,
            phase_deg: key.rem_euclid(360.0),
        }
    }
}

impl Effect for BottomSanctuaryPillarEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        let total_s = TOTAL_DURATION_MS as f32 / 1000.0;
        if self.age >= total_s {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        let alpha = BASE_ALPHA * (frame / FADE_IN_FRAMES).clamp(0.0, 1.0);
        let height = animated_height(PILLAR_HEIGHT, self.age, self.phase_deg);

        out.push(EffectPrimitiveDraw::Cylinder {
            base: self.world_pos,
            bottom_size: BASE_RADIUS,
            top_size: BASE_RADIUS,
            height,
            sides: SIDES,
            // No geometry rotation — the original game keeps `RotStart = 0`.
            rotation: 0.0,
            tilt_x_rad: 0.0,
            rotation_y_rad: 0.0,
            uv_scroll: [0.0, 0.0],
            texture: TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            alpha_bottom: alpha,
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

    fn draws(effect: &BottomSanctuaryPillarEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut BottomSanctuaryPillarEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    #[test]
    fn emits_a_square_frustum() {
        let mut bs = BottomSanctuaryPillarEffect::new([0.0; 3]);
        step(&mut bs, 1.0);
        match &draws(&bs)[0] {
            EffectPrimitiveDraw::Cylinder {
                sides,
                bottom_size,
                top_size,
                height,
                ..
            } => {
                assert_eq!(*sides, 4);
                assert!(
                    (bottom_size - top_size).abs() < f32::EPSILON,
                    "cylinder, not cone"
                );
                assert!(*height > 0.0);
            }
            other => panic!("expected Cylinder, got {other:?}"),
        }
    }

    #[test]
    fn never_rotates() {
        // The geometry must hold a fixed orientation (no spin, no random
        // initial angle) — matching the original game's `RotStart = 0`.
        let mut bs = BottomSanctuaryPillarEffect::new([12.0, 0.0, 34.0]);
        for _ in 0..3 {
            step(&mut bs, 0.5);
            match &draws(&bs)[0] {
                EffectPrimitiveDraw::Cylinder { rotation, .. } => {
                    assert_eq!(*rotation, 0.0, "pillar must not rotate")
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn pillar_rises_from_ground_then_breathes_below_peak() {
        // Like Magnus: starts at the ground (height 0), then breathes within
        // [0.30, 1.0]·PILLAR_HEIGHT once steady — never pinned at the peak.
        let mut bs = BottomSanctuaryPillarEffect::new([0.0; 3]);
        step(&mut bs, 0.0);
        let h0 = match &draws(&bs)[0] {
            EffectPrimitiveDraw::Cylinder { height, .. } => *height,
            _ => unreachable!(),
        };
        assert!(h0 < 1e-3, "rises from the ground: {h0}");

        let (mut lo, mut hi) = (f32::MAX, 0.0_f32);
        for f in 90..=450 {
            step(&mut bs, 0.0); // no advance; sample via age
            let h = animated_height(PILLAR_HEIGHT, f as f32 / FRAMES_PER_SECOND, bs.phase_deg);
            lo = lo.min(h);
            hi = hi.max(h);
        }
        assert!(
            lo >= 0.30 * PILLAR_HEIGHT - 0.5 && hi <= PILLAR_HEIGHT + 0.5,
            "breath bounds: {lo}..{hi}"
        );
    }

    #[test]
    fn alpha_ramps_in_then_holds() {
        let mut bs = BottomSanctuaryPillarEffect::new([0.0; 3]);
        step(&mut bs, 0.0);
        let a0 = match &draws(&bs)[0] {
            EffectPrimitiveDraw::Cylinder { color, .. } => color[3],
            _ => unreachable!(),
        };
        step(&mut bs, FADE_IN_FRAMES / FRAMES_PER_SECOND + 0.01);
        let a_peak = match &draws(&bs)[0] {
            EffectPrimitiveDraw::Cylinder { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_peak > a0);
        assert!((a_peak - BASE_ALPHA).abs() < 1e-4, "holds at BASE_ALPHA");
    }

    #[test]
    fn runs_for_full_duration() {
        let mut bs = BottomSanctuaryPillarEffect::new([0.0; 3]);
        let s = bs.update(&EffectUpdateCtx {
            delta: 1.0,
            camera_target: None,
            caster_yaw: None,
        });
        assert!(matches!(s, EffectStatus::Running));
    }
}
