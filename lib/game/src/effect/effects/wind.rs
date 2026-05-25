//! `EF_WIND` — Wizard's Wind tornado / cloud funnel above the caster.
//!
//! Reference: `ro-effects/effects/imgs/200-250/222.gif`.
//!
//! Funnel uses `cloud11.tga` across four nested expanding
//! partial-arc ribbons sitting at altitude `max_height = 8` above the
//! player. Each slot has a different starting arc width and rotation
//! offset; together they read
//! as a swirling funnel.
//!
//! Slot seed:
//! * `max_height = 8`, `rise_angle = 90` (fully upright), top alpha 0
//!   for every slot.
//! * `(arc°, distance, rot_start°, ribbon_height)` per slot:
//!     * 0: `(90, 3.0, 0,  3.0)`
//!     * 1: `(70, 3.2, 10, 2.5)`
//!     * 2: `(50, 3.4, 20, 2.0)`
//!     * 3: `(30, 3.6, 30, 1.5)`
//!
//! Per-frame tick (default branch):
//! * `rot_start += 5` (wrap at 360).
//! * arc span grows +3°/frame, capped at 120°.
//! * first 12 frames: `alpha_b += 10` (capped at 250). After frame 12 the
//!   alpha holds at 120/255 ≈ 0.47 for the rest of the effect.
//! * a late fade exists but is never reached (duration is 300 frames).
//!
//! Render: each slot's ring base sits at altitude `-max_height` (native
//! RO `-Y = up`), with `SEGMENTS` segments across the arc. Each segment's
//! per-position quad height is the constant `height[i]` set at spawn.
//! Connected ribbon strip — same primitive as Defender, just with
//! `full_arc_rad < TAU` and the centre shifted up.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::radial_emitter::{
    RADIAL_EMITTER_DIVISION, RADIAL_EMITTER_SLOTS, RadialEmitter, RadialEmitterSlot,
};

pub const TEXTURE: &str = "cloud11.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// 300 frames at 60 fps = 5 s, matching the original game.
const TOTAL_FRAMES: u32 = 300;
pub const TOTAL_DURATION_MS: u32 =
    ((TOTAL_FRAMES as f32) / FRAMES_PER_SECOND * 1000.0) as u32;

const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;
const RISE_ANGLE_RAD: f32 = std::f32::consts::FRAC_PI_2;

/// Per-slot `(arc°, distance, rot_start°, ribbon_height)` matching the
/// original game's funnel seed.
const SLOT_SEED: [(f32, f32, f32, f32); RADIAL_EMITTER_SLOTS] = [
    (90.0, 3.0, 0.0, 3.0),
    (70.0, 3.2, 10.0, 2.5),
    (50.0, 3.4, 20.0, 2.0),
    (30.0, 3.6, 30.0, 1.5),
];

/// The cloud ribbon's base elevation
/// above the caster's feet. Native RO `-Y = up`, so the renderer's centre
/// is shifted by `-MAX_HEIGHT` in Y.
const MAX_HEIGHT: f32 = 8.0;

/// Heights are already in world-unit-ish numbers (~1.5-3), but the
/// original game's world is smaller than ours; this scale tunes
/// the ribbon thickness to match the reference funnel silhouette.
const HEIGHT_SCALE: f32 = 1.0;

const ROT_SPIN_PER_FRAME_DEG: f32 = 5.0;
const ARC_GROWTH_PER_FRAME_DEG: f32 = 3.0;
const ARC_MAX_DEG: f32 = 120.0;

const ALPHA_RAMP_FRAMES: u32 = 12;
const ALPHA_RAMP_STEP: f32 = 10.0 / 255.0;
const ALPHA_CAP: f32 = 250.0 / 255.0;

pub struct WindEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    last_processed_frame: u32,
    emitter: RadialEmitter,
}

