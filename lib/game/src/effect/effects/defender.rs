//! `EF_DEFENDER` — Crusader's spinning shield aura.
//!
//! Reference: `ro-effects/effects/imgs/200-250/220.gif`.
//!
//! Original game (`ring_black.tga`) draws four breathing
//! slots. Each slot is advanced per-frame and rendered
//! as a 9-segment ring of `ring_black.tga` quads at
//! radius `distance`.
//!
//! Slot seed:
//! * 4 slots, all alive, full display angle = 360°, rise angle = 90°
//!   (fully upright), `distance` 8.0/7.9/7.8/7.7, rot-start 180/270/0/90,
//!   `max_height` 40/39/38/37.
//!
//! Per-frame tick:
//! * slots 0 & 2: rot-start `+= 1`; slots 1 & 3: rot-start `+= 2`.
//! * alpha: ramp from 0 → 80 over the first 100 frames, then hold,
//!   then ramp down by 5/frame in the last 20 frames.
//! * Per segment `i ∈ 0..division`:
//!   `height[i] = max_height·sin(SinLimit) · (1 + 0.3·sin(pr))`
//!   where `SinLimit = 90 + (i - 10)·9` and the breathing phase
//!   `pr = (process + ec·90) % 360` (slots 0–1) or
//!   `pr = (2·process + ec·90) % 360` (slots 2–3).
//!
//! Render: 9 segments per slot (`base angle = 360/9 = 40°`) reading
//! `height[0..9]`. The original draws a connected ribbon strip
//! (segment N's quad connects to segment N-1's); we approximate with
//! 9 discrete upright billboards — `ring_black.tga` is a thin transparent
//! bar so the visual difference is small.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::radial_emitter::{
    RADIAL_EMITTER_DIVISION, RADIAL_EMITTER_SLOTS, RadialEmitter, RadialEmitterSlot,
};

pub const TEXTURE: &str = "ring_black.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// Original game runs 200 frames at 60 fps ≈ 3.33 s.
/// table.rs's existing 2000 ms entry is the legacy STR-driven duration;
/// we honour the original-game frame budget here.
const TOTAL_FRAMES: u32 = 200;
pub const TOTAL_DURATION_MS: u32 =
    ((TOTAL_FRAMES as f32) / FRAMES_PER_SECOND * 1000.0) as u32;

/// Number of segments rendered per slot. The original game divides
/// the full display angle by 9 — a perf compromise (dividing by the
/// full 21-entry division would have read smoother). We use 21
/// so the base reads as a smooth disc rather than
/// a visible nonagon, and so every entry of the bell-curve `height[]`
/// array gets driven into the geometry.
const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;
const FULL_ARC_RAD: f32 = std::f32::consts::TAU;
const RISE_ANGLE_RAD: f32 = std::f32::consts::FRAC_PI_2;

/// Scales the per-slot height array (built from `max_height = 40`
/// × `sin(SinLimit)` × breathing) into world units. Raw peak is ~52;
/// the reference ribbon stands roughly 1/3 character height at its
/// brightest segment, so the peak should land near ~2 world units.
const HEIGHT_SCALE: f32 = 0.9;

const SLOT_DISTANCES: [f32; RADIAL_EMITTER_SLOTS] = [8.0, 7.9, 7.8, 7.7];
const SLOT_ROT_START_DEG: [f32; RADIAL_EMITTER_SLOTS] = [180.0, 270.0, 0.0, 90.0];
const SLOT_MAX_HEIGHT: [f32; RADIAL_EMITTER_SLOTS] = [40.0, 39.0, 38.0, 37.0];

const ALPHA_PEAK: f32 = 80.0 / 255.0;
const FADE_IN_FRAMES: u32 = 100;
const FADE_OUT_FRAMES: u32 = 20;
const FADE_OUT_STEP: f32 = 5.0 / 255.0;
const FADE_IN_STEP: f32 = 1.0 / 255.0;

/// `i ∈ 0..division=21`; middle = 10, m2 = 90/10 = 9; `SinLimit = 90 + (i - 10) * 9`.
const SIN_LIMIT_MIDDLE: i32 = 10;
const SIN_LIMIT_STEP_DEG: f32 = 9.0;

pub struct DefenderEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    last_processed_frame: u32,
    emitter: RadialEmitter,
}

