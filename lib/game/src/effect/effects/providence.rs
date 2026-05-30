//! `EF_PROVIDENCE` (248) — the descending light funnel of Providence.
//!
//! Reference: `ro-effects/effects/imgs/200-250/248.gif`.
//!
//! Built from four upright-but-slightly-flared rings on `ring_yellow.tga` that
//! form the tall warm light column; the central angel sprite, the ground glow
//! and the rising sparkles all come from `providence.str`, played alongside via
//! the [`Effect::str_overlay`] hook. Verification confirmed the STR renders
//! everything *except* this central column, so it is built here as a
//! primitive.
//!
//! Slot seed (`F1 == 0`):
//!
//! | slot | distance | rise_angle | rotation |
//! |------|----------|-----------|----------|
//! | 0    | 6.0      | 83°       | 0°       |
//! | 1    | 6.5      | 82°       | 90°      |
//! | 2    | 7.0      | 81°       | 180°     |
//! | 3    | 7.5      | 80°       | 270°     |
//!
//! `max_height = 120`, peak alpha 40/255, full 360° ring. The rise angle just
//! under 90° tilts each ring's top edge slightly outward, so the column flares
//! wider toward the top — the funnel silhouette in the gif. The rings rotate
//! slowly to give the whole column a turning look.
//!
//! [`Effect::str_overlay`]: crate::effect::effect_trait::Effect::str_overlay

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::radial_emitter::RADIAL_EMITTER_DIVISION;

pub const TEXTURE: &str = "ring_yellow.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

pub const STR_OVERLAY: &str = "providence";

const FRAMES_PER_SECOND: f32 = 60.0;

/// 200-frame lifetime at 60 fps.
const TOTAL_FRAMES: f32 = 200.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const SLOT_DISTANCE: [f32; 4] = [6.0, 6.5, 7.0, 7.5];
const SLOT_RISE_ANGLE_DEG: [f32; 4] = [83.0, 82.0, 81.0, 80.0];
const SLOT_ROT_START_DEG: [f32; 4] = [0.0, 90.0, 180.0, 270.0];

const MAX_HEIGHT: f32 = 120.0;
/// The gif funnel stands ~4 character-heights, so scale the 120 raw height
/// into world units.
const HEIGHT_SCALE: f32 = 0.16;
const PEAK_ALPHA: f32 = 120.0 / 255.0;
const FADE_IN_FRAMES: f32 = 14.0;
const FADE_OUT_FRAMES: f32 = 30.0;
/// Slow rotation of the whole column.
const SPIN_DEG_PER_FRAME: f32 = 1.5;

const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;
const FULL_ARC_RAD: f32 = std::f32::consts::TAU;

pub struct ProvidenceEffect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl ProvidenceEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age_frames: 0.0 }
    }

    fn alpha(&self) -> f32 {
        if self.age_frames < FADE_IN_FRAMES {
            PEAK_ALPHA * (self.age_frames / FADE_IN_FRAMES)
        } else if self.age_frames > TOTAL_FRAMES - FADE_OUT_FRAMES {
            let t = (TOTAL_FRAMES - self.age_frames).max(0.0) / FADE_OUT_FRAMES;
            PEAK_ALPHA * t
        } else {
            PEAK_ALPHA
        }
    }
}

impl Effect for ProvidenceEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return;
        }
        let spin = (self.age_frames * SPIN_DEG_PER_FRAME).to_radians();
        // Uniform full-height ring; the sub-90° rise angle does the flaring.
        let heights = [MAX_HEIGHT; RADIAL_EMITTER_DIVISION];
        for slot in 0..4 {
            out.push(EffectPrimitiveDraw::RadialRing {
                center: self.world_pos,
                distance: SLOT_DISTANCE[slot],
                rise_angle_rad: SLOT_RISE_ANGLE_DEG[slot].to_radians(),
                rot_start_rad: SLOT_ROT_START_DEG[slot].to_radians() + spin,
                full_arc_rad: FULL_ARC_RAD,
                segments: SEGMENTS,
                height_scale: HEIGHT_SCALE,
                heights,
                texture: TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
            });
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(STR_OVERLAY)
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

    fn step_and_draw(e: &mut ProvidenceEffect, frames: f32) -> Vec<EffectPrimitiveDraw> {
        e.update(&EffectUpdateCtx { delta: frames / FRAMES_PER_SECOND, camera_target: None });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_four_flared_rings_and_carries_providence_str() {
        // Sociable: four RadialRing slots, each flared (rise_angle < 90°) at
        // an increasing radius, plus the providence STR overlay for the angel.
        let mut e = ProvidenceEffect::new([5.0, 0.0, -3.0]);
        assert_eq!(e.str_overlay(), Some(STR_OVERLAY));
        let prims = step_and_draw(&mut e, FADE_IN_FRAMES);
        let rings: Vec<_> = prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::RadialRing { distance, rise_angle_rad, texture, .. } => {
                    Some((*distance, *rise_angle_rad, *texture))
                }
                _ => None,
            })
            .collect();
        assert_eq!(rings.len(), 4, "four-ring funnel");
        for (_, rise, tex) in &rings {
            assert!(*rise < std::f32::consts::FRAC_PI_2, "rings flare outward (< 90°)");
            assert_eq!(*tex, TEXTURE);
        }
        let mut dists: Vec<f32> = rings.iter().map(|(d, ..)| *d).collect();
        dists.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((dists[0] - 6.0).abs() < 1e-4 && (dists[3] - 7.5).abs() < 1e-4);
    }

    #[test]
    fn alpha_fades_in_then_out_over_lifetime() {
        let mut e = ProvidenceEffect::new([0.0; 3]);
        let early = step_and_draw(&mut e, 4.0);
        let a_early = match &early[0] {
            EffectPrimitiveDraw::RadialRing { color, .. } => color[3],
            _ => unreachable!(),
        };
        let mid = step_and_draw(&mut e, FADE_IN_FRAMES);
        let a_mid = match &mid[0] {
            EffectPrimitiveDraw::RadialRing { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_mid > a_early, "alpha climbs during fade-in");
        // Walk deep into the fade-out tail.
        let late = step_and_draw(&mut e, TOTAL_FRAMES - FADE_IN_FRAMES - 4.0 - 2.0);
        let a_late = late.first().map(|p| match p {
            EffectPrimitiveDraw::RadialRing { color, .. } => color[3],
            _ => unreachable!(),
        }).unwrap_or(0.0);
        assert!(a_late < a_mid, "alpha drops during fade-out");
    }

    #[test]
    fn dies_after_parent_duration() {
        let mut e = ProvidenceEffect::new([0.0; 3]);
        let s = e.update(&EffectUpdateCtx {
            delta: TOTAL_DURATION_MS as f32 / 1000.0 + 0.1,
            camera_target: None,
        });
        assert!(matches!(s, EffectStatus::Dead));
    }
}
