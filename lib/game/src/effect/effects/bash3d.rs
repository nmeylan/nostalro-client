//! `EF_BASH3D` — Knight's Bash impact speed-lines.
//!
//! Reference: `ro-effects/effects/imgs/350-400/364.gif`.
//!
//! Not a ring like Defender/Wind — each "slot" renders a thin triangular
//! fan blade: apex 12 units above the caster, two outer points at
//! exponentially-growing distance, fading after a brief alpha pulse.
//!
//! The original game's `EF_BASH3D` dispatcher spawns five `BASH3D()`
//! sub-prims per cast (F1 = 0..4), each holding 4 fan blades. That's 20
//! fans total, each pointing in a different direction (`rise_angle = 90·ec
//! + F1·22`). We mirror that with five [`RadialEmitter`]s indexed by F1.
//!
//! Per-fan tick (the 3D bash primitive, default `height[4] = F2 = 0` branch):
//! * `process` starts at `-24`, increments every frame. Everything below
//!   is gated by `process > 0`, so frames 0–24 are a silent wait.
//! * `process ∈ 1..=10`: base alpha += 20 (reaches 200/255 by frame 10).
//! * `process ∈ 11..=12`: hold.
//! * `process > 12`: base alpha -= 15 per frame down to 0.
//! * Every frame (once `process > 0`): `distance *= 1.15` — exponential
//!   outward growth.
//!
//! Geometry (the Bash 3D render, condensed): for each fan with rotation
//! `rot_start_deg` and azimuth `rise_angle`, the apex sits at
//! `(0, height[0]=-12, 0)` relative to the caster. The two outer points
//! at `rise ± 5°` are computed as:
//! ```text
//! outer.x = cos(rot_start) · cos(rise ± δ) · distance
//! outer.y = sin(rot_start) · cos(rise ± δ) · distance + apex_y
//! outer.z = sin(rise ± δ) · distance
//! ```
//! producing a thin sliver from apex outward in a 3D direction set by
//! `(rot_start_deg, rise_angle)`. We emit one [`EffectPrimitiveDraw::WorldQuad`]
//! per fan with two corners collapsed to the apex (degenerate quad
//! rendered as a triangle).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::radial_emitter::{RADIAL_EMITTER_SLOTS, RadialEmitter, RadialEmitterSlot};

pub const TEXTURE: &str = "alpha_center.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// Original game `SET_DURATION(200)`. The fans only animate for ~50
/// frames (25 silent + 10 ramp + 2 hold + 13 fade), but we honour the
/// prim lifetime so any companion sound/animation timings still match.
const TOTAL_FRAMES: u32 = 200;
pub const TOTAL_DURATION_MS: u32 =
    ((TOTAL_FRAMES as f32) / FRAMES_PER_SECOND * 1000.0) as u32;

/// Five sub-instances (`F1 = 0..4`) per cast, each with `RADIAL_EMITTER_SLOTS`
/// fan blades. The original game's dispatcher loops `for (i=0;i<5;i++)
/// BASH3D("...", i);`.
const SUB_INSTANCES: usize = 5;

/// `height[0]` in `BASH3D()` constructor — the apex Y offset above the
/// caster's feet. Native RO `-Y = up`, so apex is 12 units up.
const APEX_Y_OFFSET: f32 = -12.0;

/// Initial fan radius before exponential growth kicks in.
const DISTANCE_INITIAL: f32 = 2.0;
const DISTANCE_GROWTH_PER_FRAME: f32 = 1.15;

/// `process` starts at `-24` in the original game; the per-frame physics is gated by
/// `process > 0`, so the first 24 ticks are a silent wind-up.
const PROCESS_INITIAL: i32 = -24;

/// Half-spread of the fan blade in degrees. The original game's default
/// branch renders two layered quads per slot — an inner narrow one and
/// an outer wider one, tinted differently to give the blade a two-tone
/// look.
const INNER_HALF_SPREAD_DEG: f32 = 2.0;
const OUTER_HALF_SPREAD_DEG: f32 = 5.0;

/// Matches the original game's per-quad RGB tints for the F2=0 branch
/// (the 3D bash render): cyan inner blade, red outer haze.
const INNER_COLOR_RGB: [f32; 3] = [0.0, 250.0 / 255.0, 250.0 / 255.0];
const OUTER_COLOR_RGB: [f32; 3] = [250.0 / 255.0, 0.0, 0.0];

const ALPHA_RAMP_FRAMES: i32 = 10;
const ALPHA_RAMP_STEP: f32 = 20.0 / 255.0;
const ALPHA_HOLD_UNTIL_FRAME: i32 = 12;
const ALPHA_FADE_STEP: f32 = 15.0 / 255.0;
const ALPHA_CAP: f32 = 200.0 / 255.0;