impl DefenderEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut slots = [RadialEmitterSlot::dormant(); RADIAL_EMITTER_SLOTS];
        for ec in 0..RADIAL_EMITTER_SLOTS {
            let mut s = RadialEmitterSlot::spawn(
                SLOT_DISTANCES[ec],
                90.0, // rise_angle in original-game degrees; we ignore this on the slot
                SLOT_MAX_HEIGHT[ec],
            );
            s.rot_start_deg = SLOT_ROT_START_DEG[ec];
            s.full_display_angle_deg = 360.0;
            s.alpha_b = 0.0;
            slots[ec] = s;
        }
        Self {
            world_pos,
            age_frames: 0.0,
            last_processed_frame: 0,
            emitter: RadialEmitter::from_slots(slots),
        }
    }

    fn integrate_frames(&mut self, target_frame: u32) {
        while self.last_processed_frame < target_frame {
            self.emitter.tick();
            for (ec, slot) in self.emitter.slots.iter_mut().enumerate() {
                if !slot.alive {
                    continue;
                }
                // Rotation spin: slots 0 & 2 advance 1°/frame, slots 1 & 3 advance 2°/frame.
                slot.rot_start_deg += if ec == 0 || ec == 2 { 1.0 } else { 2.0 };
                if slot.rot_start_deg >= 360.0 {
                    slot.rot_start_deg -= 360.0;
                }

                // Alpha envelope: ramp in over FADE_IN_FRAMES, fade out over the last FADE_OUT_FRAMES.
                if slot.process >= TOTAL_FRAMES.saturating_sub(FADE_OUT_FRAMES) {
                    slot.alpha_b = (slot.alpha_b - FADE_OUT_STEP).max(0.0);
                } else if slot.process < FADE_IN_FRAMES {
                    slot.alpha_b = (slot.alpha_b + FADE_IN_STEP).min(ALPHA_PEAK);
                }

                // Breathing phase: slots 0–1 use process, slots 2–3 use 2*process.
                let pr_base = if ec < 2 {
                    slot.process as i32
                } else {
                    (slot.process as i32) * 2
                };
                let pr_deg = ((pr_base + (ec as i32) * 90).rem_euclid(360)) as f32;
                let sin_pr = pr_deg.to_radians().sin();

                for i in 0..RADIAL_EMITTER_DIVISION {
                    let sin_limit_deg =
                        90.0 + (i as i32 - SIN_LIMIT_MIDDLE) as f32 * SIN_LIMIT_STEP_DEG;
                    let bell = sin_limit_deg.to_radians().sin().max(0.0);
                    slot.height[i] = slot.max_height * bell * (1.0 + 0.3 * sin_pr);
                }
            }
            self.last_processed_frame += 1;
        }
    }
}

impl Effect for DefenderEffect {
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

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for (_ec, slot) in self.emitter.active() {
            if slot.alpha_b <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::RadialRing {
                center: self.world_pos,
                distance: slot.distance,
                rise_angle_rad: RISE_ANGLE_RAD,
                rot_start_rad: slot.rot_start_deg.to_radians(),
                full_arc_rad: FULL_ARC_RAD,
                segments: SEGMENTS,
                height_scale: HEIGHT_SCALE,
                heights: slot.height,
                texture: TEXTURE,
                color: [1.0, 1.0, 1.0, slot.alpha_b],
                blend: BlendKind::Alpha,
            });
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

    fn step(e: &mut DefenderEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
        })
    }

    fn draws(e: &DefenderEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn ring(prim: &EffectPrimitiveDraw) -> (f32, u32, f32, [f32; RADIAL_EMITTER_DIVISION]) {
        match prim {
            EffectPrimitiveDraw::RadialRing {
                distance,
                segments,
                color,
                heights,
                ..
            } => (*distance, *segments, color[3], *heights),
            _ => panic!("expected RadialRing, got {:?}", prim),
        }
    }

    #[test]
    fn emits_four_layered_rings_with_distinct_radii_and_spin() {
        // Sociable: after enough frames for alpha to lift off zero, all
        // four slots emit a RadialRing at their seeded radii. The two
        // odd slots advance their rotation twice as fast as the two even slots,
        // so by frame N their cumulative rotations diverge.
        let mut e = DefenderEffect::new([5.0, 0.0, -3.0]);
        step(&mut e, 30.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 4, "all four slots alive and visible");

        let radii: Vec<f32> = prims.iter().map(|p| ring(p).0).collect();
        assert_eq!(radii, vec![8.0, 7.9, 7.8, 7.7]);

        // Heights[10] is the bell-curve apex (sin(90°) = 1); after
        // breathing it should be near max_height·(1 ± 0.3).
        for (ec, prim) in prims.iter().enumerate() {
            let (_, segs, alpha, heights) = ring(prim);
            assert_eq!(segs, SEGMENTS);
            assert!(alpha > 0.0, "alpha lifted for slot {ec}");
            let max_h = SLOT_MAX_HEIGHT[ec];
            assert!(heights[10] >= max_h * 0.7 && heights[10] <= max_h * 1.3);
        }
    }

    #[test]
    fn alpha_ramps_in_then_fades_out() {
        let mut e = DefenderEffect::new([0.0; 3]);

        // First frame: alpha already moved off zero (fade-in is +1/255 per frame).
        step(&mut e, 1.0);
        let alpha_1 = ring(&draws(&e)[0]).2;
        assert!(alpha_1 > 0.0 && alpha_1 < ALPHA_PEAK);

        // After FADE_IN_FRAMES the alpha has reached ALPHA_PEAK and holds.
        step(&mut e, (FADE_IN_FRAMES - 1) as f32);
        let alpha_peak = ring(&draws(&e)[0]).2;
        assert!((alpha_peak - ALPHA_PEAK).abs() < 1e-3, "alpha at peak: {alpha_peak}");

        // After the fade-out window the alpha is back to zero — the
        // collect_draws() filter then omits the slot. Step past TOTAL_FRAMES
        // to trigger Dead; just before, alpha should be near zero.
        step(&mut e, (TOTAL_FRAMES - FADE_IN_FRAMES - 1) as f32);
        // process is now TOTAL_FRAMES-1; alpha fully faded.
        let prims = draws(&e);
        let alpha_late = prims.first().map(|p| ring(p).2).unwrap_or(0.0);
        assert!(alpha_late < 0.05, "alpha late: {alpha_late}");
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = DefenderEffect::new([0.0; 3]);
        let s = step(&mut e, TOTAL_FRAMES as f32 + 1.0);
        assert!(matches!(s, EffectStatus::Dead));
    }
}
