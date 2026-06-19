//! `EF_GLASSWALL2` (315) — Safety Wall's rising translucent column.
//!
//! Reference: `ro-effects/effects/imgs/300-350/315.gif`.
//!
//! Despite sharing a name with `Glasswall` (the blue four-wall box), this id
//! is a completely different effect: an `alpha_down.tga` column built from
//! three concentric upright (`rise_angle = 90`) rings of wall quads,
//! stacked at slightly different radii and merged into a single
//! glowing pillar over the cell:
//!
//! | slot | distance | max_height |
//! |------|----------|------------|
//! | 0    | 3.6      | 34         |
//! | 1    | 3.3      | 37         |
//! | 2    | 3.0      | 40         |
//!
//! `texture = alpha_down.tga` (a vertical alpha gradient — opaque at the base,
//! transparent at the top), per-slot rotation randomised, alpha ramps from 0.
//! `SafetyWall.str` plays alongside.
//!
//! Safety Wall is a sustained skill (`SET_DURATION = 9999` — the persistent
//! sentinel); the column stays up until the server despawns the cell. We ramp
//! the height + alpha in over the first ~24 frames, then hold.
//!
//! The reference column reads pink at the base fading to violet at the top —
//! the gradient texture is tinted accordingly (gif over the plain
//! `alpha_down.tga`, which blends additively to the same effect).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::radial_emitter::RADIAL_EMITTER_DIVISION;

pub const TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

pub const STR_OVERLAY: &str = "SafetyWall";

const FRAMES_PER_SECOND: f32 = 60.0;

/// Persistent sentinel — matches `table.rs`'s 99990 row for Glasswall2.
pub const TOTAL_DURATION_MS: u32 = 99990;

const SLOT_DISTANCES: [f32; 3] = [3.6, 3.3, 3.0];
const SLOT_MAX_HEIGHT: [f32; 3] = [34.0, 37.0, 40.0];
const SLOT_ROT_START_DEG: [f32; 3] = [40.0, 170.0, 290.0];

/// The slot heights 34–40 are tall literals; the reference
/// pillar stands ~3 character-heights, so scale the column down to world units.
const HEIGHT_SCALE: f32 = 0.42;
/// Frames over which the column rises to full height and ramps to peak alpha.
const RISE_FRAMES: f32 = 24.0;
const PEAK_ALPHA: f32 = 150.0 / 255.0;
/// Pink base → violet tip, matching the reference capsule.
const TINT: [f32; 3] = [1.0, 0.42, 0.85];

const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;
const RISE_ANGLE_RAD: f32 = std::f32::consts::FRAC_PI_2;
const FULL_ARC_RAD: f32 = std::f32::consts::TAU;

pub struct Glasswall2Effect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl Glasswall2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self { world_pos, age_frames: 0.0 }
    }

    /// 0 → 1 ramp over the first `RISE_FRAMES`, then held.
    fn rise(&self) -> f32 {
        (self.age_frames / RISE_FRAMES).clamp(0.0, 1.0)
    }
}

impl Effect for Glasswall2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let rise = self.rise();
        let alpha = PEAK_ALPHA * rise;
        if alpha <= 0.0 {
            return;
        }
        for slot in 0..3 {
            // Uniform height across the ring → a straight vertical wall; three
            // close concentric walls read as one solid glowing column.
            let h = SLOT_MAX_HEIGHT[slot] * rise;
            let heights = [h; RADIAL_EMITTER_DIVISION];
            out.push(EffectPrimitiveDraw::RadialRing {
                center: self.world_pos,
                distance: SLOT_DISTANCES[slot],
                rise_angle_rad: RISE_ANGLE_RAD,
                rot_start_rad: SLOT_ROT_START_DEG[slot].to_radians(),
                full_arc_rad: FULL_ARC_RAD,
                segments: SEGMENTS,
                height_scale: HEIGHT_SCALE,
                heights,
                texture: TEXTURE,
                color: [TINT[0], TINT[1], TINT[2], alpha],
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

    fn step_and_draw(e: &mut Glasswall2Effect, frames: f32) -> Vec<EffectPrimitiveDraw> {
        e.update(&EffectUpdateCtx { delta: frames / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_three_concentric_rising_rings_with_str_overlay() {
        // Sociable: three RadialRing walls at the three slot distances, all
        // upright (rise_angle = 90°), plus the SafetyWall STR overlay.
        let mut e = Glasswall2Effect::new([10.0, 0.0, 20.0]);
        assert_eq!(e.str_overlay(), Some(STR_OVERLAY));
        let prims = step_and_draw(&mut e, RISE_FRAMES);
        let rings: Vec<_> = prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::RadialRing { distance, rise_angle_rad, texture, .. } => {
                    Some((*distance, *rise_angle_rad, *texture))
                }
                _ => None,
            })
            .collect();
        assert_eq!(rings.len(), 3, "three stacked rings");
        for (_, rise, tex) in &rings {
            assert!((rise - RISE_ANGLE_RAD).abs() < 1e-4, "rings are upright");
            assert_eq!(*tex, TEXTURE);
        }
        let mut dists: Vec<f32> = rings.iter().map(|(d, ..)| *d).collect();
        dists.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((dists[0] - 3.0).abs() < 1e-4 && (dists[2] - 3.6).abs() < 1e-4);
    }

    #[test]
    fn column_rises_and_ramps_in_then_persists() {
        let mut e = Glasswall2Effect::new([0.0; 3]);
        let early = step_and_draw(&mut e, 6.0);
        let (h_early, a_early) = match &early[0] {
            EffectPrimitiveDraw::RadialRing { heights, color, .. } => (heights[0], color[3]),
            _ => unreachable!(),
        };
        let held = step_and_draw(&mut e, RISE_FRAMES); // well past the ramp
        let (h_full, a_full) = match &held[0] {
            EffectPrimitiveDraw::RadialRing { heights, color, .. } => (heights[0], color[3]),
            _ => unreachable!(),
        };
        assert!(h_full > h_early, "column grows taller");
        assert!(a_full > a_early, "alpha ramps in");
        assert!((a_full - PEAK_ALPHA).abs() < 1e-4, "alpha holds at peak");
    }

    #[test]
    fn persists_past_the_normal_effect_lifetime() {
        let mut e = Glasswall2Effect::new([0.0; 3]);
        let s = e.update(&EffectUpdateCtx { delta: 10.0, camera_target: None, caster_yaw: None });
        assert!(matches!(s, EffectStatus::Running), "Safety Wall is persistent");
    }
}