/// `rise_angle = 90·ec + F1·22`, in degrees. With 4 slots per sub-instance
/// (ec = 0..3) and 5 sub-instances (F1 = 0..4), the 20 fans cover azimuths
/// 0°, 22°, 44°, ..., 358° plus extras — a dense radial star.
const RISE_ANGLE_STEP_PER_F1_DEG: f32 = 22.0;
const RISE_ANGLE_STEP_PER_SLOT_DEG: f32 = 90.0;

pub struct Bash3dEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    last_processed_frame: u32,
    /// Per-fan signed process counter. `RadialEmitterSlot::process` is
    /// `u32`, but the original seeds it to `-24` for the F2 = 0 branch; we keep
    /// a parallel signed array so the silent wind-up frames work
    /// correctly without overflowing back through u32::MAX.
    process: [[i32; RADIAL_EMITTER_SLOTS]; SUB_INSTANCES],
    emitters: [RadialEmitter; SUB_INSTANCES],
}

impl Bash3dEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut emitters = [RadialEmitter::empty(); SUB_INSTANCES];
        for (f1, emitter) in emitters.iter_mut().enumerate() {
            let mut slots = [RadialEmitterSlot::dormant(); RADIAL_EMITTER_SLOTS];
            for ec in 0..RADIAL_EMITTER_SLOTS {
                let rise = (ec as f32) * RISE_ANGLE_STEP_PER_SLOT_DEG
                    + (f1 as f32) * RISE_ANGLE_STEP_PER_F1_DEG;
                let mut s = RadialEmitterSlot::spawn(DISTANCE_INITIAL, rise, 0.0);
                s.alpha_b = 0.0;
                s.rot_start_deg = fan_rot_start_deg(f1, ec);
                slots[ec] = s;
            }
            *emitter = RadialEmitter::from_slots(slots);
        }
        Self {
            world_pos,
            age_frames: 0.0,
            last_processed_frame: 0,
            process: [[PROCESS_INITIAL; RADIAL_EMITTER_SLOTS]; SUB_INSTANCES],
            emitters,
        }
    }

    fn integrate_frames(&mut self, target_frame: u32) {
        while self.last_processed_frame < target_frame {
            for (f1, emitter) in self.emitters.iter_mut().enumerate() {
                for ec in 0..RADIAL_EMITTER_SLOTS {
                    let slot = &mut emitter.slots[ec];
                    if !slot.alive {
                        continue;
                    }
                    let p = &mut self.process[f1][ec];
                    *p += 1;
                    if *p <= 0 {
                        continue;
                    }

                    if *p <= ALPHA_RAMP_FRAMES {
                        slot.alpha_b = (slot.alpha_b + ALPHA_RAMP_STEP).min(ALPHA_CAP);
                    } else if *p > ALPHA_HOLD_UNTIL_FRAME {
                        slot.alpha_b = (slot.alpha_b - ALPHA_FADE_STEP).max(0.0);
                    }
                    slot.distance *= DISTANCE_GROWTH_PER_FRAME;
                }
            }
            self.last_processed_frame += 1;
        }
    }
}

/// The 3D bash handler sets start rotation = random(360) per fan. Real RNG would
/// produce different visuals each cast — we use a deterministic hash of
/// `(f1, ec)` so tests are stable and the visual stays consistent across
/// frames. The pattern is "evenly spread" rather than "uniform random",
/// which actually reads better as a star burst.
fn fan_rot_start_deg(f1: usize, ec: usize) -> f32 {
    let index = (f1 * RADIAL_EMITTER_SLOTS + ec) as f32;
    (index * 360.0 / (SUB_INSTANCES * RADIAL_EMITTER_SLOTS) as f32) % 360.0
}

/// One fan blade vertex computation. Returns `(apex, outer_minus, outer_plus)`
/// in world space.
fn fan_corners(
    center: [f32; 3],
    rise_angle_deg: f32,
    rot_start_deg: f32,
    distance: f32,
    half_spread_deg: f32,
) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let apex = [
        center[0],
        center[1] + APEX_Y_OFFSET,
        center[2],
    ];
    let (sin_rs, cos_rs) = rot_start_deg.to_radians().sin_cos();
    let outer = |rise_offset_deg: f32| -> [f32; 3] {
        let (sin_r, cos_r) = (rise_angle_deg + rise_offset_deg).to_radians().sin_cos();
        [
            center[0] + cos_rs * cos_r * distance,
            apex[1] + sin_rs * cos_r * distance,
            center[2] + sin_r * distance,
        ]
    };
    (apex, outer(-half_spread_deg), outer(half_spread_deg))
}

