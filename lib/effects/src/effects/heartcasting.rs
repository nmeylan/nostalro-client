//! `EF_HEARTCASTING` (id 343) — a heart outline of 20 rising pink flame-rings,
//! the original game's love-confession / wedding casting effect.
//!
//! 20 ring nodes are placed
//! at hardcoded `(x, z)` ground offsets (× `cell = 0.27`) tracing a
//! heart. Each node has two
//! live ring slots — a short inner ring (`max_height 12`, `rise 86°`) and a
//! taller flame (`max_height 25`, `rise 90°`).
//!
//! Each node behaves as an undulating Heal
//! ring (grow-in over 90 frames, no spin) — so each node is exactly the
//! `heal.rs` `RiseLaw::Heal` ring, just offset to its heart position. This
//! effect is therefore 20 [`HealEffect`]s sharing one [`HEARTCASTING_NODE`]
//! param, each centred on a heart-outline point. Pink tint
//! (255, 89, 182), additive.

use crate::draw::{BlendKind, EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::heal::{HealEffect, HealParams, RiseLaw, SlotSeed};

pub const TEXTURES: &[&str] = &["ring_red.tga"];

/// Scales the raw heart coordinates to world units.
const CELL: f32 = 0.27;

/// Pink tint (255, 89, 182).
const PINK: [f32; 3] = [1.0, 89.0 / 255.0, 182.0 / 255.0];

/// Two rings per node — a short inner ring (`ec = 0`) and a tall flame
/// (`ec = 1`); both behave as the plain undulating Heal ring.
const HEARTCASTING_SLOTS: &[SlotSeed] = &[
    SlotSeed { ec: 0, distance: 2.3, max_height: 12.0, rise_angle_deg: 86.0, rot_start_deg: 0.0, alpha_t: 175.0, alpha_init: 0.0, flag1: 0 },
    SlotSeed { ec: 1, distance: 2.0, max_height: 25.0, rise_angle_deg: 90.0, rot_start_deg: 180.0, alpha_t: 175.0, alpha_init: 0.0, flag1: 0 },
];

/// One heart node: a short ring + a taller pink flame. Shared by all 20 nodes.
pub const HEARTCASTING_NODE: HealParams = HealParams {
    texture: "ring_red.tga",
    tint_rgb: PINK,
    blend: BlendKind::Additive,
    // max_height 12/25 → flames ~5/10 units tall; sized so the flames read
    // as roughly a third of the heart's span (gif silhouette).
    height_scale: 0.4,
    law: RiseLaw::Heal,
    slots: HEARTCASTING_SLOTS,
    duration_frames: 200.0,
    particle_up: None,
};

pub const TOTAL_DURATION_MS: u32 = HEARTCASTING_NODE.total_duration_ms();

/// Raw `(x, z)` heart-outline points (before `× CELL`). The
/// tip `(0, -60)` is the bottom of the heart; the lobes sit at `z ≈ +48`.
const HEART_POINTS: [(f32, f32); 20] = [
    (0.0, 37.0), (-13.0, 44.0), (-29.0, 48.0), (-46.0, 42.0), (-56.0, 26.0),
    (-57.0, 6.0), (-51.0, -12.0), (-40.0, -26.0), (-25.0, -38.0), (-10.0, -47.0),
    (0.0, -60.0), (11.0, -47.0), (26.0, -38.0), (41.0, -26.0), (52.0, -12.0),
    (58.0, 6.0), (57.0, 26.0), (47.0, 42.0), (30.0, 48.0), (14.0, 44.0),
];

pub struct HeartcastingEffect {
    nodes: Vec<HealEffect>,
}

impl HeartcastingEffect {
    pub fn new(anchor: [f32; 3]) -> Self {
        let [ax, ay, az] = anchor;
        let nodes = HEART_POINTS
            .iter()
            .map(|(x, z)| {
                HealEffect::new([ax + x * CELL, ay, az + z * CELL], &HEARTCASTING_NODE)
            })
            .collect();
        Self { nodes }
    }
}

impl Effect for HeartcastingEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let mut any_alive = false;
        for node in &mut self.nodes {
            if node.update(ctx) == EffectStatus::Running {
                any_alive = true;
            }
        }
        if any_alive {
            EffectStatus::Running
        } else {
            EffectStatus::Dead
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        for node in &self.nodes {
            node.collect_draws(out, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::EffectPrimitiveDraw;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut HeartcastingEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: frames / 60.0, camera_target: None, caster_yaw: None })
    }

    fn rings(e: &HeartcastingEffect) -> Vec<([f32; 3], f32, [f32; 21])> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::RadialRing { center, color, heights, .. } => {
                    Some((*center, color[3], *heights))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn twenty_nodes_each_two_rings_in_a_heart_spread() {
        // Sociable: 20 heart nodes × 2 rings = 40 RadialRings once alpha ramps
        // off zero, spread across a heart-shaped XZ footprint.
        let mut e = HeartcastingEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 3.0);
        let r = rings(&e);
        assert_eq!(r.len(), 40, "20 nodes × 2 rings");
        let xs: Vec<f32> = r.iter().map(|(c, _, _)| c[0]).collect();
        let zs: Vec<f32> = r.iter().map(|(c, _, _)| c[2]).collect();
        let (min_x, max_x) = (xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max));
        let (min_z, max_z) = (zs.iter().cloned().fold(f32::MAX, f32::min), zs.iter().cloned().fold(f32::MIN, f32::max));
        assert!(max_x - min_x > 25.0 && max_z - min_z > 25.0, "heart spans both axes");
        // Bottom tip is lower (more negative z) than the lobes.
        assert!(min_z < -15.0, "heart tip dips below the lobes");
    }

    #[test]
    fn flames_grow_in_then_effect_dies() {
        let mut e = HeartcastingEffect::new([0.0; 3]);
        step(&mut e, 5.0);
        let h_early: f32 = rings(&e).iter().map(|(_, _, h)| h[8]).sum();
        step(&mut e, 35.0); // climbing the 90-frame grow-in sin ramp
        let h_late: f32 = rings(&e).iter().map(|(_, _, h)| h[8]).sum();
        assert!(h_late > h_early, "flames rise in: {h_early} -> {h_late}");
        assert_eq!(step(&mut e, 200.0), EffectStatus::Dead, "dies by node duration");
    }
}