impl WindEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut slots = [RadialEmitterSlot::dormant(); RADIAL_EMITTER_SLOTS];
        for (ec, &(arc_deg, distance, rot_deg, ribbon_h)) in SLOT_SEED.iter().enumerate() {
            let mut s = RadialEmitterSlot::spawn(distance, 90.0, MAX_HEIGHT);
            s.full_display_angle_deg = arc_deg;
            s.rot_start_deg = rot_deg;
            s.alpha_b = 0.0;
            for h in s.height.iter_mut() {
                *h = ribbon_h;
            }
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
            for slot in self.emitter.slots.iter_mut().filter(|s| s.alive) {
                slot.rot_start_deg += ROT_SPIN_PER_FRAME_DEG;
                if slot.rot_start_deg >= 360.0 {
                    slot.rot_start_deg -= 360.0;
                }

                slot.full_display_angle_deg =
                    (slot.full_display_angle_deg + ARC_GROWTH_PER_FRAME_DEG).min(ARC_MAX_DEG);

                if slot.process < ALPHA_RAMP_FRAMES {
                    slot.alpha_b = (slot.alpha_b + ALPHA_RAMP_STEP).min(ALPHA_CAP);
                }
            }
            self.last_processed_frame += 1;
        }
    }
}

impl Effect for WindEffect {
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
        // Ribbon base sits MAX_HEIGHT above the caster (native RO -Y up,
        // so subtract from Y).
        let center = [
            self.world_pos[0],
            self.world_pos[1] - MAX_HEIGHT,
            self.world_pos[2],
        ];
        for (_ec, slot) in self.emitter.active() {
            if slot.alpha_b <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::RadialRing {
                center,
                distance: slot.distance,
                rise_angle_rad: RISE_ANGLE_RAD,
                rot_start_rad: slot.rot_start_deg.to_radians(),
                full_arc_rad: slot.full_display_angle_deg.to_radians(),
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

    fn step(e: &mut WindEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
        })
    }

    fn draws(e: &WindEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn ring(prim: &EffectPrimitiveDraw) -> (f32, f32, f32, f32, [f32; 3]) {
        match prim {
            EffectPrimitiveDraw::RadialRing {
                distance,
                full_arc_rad,
                rot_start_rad,
                color,
                center,
                ..
            } => (
                *distance,
                full_arc_rad.to_degrees(),
                rot_start_rad.to_degrees(),
                color[3],
                *center,
            ),
            _ => panic!("expected RadialRing, got {:?}", prim),
        }
    }

    #[test]
    fn emits_four_nested_arcs_above_caster() {
        // Sociable: after enough frames for alpha to lift, all four slots
        // emit a RadialRing at their seeded radii, with the ring centre
        // shifted MAX_HEIGHT above the caster's feet.
        let mut e = WindEffect::new([5.0, 0.0, -3.0]);
        step(&mut e, 15.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 4);

        let distances: Vec<f32> = prims.iter().map(|p| ring(p).0).collect();
        assert_eq!(distances, vec![3.0, 3.2, 3.4, 3.6]);

        for prim in &prims {
            let (_, _, _, alpha, center) = ring(prim);
            assert!(alpha > 0.0);
            // Native RO -Y up: centre y = world_pos.y - MAX_HEIGHT.
            assert!((center[1] + MAX_HEIGHT).abs() < 1e-4);
            assert_eq!(center[0], 5.0);
            assert_eq!(center[2], -3.0);
        }
    }

    #[test]
    fn arc_expands_and_rotation_spins() {
        // Sociable: arc grows 3°/frame to 120° cap; rot_start spins 5°/frame.
        let mut e = WindEffect::new([0.0; 3]);
        step(&mut e, 1.0);
        let (_, arc1, rot1, _, _) = ring(&draws(&e)[0]);
        // Slot 0 seeded at 90° arc, 0° rot; after one tick: arc=93, rot=5.
        assert!((arc1 - 93.0).abs() < 0.1, "arc after 1 frame: {arc1}");
        assert!((rot1 - 5.0).abs() < 0.1, "rot after 1 frame: {rot1}");

        // After enough frames, arc caps at 120 (10 more frames takes it
        // from 93 to 120 = +27 → 9 frames; step 20 to be safe).
        step(&mut e, 20.0);
        let (_, arc_late, _, _, _) = ring(&draws(&e)[0]);
        assert!((arc_late - ARC_MAX_DEG).abs() < 0.1, "arc capped: {arc_late}");
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = WindEffect::new([0.0; 3]);
        let s = step(&mut e, TOTAL_FRAMES as f32 + 1.0);
        assert!(matches!(s, EffectStatus::Dead));
    }
}