impl Effect for Bash3dEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = (self.age_frames as u32).min(TOTAL_FRAMES);
        self.integrate_frames(target);

        if self.age_frames >= TOTAL_FRAMES as f32 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some("bash3d")
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for emitter in self.emitters.iter() {
            for (_ec, slot) in emitter.active() {
                if slot.alpha_b <= 0.0 {
                    continue;
                }
                // Two layered blades per fan — cyan inner, red outer.
                for (half_spread, rgb) in [
                    (INNER_HALF_SPREAD_DEG, INNER_COLOR_RGB),
                    (OUTER_HALF_SPREAD_DEG, OUTER_COLOR_RGB),
                ] {
                    let (apex, outer_lo, outer_hi) = fan_corners(
                        self.world_pos,
                        slot.rise_angle_deg,
                        slot.rot_start_deg,
                        slot.distance,
                        half_spread,
                    );
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        // Degenerate quad → triangle: apex shared between
                        // two corners so the visible shape is a sliver
                        // from apex out to the two outer points.
                        corners: [apex, outer_lo, outer_hi, apex],
                        uv: [[0.5, 0.0], [0.0, 1.0], [1.0, 1.0], [0.5, 0.0]],
                        texture: TEXTURE,
                        color: [rgb[0], rgb[1], rgb[2], slot.alpha_b],
                        blend: BlendKind::Additive,
                    });
                }
            }
        }
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

    fn step(e: &mut Bash3dEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
        })
    }

    fn draws(e: &Bash3dEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn quad_alpha(prim: &EffectPrimitiveDraw) -> f32 {
        match prim {
            EffectPrimitiveDraw::WorldQuad { color, .. } => color[3],
            _ => panic!("expected WorldQuad, got {:?}", prim),
        }
    }

    #[test]
    fn silent_first_24_frames_then_full_starburst() {
        // process seeds at -24, so the whole effect is invisible
        // until process > 0 → frame 25. Before then, nothing emits.
        let mut e = Bash3dEffect::new([5.0, 0.0, -3.0]);
        step(&mut e, 10.0);
        assert!(draws(&e).is_empty(), "no draws during silent wind-up");

        // After the wind-up, all 20 fans should emit two layered quads each
        // (cyan inner + red outer) = 40 WorldQuads total.
        step(&mut e, 20.0);
        let prims = draws(&e);
        assert_eq!(
            prims.len(),
            SUB_INSTANCES * RADIAL_EMITTER_SLOTS * 2,
            "all 20 fan blades visible × 2 layers (inner+outer)",
        );
    }

    #[test]
    fn alpha_pulses_then_fades() {
        let mut e = Bash3dEffect::new([0.0; 3]);
        // Step past the silent wind-up + a couple ramp frames.
        step(&mut e, (24 + 3) as f32);
        let alpha_ramp = quad_alpha(&draws(&e)[0]);
        // 3 ramp ticks at +20/255 each = 60/255 ≈ 0.235.
        assert!(
            alpha_ramp > 0.05 && alpha_ramp < ALPHA_CAP,
            "ramping alpha: {alpha_ramp}",
        );

        // Step well past the fade window — all fans should be gone.
        step(&mut e, 50.0);
        assert!(
            draws(&e).is_empty(),
            "fans fully faded, no draws emitted",
        );
    }

    #[test]
    fn distance_grows_exponentially_during_active_window() {
        // Compare apex-to-outer distance between two consecutive active
        // frames to confirm the 1.15× growth law.
        let mut e = Bash3dEffect::new([0.0; 3]);
        step(&mut e, 26.0); // process=2 (first ramp tick was at 25, this is 2nd)

        // Compare the same fan's outer blade between two consecutive frames.
        // Quads are emitted [inner, outer, inner, outer, ...] — index 1 is
        // the first fan's outer blade.
        let apex_to_outer = |prim: &EffectPrimitiveDraw| -> f32 {
            match prim {
                EffectPrimitiveDraw::WorldQuad { corners, .. } => {
                    let dx = corners[1][0] - corners[0][0];
                    let dy = corners[1][1] - corners[0][1];
                    let dz = corners[1][2] - corners[0][2];
                    (dx * dx + dy * dy + dz * dz).sqrt()
                }
                _ => panic!(),
            }
        };
        let dist_a = apex_to_outer(&draws(&e)[1]);
        step(&mut e, 1.0);
        let dist_b = apex_to_outer(&draws(&e)[1]);
        let ratio = dist_b / dist_a;
        assert!(
            (ratio - DISTANCE_GROWTH_PER_FRAME).abs() < 0.01,
            "distance grew by {ratio:.3}×, want {DISTANCE_GROWTH_PER_FRAME}",
        );
    }
}
